use chrono::Utc;
use ditox_core::db::Database;
use ditox_core::entry::{Entry, EntryType};
use std::time::Instant;
use tempfile::TempDir;

fn random_string(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    // Simple PRNG since we don't have rand crate
    let mut rng = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let mut output = String::with_capacity(len);
    for _ in 0..len {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (rng % CHARSET.len() as u128) as usize;
        output.push(CHARSET[idx] as char);
    }
    output
}

fn create_random_entry(i: usize) -> Entry {
    let content = format!("Entry {} - {}", i, random_string(50));
    Entry {
        id: format!("id-{}", i),
        entry_type: EntryType::Text,
        content: content.clone(),
        hash: format!("hash-{}", i),
        byte_size: content.len(),
        created_at: Utc::now(),
        last_used: Utc::now(),
        favorite: i.is_multiple_of(100), // 1% favorites
        notes: if i.is_multiple_of(50) {
            Some("Special note".to_string())
        } else {
            None
        },
        collection_id: None,
        image_extension: None,
    }
}

#[test]
fn benchmark_fts_queries() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("ditox.db");

    println!("Creating benchmark database at {:?}", db_path);
    let db = Database::open_at(db_path.clone()).unwrap();

    // 1. Initialize Schema (includes creating FTS table)
    let start = Instant::now();
    db.init_schema().unwrap();
    // Ensure triggers are present for manual FTS sync in this test setup if necessary
    // init_schema sets them up, so we are good.
    println!("Schema init time: {:?}", start.elapsed());

    // 2. Insert 5000 entries
    println!("Inserting 100000 entries...");
    let start_insert = Instant::now();
    {
        let mut conn = rusqlite::Connection::open(&db_path).unwrap();
        let tx = conn.transaction().unwrap();

        // Manual bulk insert to verify FTS sync performance under load
        // But since Database wrapper uses single connection and we want to test its logic...
        // We'll use the DB wrapper but maybe in a loop.
        // Actually for speed let's do batch insert via raw connection to setup data fast,
        // THEN test search. BUT we need triggers to fire.
        // Triggers fire on INSERT into entries.

        for i in 0..100000 {
            let e = create_random_entry(i);
            tx.execute(
                "INSERT INTO entries (id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    e.id,
                    "text",
                    e.content,
                    e.hash,
                    e.byte_size as i64,
                    e.created_at.to_rfc3339(),
                    e.last_used.to_rfc3339(),
                    e.favorite as i32,
                    e.notes,
                    e.collection_id
                ],
            ).unwrap();
        }
        tx.commit().unwrap();
    }
    println!(
        "Insert time (5000 entries + FTS triggers): {:?}",
        start_insert.elapsed()
    );

    // 3. Benchmark Searches
    println!("\n=== Search Benchmark Results ===");

    // Case A: Common substring "Entry" (matches everything)
    let start = Instant::now();
    let results = db.search_entries("Entry", 50).unwrap();
    println!(
        "Search 'Entry' (matches ~all): {:?} (found {})",
        start.elapsed(),
        results.len()
    );

    // Case B: Random prefix
    let start = Instant::now();
    let results = db.search_entries("Abc", 50).unwrap();
    println!(
        "Search 'Abc' (random prefix): {:?} (found {})",
        start.elapsed(),
        results.len()
    );

    // Case C: Specific ID-like string
    let start = Instant::now();
    let results = db.search_entries("Entry 4999", 50).unwrap();
    println!(
        "Search 'Entry 4999' (specific): {:?} (found {})",
        start.elapsed(),
        results.len()
    );

    // Case D: Filtered Search (Favorites)
    let start = Instant::now();
    let results = db
        .search_entries_filtered("Entry", 50, "favorite", None)
        .unwrap();
    println!(
        "Search Filtered 'Entry' + Favorite: {:?} (found {})",
        start.elapsed(),
        results.len()
    );

    // Case E: Search in Notes
    let start = Instant::now();
    let results = db.search_entries("Special note", 50).unwrap();
    println!(
        "Search Notes 'Special note': {:?} (found {})",
        start.elapsed(),
        results.len()
    );

    println!("================================");
}
