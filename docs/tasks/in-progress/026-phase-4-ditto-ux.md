# Task: Phase 4 — Ditto UX replication (long-running GUI + layer-shell)

> **Status:** in-progress (8/12 sub-tasks done — 4.1, 4.2, 4.3, 4.4, 4.6, 4.7, 4.8, 4.10, 4.11 + 3.7 already covers 4.12)
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

### 2026-04-26 — sub-task 4.3 layer-shell dispatch landed

**4.3 — Layer-shell window on Linux.**

Path A1 from ADR 0001 wired in: `iced_layershell::build_pattern::application`
on Linux compositors that support `wlr-layer-shell`; existing
`iced::application` path retained for everything else (Windows,
macOS, GNOME Wayland, X11).

`Cargo.toml`:
- `iced_layershell = "=0.17.1"` workspace dep (pinned through
  Phase 4 to avoid the in-flight 0.18 beta churn; unpin in v0.5).
- Added to `ditox-gui`'s `cfg(unix)` deps. Pure Rust + already
  in lock; integrates cleanly with the rest of the iced stack.

`ditox-gui/src/app.rs`:
- `Message` enum gains `#[cfg_attr(unix, iced_layershell::to_layer_message)]`.
  The macro adds the layer-shell control variants (`AnchorChange`,
  `SizeChange`, `MarginChange`, `LayerChange`,
  `KeyboardInteractivityChange`, `SetInputRegion`,
  `AnchorSizeChange`, `VirtualKeyboardPressed`,
  `ExclusiveZoneChange`) and a `TryInto<LayershellCustomActionWithId>`
  impl. On non-Linux builds the macro still emits the variants so
  the same enum builds; iced never produces them, so they're
  harmless dead code.
- `update`'s match gains a `_ => {}` catch-all to satisfy
  exhaustiveness without acting on the synthesised control
  variants — iced_layershell dispatches them internally.
- Removed the bare `Result` import from `use ditox_core::{...}`
  because the macro expansion uses `Result<T, E>` (std's two-arg
  form) and `ditox_core::Result<T>` (one-arg) shadowed it. The
  one site that wanted `ditox_core::Result` (`run_with`) now
  qualifies it explicitly.
- New `run_layer_shell()` function (cfg=target_os="linux"):
  - `LayerShellSettings { anchor: Bottom | Left, layer: Top,
    size: Some((420, 520)), margin: (0, 0, 24, 24),
    keyboard_interactivity: Exclusive, exclusive_zone: -1,
    start_mode: Active, events_transparent: false }`.
  - Same `boot_app` (`(DitoxApp, Task<Message>)`) — works because
    `iced_layershell::application` accepts `IntoBoot`-shaped
    closures.
  - Same `subscription` and `view`.
  - Theme passed as `iced::Theme::Dark` (the trait impl on
    `iced::Theme` itself satisfies `ThemeFn<State, Theme>`).
  - Bootstrap font loaded via `.font(iced_fonts::BOOTSTRAP_FONT_BYTES)`.
- `run_with` dispatches: on Linux + `Platform::supports_layer_shell()`
  → `run_layer_shell`; else → existing iced path.

**Live verification on Hyprland 2026-04-26:**

```
INFO ditox_gui: ditox-gui daemon listening on IPC socket
INFO ditox_gui: Ditox GUI starting (daemon, action=Launch)
INFO ditox_gui: captured previous-foreground snapshot ...
INFO ditox_gui::app: starting iced_layershell window (wlr-layer-shell) platform=Linux(Hyprland { signature: ... })
```

Daemon binds IPC socket, detects Hyprland, dispatches to
`run_layer_shell`, layer-surface created without panic.

**Held back from this commit (rolling into Phase 4.4-4.12):**
- Configurable popup position (`[gui.position]` modes
  `at_caret` / `at_cursor` / `at_previous` / etc.) — sub-task 4.4.
- Custom non-client area drag handle for layer-shell — 4.5.
- Pin / always-on-top toggle — 4.6.
- Hide-on-blur grace + paste-and-stay — 4.8 + the existing
  `paste_and_exit` rework.
- Tooltip-as-preview — 4.9.
- Helper `--install-hyprland-config` — 4.11.

**Workspace test count after this session: 495 tests** (unchanged;
4.3 is wiring + cfg-gated dispatch with no new pure-logic surface
area). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 4.8 (hide-on-blur + paste-and-hide) landed

**4.8 — Hide-on-blur with grace; paste-and-hide.**

The defining behavioural change for the daemon model. The
launcher no longer exits on click / Esc / unfocus — it hides,
the daemon stays alive, and the next IPC summon reuses the
same process.

`ditox-gui/src/app.rs`:
- New `hide_window(&mut self) -> Task<Message>` helper. Sets
  `self.visible = false` and issues
  `iced::window::set_mode(id, Mode::Hidden)` if the main window's
  `Id` was captured (via `WindowOpened` subscription). When the
  Id isn't yet known (very early startup), returns `Task::none()`
  and logs at debug — Phase 4 polish will retry on the next
  iced tick.
- `paste_and_exit` renamed to `paste_and_hide`, return type
  changed from `-> !` (divergent) to `-> Task<Message>`. Body
  unchanged through clipboard-write / sentinel / focus-restore /
  keystroke-synth; final `process::exit(0)` becomes
  `self.hide_window()`.
- `Message::CopyEntry`, `Message::CopyFromPreview`, and the
  `Message::HideWindow` arm now `return self.paste_and_hide(entry)`
  / `return self.hide_window()` instead of `process::exit`.
- `Message::WindowUnfocused` (the actual hide-on-blur arm):
  preserves the existing 500 ms grace-window check (avoids the
  brief unfocus some compositors emit during animation), but on
  expiry calls `self.hide_window()` instead of exiting. Phase 4
  polish will surface the grace duration as
  `Config.gui.hide_on_blur_grace_ms` (sub-task 4.8 follow-up).
- Explicit Quit paths preserved as `process::exit(0)`:
  - `Message::QuitApp` (tray menu "Quit").
  - `Command::Quit` IPC handler.
  Both save_window_state then defer-and-exit so the user's
  intent ("quit the daemon") is honoured.

**Foreground refresh on every Show / Toggle.** New
`refresh_foreground_snapshot(&mut self)` helper. Called from
the IPC `Show` and `Toggle` (when going visible) handlers
**before** flipping `set_mode(Windowed)`. Without this, the
daemon's `previous_foreground` would stay frozen at the value
captured in `main.rs::run` at process start — meaning every
post-first-launch paste-back would target whichever app was
focused when the daemon started, not the one the user is
summoning from. The `ForegroundFilter` wrapping the tracker
already drops self-snapshots, so the daemon never picks up its
own window as the foreground.

`Message::Show` / `Toggle` (going visible): also reset
`self.last_show_time = Instant::now()` so the hide-on-blur
grace window starts fresh on each summon.

**Live verification deferred** — requires interactive layer-shell
testing (click → window hides + paste-back lands → re-summon →
new foreground tracked). User can run
`RUST_LOG=ditox_gui=info ./target/release/ditox-gui` then
`ditox-gui --toggle` from a different app to verify the round
trip.

**Workspace test count after this session: 495 tests** (still
unchanged; 4.8 is a behavioural refactor with no new
pure-logic surface area). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 4.11 landed

**4.11 — `--install-hyprland-config` helper.**

`ditox-gui/src/hyprland_config.rs` (new):
- `install() -> Result<PathBuf>`: writes
  `~/.config/hypr/conf.d/ditox.conf` (honouring `XDG_CONFIG_HOME`)
  with the snippet between `# >>> ditox-managed >>>` and
  `# <<< end ditox-managed <<<` markers. Atomic via tmp+rename.
  Idempotent — re-running overwrites only the managed block,
  preserving any user content outside the markers. Also creates
  an empty `ditox-binds.conf` placeholder so the snippet's
  `source = …` line doesn't break Hyprland on first reload
  (Phase 5 will populate that file with per-clip hotkeys).
- `uninstall() -> Result<bool>`: strips the managed block. If
  nothing else remains, removes the file; otherwise rewrites it
  atomically. `ditox-binds.conf` is left alone (the user may
  have customised it). Returns `true` iff something was removed.
- Snippet body matches the spec:
  ```text
  exec-once = ditox-gui --hide
  bind = CTRL, grave, exec, ditox-gui --toggle
  windowrulev2 = float, class:^(ditox-gui)$
  windowrulev2 = pin, class:^(ditox-gui)$
  windowrulev2 = noborder, class:^(ditox-gui)$
  windowrulev2 = noshadow, class:^(ditox-gui)$
  windowrulev2 = noanim, class:^(ditox-gui)$
  source = ~/.config/hypr/conf.d/ditox-binds.conf
  ```
- 8 unit tests covering: marker-aware strip happy path; no-marker
  passthrough; trailing-newline handling; only-begin / only-end
  marker rejection; full round-trip via `tempdir`; preservation
  of user content outside markers across re-install.

`ditox-gui/src/cli.rs`:
- `Action { …, InstallHyprlandConfig, UninstallHyprlandConfig }`
  variants. The Hyprland-specific actions never go over IPC —
  `action_to_wire` returns `None`. Mutually exclusive with the
  daemon control flags.
- `--install-hyprland-config` / `--uninstall-hyprland-config`
  long flags. Both conflict with `--toggle`/`--show`/`--hide`/
  `--quit` and with each other.

`ditox-gui/src/main.rs`:
- Helper actions handled BEFORE any IPC / lock / daemon work.
  Print the helpful "add this to hyprland.conf" message after a
  successful install; print the cleanup hint on uninstall.
- Per `H5`: the helper **never** auto-modifies `hyprland.conf`
  itself — only the conf.d files we own. The user copy-pastes
  the `source = …` line manually so they understand what's
  changing.

**Live verification on Hyprland 2026-04-26 with `HOME=/tmp/...`:**

```
$ ditox-gui --install-hyprland-config
Wrote: /tmp/.../.config/hypr/conf.d/ditox.conf
To activate, add this line to your hyprland.conf:
    source = ~/.config/hypr/conf.d/ditox.conf
Then reload:  hyprctl reload

$ cat /tmp/.../.config/hypr/conf.d/ditox.conf
# >>> ditox-managed (do not edit between these markers) >>>
exec-once = ditox-gui --hide
bind = CTRL, grave, exec, ditox-gui --toggle
windowrulev2 = float, class:^(ditox-gui)$
... (full snippet) ...
# <<< end ditox-managed <<<

$ ditox-gui --install-hyprland-config   # idempotent
Wrote: ...

$ ditox-gui --uninstall-hyprland-config
Removed ditox-managed snippet from ~/.config/hypr/conf.d/ditox.conf
If you no longer want the file at all, delete it manually:
    rm -i ~/.config/hypr/conf.d/ditox.conf

$ ls /tmp/.../.config/hypr/conf.d/
ditox-binds.conf                # ditox-binds.conf preserved

$ ditox-gui --uninstall-hyprland-config   # no-op on second run
No ditox-managed snippet found; nothing to remove.
```

End-to-end: install / re-install (idempotent) / uninstall /
re-uninstall (no-op) all behave correctly. Snippet body
verbatim matches the spec.

**Workspace test count after this session: 503 tests** (was 495;
+8 hyprland_config tests). All clippy `-D warnings` + fmt clean.

---

## Phase 4 close summary (4/12 sub-tasks landed)

The Phase 4 work shipped today brings the daemon model + IPC +
layer-shell + hide-on-blur + Hyprland helper online. Outstanding
sub-tasks (planned for follow-up commits / sessions):

- **4.4** Configurable popup position (`[gui.position]` modes).
- **4.5** Custom non-client area (drag handle in layer-shell).
- **4.6** Always-on-top toggle (pin button → `Layer::Top` ⇄ `Overlay`).
- **4.7** Modifier-held cycling (extends 2.9 cursor primitive).
- **4.9** Tooltip-as-preview (custom iced widget).
- **4.10** Inline list extras (glyphs + hotkey-number prefix; color
  swatch already done in 3.3).
- **4.12** Per-resolution window state — already covered by 3.7.

The MVP is functional: launching `ditox-gui` starts the daemon;
`ditox-gui --toggle` (or the helper-installed `Ctrl+~` keybind)
shows the layer-shell launcher; click → paste-back hides the
window; daemon stays alive for the next summon.
