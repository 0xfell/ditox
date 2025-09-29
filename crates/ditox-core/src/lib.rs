//! ditox-core: core types, storage traits, and minimal in-memory store

use serde::{Deserialize, Serialize};
use std::sync::{RwLock, Mutex};
use time::OffsetDateTime;

pub type ClipId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub text: String,
    pub created_at: OffsetDateTime,
    pub is_favorite: bool,
    pub kind: ClipKind,
}

impl Clip {
    pub fn new<S: Into<String>>(id: ClipId, text: S) -> Self {
        Self {
            id,
            text: text.into(),
            created_at: OffsetDateTime::now_utc(),
            is_favorite: false,
            kind: ClipKind::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipKind { Text, Image }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMeta {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub sha256: String,
    pub thumb_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Query {
    pub contains: Option<String>,
    pub favorites_only: bool,
    pub limit: Option<usize>,
}

pub trait Store: Send + Sync {
    fn init(&self) -> anyhow::Result<()> { Ok(()) }
    fn add(&self, text: &str) -> anyhow::Result<Clip>;
    fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>>;
    fn get(&self, id: &str) -> anyhow::Result<Option<Clip>>;
    fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()>;
    fn delete(&self, id: &str) -> anyhow::Result<()>;
    fn clear(&self) -> anyhow::Result<()>;
}

/// Minimal in-memory store used until SQLite backend lands.
#[derive(Default)]
pub struct MemStore {
    inner: RwLock<Vec<Clip>>,
}

impl MemStore {
    pub fn new() -> Self { Self { inner: RwLock::new(Vec::new()) } }
}

fn gen_id() -> String {
    // Simple sortable id placeholder: epoch nanos; replace with ULID later.
    let ns = OffsetDateTime::now_utc().unix_timestamp_nanos();
    format!("{:x}", ns)
}

impl Store for MemStore {
    fn add(&self, text: &str) -> anyhow::Result<Clip> {
        let clip = Clip::new(gen_id(), text.to_string());
        let mut v = self.inner.write().expect("poisoned");
        v.insert(0, clip.clone());
        Ok(clip)
    }

    fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
        let v = self.inner.read().expect("poisoned");
        let mut items: Vec<Clip> = v
            .iter()
            .cloned()
            .filter(|c| !q.favorites_only || c.is_favorite)
            .filter(|c| matches!(c.kind, ClipKind::Text))
            .filter(|c| match &q.contains {
                Some(s) => c.text.to_lowercase().contains(&s.to_lowercase()),
                None => true,
            })
            .collect();
        if let Some(limit) = q.limit { items.truncate(limit); }
        Ok(items)
    }

    fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
        let v = self.inner.read().expect("poisoned");
        Ok(v.iter().cloned().find(|c| c.id == id))
    }

    fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()> {
        let mut v = self.inner.write().expect("poisoned");
        if let Some(c) = v.iter_mut().find(|c| c.id == id) { c.is_favorite = fav; }
        Ok(())
    }

    fn delete(&self, id: &str) -> anyhow::Result<()> {
        let mut v = self.inner.write().expect("poisoned");
        v.retain(|c| c.id != id);
        Ok(())
    }

    fn clear(&self) -> anyhow::Result<()> {
        let mut v = self.inner.write().expect("poisoned");
        v.clear();
        Ok(())
    }
}

/// Placeholder types for future OS clipboard integrations.
pub mod clipboard {
    use anyhow::Result;

    pub trait Clipboard: Send + Sync {
        fn get_text(&self) -> Result<Option<String>>;
        fn set_text(&self, text: &str) -> Result<()>;
    }

    #[derive(Default)]
    pub struct NoopClipboard;
    impl Clipboard for NoopClipboard {
        fn get_text(&self) -> Result<Option<String>> { Ok(None) }
        fn set_text(&self, _text: &str) -> Result<()> { Ok(()) }
    }

    #[cfg(all(feature = "clipboard", target_os = "linux"))]
    pub struct ArboardClipboard;

    #[cfg(all(feature = "clipboard", target_os = "linux"))]
    impl ArboardClipboard {
        pub fn new() -> Self { Self }
    }

    #[cfg(all(feature = "clipboard", target_os = "linux"))]
    impl Clipboard for ArboardClipboard {
        fn get_text(&self) -> Result<Option<String>> {
            let mut cb = arboard::Clipboard::new()?;
            match cb.get_text() {
                Ok(s) => Ok(Some(s)),
                Err(_) => Ok(None),
            }
        }
        fn set_text(&self, text: &str) -> Result<()> {
            let mut cb = arboard::Clipboard::new()?;
            cb.set_text(text.to_string())?;
            Ok(())
        }
    }
}

#[cfg(feature = "sqlite")]
mod sqlite_store {
    use super::*;
    use rusqlite::{Connection, params, OptionalExtension};
    use std::path::{Path, PathBuf};
    use include_dir::{include_dir, Dir};

    static MIGRATIONS: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

    pub struct SqliteStore {
        path: PathBuf,
        conn: Mutex<Connection>,
        fts_enabled: bool,
    }

    impl SqliteStore {
        pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> { Self::new_with(path, true) }

        pub fn new_with<P: AsRef<Path>>(path: P, auto_migrate: bool) -> anyhow::Result<Self> {
            let path = path.as_ref().to_path_buf();
            let conn = Connection::open(&path)?;
            conn.pragma_update(None, "foreign_keys", &1)?;
            let _ = conn.pragma_update(None, "journal_mode", &"WAL");
            let mut store = Self { path, conn: Mutex::new(conn), fts_enabled: false };
            store.init_with(auto_migrate)?;
            Ok(store)
        }

        fn run_migrations(&self, conn: &Connection, auto: bool) -> anyhow::Result<()> {
            let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            let mut files: Vec<_> = MIGRATIONS
                .files()
                .filter(|f| f.path().extension().map(|e| e == "sql").unwrap_or(false))
                .collect();
            files.sort_by_key(|f| f.path().to_path_buf());
            let mut applied_any = false;
            for file in files {
                let name = file.path().file_stem().unwrap().to_string_lossy().to_string();
                let ver = super::parse_version_prefix(&name).unwrap_or(0) as i64;
                if ver <= current { continue; }
                if current == 0 && !auto && ver > 1 { continue; }
                let sql = file.contents_utf8().ok_or_else(|| anyhow::anyhow!("invalid utf-8 in migration {}", name))?;
                let tx = conn.unchecked_transaction()?;
                tx.execute_batch(sql)?;
                tx.execute(&format!("PRAGMA user_version = {}", ver), [])?;
                tx.commit()?;
                applied_any = true;
                if current == 0 && !auto { break; }
            }
            if applied_any {
                let _ = conn.execute_batch("INSERT INTO clips_fts(clips_fts) VALUES('rebuild');");
            }
            Ok(())
        }

        fn init_with(&mut self, auto_migrate: bool) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
            self.run_migrations(&conn, auto_migrate)?;
            let fts_enabled = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='clips_fts'")?
                .exists([])?;
            drop(conn);
            let _ = fts_enabled;
            Ok(())
        }

        pub fn migrate_all(&self) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            self.run_migrations(&conn, true)
        }

        pub fn migration_status(&self) -> anyhow::Result<MigrationStatus> {
            let conn = self.conn.lock().unwrap();
            let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            let mut files: Vec<_> = MIGRATIONS
                .files()
                .filter(|f| f.path().extension().map(|e| e == "sql").unwrap_or(false))
                .collect();
            files.sort_by_key(|f| f.path().to_path_buf());
            let latest = files.last()
                .and_then(|f| super::parse_version_prefix(&f.path().file_stem().unwrap().to_string_lossy()))
                .unwrap_or(0) as i64;
            let pending: Vec<String> = MIGRATIONS
                .files()
                .filter_map(|f| {
                    let name = f.path().file_name()?.to_string_lossy().to_string();
                    let ver = super::parse_version_prefix(&f.path().file_stem()?.to_string_lossy())? as i64;
                    if ver > current { Some(name) } else { None }
                })
                .collect();
            Ok(MigrationStatus { current, latest, pending })
        }
    }

    impl Store for SqliteStore {
        fn init(&self) -> anyhow::Result<()> { Ok(()) }

        fn add(&self, text: &str) -> anyhow::Result<Clip> {
            let id = super::gen_id();
            let created_at = OffsetDateTime::now_utc().unix_timestamp();
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO clips(id, kind, text, created_at, is_favorite) VALUES(?, 'text', ?, ?, 0)",
                params![id, text, created_at],
            )?;
            let clip = Clip { id, text: text.to_string(), created_at: OffsetDateTime::from_unix_timestamp(created_at)?, is_favorite: false, kind: ClipKind::Text };
            Ok(clip)
        }

        fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
            let conn = self.conn.lock().unwrap();
            let mut sql = String::from("SELECT id, text, created_at, is_favorite FROM clips WHERE deleted_at IS NULL AND kind = 'text'");
            if q.favorites_only { sql.push_str(" AND is_favorite = 1"); }
            if let Some(term) = &q.contains {
                // Try FTS path first
                let has_fts = conn.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='clips_fts'")?.exists([])?;
                if has_fts {
                    sql = String::from("SELECT c.id, c.text, c.created_at, c.is_favorite FROM clips c JOIN clips_fts f ON f.rowid = c.rowid WHERE c.deleted_at IS NULL AND c.kind = 'text'");
                    if q.favorites_only { sql.push_str(" AND c.is_favorite = 1"); }
                    sql.push_str(" AND f.text MATCH ?1 ORDER BY c.created_at DESC");
                    let mut stmt = conn.prepare(&sql)?;
                    let mut rows = stmt.query([term])?;
                    let mut out = Vec::new();
                    while let Some(row) = rows.next()? {
                        let created: i64 = row.get(2)?;
                        out.push(Clip { id: row.get(0)?, text: row.get(1)?, created_at: OffsetDateTime::from_unix_timestamp(created)?, is_favorite: row.get::<_, i64>(3)? != 0, kind: ClipKind::Text });
                    }
                    if let Some(limit) = q.limit { out.truncate(limit); }
                    return Ok(out);
                } else {
                    sql.push_str(" AND text LIKE ?1");
                    let like = format!("%{}%", term);
                    sql.push_str(" ORDER BY created_at DESC");
                    if let Some(limit) = q.limit { sql.push_str(&format!(" LIMIT {}", limit)); }
                    let mut stmt = conn.prepare(&sql)?;
                    let mut rows = stmt.query([like])?;
                    let mut out = Vec::new();
                    while let Some(row) = rows.next()? {
                        let created: i64 = row.get(2)?;
                        out.push(Clip { id: row.get(0)?, text: row.get(1)?, created_at: OffsetDateTime::from_unix_timestamp(created)?, is_favorite: row.get::<_, i64>(3)? != 0, kind: ClipKind::Text });
                    }
                    return Ok(out);
                }
            }
            sql.push_str(" ORDER BY created_at DESC");
            if let Some(limit) = q.limit { sql.push_str(&format!(" LIMIT {}", limit)); }
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let created: i64 = row.get(2)?;
                out.push(Clip { id: row.get(0)?, text: row.get(1)?, created_at: OffsetDateTime::from_unix_timestamp(created)?, is_favorite: row.get::<_, i64>(3)? != 0, kind: ClipKind::Text });
            }
            Ok(out)
        }

        fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, text, created_at, is_favorite FROM clips WHERE id = ? AND deleted_at IS NULL AND kind = 'text'")?;
            let opt = stmt.query_row([id], |row| {
                let created: i64 = row.get(2)?;
                Ok(Clip { id: row.get(0)?, text: row.get(1)?, created_at: OffsetDateTime::from_unix_timestamp(created).unwrap_or(OffsetDateTime::now_utc()), is_favorite: row.get::<_, i64>(3)? != 0, kind: ClipKind::Text })
            }).optional()?;
            Ok(opt)
        }

        fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            conn.execute("UPDATE clips SET is_favorite = ? WHERE id = ?", params![if fav {1} else {0}, id])?;
            Ok(())
        }

        fn delete(&self, id: &str) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            conn.execute("DELETE FROM clips WHERE id = ?", params![id])?;
            Ok(())
        }

        fn clear(&self) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            conn.execute("DELETE FROM clips", [])?;
            Ok(())
        }
    }

    // Re-export
    pub use SqliteStore as StoreImpl;
}

#[cfg(not(feature = "sqlite"))]
mod sqlite_store { pub use super::MemStore as StoreImpl; }

pub use sqlite_store::StoreImpl;

#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub current: i64,
    pub latest: i64,
    pub pending: Vec<String>,
}

pub(crate) fn parse_version_prefix(name: &str) -> Option<u32> {
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { None } else { digits.parse::<u32>().ok() }
}

// Content-addressed blob store scaffold for images
pub mod blobstore {
    use sha2::{Digest, Sha256};
    use std::{fs, io::Write, path::{Path, PathBuf}};

    pub struct BlobStore { root: PathBuf }
    impl BlobStore {
        pub fn new<P: AsRef<Path>>(root: P) -> Self { Self { root: root.as_ref().to_path_buf() } }
        pub fn put(&self, bytes: &[u8]) -> std::io::Result<String> {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let digest = hasher.finalize();
            let hex = hex::encode(digest);
            let (a,b) = (&hex[0..2], &hex[2..4]);
            let dir = self.root.join("objects").join(a).join(b);
            fs::create_dir_all(&dir)?;
            let path = dir.join(&hex);
            if !path.exists() {
                let mut f = fs::File::create(&path)?;
                f.write_all(bytes)?;
            }
            Ok(hex)
        }
        pub fn get(&self, sha256: &str) -> std::io::Result<Vec<u8>> {
            let (a,b) = (&sha256[0..2], &sha256[2..4]);
            let path = self.root.join("objects").join(a).join(b).join(sha256);
            fs::read(path)
        }
    }
}
