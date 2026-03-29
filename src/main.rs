mod tidal;
mod ui;
mod player;
mod app;

use anyhow::Result;
use app::{App, AppEvent, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
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

    let result = run_app(&mut terminal, &mut app).await;

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
) -> Result<()> {
    let mut ui_tick   = interval(Duration::from_millis(50));
    let mut auth_tick = interval(Duration::from_secs(5));
    auth_tick.reset();

    // Canal para recibir resultados de operaciones async
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    app.event_tx = Some(tx);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        tokio::select! {
            // Resultados de operaciones en background
            Some(event) = rx.recv() => {
                app.handle_event(event);
            }

            _ = ui_tick.tick() => {
                app.player.tick();

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
            if !app.authenticated {
                app.start_login_bg();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => app.next_track(),
        KeyCode::Up   | KeyCode::Char('k') => app.prev_track(),
        KeyCode::Enter => app.play_selected_bg(),
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