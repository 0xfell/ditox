//! Keybinding system for ditox
//!
//! This module handles parsing key strings from config and resolving
//! key events to actions.

use ditox_core::actions::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

/// Represents a parsed key combination
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    #[allow(dead_code)]
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Parse a key string like "ctrl+d", "alt+enter", "shift+g", "q", "/", "D"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('+').collect();

        let mut modifiers = KeyModifiers::empty();
        let key_part;

        if parts.len() == 1 {
            key_part = parts[0];
        } else {
            // Parse modifiers (case-insensitive)
            for part in &parts[..parts.len() - 1] {
                match part.to_lowercase().as_str() {
                    "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                    "alt" => modifiers |= KeyModifiers::ALT,
                    "shift" => modifiers |= KeyModifiers::SHIFT,
                    _ => return None, // Unknown modifier
                }
            }
            key_part = parts[parts.len() - 1];
        }

        // Parse the key code (preserves case for single chars)
        let code = parse_key_code(key_part)?;

        Some(KeyCombo { code, modifiers })
    }

    /// Convert to a human-readable string for display
    pub fn display(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }

        let key_str = key_code_to_string(&self.code);
        parts.push(&key_str);

        parts.join("+")
    }
}

impl From<KeyEvent> for KeyCombo {
    fn from(event: KeyEvent) -> Self {
        // Normalize: for Char keys, we ignore SHIFT modifier since the char itself
        // already represents the shifted state (e.g., 'G' vs 'g')
        let modifiers = match event.code {
            KeyCode::Char(_) => event.modifiers - KeyModifiers::SHIFT,
            _ => event.modifiers,
        };
        KeyCombo {
            code: event.code,
            modifiers,
        }
    }
}

/// Parse a key code string to KeyCode
/// Special keys are case-insensitive, single characters preserve case
fn parse_key_code(s: &str) -> Option<KeyCode> {
    // Single character - preserve case (important for 'D' vs 'd')
    if s.len() == 1 {
        let c = s.chars().next()?;
        return Some(KeyCode::Char(c));
    }

    // For multi-char keys, match case-insensitively
    let lower = s.to_lowercase();
    match lower.as_str() {
        // Special keys
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "tab" => Some(KeyCode::Tab),
        "space" => Some(KeyCode::Char(' ')),
        "backspace" | "bs" => Some(KeyCode::Backspace),
        "delete" | "del" => Some(KeyCode::Delete),
        "insert" | "ins" => Some(KeyCode::Insert),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" | "pgup" => Some(KeyCode::PageUp),
        "pagedown" | "pgdn" => Some(KeyCode::PageDown),

        // Arrow keys
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),

        // Function keys
        "f1" => Some(KeyCode::F(1)),
        "f2" => Some(KeyCode::F(2)),
        "f3" => Some(KeyCode::F(3)),
        "f4" => Some(KeyCode::F(4)),
        "f5" => Some(KeyCode::F(5)),
        "f6" => Some(KeyCode::F(6)),
        "f7" => Some(KeyCode::F(7)),
        "f8" => Some(KeyCode::F(8)),
        "f9" => Some(KeyCode::F(9)),
        "f10" => Some(KeyCode::F(10)),
        "f11" => Some(KeyCode::F(11)),
        "f12" => Some(KeyCode::F(12)),

        _ => None,
    }
}

/// Convert KeyCode to display string
fn key_code_to_string(code: &KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => "?".to_string(),
    }
}

/// Resolves key events to actions based on bindings
pub struct KeybindingResolver {
    /// Map from key combo to action
    bindings: HashMap<KeyCombo, Action>,
    /// Reverse map for help display: action -> list of keys
    reverse_bindings: HashMap<Action, Vec<KeyCombo>>,
}

impl KeybindingResolver {
    /// Create a new resolver with default bindings
    pub fn new() -> Self {
        let mut resolver = Self {
            bindings: HashMap::new(),
            reverse_bindings: HashMap::new(),
        };
        resolver.load_defaults();
        resolver
    }

    /// Load default keybindings
    fn load_defaults(&mut self) {
        // Navigation
        self.bind_default("j", Action::MoveDown);
        self.bind_default("down", Action::MoveDown);
        self.bind_default("k", Action::MoveUp);
        self.bind_default("up", Action::MoveUp);
        self.bind_default("g", Action::GoTop);
        self.bind_default("home", Action::GoTop);
        self.bind_default("G", Action::GoBottom);
        self.bind_default("end", Action::GoBottom);
        self.bind_default("ctrl+u", Action::PageUp);
        self.bind_default("pageup", Action::PageUp);
        self.bind_default("ctrl+d", Action::PageDown);
        self.bind_default("pagedown", Action::PageDown);
        self.bind_default("h", Action::PrevPage);
        self.bind_default("left", Action::PrevPage);
        self.bind_default("l", Action::NextPage);
        self.bind_default("right", Action::NextPage);

        // Operations
        self.bind_default("y", Action::Copy);
        self.bind_default("enter", Action::CopyAndQuit);
        self.bind_default("d", Action::Delete);
        self.bind_default("D", Action::ClearAll);
        self.bind_default("s", Action::ToggleFavorite);
        self.bind_default("r", Action::Refresh);

        // Modes
        self.bind_default("/", Action::EnterSearch);
        self.bind_default("esc", Action::ExitSearch);
        self.bind_default("tab", Action::TogglePreview);
        self.bind_default("t", Action::ToggleExpanded);
        self.bind_default("?", Action::ToggleHelp);

        // Multi-select
        self.bind_default("m", Action::ToggleMultiSelect);
        self.bind_default("space", Action::SelectCurrent);
        self.bind_default("v", Action::SelectAll);

        // Annotations
        self.bind_default("n", Action::EditAnnotation);

        // Search modes
        self.bind_default("ctrl+r", Action::EnterRegexSearch);
        self.bind_default("ctrl+t", Action::ToggleSearchMode);

        // System
        self.bind_default("q", Action::Quit);
        self.bind_default("ctrl+c", Action::ForceQuit);

        // Preview modes
        self.bind_default("p", Action::CyclePreviewMode);
        self.bind_default("L", Action::ToggleLineNumbers);

        // Quick snippet slots (1-9)
        self.bind_default("1", Action::QuickSlot1);
        self.bind_default("2", Action::QuickSlot2);
        self.bind_default("3", Action::QuickSlot3);
        self.bind_default("4", Action::QuickSlot4);
        self.bind_default("5", Action::QuickSlot5);
        self.bind_default("6", Action::QuickSlot6);
        self.bind_default("7", Action::QuickSlot7);
        self.bind_default("8", Action::QuickSlot8);
        self.bind_default("9", Action::QuickSlot9);

        // Tab navigation
        self.bind_default("[", Action::PrevTab);
        self.bind_default("]", Action::NextTab);

        // Future features (not bound by default, users can enable)
        // self.bind_default("a", Action::ShowActions);
        // self.bind_default("S", Action::ShowStats);
    }

    /// Bind a key combo (internal helper)
    fn bind_default(&mut self, key_str: &str, action: Action) {
        if let Some(combo) = KeyCombo::parse(key_str) {
            self.bindings.insert(combo.clone(), action);
            self.reverse_bindings
                .entry(action)
                .or_default()
                .push(combo);
        }
    }

    /// Add a custom binding (from config)
    /// This will override any existing binding for the same key
    pub fn add_binding(&mut self, key_str: &str, action: Action) -> bool {
        if let Some(combo) = KeyCombo::parse(key_str) {
            // Remove old reverse binding if this key was bound before
            if let Some(old_action) = self.bindings.get(&combo) {
                if let Some(keys) = self.reverse_bindings.get_mut(old_action) {
                    keys.retain(|k| k != &combo);
                }
            }

            self.bindings.insert(combo.clone(), action);
            self.reverse_bindings
                .entry(action)
                .or_default()
                .push(combo);
            true
        } else {
            tracing::warn!("Failed to parse keybinding: {}", key_str);
            false
        }
    }

    /// Remove a binding for a specific key
    #[allow(dead_code)]
    pub fn remove_binding(&mut self, key_str: &str) -> bool {
        if let Some(combo) = KeyCombo::parse(key_str) {
            if let Some(action) = self.bindings.remove(&combo) {
                if let Some(keys) = self.reverse_bindings.get_mut(&action) {
                    keys.retain(|k| k != &combo);
                }
                return true;
            }
        }
        false
    }

    /// Resolve a key event to an action
    pub fn resolve(&self, event: KeyEvent) -> Option<Action> {
        let combo = KeyCombo::from(event);
        self.bindings.get(&combo).copied()
    }

    /// Get all bindings for an action (for help display)
    #[allow(dead_code)]
    pub fn get_keys_for_action(&self, action: Action) -> Vec<String> {
        self.reverse_bindings
            .get(&action)
            .map(|combos| combos.iter().map(|c| c.display()).collect())
            .unwrap_or_default()
    }

    /// Get the primary (first) key for an action (for compact help display)
    pub fn get_primary_key(&self, action: Action) -> Option<String> {
        self.reverse_bindings
            .get(&action)
            .and_then(|combos| combos.first())
            .map(|c| c.display())
    }

    /// Check if any key is bound to an action
    pub fn has_binding(&self, action: Action) -> bool {
        self.reverse_bindings
            .get(&action)
            .map(|keys| !keys.is_empty())
            .unwrap_or(false)
    }

    /// Validate bindings - check for conflicts
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for actions without any bindings
        let important_actions = [
            Action::MoveUp,
            Action::MoveDown,
            Action::Copy,
            Action::CopyAndQuit,
            Action::Delete,
            Action::EnterSearch,
            Action::Quit,
        ];

        for action in important_actions {
            if !self.has_binding(action) {
                warnings.push(format!(
                    "Action '{}' has no keybinding",
                    action.config_name()
                ));
            }
        }

        warnings
    }
}

impl Default for KeybindingResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait to create KeybindingResolver from config
pub trait KeybindingsConfigExt {
    fn create_resolver(&self) -> KeybindingResolver;
}

impl KeybindingsConfigExt for ditox_core::config::KeybindingsConfig {
    /// Create a KeybindingResolver with custom bindings applied
    fn create_resolver(&self) -> KeybindingResolver {
        let mut resolver = KeybindingResolver::new();

        // Apply custom bindings
        for (key, action_name) in &self.bindings {
            if let Some(action) = Action::from_config_name(action_name) {
                if !resolver.add_binding(key, action) {
                    tracing::warn!(
                        "Invalid keybinding in config: '{}' = '{}'",
                        key,
                        action_name
                    );
                }
            } else {
                tracing::warn!("Unknown action in config: '{}'", action_name);
            }
        }

        // Validate and warn about missing important bindings
        for warning in resolver.validate() {
            tracing::warn!("{}", warning);
        }

        resolver
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_keys() {
        assert_eq!(
            KeyCombo::parse("q"),
            Some(KeyCombo::new(KeyCode::Char('q'), KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("/"),
            Some(KeyCombo::new(KeyCode::Char('/'), KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("?"),
            Some(KeyCombo::new(KeyCode::Char('?'), KeyModifiers::empty()))
        );
    }

    #[test]
    fn test_parse_uppercase_keys() {
        // Uppercase letters should be distinct from lowercase
        assert_eq!(
            KeyCombo::parse("D"),
            Some(KeyCombo::new(KeyCode::Char('D'), KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("G"),
            Some(KeyCombo::new(KeyCode::Char('G'), KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("L"),
            Some(KeyCombo::new(KeyCode::Char('L'), KeyModifiers::empty()))
        );
        // Verify 'd' and 'D' are different
        assert_ne!(
            KeyCombo::parse("d"),
            KeyCombo::parse("D")
        );
    }

    #[test]
    fn test_parse_special_keys() {
        assert_eq!(
            KeyCombo::parse("enter"),
            Some(KeyCombo::new(KeyCode::Enter, KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("esc"),
            Some(KeyCombo::new(KeyCode::Esc, KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("tab"),
            Some(KeyCombo::new(KeyCode::Tab, KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("space"),
            Some(KeyCombo::new(KeyCode::Char(' '), KeyModifiers::empty()))
        );
    }

    #[test]
    fn test_parse_modified_keys() {
        assert_eq!(
            KeyCombo::parse("ctrl+c"),
            Some(KeyCombo::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            KeyCombo::parse("ctrl+d"),
            Some(KeyCombo::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            KeyCombo::parse("alt+x"),
            Some(KeyCombo::new(KeyCode::Char('x'), KeyModifiers::ALT))
        );
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(
            KeyCombo::parse("up"),
            Some(KeyCombo::new(KeyCode::Up, KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("down"),
            Some(KeyCombo::new(KeyCode::Down, KeyModifiers::empty()))
        );
    }

    #[test]
    fn test_parse_function_keys() {
        assert_eq!(
            KeyCombo::parse("f1"),
            Some(KeyCombo::new(KeyCode::F(1), KeyModifiers::empty()))
        );
        assert_eq!(
            KeyCombo::parse("f12"),
            Some(KeyCombo::new(KeyCode::F(12), KeyModifiers::empty()))
        );
    }

    #[test]
    fn test_resolver_default_bindings() {
        let resolver = KeybindingResolver::new();

        // Test that default bindings exist
        let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(resolver.resolve(event), Some(Action::Quit));

        let event = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        assert_eq!(resolver.resolve(event), Some(Action::MoveDown));

        let event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(resolver.resolve(event), Some(Action::PageDown));
    }

    #[test]
    fn test_custom_binding() {
        let mut resolver = KeybindingResolver::new();

        // Override q to be delete instead of quit
        resolver.add_binding("q", Action::Delete);

        let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(resolver.resolve(event), Some(Action::Delete));
    }

    #[test]
    fn test_display_key_combo() {
        let combo = KeyCombo::parse("ctrl+d").unwrap();
        assert_eq!(combo.display(), "Ctrl+d");

        let combo = KeyCombo::parse("q").unwrap();
        assert_eq!(combo.display(), "q");
    }
}
