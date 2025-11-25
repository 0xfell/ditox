# Architecture Notes

## Design Decisions

### No IPC Between Components
All interfaces (TUI, CLI, watcher) access SQLite directly. This simplifies the architecture and ensures consistency.

### CLI Handlers in main.rs
Originally planned `commands.rs` and `core.rs` modules, but kept things simpler by:
- CLI handlers directly in `main.rs`
- Search operations in `app.rs`

### Clipboard Priority
`watcher.rs` checks images first (PNG, JPEG, etc.), then text. This ensures "Copy image" from browsers captures the image, not the URL.

### Content Sanitization
`entry.rs::sanitized_content()` strips ANSI escapes and control characters before TUI display to prevent rendering issues.

## Module Overview

```
src/
├── main.rs        # Entry point + CLI handlers
├── cli.rs         # Clap command definitions
├── app.rs         # TUI state + search
├── watcher.rs     # Clipboard polling daemon
├── db.rs          # SQLite CRUD
├── entry.rs       # Entry model
├── config.rs      # Config loading
├── clipboard.rs   # Wayland clipboard interface
├── error.rs       # Error types
└── ui/            # Ratatui widgets
```

## Data Flow

```
Clipboard → watcher.rs → db.rs → SQLite
                                    ↑
TUI (app.rs) ──────────────────────┘
CLI (main.rs) ─────────────────────┘
```
