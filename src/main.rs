mod tidal;
mod ui;
mod player;
mod app;

use anyhow::Result;
use app::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

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
    // Ticker de UI: redibujar y leer input cada 50ms
    let mut ui_tick   = interval(Duration::from_millis(50));
    // Ticker OAuth: sondear token cada 5s
    let mut auth_tick = interval(Duration::from_secs(5));
    // Evitar que auth_tick dispare inmediatamente
    auth_tick.reset();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        tokio::select! {
            _ = ui_tick.tick() => {
                app.player.tick();

                // Leer evento de teclado si hay uno (no bloqueante)
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.modifiers == KeyModifiers::CONTROL
                            && key.code == KeyCode::Char('c')
                        {
                            app.player.stop();
                            return Ok(());
                        }
                        match app.input_mode {
                            InputMode::Normal => handle_normal(key.code, app).await,
                            InputMode::Search => handle_search(key.code, app).await,
                        }
                    }
                }
            }

            // Sondear token OAuth solo cuando hay autenticación pendiente
            _ = auth_tick.tick() => {
                if app.device_code.is_some() && !app.authenticated {
                    app.poll_auth().await;
                }
            }
        }
    }
}

async fn handle_normal(key: KeyCode, app: &mut App) {
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
                app.start_login().await;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => app.next_track(),
        KeyCode::Up   | KeyCode::Char('k') => app.prev_track(),
        KeyCode::Enter => app.play_selected().await,
        KeyCode::Char(' ') => app.player.toggle_pause(),
        KeyCode::Char('n') => app.play_next().await,
        KeyCode::Char('p') => app.play_prev().await,
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

async fn handle_search(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.do_search().await;
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