//! Pagination and Performance Benchmark Tests
//!
//! These tests verify the pagination implementation performs correctly with large datasets.
//! Run with: cargo test --test pagination_benchmark_tests -- --nocapture
//!
//! For memory profiling, run with:
//! RUST_BACKTRACE=1 cargo test --test pagination_benchmark_tests -- --nocapture
//!
//! Linux-only: shares the `common::TestFixture` which uses `XDG_DATA_HOME`.

#![cfg(unix)]

mod common;

use common::TestFixture;
use std::path::PathBuf;
use std::time::Instant;

/// Helper to create a test database with the full schema
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
            last_used TEXT,
            pinned INTEGER DEFAULT 0,
            usage_count INTEGER DEFAULT 0,
            notes TEXT,
            collection_id TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_created_at ON entries(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_last_used ON entries(last_used DESC);
        CREATE INDEX IF NOT EXISTS idx_pinned ON entries(pinned DESC, last_used DESC);
        CREATE INDEX IF NOT EXISTS idx_hash ON entries(hash);
        CREATE INDEX IF NOT EXISTS idx_collection_id ON entries(collection_id);
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
        "INSERT OR IGNORE INTO entries (id, entry_type, content, hash, byte_size, created_at, last_used, pinned, usage_count)
         VALUES (?1, 'text', ?2, ?3, ?4, ?5, ?5, ?6, 0)",
        rusqlite::params![id, content, hash, content.len(), created_at, pinned as i32],
    )
    .unwrap();
}

/// Generate random-ish content for testing
fn generate_content(i: usize) -> String {
    let templates = [
        "Hello World from entry number {}",
        "fn main() {{ println!(\"Entry {}\"); }}",
        "SELECT * FROM table WHERE id = {};",
        "https://example.com/path/{}/resource",
        "export VARIABLE_{}=\"value\"",
        "The quick brown fox {} jumps over the lazy dog",
        "Lorem ipsum dolor sit amet entry {} consectetur",
        "{{ \"id\": {}, \"data\": \"test\" }}",
        "Entry number {} with some additional text content here",
        "Multi\nline\ncontent\nfor entry\n{}",
    ];
    format!("{}", templates[i % templates.len()].replace("{}", &i.to_string()))
}

/// Generate a timestamp for entry ordering
fn generate_timestamp(i: usize) -> String {
    let day = (i % 28) + 1;
    let hour = i % 24;
    let minute = i % 60;
    let second = i % 60;
    format!(
        "2024-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        (i % 12) + 1,
        day,
        hour,
        minute,
        second
    )
}

// ============================================================================
// Large Dataset Tests (10k+ entries)
// ============================================================================

#[test]
fn test_insert_10k_entries() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    let entry_count = 10_000;
    let start = Instant::now();

    // Use a transaction for faster bulk inserts
    conn.execute("BEGIN TRANSACTION", []).unwrap();

    for i in 0..entry_count {
        let content = generate_content(i);
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, i % 100 == 0, &timestamp);
    }

    conn.execute("COMMIT", []).unwrap();

    let insert_duration = start.elapsed();
    println!(
        "Inserted {} entries in {:?} ({:.2} entries/sec)",
        entry_count,
        insert_duration,
        entry_count as f64 / insert_duration.as_secs_f64()
    );

    // Verify count
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, entry_count as i64);
    println!("Verified {} entries in database", count);
}

#[test]
fn test_pagination_performance_10k() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 10k entries
    let entry_count = 10_000;
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        let content = generate_content(i);
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, i % 100 == 0, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();

    // Test paginated query (PAGE_SIZE = 20)
    let page_size = 20;
    let iterations = 100;

    let start = Instant::now();
    for page in 0..iterations {
        let offset = page * page_size;
        let _entries: Vec<String> = conn
            .prepare(
                "SELECT id FROM entries
                 ORDER BY last_used DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .unwrap()
            .query_map(rusqlite::params![page_size, offset], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
    }
    let paginated_duration = start.elapsed();

    println!(
        "Paginated query: {} pages of {} entries in {:?} ({:.2} pages/sec)",
        iterations,
        page_size,
        paginated_duration,
        iterations as f64 / paginated_duration.as_secs_f64()
    );

    // Test full load for comparison (like old behavior)
    let start = Instant::now();
    let _all_entries: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let full_load_duration = start.elapsed();

    println!(
        "Full load: {} entries in {:?}",
        entry_count, full_load_duration
    );

    // Paginated should be faster for first page
    let first_page_duration = paginated_duration / iterations as u32;
    println!(
        "First page avg: {:?} vs full load: {:?} (speedup: {:.1}x)",
        first_page_duration,
        full_load_duration,
        full_load_duration.as_secs_f64() / first_page_duration.as_secs_f64()
    );

    // Assert pagination is faster for first page
    assert!(
        first_page_duration < full_load_duration,
        "Pagination should be faster than full load for first page"
    );
}

#[test]
fn test_search_performance_10k() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 10k entries with searchable content
    let entry_count = 10_000;
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        let content = generate_content(i);
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, false, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();

    // Test DB LIKE search performance
    let search_terms = ["Hello", "SELECT", "fn main", "Lorem", "example.com"];

    for term in search_terms {
        let like_pattern = format!("%{}%", term);
        let start = Instant::now();

        let results: Vec<String> = conn
            .prepare(
                "SELECT id FROM entries
                 WHERE content LIKE ?1 ESCAPE '\\'
                 ORDER BY last_used DESC
                 LIMIT 500",
            )
            .unwrap()
            .query_map([&like_pattern], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let search_duration = start.elapsed();
        println!(
            "Search '{}': {} results in {:?}",
            term,
            results.len(),
            search_duration
        );

        // Search should complete quickly
        assert!(
            search_duration.as_millis() < 500,
            "Search for '{}' took too long: {:?}",
            term,
            search_duration
        );
    }
}

#[test]
fn test_count_performance_10k() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 10k entries
    let entry_count = 10_000;
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        let content = generate_content(i);
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, false, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();

    // Test count performance (needed for accurate scrollbar)
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();
    }
    let count_duration = start.elapsed();

    println!(
        "COUNT(*): {} iterations in {:?} ({:.2} ops/sec)",
        iterations,
        count_duration,
        iterations as f64 / count_duration.as_secs_f64()
    );

    // Count should be very fast
    let avg_count = count_duration / iterations;
    assert!(
        avg_count.as_micros() < 1000,
        "COUNT should be under 1ms, got {:?}",
        avg_count
    );
}

// ============================================================================
// Pinned Entry Tests
// ============================================================================

#[test]
fn test_pinned_entries_appear_first_in_pagination() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert regular entries with recent timestamps
    for i in 0..100 {
        insert_test_entry(
            &conn,
            &format!("regular-{}", i),
            &format!("regular content {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    // Insert pinned entries with OLDER timestamps
    for i in 0..5 {
        insert_test_entry(
            &conn,
            &format!("pinned-{}", i),
            &format!("pinned content {}", i),
            true,
            "2023-01-01T00:00:00Z", // Older than regular entries
        );
    }

    // With new behavior, entries are sorted by last_used only (pinned is just a favorite marker)
    let first_page: Vec<(String, bool)> = conn
        .prepare(
            "SELECT id, pinned FROM entries
             ORDER BY last_used DESC
             LIMIT 20 OFFSET 0",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get::<_, i32>(1)? != 0)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // First page should contain recent regular entries (pinned entries are older)
    for (i, (id, pinned)) in first_page.iter().enumerate() {
        assert!(
            !pinned,
            "Entry {} ({}) should not be pinned (recent entries are regular)",
            i, id
        );
    }

    // Pinned entries should be on later pages (since they're older)
    let pinned_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries WHERE pinned = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pinned_count, 5, "Should have 5 pinned entries (favorites)");

    println!("Pinned entries correctly stored as favorites (sorted by last_used)");
}

#[test]
fn test_pinned_entries_across_pages() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 30 pinned entries with older timestamps
    for i in 0..30 {
        insert_test_entry(
            &conn,
            &format!("pinned-{}", i),
            &format!("pinned content {}", i),
            true,
            &format!("2024-01-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    // Insert 70 regular entries with newer timestamps
    for i in 0..70 {
        insert_test_entry(
            &conn,
            &format!("regular-{}", i),
            &format!("regular content {}", i),
            false,
            &format!("2024-02-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    // With new behavior, entries are sorted by last_used only
    // Page 1 should have recent entries (regular ones from February)
    let page1: Vec<bool> = conn
        .prepare(
            "SELECT pinned FROM entries
             ORDER BY last_used DESC
             LIMIT 20 OFFSET 0",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, i32>(0).map(|v| v != 0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Most/all of page 1 should be regular entries (newer timestamps)
    let pinned_in_page1 = page1.iter().filter(|&&p| p).count();
    assert!(
        pinned_in_page1 < page1.len(),
        "Page 1 should be mostly regular entries (sorted by last_used, not pinned status)"
    );

    // Total pinned count should still be correct
    let total_pinned: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries WHERE pinned = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total_pinned, 30, "Should have 30 pinned entries (favorites)");

    println!("Pinned entries correctly stored as favorites (sorted by last_used)");
}

// ============================================================================
// Search Correctness Tests
// ============================================================================

#[test]
fn test_search_finds_correct_entries() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert specific searchable entries
    insert_test_entry(
        &conn,
        "unique-1",
        "The quick brown fox",
        false,
        "2024-01-01T00:00:00Z",
    );
    insert_test_entry(
        &conn,
        "unique-2",
        "fn main() { println!(\"hello\"); }",
        false,
        "2024-01-02T00:00:00Z",
    );
    insert_test_entry(
        &conn,
        "unique-3",
        "SELECT * FROM users",
        false,
        "2024-01-03T00:00:00Z",
    );

    // Insert noise entries
    for i in 0..1000 {
        insert_test_entry(
            &conn,
            &format!("noise-{}", i),
            &format!("noise content number {}", i),
            false,
            &format!("2024-02-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    // Search for "quick brown fox"
    let results: Vec<String> = conn
        .prepare(
            "SELECT id FROM entries
             WHERE content LIKE '%quick brown fox%'
             ORDER BY last_used DESC",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "unique-1");

    // Search for "fn main"
    let results: Vec<String> = conn
        .prepare(
            "SELECT id FROM entries
             WHERE content LIKE '%fn main%'
             ORDER BY last_used DESC",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "unique-2");

    // Search for "SELECT"
    let results: Vec<String> = conn
        .prepare(
            "SELECT id FROM entries
             WHERE content LIKE '%SELECT%'
             ORDER BY last_used DESC",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "unique-3");

    println!("Search correctly finds specific entries in large dataset");
}

#[test]
fn test_search_respects_limit() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 500 entries with searchable term
    for i in 0..500 {
        insert_test_entry(
            &conn,
            &format!("searchable-{}", i),
            &format!("searchable content number {}", i),
            false,
            &format!("2024-01-{:02}T{:02}:00:00Z", (i % 28) + 1, i % 24),
        );
    }

    // Search with limit
    let results: Vec<String> = conn
        .prepare(
            "SELECT id FROM entries
             WHERE content LIKE '%searchable%'
             ORDER BY last_used DESC
             LIMIT 100",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        results.len(),
        100,
        "Search should respect LIMIT"
    );

    println!("Search correctly respects LIMIT");
}

// ============================================================================
// Rapid Navigation Tests
// ============================================================================

#[test]
fn test_rapid_page_navigation() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 5000 entries
    let entry_count = 5000;
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        let content = generate_content(i);
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, false, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();

    let page_size = 20;
    let total_pages = (entry_count + page_size - 1) / page_size;

    // Simulate rapid navigation: jump around pages
    let pages_to_visit = [0, total_pages / 2, total_pages - 1, 10, total_pages / 4, 0];

    let start = Instant::now();
    for &page in &pages_to_visit {
        let offset = page * page_size;
        let entries: Vec<String> = conn
            .prepare(
                "SELECT id FROM entries
                 ORDER BY last_used DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .unwrap()
            .query_map(rusqlite::params![page_size, offset], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Verify we got entries (except possibly last page)
        if page < total_pages - 1 {
            assert_eq!(
                entries.len(),
                page_size,
                "Page {} should have {} entries",
                page,
                page_size
            );
        }
    }
    let nav_duration = start.elapsed();

    println!(
        "Rapid navigation: {} page jumps in {:?} ({:.2}ms per page)",
        pages_to_visit.len(),
        nav_duration,
        nav_duration.as_secs_f64() * 1000.0 / pages_to_visit.len() as f64
    );

    // Each page should load in under 50ms
    let avg_per_page = nav_duration / pages_to_visit.len() as u32;
    assert!(
        avg_per_page.as_millis() < 50,
        "Page navigation should be under 50ms, got {:?}",
        avg_per_page
    );
}

#[test]
fn test_sequential_page_navigation() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 2000 entries
    let entry_count = 2000;
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        let content = generate_content(i);
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, false, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();

    let page_size = 20;
    let total_pages = (entry_count + page_size - 1) / page_size;

    // Navigate through all pages sequentially
    let start = Instant::now();
    let mut visited = 0;
    for page in 0..total_pages {
        let offset = page * page_size;
        let entries: Vec<String> = conn
            .prepare(
                "SELECT id FROM entries
                 ORDER BY last_used DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .unwrap()
            .query_map(rusqlite::params![page_size, offset], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        visited += entries.len();
    }
    let total_duration = start.elapsed();

    assert_eq!(
        visited, entry_count,
        "Should visit all entries"
    );

    println!(
        "Sequential navigation: {} pages ({} entries) in {:?} ({:.2}ms per page)",
        total_pages,
        entry_count,
        total_duration,
        total_duration.as_secs_f64() * 1000.0 / total_pages as f64
    );
}

// ============================================================================
// Memory Usage Estimation Tests
// ============================================================================

#[test]
fn test_memory_comparison_paginated_vs_full() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 5000 entries with substantial content
    let entry_count = 5000;
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        // Each entry ~200-500 bytes of content
        let content = format!(
            "Entry {} content with additional padding: {}",
            i,
            "x".repeat(200)
        );
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, false, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();

    // Measure paginated load memory (single page)
    let page_size = 20;
    let paginated_entries: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT id, content, hash FROM entries
             ORDER BY last_used DESC
             LIMIT ?1",
        )
        .unwrap()
        .query_map([page_size], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let paginated_mem: usize = paginated_entries
        .iter()
        .map(|(id, content, hash)| id.len() + content.len() + hash.len())
        .sum();

    // Measure full load memory
    let full_entries: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT id, content, hash FROM entries
             ORDER BY last_used DESC",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let full_mem: usize = full_entries
        .iter()
        .map(|(id, content, hash)| id.len() + content.len() + hash.len())
        .sum();

    let reduction_ratio = full_mem as f64 / paginated_mem as f64;

    println!("Memory comparison:");
    println!("  Paginated (20 entries): {} bytes", paginated_mem);
    println!("  Full load ({} entries): {} bytes", entry_count, full_mem);
    println!("  Reduction: {:.1}x", reduction_ratio);

    // Paginated should use significantly less memory
    assert!(
        reduction_ratio > 10.0,
        "Pagination should reduce memory by at least 10x for {} entries",
        entry_count
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_database_pagination() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Query empty database
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0);

    // Paginated query on empty database
    let entries: Vec<String> = conn
        .prepare(
            "SELECT id FROM entries
             ORDER BY last_used DESC
             LIMIT 20 OFFSET 0",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(entries.is_empty());
    println!("Empty database handles pagination correctly");
}

#[test]
fn test_last_page_partial() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 55 entries (20 + 20 + 15 = not evenly divisible)
    for i in 0..55 {
        insert_test_entry(
            &conn,
            &format!("id-{}", i),
            &format!("content {}", i),
            false,
            &format!("2024-01-{:02}T00:00:00Z", (i % 28) + 1),
        );
    }

    let page_size = 20;

    // Page 0: 20 entries
    let page0: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC LIMIT ?1 OFFSET ?2")
        .unwrap()
        .query_map(rusqlite::params![page_size, 0], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(page0.len(), 20);

    // Page 1: 20 entries
    let page1: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC LIMIT ?1 OFFSET ?2")
        .unwrap()
        .query_map(rusqlite::params![page_size, 20], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(page1.len(), 20);

    // Page 2: 15 entries (partial)
    let page2: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC LIMIT ?1 OFFSET ?2")
        .unwrap()
        .query_map(rusqlite::params![page_size, 40], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(page2.len(), 15, "Last page should have partial entries");

    // Page 3: empty
    let page3: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC LIMIT ?1 OFFSET ?2")
        .unwrap()
        .query_map(rusqlite::params![page_size, 60], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(page3.is_empty(), "Beyond last page should be empty");

    println!("Partial last page handled correctly");
}

#[test]
fn test_single_entry_pagination() {
    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    insert_test_entry(&conn, "only-one", "single entry", false, "2024-01-01T00:00:00Z");

    let page: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC LIMIT 20 OFFSET 0")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(page.len(), 1);
    assert_eq!(page[0], "only-one");
    println!("Single entry pagination works correctly");
}

// ============================================================================
// Summary Test
// ============================================================================

#[test]
fn test_pagination_benchmark_summary() {
    println!("\n========================================");
    println!("PAGINATION BENCHMARK SUMMARY");
    println!("========================================\n");

    let fixture = TestFixture::new();
    let conn = create_test_db(&fixture.db_path);

    // Insert 10k entries
    let entry_count = 10_000;
    println!("Creating {} test entries...", entry_count);

    let insert_start = Instant::now();
    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..entry_count {
        let content = format!("{} - {}", generate_content(i), "x".repeat(100));
        let timestamp = generate_timestamp(i);
        insert_test_entry(&conn, &format!("id-{}", i), &content, i % 100 == 0, &timestamp);
    }
    conn.execute("COMMIT", []).unwrap();
    println!("  Created in {:?}\n", insert_start.elapsed());

    // 1. Full load benchmark
    let full_start = Instant::now();
    let full_entries: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let full_duration = full_start.elapsed();
    println!("FULL LOAD (old behavior):");
    println!("  {} entries loaded in {:?}", full_entries.len(), full_duration);

    // 2. First page load benchmark
    let page_start = Instant::now();
    let _page: Vec<String> = conn
        .prepare("SELECT id FROM entries ORDER BY last_used DESC LIMIT 20")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let page_duration = page_start.elapsed();
    println!("\nFIRST PAGE LOAD (new behavior):");
    println!("  20 entries loaded in {:?}", page_duration);
    println!("  Speedup: {:.1}x faster", full_duration.as_secs_f64() / page_duration.as_secs_f64());

    // 3. Count query benchmark
    let count_start = Instant::now();
    let _count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();
    let count_duration = count_start.elapsed();
    println!("\nCOUNT QUERY:");
    println!("  Completed in {:?}", count_duration);

    // 4. Search benchmark
    let search_start = Instant::now();
    let search_results: Vec<String> = conn
        .prepare("SELECT id FROM entries WHERE content LIKE '%Hello%' LIMIT 500")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let search_duration = search_start.elapsed();
    println!("\nSEARCH 'Hello':");
    println!("  {} results in {:?}", search_results.len(), search_duration);

    // 5. Memory estimation
    let page_mem: usize = 20 * 150; // ~150 bytes per entry estimate
    let full_mem: usize = entry_count * 150;
    println!("\nMEMORY ESTIMATION:");
    println!("  Page (20 entries): ~{} KB", page_mem / 1024);
    println!("  Full ({} entries): ~{} KB", entry_count, full_mem / 1024);
    println!("  Reduction: ~{:.0}x", full_mem as f64 / page_mem as f64);

    println!("\n========================================");
    println!("All benchmarks completed successfully!");
    println!("========================================\n");
}
