use crate::app::{App, InputMode, Tab};
use crate::player::PlayerState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, ListState,
        Padding, Paragraph, Row, Table, Wrap,
    },
    Frame,
};

// ─── Paleta de colores ────────────────────────────────────────────────────
// Estética: terminal audiophile oscura — como un VU meter de alta gama

const BG:        Color = Color::Rgb(10, 10, 14);      // negro azulado profundo
const BG2:       Color = Color::Rgb(18, 18, 26);      // superficie elevada
const BG3:       Color = Color::Rgb(28, 28, 40);      // hover / seleccionado
const ACCENT:    Color = Color::Rgb(99, 202, 183);    // teal vibrante
const ACCENT2:   Color = Color::Rgb(180, 110, 255);   // púrpura eléctrico
const GOLD:      Color = Color::Rgb(255, 200, 80);    // ámbar / HiRes
const TEXT:      Color = Color::Rgb(210, 208, 220);   // texto principal
const MUTED:     Color = Color::Rgb(110, 108, 130);   // texto secundario
const DIM:       Color = Color::Rgb(60, 58, 80);      // bordes / separadores
const RED:       Color = Color::Rgb(255, 85, 100);    // error / pausa

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Fondo global
    f.render_widget(
        Block::default().style(Style::default().bg(BG)),
        area,
    );

    // Layout principal: header | body | player bar | status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // header / logo
            Constraint::Min(0),      // contenido
            Constraint::Length(5),   // player bar
            Constraint::Length(1),   // status line
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_player(f, app, chunks[2]);
    draw_status(f, app, chunks[3]);

    // Overlay de login si está en proceso
    if app.device_code.is_some() {
        draw_login_overlay(f, app, area);
    }
}

// ─── Header ──────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),  // logo
            Constraint::Min(0),      // tabs
            Constraint::Length(22),  // calidad / auth
        ])
        .split(area);

    // Logo
    let logo = Paragraph::new(Line::from(vec![
        Span::styled("◈ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("TIDAL", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(" TUI", Style::default().fg(MUTED)),
    ]))
    .block(Block::default()
        .borders(Borders::BOTTOM | Borders::RIGHT)
        .border_style(Style::default().fg(DIM))
        .padding(Padding::horizontal(1)));
    f.render_widget(logo, cols[0]);

    // Tabs
    let tab_items: Vec<Span> = vec![
        tab_span("Buscar", Tab::Search, &app.active_tab),
        Span::styled("  ", Style::default()),
        tab_span("Cola", Tab::Queue, &app.active_tab),
    ];
    let tabs = Paragraph::new(Line::from(tab_items))
        .block(Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(DIM))
            .padding(Padding::new(1, 0, 1, 0)));
    f.render_widget(tabs, cols[1]);

    // Indicador calidad + auth
    let quality_label = app.tidal.quality.label();
    let auth_icon = if app.authenticated {
        Span::styled("● ", Style::default().fg(ACCENT))
    } else {
        Span::styled("○ ", Style::default().fg(RED))
    };
    let quality_info = Paragraph::new(Line::from(vec![
        auth_icon,
        Span::styled(quality_label, Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Right)
    .block(Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT)
        .border_style(Style::default().fg(DIM))
        .padding(Padding::horizontal(1)));
    f.render_widget(quality_info, cols[2]);
}

fn tab_span<'a>(label: &'a str, tab: Tab, active: &Tab) -> Span<'a> {
    if &tab == active {
        Span::styled(
            format!(" {label} "),
            Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {label} "),
            Style::default().fg(MUTED),
        )
    }
}

// ─── Body ─────────────────────────────────────────────────────────────────

fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    match app.active_tab {
        Tab::Search => draw_search_tab(f, app, area),
        Tab::Queue  => draw_queue_tab(f, app, area),
    }
}

fn draw_search_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // search box
            Constraint::Min(0),     // results
        ])
        .split(area);

    // Buscador
    let is_searching = app.input_mode == InputMode::Search;
    let border_color = if is_searching { ACCENT } else { DIM };
    let prefix = if is_searching {
        Span::styled("/ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("/ ", Style::default().fg(MUTED))
    };
    let cursor = if is_searching {
        Span::styled("▎", Style::default().fg(ACCENT).add_modifier(Modifier::SLOW_BLINK))
    } else {
        Span::raw("")
    };
    let hint = if app.search_input.is_empty() && !is_searching {
        Span::styled("Presiona / para buscar...", Style::default().fg(DIM))
    } else {
        Span::raw("")
    };

    let search_box = Paragraph::new(Line::from(vec![
        prefix,
        Span::styled(app.search_input.as_str(), Style::default().fg(TEXT)),
        cursor,
        hint,
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1)));
    f.render_widget(search_box, chunks[0]);

    // Resultados
    draw_track_list(f, app, chunks[1], &app.search_results, app.selected, "Resultados");
}

fn draw_queue_tab(f: &mut Frame, app: &App, area: Rect) {
    draw_track_list(f, app, area, &app.queue, app.selected, "Cola de reproducción");
}

fn draw_track_list(
    f: &mut Frame,
    app: &App,
    area: Rect,
    tracks: &[crate::tidal::Track],
    selected: usize,
    title: &str,
) {
    if tracks.is_empty() {
        let msg = if app.loading {
            "  Cargando..."
        } else if !app.authenticated {
            "  Presiona L para iniciar sesión en Tidal"
        } else {
            "  Sin resultados — busca con /"
        };

        let p = Paragraph::new(msg)
            .style(Style::default().fg(MUTED))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM))
                .title(Span::styled(
                    format!(" {title} "),
                    Style::default().fg(MUTED),
                ))
                .padding(Padding::new(1, 1, 1, 0)));
        f.render_widget(p, area);
        return;
    }

    let now_playing_id = app.queue_index
        .and_then(|i| app.queue.get(i))
        .map(|t| t.id);

    let rows: Vec<Row> = tracks.iter().enumerate().map(|(i, t)| {
        let is_selected    = i == selected;
        let is_now_playing = Some(t.id) == now_playing_id;

        let row_bg = if is_selected {
            BG3
        } else if i % 2 == 0 {
            BG
        } else {
            BG2
        };

        let play_icon = if is_now_playing {
            Span::styled("▶ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("  ", Style::default())
        };

        let quality_icon = Span::styled(
            format!("{} ", t.quality_icon()),
            Style::default().fg(match t.audio_quality.as_deref() {
                Some("HI_RES_LOSSLESS") => GOLD,
                Some("LOSSLESS")        => ACCENT,
                _                       => MUTED,
            }),
        );

        let num_cell = Cell::from(
            Span::styled(
                format!("{:>3}", i + 1),
                Style::default().fg(MUTED),
            )
        );

        let title_cell = Cell::from(Line::from(vec![
            play_icon,
            quality_icon,
            Span::styled(
                truncate(&t.title, 36),
                Style::default()
                    .fg(if is_now_playing { ACCENT } else { TEXT })
                    .add_modifier(if is_now_playing { Modifier::BOLD } else { Modifier::empty() }),
            ),
        ]));

        let artist_cell = Cell::from(
            Span::styled(truncate(&t.artist_names(), 24), Style::default().fg(MUTED))
        );

        let album_cell = Cell::from(
            Span::styled(truncate(&t.album.title, 22), Style::default().fg(DIM))
        );

        let dur_cell = Cell::from(
            Span::styled(t.duration_str(), Style::default().fg(MUTED))
        );

        Row::new(vec![num_cell, title_cell, artist_cell, album_cell, dur_cell])
            .style(Style::default().bg(row_bg))
            .height(1)
    }).collect();

    let header = Row::new(vec![
        Cell::from(Span::styled("  #", Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("  Título",   Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Artista",    Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Álbum",      Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Dur.",       Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
    ])
    .style(Style::default().bg(BG2))
    .height(1);

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(30),
            Constraint::Length(24),
            Constraint::Length(22),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(
            format!(" {title} ({}) ", tracks.len()),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::ZERO))
    .row_highlight_style(Style::default().bg(BG3))
    .highlight_symbol("  ");

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(selected));
    f.render_stateful_widget(table, area, &mut state);
}

// ─── Player Bar ──────────────────────────────────────────────────────────

fn draw_player(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(BG2));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // track info
            Constraint::Length(1),  // progress bar
            Constraint::Length(1),  // controls hint
        ])
        .split(area);

    // Track info + estado
    let (state_icon, state_color) = match app.player.state {
        PlayerState::Playing => ("▶", ACCENT),
        PlayerState::Paused  => ("⏸", GOLD),
        PlayerState::Stopped => ("■", MUTED),
    };

    let track_line = if let Some(info) = &app.player.current {
        let quality_tag = format!(
            "{}bit/{}kHz {}",
            info.bit_depth,
            info.sample_rate / 1000,
            info.codec.to_uppercase()
        );
        Line::from(vec![
            Span::styled(
                format!(" {state_icon} "),
                Style::default().fg(state_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate(&info.title, 30),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  —  ", Style::default().fg(DIM)),
            Span::styled(truncate(&info.artist, 24), Style::default().fg(MUTED)),
            Span::styled("   ", Style::default()),
            Span::styled(
                quality_tag,
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   {}  vol {}%", app.player.elapsed_str(), app.player.volume),
                Style::default().fg(MUTED),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {state_icon} "),
                Style::default().fg(MUTED),
            ),
            Span::styled("Sin reproducción", Style::default().fg(DIM)),
        ])
    };

    f.render_widget(Paragraph::new(track_line), inner[0]);

    // Barra de progreso
    let progress = app.player.progress();
    let duration_str = app.player.current
        .as_ref()
        .map(|t| format!("{}:{:02}", t.duration / 60, t.duration % 60))
        .unwrap_or_else(|| "0:00".to_string());

    // Dibujar barra de progreso manual con Unicode
    let bar_area = inner[1];
    let bar_width = bar_area.width.saturating_sub(12) as usize;
    let filled = (progress * bar_width as f64) as usize;
    let rest   = bar_width.saturating_sub(filled);

    let bar = format!(
        "{}  {}{}  {}",
        app.player.elapsed_str(),
        "━".repeat(filled),
        "╌".repeat(rest),
        duration_str,
    );

    let bar_spans = vec![
        Span::styled(
            format!("{}  ", app.player.elapsed_str()),
            Style::default().fg(MUTED),
        ),
        Span::styled("━".repeat(filled), Style::default().fg(ACCENT)),
        Span::styled("╌".repeat(rest),   Style::default().fg(DIM)),
        Span::styled(
            format!("  {duration_str}"),
            Style::default().fg(MUTED),
        ),
    ];
    let _ = bar; // evitar warning
    f.render_widget(Paragraph::new(Line::from(bar_spans)), bar_area);

    // Hints de teclas
    let hints = Line::from(vec![
        hint_key("Enter", "reproducir"),
        hint_key("Space", "pausa"),
        hint_key("n/p", "sig/ant"),
        hint_key("←/→", "seek"),
        hint_key("+/-", "volumen"),
        hint_key("Tab", "cola"),
        hint_key("1/2/3", "calidad"),
        hint_key("q", "salir"),
    ]);
    f.render_widget(Paragraph::new(hints), inner[2]);
}

fn hint_key<'a>(key: &'a str, desc: &'a str) -> Span<'a> {
    // Usamos un closure que produce spans — aquí los concatenamos en un solo span por sencillez
    Span::styled(
        format!("  [{key}]{desc}"),
        Style::default().fg(DIM),
    )
}

// ─── Status bar ──────────────────────────────────────────────────────────

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let (icon, color) = if app.loading {
        ("◌", GOLD)
    } else if app.status_msg.starts_with("✓") || app.status_msg.starts_with("▶") {
        ("●", ACCENT)
    } else if app.status_msg.starts_with("✗") {
        ("●", RED)
    } else {
        ("○", MUTED)
    };

    let line = Line::from(vec![
        Span::styled(format!(" {icon} "), Style::default().fg(color)),
        Span::styled(app.status_msg.as_str(), Style::default().fg(TEXT)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(BG)),
        area,
    );
}

// ─── Login overlay ───────────────────────────────────────────────────────

fn draw_login_overlay(f: &mut Frame, app: &App, area: Rect) {
    // Centrar overlay
    let popup = centered_rect(60, 12, area);

    // Fondo borroso (limpiar área)
    f.render_widget(Clear, popup);

    let user_code = app.user_code.as_deref().unwrap_or("...");
    let auth_url  = app.auth_url.as_deref().unwrap_or("...");

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Inicia sesión en Tidal\n", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. Abre este URL: ", Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled(format!("     {auth_url}"), Style::default().fg(ACCENT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  2. Introduce el código: ", Style::default().fg(MUTED)),
            Span::styled(
                user_code,
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Esperando autorización...", Style::default().fg(DIM)),
        ]),
    ];

    let popup_widget = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(ACCENT))
            .title(Span::styled(
                " ◈ Autenticación ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(BG2)))
        .wrap(Wrap { trim: false });

    f.render_widget(popup_widget, popup);
}

// ─── Utilidades ──────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        format!("{}…", chars[..max.saturating_sub(1)].iter().collect::<String>())
    }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width  = area.width * percent_x / 100;
    let popup_x      = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y      = (area.height.saturating_sub(height)) / 2;
    Rect::new(popup_x, popup_y, popup_width, height)
}