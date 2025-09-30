mod common;
use common::TestEnv;
use predicates::prelude::*;

#[test]
fn migrate_status_and_backup() {
    let t = TestEnv::new();

    // Status on fresh DB should create DB and show pending > 0
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .args(["migrate", "--status"]) // read-only path
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("current:"));
    assert!(s.contains("latest:"));
    // Apply with backup
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["migrate", "--backup"]) // apply + backup
        .assert()
        .success()
        .stdout(predicate::str::contains("migrated to version "));

    // Backup file should exist
    let parent = t.db.parent().unwrap();
    let backups: Vec<_> = std::fs::read_dir(parent)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(".bak."))
                .unwrap_or(false)
        })
        .collect();
    assert!(!backups.is_empty(), "expected a .bak.<timestamp> file");
}
