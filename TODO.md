# Ditox Fresh-Start TODO

This file tracks what remains after the initial OpenTUI + Zig scaffold.

## 1. Make The MVP Usable End-To-End

Highest priority: complete the core workflow.

- Fix `ditox launch` so it captures the active Hyprland window before opening the TUI.
- Pass the captured target window into the TUI process, likely through `DITOX_TARGET_WINDOW`.
- Make `Enter` in the TUI copy the selected entry and paste it into the previous app.
- Keep `Ctrl+Y` as copy-only without paste-back.
- Verify the full Hyprland loop:
  - user presses compositor keybind.
  - terminal opens with Ditox TUI.
  - user selects an entry.
  - Ditox writes it with `wl-copy`.
  - Ditox refocuses the original Hyprland window.
  - Ditox dispatches `hyprctl dispatch sendshortcut "CTRL,V,"`.
  - TUI exits or shows a clear result.

## 2. Polish The OpenTUI Interface

Initial visual polish is implemented:

- Centralized semantic theme tokens with dark and light variants.
- Pager-style UI configuration for compact mode, panel sizing, metadata, preview length, and scrollbar visibility.
- Structured OpenTUI components for shell, header, history list, preview pane, status line, and overlays.
- Improved layout sizing, list scrolling, row truncation, preview behavior, and empty states.
- Semantic status colors and clearer runtime error messages for `ditoxd`, `wl-copy`, `hyprctl`, and paste failures.
- `@opentui/keymap` is the default key handling layer.
- Search, help, delete, and clear confirmations now share overlay styling.
- Presentation helpers have direct Bun test coverage.

Remaining polish:

- Add user-configurable keybindings after the default keymap stabilizes.
- Add terminal snapshot or PTY-level visual regression coverage when the TUI test harness exists.

## 3. Backend Correctness

- Add explicit schema versioning.
- Add migrations instead of assuming a brand-new database forever.
- Use real SQLite FTS5 queries for search instead of current `LIKE` matching.
- Add method-specific JSON schemas for RPC params/results.
- Keep JSON Schema as the contract source and regenerate TypeScript types.
- Add temp-database integration tests for:
  - add.
  - list.
  - search.
  - get.
  - copy mock path.
  - delete.
  - favorite/unfavorite.
  - clear text/images/all.
  - repair.

## 4. Clipboard Watcher

Current watcher is text-only and basic polling.

- Add image-first capture priority.
- Detect available MIME types from `wl-paste`.
- Avoid recapturing Ditox's own clipboard writes.
- Add excluded app/window rules.
- Add pause/resume state.
- Add watcher health/status that reflects actual runtime state.
- Decide whether watcher stays a long-running process only, or becomes part of a socket daemon.

## 5. Image Support

First implementation should remain metadata-first.

- Capture image data from Wayland.
- Hash image bytes.
- Store blobs content-addressed under `images-v2`.
- Write blobs atomically: temp file, fsync, rename, fsync parent.
- Store metadata in SQLite:
  - MIME.
  - byte length.
  - hash.
  - blob path.
  - dimensions when available.
- Show image metadata in the TUI preview.
- Defer Kitty/Sixel image rendering until metadata capture is solid.

## 6. Clipse-Like Power Features

- Multi-select entries in the TUI.
- Bulk copy/output selected entries.
- Better pin/favorite workflow.
- Clear text, clear images, and clear all from the TUI with confirmation.
- Add operational CLI parity:
  - `ditox add`.
  - `ditox copy`.
  - `ditox print`.
  - `ditox clear`.
  - `ditox pause`.
  - `ditox repair`.
  - `ditox status`.
  - `ditox launch`.
- Add export/output commands once storage is stable.

## 7. Packaging And Dev Ergonomics

- Make `ditox tui` work outside the repo checkout.
- Decide TUI shipping format:
  - Bun source executed by `bun`.
  - bundled JS.
  - compiled Bun executable.
- Add a single check command that runs:
  - `zig build test`.
  - TypeScript typecheck.
  - Bun tests.
  - TUI build.
- Add install/dev scripts.
- Keep `.envrc` + Nix dev shell working as the primary local development path.
- Keep native commands usable for contributors who do not use Nix.

## Recommended Next Step

Implement the Hyprland launch + paste-back loop first.

That turns the scaffold into the real workflow: press keybind, open TUI, pick history item, paste into the previous app.
