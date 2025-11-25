use ditox_core::actions::Action;
use crate::keybindings::KeybindingResolver;
use crate::ui::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn draw(frame: &mut Frame, theme: &Theme, keybindings: &KeybindingResolver) {
    let area = frame.area();

    // Center the help popup
    let popup_width = 56.min(area.width.saturating_sub(4));
    let popup_height = 28.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Generate dynamic help from keybindings
    let help_text = generate_help(keybindings);

    // Clear the area behind popup
    frame.render_widget(Clear, popup_area);

    let help = Paragraph::new(help_text).style(theme.normal()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.accent())
            .title(" Help ")
            .title_style(theme.title()),
    );

    frame.render_widget(help, popup_area);
}

/// Get the primary key for an action, or "N/A" if unbound
fn key_for(keybindings: &KeybindingResolver, action: Action) -> String {
    keybindings
        .get_primary_key(action)
        .unwrap_or_else(|| "N/A".to_string())
}

/// Generate help text dynamically from keybindings
fn generate_help(keybindings: &KeybindingResolver) -> String {
    format!(
        r#"
  Navigation
  ──────────
  {:>10}  Down       {:>10}  Top
  {:>10}  Up         {:>10}  Bottom
  {:>10}  Prev page  {:>10}  Next page
  {:>10}  Prev/Next tab

  Actions
  ───────
  {:>10}  Copy & exit   {:>10}  Delete
  {:>10}  Copy          {:>10}  Clear all
  {:>10}  Toggle fav    {:>10}  Edit note

  Search
  ──────
  {:>10}  Start search  {:>10}  Regex mode
  {:>10}  Toggle mode   {:>10}  Clear/exit

  Multi-select
  ────────────
  {:>10}  Toggle mode   {:>10}  Select
  {:>10}  Select all

  View
  ────
  {:>10}  Expand        {:>10}  Preview
  {:>10}  Preview mode  {:>10}  Line numbers
  {:>10}  Help          {:>10}  Quit
"#,
        // Navigation
        key_for(keybindings, Action::MoveDown),
        key_for(keybindings, Action::GoTop),
        key_for(keybindings, Action::MoveUp),
        key_for(keybindings, Action::GoBottom),
        key_for(keybindings, Action::PrevPage),
        key_for(keybindings, Action::NextPage),
        format!("{}/{}",
            key_for(keybindings, Action::PrevTab),
            key_for(keybindings, Action::NextTab)
        ),
        // Actions
        key_for(keybindings, Action::CopyAndQuit),
        key_for(keybindings, Action::Delete),
        key_for(keybindings, Action::Copy),
        key_for(keybindings, Action::ClearAll),
        key_for(keybindings, Action::ToggleFavorite),
        key_for(keybindings, Action::EditAnnotation),
        // Search
        key_for(keybindings, Action::EnterSearch),
        key_for(keybindings, Action::EnterRegexSearch),
        key_for(keybindings, Action::ToggleSearchMode),
        key_for(keybindings, Action::ExitSearch),
        // Multi-select
        key_for(keybindings, Action::ToggleMultiSelect),
        key_for(keybindings, Action::SelectCurrent),
        key_for(keybindings, Action::SelectAll),
        // View
        key_for(keybindings, Action::ToggleExpanded),
        key_for(keybindings, Action::TogglePreview),
        key_for(keybindings, Action::CyclePreviewMode),
        key_for(keybindings, Action::ToggleLineNumbers),
        key_for(keybindings, Action::ToggleHelp),
        key_for(keybindings, Action::Quit),
    )
}
