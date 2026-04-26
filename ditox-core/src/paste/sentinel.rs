//! Cross-process paste-back sentinel (Phase 2 sub-task 2.7, Linux).
//!
//! When the launcher pastes a clip back into the previously-focused
//! window, the watcher (whether the long-running daemon or the GUI's
//! in-process watcher) inevitably observes the resulting clipboard
//! change and would record it as a *new* entry — duplicating the
//! original. Ditto's solution on Windows is to set a registered
//! format named `Clipboard Viewer Ignore`; downstream watchers check
//! for it and skip the capture.
//!
//! On Wayland, registering an extra MIME on every paste is ergonomic
//! only if we also use a multi-format clipboard write. Until that
//! lands (see Phase 1 sub-task 1.x and the GUI's paste-back
//! sequence), we use a **content + timestamp file** as the
//! cross-process signal:
//!
//! 1. After [`crate::clipboard::Clipboard::set_text`] /
//!    `set_image` succeeds, the launcher calls [`PasteSentinel::record`]
//!    with the SHA-256 of the just-written content.
//! 2. The watcher calls [`PasteSentinel::matches`] before recording
//!    each captured clip. If the captured content's hash matches and
//!    the record is fresh (within `ttl`), the capture is dropped.
//!
//! The sentinel file is `<data_dir>/last-paste.json` — the same
//! directory the watcher PID file and the SQLite DB live in, so file
//! permissions / mountpoint concerns are identical.
//!
//! ## Failure mode
//!
//! Both `record` and `matches` are **best-effort**: a missing /
//! unreadable / corrupt file is treated as "no recent paste", not
//! propagated as an error. This trades occasional duplicate captures
//! (during a paste-back when the file write fails) for never blocking
//! the launcher or the watcher on a transient I/O hiccup.
//!
//! ## Concurrency
//!
//! The file is written atomically via tmp-write + rename. Multiple
//! concurrent paste-backs from racing launchers (rare) result in a
//! last-writer-wins outcome — acceptable since the worst case is one
//! of the two paste-backs gets re-captured.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{DitoxError, Result};

/// Filename inside the data dir.
const SENTINEL_FILENAME: &str = "last-paste.json";

/// Sentinel file payload. Written by the launcher, read by the
/// watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SentinelRecord {
    /// SHA-256 hex of the just-pasted content (matches the value
    /// the watcher computes via `Clipboard::hash`).
    hash: String,
    /// Wall-clock instant the paste happened, milliseconds since
    /// UNIX epoch.
    at_ms: u128,
}

/// Cross-process sentinel. Cheap to construct — the file isn't
/// touched until [`PasteSentinel::record`] / [`PasteSentinel::matches`]
/// is called.
#[derive(Debug, Clone)]
pub struct PasteSentinel {
    path: PathBuf,
}

impl PasteSentinel {
    /// Construct a sentinel rooted at the ditox data directory's
    /// default location (resolved via the same mechanism as the
    /// SQLite DB and image store). Errors when the data dir can't
    /// be determined.
    pub fn at_default_path() -> Result<Self> {
        let dir = crate::db::Database::get_data_dir()?;
        Ok(Self::at(dir.join(SENTINEL_FILENAME)))
    }

    /// Construct a sentinel at an explicit path. Used by tests + by
    /// callers that need to override the data dir for their process.
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Path to the sentinel file. Useful for diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Record that a paste-back just wrote `hash` to the clipboard.
    ///
    /// Best-effort: a write failure is logged at `warn` and
    /// otherwise swallowed. The watcher's worst-case outcome is one
    /// duplicate capture; not worth aborting the paste flow over.
    pub fn record(&self, hash: &str) {
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis(),
            Err(_) => {
                tracing::warn!("system clock before UNIX epoch; skipping sentinel record");
                return;
            }
        };
        let record = SentinelRecord {
            hash: hash.to_string(),
            at_ms: now,
        };
        let json = match serde_json::to_string(&record) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialise sentinel record");
                return;
            }
        };
        if let Err(e) = atomic_write(&self.path, json.as_bytes()) {
            tracing::warn!(
                error = %e,
                path = %self.path.display(),
                "failed to write sentinel file"
            );
        }
    }

    /// Check whether `hash` was just pasted and the record is still
    /// within `ttl`. Returns `false` for any of:
    /// - No sentinel file present.
    /// - File unreadable / corrupt.
    /// - Recorded hash doesn't match.
    /// - Record older than `ttl`.
    pub fn matches(&self, hash: &str, ttl: Duration) -> bool {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(_) => return false, // file absent or unreadable → no match
        };
        let record: SentinelRecord = match serde_json::from_str(&raw) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %self.path.display(),
                    "sentinel file corrupt; ignoring"
                );
                return false;
            }
        };
        if record.hash != hash {
            return false;
        }
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis(),
            Err(_) => return false,
        };
        let age_ms = now.saturating_sub(record.at_ms);
        age_ms < ttl.as_millis()
    }

    /// Remove the sentinel file. Optional — used at watcher shutdown
    /// to avoid stale records persisting across restarts. Best-effort.
    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Write `bytes` to `path` via tmp-write + rename so concurrent
/// readers never see a half-written file.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(DitoxError::Io)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(DitoxError::Io)?;
    std::fs::rename(&tmp, path).map_err(DitoxError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sentinel_in(dir: &TempDir) -> PasteSentinel {
        PasteSentinel::at(dir.path().join("last-paste.json"))
    }

    #[test]
    fn missing_file_means_no_match() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        assert!(!s.matches("any-hash", Duration::from_secs(60)));
    }

    #[test]
    fn record_then_match_within_ttl_succeeds() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.record("abc123");
        assert!(s.matches("abc123", Duration::from_secs(60)));
    }

    #[test]
    fn record_then_mismatched_hash_returns_false() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.record("abc123");
        assert!(!s.matches("def456", Duration::from_secs(60)));
    }

    #[test]
    fn match_outside_ttl_returns_false() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.record("abc123");
        // Race: by the time this runs the record is already a few μs
        // old. Use a TTL of zero to guarantee expiration.
        assert!(!s.matches("abc123", Duration::from_millis(0)));
    }

    #[test]
    fn corrupt_file_returns_false_and_doesnt_panic() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        std::fs::write(s.path(), b"not json").unwrap();
        assert!(!s.matches("abc123", Duration::from_secs(60)));
    }

    #[test]
    fn record_is_atomic_via_tmp_rename() {
        // After record(), no `.tmp` file should remain.
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.record("abc");
        let tmp_path = s.path().with_extension("tmp");
        assert!(!tmp_path.exists(), "tmp file should be renamed away");
        assert!(s.path().exists(), "final sentinel file should exist");
    }

    #[test]
    fn second_record_overwrites_first() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.record("first");
        s.record("second");
        assert!(s.matches("second", Duration::from_secs(60)));
        assert!(!s.matches("first", Duration::from_secs(60)));
    }

    #[test]
    fn clear_removes_file() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.record("abc");
        assert!(s.path().exists());
        s.clear();
        assert!(!s.path().exists());
    }

    #[test]
    fn clear_on_missing_file_is_noop() {
        let dir = TempDir::new().unwrap();
        let s = sentinel_in(&dir);
        s.clear();
        s.clear(); // idempotent
    }

    #[test]
    fn record_creates_parent_dirs_if_needed() {
        let dir = TempDir::new().unwrap();
        let s = PasteSentinel::at(dir.path().join("nested/sub/last-paste.json"));
        s.record("abc");
        assert!(s.path().exists());
    }

    #[test]
    fn path_returns_configured_location() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("custom-name.json");
        let s = PasteSentinel::at(p.clone());
        assert_eq!(s.path(), p);
    }
}
