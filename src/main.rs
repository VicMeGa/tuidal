mod tidal;
mod ui;
mod player;
mod app;
mod api;

use anyhow::Result;
use app::{App, AppEvent, ApiStatus, InputMode, Tab};
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::{Arc, RwLock}, time::Duration};
use tokio::sync::mpsc;
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<()> {
    // Debe hacerse ANTES de enable_raw_mode y EnterAlternateScreen
    let picker = ratatui_image::picker::Picker::from_query_stdio().ok();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.picker = picker;

    app.status_msg = "Cargando sesión...".to_string();
    if app.tidal.load_session().await.is_ok() {
        app.status_msg = "✓ Sesión activa".to_string();
        app.authenticated = true;
    } else {
        app.status_msg = "Presiona 'L' para iniciar sesión en Tidal".to_string();
    }

    let (event_tx, event_rx) = mpsc::unbounded_channel::<AppEvent>();
    app.event_tx = Some(event_tx.clone());

    let api_status = Arc::new(RwLock::new(ApiStatus::default()));
    tokio::spawn(api::start_server(
        event_tx,
        api_status.clone(),
        app.tidal.script_path.clone(),
        app.tidal.quality,
        app.tidal.python_path.clone(),
    ));

    let result = run_app(&mut terminal, &mut app, event_rx, api_status).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut rx: mpsc::UnboundedReceiver<AppEvent>,
    api_status: Arc<RwLock<ApiStatus>>,
) -> Result<()> {
    let mut ui_tick   = interval(Duration::from_millis(50));
    let mut auth_tick = interval(Duration::from_secs(5));
    auth_tick.reset();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        tokio::select! {
            // Resultados de operaciones en background
            Some(event) = rx.recv() => {
                app.handle_event(event);
            }

            _ = ui_tick.tick() => {
                app.player.tick();
                if let Ok(mut s) = api_status.write() {
                    *s = app.api_status_snapshot();
                }

                // Avance automático al siguiente track
                if app.player.state == player::PlayerState::Stopped
                    && app.queue_index.is_some()
                    && !app.queue.is_empty()
                    && app.auto_advance
                {
                    app.auto_advance = false;
                    app.play_next_bg();
                }

                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.modifiers == KeyModifiers::CONTROL
                            && key.code == KeyCode::Char('c')
                        {
                            app.player.stop();
                            return Ok(());
                        }
                        match app.input_mode {
                            InputMode::Normal => handle_normal(key.code, app),
                            InputMode::Search => handle_search(key.code, app),
                        }
                    }
                }
            }

            _ = auth_tick.tick() => {
                if app.device_code.is_some() && !app.authenticated {
                    app.poll_auth_bg();
                }
            }
        }
    }
}
fn handle_normal(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Char('q') => {
            app.player.stop();
            std::process::exit(0);
        }
        KeyCode::Char('/') | KeyCode::Char('s') => {
            app.input_mode = InputMode::Search;
            app.search_input.clear();
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            if !app.authenticated { app.start_login_bg(); }
        }
        KeyCode::Char('i') => {
            if app.authenticated { app.load_library_bg(); }
        }
        KeyCode::Char('F') => {
            if app.authenticated { app.load_fav_tracks_bg(); }
        }
        KeyCode::Char('A') => {
            if app.authenticated { app.load_fav_albums_bg(); }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.active_tab == Tab::Library {
                if app.collection_view == app::CollectionView::Albums {
                    let max = app.fav_albums.len();
                    if max > 0 { app.fav_album_selected = (app.fav_album_selected + 1) % max; }
                } else {
                    let max = app.playlists.len() + app.mixes.len();
                    if max > 0 { app.library_selected = (app.library_selected + 1) % max; }
                }
            } else {
                app.next_track();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.active_tab == Tab::Library {
                if app.collection_view == app::CollectionView::Albums {
                    let max = app.fav_albums.len();
                    if max > 0 {
                        app.fav_album_selected = if app.fav_album_selected == 0 { max - 1 } else { app.fav_album_selected - 1 };
                    }
                } else {
                    let max = app.playlists.len() + app.mixes.len();
                    if max > 0 {
                        app.library_selected = if app.library_selected == 0 { max - 1 } else { app.library_selected - 1 };
                    }
                }
            } else {
                app.prev_track();
            }
        }
        KeyCode::Enter => {
            if app.active_tab == Tab::Library {
                app.library_select_enter();
            } else {
                app.play_selected_bg();
            }
        }
        KeyCode::Char(' ') => app.player.toggle_pause(),
        KeyCode::Char('n') => app.play_next_bg(),
        KeyCode::Char('p') => app.play_prev_bg(),
        KeyCode::Right => app.player.seek_forward(),
        KeyCode::Left  => app.player.seek_backward(),
        KeyCode::Char('+') | KeyCode::Char('=') => app.player.volume_up(),
        KeyCode::Char('-') => app.player.volume_down(),
        KeyCode::Tab => app.next_tab(),
        KeyCode::Char('1') => app.set_quality(tidal::Quality::HiResLossless),
        KeyCode::Char('2') => app.set_quality(tidal::Quality::Lossless),
        KeyCode::Char('3') => app.set_quality(tidal::Quality::High),
        _ => {}
    }
}

fn handle_search(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.do_search_bg();
        }
        KeyCode::Backspace => {
            app.search_input.pop();
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
        }
        _ => {}
    }
}