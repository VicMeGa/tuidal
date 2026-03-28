use crate::player::{Player, PlayerState, TrackInfo};
use crate::tidal::{Quality, TidalClient, Track, StreamInfo};
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
}

/// Eventos que llegan desde tareas en background al event loop principal
pub enum AppEvent {
    SearchDone(Result<Vec<Track>, String>),
    StreamReady { track: Track, info: StreamInfo, queue_index: usize },
    StreamError(String),
    AuthStarted { url: String, code: String, device_code: String },
    AuthDone,
    AuthError(String),
    StatusMsg(String),
}

pub struct App {
    pub tidal:        TidalClient,
    pub player:       Player,
    pub input_mode:   InputMode,
    pub active_tab:   Tab,

    pub search_input:   String,
    pub search_results: Vec<Track>,
    pub queue:          Vec<Track>,

    pub selected:    usize,
    pub queue_index: Option<usize>,

    pub authenticated: bool,
    pub status_msg:    String,
    pub loading:       bool,
    pub auto_advance:  bool,

    pub device_code: Option<String>,
    pub user_code:   Option<String>,
    pub auth_url:    Option<String>,
    pub poll_handle: Option<tokio::task::JoinHandle<()>>,

    pub event_tx: Option<UnboundedSender<AppEvent>>,
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
            auto_advance:   false,
            device_code:    None,
            user_code:      None,
            auth_url:       None,
            poll_handle:    None,
            event_tx:       None,
        }
    }

    fn tx(&self) -> UnboundedSender<AppEvent> {
        self.event_tx.clone().expect("event_tx no inicializado")
    }

    // ── Manejo de eventos entrantes ───────────────────────────────────────

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::SearchDone(Ok(results)) => {
                self.status_msg = if results.is_empty() {
                    "Sin resultados".to_string()
                } else {
                    format!("{} resultados", results.len())
                };
                self.search_results = results;
                self.selected       = 0;
                self.active_tab     = Tab::Search;
                self.loading        = false;
            }
            AppEvent::SearchDone(Err(e)) => {
                self.status_msg = format!("✗ Error búsqueda: {e}");
                self.loading    = false;
            }
            AppEvent::StreamReady { track, info, queue_index } => {
                self.queue_index = Some(queue_index);
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
                self.loading      = false;
                self.auto_advance = true;
            }
            AppEvent::StreamError(e) => {
                self.status_msg = format!("✗ Error stream: {e}");
                self.loading    = false;
            }
            AppEvent::AuthStarted { url, code, device_code } => {
                self.device_code = Some(device_code);
                self.user_code   = Some(code.clone());
                self.auth_url    = Some(url.clone());
                self.status_msg  = format!("Abre: {url}  Código: {code}");
                self.loading     = false;
            }
            AppEvent::AuthDone => {
                self.authenticated = true;
                self.device_code   = None;
                self.user_code     = None;
                self.auth_url      = None;
                self.status_msg    = "✓ Autenticado con Tidal".to_string();
                self.loading       = false;
            }
            AppEvent::AuthError(e) => {
                self.status_msg = format!("✗ Error auth: {e}");
                self.loading    = false;
            }
            AppEvent::StatusMsg(msg) => {
                self.status_msg = msg;
            }
        }
    }

    // ── Operaciones en background ─────────────────────────────────────────

    pub fn do_search_bg(&mut self) {
        if !self.authenticated {
            self.status_msg = "Primero inicia sesión con 'L'".to_string();
            return;
        }
        if self.search_input.is_empty() { return; }

        self.loading    = true;
        self.status_msg = format!("Buscando \"{}\"...", self.search_input);

        let tx      = self.tx();
        let query   = self.search_input.clone();
        let script  = self.tidal.script_path.clone();
        let quality = self.tidal.quality;

        tokio::spawn(async move {
            let client = TidalClient::with_path_and_quality(script, quality);
            let result = client.search(&query, 20).await
                .map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::SearchDone(result));
        });
    }

    pub fn play_selected_bg(&mut self) {
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
        let queue_index = if self.active_tab == Tab::Search {
            if !self.queue.iter().any(|t| t.id == track.id) {
                self.queue.push(track.clone());
            }
            self.queue.iter().position(|t| t.id == track.id).unwrap_or(0)
        } else {
            self.selected
        };

        self.stream_track_bg(track, queue_index);
    }

    pub fn play_next_bg(&mut self) {
        if self.queue.is_empty() { return; }
        let next = match self.queue_index {
            Some(i) if i + 1 < self.queue.len() => i + 1,
            _ => 0,
        };
        let track = self.queue[next].clone();
        self.stream_track_bg(track, next);
    }

    pub fn play_prev_bg(&mut self) {
        if self.queue.is_empty() { return; }
        let prev = match self.queue_index {
            Some(i) if i > 0 => i - 1,
            _ => self.queue.len().saturating_sub(1),
        };
        let track = self.queue[prev].clone();
        self.stream_track_bg(track, prev);
    }

    fn stream_track_bg(&mut self, track: Track, queue_index: usize) {
        self.loading    = true;
        self.status_msg = format!("⟳ Obteniendo stream: {}...", track.title);
        self.player.stop();

        let tx     = self.tx();
        let script = self.tidal.script_path.clone();
        let quality = self.tidal.quality;

        tokio::spawn(async move {
            let client = TidalClient::with_path_and_quality(script, quality);
            match client.get_stream_info(track.id).await {
                Ok(info) => {
                    let _ = tx.send(AppEvent::StreamReady { track, info, queue_index });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::StreamError(e.to_string()));
                }
            }
        });
    }

    pub fn start_login_bg(&mut self) {
        self.loading    = true;
        self.status_msg = "Iniciando login...".to_string();

        let tx     = self.tx();
        let script = self.tidal.script_path.clone();
        let quality = self.tidal.quality;

        tokio::spawn(async move {
            let client = TidalClient::with_path_and_quality(script, quality);
            match client.start_device_auth().await {
                Ok((device_code, user_code, url)) => {
                    let _ = tx.send(AppEvent::AuthStarted { url, code: user_code, device_code });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::AuthError(e.to_string()));
                }
            }
        });
    }

    pub fn poll_auth_bg(&mut self) {
        let tx     = self.tx();
        let script = self.tidal.script_path.clone();
        let quality = self.tidal.quality;
        let code   = self.device_code.clone().unwrap_or_default();

        tokio::spawn(async move {
            let client = TidalClient::with_path_and_quality(script, quality);
            match client.poll_device_token(&code).await {
                Ok(true)  => { let _ = tx.send(AppEvent::AuthDone); }
                Ok(false) => {}
                Err(e)    => { let _ = tx.send(AppEvent::StatusMsg(format!("Error poll: {e}"))); }
            }
        });
    }

    // ── Helpers síncronos ─────────────────────────────────────────────────

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
        if len > 0 { self.selected = (self.selected + 1) % len; }
    }

    pub fn prev_track(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.selected = if self.selected == 0 { len - 1 } else { self.selected - 1 };
        }
    }

    // ── Auth poll síncrono (solo para load_session al inicio) ─────────────
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
                Ok(false) => {}
                Err(e) => { self.status_msg = format!("Error poll: {e}"); }
            }
        }
        false
    }
}