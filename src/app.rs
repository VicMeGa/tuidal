use crate::i18n::Lang;
use crate::library::{LibraryFocus, LibrarySection, LibraryState, LibraryViewing};
use crate::player::{Player, TrackInfo};
use crate::settings::Settings;
use crate::tasks;
use crate::tidal::{
    Album, Artist, CoverInfo, FavAlbum, Lyrics, Mix, Playlist, Quality, SearchResults, StreamInfo,
    TidalDaemonClient, Track,
};
use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use serde::Deserialize as DeserializeAttr;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchFocus {
    Sidebar,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum SearchSection {
    Tracks = 0,
    Albums = 1,
    Artists = 2,
    Playlists = 3,
}

impl SearchSection {
    pub fn next(self) -> Self {
        match self {
            SearchSection::Tracks => SearchSection::Albums,
            SearchSection::Albums => SearchSection::Artists,
            SearchSection::Artists => SearchSection::Playlists,
            SearchSection::Playlists => SearchSection::Tracks,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SearchSection::Tracks => SearchSection::Playlists,
            SearchSection::Albums => SearchSection::Tracks,
            SearchSection::Artists => SearchSection::Albums,
            SearchSection::Playlists => SearchSection::Artists,
        }
    }
}

pub struct SearchState {
    pub focus: SearchFocus,
    pub active_section: SearchSection,
    pub cursor: [usize; 4],
    pub results: SearchResults,
    pub viewing: Option<LibraryViewing>,
    pub pending_viewing_title: String,
    pub input: String,
    pub history: VecDeque<String>,
    pub history_cursor: Option<usize>,
    pub draft: String,
    /// Items already fetched per section — doubles as the next pagination offset.
    pub loaded: [usize; 4],
    /// True when Tidal reported no more results for that section.
    pub exhausted: [bool; 4],
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            focus: SearchFocus::Sidebar,
            active_section: SearchSection::Tracks,
            cursor: [0; 4],
            results: SearchResults::default(),
            viewing: None,
            pending_viewing_title: String::new(),
            input: String::new(),
            history: VecDeque::new(),
            history_cursor: None,
            draft: String::new(),
            loaded: [0; 4],
            exhausted: [false; 4],
        }
    }

    pub fn len(&self, section: SearchSection) -> usize {
        match section {
            SearchSection::Tracks => self.results.tracks.len(),
            SearchSection::Albums => self.results.albums.len(),
            SearchSection::Artists => self.results.artists.len(),
            SearchSection::Playlists => self.results.playlists.len(),
        }
    }

    pub fn current_len(&self) -> usize {
        self.len(self.active_section)
    }

    pub fn record_history(&mut self, query: &str) {
        if let Some(pos) = self.history.iter().position(|h| h == query) {
            self.history.remove(pos);
        }
        self.history.push_front(query.to_string());
        while self.history.len() > 10 {
            self.history.pop_back();
        }
        self.history_cursor = None;
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.history_cursor.is_none() {
            self.draft = self.input.clone();
            self.history_cursor = Some(0);
        } else {
            let cursor = self.history_cursor.unwrap();
            if cursor + 1 < self.history.len() {
                self.history_cursor = Some(cursor + 1);
            } else {
                return;
            }
        }
        let idx = self.history_cursor.unwrap();
        self.input.clone_from(&self.history[idx]);
    }

    pub fn history_down(&mut self) {
        match self.history_cursor {
            None => {}
            Some(0) => {
                self.history_cursor = None;
                self.input = std::mem::take(&mut self.draft);
            }
            Some(cursor) => {
                let prev = cursor - 1;
                self.history_cursor = Some(prev);
                self.input.clone_from(&self.history[prev]);
            }
        }
    }
}

/// Appends items from `src` that aren't already present in `dst` (keyed by `key`).
/// Returns the number of items actually appended.
fn append_dedup<T, K: Eq>(dst: &mut Vec<T>, src: Vec<T>, key: impl Fn(&T) -> K) -> usize {
    let before = dst.len();
    for item in src {
        if !dst.iter().any(|existing| key(existing) == key(&item)) {
            dst.push(item);
        }
    }
    dst.len() - before
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Search,
    Queue,
    Now,
    Library,
}

pub enum AppEvent {
    SearchDone(Result<SearchResults, String>, u64),
    SearchMoreDone(Result<SearchResults, String>, u64, SearchSection),
    StreamReady {
        track: Track,
        info: StreamInfo,
        queue_index: usize,
        generation: u64,
    },
    StreamError {
        error: String,
        generation: u64,
    },
    AuthStarted {
        url: String,
        code: String,
        device_code: String,
    },
    AuthDone,
    AuthError(String),
    StatusMsg(String),
    CoverReady {
        info: CoverInfo,
        image: DynamicImage,
    },
    CoverError,
    LibraryLoaded {
        playlists: Vec<Playlist>,
        mixes: Vec<Mix>,
    },
    PlaylistTracksLoaded(Vec<Track>),
    FavTracksLoaded(Vec<Track>),
    FavAlbumsLoaded(Vec<FavAlbum>),
    ApiCmd(ApiCommand),
    LyricsReady(Lyrics),
    LyricsError,
}

pub enum ApiCommand {
    PlayPause,
    Next,
    Prev,
    VolumeUp,
    VolumeDown,
    VolumeSet(u8),
    SeekForward,
    SeekBackward,
    PlayTrack(ApiTrack),
    ToggleShuffle,
    CycleRepeat,
    Stop,
    Seek(i64),
    SetPosition(u64),
}

#[derive(Debug, Clone, DeserializeAttr)]
pub struct ApiTrack {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    Off,
    One,
    All,
}

impl Default for RepeatMode {
    fn default() -> Self {
        RepeatMode::All
    }
}

#[derive(Clone, Default, Serialize)]
pub struct ApiStatus {
    pub state: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<u64>,
    pub elapsed: u64,
    pub volume: u8,
    pub progress: f64,
    pub bit_depth: Option<u32>,
    pub sample_rate: Option<u32>,
    pub codec: Option<String>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub authenticated: bool,
    pub track_id: Option<u64>,
    pub queue: Arc<Vec<Track>>,
    pub queue_index: Option<usize>,
}

pub struct App {
    pub tidal: Arc<TidalDaemonClient>,
    pub player: Player,
    pub input_mode: InputMode,
    pub active_tab: Tab,

    pub search: SearchState,
    pub queue: Arc<Vec<Track>>,

    pub selected: usize,
    pub queue_index: Option<usize>,

    pub authenticated: bool,
    pub status_msg: String,
    pub loading: bool,
    pub auto_advance: bool,
    pub should_quit: bool,
    pub show_help: bool,

    pub device_code: Option<String>,
    pub user_code: Option<String>,
    pub auth_url: Option<String>,

    pub event_tx: Option<UnboundedSender<AppEvent>>,

    pub cover_info: Option<CoverInfo>,
    pub cover_image: Option<DynamicImage>,
    pub cover_proto: Option<StatefulProtocol>,
    pub picker: Option<Picker>,
    pub last_img_area: Option<(u16, u16)>,
    pub current_track_id: Option<u64>,
    pub stream_generation: u64,
    pub search_generation: u64,

    pub library: LibraryState,

    pub lang: Lang,
    pub quality: Quality,
    pub shuffle: bool,
    pub repeat: RepeatMode,

    pub lyrics: Option<Lyrics>,
}

impl App {
    pub fn new(tidal: Arc<TidalDaemonClient>) -> Self {
        Self {
            tidal,
            player: Player::new(),
            input_mode: InputMode::Normal,
            active_tab: Tab::Search,
            search: SearchState::new(),
            queue: Arc::new(Vec::new()),
            selected: 0,
            queue_index: None,
            authenticated: false,
            status_msg: String::new(),
            loading: false,
            auto_advance: false,
            should_quit: false,
            show_help: false,
            device_code: None,
            user_code: None,
            auth_url: None,
            event_tx: None,
            cover_info: None,
            cover_image: None,
            cover_proto: None,
            picker: None,
            last_img_area: None,
            current_track_id: None,
            stream_generation: 0,
            search_generation: 0,
            library: LibraryState::new(),

            lang: Lang::Es,
            quality: Quality::Lossless,
            shuffle: false,
            repeat: RepeatMode::All,
            lyrics: None,
        }
    }

    pub fn switch_tab(&mut self, tab: Tab) {
        if self.active_tab == tab {
            return;
        }

        let entering_library = tab == Tab::Library;

        self.active_tab = tab;

        if entering_library {
            self.library.active_section = LibrarySection::Playlists;
            self.library.focus = LibraryFocus::Sidebar;
            self.ensure_section_loaded();
        }
        self.selected = 0;
    }

    pub fn cycle_lang(&mut self) {
        self.lang = self.lang.cycle();
        self.status_msg = self.lang.lang_changed();
        self.save_settings();
    }

    fn tx(&self) -> UnboundedSender<AppEvent> {
        self.event_tx.clone().expect("event_tx no inicializado")
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::SearchDone(Ok(results), generation) => {
                if generation != self.search_generation {
                    return;
                }
                let total: usize = results.tracks.len()
                    + results.albums.len()
                    + results.artists.len()
                    + results.playlists.len();
                self.status_msg = if total == 0 {
                    self.lang.strings().status_no_results.to_string()
                } else {
                    self.lang.results_count(total)
                };
                self.search.results = results;
                self.search.cursor = [0; 4];
                self.search.focus = SearchFocus::Sidebar;
                self.search.active_section = SearchSection::Tracks;
                self.search.viewing = None;
                self.search.loaded = [
                    self.search.results.tracks.len(),
                    self.search.results.albums.len(),
                    self.search.results.artists.len(),
                    self.search.results.playlists.len(),
                ];
                self.search.exhausted = [
                    self.search.results.tracks.len() < 10,
                    self.search.results.albums.len() < 10,
                    self.search.results.artists.len() < 10,
                    self.search.results.playlists.len() < 10,
                ];
                self.active_tab = Tab::Search;
                self.loading = false;
            }
            AppEvent::SearchMoreDone(Ok(results), generation, section) => {
                if generation != self.search_generation {
                    return;
                }
                let idx = section as usize;
                let page_size = match section {
                    SearchSection::Tracks => results.tracks.len(),
                    SearchSection::Albums => results.albums.len(),
                    SearchSection::Artists => results.artists.len(),
                    SearchSection::Playlists => results.playlists.len(),
                };
                let appended = match section {
                    SearchSection::Tracks => {
                        append_dedup(&mut self.search.results.tracks, results.tracks, |t| t.id)
                    }
                    SearchSection::Albums => {
                        append_dedup(&mut self.search.results.albums, results.albums, |a| a.id)
                    }
                    SearchSection::Artists => {
                        append_dedup(&mut self.search.results.artists, results.artists, |a| a.id)
                    }
                    SearchSection::Playlists => {
                        append_dedup(&mut self.search.results.playlists, results.playlists, |p| {
                            p.uuid.clone()
                        })
                    }
                };
                self.search.loaded[idx] += appended;
                if page_size < 10 || appended == 0 {
                    self.search.exhausted[idx] = true;
                }
                self.status_msg = self.lang.results_count(self.search.len(section));
                self.loading = false;
            }
            AppEvent::SearchMoreDone(Err(e), generation, _section) => {
                if generation != self.search_generation {
                    return;
                }
                self.status_msg = self.lang.search_error(&e);
                self.loading = false;
            }
            AppEvent::SearchDone(Err(e), generation) => {
                if generation != self.search_generation {
                    return;
                }
                self.status_msg = self.lang.search_error(&e);
                self.loading = false;
            }
            AppEvent::StreamReady {
                track,
                info,
                queue_index,
                generation,
            } => {
                if generation != self.stream_generation {
                    return;
                }
                self.queue_index = Some(queue_index);
                self.current_track_id = Some(track.id);
                let ti = TrackInfo {
                    title: track.title.clone(),
                    artist: track.artist_names(),
                    album: track.album.title.clone(),
                    duration: track.duration,
                    bit_depth: info.bit_depth,
                    sample_rate: info.sample_rate,
                    codec: info.codec.clone(),
                };
                self.status_msg = format!(
                    "▶ {} — {} | {}/{} {}",
                    track.artist_names(),
                    track.title,
                    info.bit_depth,
                    info.sample_rate,
                    info.codec.to_uppercase()
                );
                self.player.play(&info.url, ti);
                self.loading = false;
                self.auto_advance = true;
                self.load_cover_bg(track.id);
                self.load_lyrics_bg(track.id);
            }
            AppEvent::StreamError { error, generation } => {
                if generation != 0 && generation != self.stream_generation {
                    return;
                }
                self.status_msg = self.lang.stream_error(&error);
                self.loading = false;
                if generation != 0 {
                    if let Some(i) = self.queue_index {
                        let next = i + 1;
                        if next < self.queue.len() {
                            let track = self.queue[next].clone();
                            self.stream_track_bg(track, next);
                        }
                    }
                }
            }
            AppEvent::AuthStarted {
                url,
                code,
                device_code,
            } => {
                self.device_code = Some(device_code);
                self.user_code = Some(code.clone());
                self.auth_url = Some(url.clone());
                let url_to_open = if url.starts_with("http://") || url.starts_with("https://") {
                    url.clone()
                } else {
                    format!("https://{}", url)
                };
                if let Err(e) = open::that(&url_to_open) {
                    self.status_msg = self.lang.browser_failed(&e.to_string(), &url);
                } else {
                    self.status_msg = self.lang.browser_opened(&code);
                }
                self.loading = false;
            }
            AppEvent::AuthDone => {
                self.authenticated = true;
                self.device_code = None;
                self.user_code = None;
                self.auth_url = None;
                self.status_msg = self.lang.strings().status_auth_done.to_string();
                self.loading = false;
            }
            AppEvent::AuthError(e) => {
                self.status_msg = self.lang.auth_error(&e);
                self.loading = false;
            }
            AppEvent::StatusMsg(msg) => {
                self.status_msg = msg;
            }
            AppEvent::CoverReady { info, image } => {
                self.cover_info = Some(info);
                self.cover_image = Some(image);
                self.cover_proto = None;
                self.last_img_area = None;
            }
            AppEvent::CoverError => {
                self.cover_image = None;
                self.cover_proto = None;
            }
            AppEvent::LyricsReady(lyrics) => {
                self.lyrics = Some(lyrics);
            }
            AppEvent::LyricsError => {
                self.lyrics = None;
            }
            AppEvent::LibraryLoaded { playlists, mixes } => {
                self.library.playlists = Some(playlists);
                self.library.mixes = Some(mixes);
                self.active_tab = Tab::Library;
                self.loading = false;
                let plen = self.library.playlists.as_ref().map_or(0, |p| p.len());
                let mlen = self.library.mixes.as_ref().map_or(0, |m| m.len());
                self.status_msg = self.lang.library_loaded(plen, mlen);
            }
            AppEvent::PlaylistTracksLoaded(tracks) => {
                let n = tracks.len();
                if !self.search.pending_viewing_title.is_empty() {
                    let title = std::mem::take(&mut self.search.pending_viewing_title);
                    self.search.viewing = Some(LibraryViewing {
                        title,
                        tracks,
                        cursor: 0,
                    });
                } else {
                    let title = std::mem::take(&mut self.library.pending_viewing_title);
                    self.library.viewing = Some(LibraryViewing {
                        title,
                        tracks,
                        cursor: 0,
                    });
                }
                self.loading = false;
                self.status_msg = self.lang.tracks_loaded(n);
            }
            AppEvent::FavTracksLoaded(tracks) => {
                let n = tracks.len();
                self.library.fav_tracks = Some(tracks);
                self.loading = false;
                self.status_msg = self.lang.fav_tracks_loaded(n);
            }
            AppEvent::FavAlbumsLoaded(albums) => {
                self.library.fav_albums = Some(albums);
                self.library.cursor[LibrarySection::FavAlbums as usize] = 0;
                self.library.active_section = LibrarySection::FavAlbums;
                self.active_tab = Tab::Library;
                self.loading = false;
                let count = self.library.fav_albums.as_ref().map_or(0, |a| a.len());
                self.status_msg = self.lang.fav_albums_loaded(count);
            }
            AppEvent::ApiCmd(cmd) => match cmd {
                ApiCommand::PlayPause => {
                    self.player.toggle_pause();
                }
                ApiCommand::Next => {
                    self.play_next_bg();
                }
                ApiCommand::Prev => {
                    self.play_prev_bg();
                }
                ApiCommand::VolumeUp => {
                    self.volume_up();
                }
                ApiCommand::VolumeDown => {
                    self.volume_down();
                }
                ApiCommand::VolumeSet(v) => {
                    self.set_volume(v);
                }
                ApiCommand::SeekForward => {
                    self.player.seek_forward();
                }
                ApiCommand::SeekBackward => {
                    self.player.seek_backward();
                }
                ApiCommand::ToggleShuffle => {
                    self.shuffle = !self.shuffle;
                    self.status_msg = if self.shuffle {
                        "Shuffle: on".into()
                    } else {
                        "Shuffle: off".into()
                    };
                }
                ApiCommand::CycleRepeat => {
                    self.repeat = match self.repeat {
                        RepeatMode::All => RepeatMode::One,
                        RepeatMode::One => RepeatMode::Off,
                        RepeatMode::Off => RepeatMode::All,
                    };
                    self.status_msg = format!(
                        "Repeat: {}",
                        match self.repeat {
                            RepeatMode::All => "all",
                            RepeatMode::One => "one",
                            RepeatMode::Off => "off",
                        }
                    );
                }
                ApiCommand::Stop => {
                    self.player.stop();
                }
                ApiCommand::Seek(secs) => {
                    self.player.seek_relative(secs);
                }
                ApiCommand::SetPosition(secs) => {
                    self.player.seek_absolute(secs);
                }
                ApiCommand::PlayTrack(api_track) => {
                    let track = Track {
                        id: api_track.id,
                        title: api_track.title.clone(),
                        duration: api_track.duration,
                        track_number: None,
                        artists: vec![Artist {
                            id: 0,
                            name: api_track.artist.clone(),
                        }],
                        album: Album {
                            id: 0,
                            title: api_track.album.clone(),
                        },
                        audio_quality: None,
                        explicit: None,
                    };
                    if !self.queue.iter().any(|t| t.id == track.id) {
                        Arc::make_mut(&mut self.queue).push(track.clone());
                    }
                    let qi = self
                        .queue
                        .iter()
                        .position(|t| t.id == track.id)
                        .unwrap_or(0);
                    self.stream_track_bg(track, qi);
                }
            },
        }
    }

    // ── Background tasks (simplificados — usan Arc<TidalDaemonClient>) ──────

    pub fn do_search_bg(&mut self) {
        if !self.authenticated {
            self.status_msg = self.lang.strings().status_login_required.to_string();
            return;
        }
        if self.search.input.is_empty() {
            return;
        }
        let query = self.search.input.clone();
        self.search.record_history(&query);
        self.search.loaded = [0; 4];
        self.search.exhausted = [false; 4];
        self.search_generation += 1;
        let generation = self.search_generation;
        self.loading = true;
        self.status_msg = self.lang.searching(&self.search.input);
        tasks::search(
            self.tidal.clone(),
            self.tx(),
            self.search.input.clone(),
            10,
            0,
            generation,
            None,
        );
    }

    /// Fetches the next page for the active search section and appends it.
    /// Only valid when browsing search content (not the sidebar/drill-down).
    pub fn load_more_bg(&mut self) {
        if !self.authenticated {
            self.status_msg = self.lang.strings().status_login_required_short.to_string();
            return;
        }
        if self.active_tab != Tab::Search
            || self.search.viewing.is_some()
            || self.search.focus != SearchFocus::Content
        {
            return;
        }
        if self.search.input.is_empty() {
            return;
        }
        let section = self.search.active_section;
        let idx = section as usize;
        if self.search.exhausted[idx] {
            self.status_msg = self.lang.strings().search_no_more_results.to_string();
            return;
        }
        let offset = self.search.loaded[idx];
        if offset >= 300 {
            self.search.exhausted[idx] = true;
            self.status_msg = self.lang.strings().search_no_more_results.to_string();
            return;
        }
        let query = self.search.input.clone();
        self.search_generation += 1;
        let generation = self.search_generation;
        self.loading = true;
        self.status_msg = self.lang.searching_more(&query);
        tasks::search(
            self.tidal.clone(),
            self.tx(),
            query,
            10,
            offset,
            generation,
            Some(section),
        );
    }

    pub fn add_selected_to_queue(&mut self) {
        if !self.authenticated {
            self.status_msg = self.lang.strings().status_login_required_short.to_string();
            return;
        }
        let track = match self.active_tab {
            Tab::Search => {
                if self.search.active_section != SearchSection::Tracks
                    || self.search.viewing.is_some()
                {
                    return;
                }
                self.search
                    .results
                    .tracks
                    .get(self.search.cursor[SearchSection::Tracks as usize])
                    .cloned()
            }
            Tab::Queue => self.queue.get(self.selected).cloned(),
            Tab::Now | Tab::Library => return,
        };
        let Some(track) = track else { return };
        if self.queue.iter().any(|t| t.id == track.id) {
            self.status_msg = self.lang.strings().status_already_in_queue.to_string();
            return;
        }
        Arc::make_mut(&mut self.queue).push(track);
        self.status_msg = self.lang.strings().status_added_to_queue.to_string();
    }

    pub fn remove_from_queue(&mut self, index: usize) {
        let playing_was_removed = self.queue_index == Some(index);
        let removed = Self::remove_queue_index(&mut self.queue, &mut self.queue_index, index);
        let Some(removed) = removed else { return };
        if playing_was_removed {
            if index < self.queue.len() {
                self.stream_track_bg(self.queue[index].clone(), index);
            } else if !self.queue.is_empty() {
                self.stream_track_bg(self.queue[0].clone(), 0);
            } else {
                self.player.stop();
                self.queue_index = None;
            }
        }
        self.status_msg = format!("Eliminada: {}", removed.title);
    }

    /// Remove a track from the queue and adjust `queue_index`.
    /// Returns the removed track, or `None` if `index` is out of bounds.
    fn remove_queue_index(
        queue: &mut Arc<Vec<Track>>,
        queue_index: &mut Option<usize>,
        index: usize,
    ) -> Option<Track> {
        if index >= queue.len() {
            return None;
        }
        let removed = queue[index].clone();
        Arc::make_mut(queue).remove(index);
        if let Some(i) = queue_index.as_mut() {
            if *i >= index {
                *i = i.saturating_sub(1);
            }
        }
        Some(removed)
    }

    pub fn play_selected_bg(&mut self) {
        if !self.authenticated {
            self.status_msg = self.lang.strings().status_login_required_short.to_string();
            return;
        }
        let track = match self.active_tab {
            Tab::Search => {
                if self.search.viewing.is_some() {
                    return;
                }
                self.search
                    .results
                    .tracks
                    .get(self.search.cursor[SearchSection::Tracks as usize])
                    .cloned()
            }
            Tab::Queue => self.queue.get(self.selected).cloned(),
            Tab::Now | Tab::Library => return,
        };
        let Some(track) = track else { return };
        let queue_index = if self.active_tab == Tab::Search
            && self.search.active_section == SearchSection::Tracks
        {
            // ponytail: append not replace — search results are loose items by
            // relevance, not a coherent sequence like a playlist/library view.
            // Library play_drilldown_track/play_fav_track replace queue instead.
            if !self.queue.iter().any(|t| t.id == track.id) {
                Arc::make_mut(&mut self.queue).push(track.clone());
            }
            self.queue
                .iter()
                .position(|t| t.id == track.id)
                .unwrap_or(0)
        } else {
            self.selected
        };
        self.stream_track_bg(track, queue_index);
    }

    pub fn play_next_bg(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let next = if self.repeat == RepeatMode::One {
            self.queue_index.unwrap_or(0)
        } else if self.shuffle {
            self.random_queue_index()
        } else {
            match self.queue_index {
                Some(i) if i + 1 < self.queue.len() => i + 1,
                _ => match self.repeat {
                    RepeatMode::Off => return,
                    _ => 0,
                },
            }
        };
        let track = self.queue[next].clone();
        self.stream_track_bg(track, next);
    }

    fn random_queue_index(&self) -> usize {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize;
        let seed = nanos ^ (self.player.elapsed.as_nanos() as usize);
        if self.queue.len() <= 1 {
            return 0;
        }
        let candidate = seed % self.queue.len();
        if Some(candidate) == self.queue_index {
            (candidate + 1) % self.queue.len()
        } else {
            candidate
        }
    }

    pub fn play_prev_bg(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let prev = match self.queue_index {
            Some(i) if i > 0 => i - 1,
            _ => self.queue.len().saturating_sub(1),
        };
        let track = self.queue[prev].clone();
        self.stream_track_bg(track, prev);
    }

    fn stream_track_bg(&mut self, track: Track, queue_index: usize) {
        self.auto_advance = false;
        self.stream_generation += 1;
        let generation = self.stream_generation;
        self.queue_index = Some(queue_index);
        self.loading = true;
        self.status_msg = self.lang.loading_stream(&track.title);
        self.cover_image = None;
        self.cover_info = None;
        self.cover_proto = None;
        self.lyrics = None;
        self.player.stop();
        let quality = self.quality.as_api_str().to_string();
        tasks::stream(
            self.tidal.clone(),
            self.tx(),
            quality,
            track,
            queue_index,
            generation,
        );
    }

    fn load_lyrics_bg(&mut self, track_id: u64) {
        tasks::lyrics(self.tidal.clone(), self.tx(), track_id);
    }

    fn load_cover_bg(&mut self, track_id: u64) {
        tasks::cover(self.tidal.clone(), self.tx(), track_id);
    }

    pub fn start_login_bg(&mut self) {
        self.loading = true;
        self.status_msg = self.lang.strings().status_starting_login.to_string();
        tasks::start_auth(self.tidal.clone(), self.tx());
    }

    pub fn poll_auth_bg(&mut self) {
        tasks::poll_auth(self.tidal.clone(), self.tx());
    }

    pub fn load_library_bg(&mut self) {
        if !self.authenticated {
            return;
        }
        self.loading = true;
        self.status_msg = self.lang.strings().status_loading_lib.to_string();
        tasks::library(self.tidal.clone(), self.tx());
    }

    pub fn load_playlist_tracks_bg(&mut self, uuid: String) {
        self.loading = true;
        self.status_msg = self.lang.strings().status_loading_playlist.to_string();
        tasks::playlist_tracks(self.tidal.clone(), self.tx(), uuid);
    }

    pub fn load_mix_tracks_bg(&mut self, mix_id: String) {
        self.loading = true;
        self.status_msg = self.lang.strings().status_loading_mix.to_string();
        tasks::mix_tracks(self.tidal.clone(), self.tx(), mix_id);
    }

    pub fn library_select_enter(&mut self) {
        match self.library.active_section {
            LibrarySection::Playlists => {
                let cursor = self.library.cursor[LibrarySection::Playlists as usize];
                if let Some(playlists) = &self.library.playlists {
                    if let Some(p) = playlists.get(cursor) {
                        self.library.pending_viewing_title = p.title.clone();
                        self.load_playlist_tracks_bg(p.uuid.clone());
                    }
                }
            }
            LibrarySection::Mixes => {
                let cursor = self.library.cursor[LibrarySection::Mixes as usize];
                if let Some(mixes) = &self.library.mixes {
                    if let Some(m) = mixes.get(cursor) {
                        self.library.pending_viewing_title = m.title.clone();
                        self.load_mix_tracks_bg(m.id.clone());
                    }
                }
            }
            LibrarySection::FavTracks => {
                // ponytail: Enter on individual track handled by play_fav_track
            }
            LibrarySection::FavAlbums => {
                let cursor = self.library.cursor[LibrarySection::FavAlbums as usize];
                if let Some(albums) = &self.library.fav_albums {
                    if let Some(a) = albums.get(cursor) {
                        self.library.pending_viewing_title = a.title.clone();
                        self.load_album_tracks_bg(a.id, a.title.clone());
                    }
                }
            }
        }
    }

    pub fn play_drilldown_track(&mut self) {
        let (track, remaining) = if let Some(ref v) = self.search.viewing {
            match v.tracks.get(v.cursor) {
                Some(t) => (t.clone(), v.tracks[v.cursor..].to_vec()),
                None => return,
            }
        } else if let Some(ref v) = self.library.viewing {
            match v.tracks.get(v.cursor) {
                Some(t) => (t.clone(), v.tracks[v.cursor..].to_vec()),
                None => return,
            }
        } else {
            return;
        };
        self.queue = Arc::new(remaining);
        self.stream_track_bg(track, 0);
    }

    pub fn play_fav_track(&mut self) {
        let cursor = self.library.cursor[LibrarySection::FavTracks as usize];
        let (track, remaining) = match self.library.fav_tracks.as_ref() {
            Some(tracks) => match tracks.get(cursor) {
                Some(t) => (t.clone(), tracks[cursor..].to_vec()),
                None => return,
            },
            None => return,
        };
        self.queue = Arc::new(remaining);
        self.stream_track_bg(track, 0);
    }

    pub fn add_current_track_to_queue(&mut self) {
        let track = if let Some(ref viewing) = self.search.viewing {
            match viewing.tracks.get(viewing.cursor) {
                Some(t) => t.clone(),
                None => return,
            }
        } else if let Some(ref viewing) = self.library.viewing {
            match viewing.tracks.get(viewing.cursor) {
                Some(t) => t.clone(),
                None => return,
            }
        } else if self.library.active_section == LibrarySection::FavTracks {
            let cursor = self.library.cursor[LibrarySection::FavTracks as usize];
            match self.library.fav_tracks.as_ref().and_then(|t| t.get(cursor)) {
                Some(t) => t.clone(),
                None => return,
            }
        } else {
            return;
        };
        if self.queue.iter().any(|t| t.id == track.id) {
            self.status_msg = self.lang.strings().status_already_in_queue.to_string();
            return;
        }
        Arc::make_mut(&mut self.queue).push(track.clone());
        self.status_msg = format!("▶ Añadida: {}", track.title);
    }

    pub fn add_all_tracks_to_queue(&mut self) {
        let tracks: Vec<Track> = if let Some(ref viewing) = self.search.viewing {
            viewing.tracks.clone()
        } else if let Some(ref viewing) = self.library.viewing {
            viewing.tracks.clone()
        } else if self.library.active_section == LibrarySection::FavTracks {
            match self.library.fav_tracks.as_ref() {
                Some(t) => t.clone(),
                None => return,
            }
        } else {
            return;
        };
        let mut added = 0usize;
        for track in &tracks {
            if !self.queue.iter().any(|t| t.id == track.id) {
                Arc::make_mut(&mut self.queue).push(track.clone());
                added += 1;
            }
        }
        self.status_msg = format!("▶ {added} canciones añadidas a la cola");
    }

    pub fn set_quality(&mut self, q: Quality) {
        self.quality = q;
        self.status_msg = self.lang.quality_changed(q.label());
        tasks::set_quality(self.tidal.clone(), q.as_api_str().to_string());
        self.save_settings();
    }

    pub fn next_tab(&mut self) {
        let entering_library = self.active_tab == Tab::Now;
        self.active_tab = match self.active_tab {
            Tab::Search => Tab::Queue,
            Tab::Queue => Tab::Now,
            Tab::Now => Tab::Library,
            Tab::Library => {
                self.library.active_section = LibrarySection::Playlists;
                self.library.focus = LibraryFocus::Sidebar;
                Tab::Search
            }
        };
        if entering_library {
            self.ensure_section_loaded();
        }
        self.selected = 0;
    }

    pub fn current_list_len(&self) -> usize {
        match self.active_tab {
            Tab::Search => {
                if self.search.viewing.is_some() {
                    self.search.viewing.as_ref().map_or(0, |v| v.tracks.len())
                } else {
                    self.search.current_len()
                }
            }
            Tab::Queue => self.queue.len(),
            Tab::Now => 0,
            Tab::Library => self.section_len(self.library.active_section),
        }
    }

    pub fn section_len(&self, section: LibrarySection) -> usize {
        match section {
            LibrarySection::Playlists => self.library.playlists.as_ref().map_or(0, |p| p.len()),
            LibrarySection::Mixes => self.library.mixes.as_ref().map_or(0, |m| m.len()),
            LibrarySection::FavTracks => self.library.fav_tracks.as_ref().map_or(0, |t| t.len()),
            LibrarySection::FavAlbums => self.library.fav_albums.as_ref().map_or(0, |a| a.len()),
        }
    }

    pub fn ensure_section_loaded(&mut self) {
        match self.library.active_section {
            LibrarySection::Playlists | LibrarySection::Mixes => {
                if self.library.playlists.is_none() && self.library.mixes.is_none() {
                    self.load_library_bg();
                }
            }
            LibrarySection::FavTracks => {
                if self.library.fav_tracks.is_none() {
                    self.load_fav_tracks_bg();
                }
            }
            LibrarySection::FavAlbums => {
                if self.library.fav_albums.is_none() {
                    self.load_fav_albums_bg();
                }
            }
        }
    }

    pub fn next_track(&mut self) {
        if self.active_tab == Tab::Search {
            if let Some(ref mut v) = self.search.viewing {
                let max = v.tracks.len();
                if max > 0 {
                    v.cursor = (v.cursor + 1) % max;
                }
            } else if self.search.focus == SearchFocus::Sidebar {
                self.search.active_section = self.search.active_section.next();
            } else {
                let section = self.search.active_section as usize;
                let max = self.search.current_len();
                if max > 0 {
                    self.search.cursor[section] = (self.search.cursor[section] + 1) % max;
                }
            }
            return;
        }
        let len = self.current_list_len();
        if len > 0 {
            self.selected = (self.selected + 1) % len;
        }
    }

    pub fn prev_track(&mut self) {
        if self.active_tab == Tab::Search {
            if let Some(ref mut v) = self.search.viewing {
                let max = v.tracks.len();
                if max > 0 {
                    v.cursor = if v.cursor == 0 { max - 1 } else { v.cursor - 1 };
                }
            } else if self.search.focus == SearchFocus::Sidebar {
                self.search.active_section = self.search.active_section.prev();
            } else {
                let section = self.search.active_section as usize;
                let max = self.search.current_len();
                if max > 0 {
                    self.search.cursor[section] = if self.search.cursor[section] == 0 {
                        max - 1
                    } else {
                        self.search.cursor[section] - 1
                    };
                }
            }
            return;
        }
        let len = self.current_list_len();
        if len > 0 {
            self.selected = if self.selected == 0 {
                len - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn load_fav_tracks_bg(&mut self) {
        if !self.authenticated {
            return;
        }
        self.loading = true;
        self.status_msg = self.lang.strings().status_loading_fav_tracks.to_string();
        tasks::fav_tracks(self.tidal.clone(), self.tx());
    }

    pub fn load_fav_albums_bg(&mut self) {
        if !self.authenticated {
            return;
        }
        self.loading = true;
        self.status_msg = self.lang.strings().status_loading_fav_albums.to_string();
        tasks::fav_albums(self.tidal.clone(), self.tx());
    }

    pub fn api_status_snapshot(&self) -> ApiStatus {
        use crate::player::PlayerState;
        let state = match self.player.state {
            PlayerState::Playing => "playing",
            PlayerState::Paused => "paused",
            PlayerState::Stopped => "stopped",
        }
        .to_string();
        let (title, artist, album, duration, bit_depth, sample_rate, codec) = self
            .player
            .current
            .as_ref()
            .map(|ti| {
                (
                    Some(ti.title.clone()),
                    Some(ti.artist.clone()),
                    Some(ti.album.clone()),
                    Some(ti.duration),
                    Some(ti.bit_depth),
                    Some(ti.sample_rate),
                    Some(ti.codec.clone()),
                )
            })
            .unwrap_or_default();
        ApiStatus {
            state,
            title,
            artist,
            album,
            duration,
            elapsed: self.player.elapsed.as_secs(),
            volume: self.player.volume,
            progress: self.player.progress(),
            bit_depth,
            sample_rate,
            codec,
            track_id: self.current_track_id,
            shuffle: self.shuffle,
            repeat: self.repeat.clone(),
            authenticated: self.authenticated,
            queue: self.queue.clone(),
            queue_index: self.queue_index,
        }
    }

    pub fn load_album_tracks_bg(&mut self, album_id: u64, album_title: String) {
        self.loading = true;
        self.status_msg = self.lang.loading_album(&album_title);
        tasks::album_tracks(self.tidal.clone(), self.tx(), album_id);
    }

    fn save_settings(&self) {
        let settings = Settings {
            lang: self.lang,
            quality: self.quality,
            volume: self.player.volume,
        };
        settings.save();
    }

    pub fn load_settings(&mut self) {
        if let Some(settings) = Settings::load() {
            self.lang = settings.lang;
            self.quality = settings.quality;
            self.player.volume = settings.volume.min(100);
        }
    }

    pub fn volume_up(&mut self) {
        self.player.volume_up();
        self.save_settings();
    }

    pub fn volume_down(&mut self) {
        self.player.volume_down();
        self.save_settings();
    }

    pub fn set_volume(&mut self, v: u8) {
        self.player.set_volume(v);
        self.save_settings();
    }

    pub fn clear_search(&mut self) {
        if self.active_tab != Tab::Search {
            return;
        }
        self.search = SearchState::new();
        self.input_mode = InputMode::Normal;
        self.status_msg = self.lang.strings().status_search_cleared.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_track(id: u64, title: &str) -> Track {
        Track {
            id,
            title: title.to_string(),
            duration: 200,
            track_number: None,
            artists: vec![],
            album: Album {
                id: 0,
                title: String::new(),
            },
            audio_quality: None,
            explicit: None,
        }
    }

    #[test]
    fn test_remove_queue_index_after_current() {
        let mut queue: Arc<Vec<Track>> = Arc::new(vec![
            make_track(1, "A"),
            make_track(2, "B"),
            make_track(3, "C"),
        ]);
        let mut queue_index = Some(1);
        let removed = App::remove_queue_index(&mut queue, &mut queue_index, 2);
        assert_eq!(removed.unwrap().id, 3);
        assert_eq!(queue.len(), 2);
        assert_eq!(queue_index, Some(1));
    }

    #[test]
    fn test_remove_queue_index_before_current() {
        let mut queue: Arc<Vec<Track>> = Arc::new(vec![
            make_track(1, "A"),
            make_track(2, "B"),
            make_track(3, "C"),
        ]);
        let mut queue_index = Some(2);
        let removed = App::remove_queue_index(&mut queue, &mut queue_index, 0);
        assert_eq!(removed.unwrap().id, 1);
        assert_eq!(queue.len(), 2);
        assert_eq!(queue_index, Some(1));
    }

    #[test]
    fn test_remove_queue_index_current_is_first() {
        let mut queue: Arc<Vec<Track>> = Arc::new(vec![make_track(1, "A"), make_track(2, "B")]);
        let mut queue_index = Some(0);
        let removed = App::remove_queue_index(&mut queue, &mut queue_index, 0);
        assert_eq!(removed.unwrap().id, 1);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue_index, Some(0));
    }

    #[test]
    fn test_remove_queue_index_out_of_bounds() {
        let mut queue: Arc<Vec<Track>> = Arc::new(vec![make_track(1, "A")]);
        let mut queue_index = Some(0);
        let removed = App::remove_queue_index(&mut queue, &mut queue_index, 5);
        assert!(removed.is_none());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue_index, Some(0));
    }

    #[test]
    fn test_remove_queue_index_empty_queue() {
        let mut queue: Arc<Vec<Track>> = Arc::new(vec![]);
        let mut queue_index = None;
        let removed = App::remove_queue_index(&mut queue, &mut queue_index, 0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_search_history_caps_at_10() {
        let mut state = SearchState::new();
        for i in 0..12 {
            state.record_history(&format!("query{i}"));
        }
        assert_eq!(state.history.len(), 10);
        assert_eq!(state.history[0], "query11");
        assert_eq!(state.history[9], "query2");
        assert!(!state.history.iter().any(|h| h == "query0"));
        assert!(!state.history.iter().any(|h| h == "query1"));
    }

    #[test]
    fn test_search_history_moves_duplicate_to_front() {
        let mut state = SearchState::new();
        for i in 0..5 {
            state.record_history(&format!("query{i}"));
        }
        // history: q4, q3, q2, q1, q0 — repeating q2 moves it to front
        state.record_history("query2");
        assert_eq!(state.history.len(), 5);
        assert_eq!(state.history[0], "query2");
        assert_eq!(state.history[1], "query4");
        assert_eq!(state.history[2], "query3");
        assert_eq!(state.history[3], "query1");
        assert_eq!(state.history[4], "query0");
    }

    #[test]
    fn test_search_history_navigation_up_down() {
        let mut state = SearchState::new();
        for i in 0..3 {
            state.record_history(&format!("query{i}"));
        }
        // history: q2, q1, q0
        state.input = "draft text".to_string();
        assert_eq!(state.history_cursor, None);

        // ↑ once → most recent
        state.history_up();
        assert_eq!(state.history_cursor, Some(0));
        assert_eq!(state.input, "query2");
        assert_eq!(state.draft, "draft text");

        // ↑ again → older
        state.history_up();
        assert_eq!(state.history_cursor, Some(1));
        assert_eq!(state.input, "query1");

        // ↑ again → oldest
        state.history_up();
        assert_eq!(state.history_cursor, Some(2));
        assert_eq!(state.input, "query0");

        // ↑ at oldest → no change
        state.history_up();
        assert_eq!(state.history_cursor, Some(2));
        assert_eq!(state.input, "query0");

        // ↓ back one
        state.history_down();
        assert_eq!(state.history_cursor, Some(1));
        assert_eq!(state.input, "query1");

        // ↓ back to most recent
        state.history_down();
        assert_eq!(state.history_cursor, Some(0));
        assert_eq!(state.input, "query2");

        // ↓ at 0 → restore draft and exit navigation
        state.history_down();
        assert_eq!(state.history_cursor, None);
        assert_eq!(state.input, "draft text");
    }

    #[test]
    fn test_search_generation_discards_stale() {
        let mut app = make_app_for_test();
        app.search_generation = 5;

        // Stale result (generation 3 < 5) should be silently ignored
        let stale = SearchResults {
            tracks: vec![make_track(99, "Stale")],
            ..SearchResults::default()
        };
        app.handle_event(AppEvent::SearchDone(Ok(stale), 3));
        assert_eq!(app.search.results.tracks.len(), 0);
        assert_eq!(app.search.results.albums.len(), 0);

        // Current generation result (5) should update state
        let current = SearchResults {
            tracks: vec![make_track(1, "Current")],
            ..SearchResults::default()
        };
        app.handle_event(AppEvent::SearchDone(Ok(current), 5));
        assert_eq!(app.search.results.tracks.len(), 1);
        assert_eq!(app.search.results.tracks[0].title, "Current");

        // Stale error should also be ignored
        app.handle_event(AppEvent::SearchDone(Err("stale error".into()), 4));
        // Status message should still reflect the last successful search
        assert_eq!(app.status_msg, "1 resultados");
    }

    fn make_app_for_test() -> App {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tidal = rt.block_on(async {
            crate::tidal::TidalDaemonClient::spawn("tidal.py", "python3", "LOSSLESS")
                .await
                .expect("tidal.py must exist for this test")
        });
        App::new(tidal)
    }

    #[test]
    fn test_search_persists_across_tab_switch() {
        let mut app = make_app_for_test();
        app.search.input = "radiohead".to_string();
        app.search.results = SearchResults {
            tracks: vec![make_track(1, "Karma Police")],
            ..SearchResults::default()
        };
        app.search.active_section = SearchSection::Albums;
        app.search.cursor = [3, 7, 0, 0];
        app.search.focus = SearchFocus::Content;

        // Cycle through all tabs and back to Search
        app.next_tab(); // Queue
        app.next_tab(); // Now
        app.next_tab(); // Library
        app.next_tab(); // back to Search

        assert_eq!(app.search.input, "radiohead");
        assert_eq!(app.search.results.tracks.len(), 1);
        assert_eq!(app.search.results.tracks[0].title, "Karma Police");
        assert_eq!(app.search.active_section, SearchSection::Albums);
        assert_eq!(app.search.cursor, [3, 7, 0, 0]);
        assert_eq!(app.search.focus, SearchFocus::Content);
    }

    #[test]
    fn test_clear_search_resets_state() {
        let mut app = make_app_for_test();
        app.active_tab = Tab::Search;
        app.search.input = "radiohead".to_string();
        app.search.results = SearchResults {
            tracks: vec![make_track(1, "Karma Police")],
            ..SearchResults::default()
        };
        app.search.cursor = [3, 2, 1, 0];
        app.search.focus = SearchFocus::Content;
        app.search.active_section = SearchSection::Artists;

        app.clear_search();

        assert!(app.search.input.is_empty());
        assert_eq!(app.search.results.tracks.len(), 0);
        assert_eq!(app.search.results.albums.len(), 0);
        assert_eq!(app.search.results.artists.len(), 0);
        assert_eq!(app.search.results.playlists.len(), 0);
        assert_eq!(app.search.cursor, [0; 4]);
        assert_eq!(app.search.focus, SearchFocus::Sidebar);
        assert_eq!(app.search.active_section, SearchSection::Tracks);
        assert!(app.search.viewing.is_none());
        assert_eq!(app.status_msg, "Búsqueda limpiada");
    }

    #[test]
    fn test_search_more_appends_and_dedups() {
        let mut app = make_app_for_test();
        app.search_generation = 5;
        app.search.active_section = SearchSection::Tracks;
        app.search.loaded = [10, 0, 0, 0];
        app.search.results = SearchResults {
            tracks: vec![make_track(1, "A"), make_track(2, "B")],
            ..SearchResults::default()
        };
        // full page: duplicate "B" + 9 new tracks (ids 3..=11)
        let mut page2 = vec![make_track(2, "B")];
        for id in 3..=11 {
            page2.push(make_track(id, &format!("T{id}")));
        }
        let page2 = SearchResults {
            tracks: page2,
            ..SearchResults::default()
        };
        app.handle_event(AppEvent::SearchMoreDone(
            Ok(page2),
            5,
            SearchSection::Tracks,
        ));

        assert_eq!(app.search.results.tracks.len(), 11);
        assert_eq!(app.search.results.tracks[2].title, "T3");
        // duplicate "B" not appended → only 9 new items on top of the 10 already loaded
        assert_eq!(app.search.loaded[0], 19);
        assert!(!app.search.exhausted[0]);
    }

    #[test]
    fn test_search_more_marks_exhausted_on_short_page() {
        let mut app = make_app_for_test();
        app.search_generation = 5;
        app.search.loaded = [10, 0, 0, 0];
        let page = SearchResults {
            tracks: vec![make_track(3, "C"), make_track(4, "D"), make_track(5, "E")],
            ..SearchResults::default()
        };
        app.handle_event(AppEvent::SearchMoreDone(Ok(page), 5, SearchSection::Tracks));

        assert_eq!(app.search.results.tracks.len(), 3);
        assert_eq!(app.search.loaded[0], 13);
        assert!(app.search.exhausted[0]);
    }

    #[test]
    fn test_search_more_stale_discarded() {
        let mut app = make_app_for_test();
        app.search_generation = 5;
        app.search.results = SearchResults {
            tracks: vec![make_track(1, "A")],
            ..SearchResults::default()
        };
        app.handle_event(AppEvent::SearchMoreDone(
            Ok(SearchResults {
                tracks: vec![make_track(9, "Stale")],
                ..SearchResults::default()
            }),
            3,
            SearchSection::Tracks,
        ));
        assert_eq!(app.search.results.tracks.len(), 1);
        assert_eq!(app.search.results.tracks[0].title, "A");
        assert_eq!(app.search.loaded[0], 0);
    }

    #[test]
    fn test_search_done_sets_loaded_and_exhausted() {
        let mut app = make_app_for_test();
        app.search_generation = 1;
        let results = SearchResults {
            tracks: vec![make_track(1, "A"), make_track(2, "B")],
            artists: vec![Artist {
                id: 1,
                name: "X".to_string(),
            }],
            ..SearchResults::default()
        };
        app.handle_event(AppEvent::SearchDone(Ok(results), 1));

        assert_eq!(app.search.loaded, [2, 0, 1, 0]);
        // every section returned fewer than a full page → all exhausted
        assert_eq!(app.search.exhausted, [true, true, true, true]);
    }

    #[test]
    fn test_load_more_bg_guards() {
        let mut app = make_app_for_test();

        // not authenticated → status message, no request
        app.load_more_bg();
        assert_eq!(
            app.status_msg,
            app.lang.strings().status_login_required_short.to_string()
        );
        assert_eq!(app.search_generation, 0);

        app.authenticated = true;
        app.search.input = "radiohead".to_string();

        // sidebar focus → no-op
        app.search.focus = SearchFocus::Sidebar;
        app.status_msg.clear();
        app.load_more_bg();
        assert_eq!(app.status_msg, "");
        assert_eq!(app.search_generation, 0);

        // content focus but empty input → no-op
        app.search.focus = SearchFocus::Content;
        app.search.input.clear();
        app.status_msg.clear();
        app.load_more_bg();
        assert_eq!(app.status_msg, "");
        assert_eq!(app.search_generation, 0);

        // exhausted section → status message, no request
        app.search.input = "radiohead".to_string();
        app.search.loaded[0] = 10;
        app.search.exhausted[0] = true;
        app.load_more_bg();
        assert_eq!(
            app.status_msg,
            app.lang.strings().search_no_more_results.to_string()
        );
        assert_eq!(app.search_generation, 0);
    }

    #[test]
    fn test_clear_search_ignored_on_other_tab() {
        let mut app = make_app_for_test();
        app.active_tab = Tab::Queue;
        app.search.input = "radiohead".to_string();
        app.search.results = SearchResults {
            tracks: vec![make_track(1, "Karma Police")],
            ..SearchResults::default()
        };
        app.search.cursor = [3, 2, 1, 0];
        app.search.focus = SearchFocus::Content;
        app.search.active_section = SearchSection::Artists;

        app.clear_search();

        assert_eq!(app.search.input, "radiohead");
        assert_eq!(app.search.results.tracks.len(), 1);
        assert_eq!(app.search.cursor, [3, 2, 1, 0]);
        assert_eq!(app.search.focus, SearchFocus::Content);
        assert_eq!(app.search.active_section, SearchSection::Artists);
        assert_ne!(app.status_msg, "Búsqueda limpiada");
    }
}
