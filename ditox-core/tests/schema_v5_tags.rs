//! Snapshot and CRUD tests for the v4 -> v5 tags migration.

#![cfg(unix)]

use chrono::{Duration, Utc};
use ditox_core::db::{set_data_dir_override, Database, SCHEMA_VERSION};
use ditox_core::Entry;
use rusqlite::Connection;
use std::sync::Mutex;
use tempfile::TempDir;

static OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

fn db_path(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join("ditox.db")
}

fn with_db<T>(f: impl FnOnce(&Database) -> T) -> T {
    let _guard = OVERRIDE_LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    let out = f(&db);
    set_data_dir_override(None).unwrap();
    out
}

fn build_v4_snapshot(tmp: &TempDir) {
    let conn = Connection::open(db_path(tmp)).unwrap();
    conn.execute_batch(
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

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
            captured_at TEXT,
            canonical_format TEXT
        );

        CREATE TABLE collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT,
            keybind TEXT,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE filter_rules (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL,
            pattern      TEXT NOT NULL,
            pattern_kind TEXT NOT NULL,
            process_glob TEXT,
            action       TEXT NOT NULL,
            enabled      INTEGER NOT NULL DEFAULT 1,
            position     INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL
        );

        INSERT INTO schema_meta(key, value) VALUES ('version', '4');",
    )
    .unwrap();
}

#[test]
fn migrates_v4_snapshot_to_v5_tag_schema() {
    let _guard = OVERRIDE_LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    build_v4_snapshot(&tmp);

    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    set_data_dir_override(None).unwrap();

    let conn = Connection::open(db_path(&tmp)).unwrap();
    let version: String = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION.to_string());

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN ('tags', 'entry_tags')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 2);

    let idx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name IN
                ('idx_entry_tags_tag', 'idx_entry_tags_entry')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(idx_count, 2);
}

#[test]
fn tag_crud_and_entry_links_are_idempotent() {
    with_db(|db| {
        let entry = Entry::new_text("hello tags".to_string());
        db.insert(&entry).unwrap();

        let tag = db
            .add_tag_to_entry_by_name(&entry.id, "rust", Some("#dea584"))
            .unwrap();
        assert_eq!(tag.name, "rust");
        assert_eq!(tag.color.as_deref(), Some("#dea584"));

        db.add_tag_to_entry_by_name(&entry.id, "rust", None)
            .unwrap();
        let entry_tags = db.get_tags_for_entry(&entry.id).unwrap();
        assert_eq!(entry_tags.len(), 1);
        assert_eq!(entry_tags[0].name, "rust");

        let tagged_entries = db.get_entries_with_tag(&tag.id, 10).unwrap();
        assert_eq!(tagged_entries.len(), 1);
        assert_eq!(tagged_entries[0].id, entry.id);
        assert_eq!(db.count_entries_with_tag(&tag.id).unwrap(), 1);

        assert!(db.remove_tag_from_entry(&entry.id, &tag.id).unwrap());
        assert!(!db.remove_tag_from_entry(&entry.id, &tag.id).unwrap());
        assert!(db.get_tags_for_entry(&entry.id).unwrap().is_empty());
    });
}

#[test]
fn filtered_tag_queries_compose_with_time_and_type_filters() {
    with_db(|db| {
        let mut recent_text = Entry::new_text("recent tagged".to_string());
        recent_text.created_at = Utc::now();
        recent_text.last_used = recent_text.created_at;
        db.insert(&recent_text).unwrap();

        let mut old_text = Entry::new_text("old tagged".to_string());
        old_text.created_at = Utc::now() - Duration::days(45);
        old_text.last_used = old_text.created_at;
        db.insert(&old_text).unwrap();

        let tag = db
            .add_tag_to_entry_by_name(&recent_text.id, "work", None)
            .unwrap();
        db.add_tag_to_entry(&old_text.id, &tag.id).unwrap();

        let text_entries = db
            .get_page_filtered_with_tag(0, 10, "text", None, &tag.id)
            .unwrap();
        assert_eq!(text_entries.len(), 2);

        let today_entries = db
            .get_page_filtered_with_tag(0, 10, "today", None, &tag.id)
            .unwrap();
        assert_eq!(today_entries.len(), 1);
        assert_eq!(today_entries[0].id, recent_text.id);

        let older_count = db.count_filtered_with_tag("older", None, &tag.id).unwrap();
        assert_eq!(older_count, 1);
    });
}
