use anyhow::Result;
use ditox_core::{Clip, ImageMeta, ImageRgba, Query, Store};
use std::sync::Mutex;

enum BackendInit {
    LocalSqlite(std::path::PathBuf, bool),
    #[cfg(feature = "libsql")]
    RemoteLibsql {
        url: String,
        token: Option<String>,
    },
}

pub struct LazyStore {
    init: Mutex<Option<BackendInit>>, // consumed on first open
    inner: Mutex<Option<Box<dyn Store>>>,
}

impl LazyStore {
    pub fn local_sqlite(path: std::path::PathBuf, auto_migrate: bool) -> Self {
        Self {
            init: Mutex::new(Some(BackendInit::LocalSqlite(path, auto_migrate))),
            inner: Mutex::new(None),
        }
    }
    #[cfg(feature = "libsql")]
    pub fn remote_libsql(url: String, token: Option<String>) -> Self {
        Self {
            init: Mutex::new(Some(BackendInit::RemoteLibsql { url, token })),
            inner: Mutex::new(None),
        }
    }

    fn ensure_open(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.is_some() {
            return Ok(());
        }
        let mut init = self.init.lock().unwrap();
        if let Some(backend) = init.take() {
            match backend {
                BackendInit::LocalSqlite(path, auto_migrate) => {
                    if let Some(dir) = path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    let s = ditox_core::StoreImpl::new_with(&path, auto_migrate)?;
                    *inner = Some(Box::new(s));
                }
                #[cfg(feature = "libsql")]
                BackendInit::RemoteLibsql { url, token } => {
                    let s = ditox_core::libsql_backend::LibsqlStore::new(&url, token.as_deref())?;
                    *inner = Some(Box::new(s));
                }
            }
        }
        Ok(())
    }

    fn get(&self) -> Result<std::sync::MutexGuard<'_, Option<Box<dyn Store>>>> {
        self.ensure_open()?;
        Ok(self.inner.lock().unwrap())
    }
}

impl Store for LazyStore {
    fn init(&self) -> Result<()> {
        Ok(())
    }
    fn add(&self, text: &str) -> Result<Clip> {
        self.get()?.as_ref().unwrap().add(text)
    }
    fn list(&self, q: Query) -> Result<Vec<Clip>> {
        self.get()?.as_ref().unwrap().list(q)
    }
    fn get(&self, id: &str) -> Result<Option<Clip>> {
        self.get()?.as_ref().unwrap().get(id)
    }
    fn touch_last_used(&self, id: &str) -> Result<()> {
        self.get()?.as_ref().unwrap().touch_last_used(id)
    }
    fn favorite(&self, id: &str, fav: bool) -> Result<()> {
        self.get()?.as_ref().unwrap().favorite(id, fav)
    }
    fn delete(&self, id: &str) -> Result<()> {
        self.get()?.as_ref().unwrap().delete(id)
    }
    fn clear(&self) -> Result<()> {
        self.get()?.as_ref().unwrap().clear()
    }
    fn add_tags(&self, id: &str, tags: &[String]) -> Result<()> {
        self.get()?.as_ref().unwrap().add_tags(id, tags)
    }
    fn remove_tags(&self, id: &str, tags: &[String]) -> Result<()> {
        self.get()?.as_ref().unwrap().remove_tags(id, tags)
    }
    fn list_tags(&self, id: &str) -> Result<Vec<String>> {
        self.get()?.as_ref().unwrap().list_tags(id)
    }
    fn add_image_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> Result<Clip> {
        self.get()?
            .as_ref()
            .unwrap()
            .add_image_rgba(width, height, rgba)
    }
    fn get_image_meta(&self, id: &str) -> Result<Option<ImageMeta>> {
        self.get()?.as_ref().unwrap().get_image_meta(id)
    }
    fn get_image_rgba(&self, id: &str) -> Result<Option<ImageRgba>> {
        self.get()?.as_ref().unwrap().get_image_rgba(id)
    }
    fn list_images(&self, q: Query) -> Result<Vec<(Clip, ImageMeta)>> {
        self.get()?.as_ref().unwrap().list_images(q)
    }
    fn add_image_from_path(&self, path: &std::path::Path) -> Result<Clip> {
        self.get()?.as_ref().unwrap().add_image_from_path(path)
    }
    fn prune(
        &self,
        max_items: Option<usize>,
        max_age: Option<time::Duration>,
        keep_favorites: bool,
    ) -> Result<usize> {
        self.get()?
            .as_ref()
            .unwrap()
            .prune(max_items, max_age, keep_favorites)
    }
}
