use crate::ui::theme::Theme;
use ditox_core::app::{App, InputMode, SearchMode};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let search_style = match app.input_mode {
        InputMode::Normal | InputMode::EditNote | InputMode::Confirm => theme.normal(),
        InputMode::Search => theme.accent(),
    };

    let cursor_char = match app.input_mode {
        InputMode::Normal | InputMode::EditNote | InputMode::Confirm => "",
        InputMode::Search => "█",
    };

    // Build search mode indicator
    let mode_indicator = match app.search_mode {
        SearchMode::Fuzzy => "",
        SearchMode::Regex => "[regex] ",
    };

    // Show match count or regex error when there's a search query
    let match_info = if let Some(err) = &app.regex_error {
        // Show truncated regex error
        let short_err = if err.len() > 30 {
            format!(" ({}...)", &err[..30])
        } else {
            format!(" ({})", err)
        };
        short_err
    } else if !app.search_query.is_empty() {
        let count = app.filtered.len();
        if count == 1 {
            " (1 match)".to_string()
        } else {
            format!(" ({} matches)", count)
        }
    } else {
        String::new()
    };

    let search_text = format!(
        " {}Search: {}{}{}",
        mode_indicator, app.search_query, cursor_char, match_info
    );

    let title = match app.input_mode {
        InputMode::Normal => " Ditox ",
        InputMode::Search => match app.search_mode {
            SearchMode::Fuzzy => " Ditox (search) ",
            SearchMode::Regex => " Ditox (regex search) ",
        },
        InputMode::EditNote => " Ditox (editing note) ",
        InputMode::Confirm => " Ditox (confirm) ",
    };

    let search_bar = Paragraph::new(search_text).style(search_style).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(title)
            .title_style(theme.title()),
    );

    frame.render_widget(search_bar, area);

    // Set cursor position when in search mode (with bounds checking)
    if app.input_mode == InputMode::Search {
        // Calculate cursor X position: border(1) + " " + mode_indicator + "Search: " + query length
        let mode_indicator_len = match app.search_mode {
            SearchMode::Fuzzy => 0,
            SearchMode::Regex => 8, // "[regex] "
        };
        // border(1) + " "(1) + mode_indicator + "Search: "(8) = 10 + mode_indicator
        let cursor_offset = (10 + mode_indicator_len) as u16 + app.search_query.len() as u16;
        let x = area.x.saturating_add(cursor_offset);
        let y = area.y.saturating_add(1);

        // Only set cursor if within bounds (prevent crash on small terminals or Ghostty)
        let max_x = area.x.saturating_add(area.width.saturating_sub(1));
        let max_y = area.y.saturating_add(area.height.saturating_sub(1));
        if x <= max_x && y <= max_y {
            frame.set_cursor_position(Position::new(x, y));
        }
    }
}
