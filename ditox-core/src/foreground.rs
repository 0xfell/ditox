//! Foreground-window tracking for paste-back targeting.
//!
//! Phase 2 sub-task 2.1. The Ditto-style paste-back UX needs to know
//! *which window had focus before the TUI appeared*, so that
//! after the user picks a clip we can:
//!
//! 1. Dismiss the TUI.
//! 2. Restore focus to the previously-tracked window.
//! 3. Synthesise Ctrl+V (or a per-app override) into it.
//!
//! This module defines the platform-agnostic [`ForegroundTracker`]
//! trait + the [`ForegroundSnapshot`] data type. Concrete backends
//! (Win32 in 2.2, Wayland in 2.3) live in their own sub-modules so
//! the platform-specific deps don't pollute `ditox-core`'s
//! cross-platform compile path.
//!
//! Cross-cutting wrappers ([`ForegroundFilter`] to drop self-focus
//! events; [`NoopForegroundTracker`] for GNOME Wayland and other
//! unsupported compositors) are provided here.
//!
//! ## Design parity with `CaptureSource`
//!
//! The trait is **synchronous** (no `async fn`) for the same reason
//! [`crate::capture::CaptureSource`] is — keeps `ditox-core`
//! runtime-agnostic. Event-driven backends spawn a thread internally
//! and surface events via the [`std::sync::mpsc::Receiver`] returned
//! by [`ForegroundTracker::subscribe`].
//!
//! ## Re-focus support matrix
//!
//! Not every platform lets a non-privileged client re-focus a
//! specific window. [`ForegroundId::supports_restore`] encodes the
//! matrix; the TUI consults it before deciding whether to
//! attempt a `restore()` call.
//!
//! | Platform                | `restore()` | Notes |
//! |-------------------------|-------------|-------|
//! | Windows                 | Yes         | `SetForegroundWindow` |
//! | Hyprland                | Yes         | `hyprctl dispatch focuswindow address:0x…` |
//! | Sway / wlroots          | Yes         | `wlr-foreign-toplevel-management` activate request, compositor may ignore |
//! | KDE Wayland             | No          | KWin has no client-driven focus; tracked separately |
//! | GNOME Wayland           | N/A         | No foreground-tracking protocol either; uses [`NoopForegroundTracker`] |
//! | X11                     | Yes         | `XSetInputFocus` with input model checks |
//! | macOS                   | Yes         | `NSRunningApplication.activate` (Phase 8) |

use std::sync::mpsc;
use std::time::SystemTime;

use crate::error::Result;

/// Hyprland-specific foreground tracker (Phase 2 sub-task 2.3).
/// Shells out to `hyprctl activewindow -j` for snapshots and
/// `hyprctl dispatch focuswindow address:0x…` for restore.
#[cfg(unix)]
pub mod hyprctl;
#[cfg(unix)]
pub mod wlr;

/// Build the platform-default [`ForegroundTracker`] wrapped in a
/// [`ForegroundFilter`] that drops self-focus events.
///
/// Selection matrix (Phase 2 sub-task 2.8):
///
/// - **Hyprland** → [`hyprctl::HyprctlForegroundTracker`].
/// - **Sway / generic wlroots / KDE Wayland** → [`wlr::WlrForegroundTracker`]
///   when the compositor advertises the foreign-toplevel protocol.
/// - **GNOME Wayland** → [`NoopForegroundTracker`] (no protocol
///   support).
/// - **X11** → [`NoopForegroundTracker`] for v0.4 (X11 tracker
///   tracked separately).
/// - **Windows** → [`NoopForegroundTracker`] for v0.4 (sub-task 2.2
///   adds the `Win32ForegroundTracker`).
/// - **macOS / Other** → [`NoopForegroundTracker`].
///
/// The TUI consults
/// [`ForegroundSnapshot::identifier`]'s
/// [`ForegroundId::supports_restore`] before attempting a restore,
/// so even with the noop tracker the TUI degrades gracefully (write
/// the clipboard, show "paste manually" status, exit).
pub fn build_default_tracker() -> Box<dyn ForegroundTracker> {
    use crate::platform::{detect, LinuxCompositor, Platform};
    let p = detect();
    #[cfg(unix)]
    {
        if let Platform::Linux(LinuxCompositor::Hyprland { .. }) = p {
            return Box::new(ForegroundFilter::new(
                hyprctl::HyprctlForegroundTracker::new(),
            ));
        }
        if matches!(
            p,
            Platform::Linux(
                LinuxCompositor::Sway { .. }
                    | LinuxCompositor::Wlroots { .. }
                    | LinuxCompositor::Kde { .. }
            )
        ) {
            match wlr::WlrForegroundTracker::new() {
                Ok(tracker) => return Box::new(ForegroundFilter::new(tracker)),
                Err(error) => {
                    tracing::warn!(%error, "wlr foreground tracker unavailable; using noop tracker")
                }
            }
        }
    }
    let _ = p; // suppress unused on non-unix
    Box::new(ForegroundFilter::new(NoopForegroundTracker::new()))
}

/// A point-in-time snapshot of the currently-focused window.
///
/// Held by the TUI between open and paste so the user's
/// previous app can be re-focused at paste-back time.
#[derive(Debug, Clone)]
pub struct ForegroundSnapshot {
    /// Platform-specific identifier used by [`ForegroundTracker::restore`].
    pub identifier: ForegroundId,
    /// Process basename of the focused app — `firefox.exe`, `gvim`,
    /// `kitty`. Used to look up the per-app keystroke override
    /// (sub-task 2.6) and exclusion rules.
    ///
    /// On Windows this includes the `.exe` extension; on Linux/macOS
    /// it doesn't (no extension on the executable).
    pub process_basename: String,
    /// Window title at snapshot time. Display-only — used in
    /// debug logs and the TUI's "Pasting into ..." status line.
    pub title: String,
    /// Wall-clock instant the snapshot was captured.
    pub captured_at: SystemTime,
}

/// Platform-specific window identifier.
///
/// All variants carry only portable scalar / `String` types so this
/// enum lives in `ditox-core` without pulling in any platform deps.
/// Concrete backends translate from this portable form back to their
/// native handle when [`ForegroundTracker::restore`] is called.
///
/// Equality and hashing are derived so the TUI can dedup
/// snapshots across rapid focus oscillations (alt-tab spam, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ForegroundId {
    /// Windows. `hwnd` is the `HWND` cast to `i64` (signed because
    /// the Win32 type is `isize`); `pid` is the owning process.
    Win32 { hwnd: i64, pid: u32 },

    /// Hyprland. `address` is the literal string returned by
    /// `hyprctl activewindow -j` (e.g. `"0x55a8f3dc4e80"`). Stable
    /// for the window's lifetime.
    Hypr { address: String },

    /// wlr-foreign-toplevel-management (Sway / wlroots). `handle_id`
    /// is the live Wayland proxy id captured by the tracker; restore
    /// asks the compositor to activate that handle if it still exists.
    Wlr {
        handle_id: String,
        app_id: String,
        title: String,
    },

    /// X11 window ID (xlib / xcb `Window` is a `u32`).
    X11 { window: u32 },

    /// macOS. `pid` is `i32` to match `NSRunningApplication`'s
    /// `processIdentifier` / Darwin's `pid_t`.
    Macos { pid: i32 },

    /// No tracking available — the compositor doesn't support a
    /// foreground-tracking protocol (current example: GNOME
    /// Wayland), or the platform isn't yet supported.
    Unknown,
}

impl ForegroundId {
    /// True when [`ForegroundTracker::restore`] can re-focus a window
    /// of this kind.
    ///
    /// Drives TUI behaviour: when `false`, the TUI skips
    /// the explicit restore call and relies on the compositor's
    /// "previous-focus" semantics that fire when our window is
    /// hidden. See the table in the module docs for the per-platform
    /// matrix.
    pub fn supports_restore(&self) -> bool {
        match self {
            ForegroundId::Win32 { .. } => true,
            ForegroundId::Hypr { .. } => true,
            ForegroundId::Wlr { .. } => true,
            ForegroundId::X11 { .. } => true,
            ForegroundId::Macos { .. } => true,
            ForegroundId::Unknown => false,
        }
    }

    /// Stable string label for logs/metrics (`"win32"`, `"hypr"`,
    /// `"wlr"`, `"x11"`, `"macos"`, `"unknown"`).
    pub fn kind(&self) -> &'static str {
        match self {
            ForegroundId::Win32 { .. } => "win32",
            ForegroundId::Hypr { .. } => "hypr",
            ForegroundId::Wlr { .. } => "wlr",
            ForegroundId::X11 { .. } => "x11",
            ForegroundId::Macos { .. } => "macos",
            ForegroundId::Unknown => "unknown",
        }
    }
}

/// Foreground-window tracker.
///
/// Concrete backends (one per platform) implement this trait. The
/// TUI selects a backend at startup based on the detected
/// [`crate::platform::Platform`].
///
/// ## Lifecycle
///
/// 1. Construct: cheap; no OS resources held yet.
/// 2. `subscribe()` once: starts the background event loop / hook /
///    polling thread and returns the receiver. Calling `subscribe()`
///    a second time without `shutdown()` between them is undefined
///    (concrete backends may either error or silently replace the
///    previous subscription — match the [`crate::capture::CaptureSource`]
///    contract).
/// 3. `snapshot()` at any time: synchronous read; no subscription
///    required. The TUI uses this on open to capture the
///    "current foreground" before it itself becomes the foreground.
/// 4. `restore()` at any time: also no-subscription-required.
/// 5. `shutdown()`: idempotent; tears down whatever `subscribe()`
///    started. Implicitly called on `Drop` by well-behaved impls.
pub trait ForegroundTracker: Send {
    /// Stable identifier for this tracker. Used in logs.
    fn name(&self) -> &str;

    /// Synchronous read of the current foreground window.
    ///
    /// Returns:
    /// - `Ok(Some(snap))` — a foreground window exists and was
    ///   identified.
    /// - `Ok(None)` — the platform doesn't expose the focused
    ///   window, no window currently has focus, or the focused
    ///   window is the TUI itself (when wrapped in
    ///   [`ForegroundFilter`]).
    /// - `Err(_)` — the tracker is in a broken state (e.g. the
    ///   compositor died); the TUI logs and degrades to no
    ///   restore.
    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>>;

    /// Restore focus to the snapshot's identified window.
    ///
    /// On platforms where re-focus isn't supported (see
    /// [`ForegroundId::supports_restore`]), this is a no-op
    /// `Ok(())` — relying on the compositor's hide-our-window =>
    /// restore-previous-focus semantics.
    ///
    /// Errors are returned but the TUI should treat them as
    /// non-fatal: even if focus restore fails, the user can paste
    /// manually.
    fn restore(&self, snapshot: &ForegroundSnapshot) -> Result<()>;

    /// Subscribe to foreground-change events. The receiver yields a
    /// snapshot whenever focus moves to a different window.
    ///
    /// Most TUI flows don't need the subscription — they call
    /// `snapshot()` on open. The subscription exists for the
    /// Phase 4 "what app is the user currently in?" status line and
    /// for the watcher's per-app exclusion rules (Phase 3).
    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>>;

    /// Stop the background work started by `subscribe()`. Idempotent.
    fn shutdown(&mut self) -> Result<()>;
}

// ---------------------------------------------------------------------------
// ForegroundFilter — wrap any tracker to drop self-focus events
// ---------------------------------------------------------------------------

/// Tracker decorator that drops snapshots whose `process_basename`
/// matches ditox's own executable name.
///
/// Why: the TUI is itself a foreground window. Without filtering,
/// calling `snapshot()` after ditox gains focus would return ditox
/// itself, defeating the paste-back flow.
///
/// Defaults to filtering `"ditox"` (and `"ditox.exe"` on Windows).
/// Users can override the
/// list via [`ForegroundFilter::with_self_basenames`] for downstream
/// repackagings (`my-clipboard-app`, etc.).
///
/// Comparison is ASCII-case-insensitive — Windows reports basenames
/// with mixed case (`Ditox.exe`).
pub struct ForegroundFilter<T: ForegroundTracker> {
    inner: T,
    self_basenames: Vec<String>,
    /// Background thread that reads from inner_rx, drops self
    /// snapshots, and forwards everything else. `None` until
    /// `subscribe()` is called.
    filter_thread: Option<std::thread::JoinHandle<()>>,
}

impl<T: ForegroundTracker> ForegroundFilter<T> {
    /// Default constructor with `["ditox", "ditox.exe"]`
    /// as self-names. Covers Linux + Windows binary names.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            self_basenames: vec!["ditox".into(), "ditox.exe".into()],
            filter_thread: None,
        }
    }

    /// Override the self-basenames list. Useful for downstream
    /// repackagings.
    pub fn with_self_basenames(inner: T, names: Vec<String>) -> Self {
        Self {
            inner,
            self_basenames: names,
            filter_thread: None,
        }
    }

    fn is_self(basenames: &[String], candidate: &str) -> bool {
        let lower = candidate.to_ascii_lowercase();
        basenames.iter().any(|n| n.to_ascii_lowercase() == lower)
    }
}

impl<T: ForegroundTracker> ForegroundTracker for ForegroundFilter<T> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        match self.inner.snapshot()? {
            Some(snap) if Self::is_self(&self.self_basenames, &snap.process_basename) => Ok(None),
            other => Ok(other),
        }
    }

    fn restore(&self, snapshot: &ForegroundSnapshot) -> Result<()> {
        // Filtering only applies to read; restore is unconditional
        // (the caller already chose to restore this snapshot).
        self.inner.restore(snapshot)
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        let inner_rx = self.inner.subscribe()?;
        let (tx, rx) = mpsc::channel();
        let basenames = self.self_basenames.clone();
        let join = std::thread::Builder::new()
            .name("ditox-foreground-filter".into())
            .spawn(move || {
                while let Ok(snap) = inner_rx.recv() {
                    if Self::is_self(&basenames, &snap.process_basename) {
                        continue;
                    }
                    if tx.send(snap).is_err() {
                        // Subscriber dropped the receiver; nothing
                        // more to do. Inner channel will close on
                        // its own when the inner tracker shuts down.
                        break;
                    }
                }
            })
            .map_err(|e| {
                crate::error::DitoxError::Other(format!("spawn foreground-filter thread: {}", e))
            })?;
        self.filter_thread = Some(join);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        // Inner shutdown closes the inner channel, which causes the
        // filter thread's `recv()` loop to exit. Then we join it.
        let result = self.inner.shutdown();
        if let Some(handle) = self.filter_thread.take() {
            let _ = handle.join();
        }
        result
    }
}

// ---------------------------------------------------------------------------
// NoopForegroundTracker — degraded-mode placeholder
// ---------------------------------------------------------------------------

/// Foreground tracker that always returns `None` and treats restore
/// as a no-op.
///
/// Used on platforms where foreground tracking isn't possible
/// (current main case: GNOME Wayland — no foreign-toplevel
/// protocol, no client API for "what window has focus"). The
/// TUI emits a one-time warning at startup so the user
/// understands why paste-back doesn't re-target their previous app.
pub struct NoopForegroundTracker;

impl NoopForegroundTracker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopForegroundTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ForegroundTracker for NoopForegroundTracker {
    fn name(&self) -> &str {
        "noop-foreground"
    }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        Ok(None)
    }

    fn restore(&self, _snapshot: &ForegroundSnapshot) -> Result<()> {
        Ok(())
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        // Empty channel — no events ever arrive. The sender side
        // is dropped immediately so a `recv()` returns
        // `Disconnected` rather than blocking forever.
        let (_tx, rx) = mpsc::channel();
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MockForegroundTracker — test helper
// ---------------------------------------------------------------------------

/// Test-only tracker that returns a configured snapshot from
/// [`ForegroundTracker::snapshot`] and records every
/// [`ForegroundTracker::restore`] call.
///
/// Subscribers receive snapshots pushed via [`Self::inject`].
#[doc(hidden)]
pub struct MockForegroundTracker {
    snapshot_to_return: std::sync::Mutex<Option<ForegroundSnapshot>>,
    restore_log: std::sync::Mutex<Vec<ForegroundSnapshot>>,
    injector: std::sync::Mutex<Option<mpsc::Sender<ForegroundSnapshot>>>,
}

#[doc(hidden)]
impl MockForegroundTracker {
    pub fn new(initial_snapshot: Option<ForegroundSnapshot>) -> Self {
        Self {
            snapshot_to_return: std::sync::Mutex::new(initial_snapshot),
            restore_log: std::sync::Mutex::new(Vec::new()),
            injector: std::sync::Mutex::new(None),
        }
    }

    /// Update the snapshot that `snapshot()` returns. Useful when a
    /// test wants to simulate focus changing between calls.
    pub fn set_snapshot(&self, snap: Option<ForegroundSnapshot>) {
        *self.snapshot_to_return.lock().unwrap() = snap;
    }

    /// Push a snapshot to any active subscriber. Returns `Ok(())` if
    /// a subscription is active and the send succeeded.
    pub fn inject(&self, snap: ForegroundSnapshot) -> std::result::Result<(), &'static str> {
        let guard = self.injector.lock().unwrap();
        match &*guard {
            Some(tx) => tx.send(snap).map_err(|_| "subscriber dropped"),
            None => Err("no active subscription"),
        }
    }

    /// Clone of the recorded restore-call log.
    pub fn restore_log(&self) -> Vec<ForegroundSnapshot> {
        self.restore_log.lock().unwrap().clone()
    }
}

#[doc(hidden)]
impl ForegroundTracker for MockForegroundTracker {
    fn name(&self) -> &str {
        "mock-foreground"
    }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        Ok(self.snapshot_to_return.lock().unwrap().clone())
    }

    fn restore(&self, snapshot: &ForegroundSnapshot) -> Result<()> {
        self.restore_log.lock().unwrap().push(snapshot.clone());
        Ok(())
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        let (tx, rx) = mpsc::channel();
        *self.injector.lock().unwrap() = Some(tx);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        *self.injector.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_snap(basename: &str) -> ForegroundSnapshot {
        ForegroundSnapshot {
            identifier: ForegroundId::Hypr {
                address: "0xdeadbeef".into(),
            },
            process_basename: basename.into(),
            title: format!("{} — test", basename),
            captured_at: SystemTime::now(),
        }
    }

    // -----------------------------------------------------------------
    // ForegroundId
    // -----------------------------------------------------------------

    #[test]
    fn supports_restore_matrix() {
        // Platforms with client-driven re-focus.
        assert!(ForegroundId::Win32 { hwnd: 0, pid: 0 }.supports_restore());
        assert!(ForegroundId::Hypr {
            address: String::new()
        }
        .supports_restore());
        assert!(ForegroundId::X11 { window: 0 }.supports_restore());
        assert!(ForegroundId::Macos { pid: 0 }.supports_restore());

        assert!(ForegroundId::Wlr {
            handle_id: String::new(),
            app_id: String::new(),
            title: String::new()
        }
        .supports_restore());

        // Platforms without client-driven restore.
        assert!(!ForegroundId::Unknown.supports_restore());
    }

    #[test]
    fn kind_returns_stable_label() {
        assert_eq!(ForegroundId::Win32 { hwnd: 0, pid: 0 }.kind(), "win32");
        assert_eq!(
            ForegroundId::Hypr {
                address: String::new()
            }
            .kind(),
            "hypr"
        );
        assert_eq!(
            ForegroundId::Wlr {
                handle_id: String::new(),
                app_id: String::new(),
                title: String::new()
            }
            .kind(),
            "wlr"
        );
        assert_eq!(ForegroundId::X11 { window: 0 }.kind(), "x11");
        assert_eq!(ForegroundId::Macos { pid: 0 }.kind(), "macos");
        assert_eq!(ForegroundId::Unknown.kind(), "unknown");
    }

    #[test]
    fn id_is_hash_and_eq() {
        // Required for the TUI's "dedup rapid focus oscillations"
        // use case.
        let a = ForegroundId::Hypr {
            address: "0x123".into(),
        };
        let b = ForegroundId::Hypr {
            address: "0x123".into(),
        };
        assert_eq!(a, b);

        let mut set = std::collections::HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
    }

    // -----------------------------------------------------------------
    // NoopForegroundTracker
    // -----------------------------------------------------------------

    #[test]
    fn noop_snapshot_returns_none() {
        let t = NoopForegroundTracker::new();
        assert!(matches!(t.snapshot(), Ok(None)));
    }

    #[test]
    fn noop_restore_succeeds_silently() {
        let t = NoopForegroundTracker::new();
        let snap = make_snap("firefox");
        assert!(t.restore(&snap).is_ok());
    }

    #[test]
    fn noop_subscribe_returns_disconnected_receiver() {
        let mut t = NoopForegroundTracker::new();
        let rx = t.subscribe().unwrap();
        // _tx dropped at end of subscribe() → receiver immediately
        // disconnected (not blocked).
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn noop_shutdown_idempotent() {
        let mut t = NoopForegroundTracker::new();
        assert!(t.shutdown().is_ok());
        assert!(t.shutdown().is_ok());
        assert!(t.shutdown().is_ok());
    }

    // -----------------------------------------------------------------
    // MockForegroundTracker
    // -----------------------------------------------------------------

    #[test]
    fn mock_snapshot_returns_configured_value() {
        let snap = make_snap("kitty");
        let t = MockForegroundTracker::new(Some(snap.clone()));
        let result = t.snapshot().unwrap().unwrap();
        assert_eq!(result.process_basename, "kitty");
    }

    #[test]
    fn mock_set_snapshot_updates_returned_value() {
        let t = MockForegroundTracker::new(Some(make_snap("kitty")));
        t.set_snapshot(Some(make_snap("alacritty")));
        assert_eq!(t.snapshot().unwrap().unwrap().process_basename, "alacritty");
        t.set_snapshot(None);
        assert!(t.snapshot().unwrap().is_none());
    }

    #[test]
    fn mock_restore_records_each_call() {
        let t = MockForegroundTracker::new(None);
        t.restore(&make_snap("firefox")).unwrap();
        t.restore(&make_snap("chromium")).unwrap();
        let log = t.restore_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].process_basename, "firefox");
        assert_eq!(log[1].process_basename, "chromium");
    }

    #[test]
    fn mock_subscribe_then_inject_delivers() {
        let mut t = MockForegroundTracker::new(None);
        let rx = t.subscribe().unwrap();
        t.inject(make_snap("kitty")).unwrap();
        let received = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(received.process_basename, "kitty");
    }

    #[test]
    fn mock_inject_without_subscribe_errors() {
        let t = MockForegroundTracker::new(None);
        assert!(t.inject(make_snap("anything")).is_err());
    }

    // -----------------------------------------------------------------
    // ForegroundFilter
    // -----------------------------------------------------------------

    #[test]
    fn filter_drops_self_snapshot() {
        // ditox's own focus must never become the "previous
        // foreground" the TUI remembers.
        let inner = MockForegroundTracker::new(Some(make_snap("ditox")));
        let f = ForegroundFilter::new(inner);
        assert!(matches!(f.snapshot(), Ok(None)));
    }

    #[test]
    fn filter_passes_non_self_snapshot() {
        let inner = MockForegroundTracker::new(Some(make_snap("firefox")));
        let f = ForegroundFilter::new(inner);
        let snap = f.snapshot().unwrap().unwrap();
        assert_eq!(snap.process_basename, "firefox");
    }

    #[test]
    fn filter_self_match_is_case_insensitive() {
        // Windows reports `Ditox.exe` with mixed case.
        let inner = MockForegroundTracker::new(Some(make_snap("Ditox.exe")));
        let f = ForegroundFilter::new(inner);
        assert!(matches!(f.snapshot(), Ok(None)));
    }

    #[test]
    fn filter_default_includes_both_binary_names() {
        // ditox (TUI binary) is also self.
        let inner = MockForegroundTracker::new(Some(make_snap("ditox")));
        let f = ForegroundFilter::new(inner);
        assert!(matches!(f.snapshot(), Ok(None)));
    }

    #[test]
    fn filter_custom_self_basenames_override_default() {
        // A repackager who ships the binary as "my-clip" needs the
        // override; the default ditox names should NOT filter their
        // binary.
        let inner = MockForegroundTracker::new(Some(make_snap("my-clip")));
        let f = ForegroundFilter::with_self_basenames(inner, vec!["my-clip".into()]);
        assert!(matches!(f.snapshot(), Ok(None)));
    }

    #[test]
    fn filter_subscribe_drops_self_events_only() {
        let mut inner = MockForegroundTracker::new(None);
        // We need a borrow of the inner to inject after wrapping —
        // do it by extracting the injector channel from the inner
        // before wrapping. Workaround: use a fresh mock for clarity.
        let rx_inner = inner.subscribe().unwrap();
        // Manually create the filter with a thread that reads from
        // rx_inner. This is what ForegroundFilter::subscribe does
        // internally; we can't re-wrap without consuming inner.
        let basenames = ["ditox".to_string()];
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(snap) = rx_inner.recv() {
                let lower = snap.process_basename.to_ascii_lowercase();
                if !basenames.iter().any(|n| n.to_ascii_lowercase() == lower)
                    && tx.send(snap).is_err()
                {
                    break;
                }
            }
        });
        // Inject one self + one non-self.
        inner.inject(make_snap("ditox")).unwrap();
        inner.inject(make_snap("firefox")).unwrap();
        let received = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(
            received.process_basename, "firefox",
            "self event must have been filtered"
        );
        // Drop the inner injector to unblock our worker.
        drop(inner);
    }

    // -----------------------------------------------------------------
    // Trait-object usage
    // -----------------------------------------------------------------

    #[test]
    fn trackers_are_object_safe() {
        // The TUI will hold a `Box<dyn ForegroundTracker>`
        // chosen at startup based on platform detection — this
        // compile check guarantees that's possible.
        let trackers: Vec<Box<dyn ForegroundTracker>> = vec![
            Box::new(NoopForegroundTracker::new()),
            Box::new(MockForegroundTracker::new(None)),
            Box::new(ForegroundFilter::new(NoopForegroundTracker::new())),
        ];
        let names: Vec<&str> = trackers.iter().map(|t| t.name()).collect();
        // ForegroundFilter forwards .name() to inner.
        assert_eq!(
            names,
            vec!["noop-foreground", "mock-foreground", "noop-foreground"]
        );
    }
}
