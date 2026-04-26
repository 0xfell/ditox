# Task: Phase 4 — Ditto UX replication (long-running GUI + layer-shell)

> **Status:** in-progress (1/12 sub-tasks done — 4.1 + 4.2 plumbing only; full long-running UX in 4.3)
> **Priority:** high
> **Phase:** 4 — Ditto UX
> **Created:** 2026-04-26
> **Started:** 2026-04-26
> **Estimated:** 5-6 weeks

## Description

Architecture reversal of task `013`. Replace the one-shot launcher with
a long-running daemon that owns the layer-shell window, IPC socket,
foreground tracker subscription, modifier-held cycling cursor, and per-clip
hotkey registration.

Visual design (420×520 floating panel) is retained. Process model is not.

This is **the most user-visible phase** — it's what makes ditox *feel*
like Ditto.

Decisions baked in:
- **Long-running process** (D2).
- **`wlr-layer-shell` for Linux** (H2 — proper integration, no window
  rule hacks).
- **Position modes:** at_caret (Win), at_cursor (Win + Hyprland),
  at_previous, at_active_window_centre, fixed (H6).
- **Hyprland config helper is opt-in only** (`--install-hyprland-config`)
  (H5).

Companion docs:
- `docs/notes/ui-replication.md` — internal design.
- `docs/notes/hyprland-setup.md` — user-facing.

## Sub-tasks

### 4.1 Long-running daemon scaffold

- **Single instance.**
  - Linux: `flock(LOCK_EX | LOCK_NB)` on
    `$XDG_RUNTIME_DIR/ditox-gui-${UID}.lock`.
  - Windows: `CreateMutexW(L"Local\\ditox-gui-{user}")`.
- **IPC channel.**
  - Linux: Unix socket at `$XDG_RUNTIME_DIR/ditox-gui-${UID}.sock`,
    `0600` perms.
  - Windows: named pipe `\\.\pipe\ditox-gui-{Username}`.
- **First launch:** acquire lock, bind socket, become daemon.
- **Second launch:** detect existing instance, send command, exit.
- **CLI flags become IPC commands:**
  - `--toggle` → `TOGGLE\n`
  - `--show` → `SHOW\n`
  - `--hide` → `HIDE\n`
  - `--quit` → `QUIT\n`
  - `--status` → `STATUS\n` (prints reply)
  - `paste-clip <id>` → `PASTE-CLIP <id>\n`
  - `cycle-next` → `CYCLE-NEXT\n`
  - `cycle-prev` → `CYCLE-PREV\n`
- **Reply format:** `OK\n` or `ERR <msg>\n`.

### 4.2 IPC protocol implementation

Async listener task in the daemon:

```rust
loop {
    let (stream, _) = listener.accept().await?;
    tokio::spawn(handle_client(stream, app_handle.clone()));
}

async fn handle_client(mut stream: UnixStream, handle: AppHandle) {
    let mut line = String::new();
    BufReader::new(&mut stream).read_line(&mut line).await?;
    let response = handle.dispatch(parse_command(&line)).await;
    stream.write_all(format!("{}\n", response).as_bytes()).await?;
}
```

Commands route through `iced::Command` so the GUI thread stays in
charge of UI state.

### 4.3 Layer-shell window on Linux

**Decided:** Path A1 — `iced_layershell` (see
[ADR 0001](../../notes/adr/0001-layer-shell-strategy.md)).

Concrete steps:
- Add `iced_layershell = "=0.17.1"` to `ditox-gui/Cargo.toml`
  (pinned through Phase 4 to avoid the in-flight v0.18 beta churn;
  unpin in v0.5).
- Replace `iced::application(...)` with
  `iced_layershell::build_pattern::application(...)` behind
  `#[cfg(unix)]`. Windows continues to use iced's default winit
  backend.
- Wrap the existing `Message` enum with `#[to_layer_message]` to
  inherit the layer-shell control variants.
- Translate the existing `Position::SpecificWith` window placement
  into `LayerShellSettings { anchor: Anchor::Bottom | Anchor::Left,
  margin: (0, 0, 24, 24), size: Some((420, 520)),
  keyboard_interactivity: KeyboardInteractivity::Exclusive,
  layer: Layer::Top, .. }`.
- Runtime selection via task 021's `Platform`:
  `Hyprland | Sway | Kde | Wlroots` → layer-shell;
  `GnomeWayland | Other` → fall back to xdg_toplevel.
- Reference: `spike/a1-iced-layershell/src/main.rs` (179 LOC) shows
  the full integration shape including `view`/`update`/`subscription`
  for a 5-entry launcher.

**Path A2 (rejected, kept as fallback):** custom `ditox-layershell`
crate using `smithay-client-toolkit` + `tiny-skia`/`wgpu`. Rejected
because A1 is a 1-callsite swap that preserves all 1100+ LOC of
existing iced widgets, while A2 would require rewriting `view`,
theming, image cache, scroll, and focus management (~1500-2000
LOC). Re-evaluate only if A1's beta churn proves unworkable.

On Windows the existing iced/winit path is unchanged.

### 4.4 Configurable popup position

`Config.gui.position`:

```toml
[gui]
position = "at_cursor"
# possible values:
#   "at_caret"            — Windows only; Wayland fallback to active_window_centre
#   "at_cursor"           — Windows + Hyprland (hyprctl cursorpos)
#   "at_previous"         — last-known geometry per resolution (see 4.10)
#   "at_active_window_centre"
#   "fixed"               — uses [gui.fixed_anchor]

[gui.fixed_anchor]
horizontal = "left"        # left | centre | right
vertical   = "bottom"      # top | middle | bottom
monitor    = "primary"     # primary | active | "<monitor-id>"
offset     = [20, -20]     # [x, y] in DIPs
```

Position math in `ditox-core/src/position.rs` (per `ui-replication.md::A4`).

### 4.5 Custom non-client area

- **Windows:** existing custom title bar; extend to all four caption
  positions (top/bottom/left/right).
- **Linux:** layer-shell is already borderless; add an internal "drag
  handle" if user picks `position = "at_previous"` or `"fixed"` (but
  layer-shell windows don't move via compositor drag — the drag is
  internal: we update the layer-surface anchor offset).

### 4.6 Always-on-top toggle

Pin button in the launcher header. Behaviour:

- **Windows:** `SetWindowPos(HWND_TOPMOST, ...)`.
- **Linux:** layer-shell `Top` layer already gives this; pin toggles
  between `Top` and `Overlay`.

### 4.7 Modifier-held cycling

Detection via IPC re-fire timing (per `ui-replication.md::A5`).

```rust
struct CycleState {
    last_toggle_at: Option<Instant>,
    cycle_window: Duration,  // 800 ms default
}

fn handle_toggle(state: &mut CycleState, ...) {
    let now = Instant::now();
    let in_window = state.last_toggle_at
        .map(|prev| now.duration_since(prev) < state.cycle_window)
        .unwrap_or(false);

    if in_window && app.visible {
        app.advance_selection();
    } else {
        app.show_at_position(...);
        app.reset_selection();
    }

    state.last_toggle_at = Some(now);
}
```

### 4.8 Hide-on-blur with grace period

Subscribe to `window::Event::Unfocused` (already used today). Gate by:

- `Config.gui.hide_on_blur` (default true).
- `Config.gui.hide_on_blur_grace_ms` (default 250).
- Don't hide if the unfocus happens within `grace_ms` of the show.

### 4.9 Tooltip-as-preview

Custom iced widget `EntryPreview` that renders next to the hovered list
row.

- Trigger after `Config.gui.hover_delay_ms` (default 400).
- Content: text → monospace + optional syntect highlight; image → 256px
  thumbnail; HTML → sanitised via `ammonia` rendered as styled iced
  text; RTF → stripped to plain.
- Hide on mouse-leave or selection-change.

### 4.10 Inline list extras

- Color swatches (Phase 3.3 — fold in here if not already done).
- Hotkey number prefix for the first 10 entries (`1`–`9`, `0` for 10th)
  in a small monospace font.
- Glyphs:
  - 🔒 (no-auto-delete)
  - 📌 (sticky / pinned)
  - 🏷️ (has tags)
  - 📁 (in a collection)
  - 📝 (has notes)
  - ✓ (was just pasted)

Use Bootstrap Icons (`iced_fonts`) on GUI; Unicode symbols on TUI.

### 4.11 `--install-hyprland-config` helper

```
$ ditox-gui --install-hyprland-config

Wrote: /home/user/.config/hypr/conf.d/ditox.conf

To activate, add this line to your hyprland.conf:

    source = ~/.config/hypr/conf.d/ditox.conf

Then reload:  hyprctl reload
```

File contents (between `# >>> ditox-managed >>>` markers):

```hyprlang
# >>> ditox-managed (do not edit between these markers) >>>
exec-once = ditox-gui --hide
bind = CTRL, grave, exec, ditox-gui --toggle
windowrulev2 = float, class:^(ditox-gui)$
windowrulev2 = pin, class:^(ditox-gui)$
windowrulev2 = noborder, class:^(ditox-gui)$
windowrulev2 = noshadow, class:^(ditox-gui)$
windowrulev2 = noanim, class:^(ditox-gui)$
source = ~/.config/hypr/conf.d/ditox-binds.conf
# <<< end ditox-managed <<<
```

The `ditox-binds.conf` source line is for Phase 5 per-clip hotkeys
(file is created empty here).

`--uninstall-hyprland-config` removes the file (and the source line if
present in main config).

### 4.12 Per-resolution window state

Migrate `window_state.json` per `ui-replication.md::A10`.
Backwards-compat: old file format (single object) parsed as the
"default" geometry under key `"_default"`.

## Acceptance criteria

- [ ] Second `ditox-gui` invocation talks to the running first via IPC,
      doesn't spawn a new process.
- [ ] On Hyprland with the helper-installed config, `Ctrl+~` shows a
      properly-floating launcher (no tiling, no compositor weirdness).
- [ ] On Sway, the same works with manual config (helper is
      Hyprland-specific).
- [ ] On Windows 11, `Ctrl+Shift+V` shows the launcher; modifier-held
      cycling advances selection.
- [ ] Tooltip preview appears on hover after 400 ms; supports
      text/image/HTML/RTF.
- [ ] Pin toggle keeps launcher visible across blur events.
- [ ] Position-at-cursor places the launcher at the actual cursor
      position on both Windows and Hyprland.
- [ ] Window state survives reboots; different resolution = different
      saved geometry.
- [ ] `--install-hyprland-config` is idempotent (re-running is safe).

## Implementation Notes

This phase reverts task `013`. Coordinate carefully:

- The visual design (420×520, bottom-left default) is retained.
- The internal animation, theming, and styling code is retained.
- The single-launch process model is what changes.
- `gui-improvements.md` task references should be reconciled.

The lock + sock IPC code can be partially recovered from git history
(pre-`013`).

## Risks

- **Risk:** Layer-shell A1 path doesn't materialise in time.
  Mitigation: Phase 0.9 spike forces an early decision.
- **Risk:** GTK tray-icon thread interferes with iced event loop on
  Linux when we add layer-shell. Mitigation: keep the GTK thread
  strictly separate (already the case).
- **Risk:** Modifier-held cycling on Linux (where we can't query
  modifier state) feels wrong. Mitigation: 800 ms re-fire window is
  generous; user feedback during beta.

## Work Log

### 2026-04-26 — task moved to in-progress; sub-tasks 4.1 + 4.2 plumbing landed

Phase 4 begins. The first commit ships the IPC plumbing only —
full long-running UX (paste-and-stay, hide-on-blur with grace,
modifier-held cycling, layer-shell window) lands incrementally
across sub-tasks 4.3-4.12.

**4.1 — Single-instance lock + IPC socket.**

`ditox-gui/src/ipc.rs` (new module):
- `lock_path()` / `socket_path()` → `$XDG_RUNTIME_DIR/ditox-gui-<uid>.{lock,sock}`
  (falls back to `/tmp/ditox-gui-<uid>` when `XDG_RUNTIME_DIR` is
  unset). Both files are mode 0600.
- `acquire_lock()` opens the lock file with `OpenOptions` (mode
  0600 via `OpenOptionsExt`) and `try_lock_exclusive()` via
  `fs2`. Returns `Some(file)` on success, `None` on contention.
  Caller must keep the file alive — drop releases the flock and
  process exit also releases it (kernel handles).
- `try_send_to_daemon(action)` returns:
  - `Sent { reply }` — daemon present, reply received.
  - `Rejected { message }` — daemon answered `ERR ...`.
  - `NoDaemon` — socket absent or connection refused.
- `spawn_listener()` binds the Unix socket (with stale-file
  cleanup), spawns a `ditox-gui-ipc` accept thread that spawns
  per-client `ditox-gui-ipc-client` worker threads, returns
  `(Receiver<DaemonCommand>, SocketGuard)`. The guard unlinks
  the socket file on drop (best-effort; `process::exit` skips
  it but the next `bind` cleans up the stale path).
- `DaemonCommand { command, reply }` carries a `SyncSender<String>`
  for the IPC client's reply. Drop impl emits `ERR no-reply` so
  the client never hangs on read. `reply_ok()`, `reply_ok_with()`,
  and `reply_err()` (allow-dead-code, future use).
- 8 unit tests covering: command parsing (canonical + case-insensitive
  + reject), action wire round-trip, lock-path shape, reply
  semantics (ok/err/dropped-without-reply).

**4.2 — IPC protocol (newline-terminated text).**

```text
TOGGLE | SHOW | HIDE | QUIT | STATUS    →    OK | OK <payload> | ERR <msg>
```

Wire-format and parsing are stable; future commands (Phase 5
`paste-clip <id>`, `cycle-next`, `cycle-prev`) extend the same
grammar.

**`ditox-gui/src/main.rs`:**
- Step 1: `try_send_to_daemon(action)` first — for every action
  including bare `Launch`. Sent → exit 0; Rejected → exit 1;
  NoDaemon → fall through. Lets the same `ditox-gui` keybind
  serve both "first launch" (start daemon) and "summon"
  (forward to running one).
- `--quit` with no daemon prints "nothing to do" and exits 0.
- Step 2: `acquire_lock()`. On contention (race with another
  starter) retries `try_send_to_daemon` once before erroring out.
- Step 3: `spawn_listener()` binds the socket, threads the
  receiver into `app::run_with` as a new 8th arg.

**`ditox-gui/src/app.rs`:**
- `Message::PollIpc` and `Message::WindowOpened(window::Id)`
  variants added.
- Subscriptions: 50 ms `iced::time::every` tick → `PollIpc`;
  `iced::window::open_events()` → `WindowOpened`.
- `update::Message::PollIpc → drain_ipc()` drains the receiver
  via `try_recv` and processes each `DaemonCommand`:
  - `Show`/`Hide`/`Toggle`: update `self.visible` flag, reply
    `OK`, and (best-effort) issue `iced::window::set_mode(id,
    Mode::Hidden|Windowed)` if the main window's `Id` was
    captured.
  - `Quit`: reply `OK`, save window state, then
    `std::process::exit(0)` from a deferred 50 ms timer thread
    so the IPC reply lands at the client before the process
    dies.
  - `Status`: reply `OK visible=<b> entries=<n> version=<v>`.

**Scope explicitly held back from this commit:**
- Window stays open after copy/paste (one-shot `paste_and_exit`
  semantics retained). Phase 4.3 will replace with
  `paste_and_hide` so the daemon truly outlives a paste cycle.
- Layer-shell on Linux (Phase 4.3, builds on the 022 ADR's
  `iced_layershell` decision).
- Hide-on-blur grace period (Phase 4.8).
- Modifier-held cycling beyond the existing 2.9 cursor (Phase 4.7).

**Live verification on Hyprland 2026-04-26:**

```
$ ditox-gui &     # starts daemon
INFO ditox_gui: ditox-gui daemon listening on IPC socket socket=/run/user/1000/ditox-gui-1000.sock
INFO ditox_gui: Ditox GUI starting (daemon, action=Launch)

$ ditox-gui --hide
INFO ditox_gui: forwarded to running daemon action=Hide reply=OK

$ ditox-gui --show
INFO ditox_gui: forwarded to running daemon action=Show reply=OK

$ ditox-gui --toggle
INFO ditox_gui: forwarded to running daemon action=Toggle reply=OK

$ ditox-gui --quit
INFO ditox_gui: forwarded to running daemon action=Quit reply=OK
# (daemon exits, no longer in process list)
```

End-to-end: socket bound, all four IPC commands accepted,
quit-with-deferred-exit pattern works (reply received before
process death).

**Workspace test count after this session: 495 tests** (was 487;
+8 ipc unit tests). All clippy `-D warnings` + fmt clean.
