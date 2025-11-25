//! Note editor modal widget

use crate::ui::theme::Theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render the note editor modal
pub fn render(frame: &mut Frame, note: &str, theme: &Theme) {
    let area = frame.area();

    // Calculate centered modal size (60% width, 7 lines height)
    let modal_width = (area.width * 60 / 100).max(40).min(80);
    let modal_height = 7;

    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

    // Clear the area behind the modal
    frame.render_widget(Clear, modal_area);

    // Build the modal content
    let block = Block::default()
        .title(" Edit Note ")
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title_style(theme.title());

    // Show the note with a cursor indicator
    let display_text = format!("{}█", note);

    let paragraph = Paragraph::new(display_text)
        .block(block)
        .style(theme.normal())
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, modal_area);

    // Render help text below the input
    let help_area = Rect::new(
        modal_x,
        modal_y + modal_height,
        modal_width,
        1,
    );

    if help_area.y < area.height {
        let help = Paragraph::new("Enter: Save  Esc: Cancel")
            .style(theme.muted())
            .alignment(Alignment::Center);
        frame.render_widget(help, help_area);
    }
}
