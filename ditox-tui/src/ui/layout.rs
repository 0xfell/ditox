use ditox_core::app::{App, InputMode};
use crate::keybindings::KeybindingResolver;
use crate::ui::confirm;
use crate::ui::help;
use crate::ui::list;
use crate::ui::note_editor;
use crate::ui::preview::{self, ImageCache, ImageLoader};
use crate::ui::search;
use crate::ui::tabs;
use crate::ui::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui_image::picker::Picker;

/// Minimum terminal dimensions
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 10;

pub fn draw(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    cache: &mut ImageCache,
    picker: &mut Option<Picker>,
    loader: &ImageLoader,
    keybindings: &KeybindingResolver,
) {
    let area = frame.area();

    // Update terminal height for page navigation
    app.terminal_height = area.height;

    // Check minimum terminal size
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_size_warning(frame, area, theme);
        return;
    }

    // Narrow terminal handling - auto-hide UI elements for graceful degradation
    // Width < 80: hide tabs
    // Width < 100: hide snippets
    let effective_show_tabs = app.show_tabs && area.width >= 80;
    let effective_show_snippets = app.show_snippets && area.width >= 100;

    // Expanded preview mode - show only the selected entry fullscreen
    if app.show_expanded {
        draw_expanded(frame, app, theme, area, cache, picker, loader);
        return;
    }

    // Build layout based on whether tabs are shown (respecting narrow terminal)
    let chunks = if effective_show_tabs {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tabs
                Constraint::Length(3), // Search
                Constraint::Min(5),    // Content
                Constraint::Length(1), // Status
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search
                Constraint::Min(5),    // Content
                Constraint::Length(1), // Status
            ])
            .split(area)
    };

    // Get chunk indices based on layout
    let (tabs_chunk, search_chunk, content_chunk, status_chunk) = if effective_show_tabs {
        (Some(chunks[0]), chunks[1], chunks[2], chunks[3])
    } else {
        (None, chunks[0], chunks[1], chunks[2])
    };

    // Tabs bar (if shown)
    if let Some(tabs_area) = tabs_chunk {
        tabs::draw(frame, app, theme, tabs_area);
    }

    // Search bar
    search::draw(frame, app, theme, search_chunk);

    // Content area (list + optional preview)
    if app.show_preview && area.width > 60 {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(content_chunk);

        list::draw(frame, app, theme, content_chunks[0]);
        preview::draw(frame, app, theme, content_chunks[1], cache, picker, loader);
    } else {
        list::draw(frame, app, theme, content_chunk);
    }

    // Status bar
    draw_status(frame, app, theme, status_chunk, effective_show_snippets);

    // Help overlay
    if app.show_help {
        help::draw(frame, theme, keybindings);
    }

    // Note editor overlay
    if app.input_mode == InputMode::EditNote {
        note_editor::render(frame, &app.note_input, theme);
    }

    // Confirmation dialog overlay
    if app.input_mode == InputMode::Confirm {
        confirm::draw(frame, app, theme);
    }
}

fn draw_expanded(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    cache: &mut ImageCache,
    picker: &mut Option<Picker>,
    loader: &ImageLoader,
) {
    use ditox_core::entry::EntryType;
    use ratatui_image::StatefulImage;

    // Check for completed image loads
    while let Some(response) = loader.try_recv() {
        if let (Some(picker), Some(img)) = (picker.as_mut(), response.image) {
            let protocol = picker.new_resize_protocol(img);
            cache.insert(response.path, protocol);
        } else {
            cache.clear_pending();
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Content
            Constraint::Length(1), // Status
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Expanded Preview (Esc to close) ")
        .title_style(theme.title());

    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);

    match app.selected_entry() {
        Some(entry) => match entry.entry_type {
            EntryType::Text => {
                // Show full text content with scroll area
                let paragraph = Paragraph::new(entry.sanitized_content())
                    .style(theme.normal())
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, inner);
            }
            EntryType::Image => {
                // Split into image and info
                let img_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(4),    // Image (most of the space)
                        Constraint::Length(3), // Info
                    ])
                    .split(inner);

                let path = &entry.content;

                // Render image
                if let Some(protocol) = cache.get_mut(path) {
                    let image_widget = StatefulImage::default();
                    frame.render_stateful_widget(image_widget, img_chunks[0], protocol);
                } else if !cache.is_pending(path) {
                    cache.set_pending(path);
                    loader.request_load(path);
                    let placeholder = Paragraph::new("Loading image...")
                        .style(theme.muted())
                        .alignment(Alignment::Center);
                    frame.render_widget(placeholder, img_chunks[0]);
                } else {
                    let placeholder = Paragraph::new("Loading image...")
                        .style(theme.muted())
                        .alignment(Alignment::Center);
                    frame.render_widget(placeholder, img_chunks[0]);
                }

                // Render info
                let dimensions_str = if let Some((w, h)) = image::image_dimensions(&entry.content).ok() {
                    format!("{}x{}", w, h)
                } else {
                    "unknown".to_string()
                };
                let info = format!(
                    "Path: {} │ Size: {} │ {} │ Created: {}",
                    entry.content,
                    format_size(entry.byte_size),
                    dimensions_str,
                    entry.created_at.format("%Y-%m-%d %H:%M:%S")
                );
                let info_paragraph = Paragraph::new(info)
                    .style(theme.muted())
                    .wrap(Wrap { trim: true });
                frame.render_widget(info_paragraph, img_chunks[1]);
            }
        },
        None => {
            let paragraph = Paragraph::new("No entry selected")
                .style(theme.muted())
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, inner);
        }
    }

    // Status bar
    let status = " t/Esc: Close │ j/k: Navigate │ Enter: Copy+Exit │ y: Copy";
    let status_bar = Paragraph::new(status)
        .style(theme.muted())
        .block(Block::default());
    frame.render_widget(status_bar, chunks[1]);
}

fn draw_status(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, show_snippets: bool) {
    // Check watcher status
    let watcher_status = if ditox_core::watcher::is_watcher_running() {
        "●"  // Green dot (will be styled)
    } else {
        "○"  // Empty dot
    };

    let status = if let Some(msg) = &app.message {
        format!(" {} │ {}", watcher_status, msg)
    } else if app.multi_select_mode {
        // Multi-select mode status
        let selected_count = app.multi_selected.len();
        format!(
            " {} │ [MULTI] Space:Select  v:All  d:Delete  y:Copy  Esc:Exit │ {} selected",
            watcher_status,
            selected_count
        )
    } else {
        // Show snippet hints if any slots are filled (respecting narrow terminal)
        let has_snippets = app.snippet_slots.iter().any(|s| s.is_some());
        let snippet_hint = if has_snippets && show_snippets {
            "1-9:Snippet  "
        } else {
            ""
        };
        let keybindings = format!("{}j/k:Move  Enter:Copy  /:Search  ?:Help  q:Quit", snippet_hint);
        let entry_count = if !app.search_query.is_empty() {
            // Show filtered/total when searching
            format!("{}/{} filtered", app.filtered.len(), app.entries.len())
        } else {
            format!("{} entries", app.entries.len())
        };
        let refresh_time = app.time_since_refresh();
        format!(" {} │ {} │ {} │ Updated: {} ago", watcher_status, keybindings, entry_count, refresh_time)
    };

    let status_bar = Paragraph::new(status)
        .style(theme.muted())
        .block(Block::default());

    frame.render_widget(status_bar, area);
}

fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Draw a warning when terminal is too small
fn draw_size_warning(frame: &mut Frame, area: Rect, theme: &Theme) {
    let msg = format!(
        "Terminal too small\n\nCurrent: {}x{}\nMinimum: {}x{}\n\nPlease resize your terminal",
        area.width, area.height, MIN_WIDTH, MIN_HEIGHT
    );

    let paragraph = Paragraph::new(msg)
        .style(theme.muted())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(" ditox ")
                .title_style(theme.title()),
        );

    frame.render_widget(paragraph, area);
}
