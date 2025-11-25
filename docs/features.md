# Ditox Features

A comprehensive list of all features available in ditox, the Wayland clipboard manager.

## Core Functionality

### Clipboard Monitoring
- **Wayland clipboard daemon** - Background service that monitors clipboard changes
- **Automatic capture** - Configurable polling interval (default: 250ms)
- **Smart deduplication** - SHA256 hashing prevents duplicate entries
- **Auto-cleanup** - Maintains configurable max entries (default: 500)

### Entry Types
- **Text entries** - Plain text, code, URLs, emails, etc.
- **Image entries** - PNG, JPEG, GIF, BMP, WebP support
- **Content detection** - Automatically detects URLs, emails, file paths, JSON, YAML, code, shell commands, colors, UUIDs, IP addresses, phone numbers

### Entry Management
- **Favorites** - Mark entries as favorites (protected from cleanup, easy access via Favorites tab)
- **Notes** - Add annotations to any entry
- **Usage tracking** - Copy count and last-used timestamp
- **Collections** - Organize entries into named groups

## TUI Interface

### Navigation
- Vim-style keybindings (`j`/`k`, `g`/`G`, etc.)
- Page navigation with visual feedback
- Scrollbar indicator showing position
- Jump to top/bottom

### Search
- **Fuzzy search** - Typo-tolerant, fast matching
- **Regex search** - Full regex pattern support
- **Real-time filtering** - Results update as you type
- **Match highlighting** - Matched characters highlighted in list and preview
- **Result count** - Shows number of matches

### Preview Pane
- **Multiple modes**:
  - Wrap - Text wraps at pane width
  - Scroll - Horizontal scrolling for long lines
  - Truncate - Shows first N lines
  - Hex - Hexdump view for binary inspection
  - Raw - Shows escape sequences
- **Line numbers** - Toggle with `L` key
- **Image preview** - Inline image display with dimensions
- **Multiple graphics protocols** - Kitty, Sixel, iTerm2, Half-blocks

### Tabs
- **All** - All entries
- **Text** - Text entries only
- **Images** - Image entries only
- **Favorites** - Favorite entries for quick access
- **Today** - Today's entries
- **Collections** - Custom collection tabs

### Multi-Select
- Select multiple entries with `Space`
- Select/deselect all with `v`
- Batch delete selected entries
- Batch copy selected text entries

### Mouse Support
- Click to select entries
- Scroll wheel navigation
- Double-click to copy

### Visual Features
- Entry type icons (T/I)
- Favorite status indicator (star)
- Relative timestamps ("2m ago")
- Entry size display
- Search highlights
- Scrollbar with position indicator

### Terminal Handling
- Minimum size detection (60x10)
- Graceful degradation for narrow terminals
- Auto-hide tabs when width < 80
- Auto-hide snippets when width < 100
- Auto-hide preview when width <= 60

### Status Bar
- Watcher status indicator
- Entry count
- Keybinding hints
- Message display with auto-timeout (2s)
- Last refresh timestamp

## CLI Commands

### Entry Operations
```bash
ditox list [--limit N] [--json] [--favorites]  # List entries
ditox get <index|uuid> [--json]              # Get full content
ditox search <query> [--limit N] [--json]    # Search entries
ditox copy <index|uuid>                       # Copy to clipboard
ditox delete <index|uuid>                     # Delete entry
ditox favorite <index|uuid>                   # Toggle favorite
```

### Metadata
```bash
ditox count                  # Total entry count
ditox status                 # Watcher status
ditox stats [--json]         # Usage statistics
ditox clear [--confirm]      # Clear all history
```

### Daemon
```bash
ditox watch                  # Start watcher daemon
```

### Collections
```bash
ditox collection list [--json]
ditox collection create <name> [-c color] [-k keybind]
ditox collection delete <name|id>
ditox collection rename <name|id> <new_name>
ditox collection add <entry> <collection>
ditox collection remove <entry>
ditox collection show <name|id> [-l limit] [--json]
```

All commands support `--json` for programmatic access.

## Configuration

Configuration file: `~/.config/ditox/config.toml`

### General Settings
```toml
[general]
max_entries = 500           # Max history size
poll_interval_ms = 250      # Polling interval
```

### Storage
```toml
[storage]
data_dir = "~/.local/share/ditox"  # Custom data directory
```

### UI Settings
```toml
[ui]
show_preview = true         # Show preview by default
date_format = "relative"    # "relative" or "iso"
graphics_protocol = "auto"  # "auto", "kitty", "sixel", "iterm2", "halfblocks"

[ui.font_size]
width = 9                   # Font width in pixels (for image rendering)
height = 18                 # Font height in pixels
```

### Theme
```toml
[ui.theme]
# Full color customization available
# See default config for all options
```

### Keybindings
```toml
[keybindings]
move_up = "k"
move_down = "j"
copy = "y"
# ... customize any action
```

## Quick Snippets

- 9 quick-access slots (keys 1-9)
- Assign favorite entries to slots
- Fast clipboard copy with single keypress
- Shown in status bar when configured

## Data Storage

- **Database**: `~/.local/share/ditox/ditox.db` (SQLite)
- **Images**: `~/.local/share/ditox/images/`
- **Config**: `~/.config/ditox/config.toml`
- **PID file**: `~/.local/share/ditox/watcher.pid`

## Statistics

Track your clipboard usage:
- Total entries (text vs images)
- Copy count per entry
- Top used entries
- Time-based stats (today, week, month)
- Storage size (database + images)

## Image Handling

- Background async loading
- Multiple MIME type support
- Dimensions display in preview
- Multiple terminal graphics protocols
- Proper clipboard copy (maintains image format)

## Search & Matching

- Case-insensitive fuzzy search
- Nucleo matcher for fast fuzzy matching
- Regex mode with error feedback
- Searches content and notes
- SQL pre-filtering for performance

## Performance

- Lazy loading with pagination (20 entries per page)
- Indexed database queries
- Background image loading
- Efficient incremental search
- Configurable history limits
