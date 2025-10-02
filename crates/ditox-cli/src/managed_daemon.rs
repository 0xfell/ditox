use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config;
use ditox_core::{clipboard::Clipboard as _, Store};

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub sample_ms: u64,
    pub images: bool,
}

pub struct ManagedControl {
    paused: AtomicBool,
    images: AtomicBool,
    sample_ms: AtomicU64,
    shutdown: AtomicBool,
}

impl ManagedControl {
    pub fn new(sample_ms: u64, images: bool) -> Self {
        Self {
            paused: AtomicBool::new(false),
            images: AtomicBool::new(images),
            sample_ms: AtomicU64::new(sample_ms),
            shutdown: AtomicBool::new(false),
        }
    }
    pub fn toggle_pause(&self) {
        self.paused
            .store(!self.paused.load(Ordering::Relaxed), Ordering::Relaxed);
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
    pub fn toggle_images(&self) {
        self.images
            .store(!self.images.load(Ordering::Relaxed), Ordering::Relaxed);
    }
    pub fn images_on(&self) -> bool {
        self.images.load(Ordering::Relaxed)
    }
    #[allow(dead_code)]
    pub fn set_sample_ms(&self, ms: u64) {
        self.sample_ms.store(ms, Ordering::Relaxed);
    }
    pub fn sample_ms(&self) -> u64 {
        self.sample_ms.load(Ordering::Relaxed)
    }
    fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
    fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

pub struct ManagedHandle {
    join: Option<JoinHandle<()>>,
    lock_path: PathBuf,
    ctrl: Arc<ManagedControl>,
}

impl ManagedHandle {
    pub fn control(&self) -> Arc<ManagedControl> {
        self.ctrl.clone()
    }
}

impl Drop for ManagedHandle {
    fn drop(&mut self) {
        self.ctrl.request_shutdown();
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipdInfo {
    port: u16,
}

pub fn detect_external_clipd() -> bool {
    let info_path = config::config_dir().join("clipd.json");
    if let Ok(bytes) = fs::read(&info_path) {
        if let Ok(info) = serde_json::from_slice::<ClipdInfo>(&bytes) {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], info.port));
            if std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                return true;
            }
        }
    }
    false
}

fn lock_path() -> PathBuf {
    config::state_dir().join("managed-daemon.lock")
}

fn is_pid_alive(pid: i32) -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Conservative default on non-Linux
        let _ = pid;
        false
    }
}

pub fn try_create_lock() -> Result<PathBuf> {
    let lp = lock_path();
    if let Some(parent) = lp.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if lp.exists() {
        if let Ok(s) = fs::read_to_string(&lp) {
            let pid: i32 = s.trim().parse().unwrap_or(0);
            if pid > 0 && is_pid_alive(pid) {
                anyhow::bail!("managed lock active (pid {})", pid);
            } else {
                let _ = fs::remove_file(&lp);
            }
        }
    }
    let mut f = std::fs::File::create(&lp)?;
    writeln!(f, "{}", std::process::id())?;
    Ok(lp)
}

static GLOBAL_CTRL: once_cell::sync::OnceCell<Arc<ManagedControl>> =
    once_cell::sync::OnceCell::new();

pub fn set_global_control(ctrl: Arc<ManagedControl>) {
    let _ = GLOBAL_CTRL.set(ctrl);
}
pub fn global_control() -> Option<Arc<ManagedControl>> {
    GLOBAL_CTRL.get().cloned()
}

#[cfg(target_os = "linux")]
fn run_loop(store: Arc<dyn Store>, ctrl: Arc<ManagedControl>) {
    use ditox_core::clipboard::ArboardClipboard as SystemClipboard;
    let mut last_text: Option<String> = None;
    let mut last_img_sig: Option<(u32, u32, usize)> = None; // width,height,size
    loop {
        if ctrl.should_shutdown() {
            break;
        }
        let sleep_ms = ctrl.sample_ms();
        if ctrl.is_paused() {
            sleep(Duration::from_millis(sleep_ms));
            continue;
        }
        // Text
        let cb = SystemClipboard::new();
        if let Ok(Some(txt)) = cb.get_text() {
            if last_text.as_deref() != Some(txt.as_str()) {
                let _ = store.add(&txt);
                last_text = Some(txt);
            }
        }
        // Images (optional)
        if ctrl.images_on() {
            if let Ok(Some(img)) = cb.get_image() {
                let sig = (img.width, img.height, img.bytes.len());
                if last_img_sig != Some(sig) {
                    let _ = store.add_image_rgba(img.width, img.height, &img.bytes);
                    last_img_sig = Some(sig);
                }
            }
        }
        sleep(Duration::from_millis(sleep_ms));
    }
}

#[cfg(not(target_os = "linux"))]
fn run_loop(_store: Arc<dyn Store>, ctrl: Arc<ManagedControl>) {
    // No-op watcher on non-Linux platforms
    while !ctrl.should_shutdown() {
        sleep(Duration::from_millis(ctrl.sample_ms()));
    }
}

pub fn start_managed(store: Arc<dyn Store>, cfg: DaemonConfig) -> Result<ManagedHandle> {
    let lp = try_create_lock()?;
    let ctrl = Arc::new(ManagedControl::new(cfg.sample_ms, cfg.images));
    let ctrl_clone = ctrl.clone();
    let store_clone = store.clone();
    let join = std::thread::spawn(move || run_loop(store_clone, ctrl_clone));
    Ok(ManagedHandle {
        join: Some(join),
        lock_path: lp,
        ctrl,
    })
}

// no status formatter; picker reads DITOX_CAPTURE_STATUS for footer
