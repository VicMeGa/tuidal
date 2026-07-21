use crate::tidal::{FavAlbum, Mix, Playlist, Track};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LibraryFocus {
    Sidebar,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum LibrarySection {
    Playlists = 0,
    Mixes = 1,
    FavTracks = 2,
    FavAlbums = 3,
}

impl LibrarySection {
    pub fn next(self) -> Self {
        match self {
            LibrarySection::Playlists => LibrarySection::Mixes,
            LibrarySection::Mixes => LibrarySection::FavTracks,
            LibrarySection::FavTracks => LibrarySection::FavAlbums,
            LibrarySection::FavAlbums => LibrarySection::Playlists,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            LibrarySection::Playlists => LibrarySection::FavAlbums,
            LibrarySection::Mixes => LibrarySection::Playlists,
            LibrarySection::FavTracks => LibrarySection::Mixes,
            LibrarySection::FavAlbums => LibrarySection::FavTracks,
        }
    }
}

pub struct LibraryViewing {
    pub title: String,
    pub tracks: Vec<Track>,
    pub cursor: usize,
}

pub struct LibraryState {
    pub focus: LibraryFocus,
    pub active_section: LibrarySection,
    pub cursor: [usize; 4],
    pub playlists: Option<Vec<Playlist>>,
    pub mixes: Option<Vec<Mix>>,
    pub fav_tracks: Option<Vec<Track>>,
    pub fav_albums: Option<Vec<FavAlbum>>,
    pub viewing: Option<LibraryViewing>,
    pub pending_viewing_title: String,
}

impl LibraryState {
    pub fn new() -> Self {
        Self {
            focus: LibraryFocus::Sidebar,
            active_section: LibrarySection::Playlists,
            cursor: [0; 4],
            playlists: None,
            mixes: None,
            fav_tracks: None,
            fav_albums: None,
            viewing: None,
            pending_viewing_title: String::new(),
        }
    }
}
