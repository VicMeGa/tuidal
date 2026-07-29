mod api;
mod app;
mod i18n;
mod library;
mod mpris;
mod player;
mod settings;
mod tasks;
mod tidal;
mod ui;

use anyhow::Result;
use app::{ApiStatus, App, AppEvent, InputMode, Tab};
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use library::{LibraryFocus, LibrarySection};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::time::interval;

struct TerminalRestorer;

impl Drop for TerminalRestorer {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        let _ = execute!(stdout, crossterm::cursor::Show);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _restorer = TerminalRestorer;
    let picker = ratatui_image::picker::Picker::from_query_stdio().ok();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Inicializar daemon persistente de Python
    let script_path = tidal::TidalDaemonClient::default_script_path();
    let python_path = std::env::var("TUIDAL_PYTHON_PATH").unwrap_or_else(|_| "python3".to_string());
    let tidal = tidal::TidalDaemonClient::spawn(&script_path, &python_path, "LOSSLESS")
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error iniciando daemon Python: {e}");
            std::process::exit(1);
        });

    let mut app = App::new(tidal.clone());
    app.picker = picker;

    app.load_settings();
    // Sincronizar calidad local con el daemon
    let _ = tidal.set_quality(app.quality.as_api_str()).await;

    app.status_msg = app.lang.strings().status_session_loading.to_string();
    if tidal.poll_device_token().await.unwrap_or(false) {
        app.status_msg = app.lang.strings().status_session_active.to_string();
        app.authenticated = true;
    } else {
        app.status_msg = app.lang.strings().status_press_l.to_string();
    }

    let (event_tx, event_rx) = mpsc::unbounded_channel::<AppEvent>();
    app.event_tx = Some(event_tx.clone());

    let api_status = Arc::new(RwLock::new(ApiStatus::default()));
    let api_handle = tokio::spawn(api::start_server(
        event_tx.clone(),
        api_status.clone(),
        tidal.clone(),
    ));

    let mpris_handle = tokio::spawn(mpris::start_mpris_server(
        api_status.clone(),
        event_tx.clone(),
    ));

    let result = run_app(&mut terminal, &mut app, event_rx, api_status).await;

    // Graceful shutdown
    tidal.shutdown().await;
    api_handle.abort();
    mpris_handle.abort();

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
    let mut ui_tick = interval(Duration::from_millis(50));
    let mut auth_tick = interval(Duration::from_secs(5));
    let mut status_skip = 0u32;
    auth_tick.reset();

    loop {
        if app.should_quit {
            break;
        }

        terminal.draw(|f| ui::draw(f, app))?;

        tokio::select! {
            Some(event) = rx.recv() => {
                app.handle_event(event);
            }

            _ = ui_tick.tick() => {
                app.player.tick();
                // ponytail: snapshot status every 500ms instead of every 50ms
                status_skip += 1;
                if status_skip % 10 == 0 {
                    if let Ok(mut s) = api_status.write() {
                        *s = app.api_status_snapshot();
                    }
                }

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
                            app.should_quit = true;
                            continue;
                        }
                        if key.code == KeyCode::Char('l') && key.modifiers == KeyModifiers::ALT
                        {
                            app.cycle_lang();
                        } else {
                            match app.input_mode {
                                InputMode::Normal => handle_normal(key.code, app),
                                InputMode::Search => handle_search(key.code, app),
                            }
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
    Ok(())
}

fn handle_normal(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Char('q') => {
            app.player.stop();
            app.should_quit = true;
        }
        KeyCode::Char('/') | KeyCode::Char('s') => {
            app.input_mode = InputMode::Search;
            app.search_input.clear();
        }
        KeyCode::Char('L') => {
            if !app.authenticated {
                app.start_login_bg();
            }
        }
        KeyCode::Char('i') => {
            if app.authenticated {
                app.active_tab = Tab::Library;
                app.library.viewing = None;
                app.ensure_section_loaded();
            }
        }
        KeyCode::Char('F') => {
            if app.authenticated {
                app.active_tab = Tab::Library;
                app.library.focus = LibraryFocus::Content;
                app.library.active_section = LibrarySection::FavTracks;
                app.library.viewing = None;
                app.ensure_section_loaded();
            }
        }
        KeyCode::Char('A') => {
            if app.active_tab == Tab::Library
                && app.library.focus == LibraryFocus::Content
                && (app.library.viewing.is_some()
                    || app.library.active_section == LibrarySection::FavTracks)
            {
                app.add_all_tracks_to_queue();
            } else if app.authenticated {
                app.active_tab = Tab::Library;
                app.library.focus = LibraryFocus::Content;
                app.library.active_section = LibrarySection::FavAlbums;
                app.library.viewing = None;
                app.ensure_section_loaded();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.active_tab == Tab::Library {
                if let Some(ref mut viewing) = app.library.viewing {
                    let max = viewing.tracks.len();
                    if max > 0 {
                        viewing.cursor = (viewing.cursor + 1) % max;
                    }
                } else if app.library.focus == LibraryFocus::Sidebar {
                    app.library.active_section = app.library.active_section.next();
                    app.ensure_section_loaded();
                } else {
                    let section = app.library.active_section;
                    let max = app.section_len(section);
                    if max > 0 {
                        let cur = app.library.cursor[section as usize];
                        app.library.cursor[section as usize] = (cur + 1) % max;
                    }
                }
            } else {
                app.next_track();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.active_tab == Tab::Library {
                if let Some(ref mut viewing) = app.library.viewing {
                    let max = viewing.tracks.len();
                    if max > 0 {
                        viewing.cursor = if viewing.cursor == 0 {
                            max - 1
                        } else {
                            viewing.cursor - 1
                        };
                    }
                } else if app.library.focus == LibraryFocus::Sidebar {
                    app.library.active_section = app.library.active_section.prev();
                    app.ensure_section_loaded();
                } else {
                    let section = app.library.active_section;
                    let max = app.section_len(section);
                    if max > 0 {
                        let cur = app.library.cursor[section as usize];
                        app.library.cursor[section as usize] =
                            if cur == 0 { max - 1 } else { cur - 1 };
                    }
                }
            } else {
                app.prev_track();
            }
        }
        KeyCode::Enter => {
            if app.active_tab == Tab::Library {
                if app.library.focus == LibraryFocus::Sidebar {
                    app.library.focus = LibraryFocus::Content;
                }
                if app.library.viewing.is_some() {
                    app.play_drilldown_track();
                } else if app.library.active_section == LibrarySection::FavTracks {
                    app.play_fav_track();
                } else {
                    app.library_select_enter();
                }
            } else {
                app.play_selected_bg();
            }
        }
        KeyCode::Char('a') => {
            if app.active_tab == Tab::Library
                && app.library.focus == LibraryFocus::Content
                && (app.library.viewing.is_some()
                    || app.library.active_section == LibrarySection::FavTracks)
            {
                app.add_current_track_to_queue();
            } else {
                app.add_selected_to_queue();
            }
        }
        KeyCode::Char(' ') => app.player.toggle_pause(),
        KeyCode::Char('d') => {
            if app.active_tab == Tab::Queue && !app.queue.is_empty() {
                app.remove_from_queue(app.selected);
            }
        }
        KeyCode::Char('n') => app.play_next_bg(),
        KeyCode::Char('p') => app.play_prev_bg(),
        KeyCode::Right | KeyCode::Char('l') => {
            if app.active_tab == Tab::Library {
                if app.library.focus == LibraryFocus::Sidebar {
                    app.library.focus = LibraryFocus::Content;
                }
            } else {
                app.player.seek_forward();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.active_tab == Tab::Library {
                if app.library.viewing.is_some() {
                    app.library.viewing = None;
                } else {
                    app.library.focus = LibraryFocus::Sidebar;
                }
            } else {
                app.player.seek_backward();
            }
        }
        KeyCode::Esc => {
            if app.active_tab == Tab::Library && app.library.viewing.is_some() {
                app.library.viewing = None;
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') => app.volume_up(),
        KeyCode::Char('-') => app.volume_down(),
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
