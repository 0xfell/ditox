use ditox_core::db::{set_data_dir_override, ExtraFormat};
use ditox_core::sync::{
    public_key_fingerprint, AdvertisedPeer, FormatBody, PeerTrustState, SyncDirection, SyncStatus,
};
use ditox_core::{Database, Entry};
use std::sync::Mutex;

static DATA_DIR_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn schema_v7_adds_peer_tables() {
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
    assert_eq!(version, "7");

    let peers_count: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM peers", [], |row| row.get(0))
        .unwrap();
    let log_count: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM sync_log", [], |row| row.get(0))
        .unwrap();
    assert_eq!(peers_count, 0);
    assert_eq!(log_count, 0);
}

#[test]
fn advertised_peer_can_be_persisted_as_discovered_peer() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let advertised = AdvertisedPeer::new("desk", [4u8; 32], "127.0.0.1:9001");
    let peer = db.upsert_advertised_peer(&advertised).unwrap();

    assert_eq!(peer.name, advertised.name);
    assert_eq!(peer.fingerprint, advertised.fingerprint);
    assert_eq!(peer.addresses, vec![advertised.address]);
}

#[test]
fn entry_digests_are_recent_hash_manifests() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let first = Entry::new_text("first".into());
    let first_id = first.id.clone();
    let second = Entry::new_text("second".into());
    let second_hash = second.hash.clone();
    db.insert(&first).unwrap();
    db.insert(&second).unwrap();
    db.toggle_favorite(&second.id).unwrap();

    let since = db
        .get_by_id(&first_id)
        .unwrap()
        .unwrap()
        .created_at
        .to_rfc3339();
    let digests = db.entry_digests(10, Some(&since)).unwrap();

    assert_eq!(digests.len(), 1);
    assert_eq!(digests[0].entry_hash, second_hash);
    assert!(digests[0].pinned);
}

#[test]
fn missing_entry_ids_from_digests_skips_existing_hashes() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let local = Entry::new_text("already here".into());
    db.insert(&local).unwrap();

    let remote = vec![
        ditox_core::sync::EntryDigest {
            id: "remote-existing".to_string(),
            entry_hash: local.hash.clone(),
            updated_at: "2026-04-27T00:00:00Z".to_string(),
            pinned: false,
        },
        ditox_core::sync::EntryDigest {
            id: "remote-missing".to_string(),
            entry_hash: "missing-hash".to_string(),
            updated_at: "2026-04-27T00:00:01Z".to_string(),
            pinned: false,
        },
    ];

    assert_eq!(
        db.missing_entry_ids_from_digests(&remote).unwrap(),
        vec!["remote-missing".to_string()]
    );
}

#[test]
fn entry_payload_includes_inline_formats() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let entry = Entry::new_text("plain".into());
    let id = entry.id.clone();
    db.insert_multi(
        &entry,
        &[ExtraFormat::new("text/html", b"<b>plain</b>".to_vec())],
    )
    .unwrap();

    let payload = db.entry_payload(&id).unwrap().unwrap();
    assert_eq!(payload.id, id);
    assert_eq!(payload.entry_hash, entry.hash);
    assert_eq!(payload.formats.len(), 2);
    assert_eq!(payload.formats[0].format_name, "text/plain;charset=utf-8");
    assert_eq!(
        payload.formats[0].body,
        FormatBody::Inline(b"plain".to_vec())
    );
    assert_eq!(payload.formats[1].format_name, "text/html");
}

#[test]
fn text_entry_payload_import_converges_second_database() {
    let dir = tempfile::tempdir().unwrap();
    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();

    let entry = Entry::new_text("sync me".into());
    source
        .insert_multi(
            &entry,
            &[ExtraFormat::new("text/html", b"<p>sync me</p>".to_vec())],
        )
        .unwrap();
    let payload = source.entry_payload(&entry.id).unwrap().unwrap();

    assert!(dest.insert_entry_payload(&payload).unwrap());
    assert!(!dest.insert_entry_payload(&payload).unwrap());
    let copied = dest.get_by_hash(&entry.hash).unwrap().unwrap();
    assert_eq!(copied.content, "sync me");
    assert_eq!(
        dest.entry_payload(&copied.id)
            .unwrap()
            .unwrap()
            .formats
            .len(),
        2
    );
}

#[test]
fn entry_payload_includes_image_blob_chunk() {
    let _guard = DATA_DIR_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    set_data_dir_override(Some(dir.path().to_path_buf())).unwrap();

    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();
    let bytes = b"not actually png, but content-addressed";
    let hash = Entry::compute_hash(bytes);
    Database::store_image_blob(&hash, "png", bytes).unwrap();
    let entry = Entry::new_image(hash.clone(), bytes.len(), "png".to_string());
    let id = entry.id.clone();
    db.insert(&entry).unwrap();

    let payload = db.entry_payload(&id).unwrap().unwrap();
    set_data_dir_override(None).unwrap();

    assert_eq!(payload.formats.len(), 1);
    match &payload.formats[0].body {
        FormatBody::BlobChunk(chunk) => {
            assert_eq!(chunk.blob_hash, hash);
            assert_eq!(chunk.total_bytes, bytes.len() as u64);
            assert_eq!(chunk.offset, 0);
            assert_eq!(chunk.data, bytes);
            assert!(chunk.last);
        }
        other => panic!("expected blob chunk, got {other:?}"),
    }
}

#[test]
fn image_entry_payload_import_converges_second_database() {
    let _guard = DATA_DIR_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    set_data_dir_override(Some(dir.path().to_path_buf())).unwrap();

    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();
    let bytes = b"image bytes for import";
    let hash = Entry::compute_hash(bytes);
    Database::store_image_blob(&hash, "png", bytes).unwrap();
    let entry = Entry::new_image(hash.clone(), bytes.len(), "png".to_string());
    source.insert(&entry).unwrap();

    let payload = source.entry_payload(&entry.id).unwrap().unwrap();
    assert!(dest.insert_entry_payload(&payload).unwrap());
    let copied = dest.get_by_hash(&hash).unwrap().unwrap();
    set_data_dir_override(None).unwrap();

    assert_eq!(copied.entry_type.as_str(), "image");
    assert_eq!(copied.hash, hash);
}

#[test]
fn discovered_peer_preserves_explicit_trust_on_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let key = [9u8; 32];
    let first = db
        .upsert_discovered_peer("laptop", &key, &["192.168.1.10:9001".to_string()])
        .unwrap();
    assert_eq!(first.trust_state, PeerTrustState::Untrusted);
    assert_eq!(first.fingerprint, public_key_fingerprint(&key));
    assert!(db
        .set_peer_trust_state(&first.id, PeerTrustState::Pinned)
        .unwrap());

    let refreshed = db
        .upsert_discovered_peer("renamed", &key, &["192.168.1.10:9002".to_string()])
        .unwrap();
    assert_eq!(refreshed.id, first.id);
    assert_eq!(refreshed.name, "renamed");
    assert_eq!(refreshed.trust_state, PeerTrustState::Pinned);
    assert_eq!(refreshed.addresses, vec!["192.168.1.10:9002"]);

    assert!(db.set_peer_auto_send(&first.id, true).unwrap());
    assert!(db.mark_peer_synced(&first.id).unwrap());
    assert!(
        db.append_sync_log(
            &first.id,
            SyncDirection::Receive,
            Some("entry-1"),
            Some(42),
            SyncStatus::Ok,
            None,
        )
        .unwrap()
            > 0
    );
}
