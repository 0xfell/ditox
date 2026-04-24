# Ditox Roadmap

> **Current Version:** 0.2.1

## Status Overview

| Category | Count |
|----------|-------|
| Completed | 10 |
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

### What's Working (v0.2.1)

**TUI (`ditox`):** Full feature set — list, search, copy, delete, pin, preview,
pagination, notes, stats, collections.

**GUI (`ditox-gui`):**
- **Windows:** system tray, Ctrl+Shift+V global hotkey, auto-start via
  registry, Win32 focus recovery for Win+D.
- **Linux (Wayland + X11):** system tray (StatusNotifierItem via
  libappindicator), `--toggle` / `--show` / `--hide` / `--quit` flags for
  compositor keybinds, XDG autostart, single-instance Unix-socket IPC,
  native window decorations.

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

**GUI CLI (`ditox-gui`):** `--toggle`, `--show`, `--hide`, `--quit`, `--help`,
`--version`.

### Performance (v0.2.1)

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
- GUI window state: `~/.local/share/ditox/window_state.json`
- GUI runtime lock/socket: `$XDG_RUNTIME_DIR/ditox-gui-$UID.{lock,sock}`
- GUI autostart: `~/.config/autostart/ditox-gui.desktop`

**Windows:**
- Config: `%APPDATA%/ditox/config.toml`
- Data: `%APPDATA%/ditox/`
- GUI window state: `%APPDATA%/ditox/window_state.json`
