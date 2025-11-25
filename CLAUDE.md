# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Task & Roadmap Workflow

Documentation lives in `/docs`. After completing work, update the relevant files:

### Structure
```
docs/
├── ROADMAP.md              # Index/summary - update counts and links
├── tasks/
│   ├── TEMPLATE.md         # Copy this for new tasks
│   ├── completed/          # Done tasks (move here when finished)
│   ├── in-progress/        # Currently working on
│   └── planned/            # Backlog
└── notes/                  # Architecture decisions, testing discoveries
```

### When Starting a Task
1. Create task file from `TEMPLATE.md` or move existing from `planned/` to `in-progress/`
2. Update `docs/ROADMAP.md` status table

### When Completing a Task
1. **Verify tests pass** (`cargo test`) before marking complete
   - If tests fail due to intentional behavior changes, update the tests
   - If tests no longer make sense after refactoring, remove or rewrite them
2. **For new features**: Decide if tests are needed based on:
   - Complexity (simple CLI flag? probably no. New DB logic? yes)
   - Risk of regression
   - If unsure, ask the user
3. Move task file from `in-progress/` to `completed/`
4. Update task file: set status to `completed`, add completion date, fill work log
5. Update `docs/ROADMAP.md`: move to "Recently Completed", update counts

### When Discovering Issues
Add to `docs/notes/` or the relevant task's work log

### Task File Naming
`NNN-short-description.md` (e.g., `003-image-paste.md`)

## Build & Test Commands

```bash
# Build
cargo build                              # Debug build (all crates)
cargo build --release                    # Optimized release build
cargo build -p ditox-tui                 # Build TUI only
cargo build -p ditox-gui                 # Build GUI only

# Test
cargo test                               # Run all tests
cargo test --test cli_tests              # Run specific test file
cargo test test_name                     # Run single test by name
cargo test -p ditox-core                 # Test core library only

# Run
cargo run -p ditox-tui                   # TUI mode
cargo run -p ditox-tui -- watch          # Daemon mode (Linux)
cargo run -p ditox-tui -- list --json    # CLI commands
cargo run -p ditox-gui                   # Windows GUI

# Development with Nix (Linux only)
nix develop                              # Enter dev shell
nix build                                # Build via Nix
```

## Architecture

Ditox is a cross-platform clipboard manager (Linux/Wayland + Windows) with a workspace structure:

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontends                                │
├──────────────────┬──────────────────┬──────────────────────────┤
│  ditox-tui       │  ditox-gui       │  ditox-tui (CLI)         │
│  (Linux TUI)     │  (Windows GUI)   │  (both platforms)        │
│  Ratatui+Crossterm  Iced+tray-icon  │  Clap commands           │
└────────┬─────────┴────────┬─────────┴────────┬─────────────────┘
         │                  │                  │
         └──────────────────┼──────────────────┘
                            │
              ┌─────────────▼─────────────┐
              │       ditox-core          │
              │  - db.rs (SQLite)         │
              │  - entry.rs (model)       │
              │  - clipboard.rs (platform)│
              │  - watcher.rs (daemon)    │
              │  - config.rs              │
              └─────────────┬─────────────┘
                            │
              ┌─────────────▼─────────────┐
              │  Platform-specific data:  │
              │  Linux: ~/.local/share/   │
              │  Windows: %APPDATA%       │
              └───────────────────────────┘
```

**No IPC needed** - all interfaces access SQLite directly.

## Workspace Crates

| Crate | Binary | Purpose |
|-------|--------|---------|
| `ditox-core` | (library) | Shared business logic, DB, clipboard abstraction |
| `ditox-tui` | `ditox` | Terminal UI + CLI + watcher daemon |
| `ditox-gui` | `ditox-gui` | Windows GUI with system tray and global hotkey |

## Key Modules (ditox-core)

| Module | Purpose |
|--------|---------|
| `db.rs` | SQLite CRUD with rusqlite, collections support |
| `entry.rs` | Entry model with `sanitized_content()` for safe display |
| `clipboard.rs` | Platform abstraction: `wl-clipboard-rs` (Linux) / `arboard` (Windows) |
| `watcher.rs` | Clipboard polling daemon, SHA256 deduplication |
| `collection.rs` | Named collections for organizing entries |
| `config.rs` | TOML config loading |
| `app.rs` | Shared `TabFilter` enum (All, Text, Images, Favorites, Today) used by both TUI and GUI |

## GUI-Specific Details (ditox-gui)

The Windows GUI uses the Iced framework with several Windows-specific integrations:

**Key Dependencies:**
- `iced` - Main GUI framework with custom dark theme
- `tray-icon` - System tray with menu (Show, Run on Startup, Quit)
- `global-hotkey` - Ctrl+Shift+V hotkey registration
- `windows` crate - Direct Win32 API for window focus management
- `iced-fonts` - Bootstrap Icons for UI elements

**Window Management:**
- Custom borderless window with draggable title bar and resize grip
- Win32 `SetForegroundWindow`, `SetWindowPos` for reliable focus after Win+D
- Window position/size persisted to `window_state.json`
- `TOPMOST` flag management to appear above desktop after Win+D

**Architecture Patterns:**
- `OnceLock` statics for clipboard watcher and config (Iced 0.14 requires `Fn` boot closure)
- Image thumbnail cache (`HashMap<String, Handle>`) to avoid reloading on every render
- Delayed focus task to avoid capturing "V" from Ctrl+Shift+V hotkey

**Build:**
- `build.rs` embeds Windows manifest for proper DPI handling

## TUI Module Structure (ditox-tui)

The TUI is organized into modular UI components under `src/ui/`:

| Module | Purpose |
|--------|---------|
| `mod.rs` | Main TUI app loop, event handling, state management |
| `list.rs` | Entry list rendering with selection |
| `preview.rs` | Image preview using ratatui-image protocols |
| `search.rs` | Fuzzy search input |
| `tabs.rs` | Tab bar (All, Text, Images, Favorites, Today) |
| `theme.rs` | Color palette and styling |
| `help.rs` | Help overlay with keybindings |
| `confirm.rs` | Confirmation dialog for destructive actions |

## Platform-Specific Code

Uses conditional compilation (`#[cfg(unix)]` / `#[cfg(windows)]`):

- **clipboard.rs**: `wl-clipboard-rs` on Linux, `arboard` on Windows
- **watcher.rs**: `libc::kill()` on Unix, `sysinfo` on Windows for process checking
- **GUI app.rs**: Win32 APIs for window management (Windows only)

## Important Patterns

**Clipboard priority**: Watcher checks images first (PNG, JPEG, etc.), then text. This ensures "Copy image" from browsers captures the image, not the URL.

**Content sanitization**: `entry.rs::sanitized_content()` strips ANSI escapes and control characters before display.

**Deduplication**: All entries are SHA256 hashed. Watcher calls `db.exists_by_hash()` before inserting.

**Image handling**: Images saved to data dir `/images/` with format `{timestamp}_{hash_prefix}.{ext}`.

## Test Structure

- `tests/cli_tests.rs` - End-to-end CLI testing via `assert_cmd`
- `tests/db_tests.rs` - Database operations
- `tests/entry_tests.rs` - Entry model, hashing, sanitization
- `tests/clipboard_tests.rs` - Mock-based clipboard priority tests

Tests use `tempfile` for isolated temp directories. Note: CLI tests use `XDG_DATA_HOME` which is Linux-specific.

## Data Locations

**Linux:**
- Database: `~/.local/share/ditox/ditox.db`
- Images: `~/.local/share/ditox/images/`
- Config: `~/.config/ditox/config.toml`

**Windows:**
- Database: `%APPDATA%/ditox/ditox.db`
- Images: `%APPDATA%/ditox/images/`
- Config: `%APPDATA%/ditox/config.toml`
