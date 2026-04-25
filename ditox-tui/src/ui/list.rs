use crate::ui::theme::Theme;
use ditox_core::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use std::collections::HashSet;

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    // Calculate base index for global entry numbering
    let base_index = if app.search_query.is_empty() {
        app.current_page * 20 // PAGE_SIZE
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let entry = &app.entries[idx];
            let match_indices = app.match_indices.get(&idx);
            let is_multi_selected = app.is_multi_selected(i);
            format_entry_row(
                entry,
                base_index + i + 1, // Global entry number
                i == app.selected,
                is_multi_selected,
                app.multi_select_mode,
                theme,
                area.width,
                match_indices,
            )
        })
        .collect();

    // Build title with page indicator
    let title = if app.search_query.is_empty() {
        format!(
            " History  [Page {} of {}] ",
            app.display_page(),
            app.total_pages()
        )
    } else {
        format!(" Search Results ({}) ", app.filtered.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(title)
                .title_style(theme.title()),
        )
        .highlight_style(theme.selected());

    let mut state = ListState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut state);

    // Render scrollbar showing position across ALL entries (not just current page)
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    if app.total_count > visible_height {
        // Calculate global position: current page offset + selection within page
        let global_position = base_index + app.selected;

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(app.total_count)
            .viewport_content_length(visible_height)
            .position(global_position);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        // Render scrollbar in the border area
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

// Each argument threads through rendering; grouping into a struct would
// hide the dependencies without reducing coupling.
#[allow(clippy::too_many_arguments)]
fn format_entry_row(
    entry: &ditox_core::entry::Entry,
    index: usize,
    selected: bool,
    is_multi_selected: bool,
    multi_select_mode: bool,
    theme: &Theme,
    width: u16,
    match_indices: Option<&Vec<u32>>,
) -> ListItem<'static> {
    // Entry type icon (T/I)
    let type_str = entry.entry_type.icon();
    let time = entry.relative_time();

    // Show checkbox in multi-select mode, pin marker otherwise
    let marker = if multi_select_mode {
        if is_multi_selected {
            "[✓]"
        } else {
            "[ ]"
        }
    } else if entry.favorite {
        " ★ "
    } else {
        "   "
    };

    // Calculate available width for content preview
    // Format: " {marker} {idx} │ {type} │ {content...} │ {time} "
    let fixed_width = 5 + 4 + 3 + 2 + 3 + 6 + 2; // marker + idx + sep + type(1) + sep + time + padding
    let content_width = (width as usize).saturating_sub(fixed_width).max(10);

    let base_style = if selected {
        theme.selected()
    } else {
        theme.normal()
    };

    let highlight_style = if selected {
        theme.highlight_selected()
    } else {
        theme.highlight()
    };

    // Build prefix spans
    let prefix = format!("{} {:>3} │ {} │ ", marker, index, type_str);

    // Build content with highlighting
    let preview = entry.preview(content_width);
    let padded_preview = format!("{:<width$}", preview, width = content_width);

    let content_spans = if let Some(indices) = match_indices {
        // Create a set of match indices for O(1) lookup
        let match_set: HashSet<u32> = indices.iter().copied().collect();

        // Build spans with highlighting
        let mut spans: Vec<Span> = Vec::new();
        let mut current_str = String::new();
        let mut in_highlight = false;

        for (i, ch) in padded_preview.chars().enumerate() {
            let is_match = match_set.contains(&(i as u32));

            if is_match != in_highlight {
                // Flush current segment
                if !current_str.is_empty() {
                    let style = if in_highlight {
                        highlight_style
                    } else {
                        base_style
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
                base_style
            };
            spans.push(Span::styled(current_str, style));
        }

        spans
    } else {
        vec![Span::styled(padded_preview, base_style)]
    };

    // Build suffix
    let suffix = format!(" │ {:>4}", time);

    // Combine all spans into a line
    let mut all_spans = vec![Span::styled(prefix, base_style)];
    all_spans.extend(content_spans);
    all_spans.push(Span::styled(suffix, base_style));

    ListItem::new(Line::from(all_spans))
}
