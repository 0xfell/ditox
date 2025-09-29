use ditox_core::StoreImpl;
use tempfile::tempdir;

#[test]
fn migration_adds_image_columns() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("mig.db");
    let _store = StoreImpl::new_with(&db, true).expect("store");
    // ensure migrations ran
    let conn = rusqlite::Connection::open(&db).unwrap();
    let has_is_image: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM pragma_table_info('clips') WHERE name='is_image'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let has_image_path: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM pragma_table_info('clips') WHERE name='image_path'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_is_image, 1, "is_image column missing");
    assert_eq!(has_image_path, 1, "image_path column missing");
}
