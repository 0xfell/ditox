# Task: Floating-launcher GUI redesign (v0.3.1)

> **Status:** completed
> **Priority:** high
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

Pivot the GUI from a "long-lived app with show/hide" model to a one-shot
floating launcher that opens at the bottom-left corner, copies on click,
and exits as soon as the user is done.

## Background

The previous model relied on `iced::window::set_mode(Hidden)` to hide the
window on copy / Esc / focus-loss, then re-show it on `--toggle` from a
compositor keybind. Empirically this does not work on Wayland (Hyprland in
particular): `winit::Window::set_visible(false)` is a no-op for an already
mapped Wayland surface, and `xdg_toplevel.set_minimized` is a hint the
compositor freely ignores. The window stayed mapped after every "hide",
which made the GUI feel broken.

Rather than fight Wayland, we pivot the UX: each `ditox-gui` invocation is
its own short-lived process. SUPER+V launches a fresh window; copying or
losing focus exits.

## Requirements

- [x] Each launch opens a fresh window; copy / Esc / unfocus / close-button
      all exit the process via `std::process::exit(0)`.
- [x] Window is a compact floating panel (420×520) anchored 20px from the
      bottom-left corner of the active monitor.
- [x] Saved `window_state.json` size/position is ignored (still saved for
      telemetry).
- [x] Image entries copy on click, same as text. The previous "click image
      to preview" behaviour is gone.
- [x] Tab toggles a side inspector panel for the focused entry — text or
      image — without copying. Tab again or Esc closes it.
- [x] Status bar reads `"{n} items · Press Tab to preview · vX.Y.Z"`.
- [x] Single-instance lock and IPC server are dropped; concurrent launches
      are independent processes.
- [x] `--toggle` / `--show` / `--hide` flags kept as no-op compatibility
      shims so existing keybinds keep working; `--quit` early-exits.

## Implementation Notes

### Files changed

- `ditox-gui/src/app.rs` — main edits:
  - `Message::CopyEntry`, `Message::CopyFromPreview`, `Message::HideWindow`,
    `Message::WindowUnfocused` → `std::process::exit(0)` after
    `save_window_state`. The `HideWindow` arm still pops a non-Main view
    mode (Settings/Help/Panel) before exiting, so Esc keeps its
    "close-overlay-first" semantics.
  - Renamed `ViewMode::ImagePreview(_)` → `ViewMode::EntryPanel(_)` and
    rewrote `view_image_preview` as `view_entry_panel`; the panel now
    handles text entries too and renders side-by-side with the entry list
    (`row![main.width(FillPortion(3)), panel.width(FillPortion(2))]`).
  - Replaced `Position::Specific(saved_xy)` with `Position::SpecificWith`
    that anchors to bottom-left using a closure; added a
    `FLOATING_MARGIN = 20.0` constant.
  - Image entry click switched from `Message::ShowImagePreview` to
    `Message::CopyEntry(index)`.
  - Tab key switched from `NextTab` / `PrevTab` to `ToggleEntryPanel(_)`;
    the closure captures nothing (subscription closures must be `Fn` and
    iced reuses them across rebuilds), so we route through a new
    `Message::ToggleSelectedEntryPanel` that resolves the selection in the
    `update` handler. Tab navigation moved to `Shift+Left`/`Shift+Right`.
  - Status bar text rewritten per the floating-launcher mock.
  - Win32-only helpers (`force_restore_window`, `remove_topmost`,
    `is_window_actually_visible`, `delayed_force_focus`) marked
    `#[allow(dead_code)]` since they're no longer wired into `update`.
  - `Message::IpcShow`, `Message::ForceWindowFocus` deleted.
  - IPC subscription removed from `subscription()`.

- `ditox-gui/src/main.rs` — dropped `mod ipc`, `mod ipc_bridge`, the
  single-instance lock, and the IPC client/server bring-up. `--quit` now
  no-ops with a log line; `--toggle`/`--show`/`--hide` are accepted but
  ignored (each launch is independent).

- `ditox-gui/src/ipc.rs`, `ditox-gui/src/ipc_bridge.rs` — deleted.

- `ditox-gui/src/cli.rs` — `Action::wire()` kept behind `#[allow(dead_code)]`.

- `Cargo.toml`, `nix/package.nix`, `ditox-gui/installer/setup.iss` — version
  bump 0.3.0 → 0.3.1.

### Behavioural notes

- `WindowUnfocused` exits only after a 500 ms grace period from
  `last_show_time` (set in `DitoxApp::new`) so a brief unfocus during
  initial map doesn't kill us before the user can interact.
- iced 0.14 `Settings::exit_on_close_request` defaults to `true`, so
  clicking the compositor's close button exits cleanly with no extra
  wiring.
- The single-instance lock removal means double-pressing SUPER+V can spawn
  two processes momentarily; the second window will appear behind the
  first and unfocus → exit on its own. Acceptable trade-off vs. the
  complexity of the lock.

## Testing

Local verification (NixOS host, x86_64-linux):

```sh
nix develop --command rustup run stable cargo fmt --all
nix develop --command rustup run stable cargo clippy --workspace \
    --all-targets --locked -- -D warnings
nix develop --command rustup run stable cargo test --workspace --locked
nix build .#default
./result/bin/ditox-gui --help
```

All four pass. The release pipeline (CI) is what we trust for cross-target
coverage (Linux musl, AppImage, aarch64, Windows MSVC, Nix package).

## Work Log

### 2026-04-26
- Confirmed `Position::SpecificWith` lands the window correctly at
  bottom-left on Hyprland; iced 0.14 passes the focused-monitor size.
- Confirmed clippy 1.95 + rustfmt + workspace tests are clean.
- Bumped version to 0.3.1, regenerated `Cargo.lock`.
- Dropped `ipc.rs` / `ipc_bridge.rs` and the single-instance forwarding
  path in `main.rs`; verified the GUI still launches with `--toggle`,
  `--show`, `--hide`, `--quit` flags (all no-ops now).
