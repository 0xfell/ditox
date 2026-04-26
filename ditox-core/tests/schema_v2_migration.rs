//! Schema v1 → v2 migration tests.
//!
//! Verifies:
//! - Fresh DBs start at v2 with all expected columns and indexes.
//! - A v1 DB (image_extension column present, no v2 columns) migrates
//!   cleanly to v2 with backfill: `entry_kind = entry_type`,
//!   `format_count = 1`, `captured_at = created_at`, `source_app NULL`.
//! - The migration is idempotent — re-opening doesn't re-apply.
//! - Existing entry data survives.

use ditox_core::db::{set_data_dir_override, Database, SCHEMA_VERSION};
use ditox_core::Entry;
use rusqlite::Connection;
use std::sync::Mutex;
use tempfile::TempDir;

static OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn fresh_db_lands_at_current_schema_version() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    let conn = Connection::open(tmp.path().join("ditox.db")).unwrap();
    let version: String = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION.to_string());

    drop(db);
    set_data_dir_override(None).unwrap();
}

#[test]
fn fresh_db_has_v2_columns_and_index() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let db = Database::open().unwrap();
    db.init_schema().unwrap();

    let conn = Connection::open(tmp.path().join("ditox.db")).unwrap();

    // Inspect entries column list.
    let mut stmt = conn.prepare("PRAGMA table_info(entries)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert!(
        cols.contains(&"entry_kind".to_string()),
        "entry_kind missing"
    );
    assert!(
        cols.contains(&"format_count".to_string()),
        "format_count missing"
    );
    assert!(
        cols.contains(&"source_app".to_string()),
        "source_app missing"
    );
    assert!(
        cols.contains(&"captured_at".to_string()),
        "captured_at missing"
    );

    // Index exists.
    let idx: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_entries_source_app'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(idx, 1, "idx_entries_source_app missing");

    drop(db);
    set_data_dir_override(None).unwrap();
}

#[test]
fn migration_backfills_from_v1_data() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    // Step 1: build a v1-like DB by hand (no v2 columns, version=1).
    let db_path = tmp.path().join("ditox.db");
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            r#"
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
                image_extension TEXT
            );
            INSERT INTO schema_meta(key, value) VALUES ('version', '1');
            INSERT INTO entries
              (id, entry_type, content, hash, byte_size, created_at, last_used, pinned)
            VALUES
              ('id-text', 'text', 'hello', 'abc123', 5, '2026-01-01T00:00:00Z',
               '2026-01-02T00:00:00Z', 0),
              ('id-image', 'image', 'deadbeef', 'deadbeef00000000', 100,
               '2026-01-03T00:00:00Z', '2026-01-04T00:00:00Z', 1);
            "#,
        )
        .unwrap();
    }

    // Step 2: open via Database — migration runs.
    {
        let db = Database::open().unwrap();
        db.init_schema().unwrap();
    }

    // Step 3: verify the migrated DB.
    let conn = Connection::open(&db_path).unwrap();

    let version: String = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    // v2 column backfills run as part of the v1→…→current chain; the
    // recorded version is always the latest (`SCHEMA_VERSION`).
    assert_eq!(version, SCHEMA_VERSION.to_string());

    // entry_kind populated from entry_type
    let (text_kind, text_captured, text_format_count, text_source): (
        String,
        String,
        i64,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT entry_kind, captured_at, format_count, source_app
             FROM entries WHERE id = 'id-text'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(text_kind, "text");
    assert_eq!(text_captured, "2026-01-01T00:00:00Z"); // = created_at
    assert_eq!(text_format_count, 1);
    assert!(text_source.is_none());

    let (img_kind, img_captured, img_format_count): (String, String, i64) = conn
        .query_row(
            "SELECT entry_kind, captured_at, format_count
             FROM entries WHERE id = 'id-image'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(img_kind, "image");
    assert_eq!(img_captured, "2026-01-03T00:00:00Z");
    assert_eq!(img_format_count, 1);

    // Both rows still present.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);

    set_data_dir_override(None).unwrap();
}

#[test]
fn migration_is_idempotent() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    // Open once → fresh v2 DB.
    {
        let db = Database::open().unwrap();
        db.init_schema().unwrap();
        let entry = Entry::new_text("idempotent".to_string());
        db.insert(&entry).unwrap();
    }

    // Open again → should be no-op, no errors, data intact.
    {
        let db = Database::open().unwrap();
        db.init_schema().unwrap();
        assert_eq!(db.count().unwrap(), 1);
    }

    // Verify version is still 2 (not 3 or anything else).
    let conn = Connection::open(tmp.path().join("ditox.db")).unwrap();
    let version: String = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION.to_string());

    set_data_dir_override(None).unwrap();
}
