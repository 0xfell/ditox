use crate::collection::Collection;
use crate::entry::{Entry, EntryType};
use crate::error::{DitoxError, Result};
use crate::stats::{Stats, TopEntry};
use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Current schema version. Bumped whenever `init_schema` grows a new migration
/// step. Historical values:
/// - 0: pre-versioning (legacy image layout `{timestamp}_{hash_prefix}.ext`)
/// - 1: content-addressed image layout + `entries.image_extension` column
pub const SCHEMA_VERSION: i64 = 1;

/// Subdirectory inside `images/` that holds files quarantined by
/// `ditox repair --fix-hashes` because their on-disk hash didn't match the
/// DB hash. We don't auto-delete — the user may want to inspect them.
pub const QUARANTINE_DIR: &str = ".quarantine";

/// File extension used while a blob is being written before atomic rename.
pub const TMP_SUFFIX: &str = ".tmp";

/// Age threshold (seconds) above which leftover `.tmp` files are considered
/// abandoned and swept at startup. 60 s is comfortably longer than any write
/// we do (large screenshots complete in milliseconds even on slow disks).
pub const TMP_SWEEP_AGE_SECS: u64 = 60;

pub struct Database {
    conn: Connection,
}

/// True if a path ends with our tmp suffix. We include PID in real tmp names
/// (`…{pid}.tmp`) so a simple `ends_with(".tmp")` check covers both the
/// current and any legacy one-off tmp layouts.
fn is_tmp_leftover(path: &Path) -> bool {
    path.to_string_lossy().ends_with(TMP_SUFFIX)
}

/// Inspect a single directory entry and, if it's a stale `.tmp` file older
/// than `cutoff`, unlink it. Swallows IO errors — we'll simply retry next
/// startup, and the volume of stale tmps is bounded by process crashes.
fn sweep_one(entry: &std::fs::DirEntry, cutoff: std::time::SystemTime) {
    let path = entry.path();
    if !is_tmp_leftover(&path) {
        return;
    }
    // Use symlink_metadata so a pathological symlink doesn't lead us to
    // remove an unrelated file.
    let meta = match path.symlink_metadata() {
        Ok(m) => m,
        Err(_) => return,
    };
    if !meta.is_file() {
        return;
    }
    let modified = match meta.modified() {
        Ok(m) => m,
        Err(_) => return,
    };
    if modified < cutoff {
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::debug!("could not sweep stale tmp {}: {}", path.display(), e);
        } else {
            tracing::debug!("swept stale tmp {}", path.display());
        }
    }
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

    /// Resolve the absolute path for a content-addressed image blob.
    ///
    /// Layout: `images/{hash[..2]}/{hash}.{extension}`. The 2-char prefix
    /// directory fans the tree out so even pathological users with tens of
    /// thousands of images keep each subdirectory under a few hundred files.
    pub fn image_path(hash: &str, extension: &str) -> Result<PathBuf> {
        if hash.len() < 2 {
            return Err(DitoxError::Other(format!("invalid image hash: {}", hash)));
        }
        let base = Self::get_images_dir()?;
        Ok(base
            .join(&hash[..2])
            .join(format!("{}.{}", hash, extension)))
    }

    /// Content-addressed write.
    ///
    /// - If the destination already exists: no-op, returns `(path, false)`.
    /// - Otherwise writes to `{path}.tmp`, `fsync`s, then atomically
    ///   renames into place and `fsync`s the parent directory so the rename
    ///   itself is durable. On kill-mid-write the leftover `.tmp` is
    ///   reclaimed by [`sweep_stale_tmp_files`] on next startup.
    /// - Safe under concurrent writers: if another process wins the rename
    ///   race we observe `AlreadyExists`-like behaviour (the destination
    ///   now exists) and quietly clean up our `.tmp`.
    pub fn store_image_blob(hash: &str, extension: &str, bytes: &[u8]) -> Result<(PathBuf, bool)> {
        let path = Self::image_path(hash, extension)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if path.exists() {
            return Ok((path, false));
        }

        // Temp file name includes our PID so two concurrent writers can't
        // clobber each other's tmp on the way to the same destination.
        let tmp_name = format!(
            "{}.{}{}",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("blob"),
            std::process::id(),
            TMP_SUFFIX
        );
        let tmp_path = path.with_file_name(tmp_name);

        {
            let mut f = std::fs::File::create(&tmp_path)?;
            f.write_all(bytes)?;
            f.sync_all()?; // durable contents before rename
        }

        match std::fs::rename(&tmp_path, &path) {
            Ok(()) => {
                // Make the rename itself durable. Best-effort: on some
                // filesystems (e.g. tmpfs) this is a no-op, which is fine.
                if let Some(parent) = path.parent() {
                    if let Ok(dir) = std::fs::File::open(parent) {
                        let _ = dir.sync_all();
                    }
                }
                Ok((path, true))
            }
            Err(e) => {
                // Rename failed — either someone else materialised the
                // destination in the meantime, or something else is wrong.
                let _ = std::fs::remove_file(&tmp_path);
                if path.exists() {
                    Ok((path, false))
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Queue `(hash, extension)` for pruning. Call this inside the same
    /// transaction that deletes the entry row so a crash between row-gone
    /// and file-gone doesn't leak a blob. [`drain_pending_blob_prunes`]
    /// runs on every startup and processes the queue.
    fn queue_blob_prune_tx(tx: &rusqlite::Transaction, hash: &str, extension: &str) -> Result<()> {
        tx.execute(
            "INSERT OR IGNORE INTO pending_blob_prunes (hash, extension, queued_at)
             VALUES (?1, ?2, ?3)",
            params![hash, extension, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Process any queued prunes. For each (hash, extension):
    /// - If no image entry still references the hash, delete the blob and
    ///   pop the queue row.
    /// - If something still references it (shouldn't happen today since
    ///   `hash` is UNIQUE, but future-proof), leave the queue entry alone
    ///   so it's reconsidered next startup.
    pub(crate) fn drain_pending_blob_prunes(&self) {
        let mut stmt = match self
            .conn
            .prepare("SELECT hash, extension FROM pending_blob_prunes")
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("could not read pending_blob_prunes: {}", e);
                return;
            }
        };
        let rows: Vec<(String, String)> =
            match stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?))) {
                Ok(it) => it.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    tracing::warn!("could not iterate pending_blob_prunes: {}", e);
                    return;
                }
            };
        drop(stmt);

        for (hash, extension) in rows {
            let refcount: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM entries WHERE entry_type = 'image' AND hash = ?1",
                    [&hash],
                    |r| r.get(0),
                )
                .unwrap_or(1);
            if refcount > 0 {
                // Still referenced — leave queued, try again next startup.
                continue;
            }
            if let Ok(path) = Self::image_path(&hash, &extension) {
                match std::fs::remove_file(&path) {
                    Ok(()) => {
                        tracing::debug!("pruned orphan blob {}", path.display());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => {
                        tracing::warn!("could not remove orphan blob {}: {}", path.display(), e);
                        // Leave in queue for retry.
                        continue;
                    }
                }
            }
            let _ = self.conn.execute(
                "DELETE FROM pending_blob_prunes WHERE hash = ?1 AND extension = ?2",
                params![&hash, &extension],
            );
        }
    }

    /// Sweep leftover `.tmp` files older than [`TMP_SWEEP_AGE_SECS`].
    /// These only appear after a process was killed mid-write.
    pub(crate) fn sweep_stale_tmp_files(&self) {
        let base = match Self::get_images_dir() {
            Ok(b) => b,
            Err(_) => return,
        };
        if !base.exists() {
            return;
        }
        let cutoff =
            std::time::SystemTime::now() - std::time::Duration::from_secs(TMP_SWEEP_AGE_SECS);

        // Walk top-level + fan-out subdirectories.
        let Ok(entries) = std::fs::read_dir(&base) else {
            return;
        };
        for top in entries.flatten() {
            let meta = match top.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                if let Ok(inner) = std::fs::read_dir(top.path()) {
                    for e in inner.flatten() {
                        sweep_one(&e, cutoff);
                    }
                }
            } else {
                sweep_one(&top, cutoff);
            }
        }
    }

    /// All image files actually on disk. Includes subdir fan-out, excludes
    /// quarantine and `.tmp` leftovers. Result is keyed by full path.
    pub fn scan_image_files(&self) -> Result<Vec<PathBuf>> {
        let base = Self::get_images_dir()?;
        let mut out = Vec::new();
        if !base.exists() {
            return Ok(out);
        }
        for top in std::fs::read_dir(&base)?.flatten() {
            // Use symlink_metadata so a user-placed symlink doesn't get
            // us to delete the target.
            let meta = match top.path().symlink_metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                // Skip quarantine
                if top.file_name() == QUARANTINE_DIR {
                    continue;
                }
                for e in std::fs::read_dir(top.path())?.flatten() {
                    let p = e.path();
                    let meta = match p.symlink_metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    if meta.is_file() && !is_tmp_leftover(&p) {
                        out.push(p);
                    }
                }
            } else if meta.is_file() && !is_tmp_leftover(&top.path()) {
                out.push(top.path());
            }
        }
        Ok(out)
    }

    /// The set of `(hash, extension)` pairs referenced by live image rows.
    pub fn referenced_image_blobs(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, COALESCE(image_extension, 'png') FROM entries WHERE entry_type = 'image'",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every image row paired with its resolved on-disk path. Does NOT
    /// check whether the file exists — callers filter as they need.
    pub fn image_rows_with_paths(&self) -> Result<Vec<(String, String, String, PathBuf)>> {
        // Returns: (id, hash, extension, path)
        let mut stmt = self.conn.prepare(
            "SELECT id, hash, COALESCE(image_extension, 'png') FROM entries WHERE entry_type = 'image'",
        )?;
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for (id, hash, ext) in rows {
            let path = Self::image_path(&hash, &ext)?;
            out.push((id, hash, ext, path));
        }
        Ok(out)
    }

    pub fn init_schema(&self) -> Result<()> {
        // Schema version table — must exist before we make any decisions
        // about migrations.
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

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

        // Migration (schema v1): add image_extension column. For image rows
        // this stores just the extension ("png", "jpg", …); `content` becomes
        // the bare content-addressable hash. For text rows the column stays
        // NULL and `content` keeps its original meaning.
        self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN image_extension TEXT;")
            .ok();

        // Pending blob-prune queue. Deletion sites (delete/cleanup_old/
        // clear_all) insert into this queue inside the same SQL transaction
        // that removes the row, so a crash between row-delete and file-delete
        // leaves work for the next startup instead of leaking a blob.
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS pending_blob_prunes (
                hash TEXT NOT NULL,
                extension TEXT NOT NULL,
                queued_at TEXT NOT NULL,
                PRIMARY KEY (hash, extension)
            );
            ",
        )?;

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

        // FTS5 Setup
        // Create virtual table for full-text search
        // We include id to map back to the main table
        self.conn.execute_batch(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(id UNINDEXED, content, notes);
            ",
        )?;

        // Triggers to keep FTS index in sync
        self.conn.execute_batch(
            "
            -- Insert trigger
            CREATE TRIGGER IF NOT EXISTS entries_ai AFTER INSERT ON entries BEGIN
                INSERT INTO entries_fts(id, content, notes) VALUES (new.id, new.content, new.notes);
            END;

            -- Delete trigger
            CREATE TRIGGER IF NOT EXISTS entries_ad AFTER DELETE ON entries BEGIN
                DELETE FROM entries_fts WHERE id = old.id;
            END;

            -- Update trigger
            CREATE TRIGGER IF NOT EXISTS entries_au AFTER UPDATE ON entries BEGIN
                DELETE FROM entries_fts WHERE id = old.id;
                INSERT INTO entries_fts(id, content, notes) VALUES (new.id, new.content, new.notes);
            END;
            ",
        )?;

        // Population migration:
        // Check if FTS table is empty but main table is not, implying we need to populate it
        let fts_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entries_fts", [], |row| row.get(0))
            .unwrap_or(0);

        let entries_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap_or(0);

        if fts_count == 0 && entries_count > 0 {
            tracing::info!(
                "Populating FTS index for {} existing entries...",
                entries_count
            );
            self.conn.execute(
                "INSERT INTO entries_fts(id, content, notes) SELECT id, content, notes FROM entries",
                [],
            )?;
        }

        // Content-addressed image store migration (legacy layout -> v1).
        let current_version = self.read_schema_version().unwrap_or(0);
        if current_version < 1 {
            self.migrate_image_store_to_v1()?;
            self.write_schema_version(1)?;
        }

        // Startup reconciliation: drain any pending prunes left from a crash,
        // and sweep stale `.tmp` files from previous aborted writes. Cheap
        // (milliseconds) and self-healing on every open.
        self.drain_pending_blob_prunes();
        self.sweep_stale_tmp_files();

        Ok(())
    }

    fn read_schema_version(&self) -> Option<i64> {
        self.conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| s.parse().ok())
    }

    fn write_schema_version(&self, version: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO schema_meta (key, value) VALUES ('version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [version.to_string()],
        )?;
        Ok(())
    }

    /// Walk every image row. For rows still in the legacy layout
    /// (`content = /…/images/{ts}_{prefix}.ext`), copy the file to the new
    /// content-addressed path (`images/{hh}/{full_hash}.{ext}`) using
    /// `store_image_blob` (idempotent), update the row so `content = hash`
    /// and `image_extension = ext`, and remove the legacy file only if the
    /// target is readable and hashes match.
    ///
    /// Safe to re-run: rows already in the new layout (`image_extension`
    /// populated AND `content` is a 64-hex hash) are skipped.
    fn migrate_image_store_to_v1(&self) -> Result<()> {
        // Snapshot first so we don't trip over our own UPDATEs.
        let mut stmt = self.conn.prepare(
            "SELECT id, hash, content, image_extension FROM entries WHERE entry_type = 'image'",
        )?;
        let rows: Vec<(String, String, String, Option<String>)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);

        if rows.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "Migrating image store to content-addressed layout ({} rows)...",
            rows.len()
        );

        let mut migrated = 0usize;
        let mut dangling = 0usize;
        let mut mismatched = 0usize;

        for (id, hash, content, ext) in rows {
            // Already migrated? image_extension populated AND content looks
            // like a full hex hash.
            if ext.is_some()
                && content.len() == 64
                && content.chars().all(|c| c.is_ascii_hexdigit())
            {
                continue;
            }

            let legacy_path = PathBuf::from(&content);
            if !legacy_path.exists() {
                tracing::warn!(
                    "image row {} references missing file {}; skipping (run `ditox repair` to prune)",
                    id,
                    content
                );
                dangling += 1;
                continue;
            }

            let bytes = match std::fs::read(&legacy_path) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("failed reading legacy image {}: {}", content, e);
                    dangling += 1;
                    continue;
                }
            };

            let actual_hash = crate::clipboard::Clipboard::hash(&bytes);
            if actual_hash != hash {
                tracing::warn!(
                    "image row {} has hash mismatch (db={}, actual={}); keeping legacy file for manual review",
                    id,
                    hash,
                    actual_hash
                );
                mismatched += 1;
                continue;
            }

            let extension = legacy_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| "png".to_string());

            match Self::store_image_blob(&hash, &extension, &bytes) {
                Ok(_) => {
                    self.conn.execute(
                        "UPDATE entries SET content = ?1, image_extension = ?2 WHERE id = ?3",
                        params![&hash, &extension, &id],
                    )?;
                    // Best-effort remove of the legacy file.
                    if let Err(e) = std::fs::remove_file(&legacy_path) {
                        tracing::warn!(
                            "migrated {} but could not remove legacy file {}: {}",
                            id,
                            legacy_path.display(),
                            e
                        );
                    }
                    migrated += 1;
                }
                Err(e) => {
                    tracing::warn!("failed storing migrated blob for {}: {}", id, e);
                }
            }
        }

        tracing::info!(
            "Image store migration complete: migrated={} dangling={} mismatched={}",
            migrated,
            dangling,
            mismatched
        );

        Ok(())
    }

    pub fn insert(&self, entry: &Entry) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entries (id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                entry.image_extension,
            ],
        )?;
        Ok(())
    }

    pub fn get_all(&self, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
             FROM entries
             ORDER BY last_used DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map([limit as i64], Self::row_to_entry)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
             FROM entries WHERE id = ?1",
        )?;

        let entry = stmt.query_row([id], Self::row_to_entry).optional()?;

        Ok(entry)
    }

    pub fn get_by_index(&self, index: usize) -> Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
             FROM entries
             ORDER BY last_used DESC
             LIMIT 1 OFFSET ?1",
        )?;

        let entry = stmt
            .query_row([index as i64], Self::row_to_entry)
            .optional()?;

        Ok(entry)
    }

    /// Delete a single entry. For image rows the backing blob is pruned
    /// iff no other live row references the same hash.
    pub fn delete(&mut self, id: &str) -> Result<bool> {
        let tx = self.conn.transaction()?;
        let removed: Option<(String, String, Option<String>)> = tx
            .query_row(
                "DELETE FROM entries WHERE id = ?1
                 RETURNING entry_type, hash, image_extension",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;

        if let Some((entry_type, hash, image_extension)) = &removed {
            if entry_type == "image" {
                let ext = image_extension.clone().unwrap_or_else(|| "png".to_string());
                Self::queue_blob_prune_tx(&tx, hash, &ext)?;
            }
        }
        tx.commit()?;

        // Outside the SQL transaction: actually unlink the file (or leave it
        // queued if something went wrong — the next startup will retry).
        self.drain_pending_blob_prunes();

        Ok(removed.is_some())
    }

    /// Delete a DB row whose backing blob is missing. Unlike `delete()`,
    /// this does NOT queue a prune (there's nothing to prune) and is safe
    /// to call in bulk from `ditox repair`.
    pub fn delete_dangling_row(&self, id: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM entries WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Move a file to the quarantine directory, preserving its content for
    /// manual inspection. Caller is expected to have verified that this
    /// file's on-disk hash doesn't match its DB hash.
    ///
    /// Destination: `images/.quarantine/{db_hash}_{actual_hash}.{ext}`.
    /// Returns the destination path.
    pub fn quarantine_file(
        path: &Path,
        db_hash: &str,
        actual_hash: &str,
        extension: &str,
    ) -> Result<PathBuf> {
        let base = Self::get_images_dir()?;
        let qdir = base.join(QUARANTINE_DIR);
        std::fs::create_dir_all(&qdir)?;
        let dest = qdir.join(format!("{}_{}.{}", db_hash, actual_hash, extension));
        // If the destination already exists (previous run quarantined the
        // same pair), leave it alone and just remove the source.
        if dest.exists() {
            let _ = std::fs::remove_file(path);
        } else {
            std::fs::rename(path, &dest)?;
        }
        Ok(dest)
    }

    /// Wipe every entry. Every image row's blob is pruned.
    pub fn clear_all(&mut self) -> Result<usize> {
        let tx = self.conn.transaction()?;
        // Queue every image row's blob BEFORE deletion (need hash+ext).
        tx.execute(
            "INSERT OR IGNORE INTO pending_blob_prunes (hash, extension, queued_at)
             SELECT hash, COALESCE(image_extension, 'png'), ?1
             FROM entries WHERE entry_type = 'image'",
            [Utc::now().to_rfc3339()],
        )?;
        let rows = tx.execute("DELETE FROM entries", [])?;
        tx.commit()?;

        self.drain_pending_blob_prunes();
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
        let rows = self
            .conn
            .execute("UPDATE entries SET pinned = NOT pinned WHERE id = ?1", [id])?;
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

    /// LRU eviction of non-pinned entries beyond `max_entries`. Image blobs
    /// are queued for pruning inside the same transaction.
    pub fn cleanup_old(&mut self, max_entries: usize) -> Result<usize> {
        let tx = self.conn.transaction()?;
        // Queue image blobs of rows we're about to evict.
        tx.execute(
            "INSERT OR IGNORE INTO pending_blob_prunes (hash, extension, queued_at)
             SELECT hash, COALESCE(image_extension, 'png'), ?1
             FROM entries
             WHERE entry_type = 'image'
               AND id IN (
                   SELECT id FROM entries
                   WHERE pinned = 0
                   ORDER BY last_used DESC
                   LIMIT -1 OFFSET ?2
               )",
            params![Utc::now().to_rfc3339(), max_entries as i64],
        )?;
        let rows = tx.execute(
            "DELETE FROM entries WHERE id IN (
                SELECT id FROM entries
                WHERE pinned = 0
                ORDER BY last_used DESC
                LIMIT -1 OFFSET ?1
            )",
            [max_entries as i64],
        )?;
        tx.commit()?;

        self.drain_pending_blob_prunes();
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
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
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

    /// Search entries using FTS5 (Full-Text Search)
    /// Returns entries matching the query, ordered by relevance (implicitly, but we sort by last_used)
    pub fn search_entries(&self, query: &str, limit: usize) -> Result<Vec<Entry>> {
        // FTS syntax: wrap in quotes to match phrase or sanitize
        // For simple partial matching in FTS, typically just the words or wildcards.
        // We'll use prefix matching for the query terms.

        let should_use_wildcard = !query.contains('"') && !query.contains('*');
        let fts_query = if should_use_wildcard {
            format!("\"{}\"*", query.replace("\"", "\"\""))
        } else {
            query.to_string()
        };

        // Note: ORDER BY rank is the default relevance for FTS, but usually users want recent stuff too.
        // However, usually detailed search means relevance matters.
        // Current requirement says "ordered by last_used" in previous implementation.
        // But FTS is most useful when ranked by match.
        // LIMIT applies.
        // Let's stick to last_used DESC as the primary sort for consistency unless the user requested relevance sort.
        // Actually, for search, let's just get matching entries and sort by last_used.

        // We join with FTS table to get the matching IDs
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.entry_type, e.content, e.hash, e.byte_size, e.created_at, e.last_used, e.pinned, e.notes, e.collection_id, e.image_extension
             FROM entries e
             JOIN entries_fts f ON e.id = f.id
             WHERE entries_fts MATCH ?1
             ORDER BY e.last_used DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![fts_query, limit as i64], |row| {
                Self::row_to_entry(row)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Search entries using FTS5 with additional filtering
    /// Returns entries matching the query and filter, ordered by last_used
    pub fn search_entries_filtered(
        &self,
        query: &str,
        limit: usize,
        filter: &str,
        collection_id: Option<&str>,
    ) -> Result<Vec<Entry>> {
        let should_use_wildcard = !query.contains('"') && !query.contains('*');
        let fts_query = if should_use_wildcard {
            format!("\"{}\"*", query.replace("\"", "\"\""))
        } else {
            query.to_string()
        };
        let limit_i64 = limit as i64;

        // Base query joins with FTS
        let base_sql = "SELECT e.id, e.entry_type, e.content, e.hash, e.byte_size, e.created_at, e.last_used, e.pinned, e.notes, e.collection_id, e.image_extension
                        FROM entries e
                        JOIN entries_fts f ON e.id = f.id
                        WHERE entries_fts MATCH ?2";

        // Append filter conditions
        match filter {
            "text" | "image" => {
                let sql = format!(
                    "{} AND e.entry_type = ?1 ORDER BY e.last_used DESC LIMIT ?3",
                    base_sql
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![filter, fts_query, limit_i64], Self::row_to_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            "today" => {
                let today = (Utc::now() - Duration::hours(24)).to_rfc3339();
                let sql = format!(
                    "{} AND e.created_at > ?1 ORDER BY e.last_used DESC LIMIT ?3",
                    base_sql
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![today, fts_query, limit_i64], Self::row_to_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            "collection" if collection_id.is_some() => {
                let cid = collection_id.unwrap();
                let sql = format!(
                    "{} AND e.collection_id = ?1 ORDER BY e.last_used DESC LIMIT ?3",
                    base_sql
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let entries = stmt
                    .query_map(params![cid, fts_query, limit_i64], Self::row_to_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            "favorite" => {
                // Fixed param index: pinned=1 is simpler to inline or use param.
                // Using ?1 for consistency in param ordering logic is tricky if no other param.
                // Here we don't have a ?1 so we need to be careful with indexing.
                // The base query uses ?2 for MATCH.
                // So we can put pinned=1 in SQL.
                let _sql = format!(
                    "{} AND e.pinned = 1 ORDER BY e.last_used DESC LIMIT ?3",
                    base_sql
                );
                // We only need fts_query and limit.
                // But wait, our base query uses ?2. So we need ?1 to be something or renumber?
                // Actually base_sql uses ?2. So we can pass a dummy as ?1 or just fix the indices.
                // Let's rewrite simple query for this case.
                let mut stmt = self.conn.prepare(
                    "SELECT e.id, e.entry_type, e.content, e.hash, e.byte_size, e.created_at, e.last_used, e.pinned, e.notes, e.collection_id, e.image_extension
                     FROM entries e
                     JOIN entries_fts f ON e.id = f.id
                     WHERE entries_fts MATCH ?1 AND e.pinned = 1
                     ORDER BY e.last_used DESC
                     LIMIT ?2"
                )?;
                let entries = stmt
                    .query_map(params![fts_query, limit_i64], Self::row_to_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return Ok(entries);
            }
            _ => {} // Fall through
        }

        // Default query (all)
        // Redefine base sql to use ?1 for match and ?2 for limit
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.entry_type, e.content, e.hash, e.byte_size, e.created_at, e.last_used, e.pinned, e.notes, e.collection_id, e.image_extension
             FROM entries e
             JOIN entries_fts f ON e.id = f.id
             WHERE entries_fts MATCH ?1
             ORDER BY e.last_used DESC
             LIMIT ?2"
        )?;

        let entries = stmt
            .query_map(params![fts_query, limit_i64], Self::row_to_entry)?
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
            image_extension: row.get(10)?,
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
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
             FROM entries
             WHERE usage_count > 0
             ORDER BY usage_count DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map(params![limit as i64], Self::row_to_entry)?
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
        let favorites_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM entries WHERE pinned = 1", [], |row| {
                    row.get(0)
                })?;

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
                        // `content` is the hash; show a stable synthesized
                        // label rather than leaking filesystem details.
                        format!("image-{}", content.chars().take(8).collect::<String>())
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
        let db_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

        // Image bytes: walk via `scan_image_files` so we pick up the 2-char
        // fan-out directories and skip `.quarantine` + `.tmp` leftovers.
        let images_size_bytes: u64 = self
            .scan_image_files()
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(|p| p.symlink_metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0);

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
            .query_map([], Self::row_to_collection)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(collections)
    }

    /// Get a collection by ID
    pub fn get_collection_by_id(&self, id: &str) -> Result<Option<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, keybind, position, created_at
             FROM collections WHERE id = ?1",
        )?;

        let collection = stmt.query_row([id], Self::row_to_collection).optional()?;

        Ok(collection)
    }

    /// Get a collection by name
    pub fn get_collection_by_name(&self, name: &str) -> Result<Option<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, keybind, position, created_at
             FROM collections WHERE name = ?1",
        )?;

        let collection = stmt.query_row([name], Self::row_to_collection).optional()?;

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
        let rows = self
            .conn
            .execute("DELETE FROM collections WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Set or unset collection for an entry
    pub fn set_entry_collection(
        &self,
        entry_id: &str,
        collection_id: Option<&str>,
    ) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE entries SET collection_id = ?1 WHERE id = ?2",
            params![collection_id, entry_id],
        )?;
        Ok(rows > 0)
    }

    /// Get entries in a specific collection
    pub fn get_entries_in_collection(
        &self,
        collection_id: &str,
        limit: usize,
    ) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
             FROM entries
             WHERE collection_id = ?1
             ORDER BY last_used DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![collection_id, limit as i64], Self::row_to_entry)?
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
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
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
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
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
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
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
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
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
                    "SELECT id, entry_type, content, hash, byte_size, created_at, last_used, pinned, notes, collection_id, image_extension
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
