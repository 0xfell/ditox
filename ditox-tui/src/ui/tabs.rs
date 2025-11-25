//! Tab bar widget for filtering entries
//!
//! Shows a horizontal tab bar with filter options.

use ditox_core::app::App;
use crate::ui::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Draw the tab bar
pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if !app.show_tabs || area.width < 30 {
        return;
    }

    let mut spans: Vec<Span> = Vec::new();

    for (i, tab) in app.tabs.iter().enumerate() {
        let label = tab.label();
        let is_active = i == app.active_tab;

        // Build tab display: [Label] or [*Label*] for active
        let tab_text = if is_active {
            format!("[{}]", label)
        } else {
            format!(" {} ", label)
        };

        let style = if is_active {
            theme.selected()
        } else {
            theme.normal()
        };

        spans.push(Span::styled(tab_text, style));

        // Add space between tabs
        if i < app.tabs.len() - 1 {
            spans.push(Span::styled(" ", theme.normal()));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Calculate the width needed for tab bar
#[allow(dead_code)]
pub fn calculate_width(app: &App) -> u16 {
    if !app.show_tabs {
        return 0;
    }

    // Calculate total width of all tabs
    let mut width: u16 = 0;
    for (i, tab) in app.tabs.iter().enumerate() {
        let label = tab.label();
        // Each tab: [Label] = len + 2 brackets
        width += label.len() as u16 + 2;
        // Add space between tabs
        if i < app.tabs.len() - 1 {
            width += 1;
        }
    }

    width
}
