//! ditox-core: core types, storage traits, and minimal in-memory store

use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use time::{OffsetDateTime};

pub type ClipId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub text: String,
    pub created_at: OffsetDateTime,
    pub is_favorite: bool,
}

impl Clip {
    pub fn new<S: Into<String>>(id: ClipId, text: S) -> Self {
        Self { id, text: text.into(), created_at: OffsetDateTime::now_utc(), is_favorite: false }
    }
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
}

