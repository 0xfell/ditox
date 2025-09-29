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
}

pub trait Store: Send + Sync {
    fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn add(&self, text: &str) -> anyhow::Result<Clip>;
    fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>>;
    fn get(&self, id: &str) -> anyhow::Result<Option<Clip>>;
    fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()>;
    fn delete(&self, id: &str) -> anyhow::Result<()>;
    fn clear(&self) -> anyhow::Result<()>;
    // Images
    fn add_image_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<Clip>;
    fn get_image_meta(&self, id: &str) -> anyhow::Result<Option<ImageMeta>>;
    fn get_image_rgba(&self, id: &str) -> anyhow::Result<Option<ImageRgba>>;
    fn list_images(&self, q: Query) -> anyhow::Result<Vec<(Clip, ImageMeta)>>;
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
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            images: RwLock::new(std::collections::HashMap::new()),
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
        let clip = Clip::new(gen_id(), text.to_string());
        let mut v = self.inner.write().expect("poisoned");
        v.insert(0, clip.clone());
        Ok(clip)
    }

    fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
        let v = self.inner.read().expect("poisoned");
        let mut items: Vec<Clip> = v
            .iter()
            .filter(|c| !q.favorites_only || c.is_favorite)
            .filter(|c| matches!(c.kind, ClipKind::Text))
            .filter(|c| match &q.contains {
                Some(s) => c.text.to_lowercase().contains(&s.to_lowercase()),
                None => true,
            })
            .cloned()
            .collect();
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
        Ok(())
    }

    fn add_image_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<Clip> {
        let id = gen_id();
        let clip = Clip {
            id: id.clone(),
            text: String::new(),
            created_at: OffsetDateTime::now_utc(),
            is_favorite: false,
            kind: ClipKind::Image,
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
        let mut out = Vec::new();
        for c in v.iter().filter(|c| matches!(c.kind, ClipKind::Image)) {
            if q.favorites_only && !c.is_favorite {
                continue;
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

    #[cfg(all(feature = "clipboard", target_os = "linux"))]
    pub struct ArboardClipboard;

    #[cfg(all(feature = "clipboard", target_os = "linux"))]
    impl Default for ArboardClipboard {
        fn default() -> Self {
            Self
        }
    }

    #[cfg(all(feature = "clipboard", target_os = "linux"))]
    impl ArboardClipboard {
        pub fn new() -> Self {
            Self
        }
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
    use image::{ImageFormat, ImageReader};
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
            conn.execute(
                "INSERT INTO clips(id, kind, text, created_at, is_favorite) VALUES(?, 'text', ?, ?, 0)",
                params![id, text, created_at],
            )?;
            let clip = Clip {
                id,
                text: text.to_string(),
                created_at: OffsetDateTime::from_unix_timestamp(created_at)?,
                is_favorite: false,
                kind: ClipKind::Text,
            };
            Ok(clip)
        }

        fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
            let conn = self.conn.lock().unwrap();
            let mut sql = String::from("SELECT id, text, created_at, is_favorite FROM clips WHERE deleted_at IS NULL AND kind = 'text'");
            if q.favorites_only {
                sql.push_str(" AND is_favorite = 1");
            }
            if let Some(term) = &q.contains {
                // Try FTS path first
                let has_fts = conn
                    .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='clips_fts'")?
                    .exists([])?;
                if has_fts {
                    sql = String::from("SELECT c.id, c.text, c.created_at, c.is_favorite FROM clips c JOIN clips_fts f ON f.rowid = c.rowid WHERE c.deleted_at IS NULL AND c.kind = 'text'");
                    if q.favorites_only {
                        sql.push_str(" AND c.is_favorite = 1");
                    }
                    sql.push_str(" AND f.text MATCH ?1 ORDER BY c.created_at DESC");
                    let mut stmt = conn.prepare(&sql)?;
                    let mut rows = stmt.query([term])?;
                    let mut out = Vec::new();
                    while let Some(row) = rows.next()? {
                        let created: i64 = row.get(2)?;
                        out.push(Clip {
                            id: row.get(0)?,
                            text: row.get(1)?,
                            created_at: OffsetDateTime::from_unix_timestamp(created)?,
                            is_favorite: row.get::<_, i64>(3)? != 0,
                            kind: ClipKind::Text,
                        });
                    }
                    if let Some(limit) = q.limit {
                        out.truncate(limit);
                    }
                    return Ok(out);
                } else {
                    sql.push_str(" AND text LIKE ?1");
                    let like = format!("%{}%", term);
                    sql.push_str(" ORDER BY created_at DESC");
                    if let Some(limit) = q.limit {
                        sql.push_str(&format!(" LIMIT {}", limit));
                    }
                    let mut stmt = conn.prepare(&sql)?;
                    let mut rows = stmt.query([like])?;
                    let mut out = Vec::new();
                    while let Some(row) = rows.next()? {
                        let created: i64 = row.get(2)?;
                        out.push(Clip {
                            id: row.get(0)?,
                            text: row.get(1)?,
                            created_at: OffsetDateTime::from_unix_timestamp(created)?,
                            is_favorite: row.get::<_, i64>(3)? != 0,
                            kind: ClipKind::Text,
                        });
                    }
                    return Ok(out);
                }
            }
            sql.push_str(" ORDER BY created_at DESC");
            if let Some(limit) = q.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let created: i64 = row.get(2)?;
                out.push(Clip {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    created_at: OffsetDateTime::from_unix_timestamp(created)?,
                    is_favorite: row.get::<_, i64>(3)? != 0,
                    kind: ClipKind::Text,
                });
            }
            Ok(out)
        }

        fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, kind, text, created_at, is_favorite FROM clips WHERE id = ? AND deleted_at IS NULL")?;
            let opt = stmt
                .query_row([id], |row| {
                    let created: i64 = row.get(3)?;
                    let kind_str: String = row.get(1)?;
                    let kind = if kind_str == "image" {
                        ClipKind::Image
                    } else {
                        ClipKind::Text
                    };
                    Ok(Clip {
                        id: row.get(0)?,
                        text: row.get(2)?,
                        created_at: OffsetDateTime::from_unix_timestamp(created)
                            .unwrap_or(OffsetDateTime::now_utc()),
                        is_favorite: row.get::<_, i64>(4)? != 0,
                        kind,
                    })
                })
                .optional()?;
            Ok(opt)
        }

        fn favorite(&self, id: &str, fav: bool) -> anyhow::Result<()> {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "UPDATE clips SET is_favorite = ? WHERE id = ?",
                params![if fav { 1 } else { 0 }, id],
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
            tx.execute("INSERT INTO clips(id, kind, text, created_at, is_favorite) VALUES(?, 'image', '', ?, 0)", params![id, created_at])?;
            tx.execute("INSERT INTO images(clip_id, format, width, height, size_bytes, sha256, thumb_path) VALUES(?, 'png', ?, ?, ?, ?, NULL)", params![id, width as i64, height as i64, size as i64, sha])?;
            tx.commit()?;
            Ok(Clip {
                id,
                text: String::new(),
                created_at: OffsetDateTime::from_unix_timestamp(created_at)?,
                is_favorite: false,
                kind: ClipKind::Image,
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
            let mut sql = String::from("SELECT c.id, c.created_at, c.is_favorite, i.format, i.width, i.height, i.size_bytes, i.sha256, i.thumb_path FROM clips c JOIN images i ON i.clip_id = c.id WHERE c.deleted_at IS NULL AND c.kind = 'image'");
            if q.favorites_only {
                sql.push_str(" AND c.is_favorite = 1");
            }
            sql.push_str(" ORDER BY c.created_at DESC");
            if let Some(limit) = q.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let created: i64 = row.get(1)?;
                let clip = Clip {
                    id: row.get(0)?,
                    text: String::new(),
                    created_at: OffsetDateTime::from_unix_timestamp(created)?,
                    is_favorite: row.get::<_, i64>(2)? != 0,
                    kind: ClipKind::Image,
                };
                let meta = ImageMeta {
                    format: row.get(3)?,
                    width: row.get::<_, i64>(4)? as u32,
                    height: row.get::<_, i64>(5)? as u32,
                    size_bytes: row.get::<_, i64>(6)? as u64,
                    sha256: row.get(7)?,
                    thumb_path: row.get(8)?,
                };
                out.push((clip, meta));
            }
            Ok(out)
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
                    conn.execute(stmt, ())
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
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
                is_favorite: false,
                kind: ClipKind::Text,
            })
        }

        fn list(&self, q: Query) -> anyhow::Result<Vec<Clip>> {
            let conn = self.db.connect()?;
            let mut sql = String::from("SELECT id, text, created_at, is_favorite FROM clips WHERE deleted_at IS NULL AND kind = 'text'");
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
            let rows_vec: Vec<(String, String, i64, i64)> = self.rt.block_on(async {
                let mut rows = rows;
                let mut tmp = Vec::new();
                loop {
                    match rows.next().await {
                        Ok(Some(r)) => {
                            let id: String = r.get::<String>(0)?;
                            let text: String = r.get::<String>(1)?;
                            let created: i64 = r.get::<i64>(2)?;
                            let fav: i64 = r.get::<i64>(3)?;
                            tmp.push((id, text, created, fav));
                        }
                        Ok(None) => break,
                        Err(e) => return Err(e),
                    }
                }
                Ok::<_, libsql::Error>(tmp)
            })?;
            for (id, text, created, fav) in rows_vec {
                let created_at = OffsetDateTime::from_unix_timestamp(created)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                out.push(Clip {
                    id,
                    text,
                    created_at,
                    is_favorite: fav != 0,
                    kind: ClipKind::Text,
                });
            }
            if let Some(limit) = q.limit {
                out.truncate(limit);
            }
            Ok(out)
        }

        fn get(&self, id: &str) -> anyhow::Result<Option<Clip>> {
            let conn = self.db.connect()?;
            let mut rows = self.rt.block_on(async { conn.query("SELECT id, kind, text, created_at, is_favorite FROM clips WHERE id = ? AND deleted_at IS NULL", libsql::params!(id)).await })?;
            let opt = self.rt.block_on(async {
                match rows.next().await? {
                    Some(r) => {
                        let created: i64 = r.get::<i64>(3)?;
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
                            is_favorite: {
                                let v: i64 = r.get::<i64>(4)?;
                                v != 0
                            },
                            kind,
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

        fn clear(&self) -> anyhow::Result<()> {
            let conn = self.db.connect()?;
            self.rt
                .block_on(async { conn.execute("DELETE FROM clips", ()).await })?;
            Ok(())
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
            let mut deleted = 0usize;
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
