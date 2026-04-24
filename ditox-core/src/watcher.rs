use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::db::Database;
use crate::entry::Entry;
use crate::error::Result;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, info};

pub struct Watcher {
    db: Database,
    config: Config,
    last_hash: Option<String>,
}

/// Get the path to the watcher PID file
pub fn get_pid_file_path() -> Result<PathBuf> {
    Ok(Database::get_data_dir()?.join("watcher.pid"))
}

/// Check if the watcher daemon is currently running
pub fn is_watcher_running() -> bool {
    let pid_path = match get_pid_file_path() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if !pid_path.exists() {
        return false;
    }

    // Read PID from file
    let pid_str = match fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Check if process is running - platform specific
    is_process_running_by_pid(pid)
}

/// Check if a process with the given PID is running
#[cfg(unix)]
fn is_process_running_by_pid(pid: u32) -> bool {
    // Send signal 0 to check if process exists
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
fn is_process_running_by_pid(pid: u32) -> bool {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.process(sysinfo::Pid::from_u32(pid)).is_some()
}

/// Write the current process PID to the PID file
fn write_pid_file() -> Result<()> {
    let pid_path = get_pid_file_path()?;
    let pid = std::process::id();
    fs::write(&pid_path, pid.to_string())?;
    Ok(())
}

/// Remove the PID file
fn remove_pid_file() {
    if let Ok(pid_path) = get_pid_file_path() {
        let _ = fs::remove_file(pid_path);
    }
}

impl Watcher {
    pub fn new(db: Database, config: Config) -> Self {
        Self {
            db,
            config,
            last_hash: None,
        }
    }

    /// Get the configured poll interval
    pub fn poll_interval_ms(&self) -> u64 {
        self.config.general.poll_interval_ms
    }

    /// Poll clipboard once and return true if a new entry was captured
    /// This is designed for use in async contexts (GUI, etc.)
    pub fn poll_once(&mut self) -> Result<bool> {
        self.poll_internal()
    }

    /// Main loop - runs forever, polling clipboard
    pub fn run(&mut self) -> Result<()> {
        info!("Starting clipboard watcher (poll interval: {}ms)", self.config.general.poll_interval_ms);

        // Write PID file
        write_pid_file()?;

        // Set up cleanup on exit
        let result = self.run_loop();

        // Clean up PID file on exit
        remove_pid_file();

        result
    }

    fn run_loop(&mut self) -> Result<()> {
        // Initialize last_hash with current clipboard content
        self.initialize_hash();

        loop {
            if let Err(e) = self.poll_internal() {
                error!("Error polling clipboard: {}", e);
            }

            std::thread::sleep(Duration::from_millis(self.config.general.poll_interval_ms));
        }
    }

    /// Initialize the last hash with current clipboard content. We prime
    /// from the image side first so that a restart while an image is still
    /// on the clipboard doesn't cause us to re-capture it on the very next
    /// poll (that was bug #4 in the hunt).
    pub fn initialize_hash(&mut self) {
        if let Ok(Some(img)) = Clipboard::read_image() {
            self.last_hash = Some(img.hash);
            debug!("Initialized last_hash from existing clipboard image");
            return;
        }
        if let Ok(Some(text)) = Clipboard::get_text() {
            self.last_hash = Some(Clipboard::hash(text.as_bytes()));
            debug!("Initialized last_hash from existing clipboard text");
        }
    }

    /// Internal poll that returns whether a new entry was captured.
    ///
    /// Flow (critical ordering — this is the fix for bugs #1 and #4):
    /// 1. Read image bytes into memory (no disk write yet).
    /// 2. Short-circuit if content is unchanged since last poll (`last_hash`).
    /// 3. Short-circuit if DB already has a row with this hash
    ///    (`exists_by_hash`) — no disk write, no insert, no duplication.
    /// 4. Only then store the blob (content-addressed, atomic) AND insert
    ///    the DB row. Either both succeed or neither does.
    /// 5. Run LRU eviction; evicted image rows' blobs are pruned via the
    ///    persistent queue in `Database`.
    fn poll_internal(&mut self) -> Result<bool> {
        // Image path has priority over text: browsers put both a URL (text)
        // and the rendered image on the clipboard when you "Copy image",
        // and we want the image.
        if let Some(img) = Clipboard::read_image()? {
            if self.last_hash.as_ref() == Some(&img.hash) {
                return Ok(false);
            }

            let captured = if !self.db.exists_by_hash(&img.hash)? {
                // Store the blob ONLY after we've decided we'll keep it.
                let (_path, _new) =
                    Database::store_image_blob(&img.hash, &img.extension, &img.bytes)?;
                let entry = Entry::new_image(
                    img.hash.clone(),
                    img.bytes.len(),
                    img.extension.clone(),
                );
                self.db.insert(&entry)?;
                info!(
                    "Captured image entry: {} bytes ({}.{})",
                    entry.byte_size,
                    &img.hash[..8],
                    img.extension
                );

                let removed = self.db.cleanup_old(self.config.general.max_entries)?;
                if removed > 0 {
                    debug!("Cleaned up {} old entries", removed);
                }
                true
            } else {
                // Already on record. Update last_hash below so we don't
                // keep re-checking on every poll.
                false
            };
            self.last_hash = Some(img.hash);
            return Ok(captured);
        }

        // Text path.
        if let Some(text) = Clipboard::get_text()? {
            let hash = Clipboard::hash(text.as_bytes());
            if self.last_hash.as_ref() == Some(&hash) {
                return Ok(false);
            }

            let captured = if !self.db.exists_by_hash(&hash)? {
                let entry = Entry::new_text(text);
                self.db.insert(&entry)?;
                info!("Captured text entry: {} bytes", entry.byte_size);

                let removed = self.db.cleanup_old(self.config.general.max_entries)?;
                if removed > 0 {
                    debug!("Cleaned up {} old entries", removed);
                }
                true
            } else {
                false
            };
            self.last_hash = Some(hash);
            return Ok(captured);
        }

        Ok(false)
    }
}
