//! Tests for the content-addressed image storage introduced by
//! `011-image-storage-bug`.
//!
//! These tests exercise the DB layer directly with a temp `XDG_DATA_HOME`
//! so they don't depend on the clipboard or on a running watcher.

use chrono::Utc;
use ditox_core::db::Database;
use ditox_core::entry::Entry;
use std::sync::Mutex;
use tempfile::TempDir;

/// Serialize tests that rely on mutating `XDG_DATA_HOME`. `Database::open`
/// resolves paths via `ProjectDirs` which reads the env var at call time,
/// so tests run in parallel would race.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup() -> (TempDir, std::sync::MutexGuard<'static, ()>, Database) {
    let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = TempDir::new().unwrap();
    // SAFETY: guarded by ENV_LOCK so no other test is reading XDG_DATA_HOME.
    unsafe {
        std::env::set_var("XDG_DATA_HOME", dir.path());
    }
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    (dir, lock, db)
}

fn fake_png(byte: u8) -> Vec<u8> {
    // Tiny but structurally real PNG; content matters only for hashing.
    // Shape: different `byte` -> different content -> different hash.
    let mut v = vec![
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, // signature
    ];
    v.extend_from_slice(b"IHDR");
    v.push(byte);
    v.extend_from_slice(&[0u8; 16]);
    v
}

fn insert_image(db: &Database, bytes: &[u8]) -> Entry {
    let hash = Entry::compute_hash(bytes);
    let ext = "png".to_string();
    let (_path, _new) = Database::store_image_blob(&hash, &ext, bytes).unwrap();
    let entry = Entry::new_image(hash, bytes.len(), ext);
    db.insert(&entry).unwrap();
    entry
}

#[test]
fn store_image_blob_is_idempotent_and_atomic() {
    let (dir, _lock, _db) = setup();
    let bytes = fake_png(1);
    let hash = Entry::compute_hash(&bytes);
    let ext = "png";

    let (path1, new1) = Database::store_image_blob(&hash, ext, &bytes).unwrap();
    assert!(new1, "first store should materialize the file");
    assert!(path1.exists());

    // Second store with the same hash should be a no-op.
    let (path2, new2) = Database::store_image_blob(&hash, ext, &bytes).unwrap();
    assert_eq!(path1, path2);
    assert!(!new2, "second store should no-op (content-addressed)");

    // No leftover .tmp.
    let images_dir = dir.path().join("ditox/images");
    let has_tmp = walkdir(&images_dir)
        .iter()
        .any(|p| p.to_string_lossy().ends_with(".tmp"));
    assert!(!has_tmp, "no tmp leftover after atomic rename");
}

#[test]
fn delete_prunes_backing_file() {
    let (_dir, _lock, mut db) = setup();
    let entry = insert_image(&db, &fake_png(2));
    let path = entry.image_path().unwrap();
    assert!(path.exists());

    assert!(db.delete(&entry.id).unwrap());
    assert!(!path.exists(), "delete should unlink the blob");
}

#[test]
fn clear_all_prunes_every_file() {
    let (_dir, _lock, mut db) = setup();
    let e1 = insert_image(&db, &fake_png(3));
    let e2 = insert_image(&db, &fake_png(4));
    let p1 = e1.image_path().unwrap();
    let p2 = e2.image_path().unwrap();
    assert!(p1.exists() && p2.exists());

    let n = db.clear_all().unwrap();
    assert!(n >= 2, "clear_all should report >=2 rows deleted");
    assert!(!p1.exists() && !p2.exists(), "both blobs must be gone");
}

#[test]
fn cleanup_old_prunes_evicted_blobs() {
    let (_dir, _lock, mut db) = setup();
    let entries: Vec<Entry> = (10..15).map(|b| insert_image(&db, &fake_png(b))).collect();
    assert_eq!(db.count().unwrap(), 5);

    // Cap to 3 — oldest 2 must be evicted AND their blobs unlinked.
    let evicted = db.cleanup_old(3).unwrap();
    assert_eq!(evicted, 2);

    // Exactly 3 blobs remain on disk.
    let paths: Vec<_> = entries.iter().map(|e| e.image_path().unwrap()).collect();
    let remaining: usize = paths.iter().filter(|p| p.exists()).count();
    assert_eq!(remaining, 3, "blobs for retained entries should exist");
}

#[test]
fn startup_drains_pending_prune_queue() {
    // Simulate a crash: row gone, queue populated, file still on disk.
    // A fresh `open + init_schema` should reclaim the file.
    let (dir, _lock, _db_scope) = setup();
    drop(_db_scope);

    // Populate queue + file by hand.
    let hash = "aa".repeat(32); // 64 hex chars
    let ext = "png";
    let bytes = fake_png(99);
    let path = Database::image_path(&hash, ext).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(path.exists());

    // Open a connection and inject a pending prune. We bypass the
    // high-level API here because the scenario is "crash left a queue
    // entry behind" — there's no normal code path that does this without
    // also deleting the blob.
    {
        let conn = rusqlite::Connection::open(dir.path().join("ditox/ditox.db")).unwrap();
        conn.execute(
            "INSERT INTO pending_blob_prunes (hash, extension, queued_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![hash, ext, Utc::now().to_rfc3339()],
        )
        .unwrap();
    }

    // Re-open via the public API — startup should drain the queue.
    let db2 = Database::open().unwrap();
    db2.init_schema().unwrap();
    assert!(!path.exists(), "startup should have pruned the orphan blob");
}

#[test]
fn scan_image_files_ignores_quarantine_and_tmp() {
    let (dir, _lock, _db) = setup();
    let images = dir.path().join("ditox/images");
    std::fs::create_dir_all(images.join(".quarantine")).unwrap();
    std::fs::create_dir_all(images.join("ab")).unwrap();

    std::fs::write(images.join(".quarantine/should-ignore.png"), b"x").unwrap();
    std::fs::write(images.join("ab/somefile.tmp"), b"x").unwrap();
    std::fs::write(
        images.join("ab").join(format!("{}.png", "a".repeat(64))),
        b"x",
    )
    .unwrap();

    let db = Database::open().unwrap();
    let files = db.scan_image_files().unwrap();
    // Only the real .png under `ab/` should surface.
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().ends_with(".png"));
}

fn walkdir(p: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if !p.exists() {
        return out;
    }
    for e in std::fs::read_dir(p).unwrap().flatten() {
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            out.extend(walkdir(&e.path()));
        } else {
            out.push(e.path());
        }
    }
    out
}
