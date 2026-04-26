use crate::capture::{clip_hash, CaptureSource, PollingCaptureSource, RawClip};
use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::db::Database;
use crate::entry::Entry;
use crate::error::{DitoxError, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

pub struct Watcher {
    db: Database,
    config: Config,
    /// Capture sources, polled in priority order. The default
    /// constructor installs a single legacy clipboard source; tests
    /// and Phase 1 backends inject their own via `with_sources`.
    sources: Vec<Box<dyn CaptureSource>>,
    /// Last `clip_hash` we processed. Used as a fast in-memory dedup
    /// short-circuit so we don't re-hash large images on every poll.
    /// Persistent dedup happens via `Database::exists_by_hash` against
    /// the inner content hash.
    last_hash: Option<String>,
    /// Held only while the daemon owns the lock. Dropped on exit so the
    /// kernel releases the flock automatically.
    _lock: Option<File>,
}

/// Heartbeat write cadence. The daemon refreshes this file every N
/// seconds so `--status` can detect "PID still in /proc but the
/// process is wedged" cases.
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
/// Heartbeat staleness threshold beyond which `--status` reports the
/// daemon as unresponsive.
pub const HEARTBEAT_STALE_AFTER_SECS: u64 = 30;

/// Path to the watcher PID file.
pub fn get_pid_file_path() -> Result<PathBuf> {
    Ok(Database::get_data_dir()?.join("watcher.pid"))
}

/// Path to the watcher lock file.
pub fn get_lock_file_path() -> Result<PathBuf> {
    Ok(Database::get_data_dir()?.join("watcher.lock"))
}

/// Path to the watcher heartbeat file.
pub fn get_heartbeat_file_path() -> Result<PathBuf> {
    Ok(Database::get_data_dir()?.join("watcher.heartbeat"))
}

/// Aggregated status report for `ditox watch --status`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WatcherStatus {
    /// Daemon PID if a PID file exists and is parseable.
    pub pid: Option<u32>,
    /// Whether the OS still reports the PID as running.
    pub pid_alive: bool,
    /// Whether the lock file is currently held by *some* process.
    pub locked: bool,
    /// Last heartbeat timestamp (Unix epoch seconds).
    pub last_heartbeat: Option<u64>,
    /// Seconds since the last heartbeat.
    pub heartbeat_age_secs: Option<u64>,
    /// True if heartbeat is fresh (< [`HEARTBEAT_STALE_AFTER_SECS`]).
    pub healthy: bool,
}

/// Probe the watcher's current state. Reads the PID file, lock file,
/// and heartbeat file and constructs a [`WatcherStatus`].
pub fn watcher_status() -> WatcherStatus {
    let pid = read_pid_file();
    let pid_alive = pid.map(is_process_running_by_pid).unwrap_or(false);

    // Lock probe: try to acquire the lock briefly. If we get it, the
    // daemon isn't holding it. If not, someone else is.
    let locked = is_lock_held();

    let last_heartbeat = read_heartbeat();
    let now = now_secs();
    let heartbeat_age_secs = last_heartbeat.map(|h| now.saturating_sub(h));

    let healthy = pid_alive
        && heartbeat_age_secs
            .map(|age| age < HEARTBEAT_STALE_AFTER_SECS)
            .unwrap_or(false);

    WatcherStatus {
        pid,
        pid_alive,
        locked,
        last_heartbeat,
        heartbeat_age_secs,
        healthy,
    }
}

/// Quick "is the watcher running" check (used by `ditox status`). True
/// only when both the PID is alive AND the heartbeat is fresh.
pub fn is_watcher_running() -> bool {
    watcher_status().healthy
}

/// Stop the running watcher. Sends SIGTERM (Unix) or invokes
/// `TerminateProcess` (Windows) on the PID from the PID file.
///
/// Return value semantics:
/// - `Ok(true)` — daemon was running and we asked it to stop.
/// - `Ok(false)` — no daemon was running.
/// - `Err(...)` — we found a daemon but couldn't signal it.
///
/// The caller should poll [`watcher_status`] for confirmation that the
/// daemon actually exited (typically within 1-3 seconds).
pub fn stop_watcher() -> Result<bool> {
    let pid = match read_pid_file() {
        Some(p) => p,
        None => return Ok(false),
    };
    if !is_process_running_by_pid(pid) {
        // Stale PID file — clean it up while we're here.
        let _ = std::fs::remove_file(get_pid_file_path()?);
        return Ok(false);
    }
    send_term(pid)?;
    Ok(true)
}

fn read_pid_file() -> Option<u32> {
    let path = get_pid_file_path().ok()?;
    if !path.exists() {
        return None;
    }
    let s = std::fs::read_to_string(&path).ok()?;
    s.trim().parse().ok()
}

fn read_heartbeat() -> Option<u64> {
    let path = get_heartbeat_file_path().ok()?;
    if !path.exists() {
        return None;
    }
    let s = std::fs::read_to_string(&path).ok()?;
    s.trim().parse().ok()
}

fn write_heartbeat() -> Result<()> {
    let path = get_heartbeat_file_path()?;
    // Atomic write: tmp + rename so a partial write isn't visible.
    let tmp = path.with_extension("heartbeat.tmp");
    let mut f = File::create(&tmp)?;
    write!(f, "{}", now_secs())?;
    f.sync_all()?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn is_lock_held() -> bool {
    let lock_path = match get_lock_file_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    if !lock_path.exists() {
        return false;
    }
    // Attempt non-blocking exclusive lock. If it succeeds, no one else
    // is holding the lock — release it immediately.
    match OpenOptions::new().read(true).write(true).open(&lock_path) {
        Ok(f) => match f.try_lock_exclusive() {
            Ok(()) => {
                let _ = fs2::FileExt::unlock(&f);
                false
            }
            Err(_) => true,
        },
        Err(_) => false,
    }
}

#[cfg(unix)]
fn is_process_running_by_pid(pid: u32) -> bool {
    // Send signal 0 to check if process exists.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
fn is_process_running_by_pid(pid: u32) -> bool {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.process(sysinfo::Pid::from_u32(pid)).is_some()
}

#[cfg(unix)]
fn send_term(pid: u32) -> Result<()> {
    let rc = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if rc == 0 {
        Ok(())
    } else {
        Err(DitoxError::Other(format!(
            "kill(pid={}, SIGTERM) failed: {}",
            pid,
            std::io::Error::last_os_error()
        )))
    }
}

#[cfg(windows)]
fn send_term(pid: u32) -> Result<()> {
    use sysinfo::{Pid, Signal, System};
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    if let Some(proc) = sys.process(Pid::from_u32(pid)) {
        // Try graceful first; fall back to kill().
        if proc.kill_with(Signal::Term).unwrap_or(false) {
            return Ok(());
        }
        if proc.kill() {
            return Ok(());
        }
    }
    Err(DitoxError::Other(format!(
        "could not signal pid {} on Windows",
        pid
    )))
}

fn write_pid_file() -> Result<()> {
    let pid_path = get_pid_file_path()?;
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pid = std::process::id();
    std::fs::write(&pid_path, pid.to_string())?;
    Ok(())
}

fn remove_runtime_files() {
    if let Ok(p) = get_pid_file_path() {
        let _ = std::fs::remove_file(p);
    }
    if let Ok(p) = get_heartbeat_file_path() {
        let _ = std::fs::remove_file(p);
    }
}

/// Attempt to acquire the watcher lock. Returns the locked file
/// (which must be kept alive for the duration of the daemon).
///
/// Errors with `DitoxError::Other("watcher already running …")` if
/// another process holds the lock. The error message includes the PID
/// when available so callers can show a useful CLI message.
fn acquire_lock() -> Result<File> {
    let lock_path = get_lock_file_path()?;
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // `truncate(false)` is explicit because we never want to wipe the
    // lock file's contents — only its OS-level lock state matters.
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    match f.try_lock_exclusive() {
        Ok(()) => Ok(f),
        Err(_) => {
            let other_pid = read_pid_file();
            let msg = match other_pid {
                Some(p) => format!("watcher already running (pid {})", p),
                None => "watcher already running (pid unknown)".to_string(),
            };
            Err(DitoxError::Other(msg))
        }
    }
}

impl Watcher {
    /// Build a watcher with the platform-default capture source: a
    /// polling adapter that reads images first (browser "Copy image"
    /// priority) then text from the OS clipboard.
    pub fn new(db: Database, config: Config) -> Self {
        let interval = config.general.poll_interval_ms;
        let source: Box<dyn CaptureSource> = Box::new(PollingCaptureSource::new(
            "legacy-clipboard",
            interval,
            legacy_clipboard_snapshot,
        ));
        Self::with_sources(db, config, vec![source])
    }

    /// Build a watcher with explicit capture sources. Used by tests
    /// for `MockCaptureSource` injection and by Phase 1 backends that
    /// stack multiple sources (event-driven Windows + polling Wayland
    /// fallback, etc.).
    pub fn with_sources(
        db: Database,
        config: Config,
        sources: Vec<Box<dyn CaptureSource>>,
    ) -> Self {
        Self {
            db,
            config,
            sources,
            last_hash: None,
            _lock: None,
        }
    }

    pub fn poll_interval_ms(&self) -> u64 {
        self.config.general.poll_interval_ms
    }

    /// One-shot poll. Used by the GUI's in-process watcher (which
    /// doesn't acquire the daemon lock — only one daemon at a time,
    /// but the GUI's watcher and the daemon are designed to coexist
    /// because dedup catches duplicates).
    pub fn poll_once(&mut self) -> Result<bool> {
        self.poll_internal()
    }

    /// Long-running daemon entry point. Acquires the lock, writes the
    /// PID file, installs signal handlers, runs the poll loop, and
    /// guarantees cleanup on exit (whether via signal, error, or
    /// natural exit).
    pub fn run(&mut self) -> Result<()> {
        // Acquire lock first — refuse to start a second daemon.
        let lock = acquire_lock()?;
        self._lock = Some(lock);

        // PID file + initial heartbeat. PID file isn't the source of
        // truth for "is a daemon running" (the lock is) but tools that
        // existed before this hardening still read it, and `--stop`
        // uses it to find the PID.
        write_pid_file()?;
        let _ = write_heartbeat();

        info!(
            "Starting clipboard watcher (pid={}, poll={}ms)",
            std::process::id(),
            self.config.general.poll_interval_ms
        );

        // Signal handler — sets the shutdown flag. Polled by the loop.
        // Using `ctrlc` for cross-platform handling (handles SIGINT,
        // SIGTERM on Unix; Ctrl+C and Ctrl+Break on Windows).
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_handler = shutdown.clone();
        if let Err(e) = ctrlc::set_handler(move || {
            shutdown_handler.store(true, Ordering::SeqCst);
        }) {
            warn!(
                "could not install signal handler: {} (Ctrl+C may not clean up)",
                e
            );
        }

        let result = self.run_loop(shutdown);

        // Cleanup runs whether the loop exited cleanly, with an error,
        // or because of a signal.
        remove_runtime_files();
        info!("watcher exited cleanly");
        result
    }

    fn run_loop(&mut self, shutdown: Arc<AtomicBool>) -> Result<()> {
        self.initialize_hash();

        let mut last_heartbeat = SystemTime::now();
        let heartbeat_period = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);

        while !shutdown.load(Ordering::SeqCst) {
            if let Err(e) = self.poll_internal() {
                error!("error polling clipboard: {}", e);
            }

            // Refresh heartbeat at most once per HEARTBEAT_INTERVAL_SECS.
            // We don't hammer the file system on every 250 ms poll.
            if last_heartbeat.elapsed().unwrap_or(heartbeat_period) >= heartbeat_period {
                if let Err(e) = write_heartbeat() {
                    debug!("heartbeat write failed: {}", e);
                }
                last_heartbeat = SystemTime::now();
            }

            std::thread::sleep(Duration::from_millis(self.config.general.poll_interval_ms));
        }

        Ok(())
    }

    /// Initialize the last hash with current clipboard content. We
    /// prime from each source's `current_snapshot()` in priority
    /// order so a restart while content is still on the clipboard
    /// doesn't cause us to re-capture it on the very next poll (that
    /// was bug #4 in the hunt).
    pub fn initialize_hash(&mut self) {
        for source in self.sources.iter() {
            match source.current_snapshot() {
                Ok(Some(clip)) => {
                    self.last_hash = Some(clip_hash(&clip));
                    debug!("initialized last_hash from source {}", source.name());
                    return;
                }
                Ok(None) => continue,
                Err(e) => {
                    debug!("source {} snapshot error: {}", source.name(), e);
                    continue;
                }
            }
        }
    }

    /// Internal poll that returns whether a new entry was captured.
    ///
    /// Flow (critical ordering — this is the fix for bugs #1 and #4):
    /// 1. Walk capture sources in priority order, take the first that
    ///    produces a snapshot.
    /// 2. Short-circuit if the clip hash is unchanged since last poll
    ///    (`last_hash`) — avoids re-hashing large images.
    /// 3. Short-circuit if DB already has a row with the inner content
    ///    hash (`exists_by_hash`) — no disk write, no insert.
    /// 4. Only then store the blob (content-addressed, atomic) AND
    ///    insert the DB row. Either both succeed or neither does.
    /// 5. Run LRU eviction; evicted image rows' blobs are pruned via
    ///    the persistent queue in `Database`.
    fn poll_internal(&mut self) -> Result<bool> {
        // First source that yields content wins. Priority order is the
        // order sources were registered (legacy default = single
        // source, so this is a no-op until Phase 1 adds X11 selection
        // alongside the OS clipboard).
        for idx in 0..self.sources.len() {
            let snap = self.sources[idx].current_snapshot()?;
            let Some(clip) = snap else { continue };
            return self.process_clip(clip);
        }
        Ok(false)
    }

    /// Convert a `RawClip` into a DB entry. Image formats take
    /// priority (browser "Copy image" puts both URL text and rendered
    /// image on the clipboard; we want the image). Phase 1 will
    /// extend this to capture *all* formats per clip rather than
    /// picking one — see `docs/tasks/planned/023-phase-1-multi-format-capture.md`.
    fn process_clip(&mut self, clip: RawClip) -> Result<bool> {
        let h = clip_hash(&clip);
        if self.last_hash.as_ref() == Some(&h) {
            return Ok(false);
        }

        let captured = if let Some(img_format) = clip.first_with_prefix("image/") {
            let extension = mime_to_extension(&img_format.mime);
            let inner_hash = Clipboard::hash(&img_format.bytes);
            if !self.db.exists_by_hash(&inner_hash)? {
                Database::store_image_blob(&inner_hash, extension, &img_format.bytes)?;
                let entry = Entry::new_image(
                    inner_hash.clone(),
                    img_format.bytes.len(),
                    extension.to_string(),
                );
                self.db.insert(&entry)?;
                info!(
                    "captured image entry: {} bytes ({}.{})",
                    entry.byte_size,
                    &inner_hash[..8.min(inner_hash.len())],
                    extension
                );
                self.run_cleanup()?;
                true
            } else {
                false
            }
        } else if let Some(text_format) = clip.first_with_prefix("text/plain") {
            // text/plain;charset=utf-8 — we always emit valid UTF-8 in
            // `RawClip::text`, so this round-trips. Future non-UTF-8
            // text formats (Phase 1 RTF) take a different branch.
            let text = String::from_utf8_lossy(&text_format.bytes).into_owned();
            let inner_hash = Clipboard::hash(text.as_bytes());
            if !self.db.exists_by_hash(&inner_hash)? {
                let entry = Entry::new_text(text);
                self.db.insert(&entry)?;
                info!("captured text entry: {} bytes", entry.byte_size);
                self.run_cleanup()?;
                true
            } else {
                false
            }
        } else {
            // No format we recognise. Phase 1 will widen this.
            debug!(
                "skipping clip with {} unrecognised format(s)",
                clip.formats.len()
            );
            false
        };
        self.last_hash = Some(h);
        Ok(captured)
    }

    fn run_cleanup(&mut self) -> Result<()> {
        let removed = self.db.cleanup_old(self.config.general.max_entries)?;
        if removed > 0 {
            debug!("cleaned up {} old entries", removed);
        }
        Ok(())
    }
}

/// Default capture closure used by `Watcher::new`. Reads images
/// first (priority), then text. Returns `None` when the clipboard is
/// empty.
fn legacy_clipboard_snapshot() -> Result<Option<RawClip>> {
    if let Some(img) = Clipboard::read_image()? {
        return Ok(Some(RawClip::image(img.bytes, &img.extension)));
    }
    if let Some(text) = Clipboard::get_text()? {
        return Ok(Some(RawClip::text(text)));
    }
    Ok(None)
}

/// Map a MIME type to the storage extension used by content-addressed
/// image blobs. Defaults to `png` for unknown image MIMEs.
fn mime_to_extension(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        _ => "png",
    }
}
