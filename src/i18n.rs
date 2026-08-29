#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Es,
    En,
    De,
    Ro,
}

pub struct Strings {
    // Tabs
    pub tab_search: &'static str,
    pub tab_queue: &'static str,
    pub tab_now: &'static str,
    pub tab_library: &'static str,
    // Search
    pub search_placeholder: &'static str,
    pub search_results_title: &'static str,
    pub search_sidebar_tracks: &'static str,
    pub search_sidebar_albums: &'static str,
    pub search_sidebar_artists: &'static str,
    pub search_sidebar_playlists: &'static str,
    pub search_no_tracks: &'static str,
    pub search_no_albums: &'static str,
    pub search_no_artists: &'static str,
    pub search_no_playlists: &'static str,
    pub search_load_more_hint: &'static str,
    pub search_no_more_results: &'static str,
    // Queue
    pub queue_title: &'static str,
    // Now playing
    pub now_playing_empty: &'static str,
    pub now_playing_title: &'static str,
    pub now_lyrics: &'static str,
    pub now_no_lyrics: &'static str,
    pub loading_image: &'static str,
    // Track list
    pub loading: &'static str,
    pub not_authenticated: &'static str,
    pub no_results_hint: &'static str,
    pub col_title: &'static str,
    pub col_artist: &'static str,
    pub col_album: &'static str,
    pub col_duration: &'static str,
    // Player bar
    pub player_stopped: &'static str,
    // Hint bar
    pub hint_play: &'static str,
    pub hint_pause: &'static str,
    pub hint_next_prev: &'static str,
    pub hint_seek: &'static str,
    pub hint_volume: &'static str,
    pub hint_view: &'static str,
    pub hint_quality: &'static str,
    pub hint_quit: &'static str,
    pub hint_library: &'static str,
    pub hint_fav_tracks: &'static str,
    pub hint_fav_albums: &'static str,
    pub hint_lang: &'static str,
    pub hint_add_queue: &'static str,
    pub hint_remove_queue: &'static str,
    pub hint_search_clear: &'static str,
    // Library sidebar sections
    pub lib_playlists: &'static str,
    pub lib_mixes: &'static str,
    pub lib_fav_tracks: &'static str,
    pub lib_fav_albums: &'static str,
    // Library
    pub library_title: &'static str,
    // Fav tracks
    pub fav_tracks_empty: &'static str,
    // Fav albums
    pub fav_albums_empty: &'static str,
    pub fav_albums_title: &'static str,
    pub col_tracks: &'static str,
    // Login overlay
    pub login_title_text: &'static str,
    pub login_open_url: &'static str,
    pub login_code_prefix: &'static str,
    pub login_waiting: &'static str,
    pub login_overlay_title: &'static str,
    // Status messages (static)
    pub status_no_results: &'static str,
    pub status_auth_done: &'static str,
    pub status_login_required: &'static str,
    pub status_login_required_short: &'static str,
    pub status_starting_login: &'static str,
    pub status_loading_lib: &'static str,
    pub status_loading_playlist: &'static str,
    pub status_loading_mix: &'static str,
    pub status_loading_fav_tracks: &'static str,
    pub status_loading_fav_albums: &'static str,
    pub status_session_loading: &'static str,
    pub status_session_active: &'static str,
    pub status_press_l: &'static str,
    pub status_added_to_queue: &'static str,
    pub status_already_in_queue: &'static str,
    pub status_search_cleared: &'static str,
}

static ES: Strings = Strings {
    tab_search: "Buscar",
    tab_queue: "Cola",
    tab_now: "Ahora",
    tab_library: "Biblioteca",
    search_placeholder: "Presiona / para buscar...",
    search_results_title: "Resultados",
    search_sidebar_tracks: "Canciones",
    search_sidebar_albums: "Álbumes",
    search_sidebar_artists: "Artistas",
    search_sidebar_playlists: "Listas",
    search_no_tracks: "  Sin canciones",
    search_no_albums: "  Sin álbumes",
    search_no_artists: "  Sin artistas",
    search_no_playlists: "  Sin listas",
    search_load_more_hint: "Cargar más",
    search_no_more_results: "Sin más resultados",
    queue_title: "Cola de reproducción",
    now_playing_empty: "Sin reproducción — presiona Enter en una canción",
    now_playing_title: "◈ Ahora reproduciendo",
    now_lyrics: "Letras",
    now_no_lyrics: "Letras no disponibles",
    loading_image: "⟳ Cargando\n  imagen...",
    loading: "  ⟳ Cargando...",
    not_authenticated: "  Presiona L para iniciar sesión en Tidal",
    no_results_hint: "  Sin resultados — busca con /",
    col_title: "  Título",
    col_artist: "Artista",
    col_album: "Álbum",
    col_duration: "Dur.",
    player_stopped: "Sin reproducción",
    hint_play: "reproducir",
    hint_pause: "pausa",
    hint_next_prev: "sig/ant",
    hint_seek: "seek",
    hint_volume: "volumen",
    hint_view: "vista",
    hint_quality: "calidad",
    hint_quit: "salir",
    hint_library: "biblioteca",
    hint_fav_tracks: "favoritos",
    hint_fav_albums: "álbumes fav",
    hint_lang: "idioma",
    hint_add_queue: "añadir cola",
    hint_remove_queue: "eliminar",
    hint_search_clear: "limpiar búsqueda",
    lib_playlists: "Listas",
    lib_mixes: "Mixes",
    lib_fav_tracks: "Favoritos",
    lib_fav_albums: "Álbumes Fav",
    library_title: "Biblioteca",
    fav_tracks_empty: "  Sin canciones favoritas",
    fav_albums_empty: "  Sin álbumes favoritos",
    fav_albums_title: "Álbumes favoritos",
    col_tracks: "Tracks",
    login_title_text: "  Inicia sesión en Tidal",
    login_open_url: "  1. Abre este URL:",
    login_code_prefix: "  2. Código: ",
    login_waiting: "  Esperando autorización...",
    login_overlay_title: " ◈ Autenticación ",
    status_no_results: "Sin resultados",
    status_auth_done: "✓ Autenticado con Tidal",
    status_login_required: "Primero inicia sesión con 'L'",
    status_login_required_short: "Inicia sesión primero (L)",
    status_starting_login: "Iniciando login...",
    status_loading_lib: "⟳ Cargando biblioteca...",
    status_loading_playlist: "⟳ Cargando playlist...",
    status_loading_mix: "⟳ Cargando mix...",
    status_loading_fav_tracks: "⟳ Cargando canciones favoritas...",
    status_loading_fav_albums: "⟳ Cargando álbumes favoritos...",
    status_session_loading: "Cargando sesión...",
    status_session_active: "✓ Sesión activa",
    status_press_l: "Presiona 'L' para iniciar sesión en Tidal",
    status_added_to_queue: "▶ Añadido a la cola",
    status_already_in_queue: "Ya está en la cola",
    status_search_cleared: "Búsqueda limpiada",
};

static EN: Strings = Strings {
    tab_search: "Search",
    tab_queue: "Queue",
    tab_now: "Now",
    tab_library: "Library",
    search_placeholder: "Press / to search...",
    search_results_title: "Results",
    search_sidebar_tracks: "Tracks",
    search_sidebar_albums: "Albums",
    search_sidebar_artists: "Artists",
    search_sidebar_playlists: "Playlists",
    search_no_tracks: "  No tracks",
    search_no_albums: "  No albums",
    search_no_artists: "  No artists",
    search_no_playlists: "  No playlists",
    search_load_more_hint: "Load more",
    search_no_more_results: "No more results",
    queue_title: "Playback Queue",
    now_playing_empty: "Nothing playing — press Enter on a track",
    now_playing_title: "◈ Now Playing",
    now_lyrics: "Lyrics",
    now_no_lyrics: "Lyrics not available",
    loading_image: "⟳ Loading\n  image...",
    loading: "  ⟳ Loading...",
    not_authenticated: "  Press L to log in to Tidal",
    no_results_hint: "  No results — search with /",
    col_title: "  Title",
    col_artist: "Artist",
    col_album: "Album",
    col_duration: "Dur.",
    player_stopped: "Nothing playing",
    hint_play: "play",
    hint_pause: "pause",
    hint_next_prev: "next/prev",
    hint_seek: "seek",
    hint_volume: "volume",
    hint_view: "view",
    hint_quality: "quality",
    hint_quit: "quit",
    hint_library: "library",
    hint_fav_tracks: "favorites",
    hint_fav_albums: "fav albums",
    hint_lang: "language",
    hint_add_queue: "add queue",
    hint_remove_queue: "remove",
    hint_search_clear: "clear search",
    lib_playlists: "Playlists",
    lib_mixes: "Mixes",
    lib_fav_tracks: "Favorites",
    lib_fav_albums: "Fav Albums",
    library_title: "Library",
    fav_tracks_empty: "  No favorite tracks",
    fav_albums_empty: "  No favorite albums",
    fav_albums_title: "Favorite Albums",
    col_tracks: "Tracks",
    login_title_text: "  Log in to Tidal",
    login_open_url: "  1. Open this URL:",
    login_code_prefix: "  2. Code: ",
    login_waiting: "  Waiting for authorization...",
    login_overlay_title: " ◈ Authentication ",
    status_no_results: "No results",
    status_auth_done: "✓ Authenticated with Tidal",
    status_login_required: "Log in first with 'L'",
    status_login_required_short: "Log in first (L)",
    status_starting_login: "Starting login...",
    status_loading_lib: "⟳ Loading library...",
    status_loading_playlist: "⟳ Loading playlist...",
    status_loading_mix: "⟳ Loading mix...",
    status_loading_fav_tracks: "⟳ Loading favorite tracks...",
    status_loading_fav_albums: "⟳ Loading favorite albums...",
    status_session_loading: "Loading session...",
    status_session_active: "✓ Session active",
    status_press_l: "Press 'L' to log in to Tidal",
    status_added_to_queue: "▶ Added to queue",
    status_already_in_queue: "Already in queue",
    status_search_cleared: "Search cleared",
};

static DE: Strings = Strings {
    tab_search: "Suchen",
    tab_queue: "Warteschl.",
    tab_now: "Jetzt",
    tab_library: "Bibliothek",
    search_placeholder: "/ zum Suchen drücken...",
    search_results_title: "Ergebnisse",
    search_sidebar_tracks: "Lieder",
    search_sidebar_albums: "Alben",
    search_sidebar_artists: "Künstler",
    search_sidebar_playlists: "Playlists",
    search_no_tracks: "  Keine Lieder",
    search_no_albums: "  Keine Alben",
    search_no_artists: "  Keine Künstler",
    search_no_playlists: "  Keine Playlists",
    search_load_more_hint: "Mehr laden",
    search_no_more_results: "Keine weiteren Ergebnisse",
    queue_title: "Wiedergabeliste",
    now_playing_empty: "Keine Wiedergabe — Enter auf einem Titel drücken",
    now_playing_title: "◈ Jetzt läuft",
    now_lyrics: "Text",
    now_no_lyrics: "Kein Text verfügbar",
    loading_image: "⟳ Lädt\n  Bild...",
    loading: "  ⟳ Lädt...",
    not_authenticated: "  L drücken um sich bei Tidal anzumelden",
    no_results_hint: "  Keine Ergebnisse — suche mit /",
    col_title: "  Titel",
    col_artist: "Künstler",
    col_album: "Album",
    col_duration: "Dauer",
    player_stopped: "Keine Wiedergabe",
    hint_play: "abspielen",
    hint_pause: "pause",
    hint_next_prev: "vor/zurück",
    hint_seek: "spulen",
    hint_volume: "Lautstärke",
    hint_view: "Ansicht",
    hint_quality: "Qualität",
    hint_quit: "beenden",
    hint_library: "Bibliothek",
    hint_fav_tracks: "Favoriten",
    hint_fav_albums: "Fav-Alben",
    hint_lang: "Sprache",
    hint_add_queue: "hinzufügen",
    hint_remove_queue: "entfernen",
    hint_search_clear: "Suche löschen",
    lib_playlists: "Playlists",
    lib_mixes: "Mixes",
    lib_fav_tracks: "Favoriten",
    lib_fav_albums: "Fav-Alben",
    library_title: "Bibliothek",
    fav_tracks_empty: "  Keine Lieblingstitel",
    fav_albums_empty: "  Keine Lieblingsalben",
    fav_albums_title: "Lieblingsalben",
    col_tracks: "Titel",
    login_title_text: "  Bei Tidal anmelden",
    login_open_url: "  1. URL öffnen:",
    login_code_prefix: "  2. Code: ",
    login_waiting: "  Warte auf Autorisierung...",
    login_overlay_title: " ◈ Authentifizierung ",
    status_no_results: "Keine Ergebnisse",
    status_auth_done: "✓ Bei Tidal authentifiziert",
    status_login_required: "Zuerst mit 'L' anmelden",
    status_login_required_short: "Zuerst anmelden (L)",
    status_starting_login: "Anmeldung wird gestartet...",
    status_loading_lib: "⟳ Bibliothek wird geladen...",
    status_loading_playlist: "⟳ Playlist wird geladen...",
    status_loading_mix: "⟳ Mix wird geladen...",
    status_loading_fav_tracks: "⟳ Lieblingstitel werden geladen...",
    status_loading_fav_albums: "⟳ Lieblingsalben werden geladen...",
    status_session_loading: "Sitzung wird geladen...",
    status_session_active: "✓ Sitzung aktiv",
    status_press_l: "'L' drücken um sich bei Tidal anzumelden",
    status_added_to_queue: "▶ Zur Warteschlange hinzugefügt",
    status_already_in_queue: "Bereits in der Warteschlange",
    status_search_cleared: "Suche gelöscht",
};

static RO: Strings = Strings {
    tab_search: "Caută",
    tab_queue: "Coadă",
    tab_now: "Acum",
    tab_library: "Bibliotecă",
    search_placeholder: "Apasă / pentru a căuta...",
    search_results_title: "Rezultate",
    search_sidebar_tracks: "Piese",
    search_sidebar_albums: "Albume",
    search_sidebar_artists: "Artiști",
    search_sidebar_playlists: "Playlisturi",
    search_no_tracks: "  Nicio piesă",
    search_no_albums: "  Niciun album",
    search_no_artists: "  Niciun artist",
    search_no_playlists: "  Niciun playlist",
    search_load_more_hint: "Încarcă mai mult",
    search_no_more_results: "Nu mai sunt rezultate",
    queue_title: "Coadă de redare",
    now_playing_empty: "Nimic nu rulează — apasă Enter pe o piesă",
    now_playing_title: "◈ Se redă acum",
    now_lyrics: "Versuri",
    now_no_lyrics: "Versuri indisponibile",
    loading_image: "⟳ Se încarcă\n  imaginea...",
    loading: "  ⟳ Se încarcă...",
    not_authenticated: "  Apasă L pentru a te autentifica în Tidal",
    no_results_hint: "  Niciun rezultat — caută cu /",
    col_title: "  Titlu",
    col_artist: "Artist",
    col_album: "Album",
    col_duration: "Dur.",
    player_stopped: "Nimic nu rulează",
    hint_play: "redă",
    hint_pause: "pauză",
    hint_next_prev: "urm/ant",
    hint_seek: "avans",
    hint_volume: "volum",
    hint_view: "vedere",
    hint_quality: "calitate",
    hint_quit: "ieșire",
    hint_library: "bibliotecă",
    hint_fav_tracks: "favorite",
    hint_fav_albums: "albume fav",
    hint_lang: "limbă",
    hint_add_queue: "adaugă coadă",
    hint_remove_queue: "elimină",
    hint_search_clear: "șterge căutarea",
    lib_playlists: "Playlisturi",
    lib_mixes: "Mixuri",
    lib_fav_tracks: "Favorite",
    lib_fav_albums: "Albume Fav",
    library_title: "Bibliotecă",
    fav_tracks_empty: "  Nicio piesă favorită",
    fav_albums_empty: "  Niciun album favorit",
    fav_albums_title: "Albume favorite",
    col_tracks: "Piese",
    login_title_text: "  Autentifică-te în Tidal",
    login_open_url: "  1. Deschide acest URL:",
    login_code_prefix: "  2. Cod: ",
    login_waiting: "  Se așteaptă autorizarea...",
    login_overlay_title: " ◈ Autentificare ",
    status_no_results: "Niciun rezultat",
    status_auth_done: "✓ Autentificat în Tidal",
    status_login_required: "Autentifică-te mai întâi cu 'L'",
    status_login_required_short: "Autentifică-te mai întâi (L)",
    status_starting_login: "Se inițiază autentificarea...",
    status_loading_lib: "⟳ Se încarcă biblioteca...",
    status_loading_playlist: "⟳ Se încarcă playlistul...",
    status_loading_mix: "⟳ Se încarcă mixul...",
    status_loading_fav_tracks: "⟳ Se încarcă piesele favorite...",
    status_loading_fav_albums: "⟳ Se încarcă albumele favorite...",
    status_session_loading: "Se încarcă sesiunea...",
    status_session_active: "✓ Sesiune activă",
    status_press_l: "Apasă 'L' pentru a te autentifica în Tidal",
    status_added_to_queue: "▶ Adăugat în coadă",
    status_already_in_queue: "Deja în coadă",
    status_search_cleared: "Căutare ștearsă",
};

impl Lang {
    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::Es => &ES,
            Lang::En => &EN,
            Lang::De => &DE,
            Lang::Ro => &RO,
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Lang::Es => Lang::En,
            Lang::En => Lang::De,
            Lang::De => Lang::Ro,
            Lang::Ro => Lang::Es,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::Es => "ES",
            Lang::En => "EN",
            Lang::De => "DE",
            Lang::Ro => "RO",
        }
    }

    // ── Dynamic string methods ────────────────────────────────────────────────

    pub fn results_count(self, n: usize) -> String {
        match self {
            Lang::Es => format!("{n} resultados"),
            Lang::En => format!("{n} results"),
            Lang::De => format!("{n} Ergebnisse"),
            Lang::Ro => format!("{n} rezultate"),
        }
    }

    pub fn search_error(self, e: &str) -> String {
        match self {
            Lang::Es => format!("✗ Error búsqueda: {e}"),
            Lang::En => format!("✗ Search error: {e}"),
            Lang::De => format!("✗ Suchfehler: {e}"),
            Lang::Ro => format!("✗ Eroare căutare: {e}"),
        }
    }

    pub fn stream_error(self, e: &str) -> String {
        match self {
            Lang::Es => format!("✗ Error stream: {e}"),
            Lang::En => format!("✗ Stream error: {e}"),
            Lang::De => format!("✗ Stream-Fehler: {e}"),
            Lang::Ro => format!("✗ Eroare stream: {e}"),
        }
    }

    pub fn searching(self, q: &str) -> String {
        match self {
            Lang::Es => format!("Buscando \"{q}\"..."),
            Lang::En => format!("Searching \"{q}\"..."),
            Lang::De => format!("Suche \"{q}\"..."),
            Lang::Ro => format!("Se caută \"{q}\"..."),
        }
    }

    pub fn searching_more(self, _q: &str) -> String {
        match self {
            Lang::Es => "Cargando más...".to_string(),
            Lang::En => "Loading more...".to_string(),
            Lang::De => "Lade mehr...".to_string(),
            Lang::Ro => "Se încarcă mai mult...".to_string(),
        }
    }

    pub fn loading_stream(self, title: &str) -> String {
        match self {
            Lang::Es => format!("⟳ Obteniendo stream: {title}..."),
            Lang::En => format!("⟳ Getting stream: {title}..."),
            Lang::De => format!("⟳ Stream wird geladen: {title}..."),
            Lang::Ro => format!("⟳ Se obține stream: {title}..."),
        }
    }

    pub fn browser_opened(self, code: &str) -> String {
        match self {
            Lang::Es => format!("Browser abierto. Código: {code}"),
            Lang::En => format!("Browser opened. Code: {code}"),
            Lang::De => format!("Browser geöffnet. Code: {code}"),
            Lang::Ro => format!("Browser deschis. Cod: {code}"),
        }
    }

    pub fn browser_failed(self, e: &str, url: &str) -> String {
        match self {
            Lang::Es => format!("No se pudo abrir browser ({e}): {url}"),
            Lang::En => format!("Could not open browser ({e}): {url}"),
            Lang::De => format!("Browser konnte nicht geöffnet werden ({e}): {url}"),
            Lang::Ro => format!("Nu s-a putut deschide browser-ul ({e}): {url}"),
        }
    }

    pub fn auth_error(self, e: &str) -> String {
        match self {
            Lang::Es => format!("✗ Error auth: {e}"),
            Lang::En => format!("✗ Auth error: {e}"),
            Lang::De => format!("✗ Auth-Fehler: {e}"),
            Lang::Ro => format!("✗ Eroare autentificare: {e}"),
        }
    }

    pub fn library_loaded(self, playlists: usize, mixes: usize) -> String {
        match self {
            Lang::Es => format!("✓ {playlists} playlists, {mixes} mixes"),
            Lang::En => format!("✓ {playlists} playlists, {mixes} mixes"),
            Lang::De => format!("✓ {playlists} Playlists, {mixes} Mixes"),
            Lang::Ro => format!("✓ {playlists} playlisturi, {mixes} mixuri"),
        }
    }

    pub fn tracks_loaded(self, n: usize) -> String {
        match self {
            Lang::Es => format!("✓ {n} tracks cargados"),
            Lang::En => format!("✓ {n} tracks loaded"),
            Lang::De => format!("✓ {n} Titel geladen"),
            Lang::Ro => format!("✓ {n} piese încărcate"),
        }
    }

    pub fn fav_tracks_loaded(self, n: usize) -> String {
        match self {
            Lang::Es => format!("✓ {n} canciones favoritas en cola"),
            Lang::En => format!("✓ {n} favorite tracks in queue"),
            Lang::De => format!("✓ {n} Lieblingstitel in der Warteschlange"),
            Lang::Ro => format!("✓ {n} piese favorite în coadă"),
        }
    }

    pub fn fav_albums_loaded(self, n: usize) -> String {
        match self {
            Lang::Es => format!("✓ {n} álbumes en colección"),
            Lang::En => format!("✓ {n} albums in collection"),
            Lang::De => format!("✓ {n} Alben in der Sammlung"),
            Lang::Ro => format!("✓ {n} albume în colecție"),
        }
    }

    pub fn quality_changed(self, label: &str) -> String {
        match self {
            Lang::Es => format!("Calidad: {label}"),
            Lang::En => format!("Quality: {label}"),
            Lang::De => format!("Qualität: {label}"),
            Lang::Ro => format!("Calitate: {label}"),
        }
    }

    pub fn loading_album(self, title: &str) -> String {
        match self {
            Lang::Es => format!("⟳ Cargando {title}..."),
            Lang::En => format!("⟳ Loading {title}..."),
            Lang::De => format!("⟳ {title} wird geladen..."),
            Lang::Ro => format!("⟳ Se încarcă {title}..."),
        }
    }

    pub fn fav_albums_title_with_count(self, n: usize) -> String {
        match self {
            Lang::Es => format!(" ◆ Álbumes favoritos ({n}) — Enter para cargar "),
            Lang::En => format!(" ◆ Favorite Albums ({n}) — Enter to load "),
            Lang::De => format!(" ◆ Lieblingsalben ({n}) — Enter zum Laden "),
            Lang::Ro => format!(" ◆ Albume favorite ({n}) — Enter pentru a încărca "),
        }
    }

    pub fn tracks_count(self, n: u32) -> String {
        match self {
            Lang::Es => format!("{n} tracks"),
            Lang::En => format!("{n} tracks"),
            Lang::De => format!("{n} Titel"),
            Lang::Ro => format!("{n} piese"),
        }
    }

    pub fn lang_changed(self) -> String {
        match self {
            Lang::Es => "Idioma: Español".to_string(),
            Lang::En => "Language: English".to_string(),
            Lang::De => "Sprache: Deutsch".to_string(),
            Lang::Ro => "Limbă: Română".to_string(),
        }
    }
}
