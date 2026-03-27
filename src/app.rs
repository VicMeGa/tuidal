use crate::player::{Player, TrackInfo};
use crate::tidal::{Quality, TidalClient, Track};

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Search,
    Queue,
}

pub struct App {
    pub tidal:        TidalClient,
    pub player:       Player,
    pub input_mode:   InputMode,
    pub active_tab:   Tab,

    pub search_input: String,
    pub search_results: Vec<Track>,
    pub queue:        Vec<Track>,

    pub selected:     usize,   // índice en resultados/cola activa
    pub queue_index:  Option<usize>, // track actual en cola

    pub authenticated: bool,
    pub status_msg:    String,
    pub loading:       bool,

    // Device auth
    pub device_code:   Option<String>,
    pub user_code:     Option<String>,
    pub auth_url:      Option<String>,
    pub poll_handle:   Option<tokio::task::JoinHandle<()>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            tidal:          TidalClient::new(),
            player:         Player::new(),
            input_mode:     InputMode::Normal,
            active_tab:     Tab::Search,
            search_input:   String::new(),
            search_results: Vec::new(),
            queue:          Vec::new(),
            selected:       0,
            queue_index:    None,
            authenticated:  false,
            status_msg:     String::new(),
            loading:        false,
            device_code:    None,
            user_code:      None,
            auth_url:       None,
            poll_handle:    None,
        }
    }

    pub fn set_quality(&mut self, q: Quality) {
        self.tidal.quality = q;
        self.status_msg = format!("Calidad: {}", q.label());
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Search => Tab::Queue,
            Tab::Queue  => Tab::Search,
        };
        self.selected = 0;
    }

    pub fn current_list_len(&self) -> usize {
        match self.active_tab {
            Tab::Search => self.search_results.len(),
            Tab::Queue  => self.queue.len(),
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
            self.selected = if self.selected == 0 { len - 1 } else { self.selected - 1 };
        }
    }

    pub async fn do_search(&mut self) {
        if !self.authenticated || self.search_input.is_empty() {
            if !self.authenticated {
                self.status_msg = "Primero inicia sesión con 'L'".to_string();
            }
            return;
        }
        self.loading   = true;
        self.status_msg = format!("Buscando \"{}\"...", self.search_input);

        match self.tidal.search(&self.search_input.clone(), 20).await {
            Ok(results) => {
                self.status_msg = if results.is_empty() {
                    "Sin resultados".to_string()
                } else {
                    format!("{} resultados", results.len())
                };
                self.search_results = results;
                self.selected  = 0;
                self.active_tab = Tab::Search;
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
        self.loading = false;
    }

    pub async fn play_selected(&mut self) {
        if !self.authenticated {
            self.status_msg = "Inicia sesión primero (L)".to_string();
            return;
        }

        let track = match self.active_tab {
            Tab::Search => self.search_results.get(self.selected).cloned(),
            Tab::Queue  => self.queue.get(self.selected).cloned(),
        };

        let Some(track) = track else { return };

        // Agregar a cola si viene de búsqueda
        if self.active_tab == Tab::Search {
            // Si no está ya en cola, agregarla
            if !self.queue.iter().any(|t| t.id == track.id) {
                self.queue.push(track.clone());
            }
            let qi = self.queue.iter().position(|t| t.id == track.id);
            self.queue_index = qi;
        } else {
            self.queue_index = Some(self.selected);
        }

        self.stream_and_play(track).await;
    }

    async fn stream_and_play(&mut self, track: Track) {
        self.status_msg = format!("▶ Obteniendo stream: {}...", track.title);
        self.loading    = true;

        match self.tidal.get_stream_info(track.id).await {
            Ok(info) => {
                let ti = TrackInfo {
                    title:       track.title.clone(),
                    artist:      track.artist_names(),
                    album:       track.album.title.clone(),
                    duration:    track.duration,
                    bit_depth:   info.bit_depth,
                    sample_rate: info.sample_rate,
                    codec:       info.codec.clone(),
                };

                self.status_msg = format!(
                    "▶ {} — {} | {}/{} {}",
                    track.artist_names(), track.title,
                    info.bit_depth, info.sample_rate, info.codec.to_uppercase()
                );

                self.player.play(&info.url, ti);
            }
            Err(e) => {
                self.status_msg = format!("✗ Error stream: {e}");
            }
        }
        self.loading = false;
    }

    pub async fn play_next(&mut self) {
        if self.queue.is_empty() { return; }
        let next = match self.queue_index {
            Some(i) if i + 1 < self.queue.len() => i + 1,
            _ => 0,
        };
        self.queue_index = Some(next);
        let track = self.queue[next].clone();
        self.stream_and_play(track).await;
    }

    pub async fn play_prev(&mut self) {
        if self.queue.is_empty() { return; }
        let prev = match self.queue_index {
            Some(i) if i > 0 => i - 1,
            _ => self.queue.len().saturating_sub(1),
        };
        self.queue_index = Some(prev);
        let track = self.queue[prev].clone();
        self.stream_and_play(track).await;
    }

    pub async fn start_login(&mut self) {
        match self.tidal.start_device_auth().await {
            Ok((device_code, user_code, url)) => {
                self.device_code = Some(device_code);
                self.user_code   = Some(user_code.clone());
                self.auth_url    = Some(url.clone());
                self.status_msg  = format!("Abre: {url}  Código: {user_code}");
            }
            Err(e) => {
                self.status_msg = format!("Error de auth: {e}");
            }
        }
    }

    /// Sondear el token de autenticación (llamar periódicamente desde el event loop)
    pub async fn poll_auth(&mut self) -> bool {
        if let Some(code) = self.device_code.clone() {
            match self.tidal.poll_device_token(&code).await {
                Ok(true) => {
                    self.authenticated = true;
                    self.device_code   = None;
                    self.user_code     = None;
                    self.auth_url      = None;
                    self.status_msg    = "✓ Autenticado con Tidal".to_string();
                    return true;
                }
                Ok(false) => {} // pendiente
                Err(_) => {}
            }
        }
        false
    }
}