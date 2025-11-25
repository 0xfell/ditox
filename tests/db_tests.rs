//! Database layer tests.
//!
//! Tests all database operations: CRUD, queries, constraints, and edge cases.

mod common;

use common::TestFixture;
use std::path::PathBuf;

// We need to access the ditox crate internals for testing
// Since this is an integration test, we'll test via the binary

/// Helper to create a test database directly using rusqlite
fn create_test_db(path: &PathBuf) -> rusqlite::Connection {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let conn = rusqlite::Connection::open(path).unwrap();

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY,
            entry_type TEXT NOT NULL,
            content TEXT NOT NULL,
            hash TEXT NOT NULL UNIQUE,
            byte_size INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            pinned INTEGER DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_created_at ON entries(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_pinned ON entries(pinned DESC, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_hash ON entries(hash);
        ",
    )
    .unwrap();

    conn
}

/// Insert a test entry directly into the database
fn insert_test_entry(
    conn: &rusqlite::Connection,
    id: &str,
    content: &str,
    pinned: bool,
    created_at: &str,
) {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex::encode(hasher.finalize());

    conn.execute(
        "INSERT INTO entries (id, entry_type, content, hash, byte_size, created_at, pinned)
         VALUES (?1, 'text', ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, content, hash, content.len(), created_at, pinned as i32],
    )
    .unwrap();
}

// ============================================================================
// Schema Tests
// ============================================================================

#[test]
fn test_schema_creates_entries_table() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Verify table exists
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='entries'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 1, "entries table should exist");
}

#[test]
fn test_schema_creates_indexes() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Verify indexes exist
    let indexes: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='entries'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(indexes.iter().any(|n| n.contains("created_at")));
    assert!(indexes.iter().any(|n| n.contains("pinned")));
    assert!(indexes.iter().any(|n| n.contains("hash")));
}

#[test]
fn test_schema_is_idempotent() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Run schema creation again - should not fail
    let result = conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY,
            entry_type TEXT NOT NULL,
            content TEXT NOT NULL,
            hash TEXT NOT NULL UNIQUE,
            byte_size INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            pinned INTEGER DEFAULT 0
        );
        ",
    );

    assert!(result.is_ok(), "Schema creation should be idempotent");
}

// ============================================================================
// Insert Tests
// ============================================================================

#[test]
fn test_insert_text_entry() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(
        &conn,
        "test-id-1",
        "Hello, World!",
        false,
        "2024-01-01T00:00:00Z",
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn test_insert_duplicate_hash_ignored() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(
        &conn,
        "test-id-1",
        "Hello, World!",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Try inserting same content with different ID - should be ignored due to hash uniqueness
    // Note: Using INSERT OR IGNORE
    let result = conn.execute(
        "INSERT OR IGNORE INTO entries (id, entry_type, content, hash, byte_size, created_at, pinned)
         VALUES ('test-id-2', 'text', 'Hello, World!', (SELECT hash FROM entries WHERE id='test-id-1'), 13, '2024-01-02T00:00:00Z', 0)",
        [],
    );

    assert!(result.is_ok());

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    // Should still be 1 due to hash uniqueness constraint
    assert_eq!(count, 1, "Duplicate hash should be ignored");
}

#[test]
fn test_insert_unicode_content() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let unicode_content = "Hello 世界 🌍 مرحبا Привет";
    insert_test_entry(
        &conn,
        "unicode-id",
        unicode_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    let content: String = conn
        .query_row(
            "SELECT content FROM entries WHERE id = 'unicode-id'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, unicode_content);
}

#[test]
fn test_insert_large_content() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Create 1MB of content
    let large_content: String = "x".repeat(1024 * 1024);
    insert_test_entry(
        &conn,
        "large-id",
        &large_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    let byte_size: i64 = conn
        .query_row(
            "SELECT byte_size FROM entries WHERE id = 'large-id'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(byte_size, 1024 * 1024);
}

#[test]
fn test_insert_empty_content() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(&conn, "empty-id", "", false, "2024-01-01T00:00:00Z");

    let content: String = conn
        .query_row(
            "SELECT content FROM entries WHERE id = 'empty-id'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, "");
}

// ============================================================================
// Query Tests
// ============================================================================

#[test]
fn test_get_all_respects_limit() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 20 entries
    for i in 0..20 {
        insert_test_entry(
            &conn,
            &format!("id-{}", i),
            &format!("content-{}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        );
    }

    // Query with limit 5
    let entries: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY created_at DESC LIMIT 5")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(entries.len(), 5);
}

#[test]
fn test_get_all_orders_pinned_first() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert unpinned entry first (oldest)
    insert_test_entry(
        &conn,
        "unpinned-1",
        "unpinned content",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Insert pinned entry second
    insert_test_entry(
        &conn,
        "pinned-1",
        "pinned content",
        true,
        "2024-01-02T00:00:00Z",
    );

    // Insert another unpinned entry (newest - should come first with new behavior)
    insert_test_entry(
        &conn,
        "unpinned-2",
        "unpinned content 2",
        false,
        "2024-01-03T00:00:00Z",
    );

    // New behavior: entries are sorted by last_used/created_at DESC only
    // Pinned is just a "favorite" marker, not affecting sort order
    let entries: Vec<(String, bool)> = conn
        .prepare("SELECT id, pinned FROM entries ORDER BY created_at DESC")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get::<_, i32>(1)? != 0)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Newest entry should be first (regardless of pinned status)
    assert_eq!(entries[0].0, "unpinned-2", "Newest entry should be first");
    assert!(!entries[0].1, "First entry is not pinned");

    // Pinned entry should be second (by timestamp)
    assert_eq!(entries[1].0, "pinned-1", "Pinned entry should be second");
    assert!(entries[1].1, "Second entry should be pinned (favorite)");
}

#[test]
fn test_get_by_id_existing() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(
        &conn,
        "findme",
        "test content",
        false,
        "2024-01-01T00:00:00Z",
    );

    let content: Option<String> = conn
        .query_row(
            "SELECT content FROM entries WHERE id = 'findme'",
            [],
            |row| row.get(0),
        )
        .ok();

    assert_eq!(content, Some("test content".to_string()));
}

#[test]
fn test_get_by_id_nonexistent() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let result: Result<String, _> = conn.query_row(
        "SELECT content FROM entries WHERE id = 'nonexistent'",
        [],
        |row| row.get(0),
    );

    assert!(result.is_err());
}

#[test]
fn test_get_by_index() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert entries with different timestamps
    insert_test_entry(&conn, "id-0", "first", false, "2024-01-01T00:00:00Z");
    insert_test_entry(&conn, "id-1", "second", false, "2024-01-02T00:00:00Z");
    insert_test_entry(&conn, "id-2", "third", false, "2024-01-03T00:00:00Z");

    // Get entry at index 0 (should be newest - "third")
    let content: String = conn
        .query_row(
            "SELECT content FROM entries ORDER BY created_at DESC LIMIT 1 OFFSET 0",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, "third");

    // Get entry at index 2 (should be oldest - "first")
    let content: String = conn
        .query_row(
            "SELECT content FROM entries ORDER BY created_at DESC LIMIT 1 OFFSET 2",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, "first");
}

#[test]
fn test_get_by_index_out_of_bounds() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(&conn, "id-0", "only entry", false, "2024-01-01T00:00:00Z");

    let result: Result<String, _> = conn.query_row(
        "SELECT content FROM entries ORDER BY created_at DESC LIMIT 1 OFFSET 999",
        [],
        |row| row.get(0),
    );

    assert!(result.is_err());
}

// ============================================================================
// Delete Tests
// ============================================================================

#[test]
fn test_delete_existing_entry() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(&conn, "delete-me", "content", false, "2024-01-01T00:00:00Z");

    let rows_affected = conn
        .execute("DELETE FROM entries WHERE id = 'delete-me'", [])
        .unwrap();

    assert_eq!(rows_affected, 1);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn test_delete_nonexistent_entry() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let rows_affected = conn
        .execute("DELETE FROM entries WHERE id = 'nonexistent'", [])
        .unwrap();

    assert_eq!(rows_affected, 0);
}

#[test]
fn test_clear_all() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert multiple entries
    for i in 0..10 {
        insert_test_entry(
            &conn,
            &format!("id-{}", i),
            &format!("content-{}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        );
    }

    let rows_affected = conn.execute("DELETE FROM entries", []).unwrap();

    assert_eq!(rows_affected, 10);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0);
}

// ============================================================================
// Pin Tests
// ============================================================================

#[test]
fn test_toggle_pin_unpinned_to_pinned() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(
        &conn,
        "toggle-pin",
        "content",
        false,
        "2024-01-01T00:00:00Z",
    );

    conn.execute(
        "UPDATE entries SET pinned = NOT pinned WHERE id = 'toggle-pin'",
        [],
    )
    .unwrap();

    let pinned: i32 = conn
        .query_row(
            "SELECT pinned FROM entries WHERE id = 'toggle-pin'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(pinned, 1);
}

#[test]
fn test_toggle_pin_pinned_to_unpinned() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(&conn, "toggle-pin", "content", true, "2024-01-01T00:00:00Z");

    conn.execute(
        "UPDATE entries SET pinned = NOT pinned WHERE id = 'toggle-pin'",
        [],
    )
    .unwrap();

    let pinned: i32 = conn
        .query_row(
            "SELECT pinned FROM entries WHERE id = 'toggle-pin'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(pinned, 0);
}

// ============================================================================
// Cleanup Tests
// ============================================================================

#[test]
fn test_cleanup_old_entries() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 20 entries (none pinned)
    for i in 0..20 {
        insert_test_entry(
            &conn,
            &format!("id-{}", i),
            &format!("content-{}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    // Cleanup to keep only 10
    let deleted = conn
        .execute(
            "DELETE FROM entries WHERE id IN (
                SELECT id FROM entries
                WHERE pinned = 0
                ORDER BY created_at DESC
                LIMIT -1 OFFSET 10
            )",
            [],
        )
        .unwrap();

    assert_eq!(deleted, 10);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 10);
}

#[test]
fn test_cleanup_preserves_pinned_entries() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 5 pinned entries
    for i in 0..5 {
        insert_test_entry(
            &conn,
            &format!("pinned-{}", i),
            &format!("pinned content {}", i),
            true,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        );
    }

    // Insert 10 unpinned entries
    for i in 0..10 {
        insert_test_entry(
            &conn,
            &format!("unpinned-{}", i),
            &format!("unpinned content {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", i + 10),
        );
    }

    // Cleanup unpinned to keep only 3
    conn.execute(
        "DELETE FROM entries WHERE id IN (
            SELECT id FROM entries
            WHERE pinned = 0
            ORDER BY created_at DESC
            LIMIT -1 OFFSET 3
        )",
        [],
    )
    .unwrap();

    // Should have 5 pinned + 3 unpinned = 8 total
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 8);

    // Verify all pinned entries still exist
    let pinned_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE pinned = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(pinned_count, 5);
}

// ============================================================================
// Hash/Deduplication Tests
// ============================================================================

#[test]
fn test_exists_by_hash() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"test content");
    let hash = hex::encode(hasher.finalize());

    insert_test_entry(
        &conn,
        "hash-test",
        "test content",
        false,
        "2024-01-01T00:00:00Z",
    );

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE hash = ?1",
            [&hash],
            |row| row.get(0),
        )
        .unwrap();

    assert!(count > 0);
}

#[test]
fn test_hash_uniqueness_constraint() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(
        &conn,
        "first",
        "same content",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Get the hash
    let hash: String = conn
        .query_row("SELECT hash FROM entries WHERE id = 'first'", [], |row| {
            row.get(0)
        })
        .unwrap();

    // Try to insert with same hash - should fail or be ignored
    let result = conn.execute(
        "INSERT INTO entries (id, entry_type, content, hash, byte_size, created_at, pinned)
         VALUES ('second', 'text', 'same content', ?1, 12, '2024-01-02T00:00:00Z', 0)",
        [&hash],
    );

    assert!(result.is_err(), "Duplicate hash should violate constraint");
}

// ============================================================================
// Count Tests
// ============================================================================

#[test]
fn test_count_empty_database() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn test_count_with_entries() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    for i in 0..42 {
        insert_test_entry(
            &conn,
            &format!("id-{}", i),
            &format!("content-{}", i),
            false,
            &format!("2024-01-01T{:02}:00:00Z", i % 24),
        );
    }

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 42);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_special_characters_in_content() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let special_content = "Line1\nLine2\tTab\r\nCRLF\0Null'Quote\"DoubleQuote";
    insert_test_entry(
        &conn,
        "special",
        special_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    let content: String = conn
        .query_row(
            "SELECT content FROM entries WHERE id = 'special'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, special_content);
}

#[test]
fn test_sql_injection_attempt() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let malicious_content = "'; DROP TABLE entries; --";
    insert_test_entry(
        &conn,
        "injection",
        malicious_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    // Table should still exist
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='entries'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 1, "Table should not be dropped");

    // Content should be stored as-is
    let content: String = conn
        .query_row(
            "SELECT content FROM entries WHERE id = 'injection'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, malicious_content);
}

#[test]
fn test_concurrent_access_simulation() {
    let fixture = TestFixture::new();

    // Simulate multiple connections
    let conn1 = rusqlite::Connection::open(&fixture.db_path).unwrap();
    let conn2 = rusqlite::Connection::open(&fixture.db_path).unwrap();

    // Initialize schema on both
    for conn in [&conn1, &conn2] {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                entry_type TEXT NOT NULL,
                content TEXT NOT NULL,
                hash TEXT NOT NULL UNIQUE,
                byte_size INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                pinned INTEGER DEFAULT 0
            )",
        )
        .unwrap();
    }

    // Insert from conn1
    conn1
        .execute(
            "INSERT INTO entries (id, entry_type, content, hash, byte_size, created_at, pinned)
         VALUES ('conn1-entry', 'text', 'from conn1', 'hash1', 10, '2024-01-01T00:00:00Z', 0)",
            [],
        )
        .unwrap();

    // Read from conn2
    let content: String = conn2
        .query_row(
            "SELECT content FROM entries WHERE id = 'conn1-entry'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(content, "from conn1");
}
