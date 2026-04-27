use ditox_core::db::{set_data_dir_override, ExtraFormat};
use ditox_core::sync::{
    public_key_fingerprint, AdvertisedPeer, DiscoveryBackend, FormatBody, InMemoryDiscovery,
    LocalIdentity, NoiseSession, PeerTrustState, SyncDirection, SyncStatus,
};
use ditox_core::Collection;
use ditox_core::{Database, Entry};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread;

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
fn discovered_peers_can_be_ingested_from_backend() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();
    let discovery = InMemoryDiscovery::shared();
    let first = AdvertisedPeer::new("desk", [4u8; 32], "127.0.0.1:9001");
    let second = AdvertisedPeer::new("laptop", [5u8; 32], "127.0.0.1:9002");
    discovery.advertise(first.clone()).unwrap();
    discovery.advertise(second.clone()).unwrap();

    let peers = db.ingest_discovered_peers(&discovery).unwrap();
    let stored = db.list_peers().unwrap();

    assert_eq!(peers.len(), 2);
    assert_eq!(stored.len(), 2);
    assert!(stored
        .iter()
        .any(|peer| peer.fingerprint == first.fingerprint));
    assert!(stored
        .iter()
        .any(|peer| peer.fingerprint == second.fingerprint));
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
fn entry_payload_import_preserves_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();

    let collection = Collection::new("Work".to_string());
    source.create_collection(&collection).unwrap();
    let mut entry = Entry::new_text("metadata".into());
    entry.notes = Some("note from source".to_string());
    entry.collection_id = Some(collection.id.clone());
    entry.favorite = true;
    source.insert(&entry).unwrap();
    source
        .add_tag_to_entry_by_name(&entry.id, "urgent", None)
        .unwrap();
    source
        .add_tag_to_entry_by_name(&entry.id, "client", None)
        .unwrap();

    let payload = source.entry_payload(&entry.id).unwrap().unwrap();
    assert_eq!(payload.notes.as_deref(), Some("note from source"));
    assert_eq!(payload.collection.as_deref(), Some("Work"));
    assert_eq!(
        payload.tags,
        vec!["client".to_string(), "urgent".to_string()]
    );

    assert!(dest.insert_entry_payload(&payload).unwrap());
    let copied = dest.get_by_hash(&entry.hash).unwrap().unwrap();
    let copied_collection = dest
        .get_collection_by_id(copied.collection_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    let copied_tags: Vec<_> = dest
        .get_tags_for_entry(&copied.id)
        .unwrap()
        .into_iter()
        .map(|tag| tag.name)
        .collect();

    assert!(copied.favorite);
    assert_eq!(copied.notes.as_deref(), Some("note from source"));
    assert_eq!(copied_collection.name, "Work");
    assert_eq!(
        copied_tags,
        vec!["client".to_string(), "urgent".to_string()]
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
fn entry_payload_splits_large_image_blob_into_chunks() {
    let _guard = DATA_DIR_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    set_data_dir_override(Some(dir.path().to_path_buf())).unwrap();

    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();
    let bytes = vec![42_u8; ditox_core::sync::BLOB_CHUNK_BYTES + 7];
    let hash = Entry::compute_hash(&bytes);
    Database::store_image_blob(&hash, "png", &bytes).unwrap();
    let entry = Entry::new_image(hash.clone(), bytes.len(), "png".to_string());
    db.insert(&entry).unwrap();

    let payload = db.entry_payload(&entry.id).unwrap().unwrap();
    set_data_dir_override(None).unwrap();

    match &payload.formats[0].body {
        FormatBody::BlobChunks(chunks) => {
            assert_eq!(chunks.len(), 2);
            assert_eq!(chunks[0].offset, 0);
            assert_eq!(chunks[0].data.len(), ditox_core::sync::BLOB_CHUNK_BYTES);
            assert!(!chunks[0].last);
            assert_eq!(chunks[1].offset, ditox_core::sync::BLOB_CHUNK_BYTES as u64);
            assert_eq!(chunks[1].data.len(), 7);
            assert!(chunks[1].last);
        }
        other => panic!("expected chunked image body, got {other:?}"),
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
fn pull_from_database_converges_one_hundred_text_entries() {
    let dir = tempfile::tempdir().unwrap();
    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();

    for i in 0..100 {
        source
            .insert(&Entry::new_text(format!("entry {i:03}")))
            .unwrap();
    }

    let summary = dest.pull_from_database(&source, 100).unwrap();
    assert_eq!(summary.remote_digests, 100);
    assert_eq!(summary.requested_entries, 100);
    assert_eq!(summary.imported_entries, 100);
    assert_eq!(summary.skipped_entries, 0);
    assert_eq!(dest.count().unwrap(), 100);

    let noop = dest.pull_from_database(&source, 100).unwrap();
    assert_eq!(noop.requested_entries, 0);
    assert_eq!(noop.imported_entries, 0);
}

#[test]
fn pull_from_database_converges_image_entry() {
    let _guard = DATA_DIR_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    set_data_dir_override(Some(dir.path().to_path_buf())).unwrap();

    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();
    let bytes = b"image pull bytes";
    let hash = Entry::compute_hash(bytes);
    Database::store_image_blob(&hash, "png", bytes).unwrap();
    source
        .insert(&Entry::new_image(
            hash.clone(),
            bytes.len(),
            "png".to_string(),
        ))
        .unwrap();

    let summary = dest.pull_from_database(&source, 10).unwrap();
    let copied = dest.get_by_hash(&hash).unwrap().unwrap();
    set_data_dir_override(None).unwrap();

    assert_eq!(summary.imported_entries, 1);
    assert_eq!(copied.entry_type.as_str(), "image");
}

#[test]
fn pull_from_noise_session_converges_text_entries() {
    let dir = tempfile::tempdir().unwrap();
    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();
    let source_identity = LocalIdentity::generate();
    let dest_identity = LocalIdentity::generate();
    let source_peer = dest
        .upsert_discovered_peer(
            "source",
            &source_identity.public_key(),
            &["127.0.0.1:9001".to_string()],
        )
        .unwrap();
    dest.set_peer_trust_state(&source_peer.id, PeerTrustState::Pinned)
        .unwrap();
    let dest_peer = source
        .upsert_discovered_peer(
            "dest",
            &dest_identity.public_key(),
            &["127.0.0.1:9002".to_string()],
        )
        .unwrap();
    source
        .set_peer_trust_state(&dest_peer.id, PeerTrustState::Pinned)
        .unwrap();

    for i in 0..4 {
        source
            .insert(&Entry::new_text(format!("transport entry {i}")))
            .unwrap();
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut session = NoiseSession::responder(&mut stream, &source_identity).unwrap();
        source
            .authenticate_sync_responder(&mut session, &mut stream, &source_identity)
            .unwrap();
        for _ in 0..5 {
            source
                .serve_one_sync_message(&mut session, &mut stream)
                .unwrap();
        }
    });

    let mut stream = TcpStream::connect(addr).unwrap();
    let mut session = NoiseSession::initiator(&mut stream, &dest_identity).unwrap();
    dest.authenticate_sync_initiator(&mut session, &mut stream, &dest_identity)
        .unwrap();
    let summary = dest
        .pull_from_sync_session(&mut session, &mut stream, 10)
        .unwrap();

    server.join().unwrap();
    assert_eq!(summary.remote_digests, 4);
    assert_eq!(summary.requested_entries, 4);
    assert_eq!(summary.imported_entries, 4);
    assert_eq!(dest.count().unwrap(), 4);
}

#[test]
fn trusted_peer_check_requires_pinned_state() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();
    let identity = LocalIdentity::generate();
    let peer = db
        .upsert_discovered_peer(
            "peer",
            &identity.public_key(),
            &["127.0.0.1:9001".to_string()],
        )
        .unwrap();

    assert!(db.require_pinned_peer(&identity.public_key()).is_err());
    db.set_peer_trust_state(&peer.id, PeerTrustState::Pinned)
        .unwrap();
    assert_eq!(
        db.require_pinned_peer(&identity.public_key()).unwrap().id,
        peer.id
    );
    db.set_peer_trust_state(&peer.id, PeerTrustState::Rejected)
        .unwrap();
    assert!(db.require_pinned_peer(&identity.public_key()).is_err());
}

#[test]
fn pull_from_trusted_address_converges_text_entries() {
    let dir = tempfile::tempdir().unwrap();
    let source = Database::open_at(dir.path().join("source.db")).unwrap();
    let dest = Database::open_at(dir.path().join("dest.db")).unwrap();
    source.init_schema().unwrap();
    dest.init_schema().unwrap();
    let source_identity = LocalIdentity::generate();
    let dest_identity = LocalIdentity::generate();

    for i in 0..3 {
        source
            .insert(&Entry::new_text(format!("trusted address entry {i}")))
            .unwrap();
    }

    let source_peer = dest
        .upsert_discovered_peer(
            "source",
            &source_identity.public_key(),
            &["127.0.0.1:9001".to_string()],
        )
        .unwrap();
    dest.set_peer_trust_state(&source_peer.id, PeerTrustState::Pinned)
        .unwrap();
    let dest_peer = source
        .upsert_discovered_peer(
            "dest",
            &dest_identity.public_key(),
            &["127.0.0.1:9002".to_string()],
        )
        .unwrap();
    source
        .set_peer_trust_state(&dest_peer.id, PeerTrustState::Pinned)
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        source
            .serve_trusted_sync_connection(&mut stream, &source_identity, 4)
            .unwrap();
    });

    let summary = dest
        .pull_from_trusted_address(addr, &dest_identity, 10)
        .unwrap();

    server.join().unwrap();
    assert_eq!(summary.imported_entries, 3);
    assert_eq!(dest.count().unwrap(), 3);
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

#[test]
fn peer_lookup_accepts_id_and_fingerprint_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("ditox.db")).unwrap();
    db.init_schema().unwrap();

    let key = [2u8; 32];
    let peer = db
        .upsert_discovered_peer("tablet", &key, &["192.168.1.11:9001".to_string()])
        .unwrap();

    assert_eq!(db.get_peer(&peer.id).unwrap().unwrap().id, peer.id);
    assert_eq!(
        db.get_peer(&peer.fingerprint[..8]).unwrap().unwrap().id,
        peer.id
    );
}
