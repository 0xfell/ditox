use ditox_core::{StoreImpl, Store, Query, ClipKind};
use tempfile::tempdir;

#[test]
fn text_crud_and_search() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");
    let store = StoreImpl::new_with(&db, true).expect("store");

    // add
    let a = store.add("alpha one").unwrap();
    let b = store.add("bravo two").unwrap();
    assert_eq!(a.kind, ClipKind::Text);
    // list
    let list = store.list(Query { contains: None, favorites_only: false, limit: None }).unwrap();
    assert!(list.iter().any(|c| c.id == a.id));
    // search
    let res = store.list(Query { contains: Some("alpha".into()), favorites_only: false, limit: None }).unwrap();
    assert!(res.iter().any(|c| c.id == a.id));
    // favorite toggle
    store.favorite(&b.id, true).unwrap();
    let only_fav = store.list(Query { contains: None, favorites_only: true, limit: None }).unwrap();
    assert!(only_fav.iter().any(|c| c.id == b.id));
}

#[test]
fn image_roundtrip() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");
    let store = StoreImpl::new_with(&db, true).expect("store");

    // 2x2 RGBA opaque white
    let w = 2u32; let h = 2u32; let rgba = vec![255u8; (w*h*4) as usize];
    let clip = store.add_image_rgba(w, h, &rgba).unwrap();
    assert_eq!(clip.kind, ClipKind::Image);
    let meta = store.get_image_meta(&clip.id).unwrap().expect("meta");
    assert_eq!((meta.width, meta.height), (w, h));
    let img = store.get_image_rgba(&clip.id).unwrap().expect("img");
    assert_eq!((img.width, img.height), (w, h));
    assert_eq!(img.bytes.len(), (w*h*4) as usize);
    let imgs = store.list_images(Query { contains: None, favorites_only: false, limit: None }).unwrap();
    assert!(imgs.iter().any(|(c,_)| c.id == clip.id));
}
