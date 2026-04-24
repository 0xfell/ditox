use ditox_core::app::{App, PreviewMode};
use ditox_core::entry::EntryType;
use crate::ui::theme::Theme;
use image::DynamicImage;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Message to request image loading
pub enum ImageRequest {
    Load(String), // path
    Shutdown,
}

/// Message with loaded image
pub struct ImageResponse {
    pub path: String,
    pub image: Option<DynamicImage>,
}

/// Background image loader
pub struct ImageLoader {
    tx: Sender<ImageRequest>,
    rx: Receiver<ImageResponse>,
}

impl ImageLoader {
    pub fn new() -> Self {
        let (req_tx, req_rx) = channel::<ImageRequest>();
        let (resp_tx, resp_rx) = channel::<ImageResponse>();

        // Spawn background thread
        thread::spawn(move || {
            while let Ok(request) = req_rx.recv() {
                match request {
                    ImageRequest::Load(path) => {
                        let image = load_image(&path);
                        let _ = resp_tx.send(ImageResponse { path, image });
                    }
                    ImageRequest::Shutdown => break,
                }
            }
        });

        Self {
            tx: req_tx,
            rx: resp_rx,
        }
    }

    pub fn request_load(&self, path: &str) {
        let _ = self.tx.send(ImageRequest::Load(path.to_string()));
    }

    pub fn try_recv(&self) -> Option<ImageResponse> {
        self.rx.try_recv().ok()
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(ImageRequest::Shutdown);
    }
}

fn load_image(path: &str) -> Option<DynamicImage> {
    let img_path = Path::new(path);
    if !img_path.exists() {
        return None;
    }
    image::open(img_path).ok()
}

/// Get image dimensions without loading the full image
fn get_image_dimensions(path: &str) -> Option<(u32, u32)> {
    let img_path = Path::new(path);
    if !img_path.exists() {
        return None;
    }
    image::image_dimensions(img_path).ok()
}

/// Cache for rendered image protocols
pub struct ImageCache {
    cache: HashMap<String, StatefulProtocol>,
    pending: Option<String>,
    max_size: usize,
}

impl ImageCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            pending: None,
            max_size,
        }
    }

    pub fn get_mut(&mut self, path: &str) -> Option<&mut StatefulProtocol> {
        self.cache.get_mut(path)
    }

    #[allow(dead_code)] // May be useful for cache inspection
    pub fn contains(&self, path: &str) -> bool {
        self.cache.contains_key(path)
    }

    pub fn is_pending(&self, path: &str) -> bool {
        self.pending.as_ref().map(|p| p == path).unwrap_or(false)
    }

    pub fn set_pending(&mut self, path: &str) {
        self.pending = Some(path.to_string());
    }

    pub fn insert(&mut self, path: String, protocol: StatefulProtocol) {
        // Evict if at capacity (simple strategy: clear all)
        if self.cache.len() >= self.max_size {
            self.cache.clear();
        }
        self.cache.insert(path.clone(), protocol);
        if self.pending.as_ref() == Some(&path) {
            self.pending = None;
        }
    }

    pub fn clear_pending(&mut self) {
        self.pending = None;
    }
}

pub fn draw(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    cache: &mut ImageCache,
    picker: &mut Option<Picker>,
    loader: &ImageLoader,
) {
    // Show mode in title
    let title = format!(" Preview ({}) ", app.preview_mode.label());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(title)
        .title_style(theme.title());

    // Check for completed image loads
    while let Some(response) = loader.try_recv() {
        if let (Some(picker), Some(img)) = (picker.as_mut(), response.image) {
            let protocol = picker.new_resize_protocol(img);
            cache.insert(response.path, protocol);
        } else {
            cache.clear_pending();
        }
    }

    match app.selected_entry() {
        Some(entry) => {
            // Get match indices for current entry
            let entry_idx = app
                .filtered
                .get(app.selected)
                .copied();
            let match_indices = entry_idx.and_then(|idx| app.match_indices.get(&idx));

            match entry.entry_type {
                EntryType::Text => {
                    render_text_preview(
                        frame,
                        app,
                        theme,
                        area,
                        block,
                        &entry.content,
                        match_indices,
                        app.show_line_numbers,
                    );
                }
                EntryType::Image => {
                    let inner = block.inner(area);
                    frame.render_widget(block, area);

                    if inner.height < 4 {
                        return;
                    }

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(4),    // Image
                            Constraint::Length(4), // Info
                        ])
                        .split(inner);

                    // `entry.content` is the content-addressable hash; resolve
                    // to the real on-disk path before handing it to the image
                    // loader / cache.
                    let path_buf = entry.image_path().unwrap_or_default();
                    let path = path_buf.to_string_lossy().into_owned();
                    let path: &str = &path;
                    let mut image_rendered = false;

                    // Check cache first
                    if let Some(protocol) = cache.get_mut(path) {
                        let image_widget = StatefulImage::default();
                        frame.render_stateful_widget(image_widget, chunks[0], protocol);
                        image_rendered = true;
                    } else if !cache.is_pending(path) {
                        // Request background load
                        cache.set_pending(path);
                        loader.request_load(path);
                    }

                    if !image_rendered {
                        let loading_text = if cache.is_pending(path) {
                            "Loading image..."
                        } else {
                            "[Image preview not available]"
                        };
                        let placeholder = Paragraph::new(loading_text)
                            .style(theme.muted())
                            .alignment(Alignment::Center);
                        frame.render_widget(placeholder, chunks[0]);
                    }

                    // Render image info
                    let dimensions_str = if let Some((w, h)) = get_image_dimensions(path) {
                        format!("{}x{}", w, h)
                    } else {
                        "unknown".to_string()
                    };
                    let info = format!(
                        "Path: {}\nSize: {} │ Dimensions: {}\nCreated: {}",
                        path,
                        format_size(entry.byte_size),
                        dimensions_str,
                        entry.created_at.format("%Y-%m-%d %H:%M:%S")
                    );

                    let info_paragraph = Paragraph::new(info)
                        .style(theme.muted())
                        .wrap(Wrap { trim: true });

                    frame.render_widget(info_paragraph, chunks[1]);
                }
            }
        }
        None => {
            let paragraph = Paragraph::new("No entry selected")
                .style(theme.muted())
                .block(block)
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, area);
        }
    }
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

/// Create highlighted text with search match highlighting
fn create_highlighted_text<'a>(content: &str, indices: &[u32], theme: &Theme) -> Text<'a> {
    let match_set: HashSet<u32> = indices.iter().copied().collect();
    let normal_style = theme.normal();
    let highlight_style = theme.highlight();

    let mut lines: Vec<Line> = Vec::new();

    for line_content in content.lines() {
        let mut spans: Vec<Span> = Vec::new();
        let mut current_str = String::new();
        let mut in_highlight = false;

        // Calculate the starting char index for this line in the full content
        // Note: This is a simplified version - for very long content with many lines,
        // we'd need to track the cumulative character offset properly
        let line_start_idx = content
            .find(line_content)
            .unwrap_or(0);

        for (i, ch) in line_content.chars().enumerate() {
            let global_idx = (line_start_idx + i) as u32;
            let is_match = match_set.contains(&global_idx);

            if is_match != in_highlight {
                if !current_str.is_empty() {
                    let style = if in_highlight {
                        highlight_style
                    } else {
                        normal_style
                    };
                    spans.push(Span::styled(std::mem::take(&mut current_str), style));
                }
                in_highlight = is_match;
            }
            current_str.push(ch);
        }

        // Flush remaining
        if !current_str.is_empty() {
            let style = if in_highlight {
                highlight_style
            } else {
                normal_style
            };
            spans.push(Span::styled(current_str, style));
        }

        lines.push(Line::from(spans));
    }

    Text::from(lines)
}

/// Render text preview based on the current preview mode
fn render_text_preview(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    block: Block,
    content: &str,
    match_indices: Option<&Vec<u32>>,
    show_line_numbers: bool,
) {
    match app.preview_mode {
        PreviewMode::Wrap => {
            render_wrap_mode(frame, theme, area, block, content, match_indices, show_line_numbers);
        }
        PreviewMode::Scroll => {
            render_scroll_mode(frame, app, theme, area, block, content, show_line_numbers);
        }
        PreviewMode::Truncate => {
            render_truncate_mode(frame, theme, area, block, content, show_line_numbers);
        }
        PreviewMode::Hex => {
            render_hex_mode(frame, theme, area, block, content);
        }
        PreviewMode::Raw => {
            render_raw_mode(frame, theme, area, block, content);
        }
    }
}

/// Wrap mode - text wraps at pane width (default)
fn render_wrap_mode(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    block: Block,
    content: &str,
    match_indices: Option<&Vec<u32>>,
    show_line_numbers: bool,
) {
    // Sanitize content for display
    let sanitized = sanitize_for_display(content);

    let text = if show_line_numbers {
        // Add line numbers to each line
        let lines: Vec<Line> = sanitized
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let line_num = format!("{:>4} │ ", i + 1);
                let mut spans = vec![Span::styled(line_num, theme.muted())];
                spans.push(Span::styled(line.to_string(), theme.normal()));
                Line::from(spans)
            })
            .collect();
        Text::from(lines)
    } else if let Some(indices) = match_indices {
        create_highlighted_text(&sanitized, indices, theme)
    } else {
        Text::styled(sanitized, theme.normal())
    };

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Scroll mode - horizontal scroll for long lines
fn render_scroll_mode(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    block: Block,
    content: &str,
    show_line_numbers: bool,
) {
    let sanitized = sanitize_for_display(content);
    let offset = app.preview_scroll_offset;

    // Build lines with horizontal offset applied
    let lines: Vec<Line> = sanitized
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let chars: Vec<char> = line.chars().collect();
            let content_str = if offset >= chars.len() {
                String::new()
            } else {
                chars[offset..].iter().collect()
            };

            if show_line_numbers {
                let line_num = format!("{:>4} │ ", i + 1);
                Line::from(vec![
                    Span::styled(line_num, theme.muted()),
                    Span::styled(content_str, theme.normal()),
                ])
            } else {
                Line::from(Span::styled(content_str, theme.normal()))
            }
        })
        .collect();

    let text = Text::from(lines);

    // Show scroll indicator in the block title
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, inner);

    // Show scroll position indicator at bottom
    if offset > 0 {
        let indicator = format!("← {} cols", offset);
        let indicator_span = Span::styled(indicator, theme.muted());
        let indicator_para = Paragraph::new(indicator_span);
        if inner.height > 1 {
            let indicator_area = Rect {
                x: inner.x,
                y: inner.y + inner.height - 1,
                width: inner.width,
                height: 1,
            };
            frame.render_widget(indicator_para, indicator_area);
        }
    }
}

/// Truncate mode - first N lines with "... X more lines" indicator
fn render_truncate_mode(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    block: Block,
    content: &str,
    show_line_numbers: bool,
) {
    let sanitized = sanitize_for_display(content);
    let inner = block.inner(area);
    let max_lines = (inner.height.saturating_sub(1)) as usize; // Reserve 1 line for indicator

    let all_lines: Vec<&str> = sanitized.lines().collect();
    let total_lines = all_lines.len();

    let mut display_lines: Vec<Line> = all_lines
        .iter()
        .enumerate()
        .take(max_lines)
        .map(|(i, &line)| {
            if show_line_numbers {
                let line_num = format!("{:>4} │ ", i + 1);
                Line::from(vec![
                    Span::styled(line_num, theme.muted()),
                    Span::styled(line.to_string(), theme.normal()),
                ])
            } else {
                Line::from(Span::styled(line.to_string(), theme.normal()))
            }
        })
        .collect();

    // Add "more lines" indicator if truncated
    if total_lines > max_lines {
        let remaining = total_lines - max_lines;
        let indicator = format!("... {} more lines", remaining);
        display_lines.push(Line::from(Span::styled(indicator, theme.muted())));
    }

    let text = Text::from(display_lines);
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

/// Hex mode - hex dump view
fn render_hex_mode(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    block: Block,
    content: &str,
) {
    let bytes = content.as_bytes();
    let inner = block.inner(area);
    let max_lines = inner.height as usize;

    let mut lines: Vec<Line> = Vec::new();
    let bytes_per_line = 16;

    for (i, chunk) in bytes.chunks(bytes_per_line).enumerate() {
        if lines.len() >= max_lines {
            break;
        }

        // Offset
        let offset_str = format!("{:08x}  ", i * bytes_per_line);

        // Hex bytes
        let hex_str: String = chunk
            .iter()
            .map(|b| format!("{:02x} ", b))
            .collect::<String>();
        let hex_padded = format!("{:<48}", hex_str); // 16 bytes * 3 chars each

        // ASCII representation
        let ascii_str: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        let line = Line::from(vec![
            Span::styled(offset_str, theme.muted()),
            Span::styled(hex_padded, theme.normal()),
            Span::styled(" ", theme.normal()),
            Span::styled(ascii_str, theme.muted()),
        ]);

        lines.push(line);
    }

    // Show total bytes indicator
    if bytes.len() > max_lines * bytes_per_line {
        let indicator = format!("... {} total bytes", bytes.len());
        lines.push(Line::from(Span::styled(indicator, theme.muted())));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

/// Raw mode - show escape sequences and control chars literally
fn render_raw_mode(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    block: Block,
    content: &str,
) {
    // Convert control characters to visible representations
    let mut visible = String::new();
    for ch in content.chars() {
        match ch {
            '\n' => visible.push_str("\\n\n"),
            '\r' => visible.push_str("\\r"),
            '\t' => visible.push_str("\\t"),
            '\x1b' => visible.push_str("\\x1b"),
            '\0' => visible.push_str("\\0"),
            c if c.is_control() => {
                visible.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => visible.push(c),
        }
    }

    let text = Text::styled(visible, theme.normal());
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Sanitize content for safe display (strips ANSI escapes, etc.)
fn sanitize_for_display(content: &str) -> String {
    // Simple ANSI escape removal - handles common cases
    let mut result = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (end of sequence)
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if ch == '\r' || ch == '\x07' || ch == '\x08' {
            // Skip carriage return, bell, backspace
        } else if ch.is_control() && ch != '\n' && ch != '\t' {
            // Skip other control chars except newline and tab
        } else {
            result.push(ch);
        }
    }

    result
}
