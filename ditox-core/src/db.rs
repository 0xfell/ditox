use crate::collection::Collection;
use crate::entry::{Entry, EntryType};
use crate::error::{DitoxError, Result};
use crate::stats::{Stats, TopEntry};
use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

/// Helper enum for filter query parameters
enum FilterParams<'a> {
    None,
    Type(&'a str),
    Date(String),
    Collection(&'a str),
    Favorite, // No params, just WHERE favorite = 1
}

impl Database {
    pub fn open() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)] // Useful for testing with custom paths
    pub fn open_at(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        Ok(Self { conn })
    }

    fn get_db_path() -> Result<PathBuf> {
        ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.data_dir().join("ditox.db"))
            .ok_or_else(|| DitoxError::Config("Could not determine data directory".into()))
    }

    pub fn get_data_dir() -> Result<PathBuf> {
        ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .ok_or_else(|| DitoxError::Config("Could not determine data directory".into()))
    }

    pub fn get_images_dir() -> Result<PathBuf> {
        Ok(Self::get_data_dir()?.join("images"))
    }

    pub fn init_schema(&self) -> Result<()> {
        // Create table with all columns
        self.conn.execute_batch(
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
        )?;

        // Migration: add last_used column if it doesn't exist (for existing databases)
        self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN last_used TEXT;")
            .ok(); // Ignore error if column already exists

        // Set last_used = created_at for entries where last_used is NULL
        self.conn.execute(
            "UPDATE entries SET last_used = created_at WHERE last_used IS NULL",
            [],
        )?;

        // Migration: add usage_count column for usage statistics
        self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN usage_count INTEGER DEFAULT 0;")
            .ok(); // Ignore error if column already exists

        // Set usage_count = 0 for entries where usage_count is NULL
        self.conn.execute(
            "UPDATE entries SET usage_count = 0 WHERE usage_count IS NULL",
            [],
        )?;

        // Migration: add notes column for entry annotations
        self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN notes TEXT;")
            .ok(); // Ignore error if column already exists

        // Migration: add collection_id column for favorites/collections
        self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN collection_id TEXT;")
            .ok(); // Ignore error if column already exists

        // Create collections table
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                color TEXT,
                keybind TEXT,
                position INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            ",
        )?;

        // Create indexes (after last_used column exists)
        self.conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_created_at ON entries(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_last_used ON entries(last_used DESC);
            CREATE INDEX IF NOT EXISTS idx_hash ON entries(hash);
            CREATE INDEX IF NOT EXISTS idx_collection_id ON entries(collection_id);
            ",
        )?;

        Ok(())
    }

    pub fn insert(&self, entry: &Entry) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entries (id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.entry_type.as_str(),
                entry.content,
                entry.hash,
                entry.byte_size as i64,
                entry.created_at.to_rfc3339(),
                entry.last_used.to_rfc3339(),
                entry.favorite as i32,
                entry.notes,
                entry.collection_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_all(&self, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             ORDER BY last_used DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map([limit as i64], |row| Self::row_to_entry(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries WHERE id = ?1",
        )?;

        let entry = stmt
            .query_row([id], |row| Self::row_to_entry(row))
            .optional()?;

        Ok(entry)
    }

    pub fn get_by_index(&self, index: usize) -> Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             ORDER BY last_used DESC
             LIMIT 1 OFFSET ?1",
        )?;

        let entry = stmt
            .query_row([index as i64], |row| Self::row_to_entry(row))
            .optional()?;

        Ok(entry)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM entries WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    pub fn clear_all(&self) -> Result<usize> {
        let rows = self.conn.execute("DELETE FROM entries", [])?;
        Ok(rows)
    }

    pub fn exists_by_hash(&self, hash: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE hash = ?1",
            [hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn toggle_favorite(&self, id: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE entries SET pinned = NOT pinned WHERE id = ?1",
            [id],
        )?;
        Ok(rows > 0)
    }

    /// Update the last_used timestamp and increment usage_count for an entry (when it's copied)
    pub fn touch(&self, id: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE entries SET last_used = ?1, usage_count = usage_count + 1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(rows > 0)
    }

    pub fn cleanup_old(&self, max_entries: usize) -> Result<usize> {
        // Keep pinned entries, delete least recently used non-pinned entries beyond limit
        let rows = self.conn.execute(
            "DELETE FROM entries WHERE id IN (
                SELECT id FROM entries
                WHERE pinned = 0
                ORDER BY last_used DESC
                LIMIT -1 OFFSET ?1
            )",
            [max_entries as i64],
        )?;
        Ok(rows)
    }

    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Count entries matching a filter type
    pub fn count_filtered(&self, filter: &str, collection_id: Option<&str>) -> Result<usize> {
        let (where_clause, params) = self.build_filter_clause(filter, collection_id);
        let sql = format!("SELECT COUNT(*) FROM entries{}", where_clause);

        let count: i64 = match params {
            FilterParams::None => self.conn.query_row(&sql, [], |row| row.get(0))?,
            FilterParams::Type(t) => self.conn.query_row(&sql, [t], |row| row.get(0))?,
            FilterParams::Date(d) => self.conn.query_row(&sql, [d], |row| row.get(0))?,
            FilterParams::Collection(c) => self.conn.query_row(&sql, [c], |row| row.get(0))?,
            FilterParams::Favorite => self.conn.query_row(&sql, [], |row| row.get(0))?,
        };
        Ok(count as usize)
    }

    /// Get a page of entries with offset and limit (for pagination)
    pub fn get_page(&self, offset: usize, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             ORDER BY last_used DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let entries = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Self::row_to_entry(row)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Search entries using SQL LIKE (case-insensitive prefix for DB-level filtering)
    /// Returns entries matching the query, ordered by last_used
    /// Also searches in notes field
    pub fn search_entries(&self, query: &str, limit: usize) -> Result<Vec<Entry>> {
        // Use LIKE with wildcards for basic substring matching
        // This pre-filters at DB level before in-memory fuzzy matching
        let like_pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));

        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             WHERE content LIKE ?1 ESCAPE '\\' OR notes LIKE ?1 ESCAPE '\\'
             ORDER BY last_used DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![like_pattern, limit as i64], |row| {
                Self::row_to_entry(row)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Search entries using SQL LIKE with additional filtering
    /// Returns entries matching the query and filter, ordered by last_used
    pub fn search_entries_filtered(
        &self,
        query: &str,
        limit: usize,
        filter: &str,
        collection_id: Option<&str>,
    ) -> Result<Vec<Entry>> {
        let like_pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let limit_i64 = limit as i64;

        // Match on filter directly since we need custom SQL for each case (combining filter with LIKE pattern)
        match filter {
            "text" | "image" => {
                let filter_type = filter;
                let mut stmt = self.conn.prepare(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries
                     WHERE entry_type = ?1 AND (content LIKE ?2 ESCAPE '\\' OR notes LIKE ?2 ESCAPE '\\')
                     ORDER BY last_used DESC
                     LIMIT ?3"
                )?;
                let entries = stmt.query_map(params![filter_type, like_pattern, limit_i64], |row| Self::row_to_entry(row))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            "today" => {
                let today = (Utc::now() - Duration::hours(24)).to_rfc3339();
                let mut stmt = self.conn.prepare(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries
                     WHERE created_at > ?1 AND (content LIKE ?2 ESCAPE '\\' OR notes LIKE ?2 ESCAPE '\\')
                     ORDER BY last_used DESC
                     LIMIT ?3"
                )?;
                let entries = stmt.query_map(params![today, like_pattern, limit_i64], |row| Self::row_to_entry(row))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            "collection" if collection_id.is_some() => {
                let cid = collection_id.unwrap();
                let mut stmt = self.conn.prepare(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries
                     WHERE collection_id = ?1 AND (content LIKE ?2 ESCAPE '\\' OR notes LIKE ?2 ESCAPE '\\')
                     ORDER BY last_used DESC
                     LIMIT ?3"
                )?;
                let entries = stmt.query_map(params![cid, like_pattern, limit_i64], |row| Self::row_to_entry(row))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            "favorite" => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries
                     WHERE pinned = 1 AND (content LIKE ?1 ESCAPE '\\' OR notes LIKE ?1 ESCAPE '\\')
                     ORDER BY last_used DESC
                     LIMIT ?2"
                )?;
                let entries = stmt.query_map(params![like_pattern, limit_i64], |row| Self::row_to_entry(row))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            _ => {} // Fall through to "all" case below
        }

        // Default case: "all" filter or unrecognized filter
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             WHERE content LIKE ?1 ESCAPE '\\' OR notes LIKE ?1 ESCAPE '\\'
             ORDER BY last_used DESC
             LIMIT ?2"
        )?;
        let entries = stmt.query_map(params![like_pattern, limit_i64], |row| Self::row_to_entry(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Expects columns in order: id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
    fn row_to_entry(row: &rusqlite::Row) -> std::result::Result<Entry, rusqlite::Error> {
        let entry_type_str: String = row.get(1)?;
        let created_at_str: String = row.get(5)?;
        let last_used_str: Option<String> = row.get(6)?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let last_used = last_used_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(created_at);

        Ok(Entry {
            id: row.get(0)?,
            entry_type: EntryType::from_str(&entry_type_str).unwrap_or(EntryType::Text),
            content: row.get(2)?,
            hash: row.get(3)?,
            byte_size: row.get::<_, i64>(4)? as usize,
            created_at,
            last_used,
            favorite: row.get::<_, i32>(7)? != 0,
            notes: row.get(8)?,
            collection_id: row.get(9)?,
        })
    }

    /// Update notes for an entry
    pub fn update_notes(&self, id: &str, notes: Option<&str>) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE entries SET notes = ?1 WHERE id = ?2",
            params![notes, id],
        )?;
        Ok(rows > 0)
    }

    /// Get top entries by usage count (for quick snippets)
    pub fn get_top_by_usage(&self, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             WHERE usage_count > 0
             ORDER BY usage_count DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map(params![limit as i64], |row| Self::row_to_entry(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get usage statistics
    pub fn get_stats(&self) -> Result<Stats> {
        // Total entries
        let total_entries = self.count()?;

        // Text and image counts
        let text_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE entry_type = 'text'",
            [],
            |row| row.get(0),
        )?;

        let image_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE entry_type = 'image'",
            [],
            |row| row.get(0),
        )?;

        // Favorites count
        let favorites_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE pinned = 1",
            [],
            |row| row.get(0),
        )?;

        // Total usage count
        let total_usage: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(usage_count), 0) FROM entries",
            [],
            |row| row.get(0),
        )?;

        // Top entries by usage
        let mut top_stmt = self.conn.prepare(
            "SELECT id, content, entry_type, usage_count
             FROM entries
             WHERE usage_count > 0
             ORDER BY usage_count DESC
             LIMIT 5",
        )?;

        let top_entries: Vec<TopEntry> = top_stmt
            .query_map([], |row| {
                let content: String = row.get(1)?;
                let entry_type: String = row.get(2)?;
                Ok(TopEntry {
                    id: row.get(0)?,
                    preview: if entry_type == "image" {
                        // For images, show the filename
                        std::path::Path::new(&content)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "[image]".to_string())
                    } else {
                        content.chars().take(50).collect()
                    },
                    entry_type,
                    usage_count: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Copies today, this week, this month
        let now = Utc::now();
        let today_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .to_rfc3339();
        let week_start = (now - Duration::days(7)).to_rfc3339();
        let month_start = (now - Duration::days(30)).to_rfc3339();

        let copies_today: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(usage_count), 0) FROM entries WHERE last_used >= ?1",
            [&today_start],
            |row| row.get(0),
        )?;

        // For week/month we approximate by looking at usage_count of entries used in that period
        // This is not perfectly accurate but gives a good indication
        let copies_week: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(usage_count), 0) FROM entries WHERE last_used >= ?1",
            [&week_start],
            |row| row.get(0),
        )?;

        let copies_month: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(usage_count), 0) FROM entries WHERE last_used >= ?1",
            [&month_start],
            |row| row.get(0),
        )?;

        // Get file sizes
        let db_path = Self::get_db_path()?;
        let db_size_bytes = std::fs::metadata(&db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let images_dir = Self::get_images_dir()?;
        let images_size_bytes = if images_dir.exists() {
            std::fs::read_dir(&images_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .map(|m| m.len())
                        .sum()
                })
                .unwrap_or(0)
        } else {
            0
        };

        Ok(Stats {
            total_entries,
            text_count: text_count as usize,
            image_count: image_count as usize,
            favorites_count: favorites_count as usize,
            db_size_bytes,
            images_size_bytes,
            top_entries,
            copies_today: copies_today as usize,
            copies_week: copies_week as usize,
            copies_month: copies_month as usize,
            total_usage: total_usage as u64,
        })
    }

    // ============= Collection Methods =============

    /// Create a new collection
    pub fn create_collection(&self, collection: &Collection) -> Result<()> {
        self.conn.execute(
            "INSERT INTO collections (id, name, color, keybind, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                collection.id,
                collection.name,
                collection.color,
                collection.keybind.map(|c| c.to_string()),
                collection.position,
                collection.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get all collections ordered by position
    pub fn get_all_collections(&self) -> Result<Vec<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, keybind, position, created_at
             FROM collections
             ORDER BY position ASC, created_at ASC",
        )?;

        let collections = stmt
            .query_map([], |row| Self::row_to_collection(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(collections)
    }

    /// Get a collection by ID
    pub fn get_collection_by_id(&self, id: &str) -> Result<Option<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, keybind, position, created_at
             FROM collections WHERE id = ?1",
        )?;

        let collection = stmt
            .query_row([id], |row| Self::row_to_collection(row))
            .optional()?;

        Ok(collection)
    }

    /// Get a collection by name
    pub fn get_collection_by_name(&self, name: &str) -> Result<Option<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, keybind, position, created_at
             FROM collections WHERE name = ?1",
        )?;

        let collection = stmt
            .query_row([name], |row| Self::row_to_collection(row))
            .optional()?;

        Ok(collection)
    }

    /// Update a collection
    pub fn update_collection(&self, collection: &Collection) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE collections SET name = ?1, color = ?2, keybind = ?3, position = ?4 WHERE id = ?5",
            params![
                collection.name,
                collection.color,
                collection.keybind.map(|c| c.to_string()),
                collection.position,
                collection.id,
            ],
        )?;
        Ok(rows > 0)
    }

    /// Delete a collection (entries in the collection will have their collection_id set to NULL)
    pub fn delete_collection(&self, id: &str) -> Result<bool> {
        // First, unset collection_id for all entries in this collection
        self.conn.execute(
            "UPDATE entries SET collection_id = NULL WHERE collection_id = ?1",
            [id],
        )?;

        // Then delete the collection
        let rows = self.conn.execute(
            "DELETE FROM collections WHERE id = ?1",
            [id],
        )?;
        Ok(rows > 0)
    }

    /// Set or unset collection for an entry
    pub fn set_entry_collection(&self, entry_id: &str, collection_id: Option<&str>) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE entries SET collection_id = ?1 WHERE id = ?2",
            params![collection_id, entry_id],
        )?;
        Ok(rows > 0)
    }

    /// Get entries in a specific collection
    pub fn get_entries_in_collection(&self, collection_id: &str, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
             FROM entries
             WHERE collection_id = ?1
             ORDER BY last_used DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![collection_id, limit as i64], |row| Self::row_to_entry(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Count entries in a collection
    pub fn count_entries_in_collection(&self, collection_id: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE collection_id = ?1",
            [collection_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Build WHERE clause and params for tab filtering
    fn build_filter_clause<'a>(
        &self,
        filter: &'a str,
        collection_id: Option<&'a str>,
    ) -> (String, FilterParams<'a>) {
        match filter {
            "all" => (String::new(), FilterParams::None),
            "text" => (
                " WHERE entry_type = ?1".to_string(),
                FilterParams::Type("text"),
            ),
            "image" => (
                " WHERE entry_type = ?1".to_string(),
                FilterParams::Type("image"),
            ),
            "favorite" => (" WHERE pinned = 1".to_string(), FilterParams::Favorite),
            "today" => {
                let today = (Utc::now() - Duration::hours(24)).to_rfc3339();
                (
                    " WHERE created_at > ?1".to_string(),
                    FilterParams::Date(today),
                )
            }
            "collection" => {
                if let Some(cid) = collection_id {
                    (
                        " WHERE collection_id = ?1".to_string(),
                        FilterParams::Collection(cid),
                    )
                } else {
                    (String::new(), FilterParams::None)
                }
            }
            _ => (String::new(), FilterParams::None),
        }
    }

    /// Get a page of entries with filtering
    pub fn get_page_filtered(
        &self,
        offset: usize,
        limit: usize,
        filter: &str,
        collection_id: Option<&str>,
    ) -> Result<Vec<Entry>> {
        let (where_clause, params) = self.build_filter_clause(filter, collection_id);

        match params {
            FilterParams::None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries
                     ORDER BY last_used DESC
                     LIMIT ?1 OFFSET ?2",
                )?;
                let entries = stmt
                    .query_map(params![limit as i64, offset as i64], |row| {
                        Self::row_to_entry(row)
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(entries)
            }
            FilterParams::Type(t) => {
                let sql = format!(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries{}
                     ORDER BY last_used DESC
                     LIMIT ?2 OFFSET ?3",
                    where_clause
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![t, limit as i64, offset as i64], |row| {
                        Self::row_to_entry(row)
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(entries)
            }
            FilterParams::Date(ref d) => {
                let sql = format!(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries{}
                     ORDER BY last_used DESC
                     LIMIT ?2 OFFSET ?3",
                    where_clause
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![d, limit as i64, offset as i64], |row| {
                        Self::row_to_entry(row)
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(entries)
            }
            FilterParams::Collection(c) => {
                let sql = format!(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries{}
                     ORDER BY last_used DESC
                     LIMIT ?2 OFFSET ?3",
                    where_clause
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![c, limit as i64, offset as i64], |row| {
                        Self::row_to_entry(row)
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(entries)
            }
            FilterParams::Favorite => {
                let sql = format!(
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id
                     FROM entries{}
                     ORDER BY last_used DESC
                     LIMIT ?1 OFFSET ?2",
                    where_clause
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![limit as i64, offset as i64], |row| {
                        Self::row_to_entry(row)
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(entries)
            }
        }
    }

    /// Helper to convert a row to a Collection
    fn row_to_collection(row: &rusqlite::Row) -> std::result::Result<Collection, rusqlite::Error> {
        let created_at_str: String = row.get(5)?;
        let keybind_str: Option<String> = row.get(3)?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Collection {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            keybind: keybind_str.and_then(|s| s.chars().next()),
            position: row.get(4)?,
            created_at,
        })
    }
}
