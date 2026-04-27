use ditox_core::{Database, Entry};

#[test]
fn schema_v6_adds_hotkey_columns_and_index() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let version: String = db
        .connection()
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, "6");

    let mut columns = db
        .connection()
        .prepare("PRAGMA table_info(entries)")
        .unwrap();
    let names: Vec<String> = columns
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(names.contains(&"global_hotkey".to_string()));
    assert!(names.contains(&"local_hotkey".to_string()));
}

#[test]
fn entry_hotkeys_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let entry = Entry::new_text("hello".into());
    let id = entry.id.clone();
    db.insert(&entry).unwrap();
    assert!(db
        .set_entry_hotkeys(&id, Some("ctrl+alt+1"), Some("1"))
        .unwrap());

    let entry = db.get_by_id(&id).unwrap().unwrap();
    assert_eq!(entry.global_hotkey.as_deref(), Some("ctrl+alt+1"));
    assert_eq!(entry.local_hotkey.as_deref(), Some("1"));
    assert_eq!(db.entries_with_global_hotkeys().unwrap().len(), 1);
}
