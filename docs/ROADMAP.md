# Ditox Roadmap

> **Current Version:** 0.3.1

## Status Overview

| Category | Count |
|----------|-------|
| Completed | 13 |
| In Progress | 0 |
| Planned | 0 |

---

## In Progress

(None)

---

## Planned

(None)

---

## Recently Completed

| Task | Date | Description |
|------|------|-------------|
| [Floating-launcher GUI redesign](tasks/completed/013-floating-launcher-redesign.md) | 2026-04-26 | One-shot GUI: each launch opens a 420×520 floating panel at bottom-left; copy/Esc/unfocus/close exits the process. Replaces the broken Wayland hide/show model. Tab key opens a side inspector panel for text & image entries. Versions bumped to 0.3.1. |
| [Release Infrastructure](tasks/completed/012-release-infra.md) | 2026-04-25 | CI + release workflows (GitHub Actions), prebuilt Linux/Windows binaries (TUI tarball, musl static, AppImage, Windows zip), Cachix push, README rewrite, versions bumped to 0.3.0 |
| [Image Storage Bug Fix](tasks/completed/011-image-storage-bug.md) | 2026-04-25 | Content-addressed image store, refcount prune queue, schema v1 migration, `ditox repair` command. Fixes 4 disk-leak bugs. |
| [Linux GUI](tasks/completed/010-linux-gui.md) | 2026-04-24 | Cross-platform `ditox-gui` (Wayland/X11) with tray, `--toggle` IPC, XDG autostart |
| [Delete Confirmation in TUI](tasks/completed/009-delete-confirmation-tui.md) | 2025-12-02 | Add confirmation dialogs for delete operations (`d` and `D`) |
| [TUI Pagination](tasks/completed/005-tui-pagination.md) | 2025-11-27 | Lazy loading & pagination for 126x faster startup, 500x memory reduction |
| [TUI Polish & Refinements](tasks/completed/008-tui-polish.md) | 2025-11-27 | Entry type icons, line numbers, terminal size handling, message timeout, auto-help |
| [Feature Bundle Implementation](tasks/completed/007-feature-bundle-implementation.md) | 2025-11-27 | Implementation of 10 selected features (notes, stats, collections, etc.) |
| [Feature Ideas Brainstorm](tasks/completed/006-feature-ideas-brainstorm.md) | 2025-11-27 | 20 feature ideas for future development |
| [TUI UI Improvements](tasks/completed/004-tui-ui-improvements.md) | 2025-11-27 | UI/UX enhancements: scrollbar, mouse support, multi-select, search highlighting |
| [Tab Crash in Ghostty](tasks/completed/003-tab-crash-ghostty.md) | 2025-11-27 | Fix Tab key crash when using Ghostty terminal |
| [CLI Parity](tasks/completed/002-cli-parity.md) | 2024-11-27 | Add missing CLI commands (get, search, delete, pin, count) |
| [Core Implementation](tasks/completed/001-core-tui-cli.md) | 2024-11 | Initial TUI, watcher, basic CLI, NixOS integration |

---

## Quick Reference

### What's Working (v0.3.1)

**TUI (`ditox`):** Full feature set — list, search, copy, delete, pin, preview,
pagination, notes, stats, collections.

**GUI (`ditox-gui`):** One-shot floating launcher (420×520, bottom-left).
Each launch opens a fresh window; click an entry / Enter / Esc / unfocus
exits the process. Tab opens a side inspector panel.
- **Windows:** system tray, Ctrl+Shift+V global hotkey, auto-start via
  registry, Win32 focus recovery for Win+D.
- **Linux (Wayland + X11):** system tray (StatusNotifierItem via
  libappindicator); compositor keybind launches a fresh process per press.
  `--toggle` / `--show` / `--hide` retained as no-op compatibility shims.

**CLI (`ditox`):**
- `ditox` — TUI
- `ditox watch` — Watcher daemon
- `ditox list [--limit N] [--json] [--pinned]`
- `ditox get <target> [--json]` — Get full content
- `ditox search <query> [--limit N] [--json]` — Fuzzy search
- `ditox copy <target>` — Copy to clipboard
- `ditox delete <target>` — Delete entry
- `ditox favorite <target>` — Toggle favorite
- `ditox count` — Print entry count
- `ditox clear [--confirm]` — Clear history
- `ditox status` — Show status
- `ditox stats` — Show usage statistics
- `ditox collection …` — Manage collections
- `ditox repair [--dry-run] [--fix-hashes]` — Reconcile image store with DB

**GUI CLI (`ditox-gui`):** `--toggle`, `--show`, `--hide`, `--quit`, `--help`,
`--version`.

### Performance (v0.3.0)

| Metric | Result |
|--------|--------|
| First page load (10k entries) | 0.19ms |
| Startup speedup | 126.8x faster |
| Memory reduction | ~500x |
| Page navigation | ~0.25ms/page |
| Search (10k entries) | <2ms |

### File Locations

**Linux:**
- Tasks: `docs/tasks/{completed,in-progress,planned}/`
- Notes: `docs/notes/`
- Config: `~/.config/ditox/config.toml`
- Data: `~/.local/share/ditox/`
- GUI window state: `~/.local/share/ditox/window_state.json` (saved for
  telemetry; size/position are forced to 420×520 bottom-left at boot)
- GUI autostart: `~/.config/autostart/ditox-gui.desktop` (no longer
  required for keybind summon — bind directly to `ditox-gui`)

**Windows:**
- Config: `%APPDATA%/ditox/config.toml`
- Data: `%APPDATA%/ditox/`
- GUI window state: `%APPDATA%/ditox/window_state.json`
