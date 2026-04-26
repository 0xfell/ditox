# Task: Phase 2 — Paste-back UX (cross-platform)

> **Status:** completed (Linux MVP — 7/9 sub-tasks; 2 spun out to 033, 1 to 034)
> **Priority:** high
> **Phase:** 2 — Paste-back
> **Created:** 2026-04-26
> **Started:** 2026-04-26
> **Completed:** 2026-04-26
> **Estimated:** 3 weeks
> **MVP working on Hyprland (verified 2026-04-26)**
>
> **Sub-tasks landed in this task (Linux + cross-platform pure code):**
> - 2.1 `ForegroundTracker` abstraction
> - 2.3 (Hyprland part) `HyprctlForegroundTracker`
> - 2.4 Linux synthesis chain (`hyprctl` → `wtype` → `ydotool` → `off`)
> - 2.6 Per-app keystroke override parser
> - 2.7 Cross-process paste sentinel
> - 2.8 GUI integration (end-to-end paste-back flow)
> - 2.9 `SelectionCursor` groundwork
>
> **Deferred follow-ups:**
> - 2.2 Win32 foreground tracker → spun out as
>   [`033-phase-2-windows-paste-back.md`](../planned/033-phase-2-windows-paste-back.md)
> - 2.5 Win32 synthesizer → ditto (same task 033, since 2.2 + 2.5
>   share a Windows hardening pass)
> - 2.3 (cont) `wlr-foreign-toplevel-management` subscription →
>   [`034-phase-2-wlr-foreign-toplevel.md`](../planned/034-phase-2-wlr-foreign-toplevel.md)

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

### 2026-04-26 — task moved to in-progress; sub-tasks 2.1 + 2.6 landed

Phase 2 begins. Two foundational sub-tasks done in this initial
session — both Linux-testable, no platform-dep heavy lifting.

**2.1 — `ForegroundTracker` abstraction.** New
`ditox-core/src/foreground.rs` introduces:

- `ForegroundSnapshot { identifier, process_basename, title,
  captured_at }` — what the launcher remembers about the
  previously-focused window.
- `ForegroundId` enum — portable identifier carrying only scalar /
  `String` fields per platform (`Win32 { hwnd: i64, pid: u32 }`,
  `Hypr { address }`, `Wlr { app_id, title }`, `X11 { window }`,
  `Macos { pid }`, `Unknown`). No platform-dep types leak into
  `ditox-core`.
- `ForegroundId::supports_restore()` — encodes the per-platform
  matrix of "can we re-focus this window from a non-privileged
  client". Win32 / Hypr / X11 / Macos = yes; Wlr / Unknown = no
  (wlroots' foreign-toplevel-management protocol has no
  client-driven activate request — documented in the doc comment).
- `ForegroundTracker` trait — sync-only (no async) per the same
  reasoning as `CaptureSource`: keep `ditox-core` runtime-agnostic;
  event-driven backends spawn a thread and surface events through
  `mpsc::Receiver`. Methods: `name`, `snapshot`, `restore`,
  `subscribe`, `shutdown`.
- `ForegroundFilter<T>` decorator — wraps any tracker and drops
  any snapshot whose `process_basename` matches one of the
  configured launcher names. Default self-names: `["ditox-gui",
  "ditox", "ditox-gui.exe", "ditox.exe"]`. Comparison is ASCII
  case-insensitive (Windows reports mixed-case basenames).
  Subscription path spawns a filter thread that drops self-events
  before forwarding.
- `NoopForegroundTracker` — for GNOME Wayland and other
  unsupported compositors. `snapshot` returns `None`, `restore`
  is a successful no-op. Documented degraded-mode behaviour.
- `MockForegroundTracker` (`#[doc(hidden)]`) — test helper with
  `set_snapshot()`, `inject()` (push to subscriber), `restore_log()`.

**Deviation from epic spec:** the spec used `async_trait` and
`chrono::DateTime<Utc>`. Replaced both: sync trait per the
established `CaptureSource` pattern; `SystemTime` for parity with
`RawClip.captured_at`. Avoids pulling `tokio` and `chrono` into
`ditox-core`.

19 unit tests covering: ForegroundId variants + `supports_restore`
matrix + `kind` labels + Hash/Eq; Noop returns/disconnect/idempotent
shutdown; Mock snapshot/restore-log/inject; Filter drops self
(basic + case-insensitive + custom self-names + default-includes-
both-binaries); subscribe-side filtering; trait object-safety.

**2.6 — Per-app keystroke override parser.** New
`ditox-core/src/paste/keystroke.rs` (parent module
`ditox-core/src/paste.rs`) introduces:

- `Modifier` enum (`Ctrl`/`Shift`/`Alt`/`Super`) with synonym
  parsing (`super`/`logo`/`win`/`meta`/`cmd` → Super;
  `ctrl`/`control` → Ctrl). Case-insensitive.
- `SpecialKey` enum covering 39 keys: enter/tab/escape/space/
  backspace/delete/insert/home/end/pageup/pagedown/up/down/left/
  right + F1-F24. Synonyms: return → enter, esc → escape,
  bs → backspace, ins → insert, pgup → pageup, etc.
- `Key { Special(SpecialKey), Char(char) }` — final key after
  modifier resolution; `Char` carries any Unicode scalar.
- `Chord { modifiers: Vec<Modifier>, key: Key }` — one
  press-everything-at-once group.
- `KeystrokeSequence { chords: Vec<Chord> }` — ordered chord
  list, what the launcher's synthesizer iterates over.
- `parse(&str) -> Result<KeystrokeSequence, ParseError>` plus
  `FromStr` impl for `KeystrokeSequence`.
- `DEFAULT_KEYSTROKE = "ctrl+v"` constant.

**Disambiguation rules** (`+` is overloaded — both chord-joiner and
literal char):

1. Whitespace-separate input into "fragments".
2. Per fragment:
   a. Exact special-key name (`enter`, `f5`) → one Step(Special).
   b. Starts with `<modifier>+` → parse as chord (split on `+`,
      all-but-last are modifiers, last is the key).
   c. Else → each character is its own one-key chord. This is
      what makes the spec's vim example `"+gp` resolve to four
      sequential keystrokes (`"`, `+`, `g`, `p`) — none of those
      tokens before the first `+` is a modifier name, so rule 2
      doesn't fire.

`ParseError` variants: `Empty`, `DanglingPlus { fragment }`,
`UnknownModifier { fragment, token }`, `ModifierAsKey { fragment,
token }`, `UnknownKey { fragment, token }` — each carries the
context for actionable error messages.

`Display` for `KeystrokeSequence` produces a canonical
representation (`"ctrl+v ctrl+s"`); round-trips for canonical
inputs but normalises non-canonical ones (`"+gp` round-trips as
`"\" + g p"`, four space-separated chords).

31 unit tests covering: modifier parse (canonical + synonyms +
case + rejection); special-key parse (canonical + synonyms +
case + rejection — including `f0` and `f25` boundary checks);
parse happy path (simple chord, multi-modifier, special key,
two-chord sequence, special-key alone, vim register paste,
single char, multi-char literal, ctrl+special, case
insensitivity, super synonyms, extra whitespace, default
constant); parse error path (empty input, dangling `+`,
unknown modifier, modifier as key, multi-char unknown key);
Display round-trip (canonical + two-chord + literal sequence);
`FromStr` + `Default`.

**Workspace test count after this session: 207 tests** (was 159;
+19 foreground + +31 keystroke + small adjustments). All clippy
`-D warnings` + fmt clean.

**Phase 2 status: 2/9 sub-tasks done.** Next: 2.3 (Wayland
foreground tracker — needs `wayland-client` event loop on a
dedicated thread; testable on Hyprland), then 2.4 (Linux
synthesis chain — `hyprctl`/`wtype`/`ydotool` shell-outs;
testable end-to-end). Windows trackers/synthesizer (2.2, 2.5)
will follow the same defer-to-Windows pattern as task 032.

### 2026-04-26 — sub-tasks 2.3 (Hyprland), 2.4, 2.7, 2.8 landed; MVP verified end-to-end on Hyprland

Four more sub-tasks done in this session, taking Phase 2 from 2/9 to
**6/9** with the click-to-paste flow now **verified working** on
Hyprland.

**2.3 — Hyprland foreground tracker.** New
`ditox-core/src/foreground/hyprctl.rs` introduces
`HyprctlForegroundTracker`:

- `snapshot()` shells out `hyprctl activewindow -j`, parses JSON
  via serde, returns `ForegroundSnapshot { identifier:
  ForegroundId::Hypr { address }, process_basename, title,
  captured_at }`.
- `parse_activewindow()` is the pure JSON-text → snapshot helper
  used by tests; the full `snapshot()` adds the `Command::output`
  layer.
- `process_basename` resolution: prefer the wayland `class` field
  (always full app name like `brave-browser`); fall back to
  `/proc/<pid>/comm`. The `/proc` path is necessary for non-Wayland
  legacy clients and Nix-wrapped binaries (where `comm` is the
  actual binary, prefixed `.foo-wrapped`). Truncation at 15 chars
  (`TASK_COMM_LEN-1`) is documented in the doc comment.
- `restore()` shells out `hyprctl dispatch focuswindow
  address:<addr>` — Hyprland is the rare Wayland compositor that
  exposes a client-driven activate request.
- `subscribe()` is currently `Err(())` — wlr-foreign-toplevel
  event loop deferred (would need a dedicated `wayland-client`
  thread; not on the critical path for MVP).
- `is_available()` checks `which hyprctl`; `name()` returns
  `"hyprctl"`.

10 unit tests covering: parse happy path (class + initialClass
fallback + title + address); parse error paths (malformed JSON,
missing fields); `process_basename` priority (class > comm);
restore command shape; trait-object safety; subscribe returns
`Err`.

**2.4 — Linux synthesis chain.** New
`ditox-core/src/paste/synthesize.rs` introduces:

- `Synthesizer` trait — sync, `Send + Sync`. Methods: `name`,
  `is_available`, `paste(target, sequence)`. No async (consistent
  with rest of `ditox-core`).
- `HyprctlSynthesizer` — uses `hyprctl dispatch sendshortcut ,
  ctrl+v, address:<addr>`. Targeted at the specific window so no
  race vs newly-focused popup. Only available on Hyprland.
- `WtypeSynthesizer` — shells `wtype` with `-M ctrl v -m ctrl`
  argv. Works on wlroots compositors (Sway, river). Builds
  modifier press/release pairs around the key press from
  `KeystrokeSequence`.
- `YdotoolSynthesizer` — fallback. Shells `ydotool key` with
  numeric keycodes (`29:1 47:1 47:0 29:0` for Ctrl+V). Requires
  `ydotoold`; detection is `which ydotool` only (we don't probe
  the daemon — paste failure surfaces it).
- `OffSynthesizer` — no-op success. Used as the always-available
  sentinel that terminates the chain. Lets the user paste manually
  with Ctrl+V.
- `pick_chain(platform: &Platform) -> Vec<Box<dyn Synthesizer>>`
  — returns the per-platform ordered list. Hyprland: hyprctl →
  wtype → ydotool → off. Other Wayland: wtype → ydotool → off.
  Off always last.
- `paste_with_chain(chain, target, sequence)` — iterates;
  returns the first `Ok(synth_name)` or aggregated error.
- Each impl exposes an inherent `argv()` (or analogous) that
  returns the exact command-line — tests assert on this without
  spawning subprocesses.

24 unit tests covering: per-synthesizer argv shape (Ctrl+V, vim
`"+gp` four-chord sequence, F-keys, Super+L); chain composition
per platform (Hyprland vs Sway vs Windows-ignored vs Macos); `Off`
always-available + always-success; chain-skip-when-unavailable;
trait object safety; modifier release order (release after press,
LIFO); special-key keycode tables for ydotool.

**2.7 — Paste-back sentinel.** New `ditox-core/src/paste/sentinel.rs`
introduces `PasteSentinel`:

- Filesystem-backed at `<data_dir>/last-paste.json`. Atomic write
  via tmp-file + rename. Best-effort: failures logged via
  `tracing::warn!`, never propagated.
- `record(hash: &str)` — write `{ hash, recorded_at: <unix_ms> }`.
- `matches(hash, ttl: Duration)` — read; return `true` iff
  `hash == stored.hash` AND `now - stored.recorded_at <= ttl`.
- `clear()` — best-effort delete.
- `Watcher::process_clip` (in `ditox-core/src/watcher.rs`)
  consults the sentinel **before** `db.insert()` for both image
  and text branches. On match: skip insert, update `last_hash`,
  return `Ok(false)`. The hash to record matches what the watcher
  computes: text = `Clipboard::hash(content.as_bytes())`; image
  = `entry.content` (which IS the SHA hash per content-addressed
  image storage).

**Why filesystem instead of MIME-based sentinel** (deviation from
epic spec): the multi-format clipboard write path is still
write-side TODO. A filesystem sentinel works *today* for both text
and image clips, doesn't depend on the watcher recognising a
custom MIME type, and survives process boundaries (gui writes →
watcher reads).

8 unit tests covering: record + matches happy path; expired by
TTL; non-matching hash; missing file → `false`; tmp-rename
atomic write; concurrent-read tolerance; `clear` is idempotent;
malformed JSON treated as missing.

**2.8 — GUI integration.** End-to-end wiring across
`ditox-core` + `ditox-gui`:

- `Config.paste = PasteConfig { disabled, synthesizer_chain,
  keystrokes, sentinel_ttl_ms }` in `ditox-core/src/config.rs`.
  `keystroke_for(basename)` ASCII-case-insensitive lookup;
  `sentinel_ttl()` defaults to 2 s when 0. 6 unit tests.
- `build_default_tracker()` factory in `ditox-core/src/foreground.rs`
  — per-platform: Hyprland → `HyprctlForegroundTracker`; other
  Wayland/X11/Macos → `NoopForegroundTracker` for now (real
  wlr/xdg/macos trackers are 2.3-cont/2.2/2.5). Always wrapped in
  `ForegroundFilter` with default self-names.
- `ditox-gui/src/main.rs::run` captures `previous_foreground`
  **before** `app::run_with` is called — by the time iced opens
  its window, ditox-gui itself becomes foreground, so the
  snapshot must happen pre-iced.
- `ditox-gui/src/app.rs::DitoxApp` extended with three fields:
  `previous_foreground: Option<ForegroundSnapshot>`,
  `foreground_tracker: Option<Box<dyn ForegroundTracker>>`,
  `synthesizer_chain: Option<Vec<Box<dyn Synthesizer>>>`.
- `paste_and_exit(&mut self, entry: Entry) -> !` helper at
  `app.rs:1230`. Order: clipboard write → `db_handle.touch(id)`
  fire-and-forget → `PasteSentinel::record(hash)` → `tracker.
  restore(snap)` (if `supports_restore`) → 50 ms `thread::sleep`
  → parse keystroke string → `paste_with_chain(chain, snap,
  sequence)` → `save_window_state` → `process::exit(0)`. The 50 ms
  sleep gives the compositor time to switch focus before the
  keystroke is synthesised.
- `Message::CopyEntry`/`CopyFromPreview` handlers replaced to
  call `paste_and_exit`. (Names kept rather than renamed to
  `PasteEntry` — smaller diff; semantic is `copy + paste-back`.)
- iced 0.14 `boot_app` is `Fn` (not `FnOnce`) → state threaded
  via three `OnceLock<Mutex<Option<T>>>` statics
  (`APP_PREVIOUS_FOREGROUND` / `APP_FOREGROUND_TRACKER` /
  `APP_SYNTHESIZER_CHAIN`); `boot_app` `take()`s ownership on
  first call. `run_with` signature went from 3 args to 6 args.
- Build error fixed during this iteration: removed stray
  `.flatten()` at `app.rs:2973` (inner closure already returns
  `Option<ForegroundSnapshot>`, not `Option<Option<...>>`).

**Live verification on Hyprland 2026-04-26 (user-confirmed):**

```
RUST_LOG=ditox=debug,ditox_core=debug,ditox_gui=debug \
  ./target/release/ditox-gui 2>/tmp/ditox.log
```

Log shows:

```
captured previous-foreground snapshot for paste-back
  process=com.mitchellh.ghostty
  title=RUST_LOG=...
  kind=hypr
constructed paste-back synthesizer chain
  chain=["hyprctl", "wtype", "ydotool", "off"]
Pasting: Diablo® IV: Lord of Hatred™...
recorded paste sentinel hash=e975249e
paste-back succeeded synth=hyprctl target=com.mitchellh.ghostty
```

End-to-end: pre-iced foreground snapshot → entry click →
clipboard write → sentinel record → focus restore via hyprctl →
keystroke synthesis via hyprctl `sendshortcut` → text "Diablo®
IV: Lord of Hatred™..." appeared at the ghostty prompt. User
confirmed visual paste effect. Watcher correctly skipped
re-capture (sentinel matched).

**Workspace test count after this session: 274 tests** (was 207;
+10 hyprctl + +24 synthesize + +6 PasteConfig + +8 sentinel +
build_default_tracker + small adjustments). All clippy
`-D warnings` + fmt clean.

**Phase 2 status: 6/9 sub-tasks done.** Outstanding:
- **2.2** Win32 foreground tracker — Windows-side, deferred.
- **2.3 (cont)** Wayland wlr-foreign-toplevel subscription — needs
  dedicated `wayland-client` event-loop thread; not on MVP path.
- **2.5** Win32 synthesizer (`SendInput` + stuck-modifier guard) —
  Windows-side, deferred.
- **2.9** `selection_cursor` groundwork — small persistent file at
  `<data_dir>/cursor.json` with debounce.

**Pre-existing tray panic surfaced** (not blocking 2.8): on
Hyprland the `ditox-tray` thread panics at runtime —
`libappindicator-sys-0.9.0/src/lib.rs:41` fails to `dlopen`
`libayatana-appindicator3.so.1` / `libappindicator3.so.1`. The
panic is on a dedicated thread so the GUI itself stays alive,
but the tray icon never appears under Hyprland on this dev
shell. Tracked as a follow-up bug; root cause is missing
runtime library in `flake.nix` GUI environment.

### 2026-04-26 — sub-task 2.9 landed; tray panic side-fix; task closed

**Tray panic side-fix.** `tray-icon`'s Linux backend pulls in
`libappindicator-sys`, which dlopens
`libayatana-appindicator3.so.1` / fallbacks from a `lazy_static`
initialiser and panics outright on failure. The panic surfaces
from the dedicated `ditox-tray` thread on first menu build, so
`catch_unwind` can't recover. Fix: probe for a loadable candidate
soname via `libc::dlopen` (`RTLD_LAZY` + immediate `dlclose` so
the probe doesn't pin a handle) before spawning the tray thread.
If none load, log an actionable warning (NixOS: `nix run` /
`nix develop` / `nix build`; other distros: install
`libayatana-appindicator`) and skip cleanly. Verified outside
`nix develop` (warning fires, GUI continues) and inside (tray
spawns silently as before). Commit `98ef85f`.

**2.9 — Selection cursor groundwork.** New
`ditox-core/src/paste/cursor.rs` introduces:

- `SelectionCursor { index, last_fire_at }` — pure state. No IO.
  Methods `index() / last_fire_at() / index_for_list(len) /
  fire(now, window) / reset()`. `fire()` advances the index by
  one (saturating) when `now - last_fire_at <= window`, else
  resets to 0; either way `last_fire_at = now`.
- `index_for_list(len)` clamps via modulo so wrap-around past the
  list length is well-defined; returns 0 for empty lists.
- `DEFAULT_REFIRE_WINDOW: Duration = 800 ms` constant (mirrors
  master-plan D2).
- `PersistentSelectionCursor` — filesystem wrapper. Reads/writes
  `<data_dir>/cursor.json` via the same `tmp-write + rename`
  atomic-write pattern as `PasteSentinel`. `read()` returns a
  fresh cursor on missing file / corrupt JSON / unknown schema
  version (forward-compat). `write()` is best-effort with
  `tracing::warn!` on failure. `fire_and_persist()` is the
  one-call helper used by the launcher.
- Schema-versioned on-disk record (`{ version: 1, index,
  last_fire_at_ms }`); future versions can change the shape and
  old binaries gracefully reset rather than crash.

**`PasteConfig.cursor_refire_window_ms` (default 800).** New
`cursor_refire_window()` helper returns
[`DEFAULT_REFIRE_WINDOW`] when 0 (the unset default in TOML).

**`ditox-gui` integration.** `main.rs::run` now calls
`PersistentSelectionCursor::at_default_path()?.fire_and_persist(now,
config.paste.cursor_refire_window())` and threads the resulting
`cursor.index()` to `app::run_with` as a 7th arg. `DitoxApp::new`
clamps via `initial_selection % entries.len()` (modulo wrap; 0 for
empty list) and uses it as the initial `selected_index`. The
state passes through an `AtomicUsize` static, not a `Mutex` —
plain `usize` doesn't need the round-trip.

**Live verification on Hyprland 2026-04-26:**

```
Run 1 (cold):                       fired selection cursor index=0 window_ms=800
                                    cursor.json: {"version":1,"index":0,"last_fire_at_ms":...}

Run 2 (~573 ms after Run 1):        fired selection cursor index=1
                                    cursor.json: ...,"index":1,...

Run 3 (~575 ms after Run 2):        fired selection cursor index=2
                                    cursor.json: ...,"index":2,...

Sleep 1.5 s.

Run 4 (~2.0 s after Run 3):         fired selection cursor index=0
                                    cursor.json: ...,"index":0,...
```

End-to-end: rapid re-fires within 800 ms advance the cursor; an
idle past the window resets to 0. JSON persists across processes.

12 unit tests on `SelectionCursor` (new/fire/refire/reset/
boundary/overflow/index_for_list-empty/within/wrap/exact-len)
plus 9 on `PersistentSelectionCursor` (read-missing/round-trip/
corrupt-json/unknown-version/atomic-via-tmp/parent-dirs/
fire-and-persist-{cold,advance,reset}/clear-{idempotent,absent}/
path) plus 3 new on `PasteConfig` (default window /
explicit value / TOML round-trip with `cursor_refire_window_ms`).

**Workspace test count after this session: 301 tests** (was 274;
+24 cursor.rs + +3 PasteConfig). All clippy `-D warnings` + fmt
clean.

**Phase 2 closed: 7/9 sub-tasks landed.** Three follow-ups
deferred and spun out:

- **2.2** Win32 foreground tracker → task 033.
- **2.3 (cont)** Wayland wlr-foreign-toplevel subscription → task 034.
- **2.5** Win32 synthesizer → task 033 (combined with 2.2 since
  they share a Windows hardening pass).
