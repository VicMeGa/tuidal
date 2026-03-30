//use crate::app::{App, InputMode, Tab};
use crate::app::{App, InputMode, Tab, CollectionView};
use crate::player::PlayerState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear,
        Padding, Paragraph, Row, Table, Wrap,
    },
    Frame,
};
//use ratatui_image::{picker::Picker, StatefulImage, Resize};
use ratatui_image::{StatefulImage, Resize};

const BG:      Color = Color::Rgb(10, 10, 14);
const BG2:     Color = Color::Rgb(18, 18, 26);
const BG3:     Color = Color::Rgb(28, 28, 40);
const ACCENT:  Color = Color::Rgb(99, 202, 183);
const ACCENT2: Color = Color::Rgb(180, 110, 255);
const GOLD:    Color = Color::Rgb(255, 200, 80);
const TEXT:    Color = Color::Rgb(210, 208, 220);
const MUTED:   Color = Color::Rgb(110, 108, 130);
const DIM:     Color = Color::Rgb(60, 58, 80);
const RED:     Color = Color::Rgb(255, 85, 100);

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_player(f, app, chunks[2]);
    draw_status(f, app, chunks[3]);

    if app.device_code.is_some() {
        draw_login_overlay(f, app, area);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(0),
            Constraint::Length(22),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("◈ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("TIDAL", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(" TUI", Style::default().fg(MUTED)),
        ]))
        .block(Block::default()
            .borders(Borders::BOTTOM | Borders::RIGHT)
            .border_style(Style::default().fg(DIM))
            .padding(Padding::horizontal(1))),
        cols[0],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            tab_span("Buscar", Tab::Search, &app.active_tab),
            Span::styled("  ", Style::default()),
            tab_span("Cola", Tab::Queue, &app.active_tab),
            Span::styled("  ", Style::default()),
            tab_span("Ahora", Tab::Now, &app.active_tab),
            Span::styled("  ", Style::default()),
            tab_span("Biblioteca", Tab::Library, &app.active_tab),
        ]))
        .block(Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(DIM))
            .padding(Padding::new(1, 0, 1, 0))),
        cols[1],
    );

    let auth_icon = if app.authenticated {
        Span::styled("● ", Style::default().fg(ACCENT))
    } else {
        Span::styled("○ ", Style::default().fg(RED))
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            auth_icon,
            Span::styled(app.tidal.quality.label(), Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
        ]))
        .alignment(Alignment::Right)
        .block(Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT)
            .border_style(Style::default().fg(DIM))
            .padding(Padding::horizontal(1))),
        cols[2],
    );
}

fn tab_span<'a>(label: &'a str, tab: Tab, active: &Tab) -> Span<'a> {
    if &tab == active {
        Span::styled(format!(" {label} "), Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(MUTED))
    }
}

fn draw_body(f: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        Tab::Search => draw_search_tab(f, app, area),
        Tab::Queue  => draw_queue_tab(f, app, area),
        Tab::Now    => draw_now_tab(f, app, area),
        Tab::Library => draw_library_tab(f, app, area),
    }
}

fn draw_search_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let is_searching = app.input_mode == InputMode::Search;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("/ ", Style::default()
                .fg(if is_searching { ACCENT } else { MUTED })
                .add_modifier(Modifier::BOLD)),
            Span::styled(app.search_input.as_str(), Style::default().fg(TEXT)),
            if is_searching {
                Span::styled("▎", Style::default().fg(ACCENT).add_modifier(Modifier::SLOW_BLINK))
            } else { Span::raw("") },
            if app.search_input.is_empty() && !is_searching {
                Span::styled("Presiona / para buscar...", Style::default().fg(DIM))
            } else { Span::raw("") },
        ]))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if is_searching { ACCENT } else { DIM }))
            .padding(Padding::horizontal(1))),
        chunks[0],
    );

    draw_track_list(f, app, chunks[1], &app.search_results.clone(), app.selected, "Resultados");
}

fn draw_queue_tab(f: &mut Frame, app: &App, area: Rect) {
    draw_track_list(f, app, area, &app.queue.clone(), app.selected, "Cola de reproducción");
}

fn draw_now_tab(f: &mut Frame, app: &mut App, area: Rect) {
    if app.player.current.is_none() {
        f.render_widget(
            Paragraph::new("\n\n  Sin reproducción — presiona Enter en una canción")
                .style(Style::default().fg(MUTED))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(DIM))
                    .title(Span::styled(" ◈ Ahora reproduciendo ", Style::default().fg(MUTED)))),
            area,
        );
        return;
    }

    let img_cols = area.height.saturating_sub(2); // aprox cuadrado en celdas
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(img_cols),   // cuadrado
            Constraint::Min(0),             // resto para info
        ])
        .split(area);

    let img_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(BG2));
    let img_inner = img_block.inner(cols[0]);
    f.render_widget(img_block, cols[0]);

    // Recrear proto si cambió el área o la imagen
    let area_size = (img_inner.width, img_inner.height);
     // Recrear el proto si cambió el área O si no existe
    // (el picker ya fue creado antes de raw mode, así que es válido)
    if app.cover_proto.is_none() || app.last_img_area != Some(area_size) {
        app.last_img_area = Some(area_size);
        app.cover_proto = None; // descartar el anterior

        if let (Some(picker), Some(img)) = (&app.picker, &app.cover_image) {
            // Escalar la imagen al tamaño del área ANTES de crear el protocol
            // Esto evita que ratatui-image tenga que adivinar el tamaño
            let (cols, rows) = (img_inner.width as u32, img_inner.height as u32);
            let font_size    = picker.font_size(); // (ancho_px, alto_px) por celda
            let target_w     = cols * font_size.0 as u32;
            let target_h     = rows * font_size.1 as u32;

            let scaled = img.resize(
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3,
            );
            app.cover_proto = Some(picker.new_resize_protocol(scaled));
        }
    }

    if let Some(ref mut proto) = app.cover_proto {
        let widget = StatefulImage::new().resize(Resize::Fit(None));
        f.render_stateful_widget(widget, img_inner, proto);
    } else {
        f.render_widget(
            Paragraph::new("\n\n  ⟳ Cargando\n  imagen...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(MUTED)),
            img_inner,
        );
    }

    // Info
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" ◈ Ahora reproduciendo ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG2));
    let info_inner = info_block.inner(cols[1]);
    f.render_widget(info_block, cols[1]);

    if let Some(ref track) = app.player.current {
        let max_w = (info_inner.width as usize).saturating_sub(4);
        let progress  = app.player.progress();
        let bar_width = info_inner.width.saturating_sub(14) as usize;
        let filled    = (progress * bar_width as f64) as usize;
        let rest      = bar_width.saturating_sub(filled);
        let duration_str = format!("{}:{:02}", track.duration / 60, track.duration % 60);

        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(format!("  {}", truncate(&track.title, max_w)), Style::default().fg(TEXT).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(Span::styled(format!("  {}", truncate(&track.artist, max_w)), Style::default().fg(ACCENT))),
                Line::from(Span::styled(format!("  {}", truncate(&track.album, max_w)), Style::default().fg(MUTED))),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}bit / {}kHz  {}", track.bit_depth, track.sample_rate / 1000, track.codec.to_uppercase()),
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(""),
                Line::from(vec![
                    Span::styled(format!("  {} ", app.player.elapsed_str()), Style::default().fg(MUTED)),
                    Span::styled("━".repeat(filled), Style::default().fg(ACCENT)),
                    Span::styled("╌".repeat(rest),   Style::default().fg(DIM)),
                    Span::styled(format!(" {duration_str}"), Style::default().fg(MUTED)),
                ]),
            ]),
            info_inner,
        );
    }
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
        let msg = if app.loading { "  ⟳ Cargando..." }
            else if !app.authenticated { "  Presiona L para iniciar sesión en Tidal" }
            else { "  Sin resultados — busca con /" };
        f.render_widget(
            Paragraph::new(msg)
                .style(Style::default().fg(MUTED))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(DIM))
                    .title(Span::styled(format!(" {title} "), Style::default().fg(MUTED)))
                    .padding(Padding::new(1, 1, 1, 0))),
            area,
        );
        return;
    }

    let now_playing_id = app.queue_index.and_then(|i| app.queue.get(i)).map(|t| t.id);

    let rows: Vec<Row> = tracks.iter().enumerate().map(|(i, t)| {
        let is_selected    = i == selected;
        let is_now_playing = Some(t.id) == now_playing_id;
        let row_bg = if is_selected { BG3 } else if i % 2 == 0 { BG } else { BG2 };

        Row::new(vec![
            Cell::from(Span::styled(format!("{:>3}", i + 1), Style::default().fg(MUTED))),
            Cell::from(Line::from(vec![
                if is_now_playing {
                    Span::styled("▶ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                },
                Span::styled(format!("{} ", t.quality_icon()), Style::default().fg(
                    match t.audio_quality.as_deref() {
                        Some("HI_RES_LOSSLESS") => GOLD,
                        Some("LOSSLESS")        => ACCENT,
                        _                       => MUTED,
                    }
                )),
                Span::styled(
                    truncate(&t.title, 36),
                    Style::default()
                        .fg(if is_now_playing { ACCENT } else { TEXT })
                        .add_modifier(if is_now_playing { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ])),
            Cell::from(Span::styled(truncate(&t.artist_names(), 24), Style::default().fg(MUTED))),
            Cell::from(Span::styled(truncate(&t.album.title, 22), Style::default().fg(DIM))),
            Cell::from(Span::styled(t.duration_str(), Style::default().fg(MUTED))),
        ])
        .style(Style::default().bg(row_bg))
        .height(1)
    }).collect();

    let header = Row::new(vec![
        Cell::from(Span::styled("  #",      Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("  Título", Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Artista",  Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Álbum",    Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Dur.",     Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
    ]).style(Style::default().bg(BG2)).height(1);

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(selected));
    f.render_stateful_widget(
        Table::new(rows, [
            Constraint::Length(4),
            Constraint::Min(30),
            Constraint::Length(24),
            Constraint::Length(22),
            Constraint::Length(6),
        ])
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
        .highlight_symbol("  "),
        area,
        &mut state,
    );
}

fn draw_player(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(DIM))
            .style(Style::default().bg(BG2)),
        area,
    );

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let (state_icon, state_color) = match app.player.state {
        PlayerState::Playing => ("▶", ACCENT),
        PlayerState::Paused  => ("⏸", GOLD),
        PlayerState::Stopped => ("■", MUTED),
    };

    f.render_widget(
        Paragraph::new(if let Some(info) = &app.player.current {
            Line::from(vec![
                Span::styled(format!(" {state_icon} "), Style::default().fg(state_color).add_modifier(Modifier::BOLD)),
                Span::styled(truncate(&info.title, 30), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
                Span::styled("  —  ", Style::default().fg(DIM)),
                Span::styled(truncate(&info.artist, 24), Style::default().fg(MUTED)),
                Span::styled("   ", Style::default()),
                Span::styled(
                    format!("{}bit/{}kHz {}", info.bit_depth, info.sample_rate / 1000, info.codec.to_uppercase()),
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("   {}  vol {}%", app.player.elapsed_str(), app.player.volume), Style::default().fg(MUTED)),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!(" {state_icon} "), Style::default().fg(MUTED)),
                Span::styled("Sin reproducción", Style::default().fg(DIM)),
            ])
        }),
        inner[0],
    );

    let progress     = app.player.progress();
    let duration_str = app.player.current.as_ref()
        .map(|t| format!("{}:{:02}", t.duration / 60, t.duration % 60))
        .unwrap_or_else(|| "0:00".to_string());
    let bar_width = inner[1].width.saturating_sub(12) as usize;
    let filled    = (progress * bar_width as f64) as usize;
    let rest      = bar_width.saturating_sub(filled);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{}  ", app.player.elapsed_str()), Style::default().fg(MUTED)),
            Span::styled("━".repeat(filled), Style::default().fg(ACCENT)),
            Span::styled("╌".repeat(rest),   Style::default().fg(DIM)),
            Span::styled(format!("  {duration_str}"), Style::default().fg(MUTED)),
        ])),
        inner[1],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            hint_key("Enter", "reproducir"),
            hint_key("Space", "pausa"),
            hint_key("n/p", "sig/ant"),
            hint_key("←/→", "seek"),
            hint_key("+/-", "volumen"),
            hint_key("Tab", "vista"),
            hint_key("1/2/3", "calidad"),
            hint_key("q", "salir"),
            hint_key("i", "biblioteca"),
            hint_key("F", "fav tracks"),
            hint_key("A", "fav álbumes"),
            hint_key("q", "salir"),
        ])),
        inner[2],
    );
}

fn hint_key<'a>(key: &'a str, desc: &'a str) -> Span<'a> {
    Span::styled(format!("  [{key}]{desc}"), Style::default().fg(DIM))
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let (icon, color) = if app.loading { ("◌", GOLD) }
        else if app.status_msg.starts_with("✓") || app.status_msg.starts_with("▶") { ("●", ACCENT) }
        else if app.status_msg.starts_with("✗") { ("●", RED) }
        else { ("○", MUTED) };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {icon} "), Style::default().fg(color)),
            Span::styled(app.status_msg.as_str(), Style::default().fg(TEXT)),
        ])).style(Style::default().bg(BG)),
        area,
    );
}

fn draw_login_overlay(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 12, area);
    f.render_widget(Clear, popup);

    let user_code = app.user_code.as_deref().unwrap_or("...");
    let auth_url  = app.auth_url.as_deref().unwrap_or("...");

    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  Inicia sesión en Tidal", Style::default().fg(TEXT).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("  1. Abre este URL:", Style::default().fg(MUTED))),
            Line::from(Span::styled(format!("     {auth_url}"), Style::default().fg(ACCENT))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  2. Código: ", Style::default().fg(MUTED)),
                Span::styled(user_code, Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Esperando autorización...", Style::default().fg(DIM))),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(ACCENT))
            .title(Span::styled(" ◈ Autenticación ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(BG2)))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_library_tab(f: &mut Frame, app: &App, area: Rect) {
    // Si estamos en vista de álbumes favoritos
    //if app.collection_view == app::CollectionView::Albums {
    if app.collection_view == CollectionView::Albums {
        draw_fav_albums(f, app, area);
        return;
    }
    let total = app.playlists.len() + app.mixes.len();
    if total == 0 {
        f.render_widget(
            Paragraph::new(if app.loading {
                "  ⟳ Cargando biblioteca..."
            } else {
                "  Presiona 'i' para cargar playlists y mixes"
            })
            .style(Style::default().fg(MUTED))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM))
                .title(Span::styled(" Biblioteca ", Style::default().fg(MUTED)))),
            area,
        );
        return;
    }

    let items: Vec<Row> = app.playlists.iter().enumerate()
        .map(|(i, p)| {
            let is_sel = i == app.library_selected;
            Row::new(vec![
                Cell::from(Span::styled("≡ ", Style::default().fg(ACCENT2))),
                Cell::from(Span::styled(
                    truncate(&p.title, 40),
                    Style::default().fg(if is_sel { ACCENT } else { TEXT })
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                )),
                Cell::from(Span::styled(
                    format!("{} tracks", p.number_of_tracks),
                    Style::default().fg(MUTED),
                )),
                Cell::from(Span::styled("Playlist", Style::default().fg(DIM))),
            ])
            .style(Style::default().bg(if is_sel { BG3 } else if i % 2 == 0 { BG } else { BG2 }))
        })
        .chain(app.mixes.iter().enumerate().map(|(i, m)| {
            let idx    = app.playlists.len() + i;
            let is_sel = idx == app.library_selected;
            Row::new(vec![
                Cell::from(Span::styled("⊛ ", Style::default().fg(GOLD))),
                Cell::from(Span::styled(
                    truncate(&m.title, 40),
                    Style::default().fg(if is_sel { GOLD } else { TEXT })
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                )),
                Cell::from(Span::styled(
                    m.sub_title.as_deref().unwrap_or("").to_string(),
                    Style::default().fg(MUTED),
                )),
                Cell::from(Span::styled("Mix", Style::default().fg(GOLD))),
            ])
            .style(Style::default().bg(if is_sel { BG3 } else if i % 2 == 0 { BG } else { BG2 }))
        }))
        .collect();

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from(Span::styled("Nombre",  Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Info",    Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Tipo",    Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
    ]).style(Style::default().bg(BG2));

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(app.library_selected));

    f.render_stateful_widget(
        Table::new(items, [
            Constraint::Length(3),
            Constraint::Min(30),
            Constraint::Length(20),
            Constraint::Length(10),
        ])
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(DIM))
            .title(Span::styled(
                format!(" Biblioteca ({} playlists, {} mixes) ", app.playlists.len(), app.mixes.len()),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )))
        .row_highlight_style(Style::default().bg(BG3)),
        area,
        &mut state,
    );
}

fn draw_fav_albums(f: &mut Frame, app: &App, area: Rect) {
    if app.fav_albums.is_empty() {
        f.render_widget(
            Paragraph::new(if app.loading { "  ⟳ Cargando..." } else { "  Sin álbumes favoritos" })
                .style(Style::default().fg(MUTED))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(DIM))
                    .title(Span::styled(" Álbumes favoritos ", Style::default().fg(MUTED)))),
            area,
        );
        return;
    }

    let rows: Vec<Row> = app.fav_albums.iter().enumerate().map(|(i, a)| {
        let is_sel = i == app.fav_album_selected;
        Row::new(vec![
            Cell::from(Span::styled("◆ ", Style::default().fg(ACCENT))),
            Cell::from(Span::styled(
                truncate(&a.title, 40),
                Style::default()
                    .fg(if is_sel { ACCENT } else { TEXT })
                    .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
            )),
            Cell::from(Span::styled(truncate(&a.artist_names(), 28), Style::default().fg(MUTED))),
            Cell::from(Span::styled(
                format!("{} tracks", a.number_of_tracks),
                Style::default().fg(DIM),
            )),
        ])
        .style(Style::default().bg(if is_sel { BG3 } else if i % 2 == 0 { BG } else { BG2 }))
    }).collect();

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from(Span::styled("Álbum",   Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Artista", Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Tracks",  Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD))),
    ]).style(Style::default().bg(BG2));

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(app.fav_album_selected));

    f.render_stateful_widget(
        Table::new(rows, [
            Constraint::Length(3),
            Constraint::Min(30),
            Constraint::Length(28),
            Constraint::Length(10),
        ])
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(DIM))
            .title(Span::styled(
                format!(" ◆ Álbumes favoritos ({}) — Enter para cargar ", app.fav_albums.len()),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )))
        .row_highlight_style(Style::default().bg(BG3)),
        area,
        &mut state,
    );
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max { s.to_string() }
    else { format!("{}…", chars[..max.saturating_sub(1)].iter().collect::<String>()) }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let popup_x     = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y     = (area.height.saturating_sub(height)) / 2;
    Rect::new(popup_x, popup_y, popup_width, height)
}