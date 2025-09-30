use ditox_core::{Query, Store, StoreImpl};
use tempfile::tempdir;

#[test]
fn list_orders_by_recency_with_last_used() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("r.db");
    let store = StoreImpl::new_with(&db, true).expect("store");

    let a = store.add("alpha").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = store.add("bravo").unwrap();

    // Initially, bravo is first (newest)
    let list = store
        .list(Query {
            contains: None,
            favorites_only: false,
            limit: None,
            tag: None,
            rank: false,
        })
        .unwrap();
    assert_eq!(list.first().unwrap().id, b.id);

    // Touch alpha and expect it to rise to the top
    store.touch_last_used(&a.id).unwrap();
    let list2 = store
        .list(Query {
            contains: None,
            favorites_only: false,
            limit: None,
            tag: None,
            rank: false,
        })
        .unwrap();
    assert_eq!(list2.first().unwrap().id, a.id);
}
