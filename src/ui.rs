use crate::{App, astro};
use ratatui::{
    prelude::*,
    widgets::{
        canvas::{Canvas, Circle, Line as SkyLine, Points},
        *,
    },
};
const BG: Color = Color::Rgb(12, 11, 22);
const PANEL: Color = Color::Rgb(18, 16, 31);
const DIM: Color = Color::Rgb(112, 105, 139);
const INK: Color = Color::Rgb(223, 218, 239);
const AMBER: Color = Color::Rgb(236, 186, 113);
const MINT: Color = Color::Rgb(153, 214, 197);
fn block(title: impl Into<Line<'static>>, accent: Color) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(64, 51, 87)))
        .title_style(Style::default().fg(accent))
        .style(Style::default().bg(PANEL))
}
fn text(s: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(s.into(), Style::default().fg(color))
}
pub fn draw(f: &mut Frame, app: &mut App) {
    let phase = app.tick as f64 * 0.035;
    let accent = Color::Rgb(
        (184.0 + 24.0 * phase.sin()) as u8,
        (132.0 + 15.0 * (phase + 1.0).sin()) as u8,
        244,
    );
    f.render_widget(
        Block::default().style(Style::default().bg(BG).fg(INK)),
        f.area(),
    );
    app.buttons.clear();
    app.table_area = Rect::default();
    app.map_area = Rect::default();
    if f.area().width < 76 || f.area().height < 26 {
        f.render_widget(Paragraph::new("NOCTILUCA / OBSERVATORY\n\nExpand the terminal to at least 76 × 26.\nRecommended: 120 × 40.\n\nQ quit · Ctrl+C exit").style(Style::default().fg(accent)).block(block(" SIGNAL CONSTRAINED ",accent)),f.area());
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(5),
        Constraint::Length(3),
    ])
    .margin(1)
    .split(f.area());
    let header = Layout::horizontal([Constraint::Min(42), Constraint::Length(36)]).split(rows[0]);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                text("▌ ", AMBER),
                Span::styled("N O C T I L U C A", Style::default().fg(INK).bold()),
                text("   /   FIELD OBSERVATORY", DIM),
            ]),
            Line::from(vec![
                text("  ▰▰ ", AMBER),
                text("▰▰ ", accent),
                text("▰▰ ", MINT),
                text("  A SMALL WINDOW INTO THE INFINITE", DIM),
            ]),
            Line::from(text(
                "  ⊹  ⌖  ⋄  ⟡  ∷     C E L E S T I A L   R E C E I V E R",
                accent,
            )),
        ]),
        header[0],
    );
    let date = chrono::DateTime::from_timestamp(app.now(), 0).unwrap();
    f.render_widget(
        Paragraph::new(vec![
            Line::from(text(
                if app.demo {
                    "◈ DEMO TRANSMISSION"
                } else {
                    "● LOCAL SKY / LIVE CLOCK"
                },
                if app.demo { AMBER } else { MINT },
            )),
            Line::from(text(
                date.format("%d %b %Y   %H:%M:%S UTC").to_string(),
                INK,
            )),
            Line::from(text("VOL. 01                RX / HYG 4.1", DIM)),
        ])
        .alignment(Alignment::Right),
        header[1],
    );
    let loc = app
        .location
        .as_ref()
        .map(|l| format!("⌖ {}   {:.3}°, {:.3}°", l.label, l.lat, l.lon))
        .unwrap_or("⌖ Locating…  L to enter coordinates".into());
    f.render_widget(
        Paragraph::new(Line::from(vec![
            text(format!(" {loc}"), INK),
            text(
                format!("    /    {} ABOVE HORIZON", app.visible.len()),
                accent,
            ),
        ]))
        .block(block(" OBSERVER ", accent)),
        rows[1],
    );
    let columns =
        Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)]).split(rows[2]);
    render_list(f, app, columns[0], accent);
    let right = Layout::vertical([Constraint::Min(6), Constraint::Length(7)]).split(columns[1]);
    render_map(f, app, right[0], accent);
    render_detail(f, app, right[1], accent);
    let sources =
        Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)]).split(rows[3]);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(text(format!(" {}", app.catalog_status), MINT)),
            Line::from(text(format!(" {}", app.network_status), DIM)),
            Line::from(text(
                " Geometric horizon ≠ visible in daylight or cloud",
                AMBER,
            )),
        ])
        .block(block(" 03 / SOURCE TELEMETRY ", accent)),
        sources[0],
    );
    f.render_widget(
        Paragraph::new(vec![
            Line::from(text(
                format!(
                    " {}",
                    if app.offline {
                        "Weather not polled · offline session"
                    } else {
                        &app.weather
                    }
                ),
                INK,
            )),
            Line::from(text(
                " Open-Meteo · conditions, not a visibility guarantee",
                DIM,
            )),
            Line::from(text(format!(" {}", app.message), accent)),
        ])
        .block(block(" CONDITIONS / COMMS ", accent)),
        sources[1],
    );
    let buttons = [
        (" / SEARCH ", '/'),
        (" L LOCATE ", 'l'),
        (" S SORT ", 's'),
        (" H ALT>20 ", 'h'),
        (" − DIM ", '-'),
        (" + FAINT ", '+'),
        (" R SYNC ", 'r'),
        (" ? HELP ", '?'),
    ];
    let parts = Layout::horizontal(buttons.map(|_| Constraint::Ratio(1, 8))).split(rows[4]);
    for ((label, c), area) in buttons.into_iter().zip(parts.iter()) {
        app.buttons.push((*area, c));
        f.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(Style::default().fg(if c == 'h' && app.high_only {
                    AMBER
                } else {
                    accent
                }))
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(DIM)),
                ),
            *area,
        );
    }
    if app.help {
        let area = Rect::new(
            (f.area().width - 70) / 2,
            (f.area().height - 17) / 2,
            70,
            17,
        );
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(text(" A field guide to your observatory", AMBER)),
                Line::from(""),
                Line::from(" ↑ / ↓ or J / K     Select a star"),
                Line::from(" Mouse wheel         Scroll the star list"),
                Line::from(" Click               Select a row, sky target, or bottom control"),
                Line::from(" /                   Search by name, HYG ID, or constellation"),
                Line::from(" L                   Enter latitude, longitude"),
                Line::from(" S                   Sort by brightness / altitude"),
                Line::from(" H                   Toggle the 20° altitude threshold"),
                Line::from(" + / -               Include fainter / brighter stars"),
                Line::from(" R                   Refresh public data (online sessions)"),
                Line::from(" Esc                 Clear search / close this guide"),
                Line::from(" Q or Ctrl+C         Exit"),
                Line::from(""),
                Line::from(text(
                    " Geometric positions; terrain, refraction, and glare not modeled.",
                    DIM,
                )),
            ])
            .block(block(" OPERATOR GUIDE / ? TO CLOSE ", accent)),
            area,
        );
    }
    if let Some(location) = app.editing {
        let area = Rect::new(
            f.area().width.saturating_sub(64) / 2,
            f.area().height / 2 - 3,
            64,
            7,
        );
        f.render_widget(Clear, area);
        let title = if location {
            " SET OBSERVER / decimal degrees "
        } else {
            " FILTER SIGNALS / name or constellation "
        };
        f.render_widget(
            Paragraph::new(vec![
                Line::from(text(
                    if location {
                        " Latitude, longitude  •  north/east are positive"
                    } else {
                        " Search the stars above your horizon"
                    },
                    DIM,
                )),
                Line::from(text(format!(" > {}▏", app.input), INK)),
                Line::from(""),
                Line::from(text(" Enter apply   /   Esc cancel", accent)),
                Line::from(text(
                    if location {
                        " Example: 37.7749, -122.4194"
                    } else {
                        " Empty search shows all stars"
                    },
                    DIM,
                )),
            ])
            .block(block(title, accent)),
            area,
        );
    }
}
fn render_list(f: &mut Frame, app: &mut App, area: Rect, accent: Color) {
    app.table_area = area;
    let rows = app.visible.iter().map(|v| {
        let s = &app.stars[v.index];
        Row::new(vec![
            Cell::from(s.name()).style(Style::default().fg(if s.proper.is_empty() {
                DIM
            } else {
                INK
            })),
            Cell::from(s.con.clone()).style(Style::default().fg(DIM)),
            Cell::from(format!("{:.1}", s.mag)).style(Style::default().fg(AMBER)),
            Cell::from(format!("{:>4.1}°", v.alt)),
            Cell::from(astro::direction(v.az)),
        ])
    });
    let title = format!(" 01 / ABOVE YOU   ·   MAG ≤ {:.1} ", app.limit);
    let table = Table::new(
        rows,
        [
            Constraint::Min(13),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(7),
            Constraint::Length(3),
        ],
    )
    .header(
        Row::new(["STAR", "CON", "MAG", "ALT", "AZ"])
            .style(Style::default().fg(DIM))
            .bottom_margin(1),
    )
    .block(block(title, accent).title_bottom(Line::from(text(
        format!(
            " {} · {} signals {} ",
            if app.sort_alt {
                "ALTITUDE ↓"
            } else {
                "BRIGHTEST FIRST"
            },
            app.visible.len(),
            if app.query.is_empty() {
                String::new()
            } else {
                format!("/ {}", app.query)
            }
        ),
        DIM,
    ))))
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(57, 38, 83))
            .fg(INK)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ")
    .column_spacing(1);
    f.render_stateful_widget(table, area, &mut app.table);
    if app.visible.is_empty() {
        let inner = Rect::new(
            area.x + 3,
            area.y + 5,
            area.width.saturating_sub(6),
            area.height.saturating_sub(7),
        );
        f.render_widget(Paragraph::new(if app.location.is_none(){"NO OBSERVER FIX\n\nPress L to enter latitude, longitude.\nIP geolocation may be unavailable."}else{"NO MATCHING SIGNALS\n\nTry + for fainter stars, H to include\nthe horizon, or Esc to clear search."}).style(Style::default().fg(DIM)),inner);
    }
}
fn render_map(f: &mut Frame, app: &mut App, area: Rect, accent: Color) {
    let b = block(" 02 / SKY PROJECTION ", accent).title_bottom(Line::from(text(
        " LOOKING UP · E LEFT · CLICK TO LOCK ",
        DIM,
    )));
    let inner = b.inner(area);
    let width = inner.width.min(inner.height.saturating_mul(2));
    app.map_area = Rect::new(
        inner.x + (inner.width - width) / 2,
        inner.y,
        width,
        inner.height,
    );
    f.render_widget(b, area);
    let selected = app.table.selected();
    let canvas = Canvas::default()
        .background_color(PANEL)
        .marker(symbols::Marker::Braille)
        .x_bounds([-1.15, 1.15])
        .y_bounds([-1.15, 1.15])
        .paint(|ctx| {
            for radius in [1.0, 0.6667, 0.3333] {
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius,
                    color: Color::Rgb(60, 46, 82),
                });
            }
            for (x, y) in [(1.0, 0.0), (0.0, 1.0)] {
                ctx.draw(&SkyLine {
                    x1: -x,
                    y1: -y,
                    x2: x,
                    y2: y,
                    color: Color::Rgb(39, 32, 57),
                });
            }
            ctx.print(-0.02, 1.06, text("N", AMBER));
            ctx.print(-0.02, -1.09, text("S", DIM));
            ctx.print(-1.09, 0.0, text("E", DIM));
            ctx.print(1.04, 0.0, text("W", DIM));
            ctx.print(0.03, 0.04, text("+", DIM));
            for (i, v) in app.visible.iter().enumerate() {
                let (x, y) = astro::project(v.alt, v.az);
                let s = &app.stars[v.index];
                if Some(i) == selected {
                    ctx.draw(&Circle {
                        x,
                        y,
                        radius: 0.045,
                        color: AMBER,
                    });
                    ctx.print(x, y, text("✦", AMBER));
                    ctx.print((x + 0.07).min(0.60), y + 0.09, text(s.name(), AMBER));
                } else if s.mag < 2.0 {
                    ctx.print(x, y, text("✧", accent));
                } else {
                    ctx.draw(&Points {
                        coords: &[(x, y)],
                        color: if s.mag < 3.5 { accent } else { DIM },
                    });
                }
            }
        });
    f.render_widget(canvas, app.map_area);
}
fn render_detail(f: &mut Frame, app: &App, area: Rect, accent: Color) {
    let mut lines = vec![Line::from(text(
        " No target locked · select a star from the list or map",
        DIM,
    ))];
    if let Some(v) = app.table.selected().and_then(|i| app.visible.get(i)) {
        let s = &app.stars[v.index];
        lines = vec![
            Line::from(vec![
                text(format!(" ✦ {}", s.name()), AMBER),
                text(format!("   /   {}   /   {}", s.con, s.spect), DIM),
            ]),
            Line::from(text(
                format!(
                    " ALT {:05.1}°   AZ {:05.1}° {}   MAG {:+.2}",
                    v.alt,
                    v.az,
                    astro::direction(v.az),
                    s.mag
                ),
                INK,
            )),
            Line::from(text(
                format!(" RA {:07.3}h   DEC {:+07.3}°   [J2000]", s.ra, s.dec),
                DIM,
            )),
            Line::from(text(
                if s.dist >= 100000.0 {
                    " Distance unavailable".into()
                } else {
                    format!(" {:.1} light-years from home", s.dist * 3.26156)
                },
                accent,
            )),
            Line::from(text(" ⟡  ∷  ⋄  ⌖     SIGNAL RESOLVED", accent)),
        ];
    }
    f.render_widget(
        Paragraph::new(lines).block(block(" TARGET / INSPECTOR ", accent)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[test]
    fn renders_and_handles_small_terminals() {
        let args = crate::Args::parse_from(["test", "--demo"]);
        let mut app = App::new(&args);
        for (w, h) in [(120, 40), (80, 26), (40, 10), (1, 1)] {
            let mut t = Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            t.draw(|f| draw(f, &mut app)).unwrap();
            if w == 120 {
                assert!(app.visible.len() > 20);
                assert_eq!(app.buttons.len(), 8);
            }
        }
    }
    #[test]
    fn mouse_selects_rows_map_and_controls() {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let mut app = App::new(&crate::Args::parse_from(["test", "--demo"]));
        let mut t = Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
        t.draw(|f| draw(f, &mut app)).unwrap();
        let click = |x, y| MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        };
        app.mouse(click(app.table_area.x + 5, app.table_area.y + 4));
        assert_eq!(app.table.selected(), Some(1));
        let r = app.buttons.iter().find(|(_, c)| *c == 'h').unwrap().0;
        app.mouse(click(r.x + 1, r.y + 1));
        assert!(app.high_only);
        assert!(app.visible.iter().all(|v| v.alt > 20.0));
        let target = app.visible[0].clone();
        let (x, y) = astro::project(target.alt, target.az);
        let r = app.map_area;
        let column = r.x + (((x + 1.15) / 2.3) * (r.width - 1) as f64).round() as u16;
        let row = r.y + (((1.15 - y) / 2.3) * (r.height - 1) as f64).round() as u16;
        app.table.select(None);
        app.mouse(click(column, row));
        assert!(app.table.selected().is_some());
    }
    #[test]
    fn filters_and_location_validation() {
        let mut app = App::new(&crate::Args::parse_from(["test", "--demo"]));
        app.query = "no such star".into();
        app.recompute();
        assert!(app.visible.is_empty());
        assert!(app.table.selected().is_none());
        app.editing = Some(true);
        app.input = "NaN, 0".into();
        app.submit();
        assert!(app.editing.is_some());
        app.input = "51.5, -0.1".into();
        app.submit();
        assert_eq!(app.location.unwrap().lat, 51.5);
    }
}
