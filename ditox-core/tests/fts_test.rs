
use ditox_core::db::Database;
use tempfile::TempDir;
use ditox_core::entry::{Entry, EntryType};
use chrono::Utc;

fn create_entry(id: &str, content: &str) -> Entry {
    Entry {
        id: id.to_string(),
        entry_type: EntryType::Text,
        content: content.to_string(),
        hash: format!("hash-{}", id), // Mock hash
        byte_size: content.len(),
        created_at: Utc::now(),
        last_used: Utc::now(),
        favorite: false,
        notes: None,
        collection_id: None,
        image_extension: None,
    }
}

#[test]
fn test_fts_search() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("ditox.db");
    
    // Initialize DB
    let db = Database::open_at(db_path.clone()).unwrap();
    db.init_schema().unwrap();

    // Insert test data
    db.insert(&create_entry("id-1", "rust programming language")).unwrap();
    db.insert(&create_entry("id-2", "python programming")).unwrap();
    db.insert(&create_entry("id-3", "rustacean crab")).unwrap();

    // Search for "rust"
    let results = db.search_entries("rust", 10).unwrap();
    assert_eq!(results.len(), 2, "Should find 2 entries for 'rust'");
    // Verify IDs match expected
    let ids: Vec<String> = results.iter().map(|e| e.id.clone()).collect();
    assert!(ids.contains(&"id-1".to_string()));
    assert!(ids.contains(&"id-3".to_string()));

    // Search for "python"
    let results = db.search_entries("python", 10).unwrap();
    assert_eq!(results.len(), 1, "Should find 1 entry for 'python'");
    assert_eq!(results[0].id, "id-2");
    
    // Partial Match test (prefix)
    let results = db.search_entries("progr", 10).unwrap();
    assert_eq!(results.len(), 2, "Should find 2 entries for 'progr' prefix");

    // Search in notes (update entry with note)
    db.update_notes("id-1", Some("Best language ever")).unwrap();
    // Re-search content "rust" (should still work)
    let results = db.search_entries("rust", 10).unwrap();
    assert_eq!(results.len(), 2);
    
    // Search for note content "best"
    let results = db.search_entries("best", 10).unwrap();
    assert_eq!(results.len(), 1, "Should find entry by note");
    assert_eq!(results[0].id, "id-1");

    // Special Char Test: Colon should not crash
    db.insert(&create_entry("id-path", "C:\\dev\\personal\\ditox")).unwrap();
    match db.search_entries("C:\\dev", 10) {
        Ok(results) => {
            assert_eq!(results.len(), 1, "Should find 'C:\\dev' without crashing");
            assert_eq!(results[0].id, "id-path");
        }
        Err(e) => {
            panic!("Search failed with error: {}", e);
        }
    }
}
