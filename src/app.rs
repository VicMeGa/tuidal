use crate::i18n::Lang;
use crate::library::{LibraryFocus, LibrarySection, LibraryState, LibraryViewing};
use crate::player::{Player, TrackInfo};
use crate::settings::Settings;
use crate::tasks;
use crate::tidal::{
    Album, Artist, CoverInfo, FavAlbum, Lyrics, Mix, Playlist, Quality, StreamInfo,
    TidalDaemonClient, Track,
};
use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use serde::Deserialize as DeserializeAttr;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

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
    SearchDone(Result<Vec<Track>, String>),
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

    pub search_input: String,
    pub search_results: Vec<Track>,
    pub queue: Arc<Vec<Track>>,

    pub selected: usize,
    pub queue_index: Option<usize>,

    pub authenticated: bool,
    pub status_msg: String,
    pub loading: bool,
    pub auto_advance: bool,
    pub should_quit: bool,

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
            search_input: String::new(),
            search_results: Vec::new(),
            queue: Arc::new(Vec::new()),
            selected: 0,
            queue_index: None,
            authenticated: false,
            status_msg: String::new(),
            loading: false,
            auto_advance: false,
            should_quit: false,
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
            library: LibraryState::new(),

            lang: Lang::Es,
            quality: Quality::Lossless,
            shuffle: false,
            repeat: RepeatMode::All,
            lyrics: None,
        }
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
            AppEvent::SearchDone(Ok(results)) => {
                self.status_msg = if results.is_empty() {
                    self.lang.strings().status_no_results.to_string()
                } else {
                    self.lang.results_count(results.len())
                };
                self.search_results = results;
                self.selected = 0;
                self.active_tab = Tab::Search;
                self.loading = false;
            }
            AppEvent::SearchDone(Err(e)) => {
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
                let title = std::mem::take(&mut self.library.pending_viewing_title);
                self.library.viewing = Some(LibraryViewing {
                    title,
                    tracks,
                    cursor: 0,
                });
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
        if self.search_input.is_empty() {
            return;
        }
        self.loading = true;
        self.status_msg = self.lang.searching(&self.search_input);
        tasks::search(self.tidal.clone(), self.tx(), self.search_input.clone());
    }

    pub fn add_selected_to_queue(&mut self) {
        if !self.authenticated {
            self.status_msg = self.lang.strings().status_login_required_short.to_string();
            return;
        }
        let track = match self.active_tab {
            Tab::Search => self.search_results.get(self.selected).cloned(),
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
            Tab::Search => self.search_results.get(self.selected).cloned(),
            Tab::Queue => self.queue.get(self.selected).cloned(),
            Tab::Now => return,
            Tab::Library => return,
        };
        let Some(track) = track else { return };
        let queue_index = if self.active_tab == Tab::Search {
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
        let (track, remaining) = match self.library.viewing.as_ref() {
            Some(v) => match v.tracks.get(v.cursor) {
                Some(t) => (t.clone(), v.tracks[v.cursor..].to_vec()),
                None => return,
            },
            None => return,
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
        let track = if let Some(ref viewing) = self.library.viewing {
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
        let tracks: Vec<Track> = if let Some(ref viewing) = self.library.viewing {
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
            Tab::Search => self.search_results.len(),
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
        let len = self.current_list_len();
        if len > 0 {
            self.selected = (self.selected + 1) % len;
        }
    }

    pub fn prev_track(&mut self) {
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
}
