use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub storage: Storage,
    pub prune: Option<Prune>,
    pub max_storage_mb: Option<u64>,
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

pub fn config_dir() -> PathBuf {
    if let Some(bd) = directories::BaseDirs::new() {
        bd.config_dir().join("ditox")
    } else {
        PathBuf::from("./.config/ditox")
    }
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
