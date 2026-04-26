//! Suspend/resume awareness (Phase 3 sub-task 3.5).
//!
//! When the user's machine suspends (laptop lid close, "Sleep" menu,
//! systemd `suspend.target`) and later resumes, ditox's long-running
//! components want to know:
//!
//! - The watcher's in-memory `last_hash` may now be stale — anything
//!   the user copied during suspend (yes, that happens; the system
//!   may briefly wake to handle a USB event) wouldn't be in our
//!   history but our hash dedup would suppress its capture.
//! - Database file descriptors held by SQLite WAL mode survive the
//!   sleep but its connection's prepared-statement cache may have
//!   stale page references after a long idle.
//! - Wayland clients (e.g. our `wl-clipboard-rs` client inside
//!   `WaylandLibraryCapture`) may have lost their connection to the
//!   compositor depending on the compositor's session restart
//!   policy.
//!
//! Most of these auto-heal on the next operation, but the
//! `last_hash` staleness is a real bug — without an explicit reset
//! we silently miss the first identical post-resume clip.
//!
//! ## API
//!
//! ```ignore
//! pub trait PowerMonitor: Send {
//!     fn name(&self) -> &str;
//!     fn subscribe(&mut self) -> Result<mpsc::Receiver<PowerEvent>>;
//!     fn shutdown(&mut self) -> Result<()>;
//! }
//! ```
//!
//! Same shape as `ForegroundTracker` / `CaptureSource` — sync trait,
//! `Send`-only, event-driven impls run a worker thread and surface
//! events through `mpsc::Receiver`.
//!
//! ## Implementations
//!
//! - **Linux** ([`logind::LogindPowerMonitor`]): subscribes to
//!   `org.freedesktop.login1.Manager.PrepareForSleep` via `zbus`'s
//!   blocking API. Available iff `systemd-logind` is the active
//!   session manager (i.e. ~every mainstream distro except minimal
//!   non-systemd setups).
//! - **All platforms** ([`NoopPowerMonitor`]): no-op fallback.
//!   Returned from [`build_default_monitor`] when the platform-native
//!   path isn't available (Windows today; non-systemd Linux;
//!   anything macOS).
//!
//! ## Wiring (sub-task 3.5 follow-through)
//!
//! [`crate::watcher::Watcher::run_loop`] subscribes to the monitor
//! and clears `last_hash` on `PowerEvent::Resumed` so the next poll
//! re-reads whatever ended up on the clipboard during sleep.

use crate::error::Result;
use std::sync::mpsc;

#[cfg(unix)]
pub mod logind;

/// Discrete events surfaced by a [`PowerMonitor`]. Only the resume
/// case currently triggers behaviour in the watcher; the suspend
/// event is exposed for future use (e.g. flushing pending writes
/// before sleep).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerEvent {
    /// The system is about to suspend. Exposed for completeness and
    /// for future "flush before sleep" use cases.
    Suspending,
    /// The system has just resumed from suspend. Recipients should
    /// reset stale state (clipboard hash, prepared statements,
    /// compositor connections) on this signal.
    Resumed,
}

/// Source of [`PowerEvent`]s. Same sync-trait + worker-thread
/// pattern as [`crate::foreground::ForegroundTracker`] and
/// [`crate::capture::CaptureSource`].
pub trait PowerMonitor: Send {
    /// Diagnostic identifier used in logs.
    fn name(&self) -> &str;

    /// Start the monitor and return a receiver of events. The
    /// monitor owns its worker thread; events arrive asynchronously
    /// from the platform's notification subsystem (DBus signal,
    /// Win32 WM_POWERBROADCAST, etc.).
    ///
    /// Calling `subscribe` more than once on the same monitor is
    /// implementation-defined — most impls only support one
    /// subscriber.
    fn subscribe(&mut self) -> Result<mpsc::Receiver<PowerEvent>>;

    /// Stop the worker thread. Idempotent. The monitor must release
    /// any platform resources (DBus connection, message-only
    /// window) before returning.
    fn shutdown(&mut self) -> Result<()>;
}

/// No-op monitor used as fallback when the platform doesn't provide
/// a native suspend/resume signal or when the user has disabled
/// the integration. `subscribe` returns a channel whose sender is
/// dropped immediately, so the receiver yields `Disconnected` on
/// the first read — recipients can detect this and skip the
/// resume-handling code path entirely.
pub struct NoopPowerMonitor;

impl Default for NoopPowerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl NoopPowerMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl PowerMonitor for NoopPowerMonitor {
    fn name(&self) -> &str {
        "noop-power"
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<PowerEvent>> {
        let (_tx, rx) = mpsc::channel();
        // _tx is dropped here; rx returns Err(Disconnected) on
        // first recv. Callers using `try_recv` see the channel as
        // permanently empty, which is the behaviour we want.
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Build the per-platform default monitor. Walks platforms in
/// preference order:
///
/// - Linux: tries `LogindPowerMonitor`; falls back to `NoopPowerMonitor`
///   when DBus / logind are unavailable.
/// - macOS / Windows / other Unix: `NoopPowerMonitor` (Windows
///   support deferred to a follow-up; macOS to Phase 8).
///
/// Always returns a usable monitor — never errors; callers don't
/// need a fallback.
pub fn build_default_monitor() -> Box<dyn PowerMonitor> {
    #[cfg(target_os = "linux")]
    {
        match logind::LogindPowerMonitor::new() {
            Ok(monitor) => return Box::new(monitor),
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "logind power monitor unavailable; falling back to noop"
                );
            }
        }
    }

    Box::new(NoopPowerMonitor::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_subscribe_returns_disconnected_channel() {
        let mut m = NoopPowerMonitor::new();
        let rx = m.subscribe().expect("noop subscribe always succeeds");
        match rx.try_recv() {
            Err(mpsc::TryRecvError::Disconnected) => {}
            other => panic!("expected Disconnected, got {:?}", other),
        }
    }

    #[test]
    fn noop_shutdown_is_idempotent() {
        let mut m = NoopPowerMonitor::new();
        m.shutdown().expect("first shutdown");
        m.shutdown().expect("second shutdown");
    }

    #[test]
    fn noop_name() {
        assert_eq!(NoopPowerMonitor::new().name(), "noop-power");
    }

    #[test]
    fn power_event_equality() {
        assert_eq!(PowerEvent::Suspending, PowerEvent::Suspending);
        assert_eq!(PowerEvent::Resumed, PowerEvent::Resumed);
        assert_ne!(PowerEvent::Suspending, PowerEvent::Resumed);
    }

    #[test]
    fn build_default_monitor_returns_a_monitor() {
        // The factory must never panic. On a CI host without
        // logind / DBus access it returns Noop; on a desktop with
        // logind it returns the real impl. Either way it shouldn't
        // panic.
        let m = build_default_monitor();
        let _ = m.name();
    }

    #[test]
    fn power_monitor_is_object_safe() {
        // Sanity: trait must be usable as `Box<dyn PowerMonitor>`.
        let _: Box<dyn PowerMonitor> = Box::new(NoopPowerMonitor::new());
    }
}
