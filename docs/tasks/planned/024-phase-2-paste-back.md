# Task: Phase 2 — Paste-back UX (cross-platform)

> **Status:** planned
> **Priority:** high
> **Phase:** 2 — Paste-back
> **Created:** 2026-04-26
> **Estimated:** 3 weeks

## Description

Click an entry in the launcher → launcher dismisses → focus returns to
the previously-active app → Ctrl+V is synthesised into it. The
defining Ditto interaction.

Cross-platform: Windows (`SendInput`), Hyprland (`hyprctl
sendshortcut`), Sway/wlroots (`wtype`), fallback (`ydotool`), and a
config option to disable synthesis entirely (user pastes manually).

Decisions baked in:
- **Linux synthesis chain:** `hyprctl` → `wtype` → `ydotool` → off (D5).
- **Per-app keystroke override** supported (e.g. vim's `"+gp`).
- **Paste-back targets the previously-tracked foreground**, not the
  current foreground.

Depends on Phase 0's `ForegroundTracker` abstraction (extends the
`021` compositor detection module) and Phase 1's multi-format storage
(so we can put HTML/RTF/files on the clipboard, not just text).

## Sub-tasks

### 2.1 ForegroundTracker abstraction

`ditox-core/src/foreground.rs`:

```rust
pub struct ForegroundSnapshot {
    pub identifier: ForegroundId,    // platform-specific opaque
    pub process_basename: String,
    pub title: String,
    pub captured_at: DateTime<Utc>,
}

pub enum ForegroundId {
    Win32 { hwnd: isize, pid: u32 },
    Wlr   { toplevel: WlrToplevelHandle },
    Hypr  { address: String },
    X11   { window: u32 },
    Macos { pid: i32 },
    Unknown,
}

#[async_trait]
pub trait ForegroundTracker: Send + Sync {
    async fn snapshot(&self) -> Option<ForegroundSnapshot>;
    async fn restore(&self, snap: &ForegroundSnapshot) -> Result<()>;
    async fn subscribe(&mut self) -> mpsc::Receiver<ForegroundSnapshot>;
    async fn shutdown(&mut self) -> Result<()>;
}
```

Subscribers get a stream of foreground transitions; the launcher uses
the most recent non-ditox snapshot when summoned.

### 2.2 Windows tracker

Existing `force_restore_window` (`ditox-gui/src/app.rs:148-498`)
refactored into a `Win32ForegroundTracker`. New: process basename via
`QueryFullProcessImageNameW` + `Path::file_name`. Subscription via a
WinEvent hook (`SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`).

### 2.3 Wayland tracker

`WaylandForegroundTracker` subscribes to
`wlr-foreign-toplevel-management-unstable-v1` via `sctk`. Caches
`(handle, app_id, title)` for every toplevel; tracks the focused
toplevel via `OutputState`/`SeatState`.

`restore()` is a no-op on most Wayland compositors (we can't direct
the compositor to focus a specific window from a non-privileged
client). Document the limitation.

On Hyprland, `restore()` calls `hyprctl dispatch focuswindow
address:<addr>`. On Sway, `swaymsg "[con_id=N] focus"`.

### 2.4 Linux synthesis chain

`ditox-core/src/paste/synthesize.rs`:

```rust
pub trait Synthesizer: Send + Sync {
    async fn paste(&self, target: &ForegroundSnapshot, keys: &str) -> Result<()>;
    async fn is_available(&self) -> bool;
}

pub struct HyprctlSynthesizer;     // hyprctl dispatch sendshortcut
pub struct WtypeSynthesizer;       // spawn wtype
pub struct YdotoolSynthesizer;     // spawn ydotoold client
pub struct OffSynthesizer;         // no-op + user message
```

`pick_chain(platform: &Platform, config: &PasteConfig) -> Vec<Box<dyn Synthesizer>>`
returns the ordered list. `paste()` tries each until one returns Ok.

`hyprctl dispatch sendshortcut , ctrl+v, address:<addr>` is the
preferred Hyprland path because it's targeted at the specific window
(no race vs newly-focused popup).

### 2.5 Windows synthesis

`Win32Synthesizer` uses `SendInput`. Pre-flight: walk all 256 VK codes
via `GetAsyncKeyState`; release any down by sending KEYUP (Ditto's
stuck-modifier guard).

### 2.6 Per-app keystroke override

```toml
[paste.keystrokes]
"gvim.exe"          = "+gp"
"firefox.exe"       = "ctrl+v"
"konsole"           = "ctrl+shift+v"
"alacritty"         = "ctrl+shift+v"
"foot"              = "ctrl+shift+v"
```

Resolved via `process_basename`. Default `ctrl+v`.

Keystroke string format:
- Tokens separated by space or `+`.
- `+` = "and these together".
- ` ` (space) = "then this".
- Modifiers: `ctrl`, `shift`, `alt`, `super`.
- Special: `enter`, `tab`, `escape`, `space`, `backspace`, `delete`.
- Literal characters: any printable.

Examples:
- `ctrl+v` — Ctrl+V (default).
- `ctrl+shift+v` — Ctrl+Shift+V.
- `"+gp` — `"`, `+`, `g`, `p` (vim register paste).
- `shift+insert` — Shift+Insert (alternative paste).

Parser unit tests cover each token type.

### 2.7 `Clipboard Viewer Ignore` sentinel

When ditox writes to the clipboard during paste, also set the
`Clipboard Viewer Ignore` registered format (Windows) and equivalent
sentinels on Linux (custom MIME type our own capture skips).

This prevents the watcher from re-capturing what we just emitted.

### 2.8 GUI integration

Rename `Message::CopyEntry` → `Message::PasteEntry`. Flow:

1. Launcher summon: `ForegroundTracker::snapshot()` → store as
   `app.previous_foreground`.
2. User clicks entry: `Message::PasteEntry(id)`.
3. Handler:
   a. `db_handle.call(GetEntryFormats(id)).await?`
   b. `clipboard.write_all_formats(formats).await?`
   c. `tracker.restore(&previous_foreground).await?`
   d. `synthesizer.paste(&previous_foreground, &keys).await?`
   e. `Hide` IPC self-command.

Steps c and d run with a small `tokio::time::sleep(50ms)` between them
so the compositor has time to switch focus before the keystroke is
synthesised.

### 2.9 Modifier-held cycling

Already specified in `master-plan-v1.md` D2 + Phase 4 sub-task 5.
Phase 2 adds the **infrastructure**: an in-memory `selection_cursor`
that survives across hide/show and is `+1`'d when a re-fire happens
within 800ms of the previous fire.

The actual "stay-visible while modifier held" UX requires Phase 4's
long-running daemon; in Phase 2 we just lay the groundwork.

## Acceptance criteria

- [ ] **Windows:** Ctrl+Shift+V → click entry → text appears in
      Firefox/Notepad/Word.
- [ ] **Hyprland:** `bind = CTRL, grave, exec, ditox-gui --toggle` →
      click entry → text appears in the previously-focused app via
      `hyprctl sendshortcut`.
- [ ] **Sway:** same with `wtype` synthesiser.
- [ ] **Per-app override:** vim's `"+gp` synthesised when target is
      `gvim.exe`/`nvim`.
- [ ] **HTML clip pasted into Word/LibreOffice preserves formatting**
      (depends on Phase 1).
- [ ] **No re-capture loop:** pasting via ditox doesn't add a new
      entry to its own history.
- [ ] **Foreground correctness:** ditox-gui's own clicks never become
      the "previous foreground".
- [ ] **Graceful degradation:** if synthesizer fails, ditox writes the
      clip and shows an in-launcher status "Paste manually with Ctrl+V".

## Implementation Notes

`hyprctl` is shelled out via `tokio::process::Command`. Errors swallowed
to fall through to next synthesiser.

For per-process basename, the foreground snapshot already carries it.
Lookup is `paste.keystrokes.get(&basename.to_ascii_lowercase())` with a
`get_or("ctrl+v")` fallback.

`wtype` syntax: `wtype -M ctrl v -m ctrl` (press ctrl, type v, release
ctrl). Quoting matters; build the argv carefully.

`ydotool` requires `ydotoold` running. Detect via `which ydotool` plus
`systemctl --user is-active ydotool`.

## Risks

- **Risk:** GNOME Wayland has no foreground-tracking protocol; the
  Wayland tracker returns `Unknown`. Mitigation: emit a clear runtime
  warning on first launcher show, document.
- **Risk:** `hyprctl sendshortcut` syntax changes in Hyprland releases.
  Mitigation: detect `hyprctl version` at startup; pin tested versions.
- **Risk:** Apps with custom IME or multi-key sequences get wrong
  keystrokes. Mitigation: per-app override is the escape hatch.

## Work Log

### 2026-04-26
- Task file created (epic).
