use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub storage: Storage,
    pub prune: Option<Prune>,
    pub max_storage_mb: Option<u64>,
    pub sync: Option<Sync>,
    pub images: Option<Images>,
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
