//! Generalised capture-source abstraction.
//!
//! Today's `Watcher` polls a single `Clipboard` impl. Phase 1
//! (multi-format) and Phase 8 (macOS) both want multiple capture
//! sources running side-by-side: an event-driven Windows
//! `AddClipboardFormatListener`, a polling Wayland fallback, an X11
//! selection listener, etc.
//!
//! This module introduces the `CaptureSource` trait and a
//! `RawClip` / `RawFormat` data model that's intentionally richer than
//! the current `ClipboardImage` so multi-format work in Phase 1 doesn't
//! have to redo the abstractions.
//!
//! The current `Watcher::poll_internal` is **not yet refactored** to
//! consume `CaptureSource` — that's a separate, larger refactor that
//! happens with Phase 1's multi-format work. This module defines the
//! contract so that work can begin without further plumbing.

use crate::error::Result;
use std::sync::mpsc;
use std::time::SystemTime;

/// Linux-only capture backend built on `wl-clipboard-rs` (see
/// task 023 sub-task 1.3). Provides multi-format snapshots from a
/// real Wayland session; replaces the `wl-paste` shell-out path.
#[cfg(unix)]
pub mod wayland;

/// A clipboard snapshot at a particular instant, before any
/// processing or persistence.
///
/// Multi-format: `formats` may carry one entry (today: just text or
/// just image) or many (Phase 1: text + html + rtf + image variants).
#[derive(Debug, Clone)]
pub struct RawClip {
    pub captured_at: SystemTime,
    /// Process basename of the source app, when known. Filled in by
    /// the Phase 2 foreground tracker; `None` until then.
    pub source_app: Option<String>,
    /// Per-format raw bytes. At least one entry.
    pub formats: Vec<RawFormat>,
}

/// One clipboard format payload.
///
/// `mime` is the canonical MIME-type-ish string. Phase 1 adopts
/// MIME-style names (`text/plain`, `text/html`, `image/png`,
/// `application/x-files`) for cross-platform portability;
/// Windows-specific formats are prefixed `win32:` (e.g.
/// `win32:CF_DIB`, `win32:CF_HDROP`).
#[derive(Debug, Clone)]
pub struct RawFormat {
    pub mime: String,
    pub bytes: Vec<u8>,
}

impl RawClip {
    /// Convenience: build a single-format text clip.
    pub fn text(s: String) -> Self {
        Self {
            captured_at: SystemTime::now(),
            source_app: None,
            formats: vec![RawFormat {
                mime: "text/plain;charset=utf-8".to_string(),
                bytes: s.into_bytes(),
            }],
        }
    }

    /// Convenience: build a single-format image clip.
    pub fn image(bytes: Vec<u8>, extension: &str) -> Self {
        let mime = match extension {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            _ => "image/png",
        };
        Self {
            captured_at: SystemTime::now(),
            source_app: None,
            formats: vec![RawFormat {
                mime: mime.to_string(),
                bytes,
            }],
        }
    }

    /// Returns the first format whose MIME starts with `prefix`.
    /// Useful for "give me any text format" queries.
    pub fn first_with_prefix(&self, prefix: &str) -> Option<&RawFormat> {
        self.formats.iter().find(|f| f.mime.starts_with(prefix))
    }
}

/// A source that can produce `RawClip` events.
///
/// Designed for both polling backends (current Wayland `wl-paste`
/// shell-out, Windows `arboard`) and event-driven backends (Phase 1
/// `AddClipboardFormatListener`).
///
/// The contract is **synchronous** (no `async fn`) to keep
/// `ditox-core` runtime-agnostic. Async backends sit a thread away
/// behind the `mpsc::Receiver` returned by `subscribe()`.
pub trait CaptureSource: Send {
    /// Stable identifier for this source. Used in logs and metrics.
    fn name(&self) -> &str;

    /// Read the current clipboard snapshot synchronously, without
    /// subscribing. Used at startup to prime dedup state without
    /// re-capturing existing content.
    ///
    /// Returns `Ok(None)` when the clipboard is empty (or holds
    /// nothing this source recognises).
    fn current_snapshot(&self) -> Result<Option<RawClip>>;

    /// Subscribe to a stream of clipboard-changed events.
    ///
    /// The returned receiver yields one `RawClip` per detected
    /// change. Polling backends produce events at the configured
    /// poll cadence; event-driven backends produce events as the OS
    /// notifies them.
    ///
    /// The source is responsible for spawning whatever background
    /// thread / OS-level subscription it needs. Calling `shutdown()`
    /// must terminate that work cleanly.
    ///
    /// Calling `subscribe()` more than once on the same instance is
    /// undefined behaviour from the trait's POV; concrete
    /// implementations may either return an error or silently
    /// replace the previous subscription.
    fn subscribe(&mut self) -> Result<mpsc::Receiver<RawClip>>;

    /// Stop the background work started by `subscribe()`. Idempotent.
    fn shutdown(&mut self) -> Result<()>;
}

// ============================================================================
// Polling adapter
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Generic polling adapter that turns a synchronous "read current
/// clipboard" closure into a `CaptureSource`. Used by the existing
/// Wayland `wl-paste` and Windows `arboard` backends until Phase 1
/// rewrites them as event-driven.
pub struct PollingCaptureSource<F>
where
    F: Fn() -> Result<Option<RawClip>> + Send + Sync + 'static,
{
    name: String,
    interval: Duration,
    read_fn: Arc<F>,
    /// Set to true on `shutdown()`. The background thread loops on
    /// this flag.
    shutdown_flag: Arc<AtomicBool>,
    /// Handle to the background thread. `None` until `subscribe()`.
    join: Option<std::thread::JoinHandle<()>>,
}

impl<F> PollingCaptureSource<F>
where
    F: Fn() -> Result<Option<RawClip>> + Send + Sync + 'static,
{
    pub fn new(name: impl Into<String>, interval_ms: u64, read_fn: F) -> Self {
        Self {
            name: name.into(),
            interval: Duration::from_millis(interval_ms),
            read_fn: Arc::new(read_fn),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            join: None,
        }
    }
}

impl<F> CaptureSource for PollingCaptureSource<F>
where
    F: Fn() -> Result<Option<RawClip>> + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn current_snapshot(&self) -> Result<Option<RawClip>> {
        (self.read_fn)()
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<RawClip>> {
        // Reset the shutdown flag in case we're being re-subscribed.
        self.shutdown_flag.store(false, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        let read_fn = self.read_fn.clone();
        let interval = self.interval;
        let shutdown = self.shutdown_flag.clone();
        let name = self.name.clone();

        // Hash de-duping: emit only on hash change so subscribers don't
        // see the same content repeatedly.
        let join = std::thread::Builder::new()
            .name(format!("ditox-capture-{}", self.name))
            .spawn(move || {
                let mut last_hash: Option<String> = None;
                while !shutdown.load(Ordering::SeqCst) {
                    match read_fn() {
                        Ok(Some(clip)) => {
                            let h = clip_hash(&clip);
                            if last_hash.as_ref() != Some(&h) {
                                last_hash = Some(h);
                                if tx.send(clip).is_err() {
                                    // receiver dropped — no point continuing
                                    tracing::debug!(
                                        "capture source {} receiver dropped, exiting",
                                        name
                                    );
                                    break;
                                }
                            }
                        }
                        Ok(None) => {
                            // Clipboard cleared — reset the dedup hash so
                            // that repeating an old value triggers an event.
                            last_hash = None;
                        }
                        Err(e) => {
                            tracing::warn!("capture source {} read error: {}", name, e);
                        }
                    }
                    std::thread::sleep(interval);
                }
            })
            .map_err(|e| crate::error::DitoxError::Other(format!("spawn capture thread: {}", e)))?;

        self.join = Some(join);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            // Don't propagate join errors — we just want to wait politely.
            let _ = join.join();
        }
        Ok(())
    }
}

impl<F> Drop for PollingCaptureSource<F>
where
    F: Fn() -> Result<Option<RawClip>> + Send + Sync + 'static,
{
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Hash a `RawClip` for in-source dedup. Stable across runs (formats
/// sorted by MIME). Not used as a security primitive — Phase 1 will
/// replace this with proper SHA-256 + canonicalisation.
pub fn clip_hash(clip: &RawClip) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut sorted: Vec<&RawFormat> = clip.formats.iter().collect();
    sorted.sort_by(|a, b| a.mime.cmp(&b.mime));
    for f in sorted {
        hasher.update(f.mime.as_bytes());
        hasher.update(b"\0");
        hasher.update(&f.bytes);
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

// ============================================================================
// Test-only mock source
// ============================================================================

/// Mock capture source for unit tests. Holds a queued list of clips
/// and an external sender to inject more dynamically.
#[doc(hidden)]
pub struct MockCaptureSource {
    name: String,
    queued: Vec<RawClip>,
    /// External sender so tests can push more clips after `subscribe()`.
    pub injector: Option<mpsc::Sender<RawClip>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl MockCaptureSource {
    pub fn new(name: impl Into<String>, queued: Vec<RawClip>) -> Self {
        Self {
            name: name.into(),
            queued,
            injector: None,
            join: None,
        }
    }
}

impl CaptureSource for MockCaptureSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn current_snapshot(&self) -> Result<Option<RawClip>> {
        Ok(self.queued.first().cloned())
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<RawClip>> {
        let (tx, rx) = mpsc::channel();
        // Drain the queued clips immediately.
        for clip in self.queued.drain(..) {
            tx.send(clip).ok();
        }
        // Hold onto the sender so tests can inject more.
        self.injector = Some(tx);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        self.injector = None;
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn raw_clip_text_constructor_preserves_bytes() {
        let c = RawClip::text("hello".to_string());
        assert_eq!(c.formats.len(), 1);
        assert_eq!(c.formats[0].mime, "text/plain;charset=utf-8");
        assert_eq!(c.formats[0].bytes, b"hello");
        assert!(c.source_app.is_none());
    }

    #[test]
    fn raw_clip_image_constructor_picks_mime() {
        let c = RawClip::image(vec![1, 2, 3], "png");
        assert_eq!(c.formats[0].mime, "image/png");

        let c = RawClip::image(vec![1, 2, 3], "jpg");
        assert_eq!(c.formats[0].mime, "image/jpeg");

        let c = RawClip::image(vec![1, 2, 3], "unknown");
        assert_eq!(c.formats[0].mime, "image/png");
    }

    #[test]
    fn first_with_prefix_finds_format() {
        let c = RawClip {
            captured_at: SystemTime::now(),
            source_app: None,
            formats: vec![
                RawFormat {
                    mime: "image/png".to_string(),
                    bytes: vec![0x89, 0x50],
                },
                RawFormat {
                    mime: "text/plain".to_string(),
                    bytes: b"hi".to_vec(),
                },
            ],
        };
        assert!(c.first_with_prefix("text/").is_some());
        assert!(c.first_with_prefix("image/").is_some());
        assert!(c.first_with_prefix("application/").is_none());
    }

    #[test]
    fn clip_hash_is_format_order_independent() {
        let a = RawClip {
            captured_at: SystemTime::now(),
            source_app: None,
            formats: vec![
                RawFormat {
                    mime: "text/html".to_string(),
                    bytes: b"<p>hi</p>".to_vec(),
                },
                RawFormat {
                    mime: "text/plain".to_string(),
                    bytes: b"hi".to_vec(),
                },
            ],
        };
        let b = RawClip {
            captured_at: SystemTime::now(),
            source_app: None,
            formats: vec![
                RawFormat {
                    mime: "text/plain".to_string(),
                    bytes: b"hi".to_vec(),
                },
                RawFormat {
                    mime: "text/html".to_string(),
                    bytes: b"<p>hi</p>".to_vec(),
                },
            ],
        };
        assert_eq!(clip_hash(&a), clip_hash(&b));
    }

    #[test]
    fn polling_source_emits_on_change_and_dedups() {
        let counter = std::sync::Arc::new(Mutex::new(0u32));
        let counter_for_closure = counter.clone();
        let mut src = PollingCaptureSource::new("test", 5, move || {
            let mut c = counter_for_closure.lock().unwrap();
            *c += 1;
            // Toggle: produce "A", "A", "B", "B", "C"… so dedup
            // collapses repeats but emits on change.
            let payload = if *c <= 2 {
                "A"
            } else if *c <= 4 {
                "B"
            } else {
                "C"
            };
            Ok(Some(RawClip::text(payload.to_string())))
        });

        let rx = src.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(80));
        src.shutdown().unwrap();

        let received: Vec<RawClip> = rx.try_iter().collect();
        // We should see A, B, C (in order). C may or may not appear
        // depending on timing; assert at least 2 distinct clips.
        assert!(received.len() >= 2, "got {} clips", received.len());
        let bodies: Vec<String> = received
            .iter()
            .map(|c| String::from_utf8_lossy(&c.formats[0].bytes).to_string())
            .collect();
        // A must be first, then B at some point (no duplicates of A).
        assert_eq!(bodies[0], "A");
        assert!(bodies.contains(&"B".to_string()));
        // No consecutive duplicates.
        for w in bodies.windows(2) {
            assert_ne!(w[0], w[1], "got consecutive dup: {:?}", bodies);
        }
    }

    #[test]
    fn mock_source_drains_queued_then_accepts_injections() {
        let mut src = MockCaptureSource::new(
            "mock",
            vec![
                RawClip::text("pre-1".to_string()),
                RawClip::text("pre-2".to_string()),
            ],
        );
        let rx = src.subscribe().unwrap();
        // Inject one more after subscribe.
        if let Some(tx) = src.injector.clone() {
            tx.send(RawClip::text("post-1".to_string())).unwrap();
        }
        let mut got = Vec::new();
        for _ in 0..3 {
            got.push(rx.recv_timeout(Duration::from_millis(100)).unwrap());
        }
        assert_eq!(got[0].formats[0].bytes, b"pre-1");
        assert_eq!(got[1].formats[0].bytes, b"pre-2");
        assert_eq!(got[2].formats[0].bytes, b"post-1");
        src.shutdown().unwrap();
    }

    #[test]
    fn shutdown_is_idempotent() {
        let mut src = PollingCaptureSource::new("idempot", 5, || Ok(None));
        let _rx = src.subscribe().unwrap();
        src.shutdown().unwrap();
        // Second shutdown shouldn't panic.
        src.shutdown().unwrap();
    }
}
