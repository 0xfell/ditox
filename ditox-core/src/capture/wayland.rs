//! Wayland clipboard capture via the `wl-clipboard-rs` library.
//!
//! Replaces the `wl-paste` subprocess shell-out in
//! [`crate::clipboard`]. The library uses the `ext-data-control` /
//! `wlr-data-control` protocols directly, so:
//!
//! - **No `$PATH` lookup** for `wl-paste` (one less moving part).
//! - **All offered MIME types are visible**, not just text + a fixed
//!   image MIME priority list. This is the actual unlock for Phase 1
//!   multi-format capture — a single `current_snapshot()` returns
//!   `text/plain;charset=utf-8` + `text/html` + `text/rtf` + `image/png`
//!   in one [`RawClip`] when the source app offers all of them.
//! - **One Wayland connection per call** — `get_contents` opens
//!   `wl_display`, does a roundtrip, and drops the connection. Cheap
//!   enough for a 250 ms poll cadence and avoids the "watcher holds
//!   a long-lived data-control surface" coupling that complicates
//!   restart semantics.
//!
//! ## Image-priority preservation
//!
//! The "Copy image" from a browser → image-not-URL behaviour
//! (AGENTS.md "Clipboard priority") is preserved by
//! [`crate::watcher::Watcher::process_clip`], which calls
//! [`RawClip::first_with_prefix("image/")`] before
//! [`RawClip::first_with_prefix("text/plain")`] when picking the
//! canonical entry. We just hand it every offered format and let it
//! choose; the priority logic does not move into this module.
//!
//! ## v0.4 limitations
//!
//! - **`subscribe()` is a stub.** The watcher only ever calls
//!   `current_snapshot()` (see `Watcher::poll_internal`), so an
//!   event-driven backend isn't required yet. Future work will use
//!   `wlr-data-control-v1::data_offer.offer` events directly via a
//!   dedicated thread; that module is currently private inside
//!   `wl-clipboard-rs` so it would require either a fork or moving
//!   to a different protocol crate (e.g. `smithay-client-toolkit`).
//! - **Pipe reads are blocking.** A slow source app (e.g. one that
//!   converts an image on the fly) will block the watcher thread
//!   until it finishes writing. Acceptable at the 250 ms poll
//!   cadence; future hardening can put each `read_to_end` behind
//!   a `select()` timeout.
//! - **Primary selection (middle-click) is ignored.** Only
//!   `ClipboardType::Regular` is read. Phase 1 doesn't include
//!   primary-selection capture; tracked as a follow-up.

use std::collections::HashSet;
use std::io::Read;
use std::sync::mpsc;
use std::time::SystemTime;

use tracing::{trace, warn};
use wl_clipboard_rs::paste::{self, ClipboardType, Error as PasteError, MimeType, Seat};

use crate::capture::{CaptureSource, RawClip, RawFormat};
use crate::config::CaptureConfig;
use crate::error::{DitoxError, Result};
use crate::format::FormatId;

/// `CaptureSource` backed by `wl-clipboard-rs::paste`.
///
/// One instance per watcher is fine — the struct itself holds no
/// Wayland state; each `current_snapshot()` opens its own connection.
pub struct WaylandLibraryCapture {
    config: CaptureConfig,
}

impl WaylandLibraryCapture {
    /// Construct with the watcher's `CaptureConfig`. The config is
    /// cloned (cheap — it's three primitive fields plus two `Vec<String>`
    /// allow/deny lists that are typically empty).
    pub fn new(config: CaptureConfig) -> Self {
        Self { config }
    }

    /// Map a `wl-clipboard-rs` paste error into our
    /// `Result<Option<T>>`. The "clipboard is empty / no seat / no
    /// matching MIME" variants all collapse to `Ok(None)` — those are
    /// expected steady-state conditions, not errors. Everything else
    /// becomes a `DitoxError::Clipboard`.
    fn translate_paste_err<T>(err: PasteError) -> Result<Option<T>> {
        match err {
            PasteError::NoSeats | PasteError::ClipboardEmpty | PasteError::NoMimeType => Ok(None),
            other => Err(DitoxError::Clipboard(format!(
                "wayland paste failed: {}",
                other
            ))),
        }
    }
}

impl CaptureSource for WaylandLibraryCapture {
    fn name(&self) -> &str {
        "wayland-library"
    }

    fn current_snapshot(&self) -> Result<Option<RawClip>> {
        // 1. Enumerate offered MIME types in compositor order. The
        //    ordered variant matters: many apps offer the "native"
        //    format first (e.g. `image/png` from a screenshot tool)
        //    followed by on-the-fly conversions. The watcher's
        //    image-vs-text precedence is per-prefix, but where two
        //    formats of the same prefix exist (e.g. `image/png` and
        //    `image/jpeg`), insertion order in `RawClip.formats`
        //    decides which one `first_with_prefix("image/")` returns.
        let mimes = match paste::get_mime_types_ordered(ClipboardType::Regular, Seat::Unspecified) {
            Ok(m) => m,
            Err(e) => return Self::translate_paste_err(e),
        };
        if mimes.is_empty() {
            return Ok(None);
        }

        let mut seen_canonical: HashSet<String> = HashSet::with_capacity(mimes.len());
        let mut formats: Vec<RawFormat> = Vec::with_capacity(mimes.len());

        for mime in &mimes {
            let canonical = FormatId::from_wayland_mime(mime).canonical();

            // Per-format allow/deny from the user's config. Done
            // before the read so a denied format never opens a pipe.
            if !self.config.should_capture_format(&canonical) {
                trace!(
                    raw = %mime,
                    canonical = %canonical,
                    "wayland: format disallowed by capture config"
                );
                continue;
            }

            // Compositors sometimes offer the same payload under
            // multiple synonym MIMEs (`text/plain` + `UTF8_STRING` +
            // `text/plain;charset=utf-8`). They all collapse to one
            // canonical name — keep the first read, skip the rest.
            if !seen_canonical.insert(canonical.clone()) {
                trace!(
                    raw = %mime,
                    canonical = %canonical,
                    "wayland: duplicate canonical MIME after normalisation"
                );
                continue;
            }

            // Open the per-MIME pipe. Failure on a single MIME is
            // logged-and-skipped rather than fatal — the source app
            // may have unregistered the format between the
            // enumeration call and the read. A cleared clipboard
            // mid-snapshot returns `Ok(None)` for the whole call.
            let (pipe, _actual_mime) = match paste::get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                MimeType::Specific(mime),
            ) {
                Ok(t) => t,
                Err(PasteError::NoSeats)
                | Err(PasteError::ClipboardEmpty)
                | Err(PasteError::NoMimeType) => {
                    return Ok(None);
                }
                Err(e) => {
                    warn!(
                        raw = %mime,
                        error = %e,
                        "wayland: get_contents failed; skipping this format"
                    );
                    continue;
                }
            };

            // Bounded read: cap + 1 byte. The +1 lets us detect "ran
            // up to or past the cap" without a second read pass.
            let cap = self.config.max_format_size_bytes;
            let mut limited = pipe.take(cap.saturating_add(1));
            let mut bytes = Vec::new();
            if let Err(e) = limited.read_to_end(&mut bytes) {
                warn!(
                    raw = %mime,
                    error = %e,
                    "wayland: pipe read failed; skipping this format"
                );
                continue;
            }
            if (bytes.len() as u64) > cap {
                warn!(
                    raw = %mime,
                    canonical = %canonical,
                    bytes = bytes.len(),
                    cap,
                    "wayland: format exceeds max_format_size_bytes; dropping"
                );
                continue;
            }
            // X11-leaked meta-targets (TARGETS, TIMESTAMP, …) and
            // misbehaving sources sometimes resolve to zero bytes
            // for a real MIME type. Skip — an empty payload is never
            // useful and would waste an `entry_formats` row.
            if bytes.is_empty() {
                trace!(raw = %mime, canonical = %canonical, "wayland: empty payload; skipping");
                continue;
            }

            formats.push(RawFormat {
                mime: canonical,
                bytes,
            });
        }

        if formats.is_empty() {
            return Ok(None);
        }

        // Per-clip total size cap. When exceeded, drop the WHOLE clip
        // rather than partial-write — a half-captured rich clip with
        // missing image is worse than a missed clip the user can re-copy.
        let total: u64 = formats.iter().map(|f| f.bytes.len() as u64).sum();
        if !self.config.clip_size_ok(total) {
            warn!(
                total,
                cap = self.config.max_clip_size_bytes,
                "wayland: total clip exceeds max_clip_size_bytes; dropping"
            );
            return Ok(None);
        }

        Ok(Some(RawClip {
            captured_at: SystemTime::now(),
            source_app: None,
            formats,
        }))
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<RawClip>> {
        // v0.4: not wired. The watcher polls via current_snapshot()
        // at config.general.poll_interval_ms. Returning an empty
        // channel keeps the trait contract (Send, drop-safe) without
        // pretending to deliver events.
        let (_tx, rx) = mpsc::channel();
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        // Nothing to clean up — `paste::get_contents` opens and drops
        // its own `wl_display` per call. Idempotent.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CaptureConfig, CaptureMode};

    #[test]
    fn name_is_stable() {
        let cap = WaylandLibraryCapture::new(CaptureConfig::default());
        assert_eq!(cap.name(), "wayland-library");
    }

    #[test]
    fn shutdown_is_idempotent() {
        let mut cap = WaylandLibraryCapture::new(CaptureConfig::default());
        assert!(cap.shutdown().is_ok());
        assert!(cap.shutdown().is_ok());
        assert!(cap.shutdown().is_ok());
    }

    #[test]
    fn translate_paste_err_collapses_empty_variants() {
        // The three "clipboard is steady-state empty" variants must
        // all become Ok(None) — they're not errors, they're the
        // null result of a successful query.
        let r: Result<Option<RawClip>> =
            WaylandLibraryCapture::translate_paste_err::<RawClip>(PasteError::NoSeats);
        assert!(matches!(r, Ok(None)));

        let r: Result<Option<RawClip>> =
            WaylandLibraryCapture::translate_paste_err::<RawClip>(PasteError::ClipboardEmpty);
        assert!(matches!(r, Ok(None)));

        let r: Result<Option<RawClip>> =
            WaylandLibraryCapture::translate_paste_err::<RawClip>(PasteError::NoMimeType);
        assert!(matches!(r, Ok(None)));
    }

    #[test]
    fn translate_paste_err_propagates_real_errors() {
        // `MissingProtocol` is a permanent compositor-level failure —
        // user is on a compositor without `wlr-data-control`. Must
        // propagate so the operator sees it (e.g. KDE without the
        // protocol enabled).
        let r: Result<Option<RawClip>> =
            WaylandLibraryCapture::translate_paste_err::<RawClip>(PasteError::MissingProtocol {
                name: "zwlr_data_control_manager_v1",
                version: 1,
            });
        assert!(matches!(r, Err(DitoxError::Clipboard(_))));
    }

    #[test]
    fn subscribe_returns_empty_channel() {
        // v0.4: subscribe() is a stub. The receiver should be valid
        // (drop-safe) but no events are ever sent to it.
        let mut cap = WaylandLibraryCapture::new(CaptureConfig::default());
        let rx = cap.subscribe().unwrap();
        // Try-recv must return Empty (channel open, no senders alive
        // because `_tx` was dropped at the end of subscribe()).
        match rx.try_recv() {
            Err(mpsc::TryRecvError::Disconnected) => {} // expected
            other => panic!("expected Disconnected (sender dropped), got {:?}", other),
        }
    }

    #[test]
    fn config_is_held_by_value_for_should_capture_check() {
        // A custom-mode config with no allowlist means *no* format
        // should be captured. We can verify this without touching the
        // real Wayland connection by inspecting the helper directly.
        let config = CaptureConfig {
            mode: CaptureMode::Custom,
            formats: Default::default(), // include = exclude = []
            ..CaptureConfig::default()
        };
        let cap = WaylandLibraryCapture::new(config);
        assert!(!cap.config.should_capture_format("text/plain;charset=utf-8"));
        assert!(!cap.config.should_capture_format("image/png"));
    }

    /// End-to-end smoke test against the real compositor. Skipped
    /// when `WAYLAND_DISPLAY` is not set (CI / non-graphical
    /// environments) so the test suite can run anywhere.
    ///
    /// To exercise: copy something to the clipboard with `wl-copy
    /// 'hello'` (or any GUI app), then run:
    ///
    /// ```text
    /// cargo test -p ditox-core --lib --features=__live_wayland \
    ///     wayland::tests::live_snapshot_returns_text -- --ignored
    /// ```
    ///
    /// Currently `#[ignore]` so it never runs unless explicitly
    /// requested — there's no way to know from inside the test
    /// process whether the user has actually copied something.
    #[test]
    #[ignore = "requires a real Wayland session and pre-populated clipboard"]
    fn live_snapshot_returns_some_clip_when_clipboard_nonempty() {
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            eprintln!("WAYLAND_DISPLAY not set; skipping");
            return;
        }
        let cap = WaylandLibraryCapture::new(CaptureConfig::default());
        let snap = cap
            .current_snapshot()
            .expect("snapshot must not error on a healthy compositor");
        match snap {
            Some(clip) => {
                assert!(
                    !clip.formats.is_empty(),
                    "non-None snapshot must have at least one format"
                );
                for f in &clip.formats {
                    assert!(
                        !f.bytes.is_empty(),
                        "empty payload should have been filtered"
                    );
                    // Canonical MIME forms — never `text/plain` (no
                    // charset), never `UTF8_STRING`.
                    assert_ne!(f.mime, "text/plain");
                    assert_ne!(f.mime, "UTF8_STRING");
                }
            }
            None => eprintln!("clipboard is empty; nothing to verify"),
        }
    }
}
