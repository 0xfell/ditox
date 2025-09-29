use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::config;
use crate::preview;
use crate::{Query, Store};
use ditox_core::clipboard::Clipboard;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Item {
    Text {
        id: String,
        favorite: bool,
        created_at: i64,
        text: String,
    },
    Image {
        id: String,
        favorite: bool,
        created_at: i64,
        width: u32,
        height: u32,
        format: String,
        path: Option<String>,
    },
}

pub fn run_picker_default(
    store: &dyn Store,
    favorites: bool,
    images: bool,
    tag: Option<String>,
    no_daemon: bool,
) -> Result<()> {
    let mut es = RealEventSource;
    let _ = run_picker_with(store, favorites, images, tag, no_daemon, &mut es, true)?;
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

pub fn run_picker_with(
    store: &dyn Store,
    favorites: bool,
    images: bool,
    tag: Option<String>,
    no_daemon: bool,
    es: &mut dyn EventSource,
    draw: bool,
) -> Result<Option<String>> {
    const PAGE_SIZE: usize = 200;
    let use_daemon = !no_daemon;
    // Load initial dataset (paged via daemon; full via store fallback)
    let (mut items, mut has_more) = if use_daemon {
        match fetch_page_from_daemon(
            images,
            favorites,
            Some(PAGE_SIZE),
            Some(0),
            None,
            tag.clone(),
        ) {
            Ok(p) => (p.items, p.more),
            Err(_) => (
                fetch_from_store(store, images, favorites, None, tag.clone())?,
                false,
            ),
        }
    } else {
        (
            fetch_from_store(store, images, favorites, None, tag.clone())?,
            false,
        )
    };

    let mut terminal = if draw {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        Some(Terminal::new(backend)?)
    } else {
        None
    };

    let mut query = String::new();
    let matcher = SkimMatcherV2::default();
    let mut filtered: Vec<usize> = (0..items.len()).collect();
    let mut last_query = String::new();
    let mut selected = 0usize;
    let mut last_fetch: Instant = Instant::now();

    loop {
        // recompute filtered when query changes
        if query != last_query {
            if use_daemon && !images {
                if let Ok(p) = fetch_page_from_daemon(
                    images,
                    favorites,
                    Some(PAGE_SIZE),
                    Some(0),
                    Some(query.clone()),
                    tag.clone(),
                ) {
                    items = p.items;
                    has_more = p.more;
                }
                filtered = (0..items.len()).collect();
            } else {
                filtered = if query.trim().is_empty() {
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
        }
        if selected >= filtered.len() {
            selected = filtered.len().saturating_sub(1);
        }

        if let Some(ref mut term) = terminal {
            term.draw(|f| {
                let size = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(5)])
                    .split(size);
                let q = Paragraph::new(query.as_str()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Query (Esc cancel, Enter copy)"),
                );
                f.render_widget(q, chunks[0]);

                let list_items: Vec<ListItem> = filtered
                    .iter()
                    .take(500)
                    .map(|&i| match &items[i] {
                        Item::Text {
                            id, favorite, text, ..
                        } => ListItem::new(Line::from(format!(
                            "{} {} {}",
                            if *favorite { "*" } else { " " },
                            id,
                            preview(text)
                        ))),
                        Item::Image {
                            id,
                            favorite,
                            width,
                            height,
                            format,
                            path,
                            ..
                        } => {
                            let name = path
                                .as_deref()
                                .and_then(|p| {
                                    std::path::Path::new(p).file_name().and_then(|n| n.to_str())
                                })
                                .unwrap_or("");
                            ListItem::new(Line::from(format!(
                                "{} {} {}x{} {} {}",
                                if *favorite { "*" } else { " " },
                                id,
                                width,
                                height,
                                format,
                                name
                            )))
                        }
                    })
                    .collect();
                let list = List::new(list_items)
                    .block(Block::default().borders(Borders::ALL).title(if images {
                        "Images"
                    } else {
                        "Text"
                    }))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                f.render_stateful_widget(
                    list,
                    chunks[1],
                    &mut ratatui::widgets::ListState::default().with_selected(
                        if filtered.is_empty() {
                            None
                        } else {
                            Some(selected)
                        },
                    ),
                );
            })?;
        }

        if let Some(ev) = es.poll(Duration::from_millis(100))? {
            match ev {
                Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Esc => break,
                    KeyCode::Char('c')
                        if k.modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break
                    }
                    KeyCode::Char(ch) => query.push(ch),
                    KeyCode::Backspace => {
                        query.pop();
                    }
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if selected + 1 < filtered.len() {
                            selected += 1;
                            if use_daemon
                                && has_more
                                && query.is_empty()
                                && filtered.len().saturating_sub(selected) < 5
                            {
                                if let Ok(p) = fetch_page_from_daemon(
                                    images,
                                    favorites,
                                    Some(PAGE_SIZE),
                                    Some(items.len()),
                                    None,
                                    tag.clone(),
                                ) {
                                    has_more = p.more;
                                    items.extend(p.items);
                                    last_query.clear(); // trigger re-filter include new items
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = filtered.get(selected).cloned() {
                            // perform copy and exit
                            match &items[idx] {
                                Item::Text { id, text, .. } => {
                                    // copy via clipboard
                                    #[cfg(target_os = "linux")]
                                    let cb = crate::SystemClipboard::new();
                                    #[cfg(not(target_os = "linux"))]
                                    let cb = ditox_core::clipboard::NoopClipboard::default();
                                    let _ = cb.set_text(text);
                                    println!("{}", id);
                                    if !draw {
                                        return Ok(Some(id.clone()));
                                    }
                                }
                                Item::Image { id, .. } => {
                                    if let Ok(Some(img)) = store.get_image_rgba(id) {
                                        #[cfg(target_os = "linux")]
                                        let cb = crate::SystemClipboard::new();
                                        #[cfg(not(target_os = "linux"))]
                                        let cb = ditox_core::clipboard::NoopClipboard::default();
                                        let _ = cb.set_image(&img);
                                    }
                                    println!("{}", id);
                                    if !draw {
                                        return Ok(Some(id.clone()));
                                    }
                                }
                            }
                        }
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // refresh from daemon periodically (e.g., new clips) when idle
        if use_daemon && last_fetch.elapsed() > Duration::from_millis(1500) && query.is_empty() {
            if let Ok(p) = fetch_page_from_daemon(
                images,
                favorites,
                Some(PAGE_SIZE),
                Some(0),
                None,
                tag.clone(),
            ) {
                items = p.items;
                has_more = p.more;
                last_query.clear();
            }
            last_fetch = Instant::now();
        }
    }

    if draw {
        disable_raw_mode()?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
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
                created_at: c.created_at.unix_timestamp(),
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
                created_at: c.created_at.unix_timestamp(),
                text: c.text,
            })
            .collect())
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
        let selected = run_picker_with(&store, false, false, None, true, &mut es, false).unwrap();
        assert_eq!(selected.as_deref(), Some(c1.id.as_str()));
    }
}
