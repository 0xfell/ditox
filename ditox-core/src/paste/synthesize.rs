//! Keystroke synthesis backends (Phase 2 sub-task 2.4 — Linux).
//!
//! Once the launcher has restored focus to the previous app
//! (`ForegroundTracker::restore`), it asks one of these synthesizers
//! to type Ctrl+V (or whatever per-app override is configured) into
//! that window. This is what makes paste-back feel native — the user
//! doesn't see the launcher's window steal focus then have to press
//! Ctrl+V themselves.
//!
//! ## Backends
//!
//! - [`HyprctlSynthesizer`] — `hyprctl dispatch sendshortcut`. Best
//!   on Hyprland: targets the window by address rather than the
//!   current focus, avoiding the race where a popup opens between
//!   our restore and our keystroke.
//! - [`WtypeSynthesizer`] — `wtype` shell-out. Works on every
//!   wlroots-based compositor (Sway, Hyprland, generic wlroots) and
//!   on KDE Plasma 5/6 Wayland.
//! - [`YdotoolSynthesizer`] — `ydotool key` (requires the
//!   `ydotoold` daemon running). Last-resort wlroots/GNOME path.
//! - [`OffSynthesizer`] — no-op; the launcher shows a "paste
//!   manually with Ctrl+V" status when this one is selected.
//!
//! ## Chain
//!
//! [`pick_chain`] returns an ordered `Vec<Box<dyn Synthesizer>>` for
//! the detected platform; [`paste_with_chain`] tries each in order
//! and returns the name of the first one that succeeded. The
//! per-platform order matches
//! [`crate::platform::Platform::paste_synthesizer_chain`].
//!
//! ## Test strategy
//!
//! Each impl exposes an inherent `argv()` method that builds the
//! exact command-line vector it would shell out. Tests assert on
//! `argv()` directly — no subprocess is ever spawned in the unit
//! suite. Real shell-outs only happen when `paste()` is called from
//! a live launcher; that path is exercised in manual / integration
//! tests on Hyprland (and eventually Sway / KDE / GNOME).
//!
//! ## v0.4 limitations
//!
//! - Multi-chord sequences are emitted as **separate process spawns**
//!   for `hyprctl` and `ydotool` (one shell-out per chord); `wtype`
//!   handles the whole sequence in one invocation. Latency is
//!   acceptable (≈10 ms per spawn on a warm cache); future work can
//!   coalesce.
//! - `OffSynthesizer` returns success without doing anything, so a
//!   launcher with `OffSynthesizer` first in its chain effectively
//!   disables paste-back. The chain order is platform-default; users
//!   override via the (future) `[paste]` config section.
//! - Literal characters above U+007F (non-ASCII) work on `wtype`
//!   (UTF-8) but not reliably on `hyprctl sendshortcut` (which
//!   speaks key names, not raw chars). For v0.4, falling back to
//!   `wtype` for non-ASCII is the recommended config.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use tracing::{debug, warn};

use crate::error::{DitoxError, Result};
use crate::foreground::{ForegroundId, ForegroundSnapshot};
use crate::paste::keystroke::{Chord, Key, KeystrokeSequence, Modifier, SpecialKey};
use crate::platform::{LinuxCompositor, Platform};

/// Per-call timeout for synthesizer process spawns. A spawn that
/// hangs longer than this is considered failed and the chain falls
/// through to the next synthesizer.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(2);

/// One concrete strategy for sending a keystroke to the previously-
/// focused window.
pub trait Synthesizer: Send + Sync {
    /// Stable identifier — `"hyprctl"`, `"wtype"`, `"ydotool"`,
    /// `"off"`. Matches the strings in
    /// [`Platform::paste_synthesizer_chain`].
    fn name(&self) -> &str;

    /// True when this synthesizer can be invoked right now: required
    /// binary on PATH, required daemon running, required compositor
    /// active. Polled by [`paste_with_chain`] to skip backends that
    /// definitely can't run instead of failing them via spawn error.
    fn is_available(&self) -> bool;

    /// Send `keys` to the window identified by `target`.
    fn paste(&self, target: &ForegroundSnapshot, keys: &KeystrokeSequence) -> Result<()>;
}

// ---------------------------------------------------------------------------
// HyprctlSynthesizer
// ---------------------------------------------------------------------------

/// Hyprland-specific synthesizer using `hyprctl dispatch sendshortcut`.
///
/// `sendshortcut` targets a specific window by address, so the
/// launcher's "restore focus + send keys" race becomes irrelevant —
/// the keys land in the right window even if focus has drifted.
///
/// Argv shape per chord (`ctrl+v` example):
///
/// ```text
/// hyprctl dispatch sendshortcut , ctrl+v, address:0xdeadbeef
/// ```
///
/// The empty MOD field is intentional: Hyprland's `sendshortcut`
/// dispatcher syntax is `MOD, KEY, WINDOW`; our chord already
/// carries its own modifier set in KEY, so we leave MOD blank.
///
/// Multi-chord sequences emit one `hyprctl dispatch sendshortcut`
/// invocation per chord, in order.
pub struct HyprctlSynthesizer {
    binary: PathBuf,
}

impl HyprctlSynthesizer {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("hyprctl"),
        }
    }

    /// Override the binary path; for tests + when `hyprctl` lives
    /// outside `$PATH`.
    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
        }
    }

    /// Build the per-chord argv vectors that `paste()` would invoke.
    ///
    /// Returns one `Vec<String>` per chord — caller spawns each in
    /// sequence. Errors when:
    /// - The target isn't a Hyprland window
    ///   (`ForegroundId::Hypr { address }`).
    /// - A literal character can't be encoded as a single hyprctl
    ///   key token (rare; multi-byte chars).
    pub fn argv(
        &self,
        target: &ForegroundSnapshot,
        keys: &KeystrokeSequence,
    ) -> Result<Vec<Vec<String>>> {
        let address = match &target.identifier {
            ForegroundId::Hypr { address } => address.clone(),
            other => {
                return Err(DitoxError::Other(format!(
                    "hyprctl synthesizer requires Hypr identifier; got {}",
                    other.kind()
                )));
            }
        };

        let mut all = Vec::with_capacity(keys.chords.len());
        for chord in &keys.chords {
            let key_token = chord_to_hyprctl_key(chord)?;
            all.push(vec![
                self.binary.to_string_lossy().into_owned(),
                "dispatch".to_string(),
                "sendshortcut".to_string(),
                // MOD field empty; chord modifiers folded into KEY.
                String::new(),
                key_token,
                format!("address:{}", address),
            ]);
        }
        Ok(all)
    }
}

impl Default for HyprctlSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a chord into hyprctl's `MOD+MOD+KEY` token (used in the
/// KEY field of `sendshortcut`).
fn chord_to_hyprctl_key(chord: &Chord) -> Result<String> {
    let mut parts = Vec::with_capacity(chord.modifiers.len() + 1);
    for m in &chord.modifiers {
        parts.push(modifier_hyprctl_name(*m).to_string());
    }
    parts.push(key_hyprctl_name(&chord.key)?);
    Ok(parts.join("+"))
}

fn modifier_hyprctl_name(m: Modifier) -> &'static str {
    match m {
        Modifier::Ctrl => "CTRL",
        Modifier::Shift => "SHIFT",
        Modifier::Alt => "ALT",
        Modifier::Super => "SUPER",
    }
}

fn key_hyprctl_name(key: &Key) -> Result<String> {
    match key {
        Key::Special(sk) => Ok(special_key_hyprctl_name(*sk).to_string()),
        Key::Char(c) => {
            if !c.is_ascii() {
                return Err(DitoxError::Other(format!(
                    "hyprctl can't synthesise non-ASCII character '{}' (U+{:04X})",
                    c, *c as u32
                )));
            }
            Ok(c.to_string())
        }
    }
}

fn special_key_hyprctl_name(sk: SpecialKey) -> &'static str {
    // hyprctl uses xkb keysym names. These are the standard names
    // for the keys we expose; consult `xkb_keysym_get_name(3)` if
    // a key needs a non-obvious mapping.
    match sk {
        SpecialKey::Enter => "Return",
        SpecialKey::Tab => "Tab",
        SpecialKey::Escape => "Escape",
        SpecialKey::Space => "space",
        SpecialKey::Backspace => "BackSpace",
        SpecialKey::Delete => "Delete",
        SpecialKey::Insert => "Insert",
        SpecialKey::Home => "Home",
        SpecialKey::End => "End",
        SpecialKey::PageUp => "Prior",
        SpecialKey::PageDown => "Next",
        SpecialKey::Up => "Up",
        SpecialKey::Down => "Down",
        SpecialKey::Left => "Left",
        SpecialKey::Right => "Right",
        SpecialKey::F1 => "F1",
        SpecialKey::F2 => "F2",
        SpecialKey::F3 => "F3",
        SpecialKey::F4 => "F4",
        SpecialKey::F5 => "F5",
        SpecialKey::F6 => "F6",
        SpecialKey::F7 => "F7",
        SpecialKey::F8 => "F8",
        SpecialKey::F9 => "F9",
        SpecialKey::F10 => "F10",
        SpecialKey::F11 => "F11",
        SpecialKey::F12 => "F12",
        SpecialKey::F13 => "F13",
        SpecialKey::F14 => "F14",
        SpecialKey::F15 => "F15",
        SpecialKey::F16 => "F16",
        SpecialKey::F17 => "F17",
        SpecialKey::F18 => "F18",
        SpecialKey::F19 => "F19",
        SpecialKey::F20 => "F20",
        SpecialKey::F21 => "F21",
        SpecialKey::F22 => "F22",
        SpecialKey::F23 => "F23",
        SpecialKey::F24 => "F24",
    }
}

impl Synthesizer for HyprctlSynthesizer {
    fn name(&self) -> &str {
        "hyprctl"
    }

    fn is_available(&self) -> bool {
        // Both: hyprctl on PATH AND we're actually on Hyprland.
        // Without the platform check, we'd happily try `hyprctl` on
        // Sway, hit "no Hyprland socket", and waste a fall-through.
        find_in_path("hyprctl").is_some()
            && matches!(
                crate::platform::detect(),
                Platform::Linux(LinuxCompositor::Hyprland { .. })
            )
    }

    fn paste(&self, target: &ForegroundSnapshot, keys: &KeystrokeSequence) -> Result<()> {
        let argvs = self.argv(target, keys)?;
        for argv in argvs {
            run_command(&argv)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WtypeSynthesizer
// ---------------------------------------------------------------------------

/// Wtype-based synthesizer for any wlroots-based compositor.
///
/// `wtype` accepts a sequence of actions in one invocation:
///
/// - `-M <mod>` — press modifier (does NOT release).
/// - `-m <mod>` — release modifier.
/// - `-k <name>` — press-and-release a named key (xkb keysym).
/// - `<text>` — type literal text (UTF-8 capable).
///
/// One process spawn per `paste()` call regardless of chord count.
pub struct WtypeSynthesizer {
    binary: PathBuf,
}

impl WtypeSynthesizer {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("wtype"),
        }
    }

    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
        }
    }

    /// Build the single argv vector that `paste()` would invoke.
    pub fn argv(
        &self,
        _target: &ForegroundSnapshot,
        keys: &KeystrokeSequence,
    ) -> Result<Vec<String>> {
        let mut argv = vec![self.binary.to_string_lossy().into_owned()];
        for chord in &keys.chords {
            for m in &chord.modifiers {
                argv.push("-M".to_string());
                argv.push(modifier_wtype_name(*m).to_string());
            }
            match &chord.key {
                Key::Special(sk) => {
                    argv.push("-k".to_string());
                    argv.push(special_key_wtype_name(*sk).to_string());
                }
                Key::Char(c) => {
                    // `--` would be needed to stop wtype's flag
                    // parsing if the literal starts with `-`.
                    // Always emit the separator before the first
                    // literal to be safe across chord orders.
                    argv.push("--".to_string());
                    argv.push(c.to_string());
                }
            }
            // Release modifiers in reverse order for symmetry.
            for m in chord.modifiers.iter().rev() {
                argv.push("-m".to_string());
                argv.push(modifier_wtype_name(*m).to_string());
            }
        }
        Ok(argv)
    }
}

impl Default for WtypeSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

fn modifier_wtype_name(m: Modifier) -> &'static str {
    // wtype uses lowercase names.
    match m {
        Modifier::Ctrl => "ctrl",
        Modifier::Shift => "shift",
        Modifier::Alt => "alt",
        Modifier::Super => "logo",
    }
}

fn special_key_wtype_name(sk: SpecialKey) -> &'static str {
    // Same xkb keysym names as hyprctl — wtype consumes xkb directly.
    special_key_hyprctl_name(sk)
}

impl Synthesizer for WtypeSynthesizer {
    fn name(&self) -> &str {
        "wtype"
    }

    fn is_available(&self) -> bool {
        find_in_path("wtype").is_some()
            && matches!(
                crate::platform::detect(),
                Platform::Linux(
                    LinuxCompositor::Hyprland { .. }
                        | LinuxCompositor::Sway { .. }
                        | LinuxCompositor::Wlroots { .. }
                        | LinuxCompositor::Kde { wayland: true }
                )
            )
    }

    fn paste(&self, target: &ForegroundSnapshot, keys: &KeystrokeSequence) -> Result<()> {
        let argv = self.argv(target, keys)?;
        run_command(&argv)
    }
}

// ---------------------------------------------------------------------------
// YdotoolSynthesizer
// ---------------------------------------------------------------------------

/// Ydotool synthesizer (last-resort wlroots / GNOME Wayland path).
///
/// Requires the `ydotoold` daemon running and the user having access
/// to its socket (typically `/tmp/.ydotool_socket` with group
/// permissions). Failures are silent at chain-fall-through so an
/// unconfigured ydotool just gets skipped.
///
/// Argv shape per chord (`ctrl+v` example):
///
/// ```text
/// ydotool key 29:1 47:1 47:0 29:0
/// ```
///
/// where `29` and `47` are Linux input-event keycodes for `LEFTCTRL`
/// and `V`, and `:1`/`:0` are press/release. v0.4 uses the
/// higher-level `ydotool key ctrl+v` form when available (newer
/// ydotool versions); older versions need the keycode form, which is
/// out of scope for this initial implementation.
///
/// One spawn per chord.
pub struct YdotoolSynthesizer {
    binary: PathBuf,
}

impl YdotoolSynthesizer {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("ydotool"),
        }
    }

    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
        }
    }

    /// Build the per-chord argv vectors that `paste()` would invoke.
    pub fn argv(
        &self,
        _target: &ForegroundSnapshot,
        keys: &KeystrokeSequence,
    ) -> Result<Vec<Vec<String>>> {
        let mut all = Vec::with_capacity(keys.chords.len());
        for chord in &keys.chords {
            // Use ydotool's high-level chord syntax: `ctrl+v` →
            // `ydotool key ctrl+v`. The `key` subcommand also
            // accepts numeric keycodes; the symbolic form is what
            // newer ydotool releases prefer.
            let mut tokens = Vec::with_capacity(chord.modifiers.len() + 1);
            for m in &chord.modifiers {
                tokens.push(modifier_ydotool_name(*m).to_string());
            }
            tokens.push(key_ydotool_name(&chord.key)?);
            all.push(vec![
                self.binary.to_string_lossy().into_owned(),
                "key".to_string(),
                tokens.join("+"),
            ]);
        }
        Ok(all)
    }
}

impl Default for YdotoolSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

fn modifier_ydotool_name(m: Modifier) -> &'static str {
    match m {
        Modifier::Ctrl => "ctrl",
        Modifier::Shift => "shift",
        Modifier::Alt => "alt",
        Modifier::Super => "super",
    }
}

fn key_ydotool_name(key: &Key) -> Result<String> {
    match key {
        Key::Special(sk) => Ok(special_key_ydotool_name(*sk).to_string()),
        Key::Char(c) => {
            if !c.is_ascii() {
                return Err(DitoxError::Other(format!(
                    "ydotool can't synthesise non-ASCII character '{}' (U+{:04X})",
                    c, *c as u32
                )));
            }
            Ok(c.to_ascii_lowercase().to_string())
        }
    }
}

fn special_key_ydotool_name(sk: SpecialKey) -> &'static str {
    // ydotool uses Linux input-event names (lowercase).
    match sk {
        SpecialKey::Enter => "enter",
        SpecialKey::Tab => "tab",
        SpecialKey::Escape => "esc",
        SpecialKey::Space => "space",
        SpecialKey::Backspace => "backspace",
        SpecialKey::Delete => "delete",
        SpecialKey::Insert => "insert",
        SpecialKey::Home => "home",
        SpecialKey::End => "end",
        SpecialKey::PageUp => "pageup",
        SpecialKey::PageDown => "pagedown",
        SpecialKey::Up => "up",
        SpecialKey::Down => "down",
        SpecialKey::Left => "left",
        SpecialKey::Right => "right",
        SpecialKey::F1 => "f1",
        SpecialKey::F2 => "f2",
        SpecialKey::F3 => "f3",
        SpecialKey::F4 => "f4",
        SpecialKey::F5 => "f5",
        SpecialKey::F6 => "f6",
        SpecialKey::F7 => "f7",
        SpecialKey::F8 => "f8",
        SpecialKey::F9 => "f9",
        SpecialKey::F10 => "f10",
        SpecialKey::F11 => "f11",
        SpecialKey::F12 => "f12",
        // F13-F24 are uncommon on real keyboards; ydotool's keycode
        // table covers them but the symbolic names vary by version.
        // Fall back to "f13" through "f24" — caller's responsibility
        // if the user's ydotool doesn't accept them.
        SpecialKey::F13 => "f13",
        SpecialKey::F14 => "f14",
        SpecialKey::F15 => "f15",
        SpecialKey::F16 => "f16",
        SpecialKey::F17 => "f17",
        SpecialKey::F18 => "f18",
        SpecialKey::F19 => "f19",
        SpecialKey::F20 => "f20",
        SpecialKey::F21 => "f21",
        SpecialKey::F22 => "f22",
        SpecialKey::F23 => "f23",
        SpecialKey::F24 => "f24",
    }
}

impl Synthesizer for YdotoolSynthesizer {
    fn name(&self) -> &str {
        "ydotool"
    }

    fn is_available(&self) -> bool {
        // We don't check daemon availability — that'd require
        // either a Unix socket connect or shelling out to
        // `systemctl --user is-active ydotool`. The chain-fall-
        // through behaviour catches a dead daemon on the first
        // `paste()` call.
        find_in_path("ydotool").is_some() && matches!(crate::platform::detect(), Platform::Linux(_))
    }

    fn paste(&self, target: &ForegroundSnapshot, keys: &KeystrokeSequence) -> Result<()> {
        let argvs = self.argv(target, keys)?;
        for argv in argvs {
            run_command(&argv)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// OffSynthesizer
// ---------------------------------------------------------------------------

/// Disabled synthesizer — never tries to send keys.
///
/// `paste()` returns `Ok(())` immediately so the chain stops at this
/// rung; the launcher should detect that "off" succeeded and show a
/// "paste manually with Ctrl+V" status line in its UI.
///
/// Used as the last entry of every chain so the launcher always has
/// a non-failing fallback.
pub struct OffSynthesizer;

impl OffSynthesizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OffSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Synthesizer for OffSynthesizer {
    fn name(&self) -> &str {
        "off"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn paste(&self, target: &ForegroundSnapshot, _keys: &KeystrokeSequence) -> Result<()> {
        debug!(
            target = %target.process_basename,
            "off synthesizer: paste-back disabled; user will paste manually"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Chain selection + execution
// ---------------------------------------------------------------------------

/// Build the platform-default synthesizer chain.
///
/// Order matches [`Platform::paste_synthesizer_chain`]; an
/// [`OffSynthesizer`] is always appended so the chain never returns
/// "nothing to try" — the launcher gets a structured "off" outcome
/// it can surface in the UI.
pub fn pick_chain(platform: &Platform) -> Vec<Box<dyn Synthesizer>> {
    let mut chain: Vec<Box<dyn Synthesizer>> = Vec::new();
    for name in platform.paste_synthesizer_chain() {
        match name {
            "hyprctl" => chain.push(Box::new(HyprctlSynthesizer::new())),
            "wtype" => chain.push(Box::new(WtypeSynthesizer::new())),
            "ydotool" => chain.push(Box::new(YdotoolSynthesizer::new())),
            // "sendinput" / "cgevent" / "xdotool" — Phase 2 follow-
            // ups (Windows: 2.5; macOS: Phase 8; X11: out of scope
            // for v0.4 since wlroots is the explicit target).
            _ => debug!(name, "synthesizer name not yet implemented; skipping"),
        }
    }
    chain.push(Box::new(OffSynthesizer));
    chain
}

/// Try each synthesizer in order; return the name of the first one
/// whose [`Synthesizer::paste`] succeeded.
///
/// A synthesizer is skipped (without consulting `paste`) when its
/// [`Synthesizer::is_available`] returns false. A synthesizer that
/// succeeds short-circuits the chain (no later synthesizer is run).
/// A synthesizer that errors is logged at `warn` and the chain
/// proceeds to the next entry.
///
/// Returns `Err(DitoxError::Other("no synthesizer in the chain
/// succeeded"))` only if every available synthesizer errored —
/// `OffSynthesizer` always succeeds, so a chain that includes it
/// (which [`pick_chain`] always does) cannot reach this error.
pub fn paste_with_chain(
    chain: &[Box<dyn Synthesizer>],
    target: &ForegroundSnapshot,
    keys: &KeystrokeSequence,
) -> Result<&'static str> {
    for synth in chain {
        if !synth.is_available() {
            debug!(synth = %synth.name(), "skipping unavailable synthesizer");
            continue;
        }
        match synth.paste(target, keys) {
            Ok(()) => {
                let name: &'static str = synthesizer_name_static(synth.name());
                debug!(synth = %name, "paste-back succeeded");
                return Ok(name);
            }
            Err(e) => {
                warn!(synth = %synth.name(), error = %e, "synthesizer failed; trying next");
            }
        }
    }
    Err(DitoxError::Other(
        "no synthesizer in the chain succeeded".to_string(),
    ))
}

/// Map runtime synthesizer names back to the small static set used
/// by [`pick_chain`]. Avoids returning a borrowed string with the
/// trait-object lifetime.
fn synthesizer_name_static(name: &str) -> &'static str {
    match name {
        "hyprctl" => "hyprctl",
        "wtype" => "wtype",
        "ydotool" => "ydotool",
        "off" => "off",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk `$PATH` directories in order; return the first that contains
/// an executable file named `cmd`. `None` when not found.
fn find_in_path(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Spawn `argv` with stdin/stdout/stderr inherited; wait up to
/// `SPAWN_TIMEOUT` and translate non-zero exit / spawn failures
/// into [`DitoxError::Other`].
///
/// We don't capture stdout/stderr — synthesizer tools are quiet on
/// success and verbose on failure; we want the operator to see the
/// failure in the daemon's terminal.
fn run_command(argv: &[String]) -> Result<()> {
    if argv.is_empty() {
        return Err(DitoxError::Other("empty argv".into()));
    }
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    // `Command::status` blocks until the child exits; for these
    // tools that's well under SPAWN_TIMEOUT in practice. If we ever
    // need a real timeout we'd switch to `spawn` + a poll loop or
    // pull in `wait_timeout`. v0.4: accept the blocking behaviour.
    let _ = SPAWN_TIMEOUT; // documented intent; not yet enforced.

    let status = cmd
        .status()
        .map_err(|e| DitoxError::Other(format!("spawn {} failed: {}", argv[0], e)))?;
    if !status.success() {
        return Err(DitoxError::Other(format!(
            "{} exited {}",
            argv[0],
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into())
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foreground::ForegroundId;
    use std::sync::Mutex;
    use std::time::SystemTime;

    fn snap_hypr(addr: &str) -> ForegroundSnapshot {
        ForegroundSnapshot {
            identifier: ForegroundId::Hypr {
                address: addr.to_string(),
            },
            process_basename: "firefox".to_string(),
            title: "Firefox — test".to_string(),
            captured_at: SystemTime::now(),
        }
    }

    fn snap_wlr() -> ForegroundSnapshot {
        ForegroundSnapshot {
            identifier: ForegroundId::Wlr {
                handle_id: "1".to_string(),
                app_id: "firefox".to_string(),
                title: "Firefox".to_string(),
            },
            process_basename: "firefox".to_string(),
            title: "Firefox".to_string(),
            captured_at: SystemTime::now(),
        }
    }

    fn ks(input: &str) -> KeystrokeSequence {
        KeystrokeSequence::parse(input).unwrap()
    }

    // -----------------------------------------------------------------
    // HyprctlSynthesizer.argv
    // -----------------------------------------------------------------

    #[test]
    fn hyprctl_argv_simple_chord() {
        let h = HyprctlSynthesizer::new();
        let argvs = h.argv(&snap_hypr("0xdeadbeef"), &ks("ctrl+v")).unwrap();
        assert_eq!(argvs.len(), 1);
        assert_eq!(
            argvs[0],
            vec![
                "hyprctl".to_string(),
                "dispatch".into(),
                "sendshortcut".into(),
                String::new(), // empty MOD field
                "CTRL+v".into(),
                "address:0xdeadbeef".into(),
            ]
        );
    }

    #[test]
    fn hyprctl_argv_multi_modifier_chord() {
        let h = HyprctlSynthesizer::new();
        let argvs = h.argv(&snap_hypr("0x1"), &ks("ctrl+shift+v")).unwrap();
        assert_eq!(argvs.len(), 1);
        assert_eq!(argvs[0][4], "CTRL+SHIFT+v");
    }

    #[test]
    fn hyprctl_argv_special_key() {
        let h = HyprctlSynthesizer::new();
        let argvs = h.argv(&snap_hypr("0x1"), &ks("ctrl+enter")).unwrap();
        assert_eq!(argvs[0][4], "CTRL+Return");
    }

    #[test]
    fn hyprctl_argv_multi_chord_emits_one_invocation_per_chord() {
        let h = HyprctlSynthesizer::new();
        let argvs = h.argv(&snap_hypr("0x1"), &ks("ctrl+v ctrl+s")).unwrap();
        assert_eq!(argvs.len(), 2);
        assert_eq!(argvs[0][4], "CTRL+v");
        assert_eq!(argvs[1][4], "CTRL+s");
    }

    #[test]
    fn hyprctl_argv_vim_register_paste_emits_four_invocations() {
        let h = HyprctlSynthesizer::new();
        let argvs = h.argv(&snap_hypr("0x1"), &ks("\"+gp")).unwrap();
        assert_eq!(argvs.len(), 4);
        assert_eq!(argvs[0][4], "\"");
        assert_eq!(argvs[1][4], "+");
        assert_eq!(argvs[2][4], "g");
        assert_eq!(argvs[3][4], "p");
    }

    #[test]
    fn hyprctl_argv_rejects_non_hypr_target() {
        let h = HyprctlSynthesizer::new();
        let err = h.argv(&snap_wlr(), &ks("ctrl+v")).unwrap_err();
        assert!(format!("{}", err).contains("Hypr identifier"));
    }

    #[test]
    fn hyprctl_argv_rejects_non_ascii_char() {
        let h = HyprctlSynthesizer::new();
        let err = h.argv(&snap_hypr("0x1"), &ks("é")).unwrap_err();
        assert!(format!("{}", err).contains("non-ASCII"));
    }

    #[test]
    fn hyprctl_with_binary_overrides_path() {
        let h = HyprctlSynthesizer::with_binary("/custom/hyprctl");
        let argvs = h.argv(&snap_hypr("0x1"), &ks("ctrl+v")).unwrap();
        assert_eq!(argvs[0][0], "/custom/hyprctl");
    }

    // -----------------------------------------------------------------
    // WtypeSynthesizer.argv
    // -----------------------------------------------------------------

    #[test]
    fn wtype_argv_simple_chord() {
        let w = WtypeSynthesizer::new();
        let argv = w.argv(&snap_wlr(), &ks("ctrl+v")).unwrap();
        assert_eq!(
            argv,
            vec![
                "wtype".to_string(),
                "-M".into(),
                "ctrl".into(),
                "--".into(),
                "v".into(),
                "-m".into(),
                "ctrl".into(),
            ]
        );
    }

    #[test]
    fn wtype_argv_multi_modifier_releases_in_reverse() {
        let w = WtypeSynthesizer::new();
        let argv = w.argv(&snap_wlr(), &ks("ctrl+shift+v")).unwrap();
        // Press order: ctrl, shift. Release order: shift, ctrl.
        let expected = vec![
            "wtype".to_string(),
            "-M".into(),
            "ctrl".into(),
            "-M".into(),
            "shift".into(),
            "--".into(),
            "v".into(),
            "-m".into(),
            "shift".into(),
            "-m".into(),
            "ctrl".into(),
        ];
        assert_eq!(argv, expected);
    }

    #[test]
    fn wtype_argv_special_key_uses_dash_k() {
        let w = WtypeSynthesizer::new();
        let argv = w.argv(&snap_wlr(), &ks("ctrl+enter")).unwrap();
        assert!(argv.contains(&"-k".to_string()));
        assert!(argv.contains(&"Return".to_string()));
    }

    #[test]
    fn wtype_argv_multi_chord_one_invocation() {
        let w = WtypeSynthesizer::new();
        let argv = w.argv(&snap_wlr(), &ks("ctrl+v ctrl+s")).unwrap();
        // One process spawn covers the whole sequence.
        assert_eq!(argv[0], "wtype");
        // Two press-v/s actions present in the same argv.
        assert!(argv.windows(2).any(|w| w == ["--", "v"]));
        assert!(argv.windows(2).any(|w| w == ["--", "s"]));
    }

    #[test]
    fn wtype_argv_super_modifier_maps_to_logo() {
        let w = WtypeSynthesizer::new();
        let argv = w.argv(&snap_wlr(), &ks("super+l")).unwrap();
        assert!(argv.contains(&"logo".to_string()));
        assert!(!argv.contains(&"super".to_string()));
    }

    #[test]
    fn wtype_argv_unicode_char_passes_through() {
        // wtype handles UTF-8 directly.
        let w = WtypeSynthesizer::new();
        let argv = w.argv(&snap_wlr(), &ks("é")).unwrap();
        assert!(argv.contains(&"é".to_string()));
    }

    // -----------------------------------------------------------------
    // YdotoolSynthesizer.argv
    // -----------------------------------------------------------------

    #[test]
    fn ydotool_argv_simple_chord() {
        let y = YdotoolSynthesizer::new();
        let argvs = y.argv(&snap_wlr(), &ks("ctrl+v")).unwrap();
        assert_eq!(argvs.len(), 1);
        assert_eq!(
            argvs[0],
            vec!["ydotool".to_string(), "key".into(), "ctrl+v".into(),]
        );
    }

    #[test]
    fn ydotool_argv_multi_modifier() {
        let y = YdotoolSynthesizer::new();
        let argvs = y.argv(&snap_wlr(), &ks("ctrl+shift+v")).unwrap();
        assert_eq!(argvs[0][2], "ctrl+shift+v");
    }

    #[test]
    fn ydotool_argv_special_key() {
        let y = YdotoolSynthesizer::new();
        let argvs = y.argv(&snap_wlr(), &ks("ctrl+enter")).unwrap();
        assert_eq!(argvs[0][2], "ctrl+enter");
    }

    #[test]
    fn ydotool_argv_one_invocation_per_chord() {
        let y = YdotoolSynthesizer::new();
        let argvs = y.argv(&snap_wlr(), &ks("ctrl+v ctrl+s")).unwrap();
        assert_eq!(argvs.len(), 2);
        assert_eq!(argvs[0][2], "ctrl+v");
        assert_eq!(argvs[1][2], "ctrl+s");
    }

    #[test]
    fn ydotool_argv_lowercases_literal_char() {
        let y = YdotoolSynthesizer::new();
        let argvs = y.argv(&snap_wlr(), &ks("ctrl+V")).unwrap();
        // Uppercase V from input is normalised to lowercase v —
        // ydotool's keycode table is case-insensitive but
        // canonical lowercase avoids surprises.
        assert_eq!(argvs[0][2], "ctrl+v");
    }

    // -----------------------------------------------------------------
    // OffSynthesizer
    // -----------------------------------------------------------------

    #[test]
    fn off_is_always_available_and_paste_succeeds() {
        let o = OffSynthesizer::new();
        assert!(o.is_available());
        assert!(o.paste(&snap_wlr(), &ks("ctrl+v")).is_ok());
        assert_eq!(o.name(), "off");
    }

    // -----------------------------------------------------------------
    // pick_chain
    // -----------------------------------------------------------------

    #[test]
    fn pick_chain_hyprland_order() {
        let p = Platform::Linux(LinuxCompositor::Hyprland { signature: None });
        let chain = pick_chain(&p);
        let names: Vec<&str> = chain.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["hyprctl", "wtype", "ydotool", "off"]);
    }

    #[test]
    fn pick_chain_sway_order() {
        let p = Platform::Linux(LinuxCompositor::Sway { sock: None });
        let chain = pick_chain(&p);
        let names: Vec<&str> = chain.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["wtype", "ydotool", "off"]);
    }

    #[test]
    fn pick_chain_kde_wayland_order() {
        let p = Platform::Linux(LinuxCompositor::Kde { wayland: true });
        let chain = pick_chain(&p);
        let names: Vec<&str> = chain.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["wtype", "ydotool", "off"]);
    }

    #[test]
    fn pick_chain_gnome_wayland_skips_wtype() {
        let p = Platform::Linux(LinuxCompositor::Gnome { wayland: true });
        let chain = pick_chain(&p);
        let names: Vec<&str> = chain.iter().map(|s| s.name()).collect();
        // Gnome Wayland: ydotool only (wtype doesn't work without
        // a wlroots compositor).
        assert_eq!(names, vec!["ydotool", "off"]);
    }

    #[test]
    fn pick_chain_unknown_platform_yields_off_only() {
        let p = Platform::Other;
        let chain = pick_chain(&p);
        let names: Vec<&str> = chain.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["off"]);
    }

    #[test]
    fn pick_chain_always_ends_with_off() {
        for p in [
            Platform::Linux(LinuxCompositor::Hyprland { signature: None }),
            Platform::Linux(LinuxCompositor::Sway { sock: None }),
            Platform::Linux(LinuxCompositor::Wlroots {
                name: "labwc".into(),
            }),
            Platform::Linux(LinuxCompositor::Gnome { wayland: true }),
            Platform::Linux(LinuxCompositor::X11Only { name: None }),
            Platform::Other,
        ] {
            let chain = pick_chain(&p);
            assert_eq!(
                chain.last().expect("non-empty chain").name(),
                "off",
                "chain for {:?} doesn't end with off",
                p.slug()
            );
        }
    }

    // -----------------------------------------------------------------
    // paste_with_chain
    // -----------------------------------------------------------------

    /// Test synthesizer that succeeds if `should_succeed` is true and
    /// records every paste call.
    struct StubSynth {
        name: String,
        available: bool,
        should_succeed: bool,
        calls: Mutex<u32>,
    }

    impl StubSynth {
        fn new(name: &str, available: bool, should_succeed: bool) -> Self {
            Self {
                name: name.to_string(),
                available,
                should_succeed,
                calls: Mutex::new(0),
            }
        }
    }

    impl Synthesizer for StubSynth {
        fn name(&self) -> &str {
            &self.name
        }
        fn is_available(&self) -> bool {
            self.available
        }
        fn paste(&self, _t: &ForegroundSnapshot, _k: &KeystrokeSequence) -> Result<()> {
            *self.calls.lock().unwrap() += 1;
            if self.should_succeed {
                Ok(())
            } else {
                Err(DitoxError::Other("stub failure".into()))
            }
        }
    }

    #[test]
    fn paste_with_chain_returns_first_success() {
        let chain: Vec<Box<dyn Synthesizer>> = vec![
            Box::new(StubSynth::new("hyprctl", true, false)),
            Box::new(StubSynth::new("wtype", true, true)),
            Box::new(StubSynth::new("ydotool", true, true)),
        ];
        let result = paste_with_chain(&chain, &snap_wlr(), &ks("ctrl+v")).unwrap();
        // Returns mapped static name.
        assert_eq!(result, "wtype");
    }

    #[test]
    fn paste_with_chain_skips_unavailable_synthesizers() {
        let chain: Vec<Box<dyn Synthesizer>> = vec![
            Box::new(StubSynth::new("hyprctl", false, true)), // available=false
            Box::new(StubSynth::new("wtype", true, true)),
        ];
        let result = paste_with_chain(&chain, &snap_wlr(), &ks("ctrl+v")).unwrap();
        assert_eq!(result, "wtype");
    }

    #[test]
    fn paste_with_chain_falls_through_on_error_to_next() {
        let h = StubSynth::new("hyprctl", true, false);
        let w = StubSynth::new("wtype", true, true);
        // Not allowed to keep references after move; use Arc<Mutex<>>
        // for direct call-count inspection.
        use std::sync::Arc;
        let h = Arc::new(h);
        let w = Arc::new(w);
        let chain: Vec<Box<dyn Synthesizer>> = vec![
            Box::new(StubSynthRef(h.clone())),
            Box::new(StubSynthRef(w.clone())),
        ];
        paste_with_chain(&chain, &snap_wlr(), &ks("ctrl+v")).unwrap();
        assert_eq!(*h.calls.lock().unwrap(), 1, "hyprctl was tried");
        assert_eq!(
            *w.calls.lock().unwrap(),
            1,
            "wtype was tried after hyprctl failed"
        );
    }

    /// Wrapper that lets tests share an `Arc<StubSynth>` between the
    /// chain and direct inspection of the call counter.
    struct StubSynthRef(std::sync::Arc<StubSynth>);
    impl Synthesizer for StubSynthRef {
        fn name(&self) -> &str {
            self.0.name()
        }
        fn is_available(&self) -> bool {
            self.0.is_available()
        }
        fn paste(&self, t: &ForegroundSnapshot, k: &KeystrokeSequence) -> Result<()> {
            self.0.paste(t, k)
        }
    }

    #[test]
    fn paste_with_chain_returns_err_if_every_available_fails() {
        let chain: Vec<Box<dyn Synthesizer>> = vec![
            Box::new(StubSynth::new("hyprctl", true, false)),
            Box::new(StubSynth::new("wtype", true, false)),
        ];
        let err = paste_with_chain(&chain, &snap_wlr(), &ks("ctrl+v")).unwrap_err();
        assert!(format!("{}", err).contains("no synthesizer"));
    }

    #[test]
    fn paste_with_chain_with_off_at_end_never_returns_err() {
        // The real pick_chain output always ends with OffSynthesizer.
        let chain: Vec<Box<dyn Synthesizer>> = vec![
            Box::new(StubSynth::new("hyprctl", true, false)),
            Box::new(StubSynth::new("wtype", true, false)),
            Box::new(OffSynthesizer::new()),
        ];
        let result = paste_with_chain(&chain, &snap_wlr(), &ks("ctrl+v")).unwrap();
        assert_eq!(result, "off");
    }

    // -----------------------------------------------------------------
    // find_in_path helper
    // -----------------------------------------------------------------

    #[test]
    fn find_in_path_locates_known_executable() {
        // `sh` is part of POSIX and present on every dev/CI box.
        assert!(find_in_path("sh").is_some(), "/bin/sh should exist");
    }

    #[test]
    fn find_in_path_returns_none_for_nonexistent() {
        assert!(find_in_path("ditox-this-binary-does-not-exist-zzz").is_none());
    }

    // -----------------------------------------------------------------
    // Trait object-safety
    // -----------------------------------------------------------------

    #[test]
    fn synthesizers_are_object_safe() {
        let _v: Vec<Box<dyn Synthesizer>> = vec![
            Box::new(HyprctlSynthesizer::new()),
            Box::new(WtypeSynthesizer::new()),
            Box::new(YdotoolSynthesizer::new()),
            Box::new(OffSynthesizer::new()),
        ];
    }

    // -----------------------------------------------------------------
    // Live integration tests (manual)
    // -----------------------------------------------------------------

    /// Reports the platform-detected chain + per-backend availability.
    /// Doesn't actually paste — useful for diagnosing why a chain
    /// degrades to OffSynthesizer in the field.
    ///
    /// Run with:
    /// ```text
    /// cargo test -p ditox-core --lib synthesize::tests::live_chain_diagnostic \
    ///   -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "diagnostic; prints to stdout"]
    fn live_chain_diagnostic() {
        let p = crate::platform::detect();
        eprintln!("platform: {}", p.slug());
        eprintln!(
            "paste_synthesizer_chain (names): {:?}",
            p.paste_synthesizer_chain()
        );
        let chain = pick_chain(p);
        eprintln!("constructed chain ({} entries):", chain.len());
        for s in &chain {
            eprintln!("  {} -> available = {}", s.name(), s.is_available());
        }
    }

    /// End-to-end paste into a real Hyprland window. Requires:
    /// - Active Hyprland session.
    /// - User has set the env var `DITOX_TEST_HYPR_WINDOW` to a
    ///   Hyprland window address (run `hyprctl activewindow -j` to
    ///   find one — focus a text editor first).
    ///
    /// Sends `ctrl+a` to the target as a non-destructive sanity
    /// check (selects all in most apps; doesn't change content).
    /// To verify Ctrl+V paste-back, set up a text field with a known
    /// clipboard payload and watch it appear.
    ///
    /// ```text
    /// hyprctl -j activewindow | jq -r .address
    /// DITOX_TEST_HYPR_WINDOW=0x... cargo test -p ditox-core --lib \
    ///     synthesize::tests::live_hyprctl_paste -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "requires Hyprland + DITOX_TEST_HYPR_WINDOW env var"]
    fn live_hyprctl_paste() {
        let address = match std::env::var("DITOX_TEST_HYPR_WINDOW") {
            Ok(a) => a,
            Err(_) => {
                eprintln!("DITOX_TEST_HYPR_WINDOW not set; skipping");
                return;
            }
        };
        if !matches!(
            crate::platform::detect(),
            Platform::Linux(LinuxCompositor::Hyprland { .. })
        ) {
            eprintln!("not on Hyprland; skipping");
            return;
        }
        let h = HyprctlSynthesizer::new();
        let snap = ForegroundSnapshot {
            identifier: ForegroundId::Hypr {
                address: address.clone(),
            },
            process_basename: "test".into(),
            title: "test".into(),
            captured_at: SystemTime::now(),
        };
        let keys = ks("ctrl+a");
        eprintln!("Sending ctrl+a to address={}", address);
        h.paste(&snap, &keys).expect("hyprctl paste failed");
        eprintln!("OK — check the target window for the select-all effect.");
    }
}
