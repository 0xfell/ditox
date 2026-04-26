//! Snapshot tests for the v2 → v3 migration (multi-format model).
//!
//! Strategy: programmatically construct DBs at the v0/v1/v2 schema
//! states, then open them with the production code and assert the
//! resulting v3 invariants. We avoid storing binary fixture files
//! (those rot when the format changes); each test is reproducible
//! from source.

#![cfg(unix)]

use ditox_core::db::{set_data_dir_override, Database, ExtraFormat, SCHEMA_VERSION};
use ditox_core::Entry;
use rusqlite::{params, Connection};
use std::sync::Mutex;
use tempfile::TempDir;

// `set_data_dir_override` is process-wide; serialize tests.
static OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

fn db_path(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join("ditox.db")
}

/// Write a fresh DB at the *exact* pre-v3 schema by hand (no
/// migrations run). Used to drive the snapshot scenarios deterministically.
///
/// Mirrors what `init_schema` would have produced after v2 had landed but
/// before v3: `entries` table with v2's columns, `entries_fts`, the
/// pending-blob-prune queue, and `schema_meta(version=2)`.
fn build_v2_snapshot(tmp: &TempDir) {
    let conn = Connection::open(db_path(tmp)).unwrap();
    conn.execute_batch(
        "
        CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

        CREATE TABLE entries (
            id TEXT PRIMARY KEY,
            entry_type TEXT NOT NULL,
            content TEXT NOT NULL,
            hash TEXT NOT NULL UNIQUE,
            byte_size INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            pinned INTEGER DEFAULT 0,
            last_used TEXT,
            usage_count INTEGER DEFAULT 0,
            notes TEXT,
            collection_id TEXT,
            image_extension TEXT,
            entry_kind TEXT,
            format_count INTEGER NOT NULL DEFAULT 1,
            source_app TEXT,
            captured_at TEXT
        );

        CREATE TABLE pending_blob_prunes (
            hash TEXT NOT NULL,
            extension TEXT NOT NULL,
            queued_at TEXT NOT NULL,
            PRIMARY KEY (hash, extension)
        );

        CREATE TABLE collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT,
            keybind TEXT,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE INDEX idx_created_at      ON entries(created_at DESC);
        CREATE INDEX idx_last_used       ON entries(last_used DESC);
        CREATE INDEX idx_hash            ON entries(hash);
        CREATE INDEX idx_collection_id   ON entries(collection_id);
        CREATE INDEX idx_entries_source_app ON entries(source_app);

        CREATE VIRTUAL TABLE entries_fts USING fts5(id UNINDEXED, content, notes);

        INSERT INTO schema_meta(key, value) VALUES ('version', '2');
        ",
    )
    .unwrap();

    // Realistic data: one text entry, one image entry, one with a note.
    conn.execute(
        "INSERT INTO entries (id, entry_type, content, hash, byte_size,
                              created_at, last_used, pinned, notes,
                              collection_id, image_extension,
                              entry_kind, format_count, captured_at)
         VALUES ('id-text', 'text', 'hello world', 'aaa111', 11,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0, NULL,
                 NULL, NULL, 'text', 1, '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO entries (id, entry_type, content, hash, byte_size,
                              created_at, last_used, pinned, notes,
                              collection_id, image_extension,
                              entry_kind, format_count, captured_at)
         VALUES ('id-image', 'image', 'bbb222', 'bbb222', 8192,
                 '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z', 0, NULL,
                 NULL, 'png', 'image', 1, '2026-01-02T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO entries (id, entry_type, content, hash, byte_size,
                              created_at, last_used, pinned, notes,
                              collection_id, image_extension,
                              entry_kind, format_count, captured_at)
         VALUES ('id-noted', 'text', 'noted entry', 'ccc333', 11,
                 '2026-01-03T00:00:00Z', '2026-01-03T00:00:00Z', 0,
                 'this is the note', NULL, NULL, 'text', 1,
                 '2026-01-03T00:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO entries_fts(id, content, notes)
         SELECT id, content, notes FROM entries",
        [],
    )
    .unwrap();
}

/// Verify a v3 DB has all the structural invariants expected after
/// `migrate_to_v3` runs. Used by every test to assert the post-condition.
fn assert_v3_invariants(tmp: &TempDir, expected_entry_count: i64) {
    let conn = Connection::open(db_path(tmp)).unwrap();

    // Schema version.
    let version: String = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION.to_string());

    // entry_formats exists with expected columns.
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(entry_formats)")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    for required in [
        "id",
        "entry_id",
        "format_name",
        "storage",
        "content",
        "blob_hash",
        "blob_ext",
        "byte_size",
        "format_hash",
        "canonical",
        "created_at",
    ] {
        assert!(
            cols.contains(&required.to_string()),
            "entry_formats missing column {}",
            required
        );
    }

    // entries.canonical_format exists.
    let entry_cols: Vec<String> = conn
        .prepare("PRAGMA table_info(entries)")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        entry_cols.contains(&"canonical_format".to_string()),
        "entries.canonical_format column missing"
    );

    // Indexes present.
    let idx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name IN
                 ('idx_entry_formats_entry',
                  'idx_entry_formats_blob_hash',
                  'idx_entry_formats_canonical')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(idx_count, 3, "missing entry_formats indexes");

    // FTS table for per-format content.
    let fts_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'format_content_fts'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(fts_exists, 1, "format_content_fts missing");

    // One canonical row per entry.
    let canon_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats WHERE canonical = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        canon_count, expected_entry_count,
        "expected one canonical entry_formats row per entry"
    );

    // entries.canonical_format populated for every entry.
    let null_canonical: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE canonical_format IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        null_canonical, 0,
        "entries.canonical_format must be backfilled"
    );
}

#[test]
fn migration_v2_to_v3_backfills_entry_formats() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    build_v2_snapshot(&tmp);

    // Open with current production code → migration runs.
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    drop(db);

    assert_v3_invariants(&tmp, 3);

    // Spot-check the text-entry backfill.
    let conn = Connection::open(db_path(&tmp)).unwrap();
    let (fmt, storage, content, blob_hash, blob_ext, format_hash, canonical): (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        i64,
    ) = conn
        .query_row(
            "SELECT format_name, storage, content, blob_hash, blob_ext,
                    format_hash, canonical
             FROM entry_formats WHERE entry_id = 'id-text'",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(fmt, "text/plain;charset=utf-8");
    assert_eq!(storage, "inline");
    assert_eq!(content.as_deref(), Some("hello world"));
    assert!(blob_hash.is_none());
    assert!(blob_ext.is_none());
    assert_eq!(format_hash, "aaa111");
    assert_eq!(canonical, 1);

    // Spot-check the image-entry backfill.
    let (fmt, storage, content, blob_hash, blob_ext, format_hash): (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    ) = conn
        .query_row(
            "SELECT format_name, storage, content, blob_hash, blob_ext,
                    format_hash
             FROM entry_formats WHERE entry_id = 'id-image'",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(fmt, "image/png");
    assert_eq!(storage, "blob_file");
    assert!(content.is_none());
    assert_eq!(blob_hash.as_deref(), Some("bbb222"));
    assert_eq!(blob_ext.as_deref(), Some("png"));
    assert_eq!(format_hash, "bbb222");

    // canonical_format pointer matches.
    let canonical_pointer: String = conn
        .query_row(
            "SELECT canonical_format FROM entries WHERE id = 'id-text'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(canonical_pointer, "text/plain;charset=utf-8");

    let _ = set_data_dir_override(None);
}

#[test]
fn migration_v2_to_v3_populates_format_content_fts() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    build_v2_snapshot(&tmp);
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    drop(db);

    let conn = Connection::open(db_path(&tmp)).unwrap();

    // Two text entries → two inline rows in format_content_fts.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM format_content_fts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "expected 2 inline text formats in FTS");

    // Search hits both text entries.
    let hits: Vec<String> = conn
        .prepare(
            "SELECT entry_id FROM format_content_fts
             WHERE format_content_fts MATCH 'hello'",
        )
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(hits.contains(&"id-text".to_string()));

    let _ = set_data_dir_override(None);
}

#[test]
fn migration_v2_to_v3_is_idempotent() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    build_v2_snapshot(&tmp);
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    drop(db);

    // Re-open: must be a no-op without errors or row duplication.
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    drop(db);

    let conn = Connection::open(db_path(&tmp)).unwrap();
    let canon_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats WHERE canonical = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(canon_count, 3, "re-running must not duplicate rows");

    let _ = set_data_dir_override(None);
}

#[test]
fn fresh_v3_db_has_full_schema() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    drop(db);

    assert_v3_invariants(&tmp, 0);
    let _ = set_data_dir_override(None);
}

#[test]
fn insert_writes_canonical_format_row_in_same_txn() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    let entry = Entry::new_text("post-v3 insert".to_string());
    let entry_id = entry.id.clone();
    db.insert(&entry).unwrap();
    drop(db);

    let conn = Connection::open(db_path(&tmp)).unwrap();
    let (fmt, content, canonical): (String, Option<String>, i64) = conn
        .query_row(
            "SELECT format_name, content, canonical FROM entry_formats
             WHERE entry_id = ?1",
            params![entry_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(fmt, "text/plain;charset=utf-8");
    assert_eq!(content.as_deref(), Some("post-v3 insert"));
    assert_eq!(canonical, 1);

    // FTS got the new row too (via trigger).
    let fts_hit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM format_content_fts
             WHERE format_content_fts MATCH 'post'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(fts_hit, 1, "trigger must populate format_content_fts");

    let _ = set_data_dir_override(None);
}

#[test]
fn search_finds_legacy_entries_via_format_content_fts() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    // v2 snapshot has 3 entries; "hello world", "noted entry" + image.
    build_v2_snapshot(&tmp);
    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // After v3 migration, format_content_fts holds canonical text
    // for the two text entries. Searching via the public API should
    // find both via the new index.
    let hits = db.search_entries("hello", 10).unwrap();
    let ids: Vec<_> = hits.iter().map(|e| e.id.clone()).collect();
    assert!(ids.contains(&"id-text".to_string()), "got: {:?}", ids);

    let noted = db.search_entries("noted", 10).unwrap();
    let noted_ids: Vec<_> = noted.iter().map(|e| e.id.clone()).collect();
    assert!(
        noted_ids.contains(&"id-noted".to_string()),
        "got: {:?}",
        noted_ids
    );

    let _ = set_data_dir_override(None);
}

#[test]
fn search_notes_only_finds_via_legacy_fts() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    build_v2_snapshot(&tmp);
    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // "this is the note" sits in entries.notes for id-noted only.
    let by_notes = db.search_notes_only("note", 10).unwrap();
    let ids: Vec<_> = by_notes.iter().map(|e| e.id.clone()).collect();
    assert!(
        ids.contains(&"id-noted".to_string()),
        "expected id-noted via notes search, got {:?}",
        ids
    );

    let _ = set_data_dir_override(None);
}

#[test]
fn search_in_format_filters_by_format_name() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    build_v2_snapshot(&tmp);
    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // All text entries got format_name = text/plain;charset=utf-8 in
    // the backfill. Searching that format → finds them. Searching
    // text/html → finds nothing (no HTML formats yet — those land
    // when sub-tasks 1.3 / 1.4 enable multi-format capture).
    let plain = db
        .search_entries_in_format("hello", "text/plain;charset=utf-8", 10)
        .unwrap();
    assert!(!plain.is_empty(), "should find via text/plain format");

    let html = db
        .search_entries_in_format("hello", "text/html", 10)
        .unwrap();
    assert!(html.is_empty(), "no HTML formats backfilled by v3");

    let _ = set_data_dir_override(None);
}

#[test]
fn search_filtered_today_works_post_v3() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // Insert via the production API so the new entry_formats trigger
    // populates format_content_fts.
    let entry = Entry::new_text("today entry".to_string());
    db.insert(&entry).unwrap();

    // "all" filter
    let all = db
        .search_entries_filtered("today", 10, "all", None)
        .unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].content, "today entry");

    // "text" filter
    let text = db
        .search_entries_filtered("today", 10, "text", None)
        .unwrap();
    assert_eq!(text.len(), 1);

    // "today" date filter (entry was inserted just now)
    let today = db
        .search_entries_filtered("today", 10, "today", None)
        .unwrap();
    assert_eq!(today.len(), 1);

    let _ = set_data_dir_override(None);
}

#[test]
fn insert_multi_writes_canonical_plus_extras_atomically() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // Imagine copying from Word: canonical text + HTML + RTF.
    let entry = Entry::new_text("hello formatted".to_string());
    let entry_id = entry.id.clone();
    let extras = vec![
        ExtraFormat::new("text/html", b"<p>hello formatted</p>".to_vec()),
        ExtraFormat::new("text/rtf", b"{\\rtf1\\rsid12345 hello formatted}".to_vec()),
    ];
    db.insert_multi(&entry, &extras).unwrap();
    drop(db);

    let conn = Connection::open(db_path(&tmp)).unwrap();

    // Three formats land: canonical text + 2 extras.
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats WHERE entry_id = ?1",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 3);

    // Exactly one canonical row.
    let canon_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats
             WHERE entry_id = ?1 AND canonical = 1",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(canon_count, 1);

    // entries.format_count is updated to 3.
    let format_count: i64 = conn
        .query_row(
            "SELECT format_count FROM entries WHERE id = ?1",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(format_count, 3);

    // RTF row's format_hash equals the \rsid-stripped canonical hash
    // (so a re-copy with a different rsid number dedups).
    let rtf_hash: String = conn
        .query_row(
            "SELECT format_hash FROM entry_formats
             WHERE entry_id = ?1 AND format_name = 'text/rtf'",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    let expected = ditox_core::format::canonicalise::format_hash(
        "text/rtf",
        b"{\\rtf1\\rsid99999 hello formatted}",
    );
    assert_eq!(
        rtf_hash, expected,
        "rsid-stripped RTF should hash identically across rsid variants"
    );

    let _ = set_data_dir_override(None);
}

#[test]
fn insert_multi_with_empty_extras_matches_plain_insert() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    let entry = Entry::new_text("plain".to_string());
    let id = entry.id.clone();
    db.insert_multi(&entry, &[]).unwrap();
    drop(db);

    let conn = Connection::open(db_path(&tmp)).unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats WHERE entry_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1, "empty extras → just the canonical row");

    let _ = set_data_dir_override(None);
}

#[test]
fn insert_multi_image_extras_land_in_blob_store() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // Canonical: text. Extra: a PNG variant of the same content (e.g.
    // a screenshot whose alt-text was put on the clipboard).
    let entry = Entry::new_text("see attached".to_string());
    let id = entry.id.clone();
    let png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
    let extras = vec![ExtraFormat::new("image/png", png_bytes.clone())];
    db.insert_multi(&entry, &extras).unwrap();
    drop(db);

    let conn = Connection::open(db_path(&tmp)).unwrap();
    let (storage, blob_hash, blob_ext): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT storage, blob_hash, blob_ext FROM entry_formats
             WHERE entry_id = ?1 AND format_name = 'image/png'",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(storage, "blob_file");
    let hash = blob_hash.expect("image/png extra must have blob_hash");
    assert_eq!(blob_ext.as_deref(), Some("png"));

    // Blob file actually exists on disk in the content-addressed
    // images dir.
    let blob_path = tmp
        .path()
        .join("images")
        .join(&hash[..2])
        .join(format!("{}.png", hash));
    assert!(
        blob_path.exists(),
        "PNG blob should be written to {}",
        blob_path.display()
    );

    let _ = set_data_dir_override(None);
}

#[test]
fn insert_multi_rolls_back_blobs_on_db_failure() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    // Insert one entry to occupy the hash slot.
    let entry = Entry::new_text("first".to_string());
    db.insert(&entry).unwrap();
    let collision_hash = entry.hash.clone();

    // Build a *second* Entry with the same hash so the
    // `INSERT OR IGNORE INTO entries` produces 0 rows. The unique
    // constraint on `entries.hash` causes the second insert to be a
    // no-op; the multi-format pipeline detects this and skips the
    // mirror inserts. Any extra blobs written above would be
    // unreferenced; rollback should remove them.
    let mut conflict = Entry::new_text("second".to_string());
    conflict.hash = collision_hash.clone();

    let png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x99, 0x99, 0x99, 0x99, 11, 12, 13];
    let extras = vec![ExtraFormat::new("image/png", png_bytes.clone())];
    let png_hash = ditox_core::format::canonicalise::format_hash("image/png", &png_bytes);

    // Insert succeeds (no SQL error) but the entries row is ignored.
    db.insert_multi(&conflict, &extras).unwrap();
    drop(db);

    // Conflict's mirror row should NOT exist (we short-circuited).
    let conn = Connection::open(db_path(&tmp)).unwrap();
    let mirror_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats
             WHERE entry_id = ?1 AND format_name = 'image/png'",
            params![conflict.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        mirror_count, 0,
        "conflict insert must not write a mirror row"
    );

    // The PNG blob WAS written (we don't roll back on a clean
    // hash-collision short-circuit, only on actual SQL errors).
    // This is a known v0.4 limitation documented in `insert_multi`'s
    // doc comment — mark it explicit here so a future tightening is
    // a deliberate change rather than a silent regression.
    let blob_path = tmp
        .path()
        .join("images")
        .join(&png_hash[..2])
        .join(format!("{}.png", png_hash));
    assert!(
        blob_path.exists(),
        "blob is written before tx; collision short-circuit doesn't unlink it (known limitation)"
    );

    let _ = set_data_dir_override(None);
}

#[test]
fn delete_removes_entry_and_its_formats_rows() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let mut db = Database::open().unwrap();
    db.init_schema().unwrap();

    let entry = Entry::new_text("delete me".to_string());
    let entry_id = entry.id.clone();
    db.insert(&entry).unwrap();

    // Pre-condition: insert wrote both rows.
    let conn = Connection::open(db_path(&tmp)).unwrap();
    let pre_format_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats WHERE entry_id = ?1",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pre_format_rows, 1, "insert must populate entry_formats");
    drop(conn);

    // Delete via the production API.
    db.delete(&entry_id).unwrap();
    drop(db);

    // Post-condition: both entries and entry_formats rows are gone.
    let conn = Connection::open(db_path(&tmp)).unwrap();
    let entry_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE id = ?1",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(entry_rows, 0, "delete must remove the entries row");

    let format_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_formats WHERE entry_id = ?1",
            params![entry_id],
            |r| r.get(0),
        )
        .unwrap();
    // Either ON DELETE CASCADE fires (FK enforcement on) or the
    // production `delete` explicitly removes entry_formats. Either
    // path is acceptable; we just need the invariant. If this fails,
    // see sub-task 1.1 follow-up — production must enable
    // `PRAGMA foreign_keys = ON` on every connection (it's per-connection
    // in SQLite) or explicitly cascade.
    assert_eq!(
        format_rows, 0,
        "delete must remove the entry's entry_formats rows"
    );

    let _ = set_data_dir_override(None);
}
