//! CLI integration tests for tag commands.

#![cfg(unix)]

use assert_cmd::Command;
use ditox_core::db::Database;
use ditox_core::Entry;
use predicates::prelude::*;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("XDG_DATA_HOME", dir.path());
    }
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    (dir, lock)
}

fn ditox(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("ditox").unwrap();
    cmd.env("XDG_DATA_HOME", dir.path());
    cmd
}

#[test]
fn cli_tag_add_list_remove_round_trip() {
    let (dir, _lock) = setup();
    let entry = Entry::new_text("Tagged content".to_string());
    Database::open().unwrap().insert(&entry).unwrap();

    ditox(&dir)
        .args(["tag", "1", "work", "--color", "#ff5500"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tagged content"))
        .stdout(predicate::str::contains("work"));

    ditox(&dir)
        .args(["tag-list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"))
        .stdout(predicate::str::contains("#ff5500"));

    ditox(&dir)
        .args(["tag-list", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"work\""));

    ditox(&dir)
        .args(["untag", "1", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed tag 'work'"));

    ditox(&dir)
        .args(["tag-list", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tags found"));
}
