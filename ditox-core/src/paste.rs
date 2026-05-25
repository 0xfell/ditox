//! Paste-back primitives.
//!
//! Phase 2 of the v0.4 → v1.0 master plan: when the user picks a
//! clip in the TUI, we want to (a) write it to the clipboard,
//! (b) restore focus to the previously-active
//! window, and (d) synthesise the appropriate paste keystroke.
//!
//! Sub-modules:
//!
//! - [`keystroke`] — parses the per-app override string from
//!   `[paste.keystrokes]` config into a typed
//!   [`keystroke::KeystrokeSequence`]. Pure, cross-platform, no IO.
//! - [`synthesize`] — sub-task 2.4. The
//!   `Hyprctl`/`Wtype`/`Ydotool`/`Win32Synthesizer`/`OffSynthesizer`
//!   chain that consumes a `KeystrokeSequence` and pokes the OS.
//! - [`sentinel`] — sub-task 2.7. Cross-process "do not re-capture
//!   this clip" sentinel (Linux: filesystem record at
//!   `<data_dir>/last-paste.json`; Windows future: `Clipboard
//!   Viewer Ignore` registered format).
//! - [`cursor`] — sub-task 2.9 groundwork. Persistent selection
//!   cursor that advances on rapid re-fires.

pub mod cursor;
pub mod keystroke;
pub mod sentinel;
pub mod synthesize;
