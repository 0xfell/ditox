use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::config;
use ditox_core::Store;
use ditox_core::clipboard::Clipboard as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonMode {
    Managed,
    External,
    Off,
}

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub sample: Duration,
    pub images: bool,
    pub image_cap_bytes: Option<usize>,
}

pub struct ManagedHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    lock_path: PathBuf,
    paused: Arc<AtomicBool>,
    images_on: Arc<AtomicBool>,
    sample: Duration,
}

impl ManagedHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        let _ = fs::remove_file(&self.lock_path);
    }

    pub fn control(&self) -> ManagedControl {
        ManagedControl {
            paused: self.paused.clone(),
            images_on: self.images_on.clone(),
            sample: self.sample,
        }
    }
}

pub fn detect_external_clipd() -> bool {
    // Try clipd.json then optional TCP connect as confirmation
    let info_path = config::config_dir().join("clipd.json");
    if let Ok(s) = fs::read_to_string(&info_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            if let Some(p) = v.get("port").and_then(|p| p.as_u64()) {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], p as u16));
                if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                    return true;
                }
            }
        }
        // If json exists but connection fails, treat as not running
        return false;
    }
    false
}

fn managed_lock_path() -> PathBuf {
    config::state_dir().join("managed-daemon.lock")
}

fn try_create_lock() -> Result<File> {
    let path = managed_lock_path();
    if let Some(dir) = path.parent() { let _ = fs::create_dir_all(dir); }
    // Attempt exclusive create; if exists, check staleness
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut f) => {
            let pid = std::process::id();
            let started = time::OffsetDateTime::now_utc().unix_timestamp();
            writeln!(f, "pid={}\nstarted_at_unix={}\nowner=managed", pid, started)?;
            Ok(f)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Check alive
            let mut s = String::new();
            if let Ok(mut rf) = File::open(&path) {
                let _ = rf.read_to_string(&mut s);
            }
            let pid: Option<u32> = s
                .lines()
                .find_map(|l| l.strip_prefix("pid=")?.parse::<u32>().ok());
            let alive = pid.map(is_pid_alive).unwrap_or(false);
            if !alive {
                let _ = fs::remove_file(&path);
                return OpenOptions::new().write(true).create_new(true).open(&path).map_err(Into::into);
            }
            anyhow::bail!("managed-daemon: another capturer active (lock held)")
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(target_os = "linux")]
fn is_pid_alive(pid: u32) -> bool {
    // Best-effort: /proc
    std::path::Path::new("/proc").join(pid.to_string()).exists()
}
#[cfg(not(target_os = "linux"))]
fn is_pid_alive(_pid: u32) -> bool { false }

#[cfg(target_os = "linux")]
fn clipboard() -> ditox_core::clipboard::ArboardClipboard { ditox_core::clipboard::ArboardClipboard::new() }
#[cfg(not(target_os = "linux"))]
fn clipboard() -> ditox_core::clipboard::NoopClipboard { ditox_core::clipboard::NoopClipboard }

pub fn start_managed<S>(store: Arc<S>, cfg: DaemonConfig) -> Result<ManagedHandle>
where
    S: Store + 'static,
{
    let _lock = try_create_lock()?; // ensures single-instance per user session
    let lock_path = managed_lock_path();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let paused = Arc::new(AtomicBool::new(false));
    let paused2 = paused.clone();
    let images_on = Arc::new(AtomicBool::new(cfg.images));
    let images_on2 = images_on.clone();

    let join = thread::spawn(move || {
        let cb = clipboard();
        let last_text = Arc::new(Mutex::new(String::new()));
        let last_img = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sample = cfg.sample;
        let cap = cfg.image_cap_bytes;
        loop {
            if stop2.load(Ordering::SeqCst) { break; }
            if paused2.load(Ordering::SeqCst) {
                std::thread::sleep(sample);
                continue;
            }
            // Text path
            if let Ok(Some(mut text)) = cb.get_text() {
                if text.ends_with('\n') { text.pop(); }
                let mut lt = last_text.lock().unwrap();
                if *lt != text {
                    // Try to find existing recent identical entry; else insert
                    let found = match store.list(ditox_core::Query{ contains: None, favorites_only: false, limit: Some(50), tag: None, rank: false }) {
                        Ok(mut v) => {
                            v.iter().find(|c| c.text == text).map(|c| c.id.clone())
                        }
                        Err(_) => None,
                    };
                    if let Some(id) = found { let _ = store.touch_last_used(&id); } else { let _ = store.add(&text); }
                    *lt = text;
                }
            }
            // Image path (optional)
            if images_on2.load(Ordering::SeqCst) {
                if let Ok(Some(img)) = cb.get_image() {
                    let bytes = &img.bytes;
                    if let Some(maxb) = cap { if bytes.len() > maxb { /* skip oversized */ } else {
                        let mut li = last_img.lock().unwrap();
                        if *li != *bytes {
                            let _ = store.add_image_rgba(img.width, img.height, bytes);
                            *li = bytes.clone();
                        }
                    }} else {
                        let mut li = last_img.lock().unwrap();
                        if *li != *bytes {
                            let _ = store.add_image_rgba(img.width, img.height, bytes);
                            *li = bytes.clone();
                        }
                    }
                }
            }
            std::thread::sleep(sample);
        }
    });

    Ok(ManagedHandle { stop, join: Some(join), lock_path, paused, images_on, sample: cfg.sample })
}

#[derive(Clone, Debug)]
pub struct ManagedControl {
    paused: Arc<AtomicBool>,
    images_on: Arc<AtomicBool>,
    sample: Duration,
}

impl ManagedControl {
    pub fn toggle_pause(&self) -> bool {
        let v = !self.paused.load(Ordering::SeqCst);
        self.paused.store(v, Ordering::SeqCst);
        v
    }
    pub fn is_paused(&self) -> bool { self.paused.load(Ordering::SeqCst) }
    pub fn images_on(&self) -> bool { self.images_on.load(Ordering::SeqCst) }
    pub fn toggle_images(&self) -> bool {
        let v = !self.images_on.load(Ordering::SeqCst);
        self.images_on.store(v, Ordering::SeqCst);
        v
    }
    pub fn sample(&self) -> Duration { self.sample }
}
