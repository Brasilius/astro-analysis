mod astro;
mod sources;
mod ui;
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
};
use ratatui::{layout::Rect, widgets::TableState};
use sources::{Location, Request, Update};
use std::{
    io,
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(
    about = "NOCTILUCA — a live terminal observatory. Defaults to approximate IP geolocation."
)]
struct Args {
    #[arg(long, requires = "lon", allow_hyphen_values = true)]
    lat: Option<f64>,
    #[arg(long, requires = "lat", allow_hyphen_values = true)]
    lon: Option<f64>,
    /// Never access the network. Supply coordinates or use the labeled demo location.
    #[arg(long)]
    offline: bool,
    /// Offline San Francisco night-sky demonstration at a fixed time.
    #[arg(long, conflicts_with_all=["lat","lon"])]
    demo: bool,
    /// Render a deterministic 120x40 text preview and exit.
    #[arg(long)]
    snapshot: bool,
}
#[derive(Clone)]
struct Visible {
    index: usize,
    alt: f64,
    az: f64,
}
struct App {
    stars: Vec<astro::Star>,
    visible: Vec<Visible>,
    location: Option<Location>,
    table: TableState,
    limit: f64,
    high_only: bool,
    sort_alt: bool,
    query: String,
    editing: Option<bool>,
    input: String,
    message: String,
    weather: String,
    catalog_status: String,
    network_status: String,
    offline: bool,
    demo: bool,
    busy: bool,
    last_poll: Instant,
    last_catalog: Instant,
    help: bool,
    tick: u64,
    table_area: Rect,
    map_area: Rect,
    buttons: Vec<(Rect, char)>,
    tx: Sender<Request>,
    rx: Receiver<Update>,
}
impl App {
    fn new(args: &Args) -> Self {
        let (tx, rx) = sources::worker();
        let demo = args.demo || args.snapshot || (args.offline && args.lat.is_none());
        let location = if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
            Some(Location {
                lat,
                lon,
                label: "Manual coordinates".into(),
            })
        } else if demo {
            Some(Location {
                lat: 37.7749,
                lon: -122.4194,
                label: "San Francisco · DEMO".into(),
            })
        } else {
            None
        };
        let mut app = Self {
            stars: astro::catalog(include_bytes!("../data/bright_stars.csv"))
                .expect("bundled catalog"),
            visible: vec![],
            location,
            table: TableState::default(),
            limit: 4.0,
            high_only: false,
            sort_alt: false,
            query: String::new(),
            editing: None,
            input: String::new(),
            message: "Click a star to inspect · ? for controls.".into(),
            weather: "Awaiting observing conditions".into(),
            catalog_status: "HYG v4.1 · bundled".into(),
            network_status: "Ready".into(),
            offline: args.offline || demo,
            demo,
            busy: false,
            last_poll: Instant::now(),
            last_catalog: Instant::now(),
            help: false,
            tick: 0,
            table_area: Rect::default(),
            map_area: Rect::default(),
            buttons: vec![],
            tx,
            rx,
        };
        app.recompute();
        app.refresh(true);
        app
    }
    fn now(&self) -> i64 {
        if self.demo {
            1789106400
        } else {
            Utc::now().timestamp()
        }
    }
    fn recompute(&mut self) {
        let selected = self
            .table
            .selected()
            .and_then(|i| self.visible.get(i))
            .map(|v| self.stars[v.index].id);
        self.visible.clear();
        if let Some(l) = &self.location {
            for (index, s) in self.stars.iter().enumerate() {
                if s.mag > self.limit
                    || !format!("{} {}", s.name(), s.con)
                        .to_lowercase()
                        .contains(&self.query.to_lowercase())
                {
                    continue;
                }
                let (alt, az) = astro::horizontal(s.ra, s.dec, l.lat, l.lon, self.now());
                if alt > if self.high_only { 20.0 } else { 0.0 } {
                    self.visible.push(Visible { index, alt, az });
                }
            }
        }
        self.visible.sort_by(|a, b| {
            if self.sort_alt {
                b.alt.total_cmp(&a.alt)
            } else {
                self.stars[a.index].mag.total_cmp(&self.stars[b.index].mag)
            }
        });
        self.table.select(if self.visible.is_empty() {
            None
        } else {
            Some(
                selected
                    .and_then(|id| {
                        self.visible
                            .iter()
                            .position(|v| self.stars[v.index].id == id)
                    })
                    .unwrap_or(0),
            )
        });
    }
    fn refresh(&mut self, catalog: bool) {
        if self.offline {
            self.network_status = "OFFLINE · local calculations".into();
            return;
        }
        if self.busy {
            return;
        }
        self.busy = true;
        self.network_status = "POLLING · public sources".into();
        self.last_poll = Instant::now();
        if self
            .tx
            .send(Request::Refresh(self.location.clone(), catalog))
            .is_err()
        {
            self.busy = false;
            self.network_status = "Worker unavailable".into();
        }
    }
    fn updates(&mut self) {
        while let Ok(update) = self.rx.try_recv() {
            match update {
                Update::Location(l) => {
                    if self.location.is_none() {
                        self.location = Some(l);
                    }
                }
                Update::Catalog(s) => {
                    self.stars = s;
                    self.catalog_status =
                        format!("HYG v4.1 · synced {} UTC", Utc::now().format("%H:%M"));
                    self.last_catalog = Instant::now();
                }
                Update::Weather(w) => self.weather = w,
                Update::Status(source, error) => {
                    self.message = format!("{source}: {error}");
                    match source {
                        "weather" => {
                            self.weather = "Weather unavailable / stale · R retries".into()
                        }
                        "catalog" => {
                            self.catalog_status = "HYG bundled / last good · refresh failed".into()
                        }
                        _ => {}
                    }
                }
                Update::Done => {
                    self.busy = false;
                    self.network_status = "IDLE · weather polls every 10 min".into();
                }
            }
        }
    }
    fn select(&mut self, delta: isize) {
        if !self.visible.is_empty() {
            let n = self.visible.len() as isize;
            self.table.select(Some(
                (self.table.selected().unwrap_or(0) as isize + delta).rem_euclid(n) as usize,
            ));
        }
    }
    fn action(&mut self, c: char) {
        match c {
            'r' => self.refresh(true),
            's' => self.sort_alt = !self.sort_alt,
            'h' => self.high_only = !self.high_only,
            '+' => self.limit = (self.limit + 0.5).min(6.0),
            '-' => self.limit = (self.limit - 0.5).max(0.0),
            '/' => {
                self.editing = Some(false);
                self.input = self.query.clone();
            }
            'l' => {
                if self.busy {
                    self.message = "Wait for the current poll before changing location.".into();
                } else {
                    self.editing = Some(true);
                    self.input = self
                        .location
                        .as_ref()
                        .map(|l| format!("{}, {}", l.lat, l.lon))
                        .unwrap_or_default();
                }
            }
            '?' => self.help = !self.help,
            _ => {}
        }
        self.recompute();
    }
    fn submit(&mut self) {
        if self.editing == Some(true) {
            let parts: Vec<_> = self.input.split(',').map(str::trim).collect();
            let coords = if parts.len() == 2 {
                parts[0]
                    .parse::<f64>()
                    .ok()
                    .zip(parts[1].parse::<f64>().ok())
            } else {
                None
            };
            if let Some((lat, lon)) =
                coords.filter(|(a, b)| (-90.0..=90.0).contains(a) && (-180.0..=180.0).contains(b))
            {
                self.location = Some(Location {
                    lat,
                    lon,
                    label: "Manual coordinates".into(),
                });
                self.demo = false;
                self.weather = "Awaiting observing conditions".into();
                self.refresh(false);
            } else {
                self.message = "Use latitude, longitude; ranges −90…90 and −180…180.".into();
                return;
            }
        } else {
            self.query = self.input.clone();
        }
        self.editing = None;
        self.recompute();
    }
    fn mouse(&mut self, m: crossterm::event::MouseEvent) {
        if self.help {
            if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.help = false;
            }
            return;
        }
        match m.kind {
            MouseEventKind::ScrollDown => self.select(1),
            MouseEventKind::ScrollUp => self.select(-1),
            MouseEventKind::Down(MouseButton::Left) => {
                let p = ratatui::layout::Position::new(m.column, m.row);
                if let Some((_, c)) = self.buttons.iter().find(|(r, _)| r.contains(p)) {
                    self.action(*c);
                    return;
                }
                if self.table_area.contains(p) && m.row >= self.table_area.y + 3 {
                    let i = (m.row - self.table_area.y - 3) as usize + self.table.offset();
                    if i < self.visible.len() {
                        self.table.select(Some(i));
                    }
                } else if self.map_area.contains(p) {
                    let r = self.map_area;
                    let x = (m.column - r.x) as f64 / r.width.saturating_sub(1).max(1) as f64 * 2.3
                        - 1.15;
                    let y = 1.15
                        - (m.row - r.y) as f64 / r.height.saturating_sub(1).max(1) as f64 * 2.3;
                    let nearest = self
                        .visible
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let (sx, sy) = astro::project(v.alt, v.az);
                            (i, (sx - x).hypot(sy - y))
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1));
                    if let Some((i, d)) = nearest
                        && d < 0.18
                    {
                        self.table.select(Some(i));
                    }
                }
            }
            _ => {}
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some((lat, lon)) = args.lat.zip(args.lon)
        && (!(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon))
    {
        return Err("Coordinates out of range".into());
    }
    let mut app = App::new(&args);
    if args.snapshot {
        let backend = ratatui::backend::TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend)?;
        terminal.draw(|f| ui::draw(f, &mut app))?;
        for y in 0..40 {
            let line: String = (0..120)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect();
            println!("{line}");
        }
        return Ok(());
    }
    let mut terminal = ratatui::init();
    let result = (|| -> io::Result<()> {
        execute!(io::stdout(), EnableMouseCapture)?;
        loop {
            app.updates();
            app.recompute();
            if !app.busy && app.last_poll.elapsed() > Duration::from_secs(600) {
                app.refresh(app.last_catalog.elapsed() > Duration::from_secs(86400));
            }
            terminal.draw(|f| ui::draw(f, &mut app))?;
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => {
                        if k.code == KeyCode::Char('c')
                            && k.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            break;
                        }
                        if app.editing.is_some() {
                            match k.code {
                                KeyCode::Esc => app.editing = None,
                                KeyCode::Enter => app.submit(),
                                KeyCode::Backspace => {
                                    app.input.pop();
                                }
                                KeyCode::Char(c) => app.input.push(c),
                                _ => {}
                            }
                        } else {
                            match k.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Down | KeyCode::Char('j') => app.select(1),
                                KeyCode::Up | KeyCode::Char('k') => app.select(-1),
                                KeyCode::Esc => {
                                    app.help = false;
                                    app.query.clear();
                                    app.recompute();
                                }
                                KeyCode::Char(c) => app.action(c.to_ascii_lowercase()),
                                _ => {}
                            }
                        }
                    }
                    Event::Mouse(m) if app.editing.is_none() => app.mouse(m),
                    _ => {}
                }
            }
            app.tick += 1;
        }
        Ok(())
    })();
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result?;
    Ok(())
}
