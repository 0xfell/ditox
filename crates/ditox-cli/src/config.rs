use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub storage: Storage,
    pub prune: Option<Prune>,
    pub max_storage_mb: Option<u64>,
    pub sync: Option<Sync>,
    pub images: Option<Images>,
    pub tui: Option<Tui>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "lowercase")]
pub enum Storage {
    LocalSqlite {
        db_path: Option<PathBuf>,
    },
    Turso {
        url: String,
        auth_token: Option<String>,
    },
}

impl Default for Storage {
    fn default() -> Self {
        Storage::LocalSqlite { db_path: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Prune {
    pub every: Option<String>,
    pub keep_favorites: Option<bool>,
    pub max_items: Option<usize>,
    pub max_age: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sync {
    pub enabled: Option<bool>,
    pub interval: Option<String>,
    pub batch_size: Option<usize>,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Images {
    pub local_file_path_mode: Option<bool>,
    pub dir: Option<String>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tui {
    /// Page size (items per page); defaults to 10 when unset
    pub page_size: Option<usize>,
    /// Auto-apply tag (text after '#') after N ms of idle typing in query mode
    pub auto_apply_tag_ms: Option<u64>,
    /// Show absolute timestamps in the picker (Created at • Last used)
    pub absolute_times: Option<bool>,
    /// Preferred theme name (or absolute path to a theme file)
    pub theme: Option<String>,
    /// Color mode: auto|always|never
    pub color: Option<String>,
    /// Box drawing characters: unicode|ascii (ascii disables borders)
    pub box_chars: Option<String>,
    /// Whether to use the alternate screen
    pub alt_screen: Option<bool>,
    /// Enable file watching for theme reloads (not implemented; reserved)
    pub live_reload: Option<bool>,
    /// Date format for auto/absolute displays (tokens: dd, mm, yyyy). Example: "dd-mm-yyyy"
    pub date_format: Option<String>,
    /// Threshold in days for auto time to switch from relative to absolute date
    pub auto_recent_days: Option<u32>,
    /// Glyph pack name or file path
    pub glyphs: Option<String>,
    /// Layout pack name or file path
    pub layout: Option<String>,
    /// Debounced auto-refresh interval in milliseconds (default: 1500)
    pub refresh_ms: Option<u64>,
    /// Play a short sound when new items arrive (default: false)
    pub sound_on_new: Option<bool>,
}

pub fn images_dir(settings: &Settings) -> std::path::PathBuf {
    use std::path::PathBuf;
    let base = config_dir();
    if let Some(img) = &settings.images {
        if let Some(dir) = &img.dir {
            if !dir.trim().is_empty() {
                let p = shellexpand::tilde(dir).to_string();
                return PathBuf::from(p);
            }
        }
    }
    base.join("data").join("imgs")
}

pub fn config_dir() -> PathBuf {
    if let Some(bd) = directories::BaseDirs::new() {
        bd.config_dir().join("ditox")
    } else {
        PathBuf::from("./.config/ditox")
    }
}

pub fn state_dir() -> PathBuf {
    // Prefer XDG state dir when available; fall back to config dir
    if let Some(bd) = directories::BaseDirs::new() {
        if let Some(sd) = bd.state_dir() {
            return sd.join("ditox");
        }
    }
    config_dir()
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    if let Ok(s) = std::fs::read_to_string(&path) {
        toml::from_str(&s).unwrap_or_default()
    } else {
        Settings::default()
    }
}
