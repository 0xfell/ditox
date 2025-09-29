#![cfg(feature = "libsql")]
use ditox_core::sync::SyncEngine;
use tempfile::tempdir;

#[test]
fn sync_run_smoke_if_env() {
    let url = match std::env::var("TURSO_URL") {
        Ok(u) => u,
        Err(_) => return,
    };
    let token = std::env::var("TURSO_AUTH_TOKEN").ok();
    let dir = tempdir().unwrap();
    let db = dir.path().join("local.db");
    // Create engine (adds local schema)
    let engine =
        SyncEngine::new(&db, Some(&url), token.as_deref(), Some("test-device"), 50).unwrap();
    // Write some local data via raw sqlite
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute("INSERT INTO clips(id, kind, text, created_at, is_favorite, updated_at, lamport, device_id) VALUES('e2e1','text','hello',strftime('%s','now'),0,strftime('%s','now'),1,'test-device')", []).unwrap();
    // Push
    let _rep = engine.run(false, true /*pull_only*/).unwrap_or_default();
    // Push only if needed
    let _ = engine.run(false, false);
    // Status should be available
    let _ = engine.status().unwrap();
}
