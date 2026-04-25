use crate::error::{DitoxError, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    pub keybindings: KeybindingsConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralConfig {
    pub max_entries: usize,
    pub poll_interval_ms: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_entries: 500,
            poll_interval_ms: 250,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[derive(Default)]
pub struct StorageConfig {
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct UiConfig {
    pub show_preview: bool,
    pub date_format: DateFormat,
    pub theme: ThemeConfig,
    pub graphics_protocol: Option<GraphicsProtocol>,
    /// Font size in pixels (width, height) for image rendering
    /// Example: [9, 18] for 9x18 pixel font
    pub font_size: Option<(u16, u16)>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum GraphicsProtocol {
    Kitty,
    Sixel,
    Iterm2,
    Halfblocks,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_preview: true,
            date_format: DateFormat::Relative,
            theme: ThemeConfig::default(),
            graphics_protocol: None, // Auto-detect
            font_size: None,         // Auto-detect
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum DateFormat {
    #[default]
    Relative,
    Iso,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ThemeConfig {
    pub selected: String,
    pub border: String,
    pub text: String,
    pub muted: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            selected: "#7aa2f7".to_string(),
            border: "#565f89".to_string(),
            text: "#c0caf5".to_string(),
            muted: "#565f89".to_string(),
        }
    }
}

/// Custom keybindings configuration
///
/// Format: "key" = "action"
/// Keys: "q", "ctrl+d", "alt+x", "shift+g", "enter", "esc", "tab", "space", "f1"-"f12"
/// Actions: see `Action::config_name()` for all available actions
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct KeybindingsConfig {
    /// Custom key bindings that override defaults
    /// Example: { "p" = "toggle_preview", "ctrl+x" = "delete" }
    #[serde(flatten)]
    pub bindings: HashMap<String, String>,
}

// Note: KeybindingsConfig::create_resolver() is implemented in ditox-tui
// since it depends on crossterm for key parsing

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)
                .map_err(|e| DitoxError::Config(format!("Failed to parse config: {}", e)))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    fn get_config_path() -> Result<PathBuf> {
        ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .ok_or_else(|| DitoxError::Config("Could not determine config directory".into()))
    }
}
