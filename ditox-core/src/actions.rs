//! Action system for ditox keybindings
//!
//! This module defines all possible actions that can be triggered by keybindings.
//! Actions are decoupled from specific key combinations, allowing user customization.

use serde::{Deserialize, Serialize};

/// All possible actions in ditox TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    GoTop,
    GoBottom,
    PageUp,
    PageDown,
    PrevPage,
    NextPage,

    // Operations
    Copy,
    CopyAndQuit,
    Delete,
    ClearAll,
    ToggleFavorite,
    Refresh,

    // Modes
    EnterSearch,
    ExitSearch,
    TogglePreview,
    ToggleExpanded,
    ToggleHelp,

    // Multi-select
    ToggleMultiSelect,
    SelectCurrent,
    SelectAll,

    // Future features (Phase 2-4)
    // Regex Search (#10)
    EnterRegexSearch,
    ToggleSearchMode,

    // Content Detection (#3)
    ShowActions,

    // Preview Modes (#18)
    CyclePreviewMode,
    ToggleLineNumbers,

    // Tabs (#20)
    NextTab,
    PrevTab,

    // Quick Snippets (#9)
    QuickSlot1,
    QuickSlot2,
    QuickSlot3,
    QuickSlot4,
    QuickSlot5,
    QuickSlot6,
    QuickSlot7,
    QuickSlot8,
    QuickSlot9,

    // Annotations (#13)
    EditAnnotation,

    // Statistics (#8)
    ShowStats,

    // System
    Quit,
    ForceQuit,
}

impl Action {
    /// Get a human-readable description of the action
    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            // Navigation
            Action::MoveUp => "Move up",
            Action::MoveDown => "Move down",
            Action::GoTop => "Go to top",
            Action::GoBottom => "Go to bottom",
            Action::PageUp => "Page up",
            Action::PageDown => "Page down",
            Action::PrevPage => "Previous page",
            Action::NextPage => "Next page",

            // Operations
            Action::Copy => "Copy to clipboard",
            Action::CopyAndQuit => "Copy and quit",
            Action::Delete => "Delete entry",
            Action::ClearAll => "Clear all entries",
            Action::ToggleFavorite => "Toggle favorite status",
            Action::Refresh => "Refresh entries",

            // Modes
            Action::EnterSearch => "Start search",
            Action::ExitSearch => "Exit search",
            Action::TogglePreview => "Toggle preview pane",
            Action::ToggleExpanded => "Toggle expanded preview",
            Action::ToggleHelp => "Toggle help",

            // Multi-select
            Action::ToggleMultiSelect => "Toggle multi-select mode",
            Action::SelectCurrent => "Select/deselect current",
            Action::SelectAll => "Select/deselect all",

            // Future features
            Action::EnterRegexSearch => "Start regex search",
            Action::ToggleSearchMode => "Toggle search mode (fuzzy/regex)",
            Action::ShowActions => "Show contextual actions",
            Action::CyclePreviewMode => "Cycle preview mode",
            Action::ToggleLineNumbers => "Toggle line numbers",
            Action::NextTab => "Next tab",
            Action::PrevTab => "Previous tab",
            Action::QuickSlot1 => "Quick slot 1",
            Action::QuickSlot2 => "Quick slot 2",
            Action::QuickSlot3 => "Quick slot 3",
            Action::QuickSlot4 => "Quick slot 4",
            Action::QuickSlot5 => "Quick slot 5",
            Action::QuickSlot6 => "Quick slot 6",
            Action::QuickSlot7 => "Quick slot 7",
            Action::QuickSlot8 => "Quick slot 8",
            Action::QuickSlot9 => "Quick slot 9",
            Action::EditAnnotation => "Edit annotation",
            Action::ShowStats => "Show statistics",

            // System
            Action::Quit => "Quit",
            Action::ForceQuit => "Force quit",
        }
    }

    /// Get the action name as it would appear in config
    pub fn config_name(&self) -> &'static str {
        match self {
            Action::MoveUp => "move_up",
            Action::MoveDown => "move_down",
            Action::GoTop => "go_top",
            Action::GoBottom => "go_bottom",
            Action::PageUp => "page_up",
            Action::PageDown => "page_down",
            Action::PrevPage => "prev_page",
            Action::NextPage => "next_page",
            Action::Copy => "copy",
            Action::CopyAndQuit => "copy_and_quit",
            Action::Delete => "delete",
            Action::ClearAll => "clear_all",
            Action::ToggleFavorite => "toggle_favorite",
            Action::Refresh => "refresh",
            Action::EnterSearch => "enter_search",
            Action::ExitSearch => "exit_search",
            Action::TogglePreview => "toggle_preview",
            Action::ToggleExpanded => "toggle_expanded",
            Action::ToggleHelp => "toggle_help",
            Action::ToggleMultiSelect => "toggle_multi_select",
            Action::SelectCurrent => "select_current",
            Action::SelectAll => "select_all",
            Action::EnterRegexSearch => "enter_regex_search",
            Action::ToggleSearchMode => "toggle_search_mode",
            Action::ShowActions => "show_actions",
            Action::CyclePreviewMode => "cycle_preview_mode",
            Action::ToggleLineNumbers => "toggle_line_numbers",
            Action::NextTab => "next_tab",
            Action::PrevTab => "prev_tab",
            Action::QuickSlot1 => "quick_slot_1",
            Action::QuickSlot2 => "quick_slot_2",
            Action::QuickSlot3 => "quick_slot_3",
            Action::QuickSlot4 => "quick_slot_4",
            Action::QuickSlot5 => "quick_slot_5",
            Action::QuickSlot6 => "quick_slot_6",
            Action::QuickSlot7 => "quick_slot_7",
            Action::QuickSlot8 => "quick_slot_8",
            Action::QuickSlot9 => "quick_slot_9",
            Action::EditAnnotation => "edit_annotation",
            Action::ShowStats => "show_stats",
            Action::Quit => "quit",
            Action::ForceQuit => "force_quit",
        }
    }

    /// Parse action from config string
    pub fn from_config_name(name: &str) -> Option<Action> {
        match name {
            "move_up" => Some(Action::MoveUp),
            "move_down" => Some(Action::MoveDown),
            "go_top" => Some(Action::GoTop),
            "go_bottom" => Some(Action::GoBottom),
            "page_up" => Some(Action::PageUp),
            "page_down" => Some(Action::PageDown),
            "prev_page" => Some(Action::PrevPage),
            "next_page" => Some(Action::NextPage),
            "copy" => Some(Action::Copy),
            "copy_and_quit" => Some(Action::CopyAndQuit),
            "delete" => Some(Action::Delete),
            "clear_all" => Some(Action::ClearAll),
            "toggle_favorite" | "toggle_pin" => Some(Action::ToggleFavorite), // Support legacy "toggle_pin"
            "refresh" => Some(Action::Refresh),
            "enter_search" => Some(Action::EnterSearch),
            "exit_search" => Some(Action::ExitSearch),
            "toggle_preview" => Some(Action::TogglePreview),
            "toggle_expanded" => Some(Action::ToggleExpanded),
            "toggle_help" => Some(Action::ToggleHelp),
            "toggle_multi_select" => Some(Action::ToggleMultiSelect),
            "select_current" => Some(Action::SelectCurrent),
            "select_all" => Some(Action::SelectAll),
            "enter_regex_search" => Some(Action::EnterRegexSearch),
            "toggle_search_mode" => Some(Action::ToggleSearchMode),
            "show_actions" => Some(Action::ShowActions),
            "cycle_preview_mode" => Some(Action::CyclePreviewMode),
            "toggle_line_numbers" => Some(Action::ToggleLineNumbers),
            "next_tab" => Some(Action::NextTab),
            "prev_tab" => Some(Action::PrevTab),
            "quick_slot_1" => Some(Action::QuickSlot1),
            "quick_slot_2" => Some(Action::QuickSlot2),
            "quick_slot_3" => Some(Action::QuickSlot3),
            "quick_slot_4" => Some(Action::QuickSlot4),
            "quick_slot_5" => Some(Action::QuickSlot5),
            "quick_slot_6" => Some(Action::QuickSlot6),
            "quick_slot_7" => Some(Action::QuickSlot7),
            "quick_slot_8" => Some(Action::QuickSlot8),
            "quick_slot_9" => Some(Action::QuickSlot9),
            "edit_annotation" => Some(Action::EditAnnotation),
            "show_stats" => Some(Action::ShowStats),
            "quit" => Some(Action::Quit),
            "force_quit" => Some(Action::ForceQuit),
            _ => None,
        }
    }
}
