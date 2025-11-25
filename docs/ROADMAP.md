# Ditox Roadmap

> **Current Version:** 0.1.10

## Status Overview

| Category | Count |
|----------|-------|
| Completed | 9 |
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

### What's Working (v0.1.10)

**TUI:** Full feature set - list, search, copy, delete, pin, preview, pagination, notes, stats, collections

**CLI:**
- `ditox` - TUI
- `ditox watch` - Watcher daemon
- `ditox list [--limit N] [--json] [--pinned]`
- `ditox get <target> [--json]` - Get full content
- `ditox search <query> [--limit N] [--json]` - Fuzzy search
- `ditox copy <target>` - Copy to clipboard
- `ditox delete <target>` - Delete entry
- `ditox pin <target>` - Toggle pin status
- `ditox count` - Print entry count
- `ditox clear [--confirm]` - Clear history
- `ditox status` - Show status
- `ditox stats` - Show usage statistics

### Performance (v0.1.10)

| Metric | Result |
|--------|--------|
| First page load (10k entries) | 0.19ms |
| Startup speedup | 126.8x faster |
| Memory reduction | ~500x |
| Page navigation | ~0.25ms/page |
| Search (10k entries) | <2ms |

### File Locations

- Tasks: `docs/tasks/{completed,in-progress,planned}/`
- Notes: `docs/notes/`
- Config: `~/.config/ditox/config.toml`
- Data: `~/.local/share/ditox/`
