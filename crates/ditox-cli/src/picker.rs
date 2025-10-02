use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};
// no process or encoder imports needed here
use crate::copy_helpers;
use crate::managed_daemon::ManagedControl;
use crate::theme;
use ditox_core::StoreImpl;
use std::path::PathBuf;

use crate::config;
use crate::preview;
use crate::{Query, Store};
// clipboard helpers are in copy_helpers module

fn fmt_abs_ns(ts_ns: i64) -> String {
    let dt = match time::OffsetDateTime::from_unix_timestamp_nanos(ts_ns as i128) {
        Ok(d) => d,
        Err(_) => return "<invalid>".into(),
    };
    static FMT: once_cell::sync::Lazy<Vec<time::format_description::FormatItem>> =
        once_cell::sync::Lazy::new(|| {
            time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:9]",
            )
            .unwrap()
        });
    dt.format(&FMT).unwrap_or_else(|_| dt.to_string())
}

fn trace(label: &str, t0: Instant) {
    if std::env::var_os("DITOX_TRACE_STARTUP").is_some() {
        eprintln!("[trace] {} +{:?}", label, t0.elapsed());
    }
}

#[allow(dead_code)]
struct DaemonClient {
    port: u16,
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl DaemonClient {
    fn connect_with_timeout(port: u16, timeout: std::time::Duration) -> anyhow::Result<Self> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let stream = TcpStream::connect_timeout(&addr, timeout)?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(150)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(150)));
        let writer = stream.try_clone()?;
        Ok(Self {
            port,
            reader: BufReader::new(stream),
            writer,
        })
    }

    fn request_page(
        &mut self,
        images: bool,
        favorites: bool,
        limit: Option<usize>,
        offset: Option<usize>,
        query: Option<String>,
        tag: Option<String>,
    ) -> anyhow::Result<Page<Item>> {
        let req = Request::List {
            images,
            favorites,
            limit,
            offset,
            query,
            tag,
        };
        let s = serde_json::to_string(&req)?;
        writeln!(&mut self.writer, "{}", s)?;
        self.writer.flush()?;
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        let resp: Response<Page<Item>> = serde_json::from_str(&line)?;
        if resp.ok {
            Ok(resp.data.unwrap_or(Page {
                items: Vec::new(),
                more: false,
                total: None,
            }))
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "daemon error".into()))
        }
    }
}

fn read_daemon_port_from_file() -> Option<u16> {
    let info_path = config::config_dir().join("clipd.json");
    let v = std::fs::read(&info_path).ok()?;
    let info: DaemonInfo = serde_json::from_slice(&v).ok()?;
    Some(info.port)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DaemonInfo {
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum Request {
    Health,
    List {
        images: bool,
        favorites: bool,
        limit: Option<usize>,
        offset: Option<usize>,
        query: Option<String>,
        tag: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Page<T> {
    items: Vec<T>,
    more: bool,
    total: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Item {
    Text {
        id: String,
        favorite: bool,
        created_at: i64,
        last_used_at: Option<i64>,
        text: String,
    },
    Image {
        id: String,
        favorite: bool,
        created_at: i64,
        last_used_at: Option<i64>,
        width: u32,
        height: u32,
        format: String,
        path: Option<String>,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    Managed,
    External,
    Off,
}

#[derive(Clone, Debug)]
pub struct CaptureStatus {
    pub mode: CaptureMode,
    pub managed: Option<ManagedControl>,
}

fn selected_id(
    items: &[Item],
    filtered: &[usize],
    page_index: usize,
    page_size: usize,
    selected: usize,
) -> Option<String> {
    let total = filtered.len();
    let start = page_index.saturating_mul(page_size);
    if start + selected < total {
        let idx = filtered[start + selected];
        items.get(idx).map(|it| match it {
            Item::Text { id, .. } | Item::Image { id, .. } => id.clone(),
        })
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_picker_default(
    store: &dyn Store,
    favorites: bool,
    images: bool,
    tag: Option<String>,
    no_daemon: bool,
    force_wl_copy: bool,
    remote_badge: bool,
    capture: Option<CaptureStatus>,
    // optional override for auto-refresh interval (ms)
    refresh_ms_override: Option<u64>,
) -> Result<()> {
    let mut es = RealEventSource;
    let _ = run_picker_with(
        store,
        favorites,
        images,
        tag,
        no_daemon,
        &mut es,
        true,
        force_wl_copy,
        remote_badge,
        capture,
        refresh_ms_override,
    )?;
    Ok(())
}

pub trait EventSource {
    fn poll(&mut self, timeout: Duration) -> Result<Option<Event>>;
}
pub struct RealEventSource;
impl EventSource for RealEventSource {
    fn poll(&mut self, timeout: Duration) -> Result<Option<Event>> {
        if crossterm::event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub fn run_picker_with(
    store: &dyn Store,
    favorites: bool,
    images: bool,
    tag: Option<String>,
    no_daemon: bool,
    es: &mut dyn EventSource,
    draw: bool,
    force_wl_copy: bool,
    remote_badge: bool,
    capture: Option<CaptureStatus>,
    refresh_ms_override: Option<u64>,
) -> Result<Option<String>> {
    let t0 = Instant::now();
    let use_daemon = !no_daemon;
    // Alt screen preference from env (set via CLI or settings)
    let alt_env = std::env::var("DITOX_TUI_ALT_SCREEN").ok();
    let want_alt_screen = alt_env.as_deref().map(|v| v != "0").unwrap_or(true);
    // Filters are mutable during the session
    let mut fav_filter = favorites;
    let mut images_mode = images;
    let mut tag_filter = tag.clone();
    // capture copy errors to report after exiting TUI
    let mut copy_error: Option<String> = None;
    // toast + delayed exit
    let mut toast: Option<(String, Instant)> = None;
    let mut exit_after: Option<Instant> = None;
    // defer printing selected id until after TUI exits
    let mut pending_print_id: Option<String> = None;
    // delete confirmation state
    let mut pending_delete_id: Option<String> = None;
    let mut pending_delete_until: Option<Instant> = None;
    // Defer loading dataset until after first frame for faster perceived start
    #[allow(unused_assignments)]
    let mut items: Vec<Item> = Vec::new();
    let mut has_more: bool;

    let mut used_alt_screen = false;
    let mut terminal = if draw {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if want_alt_screen {
            crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
            used_alt_screen = true;
        }
        let backend = CrosstermBackend::new(stdout);
        let term = Terminal::new(backend)?;
        trace("tui: terminal", t0);
        Some(term)
    } else {
        None
    };

    let mut query = String::new();
    let matcher = SkimMatcherV2::default();
    let tui_theme = theme::load_tui_theme();
    let glyphs = theme::load_glyphs();
    let layout = theme::load_layout();
    let caps = theme::detect_caps();
    #[allow(unused_assignments)]
    let mut filtered: Vec<usize> = Vec::new();
    let mut last_query = String::new();
    // Load settings and derive paging + tag auto-apply
    let settings = crate::config::load_settings();
    // Tag auto-apply support
    let tag_auto_ms: Option<u64> = settings
        .tui
        .as_ref()
        .and_then(|t| t.auto_apply_tag_ms)
        .filter(|&ms| ms > 0);
    let mut last_tag_typed: Option<Instant> = None;
    let mut last_applied_tag: Option<String> = tag.clone();
    let mut selected = 0usize; // selected row within current page
                               // pagination & UI state
    let page_size: usize = settings
        .tui
        .as_ref()
        .and_then(|t| t.page_size)
        .filter(|&n| n > 0)
        .unwrap_or(10);
    let absolute_times: bool = settings
        .tui
        .as_ref()
        .and_then(|t| t.absolute_times)
        .unwrap_or(true);
    let refresh_ms: u64 = refresh_ms_override
        .or_else(|| settings.tui.as_ref().and_then(|t| t.refresh_ms))
        .unwrap_or(1500);
    let typing_quiet_ms: u64 = 250;
    let mut page_index: usize = 0; // 0-based page
    let mut selected_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut show_help: bool = false;
    let mut last_fetch: Instant = Instant::now();
    let mut last_key_ts: Option<Instant> = None;
    // input mode: do not capture characters until '/' pressed
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Normal,
        Query,
    }
    let mut mode = Mode::Normal;
    // Dynamic page rows (items per page) based on viewport height; initialized from settings
    let mut page_rows: usize = page_size;
    // when external filter changes (f/i/tag), we need to recompute filtered
    let mut needs_refilter = true;

    // Draw immediate loading frame
    if let Some(ref mut term) = terminal {
        term.draw(|f| {
            let size = f.area();
            let mut block = Block::default().title("Loading…");
            if caps.unicode {
                block = block.borders(Borders::ALL);
            }
            f.render_widget(block, size);
        })?;
        trace("tui: first frame", t0);
    }

    // Load initial dataset now
    let mut daemon: Option<DaemonClient> = None;
    let cap = capture.clone().unwrap_or(CaptureStatus {
        mode: if use_daemon {
            CaptureMode::External
        } else {
            CaptureMode::Off
        },
        managed: None,
    });
    let mut last_known_total: Option<usize> = None;
    let mut pending_restore_id: Option<String> = None;
    // Track ids to highlight as "New" for a short time after appearance
    let mut last_ids: HashSet<String> = HashSet::new();
    let mut new_until: HashMap<String, Instant> = HashMap::new();
    let new_badge_ms: u64 = 2500;
    #[allow(unused_assignments)]
    let mut daemon_port: Option<u16> = None;
    if use_daemon {
        daemon_port = read_daemon_port_from_file();
        if let Some(port) = daemon_port {
            if let Ok(dc) = DaemonClient::connect_with_timeout(port, Duration::from_millis(400)) {
                trace("daemon: connected", t0);
                daemon = Some(dc);
            }
        }
        if let Some(dc) = daemon.as_mut() {
            match dc.request_page(
                images_mode,
                fav_filter,
                Some(page_rows),
                Some(0),
                None,
                tag_filter.clone(),
            ) {
                Ok(p) => {
                    items = p.items;
                    has_more = p.more;
                    last_known_total = p.total;
                }
                Err(_) => {
                    items = fetch_from_store(
                        store,
                        images_mode,
                        fav_filter,
                        Some(page_rows),
                        tag_filter.clone(),
                    )?;
                    has_more = false;
                    daemon = None;
                }
            }
        } else if let Ok(p) = fetch_page_from_daemon(
            images_mode,
            fav_filter,
            Some(page_rows),
            Some(0),
            None,
            tag_filter.clone(),
        ) {
            items = p.items;
            has_more = p.more;
            last_known_total = p.total;
        } else {
            items = fetch_from_store(
                store,
                images_mode,
                fav_filter,
                Some(page_rows),
                tag_filter.clone(),
            )?;
            has_more = false;
        }
    } else {
        items = fetch_from_store(
            store,
            images_mode,
            fav_filter,
            Some(page_rows),
            tag_filter.clone(),
        )?;
        has_more = false;
    }
    trace("data: initial page", t0);
    filtered = (0..items.len()).collect();

    loop {
        // recompute filtered when query changes or filter toggles
        if needs_refilter || (mode == Mode::Query && query != last_query) {
            needs_refilter = false;
            if use_daemon && !images_mode {
                // Try persistent daemon connection first; fallback to store
                if let Some(dc) = daemon.as_mut() {
                    match dc.request_page(
                        images_mode,
                        fav_filter,
                        Some(page_rows),
                        Some(0),
                        if mode == Mode::Query && !query.is_empty() {
                            Some(query.clone())
                        } else {
                            None
                        },
                        tag_filter.clone(),
                    ) {
                        Ok(p) => {
                            items = p.items;
                            has_more = p.more;
                        }
                        Err(_) => {
                            items = fetch_from_store(
                                store,
                                images_mode,
                                fav_filter,
                                None,
                                tag_filter.clone(),
                            )?;
                            has_more = false;
                            daemon = None;
                        }
                    }
                } else {
                    items =
                        fetch_from_store(store, images_mode, fav_filter, None, tag_filter.clone())?;
                    has_more = false;
                }
                filtered = (0..items.len()).collect();
            } else {
                filtered = if mode != Mode::Query || query.trim().is_empty() {
                    (0..items.len()).collect()
                } else {
                    let mut scored: Vec<(i64, usize)> = Vec::new();
                    for (idx, it) in items.iter().enumerate() {
                        let hay = match it {
                            Item::Text { text, .. } => text.as_str(),
                            Item::Image { format, .. } => format.as_str(),
                        };
                        if let Some(s) = matcher.fuzzy_match(hay, &query) {
                            scored.push((s, idx));
                        }
                    }
                    scored.sort_by_key(|(s, _)| -*s);
                    scored.into_iter().map(|(_, i)| i).collect()
                };
            }
            last_query = query.clone();
            // Update new-badge tracking
            let now = Instant::now();
            let mut curr_ids: HashSet<String> = HashSet::new();
            for it in items.iter() {
                let id = match it {
                    Item::Text { id, .. } | Item::Image { id, .. } => id,
                };
                if !last_ids.contains(id) {
                    new_until.insert(id.clone(), now + Duration::from_millis(new_badge_ms));
                }
                curr_ids.insert(id.clone());
            }
            last_ids = curr_ids;
            new_until.retain(|_, &mut until| until > now);
            // Track tag typing timestamp when in tag mode
            if mode == Mode::Query && query.starts_with('#') {
                last_tag_typed = Some(Instant::now());
            }
            // Reset selection when filters truly changed. When auto-refresh
            // requested a re-filter, try to restore previous selection by id.
            if let Some(tgt) = pending_restore_id.take() {
                let mut pos: Option<usize> = None;
                for (p, &i) in filtered.iter().enumerate() {
                    if let Some(it) = items.get(i) {
                        let id = match it {
                            Item::Text { id, .. } | Item::Image { id, .. } => id,
                        };
                        if *id == tgt {
                            pos = Some(p);
                            break;
                        }
                    }
                }
                if let Some(p) = pos {
                    page_index = p / page_size;
                    selected = p % page_size;
                } else {
                    page_index = 0;
                    selected = 0;
                }
            } else {
                page_index = 0;
                selected = 0;
            }
        }
        if selected >= filtered.len() {
            selected = filtered.len().saturating_sub(1);
        }

        if let Some(ref mut term) = terminal {
            term.draw(|f| {
                let size = f.area();
                let chunks = if mode == Mode::Query {
                    if layout.search_bar_bottom {
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(5),     // list
                                Constraint::Length(3),  // shortcuts/status
                                Constraint::Length(3),  // search bar bottom
                            ])
                            .split(size)
                    } else {
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(3),  // search bar top
                                Constraint::Min(5),     // list
                                Constraint::Length(3),  // shortcuts/status
                            ])
                            .split(size)
                    }
                } else {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(5),     // list only
                            Constraint::Length(3),  // shortcuts/status
                        ])
                        .split(size)
                };

                if mode == Mode::Query {
                    let q_title = "Search — type to filter";
                    let mut q_block = Block::default().title(q_title);
                    if caps.unicode {
                        if let Some(bt) = layout.border_search.or(tui_theme.border_type) {
                            q_block = q_block
                                .borders(Borders::ALL)
                                .border_type(bt)
                                .border_style(Style::default().fg(tui_theme.border_fg));
                        }
                    }
                    let q = Paragraph::new(query.as_str()).block(q_block);
                    let q_idx = if layout.search_bar_bottom { 2 } else { 0 };
                    f.render_widget(q, chunks[q_idx]);
                }

                // Compute dynamic rows-per-page from list area height and item height
                let list_area_idx = if mode == Mode::Query {
                    if layout.search_bar_bottom { 0 } else { 1 }
                } else { 0 };
                let list_area = chunks[list_area_idx];
                let item_rows = layout.list_line_height.clamp(1, 2) as usize;
                page_rows = std::cmp::max(1, (list_area.height as usize) / item_rows);

                // Pagination window
                let total = filtered.len();
                let total_pages = if total == 0 { 1 } else { (total - 1) / page_rows + 1 };
                if page_index >= total_pages { page_index = total_pages.saturating_sub(1); }
                let start = page_index.saturating_mul(page_rows);
                let end = (start + page_rows).min(total);
                let visible = &filtered[start..end];


                fn highlight_line<'a>(s: String, query: &str, th: &crate::theme::TuiTheme) -> Line<'a> {
                    if query.is_empty() || query.starts_with('#') { return Line::from(s); }
                    let lc = s.to_lowercase();
                    let qlc = query.to_lowercase();
                    if let Some(idx) = lc.find(&qlc) {
                        let end = idx + qlc.len();
                        let (a,b,c) = (&s[..idx], &s[idx..end], &s[end..]);
                        Line::from(vec![
                            Span::raw(a.to_string()),
                            Span::styled(b.to_string(), Style::default().fg(th.search_match_fg).bg(th.search_match_bg).add_modifier(Modifier::BOLD)),
                            Span::raw(c.to_string()),
                        ])
                    } else {
                        Line::from(s)
                    }
                }

                fn render_item_text(
                    id: &str, favorite: bool, text: &str, created_at: i64, last_used_at: &Option<i64>,
                    absolute_times: bool, selected_ids: &std::collections::HashSet<String>, glyphs: &crate::theme::Glyphs,
                    layout: &crate::theme::LayoutPack, th: &crate::theme::TuiTheme, query: &str,
                ) -> ListItem<'static> {
                    let fav = if favorite { glyphs.favorite_on.as_str() } else { glyphs.favorite_off.as_str() };
                    let sel_mark = if selected_ids.contains(id) { glyphs.selected.as_str() } else { glyphs.unselected.as_str() };
                    let created_str = if absolute_times { fmt_abs_ns(created_at) } else { rel_time_ns(created_at) };
                    let last_str = if let Some(lu) = last_used_at { if absolute_times { fmt_abs_ns(*lu) } else { rel_time_ns(*lu) } } else { "never".into() };
                    let line1 = if let Some(tpl) = &layout.item_template {
                        let mut s = tpl.clone();
                        let pairs = [
                            ("{favorite}", fav),
                            ("{selected}", sel_mark),
                            ("{kind}", glyphs.kind_text.as_str()),
                            ("{preview}", &preview(text)),
                        ];
                        for (k,v) in pairs { s = s.replace(k, v); }
                        s
                    } else {
                        format!("{}{} {} {}", fav, sel_mark, glyphs.kind_text, preview(text))
                    };
                    let meta_s = if let Some(tpl) = &layout.meta_template {
                        let mut s = tpl.clone();
                        let (recent_ns, recent_kind) = most_recent(created_at, *last_used_at);
                        let created_rel = rel_time_ns(created_at);
                        let last_rel = last_used_at.map(rel_time_ns).unwrap_or_else(|| "never".into());
                        let created_auto = fmt_auto_ns(created_at);
                        let last_used_auto = last_used_at.map(fmt_auto_ns).unwrap_or_else(|| "never".to_string());
                        let recent_str = fmt_auto_ns(recent_ns);
                        let recent_label = if recent_kind == "created" { "Created at" } else { "Last time used" };
                        let pairs = [
                            ("{created}", created_str.as_str()),
                            ("{last_used}", last_str.as_str()),
                            ("{created_rel}", created_rel.as_str()),
                            ("{last_used_rel}", last_rel.as_str()),
                            ("{created_auto}", created_auto.as_str()),
                            ("{last_used_auto}", last_used_auto.as_str()),
                            ("{recent}", recent_str.as_str()),
                            ("{recent_kind}", recent_kind),
                            ("{recent_label}", recent_label),
                            ("{created_label}", "Created at"),
                            ("{last_used_label}", "Last time used"),
                        ];
                        for (k,v) in pairs { s = s.replace(k, v); }
                        s
                    } else {
                        format!("Created at {} • Last used {}", created_str, last_str)
                    };
                    let line1 = highlight_line(line1, query, th);
                    if layout.list_line_height == 1 {
                        ListItem::new(vec![line1])
                    } else {
                        ListItem::new(vec![
                            line1,
                            Line::from(meta_s).style(Style::default().fg(th.muted_fg).add_modifier(Modifier::DIM)),
                        ])
                    }
                }

                fn render_item_image(
                    id: &str, favorite: bool, width: u32, height: u32, format: &str, name: &str,
                    created_at: i64, last_used_at: &Option<i64>, absolute_times: bool,
                    selected_ids: &std::collections::HashSet<String>, glyphs: &crate::theme::Glyphs,
                    layout: &crate::theme::LayoutPack, th: &crate::theme::TuiTheme, query: &str,
                ) -> ListItem<'static> {
                    let fav = if favorite { glyphs.favorite_on.as_str() } else { glyphs.favorite_off.as_str() };
                    let sel_mark = if selected_ids.contains(id) { glyphs.selected.as_str() } else { glyphs.unselected.as_str() };
                    let created_str = if absolute_times { fmt_abs_ns(created_at) } else { rel_time_ns(created_at) };
                    let last_str = if let Some(lu) = last_used_at { if absolute_times { fmt_abs_ns(*lu) } else { rel_time_ns(*lu) } } else { "never".into() };
                    let line1 = if let Some(tpl) = &layout.item_template {
                        let mut s = tpl.clone();
                        let dims = format!("{}x{}", width, height);
                        let pairs = [
                            ("{favorite}", fav),
                            ("{selected}", sel_mark),
                            ("{kind}", glyphs.kind_image.as_str()),
                            ("{name}", name),
                            ("{format}", format),
                            ("{dims}", dims.as_str()),
                        ];
                        for (k,v) in pairs { s = s.replace(k, v); }
                        s
                    } else if name.is_empty() {
                        format!("{}{} {} {}x{} {}", fav, sel_mark, glyphs.kind_image, width, height, format)
                    } else {
                        format!("{}{} {} {}x{} {} {}", fav, sel_mark, glyphs.kind_image, width, height, format, name)
                    };
                    let meta_s = if let Some(tpl) = &layout.meta_template {
                        let mut s = tpl.clone();
                        let (recent_ns, recent_kind) = most_recent(created_at, *last_used_at);
                        let created_rel = rel_time_ns(created_at);
                        let last_rel = last_used_at.map(rel_time_ns).unwrap_or_else(|| "never".into());
                        let created_auto = fmt_auto_ns(created_at);
                        let last_used_auto = last_used_at.map(fmt_auto_ns).unwrap_or_else(|| "never".to_string());
                        let recent_str = fmt_auto_ns(recent_ns);
                        let recent_label = if recent_kind == "created" { "Created at" } else { "Last time used" };
                        let pairs = [
                            ("{created}", created_str.as_str()),
                            ("{last_used}", last_str.as_str()),
                            ("{created_rel}", created_rel.as_str()),
                            ("{last_used_rel}", last_rel.as_str()),
                            ("{created_auto}", created_auto.as_str()),
                            ("{last_used_auto}", last_used_auto.as_str()),
                            ("{recent}", recent_str.as_str()),
                            ("{recent_kind}", recent_kind),
                            ("{recent_label}", recent_label),
                            ("{created_label}", "Created at"),
                            ("{last_used_label}", "Last time used"),
                        ];
                        for (k,v) in pairs { s = s.replace(k, v); }
                        s
                    } else {
                        format!("Created at {} • Last used {}", created_str, last_str)
                    };
                    let line1 = highlight_line(line1, query, th);
                    if layout.list_line_height == 1 {
                        ListItem::new(vec![line1])
                    } else {
                        ListItem::new(vec![
                            line1,
                            Line::from(meta_s).style(Style::default().fg(th.muted_fg).add_modifier(Modifier::DIM)),
                        ])
                    }
                }

 5e66a45 (feat(cli): preserve selection on manual filter changes and add 'New' badge\n\n- Keep cursor position across favorites/images/tag refreshes\n- Show a subtle 'New' badge for ~2.5s on recently added items\n- Debounce selection restore and prune badges automatically\n)
                let list_items: Vec<ListItem> = visible
                    .iter()
                    .filter_map(|&i| items.get(i))
                    .map(|it| match it {
                        Item::Text {
                            id, favorite, text, created_at, last_used_at, ..
                        } => {

                            render_item_text(id, *favorite, text, *created_at, last_used_at, absolute_times, &selected_ids, &glyphs, &layout, &tui_theme, if mode == Mode::Query { &query } else { "" })
 5e66a45 (feat(cli): preserve selection on manual filter changes and add 'New' badge\n\n- Keep cursor position across favorites/images/tag refreshes\n- Show a subtle 'New' badge for ~2.5s on recently added items\n- Debounce selection restore and prune badges automatically\n)
                        }
                        Item::Image {
                            id,
                            favorite,
                            width,
                            height,
                            format,
                            path,
                            created_at,
                            last_used_at,
                            ..
                        } => {
                            let name = path
                                .as_deref()
                                .and_then(|p| {
                                    std::path::Path::new(p).file_name().and_then(|n| n.to_str())
                                })
                                .unwrap_or("");

                            render_item_image(id, *favorite, *width, *height, format, name, *created_at, last_used_at, absolute_times, &selected_ids, &glyphs, &layout, &tui_theme, if mode == Mode::Query { &query } else { "" })
 5e66a45 (feat(cli): preserve selection on manual filter changes and add 'New' badge\n\n- Keep cursor position across favorites/images/tag refreshes\n- Show a subtle 'New' badge for ~2.5s on recently added items\n- Debounce selection restore and prune badges automatically\n)
                        }
                    })
                    .collect();
                let thm = &tui_theme;
                // Compute title tokens
                let mode_str = if images_mode { "Images" } else { "Text" };
                let loaded = items.len();
                let total_known = last_known_total.or(if use_daemon { None } else { Some(total) });
                let total_to_show = total_known.unwrap_or(loaded);
                let total_pages_known = total_known.map(|tt| if tt == 0 { 1 } else { (tt - 1) / page_rows + 1 });
                let page_count = total_pages_known.unwrap_or(total_pages);
                let page_count_str = page_count.to_string();
                let favorites_str = if fav_filter { " — Favorites" } else { "" };
                let tag_str = tag_filter.as_deref().filter(|s| !s.is_empty()).map(|t| format!(" — Tag: {}", t)).unwrap_or_default();
                let remote_str = if remote_badge { " — Remote" } else { "" };
                let title_text = if let Some(tpl) = &layout.list_title_template {
                    tpl.replace("{mode}", mode_str)
                        .replace("{favorites}", favorites_str)
                        .replace("{tag}", &tag_str)
                        .replace("{total}", &total_to_show.to_string())
                        .replace("{page}", &(page_index + 1).to_string())
                        .replace("{page_count}", &page_count_str)
                        .replace("{page_size}", &page_rows.to_string())
                        .replace("{remote}", remote_str)
                } else {
                    let mut t = String::from(mode_str);
                    if fav_filter { t.push_str(" — Favorites"); }
                    if !tag_str.is_empty() { t.push_str(&tag_str); }
                    let count_label = if fav_filter { format!(" — Total favorites {}", total_to_show) } else { format!(" — Total entries {}", total_to_show) };
                    t.push_str(&count_label);
                    if remote_badge { t.push_str(" — Remote"); }
                    t
                };
                // Styled title with optional remote badge (remote already in text if template used it)
                let mut title_spans: Vec<Span> = vec![Span::styled(title_text.clone(), Style::default().fg(tui_theme.title_fg))];
                if remote_badge {
                    if let Some(tpl) = &layout.list_title_template {
                        if !tpl.contains("{remote}") {
                            title_spans.push(Span::styled(" — Remote", Style::default().fg(tui_theme.badge_fg).bg(tui_theme.badge_bg)));
                        }
                    }
                }
                let mut list_block = Block::default().title(Line::from(title_spans));
                if caps.unicode {
                    if let Some(bt) = layout.border_list.or(tui_theme.border_type) {
                        list_block = list_block
                            .borders(Borders::ALL)
                            .border_type(bt)
                            .border_style(Style::default().fg(thm.border_fg));
                    }
                }
                let list = List::new(list_items)
                    .block(list_block)
                    .highlight_style(Style::default().fg(thm.highlight_fg).bg(thm.highlight_bg).add_modifier(Modifier::REVERSED));
                f.render_stateful_widget(
                    list,
                    chunks[list_area_idx],
                    &mut ratatui::widgets::ListState::default().with_selected(
                        if visible.is_empty() {
                            None
                        } else {
                            Some(selected.min(visible.len() - 1))
                        },
                    ),
                );
                // Optional compact pager at bottom-right of the list area (e.g., "1/14" or "11-20/245")
                if layout.show_list_pager.unwrap_or(true) {
                    let total_known2 = last_known_total.or(if use_daemon { None } else { Some(total) });
                    let total_to_show2 = total_known2.unwrap_or(total);
                    let first = if total == 0 { 0 } else { start + 1 };
                    let last = end;
                    let pager_tpl = layout.pager_template.as_deref().unwrap_or("{page}/{page_count}");
                    let pager_text = pager_tpl
                        .replace("{page}", &(page_index + 1).to_string())
                        .replace("{page_count}", &page_count_str)
                        .replace("{first}", &first.to_string())
                        .replace("{last}", &last.to_string())
                        .replace("{total}", &total_to_show2.to_string());
                    let la = chunks[list_area_idx];
                    let pager_rect = ratatui::layout::Rect { x: la.x, y: la.y + la.height.saturating_sub(1), width: la.width, height: 1 };
                    let pager = Paragraph::new(pager_text).alignment(Alignment::Left).style(Style::default().fg(tui_theme.muted_fg));
                    f.render_widget(pager, pager_rect);
                }
                // Footer — simple hint (optional via layout)
                let thm2 = &tui_theme;
                let thm2 = &tui_theme;
                let mut footer_block = Block::default().title(Line::styled("Shortcuts", Style::default().fg(tui_theme.title_fg)));
                if caps.unicode {
                    if let Some(bt) = layout.border_footer.or(tui_theme.border_type) {
                        footer_block = footer_block
                            .borders(Borders::ALL)
                            .border_type(bt)
                            .border_style(Style::default().fg(thm.border_fg));
                    }
                }
                let footer_area_idx = if mode == Mode::Query { if layout.search_bar_bottom { 1 } else { 2 } } else { 1 };
                let more_hint = if has_more { " | More available…" } else { "" };
                let selected_count = selected_ids.len().to_string();
                let toast_text = if let Some((msg, until)) = &toast {
                    if Instant::now() <= *until { format!("  — {}", msg) } else { String::new() }
                } else { String::new() };
                // Capture status summary
                let cap_str = match cap.mode {
                    CaptureMode::Managed => {
                        let paused = cap.managed.as_ref().map(|m| m.is_paused()).unwrap_or(false);
                        let img_on = cap.managed.as_ref().map(|m| m.images_on()).unwrap_or(true);
                        let ms = cap.managed.as_ref().map(|m| m.sample().as_millis()).unwrap_or(0);
                        format!("managed ({}ms, images:{}, {})", ms, if img_on {"on"} else {"off"}, if paused {"paused"} else {"active"})
                    }
                    CaptureMode::External => "external".into(),
                    CaptureMode::Off => "off".into(),
                };
                let simple = if let Some(tpl) = &layout.footer_template {
                    tpl.replace("{enter_label}", &glyphs.enter_label)
                        .replace("{selected_count}", &selected_count)
                        .replace("{more_hint}", more_hint)
                        .replace("{toast}", &toast_text)
                        .replace("{page}", &(page_index + 1).to_string())
                        .replace("{page_count}", &page_count_str)
                } else {
                    let mut s = format!("{} copy | x delete | p fav/unfav | Tab favorites | D images (managed) | Ctrl+P pause (managed) | ? more", glyphs.enter_label);
                    if !selected_ids.is_empty() { s.push_str(&format!(" | {} selected", selected_ids.len())); }
                    if has_more { s.push_str(" | More available…"); }
                    if !toast_text.is_empty() { s.push_str(&toast_text); }
                    s
                };
                let simple = format!("{} | capture: {}", simple, cap_str);
                if layout.help_footer {
                    let footer = Paragraph::new(simple)
                        .block(footer_block)
                        .style(Style::default().fg(tui_theme.status_fg).bg(tui_theme.status_bg))
                        .wrap(Wrap { trim: true });
                    f.render_widget(footer, chunks[footer_area_idx]);
                }
9d55e (feat(cli): embedded managed capture, TUI status, hotkeys, and search auto-refresh\n\n- Add managed daemon inside picker with lockfile + pause/images controls\n- Show capture status in footer; bind Ctrl+P (pause) and D (images)\n- Debounced auto-refresh incl. during search; preserve selection on refresh\n- Add --refresh-ms flag and [tui].refresh_ms config\n- Optional tray (feature ) with Pause/Images/QUIT; flake app \n- Doctor prints capture: managed/external/off\n\nchore(flake): add packages.ditox-tray and apps.tray; devShell includes GTK3/AppIndicator\n)

                // Expanded help as centered modal overlay
                if show_help {
                    let overlay = centered_rect(70, 70, size);
                    // Clear underlying area so content doesn't bleed through
                    f.render_widget(Clear, overlay);
                    let mut block = Block::default()
                        .title(Line::styled("Shortcuts — Help (? to close)", Style::default().fg(tui_theme.title_fg)))
                        .style(Style::default().bg(tui_theme.status_bg));
                    if caps.unicode {
                        if let Some(bt) = layout.border_help.or(tui_theme.border_type) {
                            block = block
                                .borders(Borders::ALL)
                                .border_type(bt)
                                .border_style(Style::default().fg(thm.border_fg));
                        }
                    }
                    f.render_widget(block.clone(), overlay);
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(34),
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                        ])
                        .split(inner(overlay));
                    let col1 = Paragraph::new(
                        "↑/k up
↓/j down
→/l/PgDn next page
←/h/PgUp prev page
Home/g go to start
End/G go to end",
                    )
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(thm2.help_fg).bg(tui_theme.status_bg));
                    let col2 = Paragraph::new(
                        "/ filter
s select
S clear selected
Tab favorites toggle
i images view toggle
D images capture toggle (managed)
Ctrl+P pause/resume capture (managed)
t apply #tag
r refresh",
                    )
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(thm2.help_fg).bg(tui_theme.status_bg));
                    let mut col3_text = if caps.unicode {
                        String::from("⏎ copy | x delete | p fav/unfav
q quit
? close help")
                    } else {
                        String::from("Enter copy | x delete | p fav/unfav
q quit
? close help")
                    };
                    if has_more { col3_text.push_str("
More available…"); }
                    let col3 = Paragraph::new(col3_text)
                        .wrap(Wrap { trim: true })
                        .style(Style::default().fg(thm2.help_fg).bg(tui_theme.status_bg));
                    f.render_widget(col1, cols[0]);
                    f.render_widget(col2, cols[1]);
                    f.render_widget(col3, cols[2]);
9d55e (feat(cli): embedded managed capture, TUI status, hotkeys, and search auto-refresh\n\n- Add managed daemon inside picker with lockfile + pause/images controls\n- Show capture status in footer; bind Ctrl+P (pause) and D (images)\n- Debounced auto-refresh incl. during search; preserve selection on refresh\n- Add --refresh-ms flag and [tui].refresh_ms config\n- Optional tray (feature ) with Pause/Images/QUIT; flake app \n- Doctor prints capture: managed/external/off\n\nchore(flake): add packages.ditox-tray and apps.tray; devShell includes GTK3/AppIndicator\n)
                        .wrap(Wrap { trim: true })
                        .style(Style::default().fg(thm2.help_fg).bg(tui_theme.status_bg));
                        let col2 = Paragraph::new(
                            "/ filter\ns select\nS clear selected\nTab favorites toggle\ni images toggle\nt apply #tag\nr refresh",
                        )
                        .wrap(Wrap { trim: true })
                        .style(Style::default().fg(thm2.help_fg).bg(tui_theme.status_bg));
                        let mut col3_text = if caps.unicode {
                            String::from("⏎ copy | x delete | p fav/unfav\nq quit\n? close help")
                        } else {
                            String::from("Enter copy | x delete | p fav/unfav\nq quit\n? close help")
                        };
                        if has_more { col3_text.push_str("\nMore available…"); }
                        let col3 = Paragraph::new(col3_text)
                            .wrap(Wrap { trim: true })
                            .style(Style::default().fg(thm2.help_fg).bg(tui_theme.status_bg));
                        f.render_widget(col1, cols[0]);
                        f.render_widget(col2, cols[1]);
                        f.render_widget(col3, cols[2]);
                    }
                }
            })?;
        }

        if let Some(ev) = es.poll(Duration::from_millis(100))? {
            match ev {
                Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Char('q') => {
                        break;
                    }
                    KeyCode::Char('c')
                        if k.modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break
                    }
                    KeyCode::Char('?') if mode == Mode::Normal => {
                        show_help = !show_help;
                    }
                    KeyCode::Char('/') => {
                        // Toggle search mode. Do not clear the query; when leaving
                        // search, results revert to unfiltered. When entering, apply
                        // whatever query text is present.
                        mode = match mode {
                            Mode::Normal => Mode::Query,
                            Mode::Query => Mode::Normal,
                        };
                        last_query.clear();
                        needs_refilter = true;
                        last_key_ts = Some(Instant::now());
                    }
                    KeyCode::Tab => {
                        // preserve selection across reload
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        fav_filter = !fav_filter;
                        last_query.clear();
                        needs_refilter = true;
                        selected = 0;
                        page_index = 0;
                        pending_delete_id = None;
                        pending_delete_until = None;
                        if use_daemon {
                            if let Some(dc) = daemon.as_mut() {
                                match dc.request_page(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(0),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    Ok(p) => {
                                        items = p.items;
                                        has_more = p.more;
                                    }
                                    Err(_) => {
                                        items = fetch_from_store(
                                            store,
                                            images_mode,
                                            fav_filter,
                                            None,
                                            tag_filter.clone(),
                                        )?;
                                        has_more = false;
                                        daemon = None;
                                    }
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                                has_more = false;
                            }
                        } else {
                            items = fetch_from_store(
                                store,
                                images_mode,
                                fav_filter,
                                None,
                                tag_filter.clone(),
                            )?;
                        }
                        filtered = (0..items.len()).collect();
                    }

                    KeyCode::Char('f') if mode == Mode::Normal => {
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        fav_filter = !fav_filter;
                        last_query.clear();
                        needs_refilter = true;
                        selected = 0;
                        page_index = 0;
                        pending_delete_id = None;
                        pending_delete_until = None;
                        if use_daemon {
                            if let Some(dc) = daemon.as_mut() {
                                match dc.request_page(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(0),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    Ok(p) => {
                                        items = p.items;
                                        has_more = p.more;
                                    }
                                    Err(_) => {
                                        items = fetch_from_store(
                                            store,
                                            images_mode,
                                            fav_filter,
                                            None,
                                            tag_filter.clone(),
                                        )?;
                                        has_more = false;
                                        daemon = None;
                                    }
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                                has_more = false;
                            }
                        } else {
                            items = fetch_from_store(
                                store,
                                images_mode,
                                fav_filter,
                                None,
                                tag_filter.clone(),
                            )?;
                        }
                        filtered = (0..items.len()).collect();
                    }
                    KeyCode::Char('i') if mode == Mode::Normal => {
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        images_mode = !images_mode;
                        selected = 0;
                        page_index = 0;
                        last_query.clear();
                        needs_refilter = true;
                        pending_delete_id = None;
                        pending_delete_until = None;
                        let load_res: anyhow::Result<()> = {
                            if use_daemon {
                                if let Some(dc) = daemon.as_mut() {
                                    let p = dc.request_page(
                                        images_mode,
                                        fav_filter,
                                        Some(page_rows),
                                        Some(0),
                                        None,
                                        tag_filter.clone(),
                                    )?;
                                    items = p.items;
                                    has_more = p.more;
                                    Ok(())
                                } else {
                                    items = fetch_from_store(
                                        store,
                                        images_mode,
                                        fav_filter,
                                        None,
                                        tag_filter.clone(),
                                    )?;
                                    has_more = false;
                                    Ok(())
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                                has_more = false;
                                Ok(())
                            }
                        };
                        match load_res {
                            Ok(()) => {
                                filtered = (0..items.len()).collect();
                            }
                            Err(e) => {
                                let msg = format!("{}", e);
                                images_mode = false; // revert toggle to keep session stable
                                needs_refilter = true;
                                if msg.contains("no such column: c.image_path")
                                    || msg.contains("no such column: image_path")
                                {
                                    toast = Some((
                                        "Images schema missing. Press 'm' to run migrations."
                                            .into(),
                                        Instant::now() + Duration::from_millis(3000),
                                    ));
                                } else {
                                    toast = Some((
                                        format!("Images view failed: {}", truncate_msg(&msg, 80)),
                                        Instant::now() + Duration::from_millis(3000),
                                    ));
                                }
                            }
                        }
                    }
                    // Pause/resume capture (managed only) — Ctrl+P
                    KeyCode::Char('p')
                        if k.modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                            && mode == Mode::Normal =>
                    {
                        if let Some(mc) = cap.managed.as_ref() {
                            let now_paused = mc.toggle_pause();
                            toast = Some((
                                if now_paused {
                                    "Capture paused".into()
                                } else {
                                    "Capture resumed".into()
                                },
                                Instant::now() + Duration::from_millis(1200),
                            ));
                        } else {
                            toast = Some((
                                "Pause requires managed capture".into(),
                                Instant::now() + Duration::from_millis(1500),
                            ));
                        }
                    }
                    // Toggle managed images on/off (Shift+D)
                    KeyCode::Char('D') if mode == Mode::Normal => {
                        if let Some(mc) = cap.managed.as_ref() {
                            let on = mc.toggle_images();
                            toast = Some((
                                if on {
                                    "Images capture: on".into()
                                } else {
                                    "Images capture: off".into()
                                },
                                Instant::now() + Duration::from_millis(1200),
                            ));
                        } else {
                            toast = Some((
                                "Images capture toggle requires managed mode".into(),
                                Instant::now() + Duration::from_millis(1500),
                            ));
                        }
                    }
                    // Toggle favorite on current item or selection
                    KeyCode::Char('p') | KeyCode::Char('P') if mode == Mode::Normal => {
                        let mut ids: Vec<String> = Vec::new();
                        if !selected_ids.is_empty() {
                            ids.extend(selected_ids.iter().cloned());
                        } else if let Some(idx) = filtered
                            .get(page_index.saturating_mul(page_rows) + selected)
                            .cloned()
                        {
                            if let Some(it) = items.get(idx) {
                                let id = match it {
                                    Item::Text { id, .. } | Item::Image { id, .. } => id.clone(),
                                };
                                ids.push(id);
                            }
                        }
                        if !ids.is_empty() {
                            // Determine toggle based on first item's current fav state
                            let mut make_fav = None;
                            for id in &ids {
                                if let Some(it) = items.iter().find(|it| match it {
                                    Item::Text { id: i, .. } | Item::Image { id: i, .. } => i == id,
                                }) {
                                    let is_fav = match it {
                                        Item::Text { favorite, .. }
                                        | Item::Image { favorite, .. } => *favorite,
                                    };
                                    if make_fav.is_none() {
                                        make_fav = Some(!is_fav);
                                    }
                                    let _ = store.favorite(id, make_fav.unwrap());
                                }
                            }
                            toast = Some((
                                if make_fav.unwrap_or(true) {
                                    "Favorited".into()
                                } else {
                                    "Unfavorited".into()
                                },
                                Instant::now() + Duration::from_millis(900),
                            ));
                            // Refresh items so filters apply correctly
                            if use_daemon {
                                match fetch_page_from_daemon(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(0),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    Ok(p) => {
                                        items = p.items;
                                        has_more = p.more;
                                    }
                                    Err(_) => {
                                        items = fetch_from_store(
                                            store,
                                            images_mode,
                                            fav_filter,
                                            None,
                                            tag_filter.clone(),
                                        )?;
                                        has_more = false;
                                    }
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                            }
                            filtered = (0..items.len()).collect();
                            if selected >= page_rows {
                                selected = page_rows.saturating_sub(1);
                            }
                        }
                    }
                    // Delete current item with quick confirm; or bulk delete selected
                    KeyCode::Delete => {
                        // Immediate delete without confirm
                        let ids: Vec<String> = if !selected_ids.is_empty() {
                            selected_ids.iter().cloned().collect()
                        } else {
                            let mut v = Vec::new();
                            let total = filtered.len();
                            let start = page_index * page_size;
                            if start + selected < total {
                                if let Some(it) = items.get(filtered[start + selected]) {
                                    let id = match it {
                                        Item::Text { id, .. } | Item::Image { id, .. } => {
                                            id.clone()
                                        }
                                    };
                                    v.push(id);
                                }
                            }
                            v
                        };
                        if !ids.is_empty() {
                            let mut ok = 0usize;
                            for id in ids {
                                if store.delete(&id).is_ok() {
                                    ok += 1;
                                }
                            }
                            selected_ids.clear();
                            toast = Some((
                                format!("Deleted {}", ok),
                                Instant::now() + Duration::from_millis(900),
                            ));
                            if use_daemon {
                                if let Ok(p) = fetch_page_from_daemon(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(0),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    items = p.items;
                                    has_more = p.more;
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                            }
                            filtered = (0..items.len()).collect();
                            if selected >= page_rows {
                                selected = page_rows.saturating_sub(1);
                            }
                        }
                    }
                    KeyCode::Char('x') | KeyCode::Char('X') if mode == Mode::Normal => {
                        let now = Instant::now();
                        // Determine targets: selected set or current item
                        let mut ids: Vec<String> = if !selected_ids.is_empty() {
                            selected_ids.iter().cloned().collect()
                        } else {
                            let mut v = Vec::new();
                            let total = filtered.len();
                            let start = page_index * page_size;
                            if start + selected < total {
                                if let Some(it) = items.get(filtered[start + selected]) {
                                    let id = match it {
                                        Item::Text { id, .. } | Item::Image { id, .. } => {
                                            id.clone()
                                        }
                                    };
                                    v.push(id);
                                }
                            }
                            v
                        };
                        if ids.len() == 1 {
                            let id = ids.pop().unwrap();
                            let confirm_ok = pending_delete_id.as_deref() == Some(id.as_str())
                                && pending_delete_until.map(|t| now <= t).unwrap_or(false);
                            if confirm_ok {
                                if store.delete(&id).is_ok() {
                                    toast = Some((
                                        "Deleted".into(),
                                        Instant::now() + Duration::from_millis(900),
                                    ));
                                    // Refresh after delete
                                    if use_daemon {
                                        if let Ok(p) = fetch_page_from_daemon(
                                            images_mode,
                                            fav_filter,
                                            Some(page_rows),
                                            Some(0),
                                            None,
                                            tag_filter.clone(),
                                        ) {
                                            items = p.items;
                                            has_more = p.more;
                                        }
                                    } else {
                                        items = fetch_from_store(
                                            store,
                                            images_mode,
                                            fav_filter,
                                            None,
                                            tag_filter.clone(),
                                        )?;
                                    }
                                    filtered = (0..items.len()).collect();
                                    if selected >= page_rows {
                                        selected = page_rows.saturating_sub(1);
                                    }
                                }
                                pending_delete_id = None;
                                pending_delete_until = None;
                            } else {
                                pending_delete_id = Some(id);
                                pending_delete_until = Some(now + Duration::from_millis(1500));
                                toast = Some((
                                    "Press x again to delete".into(),
                                    now + Duration::from_millis(1500),
                                ));
                            }
                        } else if !ids.is_empty() {
                            // Bulk delete without confirm (explicit selection acts as confirm)
                            let mut ok = 0usize;
                            for id in ids {
                                if store.delete(&id).is_ok() {
                                    ok += 1;
                                }
                            }
                            selected_ids.clear();
                            toast = Some((
                                format!("Deleted {}", ok),
                                Instant::now() + Duration::from_millis(1200),
                            ));
                            if use_daemon {
                                if let Some(dc) = daemon.as_mut() {
                                    if let Ok(p) = dc.request_page(
                                        images_mode,
                                        fav_filter,
                                        Some(page_rows),
                                        Some(0),
                                        None,
                                        tag_filter.clone(),
                                    ) {
                                        items = p.items;
                                        has_more = p.more;
                                    }
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                            }
                            filtered = (0..items.len()).collect();
                            if selected >= page_rows {
                                selected = page_rows.saturating_sub(1);
                            }
                        }
                    }
                    KeyCode::Char('t') if mode == Mode::Normal => {
                        // Enter tag filter mode by priming the query with '#'
                        mode = Mode::Query;
                        if !query.starts_with('#') {
                            query.clear();
                            query.push('#');
                        }
                        last_query.clear();
                    }
                    KeyCode::Char('r') if mode == Mode::Normal => {
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        last_query.clear();
                        needs_refilter = true;
                        page_index = 0;
                        if use_daemon {
                            if let Some(dc) = daemon.as_mut() {
                                match dc.request_page(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(0),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    Ok(p) => {
                                        items = p.items;
                                        has_more = p.more;
                                    }
                                    Err(_) => {
                                        items = fetch_from_store(
                                            store,
                                            images_mode,
                                            fav_filter,
                                            None,
                                            tag_filter.clone(),
                                        )?;
                                        has_more = false;
                                        daemon = None;
                                    }
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                                has_more = false;
                            }
                        } else {
                            items = fetch_from_store(
                                store,
                                images_mode,
                                fav_filter,
                                None,
                                tag_filter.clone(),
                            )?;
                            has_more = false;
                        }
                        filtered = (0..items.len()).collect();
                    }
                    // Run migrations on DB (best-effort) and reload list
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        match migrate_current_db() {
                            Ok(()) => {
                                toast = Some((
                                    "Migrations applied".into(),
                                    Instant::now() + Duration::from_millis(1200),
                                ));
                                // reload items with current filters
                                if use_daemon {
                                    if let Some(dc) = daemon.as_mut() {
                                        if let Ok(p) = dc.request_page(
                                            images_mode,
                                            fav_filter,
                                            Some(page_rows),
                                            Some(0),
                                            None,
                                            tag_filter.clone(),
                                        ) {
                                            items = p.items;
                                            has_more = p.more;
                                        }
                                    }
                                } else if let Ok(v) = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    items = v;
                                    has_more = false;
                                }
                                filtered = (0..items.len()).collect();
                            }
                            Err(e) => {
                                toast = Some((
                                    format!(
                                        "Migration failed: {}",
                                        truncate_msg(&format!("{}", e), 80)
                                    ),
                                    Instant::now() + Duration::from_millis(3000),
                                ));
                            }
                        }
                    }

                    KeyCode::Backspace => {
                        if mode == Mode::Query {
                            query.pop();
                            last_key_ts = Some(Instant::now());
                        }
                    }
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        } else if page_index > 0 {
                            page_index -= 1;
                            selected = page_rows.saturating_sub(1);
                        }
                    }
                    KeyCode::Char('k') if mode == Mode::Normal => {
                        if selected > 0 {
                            selected -= 1;
                        } else if page_index > 0 {
                            page_index -= 1;
                            selected = page_rows.saturating_sub(1);
                        }
                    }
                    KeyCode::Down => {
                        let total = filtered.len();
                        let start = page_index.saturating_mul(page_rows);
                        let end = (start + page_rows).min(total);
                        let page_len = end.saturating_sub(start);
                        if selected + 1 < page_len {
                            selected += 1;
                        } else if end < total {
                            page_index += 1;
                            selected = 0;
                        } else if use_daemon && has_more {
                            // Optionally prefetch more from daemon when at end
                            if let Some(dc) = daemon.as_mut() {
                                if let Ok(p) = dc.request_page(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(items.len()),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    has_more = p.more;
                                    items.extend(p.items);
                                    last_query.clear();
                                    filtered = (0..items.len()).collect();
                                }
                            }
                        }
                    }
                    KeyCode::Char('j') if mode == Mode::Normal => {
                        let total = filtered.len();
                        let start = page_index.saturating_mul(page_rows);
                        let end = (start + page_rows).min(total);
                        let page_len = end.saturating_sub(start);
                        if selected + 1 < page_len {
                            selected += 1;
                        } else if end < total {
                            page_index += 1;
                            selected = 0;
                        } else if use_daemon && has_more {
                            // Optionally prefetch more from daemon when at end
                            if let Some(dc) = daemon.as_mut() {
                                if let Ok(p) = dc.request_page(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(items.len()),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    has_more = p.more;
                                    items.extend(p.items);
                                    last_query.clear();
                                    filtered = (0..items.len()).collect();
                                }
                            }
                        }
                    }
                    KeyCode::Right | KeyCode::PageDown => {
                        let total = filtered.len();
                        let start = (page_index + 1).saturating_mul(page_rows);
                        if start >= total && use_daemon && has_more {
                            // Fetch enough pages to cover the next page window
                            if let Some(dc) = daemon.as_mut() {
                                while start >= items.len() && has_more {
                                    if let Ok(p) = dc.request_page(
                                        images_mode,
                                        fav_filter,
                                        Some(page_rows),
                                        Some(items.len()),
                                        None,
                                        tag_filter.clone(),
                                    ) {
                                        has_more = p.more;
                                        items.extend(p.items);
                                        last_query.clear();
                                        filtered = (0..items.len()).collect();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        let total = filtered.len();
                        let start2 = (page_index + 1).saturating_mul(page_rows);
                        if start2 < total {
                            page_index += 1;
                            selected = 0;
                        }
                    }
                    KeyCode::PageUp | KeyCode::Left => {
                        if page_index > 0 {
                            page_index -= 1;
                            selected = 0;
                        }
                    }
                    KeyCode::Char('h') if mode == Mode::Normal => {
                        if page_index > 0 {
                            page_index -= 1;
                            selected = 0;
                        }
                    }
                    KeyCode::Home => {
                        page_index = 0;
                        selected = 0;
                    }
                    KeyCode::Char('g') if mode == Mode::Normal => {
                        page_index = 0;
                        selected = 0;
                    }
                    KeyCode::End => {
                        let total = filtered.len();
                        if total > 0 {
                            page_index = (total - 1) / page_size;
                            let start = page_index * page_size;
                            selected = (total - start).saturating_sub(1);
                        }
                    }
                    KeyCode::Char('G') if mode == Mode::Normal => {
                        let total = filtered.len();
                        if total > 0 {
                            page_index = (total - 1) / page_size;
                            let start = page_index * page_size;
                            selected = (total - start).saturating_sub(1);
                        }
                    }
                    KeyCode::Char('s') if mode == Mode::Normal => {
                        // Toggle selection of current visible item
                        let total = filtered.len();
                        if total > 0 {
                            let start = page_index * page_size;
                            if start + selected < total {
                                if let Some(it) = items.get(filtered[start + selected]) {
                                    let id = match it {
                                        Item::Text { id, .. } | Item::Image { id, .. } => {
                                            id.clone()
                                        }
                                    };
                                    if selected_ids.contains(&id) {
                                        selected_ids.remove(&id);
                                    } else {
                                        selected_ids.insert(id);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('S') if mode == Mode::Normal => {
                        selected_ids.clear();
                    }
                    KeyCode::Enter => {
                        // (debug dump removed after fixing hjkl handling in Query mode)
                        if !query.is_empty() && query.starts_with('#') {
                            // Apply tag from #tag then clear query
                            tag_filter = if query.len() == 1 {
                                None
                            } else {
                                Some(query[1..].to_string())
                            };
                            last_query.clear();
                            if use_daemon {
                                match fetch_page_from_daemon(
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    Some(0),
                                    None,
                                    tag_filter.clone(),
                                ) {
                                    Ok(p) => {
                                        items = p.items;
                                        has_more = p.more;
                                    }
                                    Err(_) => {
                                        items = fetch_from_store(
                                            store,
                                            images_mode,
                                            fav_filter,
                                            None,
                                            tag_filter.clone(),
                                        )?;
                                        has_more = false;
                                    }
                                }
                            } else {
                                items = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    None,
                                    tag_filter.clone(),
                                )?;
                            }
                            query.clear();
                            continue;
                        }
                        let start = page_index.saturating_mul(page_rows);
                        if let Some(idx) = filtered.get(start + selected).cloned() {
                            // perform copy and exit
                            match &items[idx] {
                                Item::Text { id, text, .. } => {
                                    if let Err(e) = copy_helpers::copy_text(text, force_wl_copy) {
                                        copy_error = Some(format!("copy failed: {}", e));
                                    } else {
                                        // Make it instantaneous: skip long toasts and exit now
                                        toast = None;
                                        exit_after = Some(Instant::now());
                                    }
                                    let _ = store.touch_last_used(id);
                                    pending_print_id = Some(id.clone());
                                    if !draw {
                                        return Ok(Some(id.clone()));
                                    }
                                }
                                Item::Image { id, .. } => {
                                    if let Ok(Some(img)) = store.get_image_rgba(id) {
                                        if let Err(e) =
                                            copy_helpers::copy_image(&img, force_wl_copy)
                                        {
                                            copy_error = Some(format!("image copy failed: {}", e));
                                        } else {
                                            toast = None;
                                            exit_after = Some(Instant::now());
                                        }
                                    }
                                    let _ = store.touch_last_used(id);
                                    pending_print_id = Some(id.clone());
                                    if !draw {
                                        return Ok(Some(id.clone()));
                                    }
                                }
                            }
                        }
                        // Do not break immediately; allow toast to render for a moment
                    }
                    KeyCode::Char(ch) => {
                        if mode == Mode::Query {
                            query.push(ch);
                            if query.starts_with('#') {
                                last_tag_typed = Some(Instant::now());
                            }
                            last_key_ts = Some(Instant::now());
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // Auto-apply tag after idle if enabled
        if mode == Mode::Query {
            if let Some(ms) = tag_auto_ms {
                if query.starts_with('#') && query.len() > 1 {
                    if let Some(ts) = last_tag_typed {
                        if ts.elapsed() >= Duration::from_millis(ms) {
                            let new_tag = query[1..].to_string();
                            if last_applied_tag.as_deref() != Some(&new_tag) {
                                tag_filter = Some(new_tag.clone());
                                last_applied_tag = Some(new_tag);
                                // reload items (preserve selection)
                                pending_restore_id =
                                    selected_id(&items, &filtered, page_index, page_size, selected);
                                if use_daemon {
                                    if let Some(dc) = daemon.as_mut() {
                                        if let Ok(p) = dc.request_page(
                                            images_mode,
                                            fav_filter,
                                            Some(page_rows),
                                            Some(0),
                                            None,
                                            tag_filter.clone(),
                                        ) {
                                            items = p.items;
                                            has_more = p.more;
                                            last_query.clear();
                                            filtered = (0..items.len()).collect();
                                        }
                                    }
                                } else if let Ok(v) = fetch_from_store(
                                    store,
                                    images_mode,
                                    fav_filter,
                                    Some(page_rows),
                                    tag_filter.clone(),
                                ) {
                                    items = v;
                                    has_more = false;
                                    last_query.clear();
                                    filtered = (0..items.len()).collect();
                                }
                            }
                        }
                    }
                }
            }
        }

        // Periodic auto-reload (debounced; default 1500ms). When in Query mode with a non-empty
        // query, we refresh the backing data and then re-run filtering (fuzzy) by
        // setting `needs_refilter = true`.
        if last_fetch.elapsed() > Duration::from_millis(refresh_ms) {
            if mode == Mode::Query {
                if let Some(ts) = last_key_ts {
                    if ts.elapsed() < Duration::from_millis(typing_quiet_ms) {
                        // Skip this cycle while user is actively typing
                        continue;
                    }
                }
            }
            if use_daemon {
                let mut refreshed = false;
                let target_len = (page_index + 1) * page_size;
                if let Some(dc) = daemon.as_mut() {
                    // Always refresh total from head page to keep counts up to date
                    if let Ok(p0) = dc.request_page(
                        images_mode,
                        fav_filter,
                        Some(page_rows),
                        Some(0),
                        if mode == Mode::Query && !query.is_empty() {
                            Some(query.clone())
                        } else {
                            None
                        },
                        tag_filter.clone(),
                    ) {
                        last_known_total = p0.total;
                        has_more = p0.more; // best-effort update
                    }
                    let mut fetched = items.clone();
                    let mut more = has_more;
                    while fetched.len() < target_len && more {
                        if let Ok(p) = dc.request_page(
                            images_mode,
                            fav_filter,
                            Some(page_rows),
                            Some(fetched.len()),
                            if mode == Mode::Query && !query.is_empty() {
                                Some(query.clone())
                            } else {
                                None
                            },
                            tag_filter.clone(),
                        ) {
                            more = p.more;
                            last_known_total = p.total;
                            fetched.extend(p.items);
                        } else {
                            break;
                        }
                    }
                    if fetched.len() >= items.len() {
                        // remember current selection id so we can restore it
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        items = fetched;
                        has_more = more;
                        // trigger re-filtering when in query mode
                        needs_refilter = true;
                        refreshed = true;
                    }
                }
                // Fallback: when no external daemon is connected, refresh directly from the store
                if !refreshed {
                    if let Ok(v) = fetch_from_store(
                        store,
                        images_mode,
                        fav_filter,
                        Some(page_size),
                        tag_filter.clone(),
                    ) {
                        // remember current selection id so we can restore it
                        pending_restore_id =
                            selected_id(&items, &filtered, page_index, page_size, selected);
                        items = v;
                        has_more = false;
                        // trigger re-filtering (applies fuzzy matching when in Query mode)
                        needs_refilter = true;
                        if let Ok(total) = store.count(Query {
                            contains: None,
                            favorites_only: fav_filter,
                            limit: None,
                            tag: tag_filter.clone(),
                            rank: false,
                        }) {
                            last_known_total = Some(total);
                        }
                    }
                }
            } else if let Ok(v) = fetch_from_store(
                store,
                images_mode,
                fav_filter,
                Some(page_rows),
                tag_filter.clone(),
            ) {
                items = v;
                has_more = false;
                // trigger re-filtering (applies fuzzy matching when in Query mode)
                needs_refilter = true;
                // Update total via store count
                if let Ok(total) = store.count(Query {
                    contains: None,
                    favorites_only: fav_filter,
                    limit: None,
                    tag: tag_filter.clone(),
                    rank: false,
                }) {
                    last_known_total = Some(total);
                }
            }
            last_fetch = Instant::now();
        }
        if let Some(t) = exit_after {
            if Instant::now() >= t {
                break;
            }
        }
    }

    if draw {
        disable_raw_mode()?;
        if used_alt_screen {
            crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        }
    }
    if let Some(e) = copy_error {
        eprintln!("{}", e);
    }
    if let Some(id) = pending_print_id {
        println!("{}", id);
    }
    Ok(None)
}

fn fetch_page_from_daemon(
    images: bool,
    favorites: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    query: Option<String>,
    tag: Option<String>,
) -> Result<Page<Item>> {
    let info_path = config::config_dir().join("clipd.json");
    let v = fs::read(&info_path)?;
    let info: DaemonInfo = serde_json::from_slice(&v)?;
    let mut stream = TcpStream::connect(("127.0.0.1", info.port))?;
    let req = Request::List {
        images,
        favorites,
        limit,
        offset,
        query,
        tag,
    };
    let s = serde_json::to_string(&req)?;
    writeln!(&mut stream, "{}", s)?;
    use std::io::BufRead;
    let mut reader = io::BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let resp: Response<Page<Item>> = serde_json::from_str(&line)?;
    if resp.ok {
        Ok(resp.data.unwrap_or(Page {
            items: Vec::new(),
            more: false,
            total: None,
        }))
    } else {
        anyhow::bail!(resp.error.unwrap_or_else(|| "daemon error".into()))
    }
}

fn fetch_from_store(
    store: &dyn Store,
    images: bool,
    favorites: bool,
    limit: Option<usize>,
    tag: Option<String>,
) -> Result<Vec<Item>> {
    if images {
        let items = store.list_images(Query {
            contains: None,
            favorites_only: favorites,
            limit,
            tag,
            rank: false,
        })?;
        Ok(items
            .into_iter()
            .map(|(c, m)| Item::Image {
                id: c.id,
                favorite: c.is_favorite,
                created_at: c.created_at.unix_timestamp_nanos() as i64,
                last_used_at: c.last_used_at.map(|t| t.unix_timestamp_nanos() as i64),
                width: m.width,
                height: m.height,
                format: m.format,
                path: c.image_path,
            })
            .collect())
    } else {
        let items = store.list(Query {
            contains: None,
            favorites_only: favorites,
            limit,
            tag,
            rank: false,
        })?;
        Ok(items
            .into_iter()
            .map(|c| Item::Text {
                id: c.id,
                favorite: c.is_favorite,
                created_at: c.created_at.unix_timestamp_nanos() as i64,
                last_used_at: c.last_used_at.map(|t| t.unix_timestamp_nanos() as i64),
                text: c.text,
            })
            .collect())
    }
}

// (clipboard helpers moved to crate::copy_helpers)

fn truncate_msg(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max])
    } else {
        s.to_string()
    }
}

fn inner(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let mid = vert[1];
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(mid);
    horiz[1]
}

fn resolve_db_path_from_settings() -> PathBuf {
    let settings = crate::config::load_settings();
    match settings.storage {
        crate::config::Storage::LocalSqlite { db_path } => {
            db_path.unwrap_or_else(|| crate::config::config_dir().join("db").join("ditox.db"))
        }
        _ => crate::config::config_dir().join("db").join("ditox.db"),
    }
}

fn migrate_current_db() -> anyhow::Result<()> {
    let path = resolve_db_path_from_settings();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let impls = StoreImpl::new_with(&path, true)?; // auto-migrate on open
    impls.migrate_all()?;
    Ok(())
}

fn rel_time_ns(ts_ns: i64) -> String {
    let now_ns = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let delta_ns = now_ns.saturating_sub(ts_ns as i128);
    if delta_ns <= 0 {
        return "just now".into();
    }
    let sec = (delta_ns / 1_000_000_000) as i64;
    let delta = sec;
    if delta <= 0 {
        return "just now".into();
    }
    // seconds, minutes, hours, days
    if delta < 60 {
        return "just now".into();
    }
    let minutes = delta / 60;
    if minutes < 60 {
        return format!("{}m ago", minutes);
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{}h ago", hours);
    }
    let days = hours / 24;
    if days < 7 {
        return format!("{}d ago", days);
    }
    let weeks = days / 7;
    if weeks < 5 {
        return format!("{}w ago", weeks);
    }
    // Fallback to date for older items
    let dt = time::OffsetDateTime::from_unix_timestamp_nanos(ts_ns as i128)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let date = dt.date();
    format!(
        "{}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn date_fmt(ts_ns: i64) -> String {
    let dt = time::OffsetDateTime::from_unix_timestamp_nanos(ts_ns as i128)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let d = dt.date();
    let dd = format!("{:02}", d.day());
    let mm = format!("{:02}", u8::from(d.month()));
    let yyyy = format!("{}", d.year());
    let fmt = std::env::var("DITOX_TUI_DATE_FMT").unwrap_or_else(|_| "dd-mm-yyyy".to_string());
    fmt.replace("dd", &dd)
        .replace("mm", &mm)
        .replace("yyyy", &yyyy)
}

fn fmt_auto_ns(ts_ns: i64) -> String {
    // If within N days (default 3), show relative like `10m ago`; otherwise formatted date
    let now_ns = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let delta_ns = now_ns.saturating_sub(ts_ns as i128);
    let sec = (delta_ns / 1_000_000_000) as i64;
    let days_threshold: i64 = std::env::var("DITOX_TUI_AUTO_DAYS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(3);
    if sec < days_threshold * 24 * 3600 {
        rel_time_ns(ts_ns)
    } else {
        date_fmt(ts_ns)
    }
}

fn most_recent(created_ns: i64, last_used_ns: Option<i64>) -> (i64, &'static str) {
    if let Some(lu) = last_used_ns {
        if lu >= created_ns {
            (lu, "last_used")
        } else {
            (created_ns, "created")
        }
    } else {
        (created_ns, "created")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use ditox_core::{Store, StoreImpl};
    use tempfile::tempdir;

    struct FakeEvents {
        events: std::collections::VecDeque<Event>,
    }
    impl EventSource for FakeEvents {
        fn poll(&mut self, _timeout: Duration) -> anyhow::Result<Option<Event>> {
            Ok(self.events.pop_front())
        }
    }

    #[test]
    fn headless_flow_returns_expected_id() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("p.db");
        let store = StoreImpl::new_with(&db, true).unwrap();
        let c1 = store.add("hello world").unwrap();
        let _ = store.add("second").unwrap();
        let mut q = std::collections::VecDeque::new();
        // enter search mode then type 'hello'
        q.push_back(Event::Key(KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }));
        for ch in ['h', 'e', 'l', 'l', 'o'] {
            q.push_back(Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }));
        }
        q.push_back(Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }));
        let mut es = FakeEvents { events: q };
        let selected = run_picker_with(
            &store, false, false, None, true, &mut es, false, false, false, None, None,
        )
        .unwrap();
        assert_eq!(selected.as_deref(), Some(c1.id.as_str()));
    }

    #[test]
    fn favorites_only_shows_only_favorited_item() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("p2.db");
        let store = StoreImpl::new_with(&db, true).unwrap();
        let a = store.add("alpha").unwrap();
        let b = store.add("beta").unwrap();
        // Mark only `a` as favorite
        store.favorite(&a.id, true).unwrap();

        // Press Enter immediately to select the first (and only) item
        let mut q = std::collections::VecDeque::new();
        q.push_back(Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }));
        let mut es = FakeEvents { events: q };

        let picked = run_picker_with(
            &store, true, false, None, true, &mut es, false, false, false, None, None,
        )
        .unwrap();
        // Should select the only item available in favorites-only mode
        assert_eq!(picked.as_deref(), Some(a.id.as_str()));

        // Sanity: if favorites filter is off, we can pick `b` via search
        let mut q2 = std::collections::VecDeque::new();
        q2.push_back(Event::Key(KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }));
        for ch in ['b', 'e', 't', 'a'] {
            q2.push_back(Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }));
        }
        q2.push_back(Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }));
        let mut es2 = FakeEvents { events: q2 };
        let picked2 = run_picker_with(
            &store, false, false, None, true, &mut es2, false, false, false, None, None,
        )
        .unwrap();
        assert_eq!(picked2.as_deref(), Some(b.id.as_str()));
    }
}
