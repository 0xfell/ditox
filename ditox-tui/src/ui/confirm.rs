//! Confirmation dialog UI component

use ditox_core::app::App;
use crate::ui::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Render a confirmation dialog overlay
pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = frame.area();

    // Get the confirmation message
    let message = match app.confirm_message() {
        Some(msg) => msg,
        None => return,
    };

    // Calculate popup dimensions
    let msg_width = message.len() as u16 + 4;
    let popup_width = msg_width.max(30).min(area.width.saturating_sub(4));
    let popup_height = 5u16.min(area.height.saturating_sub(4));

    // Center the popup
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind popup
    frame.render_widget(Clear, popup_area);

    // Build the content
    let content = format!("{}\n\n[y/Enter] Confirm  [n/Esc] Cancel", message);

    let dialog = Paragraph::new(content)
        .style(theme.normal())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Confirm ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(dialog, popup_area);
}
