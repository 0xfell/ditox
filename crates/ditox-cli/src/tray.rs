#![cfg(feature = "tray")]
use anyhow::Result;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config;
use crate::lazy_store::LazyStore;
use crate::managed_daemon::{self, DaemonConfig};

pub fn run_tray() -> Result<()> {
    // Resolve DB path and start managed capture unless external clipd is present
    let settings = config::load_settings();
    let db_path = match &settings.storage {
        config::Storage::LocalSqlite { db_path } => db_path.clone().unwrap_or_else(default_db_path),
        _ => default_db_path(),
    };
    let store = LazyStore::local_sqlite(db_path, false);
    let mut maybe_handle = None;
    if !managed_daemon::detect_external_clipd() {
        let cfg = DaemonConfig { sample: Duration::from_millis(200), images: true, image_cap_bytes: Some(8*1024*1024) };
        if let Ok(h) = managed_daemon::start_managed(Arc::new(store), cfg) {
            maybe_handle = Some(h);
        }
    }

    // Build tray
    use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent};
    use tray_icon::TrayIconBuilder;
    let mut menu = Menu::new();
    let pause_item = MenuItem::new("Pause Capture", true, None);
    let images_item = MenuItem::new("Images: On", true, None);
    let quit_item = PredefinedMenuItem::quit(Some("Quit"));
    menu.append(&pause_item)?;
    menu.append(&images_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit_item)?;

    // Simple 16x16 icon (cyan square)
    let (w, h) = (16, 16);
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            rgba[i] = 0; // R
            rgba[i + 1] = 200; // G
            rgba[i + 2] = 200; // B
            rgba[i + 3] = 255; // A
        }
    }
    let icon_opt = tray_icon::Icon::from_rgba(rgba, w as u32, h as u32).ok();
    let mut builder = TrayIconBuilder::new().with_menu(Box::new(menu.clone())).with_tooltip("Ditox");
    if let Some(ic) = icon_opt { builder = builder.with_icon(ic); }
    let _tray = builder.build()?;

    // Event loop: handle MenuEvent messages (non-blocking poll)
    let rx = MenuEvent::receiver();
    loop {
        while let Ok(event) = rx.try_recv() {
            let id = event.id;
            if id == quit_item.id() {
                return Ok(());
            }
            if id == pause_item.id() {
                if let Some(h) = &maybe_handle {
                    let c = h.control();
                    let now_paused = c.toggle_pause();
                    // set_text returns () on muda 0.13.x
                    pause_item.set_text(if now_paused { "Resume Capture" } else { "Pause Capture" });
                }
            }
            if id == images_item.id() {
                if let Some(h) = &maybe_handle {
                    let c = h.control();
                    let on = c.toggle_images();
                    images_item.set_text(if on { "Images: On" } else { "Images: Off" });
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn default_db_path() -> std::path::PathBuf {
    let cfg = config::config_dir();
    let p = cfg.join("db").join("ditox.db");
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    p
}
