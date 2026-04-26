//! Tests for `Config.storage.data_dir` override and path expansion.
//! Linux-only (uses tilde and `$VAR` semantics; tested via `HOME`).

#![cfg(unix)]

use ditox_core::config::{expand_path, Config};
use ditox_core::db::{data_dir_override, set_data_dir_override, Database};
use ditox_core::Entry;
use std::sync::Mutex;
use tempfile::TempDir;

// Tests mutate the process-wide DATA_DIR_OVERRIDE; serialize them.
static OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

fn reset_override() {
    let _ = set_data_dir_override(None);
}

#[test]
fn expand_tilde_uses_home_env() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let prev = std::env::var_os("HOME");
    std::env::set_var("HOME", "/tmp/fakehome");

    assert_eq!(expand_path("~"), std::path::PathBuf::from("/tmp/fakehome"));
    assert_eq!(
        expand_path("~/ditox"),
        std::path::PathBuf::from("/tmp/fakehome/ditox")
    );
    // ~user not expanded
    assert_eq!(
        expand_path("~bob/ditox"),
        std::path::PathBuf::from("~bob/ditox")
    );

    if let Some(prev) = prev {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn expand_dollar_var() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    std::env::set_var("DITOX_TEST_VAR", "/some/path");

    assert_eq!(
        expand_path("$DITOX_TEST_VAR/sub"),
        std::path::PathBuf::from("/some/path/sub")
    );
    assert_eq!(
        expand_path("${DITOX_TEST_VAR}/sub"),
        std::path::PathBuf::from("/some/path/sub")
    );

    std::env::remove_var("DITOX_TEST_VAR");
}

#[test]
fn expand_unknown_var_is_literal() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    std::env::remove_var("DEFINITELY_NOT_A_REAL_VAR_777");
    let p = expand_path("$DEFINITELY_NOT_A_REAL_VAR_777/foo");
    let s = p.to_string_lossy();
    assert!(s.contains("DEFINITELY_NOT_A_REAL_VAR_777"));
}

#[test]
fn data_dir_override_redirects_db_open() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("alt-data");

    set_data_dir_override(Some(custom.clone())).unwrap();
    assert_eq!(data_dir_override().as_deref(), Some(custom.as_path()));

    let db = Database::open().expect("open with override");
    db.init_schema().expect("schema init");

    let entry = Entry::new_text("hello-from-override".to_string());
    db.insert(&entry).expect("insert");

    let count = db.count().expect("count");
    assert_eq!(count, 1);

    // The DB file must have landed inside the override dir.
    assert!(custom.join("ditox.db").exists());

    reset_override();
}

#[test]
fn override_creates_missing_directory() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("does/not/yet/exist");
    assert!(!custom.exists());

    set_data_dir_override(Some(custom.clone())).unwrap();
    assert!(custom.exists());

    reset_override();
}

#[test]
fn config_storage_resolves_tilde() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let prev = std::env::var_os("HOME");
    std::env::set_var("HOME", "/tmp/fakehome2");

    let cfg: Config = toml::from_str("[storage]\ndata_dir = \"~/ditox-alt\"").expect("parse");
    let resolved = cfg.storage.resolved_data_dir().expect("some");
    assert_eq!(
        resolved,
        std::path::PathBuf::from("/tmp/fakehome2/ditox-alt")
    );

    if let Some(prev) = prev {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn legacy_db_warning_helper() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    // Empty candidate dir, no DB
    let candidate = tmp.path().join("new-loc");
    std::fs::create_dir_all(&candidate).unwrap();
    // Function should not crash; result depends on host's actual default data dir.
    let _ = Config::legacy_db_exists_outside(&candidate);
}
