//! Cross-process selection cursor (Phase 2 sub-task 2.9, groundwork).
//!
//! Ditto's modifier-held cycling UX — hold `Ctrl+Shift`, hit `V`
//! repeatedly to advance through the most-recent clips, release the
//! modifier to paste — requires a long-running daemon that owns the
//! picker state across keystrokes. ditox currently stores the cursor
//! on disk so separate TUI invocations can share it.
//!
//! What we *can* deliver as Phase-2 groundwork is a **selection
//! cursor**: a tiny bit of state (current index + last fire time)
//! that survives across TUI invocations. Each open fires the
//! cursor:
//!
//! - if the previous fire was within the configured "re-fire window"
//!   (default 800 ms, tunable via [`crate::config::PasteConfig`]),
//!   the cursor index is `+1`;
//! - otherwise the cursor resets to `0`.
//!
//! The TUI then pre-selects the entry at that index. Effect:
//! pressing `Ctrl+Shift+V` rapidly cycles through the most-recent
//! clips one entry at a time; idling for >800 ms resets to the top.
//!
//! A future long-running terminal service can keep the same primitive
//! in memory and skip the filesystem round-trip.
//!
//! ## File location
//!
//! `<data_dir>/cursor.json` — same directory as the SQLite DB and
//! the [`crate::paste::sentinel::PasteSentinel`] file.
//!
//! ## Failure mode
//!
//! Both [`SelectionCursor::read_from`] and
//! [`SelectionCursor::write_to`] are best-effort: a missing /
//! unreadable / corrupt file is treated as "fresh cursor" rather
//! than propagated as an error. The TUI must always boot, even
//! if cursor persistence fails.
//!
//! ## Concurrency
//!
//! Two TUI processes firing within the same millisecond is the only
//! interesting race. Writes are atomic via tmp-write + rename, so
//! readers always see a consistent snapshot. Last-writer-wins on
//! the cursor index is acceptable.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{DitoxError, Result};

/// Filename inside the data dir.
const CURSOR_FILENAME: &str = "cursor.json";

/// Default re-fire window. Mirrors the master-plan D2 spec
/// (800 ms ≈ a comfortable double-tap of `Ctrl+Shift+V`).
pub const DEFAULT_REFIRE_WINDOW: Duration = Duration::from_millis(800);

/// On-disk serialisation. Version-tagged so future schema changes
/// can be detected and the cursor reset rather than crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CursorRecord {
    /// Schema version. Currently `1`.
    #[serde(default = "default_version")]
    version: u32,
    /// Current cursor index, clamped at the call site by the
    /// TUI to fit the visible list.
    index: usize,
    /// Wall-clock instant the cursor was last fired, milliseconds
    /// since UNIX epoch. `0` = never fired.
    last_fire_at_ms: u128,
}

fn default_version() -> u32 {
    1
}

/// Pure-Rust selection cursor. Knows nothing about storage — the
/// filesystem helpers ([`SelectionCursor::read_from`] /
/// [`SelectionCursor::write_to`]) are wrappers around it. Phase 4's
/// daemon will hold one of these in memory and skip the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionCursor {
    index: usize,
    last_fire_at: SystemTime,
}

impl Default for SelectionCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionCursor {
    /// A fresh cursor: index `0`, last-fire `UNIX_EPOCH`. The first
    /// real [`fire`](Self::fire) call always resets to `0` because
    /// any sane "now" is many decades past the epoch (well outside
    /// any conceivable re-fire window).
    pub fn new() -> Self {
        Self {
            index: 0,
            last_fire_at: UNIX_EPOCH,
        }
    }

    /// Construct directly from raw fields. Used by storage layers
    /// reconstituting from disk.
    pub fn from_raw(index: usize, last_fire_at: SystemTime) -> Self {
        Self {
            index,
            last_fire_at,
        }
    }

    /// Current cursor index (uncapped — the caller clamps to its
    /// list length).
    pub fn index(&self) -> usize {
        self.index
    }

    /// Wall-clock of the most recent [`fire`](Self::fire), or
    /// [`UNIX_EPOCH`] if the cursor is fresh.
    pub fn last_fire_at(&self) -> SystemTime {
        self.last_fire_at
    }

    /// Cursor index clamped to a list of `len` entries. Wraps
    /// around modulo `len` so rapid re-fires past the end of the
    /// list go back to the top rather than getting stuck.
    /// Returns `0` for an empty list.
    pub fn index_for_list(&self, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            self.index % len
        }
    }

    /// Advance the cursor. If `now` is within `window` of the last
    /// fire, the index is `+1`'d (saturating at `usize::MAX`);
    /// otherwise the cursor resets to `0`. Either way,
    /// `last_fire_at` is updated to `now`.
    pub fn fire(&mut self, now: SystemTime, window: Duration) {
        let elapsed = now
            .duration_since(self.last_fire_at)
            .unwrap_or(Duration::MAX);
        if elapsed <= window {
            self.index = self.index.saturating_add(1);
        } else {
            self.index = 0;
        }
        self.last_fire_at = now;
    }

    /// Reset to a fresh cursor. Equivalent to constructing a new
    /// one but mutates in place — useful for in-memory storage in
    /// Phase 4's daemon.
    pub fn reset(&mut self) {
        self.index = 0;
        self.last_fire_at = UNIX_EPOCH;
    }
}

/// Filesystem-backed wrapper. Keeps the path so callers don't need
/// to thread it through every read/write call.
#[derive(Debug, Clone)]
pub struct PersistentSelectionCursor {
    path: PathBuf,
}

impl PersistentSelectionCursor {
    /// Construct rooted at the ditox data directory's default
    /// location. Errors when the data dir can't be determined.
    pub fn at_default_path() -> Result<Self> {
        let dir = crate::db::Database::get_data_dir()?;
        Ok(Self::at(dir.join(CURSOR_FILENAME)))
    }

    /// Construct at an explicit path. Used by tests and callers
    /// overriding the data dir for their process.
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Path to the cursor file. Useful for diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the cursor from disk. Returns
    /// [`SelectionCursor::new`] for any of:
    /// - File absent / unreadable.
    /// - File JSON-corrupt.
    /// - Schema version unrecognised (forward-compat: future
    ///   versions of ditox can bump the version and old binaries
    ///   gracefully reset rather than crash on an unknown shape).
    pub fn read(&self) -> SelectionCursor {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(_) => return SelectionCursor::new(),
        };
        let record: CursorRecord = match serde_json::from_str(&raw) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %self.path.display(),
                    "cursor file corrupt; resetting to fresh cursor"
                );
                return SelectionCursor::new();
            }
        };
        if record.version != 1 {
            tracing::warn!(
                version = record.version,
                path = %self.path.display(),
                "cursor file schema version not understood; resetting"
            );
            return SelectionCursor::new();
        }
        let last_fire_at = UNIX_EPOCH
            .checked_add(Duration::from_millis(
                u64::try_from(record.last_fire_at_ms).unwrap_or(0),
            ))
            .unwrap_or(UNIX_EPOCH);
        SelectionCursor::from_raw(record.index, last_fire_at)
    }

    /// Write the cursor to disk. Best-effort: errors are logged at
    /// `warn` and otherwise swallowed.
    pub fn write(&self, cursor: &SelectionCursor) {
        let last_fire_at_ms = cursor
            .last_fire_at
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let record = CursorRecord {
            version: 1,
            index: cursor.index,
            last_fire_at_ms,
        };
        let json = match serde_json::to_string(&record) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialise cursor record");
                return;
            }
        };
        if let Err(e) = atomic_write(&self.path, json.as_bytes()) {
            tracing::warn!(
                error = %e,
                path = %self.path.display(),
                "failed to write cursor file"
            );
        }
    }

    /// Convenience: read, fire, write, return the new cursor.
    /// Mirrors the typical TUI open path.
    pub fn fire_and_persist(&self, now: SystemTime, window: Duration) -> SelectionCursor {
        let mut cursor = self.read();
        cursor.fire(now, window);
        self.write(&cursor);
        cursor
    }

    /// Remove the cursor file. Best-effort. Used by tests and by
    /// `ditox` CLI commands that explicitly reset state.
    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Write `bytes` to `path` via tmp-write + rename so concurrent
/// readers never see a half-written file. Mirrors the sentinel's
/// `atomic_write` — kept module-local because both modules want
/// slightly different parent-creation semantics in the future.
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

    // -- pure cursor logic --

    #[test]
    fn new_cursor_is_zero_at_epoch() {
        let c = SelectionCursor::new();
        assert_eq!(c.index(), 0);
        assert_eq!(c.last_fire_at(), UNIX_EPOCH);
    }

    #[test]
    fn first_fire_resets_to_zero_because_epoch_is_ancient() {
        let mut c = SelectionCursor::new();
        let now = SystemTime::now();
        c.fire(now, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 0);
        assert_eq!(c.last_fire_at(), now);
    }

    #[test]
    fn refire_within_window_increments() {
        let mut c = SelectionCursor::new();
        let t0 = SystemTime::now();
        c.fire(t0, DEFAULT_REFIRE_WINDOW);
        let t1 = t0 + Duration::from_millis(200);
        c.fire(t1, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 1);
        assert_eq!(c.last_fire_at(), t1);
    }

    #[test]
    fn three_rapid_fires_advance_to_index_two() {
        let mut c = SelectionCursor::new();
        let t0 = SystemTime::now();
        c.fire(t0, DEFAULT_REFIRE_WINDOW);
        c.fire(t0 + Duration::from_millis(200), DEFAULT_REFIRE_WINDOW);
        c.fire(t0 + Duration::from_millis(400), DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 2);
    }

    #[test]
    fn refire_outside_window_resets() {
        let mut c = SelectionCursor::new();
        let t0 = SystemTime::now();
        c.fire(t0, DEFAULT_REFIRE_WINDOW);
        c.fire(t0 + Duration::from_millis(200), DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 1);
        // 1 s gap > 800 ms window.
        c.fire(t0 + Duration::from_millis(1500), DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 0);
    }

    #[test]
    fn fire_at_exact_window_boundary_increments() {
        // Inclusive boundary semantics: <= window counts as in-window.
        let mut c = SelectionCursor::new();
        let t0 = SystemTime::now();
        c.fire(t0, DEFAULT_REFIRE_WINDOW);
        c.fire(t0 + DEFAULT_REFIRE_WINDOW, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 1);
    }

    #[test]
    fn fire_one_ms_past_window_resets() {
        let mut c = SelectionCursor::new();
        let t0 = SystemTime::now();
        c.fire(t0, DEFAULT_REFIRE_WINDOW);
        c.fire(
            t0 + DEFAULT_REFIRE_WINDOW + Duration::from_millis(1),
            DEFAULT_REFIRE_WINDOW,
        );
        assert_eq!(c.index(), 0);
    }

    #[test]
    fn reset_returns_to_fresh_state() {
        let mut c = SelectionCursor::new();
        let t0 = SystemTime::now();
        c.fire(t0, DEFAULT_REFIRE_WINDOW);
        c.fire(t0 + Duration::from_millis(200), DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 1);
        c.reset();
        assert_eq!(c.index(), 0);
        assert_eq!(c.last_fire_at(), UNIX_EPOCH);
    }

    #[test]
    fn from_raw_round_trips_through_index_and_last_fire() {
        let t = SystemTime::now();
        let c = SelectionCursor::from_raw(7, t);
        assert_eq!(c.index(), 7);
        assert_eq!(c.last_fire_at(), t);
    }

    #[test]
    fn fire_does_not_overflow_at_usize_max() {
        let mut c = SelectionCursor::from_raw(usize::MAX, SystemTime::now());
        let now = SystemTime::now();
        // saturating_add: stays at MAX, doesn't panic.
        c.fire(now, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), usize::MAX);
    }

    // -- index_for_list clamp/wrap --

    #[test]
    fn index_for_empty_list_is_zero() {
        let c = SelectionCursor::from_raw(5, SystemTime::now());
        assert_eq!(c.index_for_list(0), 0);
    }

    #[test]
    fn index_for_list_within_bounds_returns_index() {
        let c = SelectionCursor::from_raw(3, SystemTime::now());
        assert_eq!(c.index_for_list(10), 3);
    }

    #[test]
    fn index_for_list_wraps_modulo() {
        let c = SelectionCursor::from_raw(7, SystemTime::now());
        // 7 % 3 = 1
        assert_eq!(c.index_for_list(3), 1);
    }

    #[test]
    fn index_for_list_at_exact_len_wraps_to_zero() {
        let c = SelectionCursor::from_raw(5, SystemTime::now());
        assert_eq!(c.index_for_list(5), 0);
    }

    // -- filesystem persistence --

    fn cursor_in(dir: &TempDir) -> PersistentSelectionCursor {
        PersistentSelectionCursor::at(dir.path().join("cursor.json"))
    }

    #[test]
    fn read_missing_file_returns_fresh_cursor() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        let c = p.read();
        assert_eq!(c, SelectionCursor::new());
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        let t = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let written = SelectionCursor::from_raw(4, t);
        p.write(&written);
        let read = p.read();
        assert_eq!(read.index(), 4);
        assert_eq!(read.last_fire_at(), t);
    }

    #[test]
    fn read_corrupt_json_returns_fresh_cursor() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        std::fs::write(p.path(), b"not json").unwrap();
        assert_eq!(p.read(), SelectionCursor::new());
    }

    #[test]
    fn read_unknown_schema_version_returns_fresh_cursor() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        let payload = serde_json::json!({
            "version": 999,
            "index": 42,
            "last_fire_at_ms": 1_700_000_000_000u64,
        });
        std::fs::write(p.path(), payload.to_string()).unwrap();
        assert_eq!(p.read(), SelectionCursor::new());
    }

    #[test]
    fn write_is_atomic_via_tmp_rename() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        p.write(&SelectionCursor::from_raw(2, SystemTime::now()));
        let tmp_path = p.path().with_extension("tmp");
        assert!(!tmp_path.exists(), "tmp file should be renamed away");
        assert!(p.path().exists(), "cursor file should exist");
    }

    #[test]
    fn write_creates_parent_dirs_if_needed() {
        let dir = TempDir::new().unwrap();
        let p = PersistentSelectionCursor::at(dir.path().join("nested/sub/cursor.json"));
        p.write(&SelectionCursor::new());
        assert!(p.path().exists());
    }

    #[test]
    fn fire_and_persist_first_call_returns_index_zero() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        let now = SystemTime::now();
        let c = p.fire_and_persist(now, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 0);
        assert_eq!(c.last_fire_at(), now);
    }

    #[test]
    fn fire_and_persist_advances_within_window() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        let t0 = SystemTime::now();
        let c1 = p.fire_and_persist(t0, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c1.index(), 0);
        // Manually push the persisted last_fire_at into the past
        // by overwriting through the API.
        let t1 = t0 + Duration::from_millis(200);
        let c2 = p.fire_and_persist(t1, DEFAULT_REFIRE_WINDOW);
        assert_eq!(c2.index(), 1);
        // The on-disk state should now show index=1.
        assert_eq!(p.read().index(), 1);
    }

    #[test]
    fn fire_and_persist_resets_outside_window() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        let t0 = SystemTime::now();
        p.fire_and_persist(t0, DEFAULT_REFIRE_WINDOW);
        p.fire_and_persist(t0 + Duration::from_millis(200), DEFAULT_REFIRE_WINDOW);
        assert_eq!(p.read().index(), 1);
        // Long gap → reset.
        let c = p.fire_and_persist(t0 + Duration::from_millis(2000), DEFAULT_REFIRE_WINDOW);
        assert_eq!(c.index(), 0);
        assert_eq!(p.read().index(), 0);
    }

    #[test]
    fn clear_removes_file() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        p.write(&SelectionCursor::from_raw(1, SystemTime::now()));
        assert!(p.path().exists());
        p.clear();
        assert!(!p.path().exists());
    }

    #[test]
    fn clear_on_missing_file_is_noop() {
        let dir = TempDir::new().unwrap();
        let p = cursor_in(&dir);
        p.clear();
        p.clear();
    }

    #[test]
    fn path_returns_configured_location() {
        let dir = TempDir::new().unwrap();
        let custom = dir.path().join("custom-cursor.json");
        let p = PersistentSelectionCursor::at(custom.clone());
        assert_eq!(p.path(), custom);
    }
}
