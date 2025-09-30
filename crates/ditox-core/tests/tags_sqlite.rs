use ditox_core::{Store, StoreImpl};
use tempfile::tempdir;

#[test]
fn tags_roundtrip() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("t.db");
    let store = StoreImpl::new_with(&db, true).expect("store");
    let c = store.add("taggable").unwrap();
    store.add_tags(&c.id, &["x".into(), "y".into()]).unwrap();
    let tags = store.list_tags(&c.id).unwrap();
    assert!(tags.contains(&"x".to_string()));
    assert!(tags.contains(&"y".to_string()));
    store.remove_tags(&c.id, &["x".into()]).unwrap();
    let tags2 = store.list_tags(&c.id).unwrap();
    assert!(!tags2.contains(&"x".to_string()));
}
