//! Quick snippets bar widget
//!
//! Shows a horizontal bar with 1-9 quick access slots populated
//! from the most-used clipboard entries.

#![allow(dead_code)]

use ditox_core::app::App;
use ditox_core::db::Database;
use crate::ui::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Draw the snippets bar
pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, db: &Database) {
    if !app.show_snippets || area.width < 20 {
        return;
    }

    // Build snippet slots display
    let mut spans: Vec<Span> = Vec::new();

    // Get top entries directly from DB for display
    let top_entries = db.get_top_by_usage(9).unwrap_or_default();

    for (i, slot_id) in app.snippet_slots.iter().enumerate() {
        let slot_num = i + 1;

        // Find the entry for this slot
        let preview = if let Some(entry_id) = slot_id {
            // Try to find entry in top_entries
            top_entries
                .iter()
                .find(|e| &e.id == entry_id)
                .map(|e| e.preview(8))
                .unwrap_or_else(|| "...".to_string())
        } else {
            "---".to_string()
        };

        // Format: "1:preview  "
        let slot_text = format!("{}:{}", slot_num, preview);

        // Style based on whether slot has content
        let style = if slot_id.is_some() {
            theme.normal()
        } else {
            theme.muted()
        };

        spans.push(Span::styled(slot_text, style));

        // Add separator except for last slot
        if i < 8 {
            spans.push(Span::styled(" ", theme.muted()));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).alignment(Alignment::Right);
    frame.render_widget(paragraph, area);
}

/// Calculate the width needed for snippets bar
pub fn calculate_width(app: &App) -> u16 {
    if !app.show_snippets {
        return 0;
    }

    // Each slot: "N:PREVIEW " = 1 + 1 + 8 + 1 = 11 chars
    // 9 slots = 99 chars, but we can truncate
    // Minimum usable: show at least 3 slots
    30 // Show first 3 slots by default
}

/// Get the number of snippet slots that can fit in given width
pub fn slots_that_fit(width: u16) -> usize {
    // Each slot takes about 11 chars
    let per_slot = 11;
    (width as usize / per_slot).min(9)
}
