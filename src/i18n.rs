#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    Es,
    En,
    De,
    Ro,
}

pub struct Strings {
    // Tabs
    pub tab_search:  &'static str,
    pub tab_queue:   &'static str,
    pub tab_now:     &'static str,
    pub tab_library: &'static str,
    // Search
    pub search_placeholder:   &'static str,
    pub search_results_title: &'static str,
    // Queue
    pub queue_title: &'static str,
    // Now playing
    pub now_playing_empty: &'static str,
    pub now_playing_title: &'static str,
    pub loading_image:     &'static str,
    // Track list
    pub loading:           &'static str,
    pub not_authenticated: &'static str,
    pub no_results_hint:   &'static str,
    pub col_title:         &'static str,
    pub col_artist:        &'static str,
    pub col_album:         &'static str,
    pub col_duration:      &'static str,
    // Player bar
    pub player_stopped: &'static str,
    // Hint bar
    pub hint_play:       &'static str,
    pub hint_pause:      &'static str,
    pub hint_next_prev:  &'static str,
    pub hint_seek:       &'static str,
    pub hint_volume:     &'static str,
    pub hint_view:       &'static str,
    pub hint_quality:    &'static str,
    pub hint_quit:       &'static str,
    pub hint_library:    &'static str,
    pub hint_fav_tracks: &'static str,
    pub hint_fav_albums: &'static str,
    pub hint_lang:       &'static str,
    // Library
    pub library_loading: &'static str,
    pub library_hint:    &'static str,
    pub library_title:   &'static str,
    pub col_name:        &'static str,
    pub col_info:        &'static str,
    pub col_type:        &'static str,
    // Fav albums
    pub fav_albums_empty: &'static str,
    pub fav_albums_title: &'static str,
    pub col_tracks:       &'static str,
    // Login overlay
    pub login_title_text:    &'static str,
    pub login_open_url:      &'static str,
    pub login_code_prefix:   &'static str,
    pub login_waiting:       &'static str,
    pub login_overlay_title: &'static str,
    // Status messages (static)
    pub status_no_results:           &'static str,
    pub status_auth_done:            &'static str,
    pub status_login_required:       &'static str,
    pub status_login_required_short: &'static str,
    pub status_starting_login:       &'static str,
    pub status_loading_lib:          &'static str,
    pub status_loading_playlist:     &'static str,
    pub status_loading_mix:          &'static str,
    pub status_loading_fav_tracks:   &'static str,
    pub status_loading_fav_albums:   &'static str,
    pub status_session_loading:      &'static str,
    pub status_session_active:       &'static str,
    pub status_press_l:              &'static str,
}

static ES: Strings = Strings {
    tab_search:  "Buscar",
    tab_queue:   "Cola",
    tab_now:     "Ahora",
    tab_library: "Biblioteca",
    search_placeholder:   "Presiona / para buscar...",
    search_results_title: "Resultados",
    queue_title:           "Cola de reproducción",
    now_playing_empty:    "Sin reproducción — presiona Enter en una canción",
    now_playing_title:    "◈ Ahora reproduciendo",
    loading_image:         "⟳ Cargando\n  imagen...",
    loading:               "  ⟳ Cargando...",
    not_authenticated:     "  Presiona L para iniciar sesión en Tidal",
    no_results_hint:       "  Sin resultados — busca con /",
    col_title:    "  Título",
    col_artist:   "Artista",
    col_album:    "Álbum",
    col_duration: "Dur.",
    player_stopped:  "Sin reproducción",
    hint_play:       "reproducir",
    hint_pause:      "pausa",
    hint_next_prev:  "sig/ant",
    hint_seek:       "seek",
    hint_volume:     "volumen",
    hint_view:       "vista",
    hint_quality:    "calidad",
    hint_quit:       "salir",
    hint_library:    "biblioteca",
    hint_fav_tracks: "fav tracks",
    hint_fav_albums: "fav álbumes",
    hint_lang:       "idioma",
    library_loading: "  ⟳ Cargando biblioteca...",
    library_hint:    "  Presiona 'i' para cargar playlists y mixes",
    library_title:   "Biblioteca",
    col_name:  "Nombre",
    col_info:  "Info",
    col_type:  "Tipo",
    fav_albums_empty: "  Sin álbumes favoritos",
    fav_albums_title: "Álbumes favoritos",
    col_tracks:       "Tracks",
    login_title_text:    "  Inicia sesión en Tidal",
    login_open_url:      "  1. Abre este URL:",
    login_code_prefix:   "  2. Código: ",
    login_waiting:       "  Esperando autorización...",
    login_overlay_title: " ◈ Autenticación ",
    status_no_results:           "Sin resultados",
    status_auth_done:            "✓ Autenticado con Tidal",
    status_login_required:       "Primero inicia sesión con 'L'",
    status_login_required_short: "Inicia sesión primero (L)",
    status_starting_login:       "Iniciando login...",
    status_loading_lib:          "⟳ Cargando biblioteca...",
    status_loading_playlist:     "⟳ Cargando playlist...",
    status_loading_mix:          "⟳ Cargando mix...",
    status_loading_fav_tracks:   "⟳ Cargando canciones favoritas...",
    status_loading_fav_albums:   "⟳ Cargando álbumes favoritos...",
    status_session_loading:      "Cargando sesión...",
    status_session_active:       "✓ Sesión activa",
    status_press_l:              "Presiona 'L' para iniciar sesión en Tidal",
};

static EN: Strings = Strings {
    tab_search:  "Search",
    tab_queue:   "Queue",
    tab_now:     "Now",
    tab_library: "Library",
    search_placeholder:   "Press / to search...",
    search_results_title: "Results",
    queue_title:           "Playback Queue",
    now_playing_empty:    "Nothing playing — press Enter on a track",
    now_playing_title:    "◈ Now Playing",
    loading_image:         "⟳ Loading\n  image...",
    loading:               "  ⟳ Loading...",
    not_authenticated:     "  Press L to log in to Tidal",
    no_results_hint:       "  No results — search with /",
    col_title:    "  Title",
    col_artist:   "Artist",
    col_album:    "Album",
    col_duration: "Dur.",
    player_stopped:  "Nothing playing",
    hint_play:       "play",
    hint_pause:      "pause",
    hint_next_prev:  "next/prev",
    hint_seek:       "seek",
    hint_volume:     "volume",
    hint_view:       "view",
    hint_quality:    "quality",
    hint_quit:       "quit",
    hint_library:    "library",
    hint_fav_tracks: "fav tracks",
    hint_fav_albums: "fav albums",
    hint_lang:       "language",
    library_loading: "  ⟳ Loading library...",
    library_hint:    "  Press 'i' to load playlists and mixes",
    library_title:   "Library",
    col_name:  "Name",
    col_info:  "Info",
    col_type:  "Type",
    fav_albums_empty: "  No favorite albums",
    fav_albums_title: "Favorite Albums",
    col_tracks:       "Tracks",
    login_title_text:    "  Log in to Tidal",
    login_open_url:      "  1. Open this URL:",
    login_code_prefix:   "  2. Code: ",
    login_waiting:       "  Waiting for authorization...",
    login_overlay_title: " ◈ Authentication ",
    status_no_results:           "No results",
    status_auth_done:            "✓ Authenticated with Tidal",
    status_login_required:       "Log in first with 'L'",
    status_login_required_short: "Log in first (L)",
    status_starting_login:       "Starting login...",
    status_loading_lib:          "⟳ Loading library...",
    status_loading_playlist:     "⟳ Loading playlist...",
    status_loading_mix:          "⟳ Loading mix...",
    status_loading_fav_tracks:   "⟳ Loading favorite tracks...",
    status_loading_fav_albums:   "⟳ Loading favorite albums...",
    status_session_loading:      "Loading session...",
    status_session_active:       "✓ Session active",
    status_press_l:              "Press 'L' to log in to Tidal",
};

static DE: Strings = Strings {
    tab_search:  "Suchen",
    tab_queue:   "Warteschl.",
    tab_now:     "Jetzt",
    tab_library: "Bibliothek",
    search_placeholder:   "/ zum Suchen drücken...",
    search_results_title: "Ergebnisse",
    queue_title:           "Wiedergabeliste",
    now_playing_empty:    "Keine Wiedergabe — Enter auf einem Titel drücken",
    now_playing_title:    "◈ Jetzt läuft",
    loading_image:         "⟳ Lädt\n  Bild...",
    loading:               "  ⟳ Lädt...",
    not_authenticated:     "  L drücken um sich bei Tidal anzumelden",
    no_results_hint:       "  Keine Ergebnisse — suche mit /",
    col_title:    "  Titel",
    col_artist:   "Künstler",
    col_album:    "Album",
    col_duration: "Dauer",
    player_stopped:  "Keine Wiedergabe",
    hint_play:       "abspielen",
    hint_pause:      "pause",
    hint_next_prev:  "vor/zurück",
    hint_seek:       "spulen",
    hint_volume:     "Lautstärke",
    hint_view:       "Ansicht",
    hint_quality:    "Qualität",
    hint_quit:       "beenden",
    hint_library:    "Bibliothek",
    hint_fav_tracks: "Fav-Titel",
    hint_fav_albums: "Fav-Alben",
    hint_lang:       "Sprache",
    library_loading: "  ⟳ Bibliothek wird geladen...",
    library_hint:    "  'i' drücken um Playlists und Mixes zu laden",
    library_title:   "Bibliothek",
    col_name:  "Name",
    col_info:  "Info",
    col_type:  "Typ",
    fav_albums_empty: "  Keine Lieblingsalben",
    fav_albums_title: "Lieblingsalben",
    col_tracks:       "Titel",
    login_title_text:    "  Bei Tidal anmelden",
    login_open_url:      "  1. URL öffnen:",
    login_code_prefix:   "  2. Code: ",
    login_waiting:       "  Warte auf Autorisierung...",
    login_overlay_title: " ◈ Authentifizierung ",
    status_no_results:           "Keine Ergebnisse",
    status_auth_done:            "✓ Bei Tidal authentifiziert",
    status_login_required:       "Zuerst mit 'L' anmelden",
    status_login_required_short: "Zuerst anmelden (L)",
    status_starting_login:       "Anmeldung wird gestartet...",
    status_loading_lib:          "⟳ Bibliothek wird geladen...",
    status_loading_playlist:     "⟳ Playlist wird geladen...",
    status_loading_mix:          "⟳ Mix wird geladen...",
    status_loading_fav_tracks:   "⟳ Lieblingstitel werden geladen...",
    status_loading_fav_albums:   "⟳ Lieblingsalben werden geladen...",
    status_session_loading:      "Sitzung wird geladen...",
    status_session_active:       "✓ Sitzung aktiv",
    status_press_l:              "'L' drücken um sich bei Tidal anzumelden",
};

static RO: Strings = Strings {
    tab_search:  "Caută",
    tab_queue:   "Coadă",
    tab_now:     "Acum",
    tab_library: "Bibliotecă",
    search_placeholder:   "Apasă / pentru a căuta...",
    search_results_title: "Rezultate",
    queue_title:           "Coadă de redare",
    now_playing_empty:    "Nimic nu rulează — apasă Enter pe o piesă",
    now_playing_title:    "◈ Se redă acum",
    loading_image:         "⟳ Se încarcă\n  imaginea...",
    loading:               "  ⟳ Se încarcă...",
    not_authenticated:     "  Apasă L pentru a te autentifica în Tidal",
    no_results_hint:       "  Niciun rezultat — caută cu /",
    col_title:    "  Titlu",
    col_artist:   "Artist",
    col_album:    "Album",
    col_duration: "Dur.",
    player_stopped:  "Nimic nu rulează",
    hint_play:       "redă",
    hint_pause:      "pauză",
    hint_next_prev:  "urm/ant",
    hint_seek:       "avans",
    hint_volume:     "volum",
    hint_view:       "vedere",
    hint_quality:    "calitate",
    hint_quit:       "ieșire",
    hint_library:    "bibliotecă",
    hint_fav_tracks: "piese fav",
    hint_fav_albums: "albume fav",
    hint_lang:       "limbă",
    library_loading: "  ⟳ Se încarcă biblioteca...",
    library_hint:    "  Apasă 'i' pentru a încărca playlisturi și mixuri",
    library_title:   "Bibliotecă",
    col_name:  "Nume",
    col_info:  "Info",
    col_type:  "Tip",
    fav_albums_empty: "  Niciun album favorit",
    fav_albums_title: "Albume favorite",
    col_tracks:       "Piese",
    login_title_text:    "  Autentifică-te în Tidal",
    login_open_url:      "  1. Deschide acest URL:",
    login_code_prefix:   "  2. Cod: ",
    login_waiting:       "  Se așteaptă autorizarea...",
    login_overlay_title: " ◈ Autentificare ",
    status_no_results:           "Niciun rezultat",
    status_auth_done:            "✓ Autentificat în Tidal",
    status_login_required:       "Autentifică-te mai întâi cu 'L'",
    status_login_required_short: "Autentifică-te mai întâi (L)",
    status_starting_login:       "Se inițiază autentificarea...",
    status_loading_lib:          "⟳ Se încarcă biblioteca...",
    status_loading_playlist:     "⟳ Se încarcă playlistul...",
    status_loading_mix:          "⟳ Se încarcă mixul...",
    status_loading_fav_tracks:   "⟳ Se încarcă piesele favorite...",
    status_loading_fav_albums:   "⟳ Se încarcă albumele favorite...",
    status_session_loading:      "Se încarcă sesiunea...",
    status_session_active:       "✓ Sesiune activă",
    status_press_l:              "Apasă 'L' pentru a te autentifica în Tidal",
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

    pub fn library_title_with_counts(self, playlists: usize, mixes: usize) -> String {
        match self {
            Lang::Es => format!(" Biblioteca ({playlists} playlists, {mixes} mixes) "),
            Lang::En => format!(" Library ({playlists} playlists, {mixes} mixes) "),
            Lang::De => format!(" Bibliothek ({playlists} Playlists, {mixes} Mixes) "),
            Lang::Ro => format!(" Bibliotecă ({playlists} playlisturi, {mixes} mixuri) "),
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
