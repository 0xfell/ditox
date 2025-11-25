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

    /// Initialize the last hash with current clipboard content
    pub fn initialize_hash(&mut self) {
        if let Ok(Some(text)) = Clipboard::get_text() {
            self.last_hash = Some(Clipboard::hash(text.as_bytes()));
            debug!("Initialized with existing clipboard content");
        }
    }

    /// Internal poll that returns whether a new entry was captured
    fn poll_internal(&mut self) -> Result<bool> {
        // Try image first - browsers put both URL (text) and image data when copying images,
        // so we prioritize image to capture the actual image instead of just the URL
        let images_dir = Database::get_images_dir()?;
        if let Some((path, size, hash)) = Clipboard::get_image(&images_dir)? {
            if self.last_hash.as_ref() != Some(&hash) {
                let captured = if !self.db.exists_by_hash(&hash)? {
                    let entry = Entry::new_image(path, size, hash.clone());
                    self.db.insert(&entry)?;
                    info!("Captured image entry: {} bytes", entry.byte_size);

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
            return Ok(false);
        }

        // Try text if no image
        if let Some(text) = Clipboard::get_text()? {
            let hash = Clipboard::hash(text.as_bytes());

            // Check if content changed
            if self.last_hash.as_ref() != Some(&hash) {
                // Check if already in database
                let captured = if !self.db.exists_by_hash(&hash)? {
                    let entry = Entry::new_text(text);
                    self.db.insert(&entry)?;
                    info!("Captured text entry: {} bytes", entry.byte_size);

                    // Cleanup old entries
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
        }

        Ok(false)
    }
}
