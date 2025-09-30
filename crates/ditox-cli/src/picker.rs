use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};
// no process or encoder imports needed here
use crate::copy_helpers;
use crate::theme;
use ditox_core::StoreImpl;
use std::path::PathBuf;

use crate::config;
use crate::preview;
use crate::{Query, Store};
// clipboard helpers are in copy_helpers module

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

pub fn run_picker_default(
    store: &dyn Store,
    favorites: bool,
    images: bool,
    tag: Option<String>,
    no_daemon: bool,
    force_wl_copy: bool,
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
pub fn run_picker_with(
    store: &dyn Store,
    favorites: bool,
    images: bool,
    tag: Option<String>,
    no_daemon: bool,
    es: &mut dyn EventSource,
    draw: bool,
    force_wl_copy: bool,
) -> Result<Option<String>> {
    const DAEMON_PAGE_FETCH: usize = 200;
    let use_daemon = !no_daemon;
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
    // Load initial dataset (paged via daemon; full via store fallback)
    let (mut items, mut has_more) = if use_daemon {
        match fetch_page_from_daemon(
            images_mode,
            fav_filter,
            Some(DAEMON_PAGE_FETCH),
            Some(0),
            None,
            tag_filter.clone(),
        ) {
            Ok(p) => (p.items, p.more),
            Err(_) => (
                fetch_from_store(store, images_mode, fav_filter, None, tag_filter.clone())?,
                false,
            ),
        }
    } else {
        (
            fetch_from_store(store, images_mode, fav_filter, None, tag_filter.clone())?,
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
    let mut selected = 0usize; // selected row within current page
                               // pagination & UI state
    let settings = crate::config::load_settings();
    let page_size: usize = settings
        .tui
        .as_ref()
        .and_then(|t| t.page_size)
        .filter(|&n| n > 0)
        .unwrap_or(10);
    let mut page_index: usize = 0; // 0-based page
    let mut selected_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut show_help: bool = false;
    let mut last_fetch: Instant = Instant::now();
    // input mode: do not capture characters until '/' pressed
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Normal,
        Query,
    }
    let mut mode = Mode::Normal;
    // when external filter changes (f/i/tag), we need to recompute filtered
    let mut needs_refilter = true;

    loop {
        // recompute filtered when query changes or filter toggles
        if needs_refilter || (mode == Mode::Query && query != last_query) {
            needs_refilter = false;
            if use_daemon && !images_mode {
                match fetch_page_from_daemon(
                    images_mode,
                    fav_filter,
                    Some(DAEMON_PAGE_FETCH),
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
                    }
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
            // Reset selection to top when filter/search changes
            page_index = 0;
            selected = 0;
        }
        if selected >= filtered.len() {
            selected = filtered.len().saturating_sub(1);
        }

        if let Some(ref mut term) = terminal {
            term.draw(|f| {
                let size = f.size();
                let footer_rows = if show_help { 9 } else { 3 };
                let chunks = if mode == Mode::Query {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3),  // search bar (only in search mode)
                            Constraint::Min(5),     // list
                            Constraint::Length(footer_rows),  // shortcuts/status (expandable)
                        ])
                        .split(size)
                } else {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(5),               // list only
                            Constraint::Length(footer_rows),  // shortcuts/status (expandable)
                        ])
                        .split(size)
                };

                if mode == Mode::Query {
                    let q_title = "Search — type to filter";
                    let q = Paragraph::new(query.as_str()).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(q_title)
                            .border_style(Style::default().fg(theme::load_tui_theme().border_fg)),
                    );
                    f.render_widget(q, chunks[0]);
                }

                // Pagination window
                let total = filtered.len();
                let total_pages = if total == 0 { 1 } else { (total - 1) / page_size + 1 };
                if page_index >= total_pages { page_index = total_pages.saturating_sub(1); }
                let start = page_index.saturating_mul(page_size);
                let end = (start + page_size).min(total);
                let visible = &filtered[start..end];

                let list_items: Vec<ListItem> = visible
                    .iter()
                    .filter_map(|&i| items.get(i))
                    .map(|it| match it {
                        Item::Text {
                            id, favorite, text, created_at, last_used_at, ..
                        } => {
                            let sel = if selected_ids.contains(id) { "[x]" } else { "[ ]" };
                            let fav = if *favorite { "*" } else { " " };
                            let line1 = format!("{}{} {}", fav, sel, preview(text));
                            let meta = Line::from(format!(
                                "Created {} • Last used {}",
                                rel_time(*created_at),
                                last_used_at
                                    .map(rel_time)
                                    .unwrap_or_else(|| "never".into())
                            ))
                            .style(Style::default().add_modifier(Modifier::DIM));
                            ListItem::new(vec![Line::from(line1), meta])
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
                            let sel = if selected_ids.contains(id) { "[x]" } else { "[ ]" };
                            let fav = if *favorite { "*" } else { " " };
                            let line1 = if name.is_empty() {
                                format!("{}{} {}x{} {}", fav, sel, width, height, format)
                            } else {
                                format!("{}{} {}x{} {} {}", fav, sel, width, height, format, name)
                            };
                            let meta = format!(
                                "Created {} • Last used {}",
                                rel_time(*created_at),
                                last_used_at.map(rel_time).unwrap_or_else(|| "never".into())
                            );
                            ListItem::new(vec![
                                Line::from(line1),
                                Line::from(meta).style(Style::default().add_modifier(Modifier::DIM)),
                            ])
                        }
                    })
                    .collect();
                let thm = theme::load_tui_theme();
                let mut title = if images_mode { String::from("Images") } else { String::from("Text") };
                if fav_filter { title.push_str(" — Favorites"); }
                if let Some(ref t) = tag_filter { if !t.is_empty() { title.push_str(&format!(" — Tag: {}", t)); } }
                // Counts + page
                let count_label = if fav_filter { format!(" — Total favorites {}", total) } else { format!(" — Total entries {}", total) };
                let page_label = format!(" — Page {}/{} (page size {})", page_index + 1, total_pages.max(1), page_size);
                title.push_str(&count_label);
                title.push_str(&page_label);
                let list = List::new(list_items)
                    .block(Block::default().borders(Borders::ALL).title(title).border_style(Style::default().fg(thm.border_fg)))
                    .highlight_style(Style::default().fg(thm.highlight_fg).bg(thm.highlight_bg).add_modifier(Modifier::REVERSED));
                let list_area_idx = if mode == Mode::Query { 1 } else { 0 };
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
                // Footer — simple vs expanded help
                let thm2 = theme::load_tui_theme();
                let footer_block = Block::default().borders(Borders::ALL).title("Shortcuts").border_style(Style::default().fg(thm.border_fg));
                let footer_area_idx = if mode == Mode::Query { 2 } else { 1 };
                if show_help {
                    // Render block and a 3-column layout inside it
                    let area = chunks[footer_area_idx];
                    f.render_widget(footer_block.clone(), area);
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(34),
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                        ])
                        .split(inner(area));
                    let col1 = Paragraph::new(
                        "↑/k up\n↓/j down\n→/l/PgDn next page\n←/h/PgUp prev page\nHome/g go to start\nEnd/G go to end",
                    )
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(thm2.help_fg));
                    let col2 = Paragraph::new(
                        "/ filter\ns select\nS clear selected\nTab favorites toggle\ni images toggle\nt apply #tag\nr refresh",
                    )
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(thm2.help_fg));
                    let mut col3_text = String::from("⏎ copy | x delete | p fav/unfav\nq quit\n? close help");
                    if has_more { col3_text.push_str("\nMore available…"); }
                    let col3 = Paragraph::new(col3_text)
                        .wrap(Wrap { trim: true })
                        .style(Style::default().fg(thm2.help_fg));
                    f.render_widget(col1, cols[0]);
                    f.render_widget(col2, cols[1]);
                    f.render_widget(col3, cols[2]);
                } else {
                    let mut simple = String::from("⏎ copy | x delete | p fav/unfav | Tab favorites | ? more");
                    if has_more { simple.push_str(" | More available…"); }
                    if let Some((msg, until)) = &toast { if Instant::now() <= *until { simple.push_str(&format!("  — {}", msg)); } }
                    let footer = Paragraph::new(simple)
                        .block(footer_block)
                        .style(Style::default().fg(thm2.help_fg))
                        .wrap(Wrap { trim: true });
                    f.render_widget(footer, chunks[footer_area_idx]);
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
                    KeyCode::Char('?') => {
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
                    }
                    KeyCode::Tab | KeyCode::Char('f') => {
                        fav_filter = !fav_filter;
                        last_query.clear();
                        needs_refilter = true;
                        selected = 0;
                        page_index = 0;
                        pending_delete_id = None;
                        pending_delete_until = None;
                        if use_daemon {
                            match fetch_page_from_daemon(
                                images_mode,
                                fav_filter,
                                Some(DAEMON_PAGE_FETCH),
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
                    }
                    KeyCode::Char('i') => {
                        images_mode = !images_mode;
                        selected = 0;
                        page_index = 0;
                        last_query.clear();
                        needs_refilter = true;
                        pending_delete_id = None;
                        pending_delete_until = None;
                        let load_res: anyhow::Result<()> = (|| {
                            if use_daemon {
                                let p = fetch_page_from_daemon(
                                    images_mode,
                                    fav_filter,
                                    Some(DAEMON_PAGE_FETCH),
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
                        })();
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
                    // Toggle favorite on current item or selection
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        let mut ids: Vec<String> = Vec::new();
                        if !selected_ids.is_empty() {
                            ids.extend(selected_ids.iter().cloned());
                        } else if let Some(idx) = filtered
                            .get(page_index.saturating_mul(page_size) + selected)
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
                                    Some(DAEMON_PAGE_FETCH),
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
                            if selected >= page_size {
                                selected = page_size.saturating_sub(1);
                            }
                        }
                    }
                    // Delete current item with quick confirm; or bulk delete selected
                    KeyCode::Delete | KeyCode::Char('x') | KeyCode::Char('X') => {
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
                                            Some(DAEMON_PAGE_FETCH),
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
                                    if selected >= page_size {
                                        selected = page_size.saturating_sub(1);
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
                                if let Ok(p) = fetch_page_from_daemon(
                                    images_mode,
                                    fav_filter,
                                    Some(DAEMON_PAGE_FETCH),
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
                            if selected >= page_size {
                                selected = page_size.saturating_sub(1);
                            }
                        }
                    }
                    KeyCode::Char('t') => {
                        // Apply current query as tag when pressing 't'
                        tag_filter = if query.trim().is_empty() {
                            None
                        } else {
                            Some(query.clone())
                        };
                        last_query.clear();
                        needs_refilter = true;
                        selected = 0;
                        page_index = 0;
                        if use_daemon {
                            match fetch_page_from_daemon(
                                images_mode,
                                fav_filter,
                                Some(DAEMON_PAGE_FETCH),
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
                    }
                    KeyCode::Char('r') => {
                        last_query.clear();
                        needs_refilter = true;
                        page_index = 0;
                        if use_daemon {
                            match fetch_page_from_daemon(
                                images_mode,
                                fav_filter,
                                Some(DAEMON_PAGE_FETCH),
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
                            has_more = false;
                        }
                        filtered = (0..items.len()).collect();
                    }
                    // Run migrations on DB (best-effort) and reload list
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        match migrate_current_db() {
                            Ok(()) => {
                                toast = Some((
                                    "Migrations applied".into(),
                                    Instant::now() + Duration::from_millis(1200),
                                ));
                                // reload items with current filters
                                if use_daemon {
                                    if let Ok(p) = fetch_page_from_daemon(
                                        images_mode,
                                        fav_filter,
                                        Some(DAEMON_PAGE_FETCH),
                                        Some(0),
                                        None,
                                        tag_filter.clone(),
                                    ) {
                                        items = p.items;
                                        has_more = p.more;
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
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected > 0 {
                            selected -= 1;
                        } else if page_index > 0 {
                            page_index -= 1;
                            selected = page_size.saturating_sub(1);
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let total = filtered.len();
                        let start = page_index.saturating_mul(page_size);
                        let end = (start + page_size).min(total);
                        let page_len = end.saturating_sub(start);
                        if selected + 1 < page_len {
                            selected += 1;
                        } else if end < total {
                            page_index += 1;
                            selected = 0;
                        } else if use_daemon && has_more && query.is_empty() {
                            // Optionally prefetch more from daemon when at end
                            if let Ok(p) = fetch_page_from_daemon(
                                images_mode,
                                fav_filter,
                                Some(DAEMON_PAGE_FETCH),
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
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::PageDown => {
                        let total = filtered.len();
                        let start = (page_index + 1).saturating_mul(page_size);
                        if start < total {
                            page_index += 1;
                            selected = 0;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::PageUp => {
                        if page_index > 0 {
                            page_index -= 1;
                            selected = 0;
                        }
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        page_index = 0;
                        selected = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        let total = filtered.len();
                        if total > 0 {
                            page_index = (total - 1) / page_size;
                            let start = page_index * page_size;
                            selected = (total - start).saturating_sub(1);
                        }
                    }
                    KeyCode::Char('s') => {
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
                    KeyCode::Char('S') => {
                        selected_ids.clear();
                    }
                    KeyCode::Enter => {
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
                                    Some(DAEMON_PAGE_FETCH),
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
                        let start = page_index.saturating_mul(page_size);
                        if let Some(idx) = filtered.get(start + selected).cloned() {
                            // perform copy and exit
                            match &items[idx] {
                                Item::Text { id, text, .. } => {
                                    if let Err(e) = copy_helpers::copy_text(text, force_wl_copy) {
                                        copy_error = Some(format!("copy failed: {}", e));
                                    } else {
                                        toast = Some((
                                            String::from("Copied!"),
                                            Instant::now() + Duration::from_millis(500),
                                        ));
                                        exit_after =
                                            Some(Instant::now() + Duration::from_millis(500));
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
                                            toast = Some((
                                                String::from("Copied!"),
                                                Instant::now() + Duration::from_millis(500),
                                            ));
                                            exit_after =
                                                Some(Instant::now() + Duration::from_millis(500));
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
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // refresh from daemon periodically (e.g., new clips) when idle
        if use_daemon && last_fetch.elapsed() > Duration::from_millis(1500) && query.is_empty() {
            match fetch_page_from_daemon(
                images_mode,
                fav_filter,
                Some(DAEMON_PAGE_FETCH),
                Some(0),
                None,
                tag_filter.clone(),
            ) {
                Ok(p) => {
                    items = p.items;
                    has_more = p.more;
                    last_query.clear();
                }
                Err(_) => { /* ignore periodic refresh when daemon missing */ }
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
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
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
                last_used_at: c.last_used_at.map(|t| t.unix_timestamp()),
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
                last_used_at: c.last_used_at.map(|t| t.unix_timestamp()),
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

fn rel_time(ts: i64) -> String {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let delta = now.saturating_sub(ts);
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
    let dt =
        time::OffsetDateTime::from_unix_timestamp(ts).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let date = dt.date();
    format!(
        "{}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
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
        let selected =
            run_picker_with(&store, false, false, None, true, &mut es, false, false).unwrap();
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

        let picked =
            run_picker_with(&store, true, false, None, true, &mut es, false, false).unwrap();
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
        let picked2 =
            run_picker_with(&store, false, false, None, true, &mut es2, false, false).unwrap();
        assert_eq!(picked2.as_deref(), Some(b.id.as_str()));
    }
}
