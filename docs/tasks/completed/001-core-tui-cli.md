# Task: Core Implementation

> **Status:** completed
> **Priority:** high
> **Created:** 2024-10
> **Completed:** 2024-11

## Description

Initial implementation of Ditox clipboard manager with TUI, watcher daemon, basic CLI, and NixOS integration.

## Requirements

- [x] SQLite database with SHA256 deduplication
- [x] Wayland clipboard monitoring (wl-clipboard-rs)
- [x] Image capture (PNG, JPEG, GIF, BMP, WebP)
- [x] TUI with Ratatui
- [x] Fuzzy search (nucleo-matcher)
- [x] Image preview (Kitty, iTerm2, Sixel protocols)
- [x] Entry pinning
- [x] Basic CLI commands (watch, list, copy, clear, status)
- [x] Nix flake with Home Manager module
- [x] Systemd user service

## Implementation Notes

### Architecture
- All interfaces access SQLite directly (no IPC)
- CLI commands in `main.rs` (no separate `commands.rs`)
- Search operations in `app.rs` (no separate `core.rs`)

### Key Files
- `src/main.rs` - Entry point, CLI handlers
- `src/watcher.rs` - Clipboard polling daemon
- `src/app.rs` - TUI state and search
- `src/db.rs` - SQLite operations
- `src/ui/` - Ratatui widgets

## Testing

```bash
cargo test                    # All tests
cargo test --test cli_tests   # CLI E2E tests
cargo run -- watch &          # Start watcher
cargo run                     # Launch TUI
```

## Work Log

### 2024-11
- Initial release v0.1.0 through v0.1.5
- Core watcher, TUI, basic CLI
- Image preview support
- NixOS Home Manager module
