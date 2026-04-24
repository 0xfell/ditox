//! Integration tests for the `ditox repair` command and the content-
//! addressed image store (delete/clear cleanup).
//!
//! These run the real `ditox` binary against a scratch `XDG_DATA_HOME`,
//! seeding rows/files directly via `ditox-core` so we don't need a
//! live clipboard.

use assert_cmd::Command;
use ditox_core::db::Database;
use ditox_core::entry::Entry;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the env lock, set up a scratch XDG_DATA_HOME, and return a
/// pre-initialised scratch + guard so the test owns the env exclusively.
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

/// Helper: produce deterministic bytes with a given discriminator.
fn png_bytes(byte: u8) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    v.extend_from_slice(b"IHDR");
    v.push(byte);
    v.extend_from_slice(&[0u8; 16]);
    v
}

fn insert_image_via_core(bytes: &[u8]) -> (Entry, PathBuf) {
    let hash = Entry::compute_hash(bytes);
    let ext = "png".to_string();
    let (path, _) = Database::store_image_blob(&hash, &ext, bytes).unwrap();
    let entry = Entry::new_image(hash, bytes.len(), ext);
    let db = Database::open().unwrap();
    db.insert(&entry).unwrap();
    (entry, path)
}

fn count_files(images_dir: &Path) -> usize {
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        if !p.exists() {
            return;
        }
        for e in std::fs::read_dir(p).unwrap().flatten() {
            let path = e.path();
            if path.to_string_lossy().contains(".quarantine") {
                continue;
            }
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().map(|x| x != "tmp").unwrap_or(true) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(images_dir, &mut out);
    out.len()
}

fn ditox(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("ditox").unwrap();
    cmd.env("XDG_DATA_HOME", dir.path());
    cmd
}

#[test]
fn cli_delete_removes_blob() {
    let (dir, _lock) = setup();
    let (entry, path) = insert_image_via_core(&png_bytes(10));
    assert!(path.exists());

    ditox(&dir).args(["delete", &entry.id]).assert().success();

    assert!(!path.exists(), "cli delete should unlink the blob");
}

#[test]
fn cli_clear_removes_all_blobs() {
    let (dir, _lock) = setup();
    let (_e1, p1) = insert_image_via_core(&png_bytes(20));
    let (_e2, p2) = insert_image_via_core(&png_bytes(21));
    assert!(p1.exists() && p2.exists());

    ditox(&dir)
        .args(["clear", "--confirm"])
        .assert()
        .success();

    assert!(!p1.exists() && !p2.exists());
    let images_dir = dir.path().join("ditox/images");
    assert_eq!(count_files(&images_dir), 0);
}

#[test]
fn cli_repair_reclaims_orphan_file() {
    let (dir, _lock) = setup();
    // Write an orphan directly to disk (no DB row).
    let images_dir = dir.path().join("ditox/images/ab");
    std::fs::create_dir_all(&images_dir).unwrap();
    let orphan = images_dir.join(format!("{}.png", "a".repeat(64)));
    std::fs::write(&orphan, b"orphan").unwrap();
    assert!(orphan.exists());

    ditox(&dir).args(["repair"]).assert().success();

    assert!(!orphan.exists(), "repair should reclaim orphans");
}

#[test]
fn cli_repair_prunes_dangling_row() {
    let (dir, _lock) = setup();
    // DB row without backing blob.
    let hash = "d".repeat(64);
    let entry = Entry::new_image(hash, 42, "png".to_string());
    let db = Database::open().unwrap();
    db.insert(&entry).unwrap();
    // Don't call store_image_blob — file is intentionally missing.

    ditox(&dir).args(["repair"]).assert().success();

    let remaining = Database::open().unwrap().count().unwrap();
    assert_eq!(remaining, 0, "dangling row should be pruned");
}

#[test]
fn cli_repair_dry_run_reports_but_does_nothing() {
    let (dir, _lock) = setup();

    let images_dir = dir.path().join("ditox/images/ab");
    std::fs::create_dir_all(&images_dir).unwrap();
    let orphan = images_dir.join(format!("{}.png", "a".repeat(64)));
    std::fs::write(&orphan, b"orphan").unwrap();

    let hash = "d".repeat(64);
    let entry = Entry::new_image(hash, 42, "png".to_string());
    let db = Database::open().unwrap();
    db.insert(&entry).unwrap();

    ditox(&dir)
        .args(["repair", "--dry-run"])
        .assert()
        .success();

    // Both untouched.
    assert!(orphan.exists(), "dry-run must not remove files");
    assert_eq!(
        Database::open().unwrap().count().unwrap(),
        1,
        "dry-run must not delete rows"
    );
}

#[test]
fn cli_repair_fix_hashes_quarantines_mismatches() {
    let (dir, _lock) = setup();

    // Insert a row whose on-disk content doesn't match the declared hash.
    let claimed_hash = "c".repeat(64);
    let entry = Entry::new_image(claimed_hash.clone(), 8, "png".to_string());
    let db = Database::open().unwrap();
    db.insert(&entry).unwrap();

    // Create the "wrong" file at the right path.
    let path = Database::image_path(&claimed_hash, "png").unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"bogus-bytes").unwrap();
    assert!(path.exists());

    ditox(&dir)
        .args(["repair", "--fix-hashes"])
        .assert()
        .success();

    // Original path should be empty (moved to quarantine), the file itself
    // should live under `images/.quarantine/`.
    assert!(!path.exists());
    let qdir = dir.path().join("ditox/images/.quarantine");
    let q_files: Vec<_> = std::fs::read_dir(&qdir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(
        q_files.len(),
        1,
        "exactly one file should be quarantined"
    );
}
