//! ditox-core: core types, storage traits, and minimal in-memory store

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, RwLock};
use time::OffsetDateTime;

pub type ClipId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub text: String,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
    pub is_favorite: bool,
    pub kind: ClipKind,
    pub is_image: bool,
    pub image_path: Option<String>,
}

impl Clip {
    pub fn new<S: Into<String>>(id: ClipId, text: S) -> Self {
        Self {
            id,
            text: text.into(),
            created_at: OffsetDateTime::now_utc(),
            last_used_at: None,
            is_favorite: false,
            kind: ClipKind::Text,
            is_image: false,
            image_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipKind {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMeta {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub sha256: String,
    pub thumb_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRgba {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Query {
    pub contains: Option<String>,
    pub favorites_only: bool,
    pub limit: Option<usize>,
    /// Optional single-tag filter (exact match)
    pub tag: Option<String>,
    /// When true and FTS is available, order by bm25 rank
    pub rank: bool,
}

pub trait Store: Send + Sync {
    fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn add(&self, text: &str) -> anyhow::Result<Clip>;
    fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>>;
    fn get(&self, id: &str) -> anyhow::Result<Option<Clip>>;
    fn touch_last_used(&self, id: &str) -> anyhow::Result<()>;
    fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()>;
    fn delete(&self, id: &str) -> anyhow::Result<()>;
    fn clear(&self) -> anyhow::Result<()>;
    // Tags
    fn add_tags(&self, id: &str, tags: &[String]) -> anyhow::Result<()>;
    fn remove_tags(&self, id: &str, tags: &[String]) -> anyhow::Result<()>;
    fn list_tags(&self, id: &str) -> anyhow::Result<Vec<String>>;
    // Images
    fn add_image_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<Clip>;
    fn get_image_meta(&self, id: &str) -> anyhow::Result<Option<ImageMeta>>;
    fn get_image_rgba(&self, id: &str) -> anyhow::Result<Option<ImageRgba>>;
    fn list_images(&self, q: Query) -> anyhow::Result<Vec<(Clip, ImageMeta)>>;
    fn add_image_from_path(&self, _path: &std::path::Path) -> anyhow::Result<Clip> {
        anyhow::bail!("not supported")
    }
    // Retention
    fn prune(
        &self,
        max_items: Option<usize>,
        max_age: Option<time::Duration>,
        keep_favorites: bool,
    ) -> anyhow::Result<usize>;
}

/// Minimal in-memory store used until SQLite backend lands.
#[derive(Default)]
pub struct MemStore {
    inner: RwLock<Vec<Clip>>,
    images: RwLock<std::collections::HashMap<ClipId, ImageRgba>>, // simple scaffold
    tags: RwLock<std::collections::HashMap<ClipId, std::collections::BTreeSet<String>>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            images: RwLock::new(std::collections::HashMap::new()),
            tags: RwLock::new(std::collections::HashMap::new()),
        }
    }
}

fn gen_id() -> String {
    // Simple sortable id placeholder: epoch nanos; replace with ULID later.
    let ns = OffsetDateTime::now_utc().unix_timestamp_nanos();
    format!("{:x}", ns)
}

impl Store for MemStore {
    fn add(&self, text: &str) -> anyhow::Result<Clip> {
        let clip = Clip {
            id: gen_id(),
            text: text.to_string(),
            created_at: OffsetDateTime::now_utc(),
            last_used_at: None,
            is_favorite: false,
            kind: ClipKind::Text,
            is_image: false,
            image_path: None,
        };
        let mut v = self.inner.write().expect("poisoned");
        v.insert(0, clip.clone());
        Ok(clip)
    }

    fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
        let v = self.inner.read().expect("poisoned");
        let tags = self.tags.read().unwrap();
        let mut items: Vec<Clip> = v
            .iter()
            .filter(|c| !q.favorites_only || c.is_favorite)
            .filter(|c| matches!(c.kind, ClipKind::Text))
            .filter(|c| match &q.tag {
                Some(tag) => tags.get(&c.id).map(|s| s.contains(tag)).unwrap_or(false),
                None => true,
            })
            .filter(|c| match &q.contains {
                Some(s) => c.text.to_lowercase().contains(&s.to_lowercase()),
                None => true,
            })
            .cloned()
            .collect();
        // Sort by the most recent of created_at or last_used_at (descending)
        items.sort_by_key(|c| {
            let created = c.created_at.unix_timestamp();
            let last = c
                .last_used_at
                .map(|t| t.unix_timestamp())
                .unwrap_or(created);
            std::cmp::Reverse(std::cmp::max(created, last))
        });
        if let Some(limit) = q.limit {
            items.truncate(limit);
        }
        Ok(items)
    }

    fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
        let v = self.inner.read().expect("poisoned");
        Ok(v.iter().find(|c| c.id == id).cloned())
    }

    fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()> {
        let mut v = self.inner.write().expect("poisoned");
        if let Some(c) = v.iter_mut().find(|c| c.id == id) {
            c.is_favorite = fav;
        }
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
        self.images.write().unwrap().clear();
        self.tags.write().unwrap().clear();
        Ok(())
    }

    fn add_image_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<Clip> {
        let id = gen_id();
        let clip = Clip {
            id: id.clone(),
            text: String::new(),
            created_at: OffsetDateTime::now_utc(),
            last_used_at: None,
            is_favorite: false,
            kind: ClipKind::Image,
            is_image: true,
            image_path: None,
        };
        self.images.write().unwrap().insert(
            id.clone(),
            ImageRgba {
                width,
                height,
                bytes: rgba.to_vec(),
            },
        );
        self.inner.write().unwrap().insert(0, clip.clone());
        Ok(clip)
    }

    fn get_image_meta(&self, id: &str) -> anyhow::Result<Option<ImageMeta>> {
        let im = self.images.read().unwrap();
        Ok(im.get(id).map(|img| ImageMeta {
            format: "rgba".into(),
            width: img.width,
            height: img.height,
            size_bytes: img.bytes.len() as u64,
            sha256: String::new(),
            thumb_path: None,
        }))
    }

    fn get_image_rgba(&self, id: &str) -> anyhow::Result<Option<ImageRgba>> {
        let im = self.images.read().unwrap();
        Ok(im.get(id).cloned())
    }

    fn list_images(&self, q: Query) -> anyhow::Result<Vec<(Clip, ImageMeta)>> {
        let v = self.inner.read().unwrap();
        let im = self.images.read().unwrap();
        let tags = self.tags.read().unwrap();
        let mut out = Vec::new();
        for c in v.iter().filter(|c| matches!(c.kind, ClipKind::Image)) {
            if q.favorites_only && !c.is_favorite {
                continue;
            }
            if let Some(tag) = &q.tag {
                if !tags.get(&c.id).map(|s| s.contains(tag)).unwrap_or(false) {
                    continue;
                }
            }
            if let Some(img) = im.get(&c.id) {
                out.push((
                    c.clone(),
                    ImageMeta {
                        format: "rgba".into(),
                        width: img.width,
                        height: img.height,
                        size_bytes: img.bytes.len() as u64,
                        sha256: String::new(),
                        thumb_path: None,
                    },
                ));
            }
        }
        // Sort by most recent of created_at or last_used_at (descending)
        out.sort_by_key(|(c, _)| {
            let created = c.created_at.unix_timestamp();
            let last = c
                .last_used_at
                .map(|t| t.unix_timestamp())
                .unwrap_or(created);
            std::cmp::Reverse(std::cmp::max(created, last))
        });
        if let Some(limit) = q.limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    fn prune(
        &self,
        max_items: Option<usize>,
        max_age: Option<time::Duration>,
        keep_favorites: bool,
    ) -> anyhow::Result<usize> {
        let mut v = self.inner.write().unwrap();
        let before = v.len();
        // age-based
        if let Some(age) = max_age {
            let cutoff = OffsetDateTime::now_utc() - age;
            v.retain(|c| c.created_at >= cutoff || (keep_favorites && c.is_favorite));
        }
        // max-items (keep newest first)
        if let Some(n) = max_items {
            if v.len() > n {
                // If keeping favorites, move them to front before truncation
                if keep_favorites {
                    v.sort_by_key(|c| (!c.is_favorite, std::cmp::Reverse(c.created_at)));
                }
                v.truncate(n);
            }
        }
        let after = v.len();
        Ok(before - after)
    }

    fn add_tags(&self, id: &str, tags: &[String]) -> anyhow::Result<()> {
        let mut all = self.tags.write().unwrap();
        let set = all.entry(id.to_string()).or_default();
        for t in tags {
            set.insert(t.to_string());
        }
        Ok(())
    }

    fn remove_tags(&self, id: &str, tags: &[String]) -> anyhow::Result<()> {
        let mut all = self.tags.write().unwrap();
        if let Some(set) = all.get_mut(id) {
            for t in tags {
                set.remove(t);
            }
        }
        Ok(())
    }

    fn list_tags(&self, id: &str) -> anyhow::Result<Vec<String>> {
        let all = self.tags.read().unwrap();
        Ok(all
            .get(id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default())
    }

    fn touch_last_used(&self, id: &str) -> anyhow::Result<()> {
        let mut v = self.inner.write().unwrap();
        if let Some(c) = v.iter_mut().find(|c| c.id == id) {
            c.last_used_at = Some(OffsetDateTime::now_utc());
        }
        Ok(())
    }
}

/// Placeholder types for future OS clipboard integrations.
pub mod clipboard {
    use super::ImageRgba;
    use anyhow::Result;

    pub trait Clipboard: Send + Sync {
        fn get_text(&self) -> Result<Option<String>>;
        fn set_text(&self, text: &str) -> Result<()>;
        fn get_image(&self) -> Result<Option<ImageRgba>> {
            Ok(None)
        }
        fn set_image(&self, _img: &ImageRgba) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct NoopClipboard;
    impl Clipboard for NoopClipboard {
        fn get_text(&self) -> Result<Option<String>> {
            Ok(None)
        }
        fn set_text(&self, _text: &str) -> Result<()> {
            Ok(())
        }
    }

    #[cfg(feature = "clipboard")]
    pub struct ArboardClipboard;

    #[cfg(feature = "clipboard")]
    impl Default for ArboardClipboard {
        fn default() -> Self {
            Self
        }
    }

    #[cfg(feature = "clipboard")]
    impl ArboardClipboard {
        pub fn new() -> Self {
            Self
        }
    }

    #[cfg(feature = "clipboard")]
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
        fn get_image(&self) -> Result<Option<ImageRgba>> {
            let mut cb = arboard::Clipboard::new()?;
            match cb.get_image() {
                Ok(img) => Ok(Some(ImageRgba {
                    width: img.width as u32,
                    height: img.height as u32,
                    bytes: img.bytes.into_owned(),
                })),
                Err(_) => Ok(None),
            }
        }
        fn set_image(&self, img: &ImageRgba) -> Result<()> {
            let mut cb = arboard::Clipboard::new()?;
            let data = arboard::ImageData {
                width: img.width as usize,
                height: img.height as usize,
                bytes: std::borrow::Cow::Borrowed(&img.bytes),
            };
            cb.set_image(data)?;
            Ok(())
        }
    }
}

#[cfg(feature = "sqlite")]
mod sqlite_store {
    use super::*;
    use crate::blobstore::BlobStore;
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    use image::{GenericImageView, ImageFormat, ImageReader};
    use include_dir::{include_dir, Dir};
    use rusqlite::{params, Connection, OptionalExtension};
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    static MIGRATIONS: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

    pub struct SqliteStore {
        path: PathBuf,
        conn: Mutex<Connection>,
        _fts_enabled: bool,
    }

    impl SqliteStore {
        pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
            Self::new_with(path, true)
        }

        pub fn new_with<P: AsRef<Path>>(path: P, auto_migrate: bool) -> anyhow::Result<Self> {
            let path = path.as_ref().to_path_buf();
            let conn = Connection::open(&path)?;
            conn.pragma_update(None, "foreign_keys", 1)?;
            let _ = conn.pragma_update(None, "journal_mode", "WAL");
            let mut store = Self {
                path,
                conn: Mutex::new(conn),
                _fts_enabled: false,
            };
            store.init_with(auto_migrate)?;
            Ok(store)
        }

        fn run_migrations(&self, conn: &Connection, auto: bool) -> anyhow::Result<()> {
            let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            // Heuristic: bump user_version if schema objects already exist, and fix partial upgrades
            let has_is_image: bool = conn
                .prepare("SELECT 1 FROM pragma_table_info('clips') WHERE name='is_image'")?
                .exists([])?;
            let has_image_path: bool = conn
                .prepare("SELECT 1 FROM pragma_table_info('clips') WHERE name='image_path'")?
                .exists([])?;
            // Some older DBs may have is_image but not image_path; add it proactively
            if has_is_image && !has_image_path {
                let _ = conn.execute_batch("ALTER TABLE clips ADD COLUMN image_path TEXT;");
            }
            if (has_is_image && has_image_path) && current < 3 {
                let _ = conn.execute_batch("PRAGMA user_version = 3");
            }
            let current = if (has_is_image && has_image_path) && current < 3 {
                3
            } else {
                current
            };
            let has_updated_at: bool = {
                conn.prepare("SELECT 1 FROM pragma_table_info('clips') WHERE name='updated_at'")?
                    .exists([])?
            };
            if has_updated_at && current < 4 {
                let _ = conn.execute_batch("PRAGMA user_version = 4");
            }
            let current = if has_updated_at && current < 4 {
                4
            } else {
                current
            };

            let mut files: Vec<_> = MIGRATIONS
                .files()
                .filter(|f| f.path().extension().map(|e| e == "sql").unwrap_or(false))
                .collect();
            files.sort_by_key(|f| f.path().to_path_buf());
            let mut applied_any = false;
            for file in files {
                let name = file
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let ver = super::parse_version_prefix(&name).unwrap_or(0) as i64;
                if ver <= current {
                    continue;
                }
                // Skip image-path migration only if both columns exist
                if name.contains("0003_images_path") {
                    let has_is_image: bool = conn
                        .prepare("SELECT 1 FROM pragma_table_info('clips') WHERE name='is_image'")?
                        .exists([])?;
                    let has_image_path: bool = conn
                        .prepare(
                            "SELECT 1 FROM pragma_table_info('clips') WHERE name='image_path'",
                        )?
                        .exists([])?;
                    if has_is_image && has_image_path {
                        continue;
                    }
                }
                if current == 0 && !auto && ver > 1 {
                    continue;
                }
                let sql = file
                    .contents_utf8()
                    .ok_or_else(|| anyhow::anyhow!("invalid utf-8 in migration {}", name))?;
                let tx = conn.unchecked_transaction()?;
                tx.execute_batch(sql)?;
                tx.execute(&format!("PRAGMA user_version = {}", ver), [])?;
                tx.commit()?;
                applied_any = true;
                if current == 0 && !auto {
                    break;
                }
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
            let fts_enabled = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='clips_fts'")?
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
            let latest = files
                .last()
                .and_then(|f| {
                    super::parse_version_prefix(&f.path().file_stem().unwrap().to_string_lossy())
                })
                .unwrap_or(0) as i64;
            let pending: Vec<String> = MIGRATIONS
                .files()
                .filter_map(|f| {
                    let name = f.path().file_name()?.to_string_lossy().to_string();
                    let ver = super::parse_version_prefix(&f.path().file_stem()?.to_string_lossy())?
                        as i64;
                    if ver > current {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();
            Ok(MigrationStatus {
                current,
                latest,
                pending,
            })
        }
    }

    impl Store for SqliteStore {
        fn init(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn add(&self, text: &str) -> anyhow::Result<Clip> {
            let id = super::gen_id();
            let created_at = OffsetDateTime::now_utc().unix_timestamp();
            let conn = self.conn.lock().unwrap();
            let updated_at = created_at;
            let lamport: i64 = conn
                .query_row("SELECT COALESCE(MAX(lamport),0)+1 FROM clips", [], |r| {
                    r.get(0)
                })
                .unwrap_or(1);
            conn.execute(
                "INSERT INTO clips(id, kind, text, created_at, is_favorite, updated_at, lamport, device_id) VALUES(?, 'text', ?, ?, 0, ?, ?, '')",
                params![id, text, created_at, updated_at, lamport],
            )?;
            let clip = Clip {
                id,
                text: text.to_string(),
                created_at: OffsetDateTime::from_unix_timestamp(created_at)?,
                last_used_at: None,
                is_favorite: false,
                kind: ClipKind::Text,
                is_image: false,
                image_path: None,
            };
            Ok(clip)
        }

        fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
            let conn = self.conn.lock().unwrap();
            let mut sql = String::from("SELECT c.id, c.text, c.created_at, c.is_favorite, c.last_used_at FROM clips c WHERE c.deleted_at IS NULL AND c.kind = 'text'");
            if q.favorites_only {
                sql.push_str(" AND c.is_favorite = 1");
            }
            let mut params: Vec<rusqlite::types::Value> = Vec::new();
            if let Some(tag) = &q.tag {
                sql.push_str(" AND EXISTS (SELECT 1 FROM clip_tags ct WHERE ct.clip_id = c.id AND ct.name = ?)");
                params.push(rusqlite::types::Value::Text(tag.clone()));
            }
            if let Some(term) = &q.contains {
                // Try FTS path first
                let has_fts = conn
                    .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='clips_fts'")?
                    .exists([])?;
                if has_fts {
                    sql = String::from("SELECT c.id, c.text, c.created_at, c.is_favorite, c.last_used_at FROM clips c JOIN clips_fts f ON f.rowid = c.rowid WHERE c.deleted_at IS NULL AND c.kind = 'text'");
                    if q.favorites_only {
                        sql.push_str(" AND c.is_favorite = 1");
                    }
                    if let Some(tag) = &q.tag {
                        sql.push_str(" AND EXISTS (SELECT 1 FROM clip_tags ct WHERE ct.clip_id = c.id AND ct.name = ?)");
                        params.push(rusqlite::types::Value::Text(tag.clone()));
                    }
                    if q.rank {
                        // Rank primary by bm25, then by recency (max of created_at or last_used_at)
                        sql.push_str(" AND f.text MATCH ? ORDER BY bm25(clips_fts) ASC, MAX(c.created_at, COALESCE(c.last_used_at, c.created_at)) DESC");
                    } else {
                        sql.push_str(" AND f.text MATCH ? ORDER BY MAX(c.created_at, COALESCE(c.last_used_at, c.created_at)) DESC");
                    }
                    params.push(rusqlite::types::Value::Text(term.clone()));
                    let mut stmt = conn.prepare(&sql)?;
                    let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
                    let mut out = Vec::new();
                    while let Some(row) = rows.next()? {
                        let created: i64 = row.get(2)?;
                        let last_used: Option<i64> = row.get(4)?;
                        out.push(Clip {
                            id: row.get(0)?,
                            text: row.get(1)?,
                            created_at: OffsetDateTime::from_unix_timestamp(created)?,
                            last_used_at: last_used
                                .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                            is_favorite: row.get::<_, i64>(3)? != 0,
                            kind: ClipKind::Text,
                            is_image: false,
                            image_path: None,
                        });
                    }
                    if let Some(limit) = q.limit {
                        out.truncate(limit);
                    }
                    return Ok(out);
                } else {
                    sql.push_str(" AND c.text LIKE ?");
                    let like = format!("%{}%", term);
                    sql.push_str(
                        " ORDER BY MAX(c.created_at, COALESCE(c.last_used_at, c.created_at)) DESC",
                    );
                    if let Some(limit) = q.limit {
                        sql.push_str(&format!(" LIMIT {}", limit));
                    }
                    params.push(rusqlite::types::Value::Text(like));
                    let mut stmt = conn.prepare(&sql)?;
                    let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
                    let mut out = Vec::new();
                    while let Some(row) = rows.next()? {
                        let created: i64 = row.get(2)?;
                        let last_used: Option<i64> = row.get(4)?;
                        out.push(Clip {
                            id: row.get(0)?,
                            text: row.get(1)?,
                            created_at: OffsetDateTime::from_unix_timestamp(created)?,
                            last_used_at: last_used
                                .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                            is_favorite: row.get::<_, i64>(3)? != 0,
                            kind: ClipKind::Text,
                            is_image: false,
                            image_path: None,
                        });
                    }
                    return Ok(out);
                }
            }
            // Order by most recent of created_at or last_used_at
            sql.push_str(
                " ORDER BY MAX(c.created_at, COALESCE(c.last_used_at, c.created_at)) DESC",
            );
            if let Some(limit) = q.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let created: i64 = row.get(2)?;
                let last_used: Option<i64> = row.get(4)?;
                out.push(Clip {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    created_at: OffsetDateTime::from_unix_timestamp(created)?,
                    last_used_at: last_used
                        .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                    is_favorite: row.get::<_, i64>(3)? != 0,
                    kind: ClipKind::Text,
                    is_image: false,
                    image_path: None,
                });
            }
            Ok(out)
        }

        fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, kind, text, created_at, is_favorite, COALESCE(image_path,''), CASE WHEN kind='image' THEN 1 ELSE 0 END, last_used_at FROM clips WHERE id = ? AND deleted_at IS NULL")?;
            let opt = stmt
                .query_row([id], |row| {
                    let created: i64 = row.get(3)?;
                    let last_used: Option<i64> = row.get(7)?;
                    let kind_str: String = row.get(1)?;
                    let kind = if kind_str == "image" {
                        ClipKind::Image
                    } else {
                        ClipKind::Text
                    };
                    let path: String = row.get(5)?;
                    let is_image: i64 = row.get(6)?;
                    Ok(Clip {
                        id: row.get(0)?,
                        text: row.get(2)?,
                        created_at: OffsetDateTime::from_unix_timestamp(created)
                            .unwrap_or(OffsetDateTime::now_utc()),
                        last_used_at: last_used
                            .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                        is_favorite: row.get::<_, i64>(4)? != 0,
                        kind,
                        is_image: is_image != 0,
                        image_path: if path.is_empty() { None } else { Some(path) },
                    })
                })
                .optional()?;
            Ok(opt)
        }

        fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            let lamport: i64 = conn
                .query_row("SELECT COALESCE(MAX(lamport),0)+1 FROM clips", [], |r| {
                    r.get(0)
                })
                .unwrap_or(1);
            let now = OffsetDateTime::now_utc().unix_timestamp();
            conn.execute(
                "UPDATE clips SET is_favorite = ?, updated_at = ?, lamport = ? WHERE id = ?",
                params![if fav { 1 } else { 0 }, now, lamport, id],
            )?;
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

        fn add_image_from_path(&self, path: &std::path::Path) -> anyhow::Result<Clip> {
            let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;
            let (w, h) = img.dimensions();
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let id = super::gen_id();
            let created_at = OffsetDateTime::now_utc().unix_timestamp();
            let conn = self.conn.lock().unwrap();
            let tx = conn.unchecked_transaction()?;
            tx.execute("INSERT INTO clips(id, kind, text, created_at, is_favorite, is_image, image_path) VALUES(?, 'image', '', ?, 0, 1, ?)", params![id, created_at, path.to_string_lossy()])?;
            tx.execute("INSERT OR REPLACE INTO images(clip_id, format, width, height, size_bytes, sha256, thumb_path) VALUES(?, 'png', ?, ?, ?, '', NULL)", params![id, w as i64, h as i64, size as i64])?;
            tx.commit()?;
            Ok(Clip {
                id,
                text: String::new(),
                created_at: OffsetDateTime::from_unix_timestamp(created_at)?,
                last_used_at: None,
                is_favorite: false,
                kind: ClipKind::Image,
                is_image: true,
                image_path: Some(path.to_string_lossy().into()),
            })
        }

        fn add_image_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<Clip> {
            // Encode to PNG
            let mut buf = Vec::new();
            PngEncoder::new(&mut buf).write_image(
                rgba,
                width,
                height,
                image::ColorType::Rgba8.into(),
            )?;
            let blob_root = self
                .path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            let bs = BlobStore::new(&blob_root);
            let sha = bs.put(&buf)?;
            let size = buf.len() as u64;
            let id = super::gen_id();
            let created_at = OffsetDateTime::now_utc().unix_timestamp();
            let conn = self.conn.lock().unwrap();
            let tx = conn.unchecked_transaction()?;
            tx.execute("INSERT INTO clips(id, kind, text, created_at, is_favorite, is_image) VALUES(?, 'image', '', ?, 0, 1)", params![id, created_at])?;
            tx.execute("INSERT INTO images(clip_id, format, width, height, size_bytes, sha256, thumb_path) VALUES(?, 'png', ?, ?, ?, ?, NULL)", params![id, width as i64, height as i64, size as i64, sha])?;
            tx.commit()?;
            Ok(Clip {
                id,
                text: String::new(),
                created_at: OffsetDateTime::from_unix_timestamp(created_at)?,
                last_used_at: None,
                is_favorite: false,
                kind: ClipKind::Image,
                is_image: true,
                image_path: None,
            })
        }

        fn get_image_meta(&self, id: &str) -> anyhow::Result<Option<ImageMeta>> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT format, width, height, size_bytes, sha256, thumb_path FROM images WHERE clip_id = ?")?;
            let opt = stmt
                .query_row([id], |row| {
                    Ok(ImageMeta {
                        format: row.get::<_, String>(0)?,
                        width: row.get::<_, i64>(1)? as u32,
                        height: row.get::<_, i64>(2)? as u32,
                        size_bytes: row.get::<_, i64>(3)? as u64,
                        sha256: row.get::<_, String>(4)?,
                        thumb_path: row.get::<_, Option<String>>(5)?,
                    })
                })
                .optional()?;
            Ok(opt)
        }

        fn get_image_rgba(&self, id: &str) -> anyhow::Result<Option<ImageRgba>> {
            // Prefer image_path if present
            if let Some(c) = self.get(id)? {
                if let Some(p) = c.image_path {
                    let img = ImageReader::open(&p)?.decode()?;
                    let rgba8 = img.to_rgba8();
                    let (w, h) = rgba8.dimensions();
                    return Ok(Some(ImageRgba {
                        width: w,
                        height: h,
                        bytes: rgba8.into_raw(),
                    }));
                }
            }
            let meta = match self.get_image_meta(id)? {
                Some(m) => m,
                None => return Ok(None),
            };
            let blob_root = self
                .path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            let bs = BlobStore::new(&blob_root);
            let bytes = bs.get(&meta.sha256)?;
            // Decode PNG to RGBA
            let img = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png).decode()?;
            let rgba8 = img.to_rgba8();
            let (w, h) = rgba8.dimensions();
            Ok(Some(ImageRgba {
                width: w,
                height: h,
                bytes: rgba8.into_raw(),
            }))
        }

        fn list_images(&self, q: Query) -> anyhow::Result<Vec<(Clip, ImageMeta)>> {
            let conn = self.conn.lock().unwrap();
            let mut sql = String::from("SELECT c.id, c.created_at, c.is_favorite, c.image_path, c.last_used_at, i.format, i.width, i.height, i.size_bytes, i.sha256, i.thumb_path FROM clips c JOIN images i ON i.clip_id = c.id WHERE c.deleted_at IS NULL AND c.kind = 'image'");
            let mut params: Vec<rusqlite::types::Value> = Vec::new();
            if q.favorites_only {
                sql.push_str(" AND c.is_favorite = 1");
            }
            if let Some(tag) = &q.tag {
                sql.push_str(" AND EXISTS (SELECT 1 FROM clip_tags ct WHERE ct.clip_id = c.id AND ct.name = ?)");
                params.push(rusqlite::types::Value::Text(tag.clone()));
            }
            // Order by most recent of created_at or last_used_at
            sql.push_str(
                " ORDER BY MAX(c.created_at, COALESCE(c.last_used_at, c.created_at)) DESC",
            );
            if let Some(limit) = q.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let created: i64 = row.get(1)?;
                let last_used: Option<i64> = row.get(4)?;
                let clip = Clip {
                    id: row.get(0)?,
                    text: String::new(),
                    created_at: OffsetDateTime::from_unix_timestamp(created)?,
                    last_used_at: last_used
                        .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                    is_favorite: row.get::<_, i64>(2)? != 0,
                    kind: ClipKind::Image,
                    is_image: true,
                    image_path: row.get::<_, Option<String>>(3)?,
                };
                let meta = ImageMeta {
                    format: row.get(5)?,
                    width: row.get::<_, i64>(6)? as u32,
                    height: row.get::<_, i64>(7)? as u32,
                    size_bytes: row.get::<_, i64>(8)? as u64,
                    sha256: row.get(9)?,
                    thumb_path: row.get(10)?,
                };
                out.push((clip, meta));
            }
            Ok(out)
        }

        fn touch_last_used(&self, id: &str) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            let now = OffsetDateTime::now_utc().unix_timestamp();
            let lamport: i64 = conn
                .query_row("SELECT COALESCE(MAX(lamport),0)+1 FROM clips", [], |r| {
                    r.get(0)
                })
                .unwrap_or(1);
            conn.execute(
                "UPDATE clips SET last_used_at = ?, updated_at = ?, lamport = ? WHERE id = ?",
                rusqlite::params![now, now, lamport, id],
            )?;
            Ok(())
        }

        fn prune(
            &self,
            max_items: Option<usize>,
            max_age: Option<time::Duration>,
            keep_favorites: bool,
        ) -> anyhow::Result<usize> {
            let conn = self.conn.lock().unwrap();
            let tx = conn.unchecked_transaction()?;
            let mut deleted = 0usize;
            if let Some(age) = max_age {
                let cutoff = OffsetDateTime::now_utc() - age;
                let cutoff_ts = cutoff.unix_timestamp();
                let sql = if keep_favorites {
                    "DELETE FROM clips WHERE created_at < ? AND deleted_at IS NULL AND is_favorite = 0"
                } else {
                    "DELETE FROM clips WHERE created_at < ? AND deleted_at IS NULL"
                };
                tx.execute(sql, rusqlite::params![cutoff_ts])?;
                deleted += tx.changes() as usize;
            }
            if let Some(n) = max_items {
                let sql = if keep_favorites {
                    "DELETE FROM clips WHERE rowid IN (
                        SELECT rowid FROM clips WHERE deleted_at IS NULL AND is_favorite = 0
                        ORDER BY created_at DESC
                        LIMIT -1 OFFSET ?1
                    )"
                } else {
                    "DELETE FROM clips WHERE rowid IN (
                        SELECT rowid FROM clips WHERE deleted_at IS NULL
                        ORDER BY created_at DESC
                        LIMIT -1 OFFSET ?1
                    )"
                };
                tx.execute(sql, rusqlite::params![n as i64])?;
                deleted += tx.changes() as usize;
            }
            tx.commit()?;
            Ok(deleted)
        }

        fn add_tags(&self, id: &str, tags: &[String]) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            let tx = conn.unchecked_transaction()?;
            for t in tags {
                tx.execute(
                    "INSERT OR IGNORE INTO tags(name) VALUES(?)",
                    rusqlite::params![t],
                )?;
                tx.execute(
                    "INSERT OR IGNORE INTO clip_tags(clip_id, name) VALUES(?,?)",
                    rusqlite::params![id, t],
                )?;
            }
            tx.commit()?;
            Ok(())
        }

        fn remove_tags(&self, id: &str, tags: &[String]) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            let tx = conn.unchecked_transaction()?;
            for t in tags {
                tx.execute(
                    "DELETE FROM clip_tags WHERE clip_id = ? AND name = ?",
                    rusqlite::params![id, t],
                )?;
            }
            tx.commit()?;
            Ok(())
        }

        fn list_tags(&self, id: &str) -> anyhow::Result<Vec<String>> {
            let conn = self.conn.lock().unwrap();
            let mut stmt =
                conn.prepare("SELECT name FROM clip_tags WHERE clip_id = ? ORDER BY name ASC")?;
            let mut rows = stmt.query([id])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                out.push(row.get::<_, String>(0)?);
            }
            Ok(out)
        }
    }

    // Re-export
    pub use SqliteStore as StoreImpl;
}

#[cfg(not(feature = "sqlite"))]
mod sqlite_store {
    pub use super::MemStore as StoreImpl;
}

pub use sqlite_store::StoreImpl;

#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub current: i64,
    pub latest: i64,
    pub pending: Vec<String>,
}

pub(crate) fn parse_version_prefix(name: &str) -> Option<u32> {
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u32>().ok()
    }
}

// Content-addressed blob store scaffold for images
pub mod blobstore {
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
    };

    pub struct BlobStore {
        root: PathBuf,
    }
    impl BlobStore {
        pub fn new<P: AsRef<Path>>(root: P) -> Self {
            Self {
                root: root.as_ref().to_path_buf(),
            }
        }
        pub fn put(&self, bytes: &[u8]) -> std::io::Result<String> {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let digest = hasher.finalize();
            let hex = hex::encode(digest);
            let (a, b) = (&hex[0..2], &hex[2..4]);
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
            let (a, b) = (&sha256[0..2], &sha256[2..4]);
            let path = self.root.join("objects").join(a).join(b).join(sha256);
            fs::read(path)
        }
    }
}

// Sync engine: local-first with optional remote (libsql)
pub mod sync {
    use super::*;
    #[cfg(feature = "libsql")]
    use libsql::{self};
    #[cfg(feature = "sqlite")]
    use rusqlite::Connection;
    #[cfg(feature = "libsql")]
    use tokio::runtime::Runtime;

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct SyncReport {
        pub pushed: usize,
        pub pulled: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct SyncStatus {
        pub last_push: Option<i64>,
        pub last_pull: Option<i64>,
        pub pending_local: usize,
        pub local_text: usize,
        pub local_images: usize,
        pub remote_ok: Option<bool>,
        pub last_error: Option<String>,
    }

    pub struct SyncEngine {
        #[cfg(feature = "sqlite")]
        local: Connection,
        #[cfg(feature = "libsql")]
        remote: Option<libsql::Database>,
        #[cfg(feature = "libsql")]
        rt: Option<Runtime>,
        #[cfg(feature = "libsql")]
        device_id: String,
        #[cfg(feature = "libsql")]
        batch_size: usize,
    }

    impl SyncEngine {
        #[allow(unused_variables)]
        pub fn new(
            local_db_path: &std::path::Path,
            remote_url: Option<&str>,
            remote_token: Option<&str>,
            device_id: Option<&str>,
            batch_size: usize,
        ) -> anyhow::Result<Self> {
            #[cfg(feature = "sqlite")]
            let local = Connection::open(local_db_path)?;
            #[cfg(feature = "sqlite")]
            {
                let _ = local.pragma_update(None, "foreign_keys", 1);
                // Best-effort schema add
                let _ = local.execute("ALTER TABLE clips ADD COLUMN updated_at INTEGER", []);
                let _ = local.execute("ALTER TABLE clips ADD COLUMN lamport INTEGER DEFAULT 0", []);
                let _ = local.execute("ALTER TABLE clips ADD COLUMN device_id TEXT DEFAULT ''", []);
                local.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS sync_state(key TEXT PRIMARY KEY, val INTEGER);
                    INSERT OR IGNORE INTO sync_state(key,val) VALUES('last_push_updated_at',0);
                    INSERT OR IGNORE INTO sync_state(key,val) VALUES('last_pull_updated_at',0);
                    "#,
                )?;
            }

            #[cfg(feature = "libsql")]
            let (remote, rt) = if let Some(url) = remote_url {
                let rt = Runtime::new()?;
                let token = remote_token.unwrap_or("").to_string();
                let db = rt.block_on(async {
                    libsql::Builder::new_remote(url.to_string(), token)
                        .build()
                        .await
                })?;
                (Some(db), Some(rt))
            } else {
                (None, None)
            };

            Ok(Self {
                #[cfg(feature = "sqlite")]
                local,
                #[cfg(feature = "libsql")]
                remote,
                #[cfg(feature = "libsql")]
                rt,
                #[cfg(feature = "libsql")]
                device_id: device_id
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "local".to_string()),
                #[cfg(feature = "libsql")]
                batch_size: if batch_size == 0 { 500 } else { batch_size },
            })
        }

        pub fn status(&self) -> anyhow::Result<SyncStatus> {
            #[cfg(feature = "sqlite")]
            {
                let last_push: Option<i64> = self
                    .local
                    .query_row(
                        "SELECT val FROM sync_state WHERE key='last_push_updated_at'",
                        [],
                        |r| r.get(0),
                    )
                    .ok();
                let last_pull: Option<i64> = self
                    .local
                    .query_row(
                        "SELECT val FROM sync_state WHERE key='last_pull_updated_at'",
                        [],
                        |r| r.get(0),
                    )
                    .ok();
                let pending: i64 = self.local.query_row("SELECT COUNT(1) FROM clips WHERE kind='text' AND COALESCE(updated_at, created_at) > COALESCE((SELECT val FROM sync_state WHERE key='last_push_updated_at'),0)", [], |r| r.get(0)).unwrap_or(0);
                let local_text: i64 = self
                    .local
                    .query_row("SELECT COUNT(1) FROM clips WHERE kind='text'", [], |r| {
                        r.get(0)
                    })
                    .unwrap_or(0);
                let local_images: i64 = self
                    .local
                    .query_row("SELECT COUNT(1) FROM clips WHERE kind='image'", [], |r| {
                        r.get(0)
                    })
                    .unwrap_or(0);
                let last_error: Option<String> = self
                    .local
                    .query_row(
                        "SELECT val FROM sync_state WHERE key='last_error'",
                        [],
                        |r| r.get(0),
                    )
                    .ok();
                #[cfg(feature = "libsql")]
                let remote_ok = if let (Some(remote), Some(rt)) = (&self.remote, &self.rt) {
                    match remote.connect() {
                        Ok(c) => rt
                            .block_on(async { c.query("SELECT 1", ()).await.is_ok() })
                            .into(),
                        Err(_) => Some(false),
                    }
                } else {
                    None
                };
                #[cfg(not(feature = "libsql"))]
                let remote_ok = None;
                return Ok(SyncStatus {
                    last_push,
                    last_pull,
                    pending_local: pending as usize,
                    local_text: local_text as usize,
                    local_images: local_images as usize,
                    remote_ok,
                    last_error,
                });
            }
            #[allow(unreachable_code)]
            Ok(SyncStatus::default())
        }

        pub fn run(&self, _push_only: bool, _pull_only: bool) -> anyhow::Result<SyncReport> {
            #[cfg(all(feature = "sqlite", feature = "libsql"))]
            {
                let mut pushed = 0usize;
                let mut pulled = 0usize;
                if let (Some(remote), Some(rt)) = (&self.remote, &self.rt) {
                    if !_pull_only {
                        match self.push(remote, rt) {
                            Ok(n) => pushed = n,
                            Err(e) => {
                                let _ = self.local.execute(
                                    "INSERT OR REPLACE INTO sync_state(key,val) VALUES('last_error', ?)",
                                    rusqlite::params![e.to_string()],
                                );
                                return Err(e);
                            }
                        }
                    }
                    if !_push_only {
                        match self.pull(remote, rt) {
                            Ok(n) => pulled = n,
                            Err(e) => {
                                let _ = self.local.execute(
                                    "INSERT OR REPLACE INTO sync_state(key,val) VALUES('last_error', ?)",
                                    rusqlite::params![e.to_string()],
                                );
                                return Err(e);
                            }
                        }
                    }
                }
                return Ok(SyncReport { pushed, pulled });
            }
            #[allow(unreachable_code)]
            Ok(SyncReport::default())
        }

        #[cfg(all(feature = "sqlite", feature = "libsql"))]
        fn push(&self, remote: &libsql::Database, rt: &Runtime) -> anyhow::Result<usize> {
            let last_push: i64 = self
                .local
                .query_row(
                    "SELECT val FROM sync_state WHERE key='last_push_updated_at'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let mut stmt = self.local.prepare(
                "SELECT id, kind, text, created_at, is_favorite, COALESCE(updated_at, created_at) AS ua, COALESCE(lamport,0) FROM clips WHERE COALESCE(updated_at, created_at) > ? ORDER BY ua ASC LIMIT ?"
            )?;
            let rows =
                stmt.query_map(rusqlite::params![last_push, self.batch_size as i64], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, i64>(6)?,
                    ))
                })?;
            let mut max_ua = last_push;
            let mut count = 0usize;
            let conn = remote.connect()?;
            for row in rows.flatten() {
                let (id, kind, text, created_at, fav, ua, lamport) = row;
                let fav = if fav != 0 { 1 } else { 0 };
                max_ua = max_ua.max(ua);
                count += 1;
                rt.block_on(async {
                    conn.execute(
                        "INSERT INTO clips(id, kind, text, created_at, is_favorite, updated_at, lamport, device_id) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                         ON CONFLICT(id) DO UPDATE SET text=excluded.text, is_favorite=excluded.is_favorite, updated_at=excluded.updated_at, lamport=excluded.lamport, device_id=excluded.device_id
                         WHERE (clips.lamport,clips.updated_at,COALESCE(clips.device_id,'')) < (excluded.lamport,excluded.updated_at,excluded.device_id)",
                        libsql::params!(id, kind, text, created_at, fav, ua, lamport, self.device_id.clone()),
                    ).await
                })?;
            }
            if count > 0 {
                let _ = self.local.execute(
                    "INSERT OR REPLACE INTO sync_state(key,val) VALUES('last_push_updated_at', ?)",
                    rusqlite::params![max_ua],
                );
            }
            Ok(count)
        }

        #[cfg(all(feature = "sqlite", feature = "libsql"))]
        fn pull(&self, remote: &libsql::Database, rt: &Runtime) -> anyhow::Result<usize> {
            let last_pull: i64 = self
                .local
                .query_row(
                    "SELECT val FROM sync_state WHERE key='last_pull_updated_at'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let conn = remote.connect()?;
            let mut rows = rt.block_on(async {
                conn.query(
                    "SELECT id, kind, text, created_at, is_favorite, COALESCE(updated_at, created_at) AS ua, COALESCE(lamport,0), COALESCE(device_id,'') FROM clips WHERE ua > ? ORDER BY ua ASC LIMIT ?",
                    libsql::params!(last_pull, self.batch_size as i64),
                ).await
            })?;
            let mut max_ua = last_pull;
            let mut count = 0usize;
            loop {
                match rt.block_on(async { rows.next().await }) {
                    Ok(Some(r)) => {
                        let id: String = r.get::<String>(0)?;
                        let kind: String = r.get::<String>(1)?;
                        let text: String = r.get::<String>(2)?;
                        let created_at: i64 = r.get::<i64>(3)?;
                        let fav: i64 = r.get::<i64>(4)?;
                        let ua: i64 = r.get::<i64>(5)?;
                        let lamport: i64 = r.get::<i64>(6)?;
                        let device: String = r.get::<String>(7)?;
                        max_ua = max_ua.max(ua);
                        count += 1;
                        self.local.execute(
                            "INSERT INTO clips(id, kind, text, created_at, is_favorite, updated_at, lamport, device_id)
                             VALUES(?,?,?,?,?,?,?,?)
                             ON CONFLICT(id) DO UPDATE SET text=excluded.text, is_favorite=excluded.is_favorite, updated_at=excluded.updated_at, lamport=excluded.lamport, device_id=excluded.device_id
                             WHERE (clips.lamport,clips.updated_at,COALESCE(clips.device_id,'')) < (excluded.lamport,excluded.updated_at,excluded.device_id)",
                            rusqlite::params![id, kind, text, created_at, fav, ua, lamport, device],
                        )?;
                    }
                    Ok(None) => break,
                    Err(e) => return Err(anyhow::anyhow!(e)),
                }
            }
            if count > 0 {
                let _ = self.local.execute(
                    "INSERT OR REPLACE INTO sync_state(key,val) VALUES('last_pull_updated_at', ?)",
                    rusqlite::params![max_ua],
                );
            }
            Ok(count)
        }
    }
}

// Optional: libsql/turso backend (stub wiring under feature)
#[cfg(feature = "libsql")]
pub mod libsql_backend {
    use super::*;
    use include_dir::{include_dir, Dir};
    static MIGRATIONS: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

    pub struct LibsqlStore {
        db: libsql::Database,
        rt: tokio::runtime::Runtime,
    }

    impl LibsqlStore {
        pub fn new(url: &str, auth_token: Option<&str>) -> anyhow::Result<Self> {
            let rt = tokio::runtime::Runtime::new()?;
            let db = if let Some(token) = auth_token {
                rt.block_on(async {
                    libsql::Builder::new_remote(url.to_string(), token.to_string())
                        .build()
                        .await
                })?
            } else {
                rt.block_on(async {
                    libsql::Builder::new_remote(url.to_string(), String::new())
                        .build()
                        .await
                })?
            };
            let s = Self { db, rt };
            s.init()?;
            Ok(s)
        }

        fn exec_batch(&self, conn: &libsql::Connection, sql: &str) -> anyhow::Result<()> {
            self.rt.block_on(async move {
                for stmt in sql.split(';') {
                    let stmt = stmt.trim();
                    if stmt.is_empty() {
                        continue;
                    }
                    if let Err(e) = conn.execute(stmt, ()).await {
                        let msg = e.to_string().to_lowercase();
                        if msg.contains("duplicate column name") || msg.contains("already exists") {
                            continue;
                        } else {
                            return Err(anyhow::anyhow!(e));
                        }
                    }
                }
                Ok(())
            })
        }

        fn run_migrations(&self) -> anyhow::Result<()> {
            let conn = self.db.connect()?;
            // Get current version
            let current: i64 = self.rt.block_on(async {
                let mut rows = conn.query("PRAGMA user_version", ()).await?;
                let r = rows.next().await?;
                Ok::<i64, libsql::Error>(match r {
                    Some(row) => row.get::<i64>(0)?,
                    None => 0,
                })
            })?;
            let mut files: Vec<_> = MIGRATIONS
                .files()
                .filter(|f| f.path().extension().map(|e| e == "sql").unwrap_or(false))
                .collect();
            files.sort_by_key(|f| f.path().to_path_buf());
            for file in files {
                let name = file
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let ver = super::parse_version_prefix(&name).unwrap_or(0) as i64;
                if ver <= current {
                    continue;
                }
                let sql = file
                    .contents_utf8()
                    .ok_or_else(|| anyhow::anyhow!("invalid utf-8 in migration {}", name))?;
                if let Err(e) = self.exec_batch(&conn, sql) {
                    if name.contains("fts") || sql.to_lowercase().contains("virtual table") {
                        // Skip FTS migration if unsupported
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                let _ = self.exec_batch(&conn, &format!("PRAGMA user_version = {}", ver));
            }
            // best-effort FTS rebuild
            let _ = self.exec_batch(&conn, "INSERT INTO clips_fts(clips_fts) VALUES('rebuild')");
            Ok(())
        }
    }

    impl Store for LibsqlStore {
        fn init(&self) -> anyhow::Result<()> {
            self.run_migrations()
        }

        fn add(&self, text: &str) -> anyhow::Result<Clip> {
            let id = super::gen_id();
            let created_at = OffsetDateTime::now_utc().unix_timestamp();
            let conn = self.db.connect()?;
            let id_clone = id.clone();
            self.rt.block_on(async {
                conn.execute(
                    "INSERT INTO clips(id, kind, text, created_at, is_favorite) VALUES(?1, 'text', ?2, ?3, 0)",
                    libsql::params!(id_clone, text, created_at),
                ).await
            })?;
            Ok(Clip {
                id,
                text: text.into(),
                created_at: OffsetDateTime::from_unix_timestamp(created_at)?,
                last_used_at: None,
                is_favorite: false,
                kind: ClipKind::Text,
                is_image: false,
                image_path: None,
            })
        }

        fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
            let conn = self.db.connect()?;
            let mut sql = String::from("SELECT id, text, created_at, is_favorite, last_used_at FROM clips WHERE deleted_at IS NULL AND kind = 'text'");
            if q.favorites_only {
                sql.push_str(" AND is_favorite = 1");
            }
            let rows = if let Some(term) = &q.contains {
                sql.push_str(" AND text LIKE ? ORDER BY created_at DESC");
                self.rt.block_on(async {
                    conn.query(&sql, libsql::params!(format!("%{}%", term)))
                        .await
                })?
            } else {
                sql.push_str(" ORDER BY created_at DESC");
                self.rt.block_on(async { conn.query(&sql, ()).await })?
            };

            let mut out = Vec::new();
            let rows_vec: Vec<(String, String, i64, i64, Option<i64>)> =
                self.rt.block_on(async {
                    let mut rows = rows;
                    let mut tmp = Vec::new();
                    loop {
                        match rows.next().await {
                            Ok(Some(r)) => {
                                let id: String = r.get::<String>(0)?;
                                let text: String = r.get::<String>(1)?;
                                let created: i64 = r.get::<i64>(2)?;
                                let fav: i64 = r.get::<i64>(3)?;
                                let last: Option<i64> = r.get::<Option<i64>>(4)?;
                                tmp.push((id, text, created, fav, last));
                            }
                            Ok(None) => break,
                            Err(e) => return Err(e),
                        }
                    }
                    Ok::<_, libsql::Error>(tmp)
                })?;
            for (id, text, created, fav, last) in rows_vec {
                let created_at = OffsetDateTime::from_unix_timestamp(created)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                out.push(Clip {
                    id,
                    text,
                    created_at,
                    last_used_at: last.and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                    is_favorite: fav != 0,
                    kind: ClipKind::Text,
                    is_image: false,
                    image_path: None,
                });
            }
            if let Some(limit) = q.limit {
                out.truncate(limit);
            }
            Ok(out)
        }

        fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
            let conn = self.db.connect()?;
            let mut rows = self.rt.block_on(async { conn.query("SELECT id, kind, text, created_at, is_favorite, last_used_at FROM clips WHERE id = ? AND deleted_at IS NULL", libsql::params!(id)).await })?;
            let opt = self.rt.block_on(async {
                match rows.next().await? {
                    Some(r) => {
                        let created: i64 = r.get::<i64>(3)?;
                        let last: Option<i64> = r.get::<Option<i64>>(5)?;
                        let kind: String = r.get::<String>(1)?;
                        let created_at = OffsetDateTime::from_unix_timestamp(created)
                            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                        let kind = if kind == "image" {
                            ClipKind::Image
                        } else {
                            ClipKind::Text
                        };
                        Ok::<Option<Clip>, libsql::Error>(Some(Clip {
                            id: r.get::<String>(0)?,
                            text: r.get::<String>(2)?,
                            created_at,
                            last_used_at: last
                                .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
                            is_favorite: {
                                let v: i64 = r.get::<i64>(4)?;
                                v != 0
                            },
                            kind,
                            is_image: matches!(kind, ClipKind::Image),
                            image_path: None,
                        }))
                    }
                    None => Ok::<Option<Clip>, libsql::Error>(None),
                }
            })?;
            Ok(opt)
        }

        fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()> {
            let conn = self.db.connect()?;
            self.rt.block_on(async {
                conn.execute(
                    "UPDATE clips SET is_favorite = ? WHERE id = ?",
                    libsql::params!(if fav { 1 } else { 0 }, id),
                )
                .await
            })?;
            Ok(())
        }

        fn delete(&self, id: &str) -> anyhow::Result<()> {
            let conn = self.db.connect()?;
            self.rt.block_on(async {
                conn.execute("DELETE FROM clips WHERE id = ?", libsql::params!(id))
                    .await
            })?;
            Ok(())
        }

        fn touch_last_used(&self, id: &str) -> anyhow::Result<()> {
            let conn = self.db.connect()?;
            let now = OffsetDateTime::now_utc().unix_timestamp();
            self.rt.block_on(async {
                conn.execute(
                    "UPDATE clips SET last_used_at = ?, updated_at = COALESCE(updated_at, ?), lamport = COALESCE(lamport, 0) + 1 WHERE id = ?",
                    libsql::params!(now, now, id),
                )
                .await
            })?;
            Ok(())
        }

        fn clear(&self) -> anyhow::Result<()> {
            let conn = self.db.connect()?;
            self.rt
                .block_on(async { conn.execute("DELETE FROM clips", ()).await })?;
            Ok(())
        }

        fn add_tags(&self, _id: &str, _tags: &[String]) -> anyhow::Result<()> {
            // Tags not supported in remote backend in this scaffold
            Ok(())
        }

        fn remove_tags(&self, _id: &str, _tags: &[String]) -> anyhow::Result<()> {
            Ok(())
        }

        fn list_tags(&self, _id: &str) -> anyhow::Result<Vec<String>> {
            Ok(Vec::new())
        }

        // Images currently unsupported in remote backend
        fn add_image_rgba(&self, _width: u32, _height: u32, _rgba: &[u8]) -> anyhow::Result<Clip> {
            anyhow::bail!("images are not supported in libsql backend yet")
        }
        fn get_image_meta(&self, _id: &str) -> anyhow::Result<Option<ImageMeta>> {
            Ok(None)
        }
        fn get_image_rgba(&self, _id: &str) -> anyhow::Result<Option<ImageRgba>> {
            Ok(None)
        }
        fn list_images(&self, _q: Query) -> anyhow::Result<Vec<(Clip, ImageMeta)>> {
            Ok(vec![])
        }
        fn prune(
            &self,
            max_items: Option<usize>,
            max_age: Option<time::Duration>,
            keep_favorites: bool,
        ) -> anyhow::Result<usize> {
            // Simplified prune: server-side deletes similar to SQLite version
            let conn = self.db.connect()?;
            let deleted = 0usize;
            if let Some(age) = max_age {
                let cutoff = (OffsetDateTime::now_utc() - age).unix_timestamp();
                let sql = if keep_favorites {
                    "DELETE FROM clips WHERE created_at < ? AND deleted_at IS NULL AND is_favorite = 0"
                } else {
                    "DELETE FROM clips WHERE created_at < ? AND deleted_at IS NULL"
                };
                self.rt
                    .block_on(async { conn.execute(sql, libsql::params!(cutoff)).await })?;
                // libsql doesn't expose changes(); we skip exact count here
            }
            if let Some(n) = max_items {
                let sql = if keep_favorites {
                    "DELETE FROM clips WHERE rowid IN (SELECT rowid FROM clips WHERE deleted_at IS NULL AND is_favorite = 0 ORDER BY created_at DESC LIMIT -1 OFFSET ?1)"
                } else {
                    "DELETE FROM clips WHERE rowid IN (SELECT rowid FROM clips WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT -1 OFFSET ?1)"
                };
                self.rt
                    .block_on(async { conn.execute(sql, libsql::params!(n as i64)).await })?;
            }
            Ok(deleted)
        }
    }
}
