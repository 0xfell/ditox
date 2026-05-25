//! Hyprland foreground tracker via `hyprctl` IPC (Phase 2 sub-task 2.3).
//!
//! Hyprland exposes the current foreground via:
//!
//! ```text
//! hyprctl activewindow -j
//! ```
//!
//! which prints a JSON object with `address`, `class`, `title`,
//! `pid`, etc. for the focused window. We shell out, parse, and
//! translate to a [`ForegroundSnapshot`] with
//! [`ForegroundId::Hypr { address }`].
//!
//! `restore()` calls:
//!
//! ```text
//! hyprctl dispatch focuswindow address:0x<address>
//! ```
//!
//! Hyprland also publishes an event stream on
//! `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`.
//! `subscribe()` tails that socket and emits a fresh
//! `hyprctl activewindow -j` snapshot whenever Hyprland reports an
//! `activewindow` / `activewindowv2` focus-change event.
//!
//! For the wlroots-but-not-Hyprland compositors (Sway, generic
//! wlroots, KDE Wayland), the equivalent is the
//! `wlr-foreign-toplevel-management-unstable-v1` Wayland protocol —
//! more complex, deferred to a follow-up sub-task.
//!
//! ## Process basename
//!
//! Hyprland's `class` field is the Wayland `app_id` (e.g.
//! `firefox`, `kitty`, `brave-browser`) — close enough to the
//! executable basename in most cases. We optionally cross-check
//! against `/proc/<pid>/comm` when present, preferring the comm
//! value (literal binary basename without extension; matches
//! Linux convention for the per-app keystroke-override lookup in
//! sub-task 2.6).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use std::os::unix::net::UnixStream;

use serde::Deserialize;

use crate::error::{DitoxError, Result};
use crate::foreground::{ForegroundId, ForegroundSnapshot, ForegroundTracker};

/// Foreground tracker for Hyprland.
///
/// One instance per launcher is fine — each call to `snapshot()` /
/// `restore()` spawns its own `hyprctl` invocation; no Wayland
/// connection is held.
pub struct HyprctlForegroundTracker {
    binary: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HyprctlForegroundTracker {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("hyprctl"),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    /// Override the binary path; for tests + non-standard installs.
    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    /// Build the argv for the snapshot call (`hyprctl activewindow -j`).
    /// Exposed for tests + diagnostic logging.
    pub fn snapshot_argv(&self) -> Vec<String> {
        vec![
            self.binary.to_string_lossy().into_owned(),
            "activewindow".to_string(),
            "-j".to_string(),
        ]
    }

    /// Build the argv for a `restore()` call. Exposed for tests +
    /// diagnostic logging.
    pub fn restore_argv(&self, snapshot: &ForegroundSnapshot) -> Result<Vec<String>> {
        let address = match &snapshot.identifier {
            ForegroundId::Hypr { address } => address.clone(),
            other => {
                return Err(DitoxError::Other(format!(
                    "hyprctl tracker can't restore non-Hypr identifier: {}",
                    other.kind()
                )));
            }
        };
        Ok(vec![
            self.binary.to_string_lossy().into_owned(),
            "dispatch".to_string(),
            "focuswindow".to_string(),
            format!("address:{}", address),
        ])
    }
}

impl Default for HyprctlForegroundTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Subset of `hyprctl activewindow -j` output we care about.
///
/// `serde(default)` so missing fields decode as their `Default`
/// (typically empty / `0`) — Hyprland adds new keys over releases
/// and we want to forward-tolerate.
#[derive(Debug, Clone, Deserialize, Default)]
struct HyprctlActiveWindow {
    #[serde(default)]
    address: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    pid: i32,
}

/// Parse `hyprctl activewindow -j` JSON into a `ForegroundSnapshot`.
///
/// Returns `None` when:
/// - `address` is empty (no window focused — Hyprland returns `{}`).
/// - JSON is malformed (logged at `warn` and treated as no focus).
///
/// `process_basename` resolution order:
/// 1. The JSON `class` field (Wayland app_id, e.g. `firefox`,
///    `kitty`, `brave-browser`). Aligns with the Linux convention
///    users configure in `[paste.keystrokes]`.
/// 2. `/proc/<pid>/comm` as fallback if `class` is empty and
///    `pid > 0`. **Note:** Linux truncates `comm` to 15 characters
///    (TASK_COMM_LEN-1) and Nix-wrapped binaries may carry a
///    leading `.` (`.firefox-wrapped` becomes `.firefox-wrappe`),
///    so `class` is preferred when both are available.
///
/// Public so test fixtures can construct snapshots from canned JSON
/// without invoking `hyprctl`.
pub fn parse_activewindow(json: &str) -> Option<ForegroundSnapshot> {
    let aw: HyprctlActiveWindow = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "hyprctl: failed to parse activewindow JSON");
            return None;
        }
    };
    if aw.address.is_empty() {
        return None;
    }
    let process_basename = if !aw.class.is_empty() {
        aw.class.clone()
    } else if aw.pid > 0 {
        proc_comm(aw.pid).unwrap_or_default()
    } else {
        String::new()
    };
    Some(ForegroundSnapshot {
        identifier: ForegroundId::Hypr {
            address: aw.address,
        },
        process_basename,
        title: aw.title,
        captured_at: SystemTime::now(),
    })
}

/// Read `/proc/<pid>/comm` and return the trimmed contents.
///
/// Returns `None` when the file is missing (process exited between
/// the `hyprctl` call and our read), unreadable, or empty.
#[cfg(unix)]
fn proc_comm(pid: i32) -> Option<String> {
    let path = format!("/proc/{}/comm", pid);
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(not(unix))]
fn proc_comm(_pid: i32) -> Option<String> {
    None
}

impl ForegroundTracker for HyprctlForegroundTracker {
    fn name(&self) -> &str {
        "hyprctl-foreground"
    }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        snapshot_with_binary(&self.binary)
    }

    fn restore(&self, snapshot: &ForegroundSnapshot) -> Result<()> {
        let argv = self.restore_argv(snapshot)?;
        let status = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| DitoxError::Other(format!("spawn hyprctl restore: {}", e)))?;
        if !status.success() {
            return Err(DitoxError::Other(format!(
                "hyprctl dispatch focuswindow exited {}",
                status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into())
            )));
        }
        Ok(())
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        if self.thread.is_some() {
            self.shutdown()?;
        }

        let socket = hyprland_event_socket_path_from_env()?;
        let stream = UnixStream::connect(&socket).map_err(|e| {
            DitoxError::Other(format!(
                "connect Hyprland event socket {}: {e}",
                socket.display()
            ))
        })?;
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .map_err(|e| DitoxError::Other(format!("set Hyprland socket read timeout: {e}")))?;

        self.stop.store(false, Ordering::Relaxed);
        let stop = Arc::clone(&self.stop);
        let binary = self.binary.clone();
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("ditox-hypr-foreground".into())
            .spawn(move || run_event_loop(binary, stream, tx, stop))
            .map_err(|e| DitoxError::Other(format!("spawn hypr foreground thread: {e}")))?;
        self.thread = Some(thread);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }
}

impl Drop for HyprctlForegroundTracker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn snapshot_with_binary(binary: &Path) -> Result<Option<ForegroundSnapshot>> {
    let out = Command::new(binary)
        .args(["activewindow", "-j"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| DitoxError::Other(format!("spawn hyprctl: {}", e)))?;
    if !out.status.success() {
        // Non-zero from hyprctl on a healthy session usually means
        // "no active window" — return None rather than erroring up
        // the chain.
        tracing::trace!(
            code = ?out.status.code(),
            "hyprctl activewindow -j non-zero exit; treating as no focus"
        );
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(parse_activewindow(&stdout))
}

fn hyprland_event_socket_path_from_env() -> Result<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        DitoxError::Other("XDG_RUNTIME_DIR is not set; cannot find Hyprland socket2".to_string())
    })?;
    let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").ok_or_else(|| {
        DitoxError::Other(
            "HYPRLAND_INSTANCE_SIGNATURE is not set; cannot find Hyprland socket2".to_string(),
        )
    })?;
    Ok(hyprland_event_socket_path(runtime, signature))
}

fn hyprland_event_socket_path(
    runtime: impl Into<PathBuf>,
    signature: impl Into<PathBuf>,
) -> PathBuf {
    runtime
        .into()
        .join("hypr")
        .join(signature.into())
        .join(".socket2.sock")
}

fn run_event_loop(
    binary: PathBuf,
    stream: UnixStream,
    tx: mpsc::Sender<ForegroundSnapshot>,
    stop: Arc<AtomicBool>,
) {
    let mut reader = BufReader::new(stream);
    while !stop.load(Ordering::Relaxed) {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) if is_focus_event(&line) => match snapshot_with_binary(&binary) {
                Ok(Some(snapshot)) => {
                    if tx.send(snapshot).is_err() {
                        break;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!(%error, "hypr foreground snapshot after event failed");
                }
            },
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => {
                tracing::debug!(%error, "hypr foreground socket read failed");
                break;
            }
        }
    }
}

fn is_focus_event(line: &str) -> bool {
    line.starts_with("activewindow>>") || line.starts_with("activewindowv2>>")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-world `hyprctl activewindow -j` output, captured from a
    /// live Hyprland session 2026-04-26. Used as the reference shape
    /// for parser tests.
    const SAMPLE_JSON: &str = r#"{
        "address": "0x6168da0d31a0",
        "mapped": true,
        "hidden": false,
        "at": [0, 36],
        "size": [2560, 1404],
        "workspace": { "id": 1, "name": "1" },
        "floating": false,
        "monitor": 0,
        "class": "brave-browser",
        "title": "Some YouTube Video - Brave",
        "initialClass": "brave-browser",
        "initialTitle": "New Tab - Brave",
        "pid": 0,
        "xwayland": false,
        "pinned": false,
        "fullscreen": 0,
        "fullscreenClient": 0,
        "overFullscreen": true,
        "grouped": [],
        "tags": [],
        "swallowing": "0x0",
        "focusHistoryID": 0,
        "inhibitingIdle": true,
        "xdgTag": "",
        "xdgDescription": "",
        "contentType": "none"
    }"#;

    // -----------------------------------------------------------------
    // Parser
    // -----------------------------------------------------------------

    #[test]
    fn parse_real_activewindow_json() {
        let snap = parse_activewindow(SAMPLE_JSON).expect("must parse");
        assert!(matches!(
            snap.identifier,
            ForegroundId::Hypr { ref address } if address == "0x6168da0d31a0"
        ));
        // pid=0 → fall back to class.
        assert_eq!(snap.process_basename, "brave-browser");
        assert_eq!(snap.title, "Some YouTube Video - Brave");
    }

    #[test]
    fn parse_empty_object_returns_none() {
        // Hyprland returns `{}` when no window is focused.
        assert!(parse_activewindow("{}").is_none());
    }

    #[test]
    fn parse_missing_address_returns_none() {
        let json = r#"{ "class": "x", "title": "y", "pid": 0 }"#;
        assert!(parse_activewindow(json).is_none());
    }

    #[test]
    fn parse_malformed_json_returns_none() {
        // Garbage in → None out (logged at warn). Don't propagate
        // to the launcher; it'd spuriously fail every paste.
        assert!(parse_activewindow("not json at all").is_none());
        assert!(parse_activewindow("{").is_none());
    }

    #[test]
    fn parse_missing_class_falls_back_to_empty_basename() {
        // pid=0 + no class → process_basename is empty.
        let json = r#"{ "address": "0xabc", "title": "t", "pid": 0 }"#;
        let snap = parse_activewindow(json).expect("must parse");
        assert_eq!(snap.process_basename, "");
    }

    #[test]
    fn parse_prefers_class_over_proc_comm() {
        // Even with a real pid (whose /proc/<pid>/comm we could
        // read), the wayland class wins for keystroke-override
        // lookup ergonomics. comm is truncated to 15 chars and
        // Nix wrappers carry a leading `.` — class is the cleaner
        // user-facing identifier.
        //
        // Test uses a non-existent pid so proc_comm returns None;
        // the class field should still be used.
        let json = r#"{
            "address": "0xabc",
            "class": "firefox",
            "title": "t",
            "pid": 999999999
        }"#;
        let snap = parse_activewindow(json).expect("must parse");
        assert_eq!(snap.process_basename, "firefox");
    }

    #[test]
    fn parse_falls_back_to_proc_comm_when_class_empty() {
        // Use the current test process's pid — its /proc/comm exists
        // and contains the test runner binary name.
        let pid = std::process::id() as i32;
        let json = format!(
            r#"{{ "address": "0xabc", "class": "", "title": "t", "pid": {} }}"#,
            pid
        );
        let snap = parse_activewindow(&json).expect("must parse");
        // The test runner basename varies (`ditox_core-<hash>`); just
        // assert it's non-empty (we successfully read /proc/<pid>/comm).
        assert!(
            !snap.process_basename.is_empty(),
            "expected /proc/<pid>/comm fallback to populate basename"
        );
    }

    #[test]
    fn parse_forward_tolerates_extra_fields() {
        // Future Hyprland releases will add fields. Our parser
        // ignores everything it doesn't know.
        let json = r#"{
            "address": "0xdeadbeef",
            "class": "kitty",
            "title": "term",
            "pid": 0,
            "future_field_42": [1, 2, 3],
            "another_future_field": { "nested": true }
        }"#;
        let snap = parse_activewindow(json).expect("must parse");
        assert_eq!(snap.process_basename, "kitty");
    }

    // -----------------------------------------------------------------
    // argv builders
    // -----------------------------------------------------------------

    #[test]
    fn snapshot_argv_is_stable() {
        let t = HyprctlForegroundTracker::new();
        assert_eq!(t.snapshot_argv(), vec!["hyprctl", "activewindow", "-j"]);
    }

    #[test]
    fn restore_argv_with_hypr_target() {
        let t = HyprctlForegroundTracker::new();
        let snap = ForegroundSnapshot {
            identifier: ForegroundId::Hypr {
                address: "0xfeedface".into(),
            },
            process_basename: "kitty".into(),
            title: "term".into(),
            captured_at: SystemTime::now(),
        };
        assert_eq!(
            t.restore_argv(&snap).unwrap(),
            vec![
                "hyprctl".to_string(),
                "dispatch".into(),
                "focuswindow".into(),
                "address:0xfeedface".into(),
            ]
        );
    }

    #[test]
    fn restore_argv_rejects_non_hypr_target() {
        let t = HyprctlForegroundTracker::new();
        let snap = ForegroundSnapshot {
            identifier: ForegroundId::Wlr {
                handle_id: "1".into(),
                app_id: "x".into(),
                title: "y".into(),
            },
            process_basename: "x".into(),
            title: "y".into(),
            captured_at: SystemTime::now(),
        };
        let err = t.restore_argv(&snap).unwrap_err();
        assert!(format!("{}", err).contains("non-Hypr"));
    }

    #[test]
    fn with_binary_overrides_path() {
        let t = HyprctlForegroundTracker::with_binary("/custom/hyprctl");
        assert_eq!(t.snapshot_argv()[0], "/custom/hyprctl");
    }

    // -----------------------------------------------------------------
    // ForegroundTracker trait surface
    // -----------------------------------------------------------------

    #[test]
    fn name_is_stable() {
        let t = HyprctlForegroundTracker::new();
        assert_eq!(t.name(), "hyprctl-foreground");
    }

    #[test]
    fn shutdown_is_idempotent() {
        let mut t = HyprctlForegroundTracker::new();
        assert!(t.shutdown().is_ok());
        assert!(t.shutdown().is_ok());
    }

    #[test]
    fn event_socket_path_uses_hyprland_layout() {
        let path = hyprland_event_socket_path("/run/user/1000", "instance-123");
        assert_eq!(
            path,
            PathBuf::from("/run/user/1000/hypr/instance-123/.socket2.sock")
        );
    }

    #[test]
    fn focus_event_detection_matches_hyprland_events() {
        assert!(is_focus_event("activewindow>>kitty,term\n"));
        assert!(is_focus_event("activewindowv2>>0xfeedface\n"));
        assert!(!is_focus_event("workspace>>1\n"));
        assert!(!is_focus_event("openwindow>>0x1,1,kitty,term\n"));
    }

    #[test]
    fn subscribe_without_hyprland_env_returns_error() {
        let mut t = HyprctlForegroundTracker::new();
        let old_runtime = std::env::var_os("XDG_RUNTIME_DIR");
        let old_signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE");
        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::remove_var("HYPRLAND_INSTANCE_SIGNATURE");

        let err = t.subscribe().unwrap_err();
        assert!(format!("{}", err).contains("XDG_RUNTIME_DIR"));

        if let Some(value) = old_runtime {
            std::env::set_var("XDG_RUNTIME_DIR", value);
        }
        if let Some(value) = old_signature {
            std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", value);
        }
    }

    /// End-to-end snapshot test against a real Hyprland session.
    /// `#[ignore]`d so it never runs in CI; manual diagnostic.
    ///
    /// Run with:
    /// ```text
    /// cargo test -p ditox-core --lib foreground::hyprctl::tests::live_snapshot \
    ///     -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "requires a real Hyprland session"]
    fn live_snapshot() {
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
            eprintln!("HYPRLAND_INSTANCE_SIGNATURE not set; skipping");
            return;
        }
        let t = HyprctlForegroundTracker::new();
        let snap = t
            .snapshot()
            .expect("snapshot must not error on healthy hyprland");
        match snap {
            Some(s) => {
                eprintln!("foreground: {:?}", s.identifier);
                eprintln!("  process_basename: {}", s.process_basename);
                eprintln!("  title: {}", s.title);
                assert!(matches!(s.identifier, ForegroundId::Hypr { .. }));
            }
            None => eprintln!("no active window (Hyprland returned empty)"),
        }
    }
}
