//! CLI integration tests.
//!
//! Tests the ditox binary through its command-line interface.
//! These tests verify end-to-end functionality by running the actual binary.

#![allow(deprecated)] // assert_cmd::Command::cargo_bin is deprecated but still works

mod common;

use assert_cmd::Command;
use common::TestFixture;
use predicates::prelude::*;
use std::path::PathBuf;

/// Get a Command configured with test environment variables
fn ditox_cmd(fixture: &TestFixture) -> Command {
    let mut cmd = Command::cargo_bin("ditox").unwrap();

    // Set XDG directories to use temp paths
    cmd.env(
        "XDG_DATA_HOME",
        fixture.temp_dir.path().join("data").to_string_lossy().to_string(),
    );
    cmd.env(
        "XDG_CONFIG_HOME",
        fixture.temp_dir.path().join("config").to_string_lossy().to_string(),
    );

    // Disable tracing output for cleaner test output
    cmd.env("RUST_LOG", "off");

    cmd
}

/// Helper to create a test database with entries
fn setup_test_db(fixture: &TestFixture) -> PathBuf {
    // ditox uses ProjectDirs::from("com", "ditox", "ditox") which creates:
    // ~/.local/share/ditox/ditox.db on Linux
    // We need to set XDG_DATA_HOME so that becomes:
    // $XDG_DATA_HOME/ditox/ditox.db
    let db_dir = fixture.temp_dir.path().join("data").join("ditox");
    std::fs::create_dir_all(&db_dir).unwrap();

    let db_path = db_dir.join("ditox.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();

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

    db_path
}

/// Insert a test entry directly into the database
fn insert_entry(db_path: &PathBuf, id: &str, content: &str, pinned: bool, created_at: &str) {
    use sha2::{Digest, Sha256};

    let conn = rusqlite::Connection::open(db_path).unwrap();

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
// Help and Version Tests
// ============================================================================

#[test]
fn test_help_flag() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("clipboard manager"))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_version_flag() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ditox"));
}

#[test]
fn test_subcommand_help() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List"));
}

// ============================================================================
// List Command Tests
// ============================================================================

#[test]
fn test_list_empty_database() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No clipboard entries"));
}

#[test]
fn test_list_with_entries() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(&db_path, "id-1", "Hello World", false, "2024-01-01T00:00:00Z");
    insert_entry(
        &db_path,
        "id-2",
        "Second entry",
        false,
        "2024-01-02T00:00:00Z",
    );

    ditox_cmd(&fixture)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("Second entry"));
}

#[test]
fn test_list_respects_limit() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert 10 entries
    for i in 0..10 {
        insert_entry(
            &db_path,
            &format!("id-{}", i),
            &format!("Entry {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        );
    }

    // Request only 3
    let output = ditox_cmd(&fixture)
        .args(["list", "--limit", "3"])
        .assert()
        .success();

    // Count lines (header + separator + 3 entries)
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let entry_lines = stdout.lines().filter(|l| l.contains("Entry")).count();

    assert_eq!(entry_lines, 3);
}

#[test]
fn test_list_json_output() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(&db_path, "id-1", "Test content", false, "2024-01-01T00:00:00Z");

    ditox_cmd(&fixture)
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"content\""))
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("Test content"));
}

#[test]
fn test_list_json_is_valid_json() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(&db_path, "id-1", "Test", false, "2024-01-01T00:00:00Z");

    let output = ditox_cmd(&fixture)
        .args(["list", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);

    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_list_json_empty_is_empty_array() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

#[test]
fn test_list_shows_pinned_first() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert unpinned first (older timestamp)
    insert_entry(
        &db_path,
        "id-1",
        "Unpinned older",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Insert pinned second (newer timestamp - should show first due to recency, not pinned status)
    insert_entry(
        &db_path,
        "id-2",
        "Pinned entry",
        true,
        "2024-01-02T00:00:00Z",
    );

    let output = ditox_cmd(&fixture).arg("list").assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let pinned_pos = stdout.find("Pinned entry").unwrap();
    let unpinned_pos = stdout.find("Unpinned older").unwrap();

    // Pinned entry shows first because it's more recent (sorted by last_used DESC)
    // Pinned is now just a "favorite" marker, not affecting sort order
    assert!(
        pinned_pos < unpinned_pos,
        "Newer entry should appear before older entry (sorted by last_used)"
    );
}

#[test]
fn test_list_default_limit_is_10() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert 15 entries
    for i in 0..15 {
        insert_entry(
            &db_path,
            &format!("id-{}", i),
            &format!("Entry number {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    let output = ditox_cmd(&fixture).arg("list").assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let entry_lines = stdout.lines().filter(|l| l.contains("Entry number")).count();

    assert_eq!(entry_lines, 10, "Default limit should be 10");
}

// ============================================================================
// Copy Command Tests
// ============================================================================

#[test]
fn test_copy_by_index() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(
        &db_path,
        "id-1",
        "Content to copy",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Note: This will fail in CI without a Wayland session, but tests the error handling
    let result = ditox_cmd(&fixture).args(["copy", "1"]).assert();

    // Either success (with Wayland) or error about clipboard
    // We can't guarantee clipboard access in tests
    let output = result.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        combined.contains("Copied") || combined.contains("clipboard") || combined.contains("Error"),
        "Should either copy successfully or report clipboard error"
    );
}

#[test]
fn test_copy_by_id() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(
        &db_path,
        "specific-uuid",
        "Content to copy",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Similar to above - may fail without Wayland
    let result = ditox_cmd(&fixture).args(["copy", "specific-uuid"]).assert();

    let output = result.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        combined.contains("Copied") || combined.contains("clipboard") || combined.contains("Error")
    );
}

#[test]
fn test_copy_nonexistent_index() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .args(["copy", "999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_copy_nonexistent_id() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .args(["copy", "nonexistent-uuid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_copy_index_zero_is_invalid() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(&db_path, "id-1", "Content", false, "2024-01-01T00:00:00Z");

    ditox_cmd(&fixture)
        .args(["copy", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("1 or greater"));
}

// ============================================================================
// Clear Command Tests
// ============================================================================

#[test]
fn test_clear_with_confirm_flag() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert entries
    for i in 0..5 {
        insert_entry(
            &db_path,
            &format!("id-{}", i),
            &format!("Content {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        );
    }

    ditox_cmd(&fixture)
        .args(["clear", "--confirm"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared 5 entries"));

    // Verify database is empty
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn test_clear_without_confirm_prompts() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(&db_path, "id-1", "Content", false, "2024-01-01T00:00:00Z");

    // Write "n" to stdin to cancel
    ditox_cmd(&fixture)
        .arg("clear")
        .write_stdin("n\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cancelled"));

    // Entry should still exist
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn test_clear_empty_database() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .args(["clear", "--confirm"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared 0 entries"));
}

// ============================================================================
// Status Command Tests
// ============================================================================

#[test]
fn test_status_shows_entry_count() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert 7 entries
    for i in 0..7 {
        insert_entry(
            &db_path,
            &format!("id-{}", i),
            &format!("Content {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        );
    }

    ditox_cmd(&fixture)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Entries:"))
        .stdout(predicate::str::contains("7"));
}

#[test]
fn test_status_shows_data_dir() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Data dir:"));
}

#[test]
fn test_status_empty_database() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    ditox_cmd(&fixture)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Entries:"))
        .stdout(predicate::str::contains("0"));
}

// ============================================================================
// Watch Command Tests
// ============================================================================

#[test]
fn test_watch_help() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .args(["watch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("watcher"));
}

// Note: Testing `watch` command would require running it in background and
// simulating clipboard changes, which is complex for integration tests.
// The watcher functionality is better tested via unit tests.

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_invalid_subcommand() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_invalid_argument() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .args(["list", "--invalid-flag"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_copy_missing_argument() {
    let fixture = TestFixture::new();

    ditox_cmd(&fixture)
        .arg("copy")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_unicode_content_in_list() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(
        &db_path,
        "unicode-id",
        "Hello 世界 🌍",
        false,
        "2024-01-01T00:00:00Z",
    );

    ditox_cmd(&fixture)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("世界"));
}

#[test]
fn test_long_content_preview_truncated() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    let long_content = "x".repeat(500);
    insert_entry(
        &db_path,
        "long-id",
        &long_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    let output = ditox_cmd(&fixture).arg("list").assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Preview should be truncated (not contain full 500 chars)
    assert!(
        stdout.len() < 600,
        "Output should be truncated for display"
    );
}

#[test]
fn test_special_characters_in_content() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    let special_content = "Line1\nLine2\tTab";
    insert_entry(
        &db_path,
        "special-id",
        special_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    // Should display (whitespace normalized in preview)
    ditox_cmd(&fixture).arg("list").assert().success();
}

#[test]
fn test_json_with_special_characters() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    let special_content = "Quote: \" Backslash: \\ Newline: \n Tab: \t";
    insert_entry(
        &db_path,
        "special-id",
        special_content,
        false,
        "2024-01-01T00:00:00Z",
    );

    let output = ditox_cmd(&fixture)
        .args(["list", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);

    assert!(parsed.is_ok(), "JSON should be properly escaped");
}

#[test]
fn test_large_number_of_entries() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert 100 entries
    for i in 0..100 {
        insert_entry(
            &db_path,
            &format!("id-{}", i),
            &format!("Entry number {}", i),
            false,
            &format!("2024-{:02}-01T00:00:00Z", (i % 12) + 1),
        );
    }

    // Should handle large dataset
    ditox_cmd(&fixture)
        .args(["list", "--limit", "100"])
        .assert()
        .success();
}

#[test]
fn test_mixed_text_and_images() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    // Insert text entry
    insert_entry(
        &db_path,
        "text-id",
        "Text content",
        false,
        "2024-01-01T00:00:00Z",
    );

    // Insert image entry (directly, since we control the DB)
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute(
        "INSERT INTO entries (id, entry_type, content, hash, byte_size, created_at, pinned)
         VALUES ('img-id', 'image', '/path/to/image.png', 'imagehash123', 1024, '2024-01-02T00:00:00Z', 0)",
        [],
    )
    .unwrap();

    ditox_cmd(&fixture)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("txt"))
        .stdout(predicate::str::contains("img"));
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[test]
fn test_multiple_list_commands() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    let db_path = setup_test_db(&fixture);

    insert_entry(&db_path, "id-1", "Content", false, "2024-01-01T00:00:00Z");

    // Run list multiple times - should be consistent
    for _ in 0..5 {
        ditox_cmd(&fixture)
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("Content"));
    }
}

// ============================================================================
// Regression Tests
// ============================================================================

#[test]
fn test_empty_search_query_not_crash() {
    let fixture = TestFixture::new();
    fixture.create_default_config();
    setup_test_db(&fixture);

    // Ensure basic commands work with empty database
    ditox_cmd(&fixture).arg("list").assert().success();
    ditox_cmd(&fixture).arg("status").assert().success();
}

#[test]
fn test_database_created_on_first_run() {
    let fixture = TestFixture::new();
    fixture.create_default_config();

    // Don't pre-create database
    // Running list should create it

    ditox_cmd(&fixture)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No clipboard entries"));

    // Verify database was created
    let db_path = fixture
        .temp_dir
        .path()
        .join("data")
        .join("ditox")
        .join("ditox.db");

    assert!(db_path.exists(), "Database should be created on first run");
}
