# Ditox Implementation Plan

## Overview

This document outlines the step-by-step implementation plan for Ditox, a terminal-based clipboard manager for NixOS/Wayland.

**Target:** Minimal viable product (v0.1) with core watcher + TUI + NixOS module

---

## Phase 1: Project Setup

### 1.1 Initialize Rust Project

```bash
cargo init
```

**Cargo.toml:**
```toml
[package]
name = "ditox"
version = "0.1.0"
edition = "2021"
description = "Terminal clipboard manager for Wayland"
license = "MIT"
repository = "https://github.com/user/ditox"

[dependencies]
# TUI
ratatui = "0.29"
crossterm = "0.28"

# Clipboard (Wayland)
wl-clipboard-rs = "0.8"

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Serialization
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Utilities
sha2 = "0.10"
uuid = { version = "1", features = ["v4"] }
directories = "5"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"

# Search
nucleo-matcher = "0.3"

# CLI
clap = { version = "4", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
strip = true
codegen-units = 1
```

### 1.2 Create Directory Structure

```
src/
├── main.rs
├── cli.rs
├── config.rs
├── db.rs
├── entry.rs
├── error.rs
├── clipboard.rs
├── watcher.rs
├── app.rs
└── ui/
    ├── mod.rs
    ├── layout.rs
    ├── list.rs
    ├── preview.rs
    ├── search.rs
    ├── help.rs
    └── theme.rs
```

### 1.3 Setup Nix Flake

Create `flake.nix` and `nix/` directory (copy from PRD).

**Deliverables:**
- [ ] `Cargo.toml` with all dependencies
- [ ] Directory structure created
- [ ] `flake.nix` with dev shell working
- [ ] `nix run` builds successfully (even if binary does nothing)

---

## Phase 2: Core Data Layer

### 2.1 Error Handling (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DitoxError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DitoxError>;
```

### 2.2 Entry Model (`src/entry.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryType {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub entry_type: EntryType,
    pub content: String,      // Text content or image path
    pub hash: String,
    pub byte_size: usize,
    pub created_at: DateTime<Utc>,
    pub pinned: bool,
}

impl Entry {
    pub fn new_text(content: String) -> Self { ... }
    pub fn new_image(path: String, size: usize) -> Self { ... }
    pub fn preview(&self, max_len: usize) -> String { ... }
    pub fn relative_time(&self) -> String { ... }
}
```

### 2.3 Database Layer (`src/db.rs`)

```rust
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self>;           // Opens/creates DB at XDG path
    pub fn init_schema(&self) -> Result<()>; // Creates tables if not exist

    // CRUD
    pub fn insert(&self, entry: &Entry) -> Result<()>;
    pub fn get_all(&self, limit: usize) -> Result<Vec<Entry>>;
    pub fn get_by_id(&self, id: &str) -> Result<Option<Entry>>;
    pub fn delete(&self, id: &str) -> Result<()>;
    pub fn clear_all(&self) -> Result<()>;

    // Deduplication
    pub fn exists_by_hash(&self, hash: &str) -> Result<bool>;

    // Cleanup
    pub fn cleanup_old(&self, max_entries: usize) -> Result<usize>;

    // Stats
    pub fn count(&self) -> Result<usize>;
}
```

**Schema (created in `init_schema`):**
```sql
CREATE TABLE IF NOT EXISTS entries (
    id TEXT PRIMARY KEY,
    entry_type TEXT NOT NULL,
    content TEXT NOT NULL,
    hash TEXT NOT NULL UNIQUE,
    byte_size INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    pinned INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_created_at ON entries(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_pinned ON entries(pinned DESC, created_at DESC);
```

### 2.4 Configuration (`src/config.rs`)

```rust
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,  // 500
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,  // 500
}

#[derive(Debug, Deserialize)]
pub struct UiConfig {
    pub show_preview: bool,
    pub date_format: DateFormat,
    #[serde(default)]
    pub theme: ThemeConfig,
}

impl Config {
    pub fn load() -> Result<Self>;  // Load from XDG config or defaults
}
```

**Deliverables:**
- [ ] `error.rs` - Error types
- [ ] `entry.rs` - Entry struct with helper methods
- [ ] `db.rs` - SQLite CRUD operations
- [ ] `config.rs` - Config loading with defaults
- [ ] Unit tests for DB operations

---

## Phase 3: Clipboard Watcher

### 3.1 Clipboard Abstraction (`src/clipboard.rs`)

```rust
use wl_clipboard_rs::copy::{MimeType, Options, Source};
use wl_clipboard_rs::paste::{get_contents, ClipboardType, Seat};

pub struct Clipboard;

impl Clipboard {
    /// Get current clipboard text content
    pub fn get_text() -> Result<Option<String>>;

    /// Get current clipboard image (saves to path, returns path)
    pub fn get_image(save_dir: &Path) -> Result<Option<String>>;

    /// Set clipboard content
    pub fn set_text(content: &str) -> Result<()>;

    /// Compute SHA256 hash of content
    pub fn hash(content: &[u8]) -> String;
}
```

### 3.2 Watcher Daemon (`src/watcher.rs`)

```rust
pub struct Watcher {
    db: Database,
    config: Config,
    last_hash: Option<String>,
}

impl Watcher {
    pub fn new(db: Database, config: Config) -> Self;

    /// Main loop - runs forever, polling clipboard
    pub fn run(&mut self) -> Result<()> {
        loop {
            if let Some(entry) = self.check_clipboard()? {
                self.db.insert(&entry)?;
                self.db.cleanup_old(self.config.general.max_entries)?;
            }
            std::thread::sleep(Duration::from_millis(
                self.config.general.poll_interval_ms
            ));
        }
    }

    /// Check clipboard for new content
    fn check_clipboard(&mut self) -> Result<Option<Entry>>;
}
```

**Key logic in `check_clipboard`:**
1. Try to get text from clipboard
2. If text exists, hash it
3. If hash differs from `last_hash` and not in DB, create entry
4. Update `last_hash`
5. (Future: same for images)

**Deliverables:**
- [ ] `clipboard.rs` - Wayland clipboard operations
- [ ] `watcher.rs` - Polling loop with deduplication
- [ ] Test: watcher captures clipboard changes
- [ ] Test: duplicates are skipped

---

## Phase 4: CLI Framework

### 4.1 CLI Definition (`src/cli.rs`)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ditox")]
#[command(about = "Terminal clipboard manager for Wayland")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start clipboard watcher daemon
    Watch,

    /// List recent clipboard entries
    List {
        #[arg(short, long, default_value = "10")]
        limit: usize,

        #[arg(long)]
        json: bool,
    },

    /// Copy entry to clipboard by index or ID
    Copy {
        /// Entry index (1-based) or ID
        target: String,
    },

    /// Clear clipboard history
    Clear {
        #[arg(long)]
        confirm: bool,
    },

    /// Show watcher status and statistics
    Status,
}
```

### 4.2 Main Entry Point (`src/main.rs`)

```rust
mod cli;
mod config;
mod db;
mod entry;
mod error;
mod clipboard;
mod watcher;
mod app;
mod ui;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> error::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("ditox=info")
        .init();

    let cli = Cli::parse();
    let config = Config::load()?;
    let db = Database::open()?;
    db.init_schema()?;

    match cli.command {
        None => run_tui(db, config),           // Default: TUI
        Some(Commands::Watch) => run_watcher(db, config),
        Some(Commands::List { limit, json }) => cmd_list(&db, limit, json),
        Some(Commands::Copy { target }) => cmd_copy(&db, &target),
        Some(Commands::Clear { confirm }) => cmd_clear(&db, confirm),
        Some(Commands::Status) => cmd_status(&db),
    }
}
```

**Deliverables:**
- [ ] `cli.rs` - Clap definitions
- [ ] `main.rs` - Command dispatch
- [ ] `ditox watch` starts watcher
- [ ] `ditox list` prints entries
- [ ] `ditox copy N` copies entry
- [ ] `ditox clear` clears history
- [ ] `ditox status` shows stats

---

## Phase 5: TUI Implementation

### 5.1 App State (`src/app.rs`)

```rust
pub enum InputMode {
    Normal,
    Search,
}

pub struct App {
    pub entries: Vec<Entry>,
    pub filtered: Vec<usize>,  // Indices into entries
    pub selected: usize,
    pub input_mode: InputMode,
    pub search_query: String,
    pub show_preview: bool,
    pub show_help: bool,
    pub should_quit: bool,
    pub message: Option<String>,

    db: Database,
    config: Config,
    matcher: nucleo_matcher::Matcher,
}

impl App {
    pub fn new(db: Database, config: Config) -> Result<Self>;
    pub fn reload_entries(&mut self) -> Result<()>;
    pub fn filter_entries(&mut self);
    pub fn selected_entry(&self) -> Option<&Entry>;

    // Actions
    pub fn move_up(&mut self);
    pub fn move_down(&mut self);
    pub fn go_top(&mut self);
    pub fn go_bottom(&mut self);
    pub fn copy_selected(&mut self) -> Result<()>;
    pub fn delete_selected(&mut self) -> Result<()>;
    pub fn toggle_pin(&mut self) -> Result<()>;
    pub fn clear_all(&mut self) -> Result<()>;

    // Search
    pub fn start_search(&mut self);
    pub fn end_search(&mut self);
    pub fn push_search_char(&mut self, c: char);
    pub fn pop_search_char(&mut self);
}
```

### 5.2 UI Theme (`src/ui/theme.rs`)

```rust
use ratatui::style::{Color, Style};

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub border: Color,
    pub muted: Color,
    pub accent: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            selected_bg: Color::Rgb(59, 66, 97),
            selected_fg: Color::Rgb(192, 202, 245),
            border: Color::Rgb(86, 95, 137),
            muted: Color::Rgb(86, 95, 137),
            accent: Color::Rgb(122, 162, 247),
        }
    }
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self;
    pub fn normal(&self) -> Style;
    pub fn selected(&self) -> Style;
    pub fn border(&self) -> Style;
    pub fn muted(&self) -> Style;
}
```

### 5.3 UI Layout (`src/ui/layout.rs`)

```rust
pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    // Main layout: vertical split
    // ┌─────────────────────────────────────┐
    // │ Search bar                          │
    // ├──────────────────────┬──────────────┤
    // │ List                 │ Preview      │
    // ├──────────────────────┴──────────────┤
    // │ Status bar / keybindings            │
    // └─────────────────────────────────────┘

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search
            Constraint::Min(10),    // Content
            Constraint::Length(1),  // Status
        ])
        .split(frame.area());

    draw_search(frame, app, theme, chunks[0]);
    draw_content(frame, app, theme, chunks[1]);
    draw_status(frame, app, theme, chunks[2]);

    if app.show_help {
        draw_help_popup(frame, theme);
    }
}
```

### 5.4 List Widget (`src/ui/list.rs`)

```rust
pub fn draw_list(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let items: Vec<ListItem> = app.filtered
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let entry = &app.entries[idx];
            let is_selected = i == app.selected;
            format_entry_row(entry, i + 1, is_selected, theme)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" History "));

    frame.render_widget(list, area);
}

fn format_entry_row(entry: &Entry, index: usize, selected: bool, theme: &Theme) -> ListItem {
    // Format: " 1 │ txt │ Hello world...     │ 2m "
    let type_str = match entry.entry_type {
        EntryType::Text => "txt",
        EntryType::Image => "img",
    };
    let preview = entry.preview(30);
    let time = entry.relative_time();

    let line = format!("{:>3} │ {} │ {:<30} │ {:>4}",
        index, type_str, preview, time);

    let style = if selected { theme.selected() } else { theme.normal() };
    ListItem::new(line).style(style)
}
```

### 5.5 Preview Widget (`src/ui/preview.rs`)

```rust
pub fn draw_preview(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let content = match app.selected_entry() {
        Some(entry) => match entry.entry_type {
            EntryType::Text => entry.content.clone(),
            EntryType::Image => format!(
                "[Image]\nPath: {}\nSize: {} bytes",
                entry.content, entry.byte_size
            ),
        },
        None => String::from("No entry selected"),
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" Preview "))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
```

### 5.6 Event Handling (`src/ui/mod.rs`)

```rust
pub fn run_tui(db: Database, config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(db, config)?;
    let theme = Theme::from_config(&app.config.ui.theme);

    // Main loop
    loop {
        terminal.draw(|f| layout::draw(f, &app, &theme))?;

        if let Event::Key(key) = event::read()? {
            handle_key(&mut app, key)?;
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Search => handle_search_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('g') => app.go_top(),
        KeyCode::Char('G') => app.go_bottom(),
        KeyCode::Enter => {
            app.copy_selected()?;
            app.should_quit = true;
        }
        KeyCode::Char('y') => app.copy_selected()?,
        KeyCode::Char('d') => app.delete_selected()?,
        KeyCode::Char('D') => app.clear_all()?,
        KeyCode::Char('/') => app.start_search(),
        KeyCode::Tab => app.show_preview = !app.show_preview,
        KeyCode::Char('?') => app.show_help = !app.show_help,
        KeyCode::Esc => app.show_help = false,
        _ => {}
    }
    Ok(())
}
```

**Deliverables:**
- [ ] `app.rs` - Application state and logic
- [ ] `ui/theme.rs` - Color scheme
- [ ] `ui/layout.rs` - Main layout
- [ ] `ui/list.rs` - Entry list widget
- [ ] `ui/preview.rs` - Content preview
- [ ] `ui/search.rs` - Search bar
- [ ] `ui/help.rs` - Help popup
- [ ] `ui/mod.rs` - Event loop and key handling
- [ ] All keybindings working
- [ ] Fuzzy search working

---

## Phase 6: NixOS Integration

### 6.1 Package Derivation (`nix/package.nix`)

```nix
{ lib
, rustPlatform
, pkg-config
, openssl
}:

rustPlatform.buildRustPackage rec {
  pname = "ditox";
  version = "0.1.0";

  src = ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];

  meta = with lib; {
    description = "Terminal clipboard manager for Wayland";
    homepage = "https://github.com/user/ditox";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "ditox";
  };
}
```

### 6.2 Home Manager Module (`nix/module.nix`)

Copy from PRD - already complete.

### 6.3 Testing

```bash
# Build
nix build

# Run
nix run . -- --help
nix run . -- watch &
nix run .

# Development
nix develop
cargo run -- watch &
cargo run
```

**Deliverables:**
- [ ] `nix/package.nix` builds successfully
- [ ] `nix/module.nix` integrates with Home Manager
- [ ] Systemd service starts on login
- [ ] Config file generated from Nix settings

---

## Phase 7: Polish & Testing

### 7.1 Error Handling
- [ ] Graceful handling of clipboard errors
- [ ] Clear error messages for users
- [ ] Logging with `tracing`

### 7.2 Edge Cases
- [ ] Empty clipboard
- [ ] Very large text (truncate in preview)
- [ ] Binary data in clipboard (skip)
- [ ] Concurrent DB access (watcher + TUI)
- [ ] Watcher already running detection

### 7.3 Documentation
- [ ] README.md with installation instructions
- [ ] Man page (optional)
- [ ] `--help` messages

### 7.4 Testing
- [ ] Unit tests for DB operations
- [ ] Unit tests for entry hashing
- [ ] Integration test: watcher + TUI
- [ ] Manual testing on Hyprland

---

## Implementation Order

```
Week 1: Foundation
├── Day 1-2: Project setup, Cargo.toml, flake.nix
├── Day 3-4: error.rs, entry.rs, db.rs
└── Day 5: config.rs, basic tests

Week 2: Core Features
├── Day 1-2: clipboard.rs, watcher.rs
├── Day 3: cli.rs, main.rs
└── Day 4-5: Test watcher + CLI commands

Week 3: TUI
├── Day 1: app.rs, theme.rs
├── Day 2-3: layout.rs, list.rs, preview.rs
├── Day 4: search.rs, help.rs
└── Day 5: Event handling, keybindings

Week 4: Integration & Polish
├── Day 1-2: Nix package + module
├── Day 3: Testing on NixOS
├── Day 4: Bug fixes, edge cases
└── Day 5: Documentation, release
```

---

## Milestones

### M1: Watcher Works (End of Week 2)
- `ditox watch` captures clipboard to SQLite
- `ditox list` shows history
- `ditox copy 1` restores entry

### M2: TUI Works (End of Week 3)
- `ditox` opens TUI
- Navigate, search, copy, delete all functional
- Preview pane shows content

### M3: NixOS Ready (End of Week 4)
- `nix run github:user/ditox` works
- Home Manager module configures everything
- Systemd service auto-starts watcher

---

## File Checklist

```
[ ] Cargo.toml
[ ] flake.nix
[ ] flake.lock
[ ] nix/package.nix
[ ] nix/module.nix
[ ] src/main.rs
[ ] src/cli.rs
[ ] src/error.rs
[ ] src/entry.rs
[ ] src/db.rs
[ ] src/config.rs
[ ] src/clipboard.rs
[ ] src/watcher.rs
[ ] src/app.rs
[ ] src/ui/mod.rs
[ ] src/ui/theme.rs
[ ] src/ui/layout.rs
[ ] src/ui/list.rs
[ ] src/ui/preview.rs
[ ] src/ui/search.rs
[ ] src/ui/help.rs
[ ] README.md
[ ] LICENSE
```
