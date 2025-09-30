use ditox_core::{Query, Store, StoreImpl};
use tempfile::tempdir;

#[test]
fn prune_keeps_favorites_and_limits_count() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("p.db");
    let store = StoreImpl::new_with(&db, true).expect("store");
    // Add 5 clips
    let mut ids = Vec::new();
    for i in 0..5 {
        ids.push(store.add(&format!("c{}", i)).unwrap().id);
    }
    // Favorite the first
    store.favorite(&ids[0], true).unwrap();

    // Prune to 2 items keeping favorites
    let _removed = store.prune(Some(2), None, true).unwrap();

    let left = store
        .list(Query {
            contains: None,
            favorites_only: false,
            limit: None,
            tag: None,
            rank: false,
        })
        .unwrap();
    assert!(left.iter().any(|c| c.id == ids[0]));
    let non_fav = left.iter().filter(|c| !c.is_favorite).count();
    assert!(non_fav <= 2, "non-favorites should be pruned to at most 2");
}
