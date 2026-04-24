# Task: Linux GUI

> **Status:** completed
> **Priority:** high
> **Created:** 2026-04-24
> **Completed:** 2026-04-24

## Description

Extend `ditox-gui` (previously Windows-only) to run on Linux/Wayland. Ship a
`ditox-gui` binary that behaves as its own summon tool: the user binds
`ditox-gui --toggle` to a compositor keybind and repeated invocations
show/hide the same window through a Unix-socket IPC.

## Requirements

- [x] Single codebase `ditox-gui`; Windows-specific bits stay `#[cfg(windows)]`
- [x] `--toggle` / `--show` / `--hide` / `--quit` CLI flags
- [x] Single-instance lock + IPC socket at `$XDG_RUNTIME_DIR/ditox-gui-$UID.{lock,sock}`
- [x] System tray via `tray-icon` on Linux (GTK event loop runs on a helper thread)
- [x] XDG autostart (`~/.config/autostart/ditox-gui.desktop`) toggled from the tray
- [x] Native window decorations on Linux (no custom title bar)
- [x] `--hide` starts the window hidden (for autostart)
- [x] Nix flake + package.nix build both `ditox` and `ditox-gui`

## Non-goals (explicit)

- Global hotkey on Wayland — portable support doesn't exist; the compositor
  keybind + `--toggle` is the intended UX. `global-hotkey` remains Windows-only.
- Paste-to-previous-window on Wayland — out of scope for v1.
- macOS support — platform gating leaves a stubbed path but no real
  implementation. Not built or tested.

## Implementation Notes

### Module layout

New files in `ditox-gui/src/`:

| File | Purpose |
|------|---------|
| `cli.rs` | `clap` definitions + the `Action` enum (Launch/Toggle/Show/Hide/Quit) |
| `ipc.rs` | Single-instance lock + Unix socket server/client (Windows: stub) |
| `ipc_bridge.rs` | Static mpsc so iced 0.14's `Fn` subscription boot closure can drain IPC commands |

`startup.rs` now has two backends: Windows registry (unchanged) and Linux XDG
autostart (new).

### IPC protocol

One line per message over the Unix socket:

```
TOGGLE\n       ; server replies OK\n
SHOW\n
HIDE\n
QUIT\n
```

The lock file is opened with `flock(LOCK_EX | LOCK_NB)` and released when the
`InstanceLock` guard in `main::run` drops.

### Tray thread (Linux)

`tray-icon` 0.22 requires a GTK event loop on the same thread that owns the
`TrayIcon`. iced/winit doesn't run GTK, so we spawn a dedicated `ditox-tray`
thread that calls `gtk::init()`, builds the tray, and runs `gtk::main()`. The
menu events reach the iced app through `MenuEvent::receiver()` (already
polled by the existing subscription) — so the cross-platform event wiring
didn't change.

### Global hotkey removal on Linux

`global-hotkey` is now gated in `ditox-gui/Cargo.toml` to `cfg(windows)`
only, and the subscription that polls `GlobalHotKeyEvent::receiver()` is
`#[cfg(windows)]`. The `Message::GlobalHotkeyPressed` variant is also gated.

### Decorations

Windows keeps `decorations = false` (custom dark title bar); Linux sets
`decorations = true` so the compositor draws the title bar using its theme.
The custom in-app title bar widgets still render — cleaning those up on
Linux is a follow-up polish task.

### Nix

`flake.nix` dev shell and `nix/package.nix` gained:

- Build-time: `gtk3`, `glib`, `cairo`, `pango`, `atk`, `gdk-pixbuf`,
  `libappindicator-gtk3`, `libayatana-appindicator`, `libdbusmenu-gtk3`,
  `xdotool`, `wayland`, `libxkbcommon`, `fontconfig`, `freetype`, `expat`,
  `chafa` (new — ratatui-image 10), `vulkan-loader`, `libGL`, X11 libs.
- Runtime: `makeWrapper` prefixes `LD_LIBRARY_PATH` on the installed
  binaries so vulkan/libGL/appindicator/etc. dlopen succeeds on pure NixOS.

`apps.ditox-gui` exposes `nix run .#ditox-gui`.

## Testing

### Manual

```
# Autostart-style launch
ditox-gui --hide &

# Summon
ditox-gui --toggle     # shows the window
ditox-gui --toggle     # hides it
ditox-gui --show       # always show
ditox-gui --quit       # tell running instance to exit
```

Smoke tested on Hyprland (Wayland, `nix build` output). IPC round-trip
logs confirmed in `/tmp/bg.log`:

```
ToggleWindow: self.visible=false, actually_visible=true
Window is hidden or not foreground, showing it
ToggleWindow: self.visible=true,  actually_visible=true
Window is visible, hiding it
```

### Automated

Existing `cargo test --workspace` still passes (21 tests across crates).

## Work Log

### 2026-04-24

- Updated all workspace deps to their latest versions (see chore commit).
  Fixed ratatui 0.30 `Terminal::draw` error type and ratatui-image 10's
  deprecation of `Picker::from_fontsize` (still used under
  `#[allow(deprecated)]` because no public replacement accepts a caller
  font size).
- Split `startup.rs` into Windows (auto-launch registry) and Linux (XDG
  autostart) backends behind a shared public API.
- Added `cli.rs` with clap-derived arg parsing and a cross-platform
  `Action` enum.
- Added `ipc.rs` with flock-based single-instance coordination and a
  Unix-socket server. Windows has a stub that always reports "first
  instance". Lock/socket go in `$XDG_RUNTIME_DIR` (fallback `/tmp`) with
  a per-UID suffix.
- Added `ipc_bridge.rs` — a global mpsc between the IPC server thread
  and the iced subscription (iced 0.14's `Fn` boot requirement forbids
  moving a `Receiver` into the closure).
- Gated `global_hotkey` imports/usage and `Message::GlobalHotkeyPressed`
  behind `#[cfg(windows)]`. Wired a new `ipc_sub` subscription that
  drains commands into iced `Message`s.
- Added `Message::IpcShow` for `--show` semantics (forces show even if
  already visible).
- Added `start_hidden` to `DitoxApp::new` and set `settings.visible`
  accordingly, plus expose `run_with(db, config, start_hidden)`.
- Native decorations on non-Windows (iced `settings.decorations = true`).
- Linux tray: dedicated `ditox-tray` thread runs `gtk::init()` + owns the
  `TrayIcon` + calls `gtk::main()`. Menu events still flow through the
  global `MenuEvent::receiver()` used by the Windows path.
- Updated `nix/package.nix` with GTK/appindicator/wayland/vulkan/chafa
  deps and an `LD_LIBRARY_PATH` wrapper via `makeWrapper`.
- `flake.nix` devShell gained the same inputs and a runtime-library
  `LD_LIBRARY_PATH`. Added `apps.ditox-gui`.
- End-to-end smoke tested on Hyprland using both the `cargo build
  --release` binary and the `nix build` output.
