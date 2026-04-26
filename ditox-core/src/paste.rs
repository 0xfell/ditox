//! Paste-back primitives.
//!
//! Phase 2 of the v0.4 → v1.0 master plan: when the user picks a
//! clip in the launcher, we want to (a) write it to the clipboard,
//! (b) hide the launcher, (c) restore focus to the previously-active
//! window, and (d) synthesise the appropriate paste keystroke.
//!
//! Sub-modules:
//!
//! - [`keystroke`] — parses the per-app override string from
//!   `[paste.keystrokes]` config into a typed
//!   [`keystroke::KeystrokeSequence`]. Pure, cross-platform, no IO.
//! - (future) `synthesize` — sub-task 2.4. The
//!   `Hyprctl`/`Wtype`/`Ydotool`/`Win32Synthesizer`/`OffSynthesizer`
//!   chain that consumes a `KeystrokeSequence` and pokes the OS.
//! - (future) `sentinel` — sub-task 2.7. Cross-platform "do not
//!   re-capture this clip" markers (Linux: custom MIME; Windows:
//!   `Clipboard Viewer Ignore`).

pub mod keystroke;
pub mod sentinel;
pub mod synthesize;
