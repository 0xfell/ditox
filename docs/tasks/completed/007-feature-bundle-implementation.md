# Task: Feature Bundle Implementation Plan

> **Status:** completed
> **Priority:** high
> **Created:** 2025-11-27
> **Completed:** 2025-11-27

## Description

Comprehensive implementation plan for 10 selected features from the brainstorm document. This plan addresses dependencies, conflicts, shared infrastructure, and provides a phased approach to implementation.

---

## Features Overview

| # | Feature | Complexity | Phase |
|---|---------|------------|-------|
| 12 | Custom Keybindings | Medium | 1 - Infrastructure |
| 8 | Usage Statistics | Low | 1 - Infrastructure |
| 13 | Entry Annotations | Low | 2 - Schema |
| 1 | Favorites/Collections | Medium | 2 - Schema |
| 10 | Regex Search Mode | Medium | 3 - Search |
| 3 | Smart Content Detection | Medium | 3 - Search |
| 18 | Preview Pane Modes | Low | 4 - UI |
| 20 | TUI Tabs | Medium | 4 - UI |
| 9 | Quick Snippets Bar | Medium | 4 - UI |
| 11 | Watch Status Indicator | Low | 4 - UI |

---

## Conflict Analysis & Resolutions

### Conflict 1: Screen Real Estate (#9 vs #20)

**Problem:** Both Quick Snippets Bar (#9) and TUI Tabs (#20) want horizontal space at the top of the screen.

**Resolution:** Unified Top Bar Design
```
┌─────────────────────────────────────────────────────────────────┐
│ [All] [Text] [Images] [Pinned] [Work]  │  1:url  2:pwd  3:email │
├─────────────────────────────────────────────────────────────────┤
│ Search: query█ (5 matches)                                      │
```
- **Left side:** Filter tabs (including collections)
- **Right side:** Quick snippet slots (1-9)
- **Separator:** `│` between tabs and snippets
- **Fallback:** If terminal width < 100, hide snippets bar (show only tabs)

### Conflict 2: Keybindings Chaos (#12 affects all)

**Problem:** Custom keybindings must work with all new features without conflicts.

**Resolution:** Implement keybindings system FIRST
- Define action enum covering all existing + planned actions
- Default keybindings in code, overridable via config
- New features register their actions in the action enum
- Keybind validation to prevent duplicates

### Conflict 3: Collections Integration (#1 + #20)

**Problem:** Collections could be tabs, but tabs also need built-in filters.

**Resolution:** Hybrid Tab System
```rust
enum TabFilter {
    // Built-in (not removable)
    All,
    Text,
    Images,
    Pinned,
    // User collections (from DB)
    Collection(String),  // collection_id
}
```
- Built-in tabs always present
- User collections appear as additional tabs
- Tab overflow: Show `[+3]` indicator, dropdown on click

### Conflict 4: Usage Data Flow (#8 → #9)

**Problem:** Quick Snippets needs usage data to show "most used" entries.

**Resolution:** Implement Usage Statistics first
- Add `usage_count` column to entries table
- Increment on each copy operation
- Quick Snippets queries top N by usage_count
- Optional: Allow manual snippet slot assignment (overrides auto)

### Conflict 5: Content Analysis Overlap (#3 + #10)

**Problem:** Both Smart Content Detection and Regex Search analyze content patterns.

**Resolution:** Shared Pattern Infrastructure
```rust
// src/patterns.rs - New module
pub struct ContentPattern {
    pub name: &'static str,      // "url", "email", "json"
    pub regex: Regex,
    pub content_type: ContentType,
}

pub enum ContentType {
    Url,
    Email,
    FilePath,
    Json,
    Code(Language),
    ColorCode,
    Plain,
}
```
- Regex search uses same `regex` crate
- Content detection uses pre-compiled patterns
- Both can share pattern matching utilities

---

## Shared Infrastructure

### 1. Action System (for #12 Custom Keybindings)

**New file:** `src/actions.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    GoTop,
    GoBottom,
    PageUp,
    PageDown,

    // Operations
    Copy,
    CopyAndQuit,
    Delete,
    ClearAll,
    TogglePin,

    // Modes
    EnterSearch,
    ExitSearch,
    TogglePreview,
    ToggleExpanded,
    ToggleHelp,
    ToggleMultiSelect,

    // Multi-select
    SelectCurrent,
    SelectAll,

    // New features
    ToggleRegexSearch,       // #10
    ShowActions,             // #3
    CyclePreviewMode,        // #18
    NextTab,                 // #20
    PrevTab,                 // #20
    QuickSlot(u8),           // #9 (1-9)
    EditAnnotation,          // #13
    ShowStats,               // #8

    // System
    Refresh,
    Quit,
}
```

### 2. Pattern Matching Module (for #3, #10)

**New file:** `src/patterns.rs`
- Pre-compiled regex patterns for content detection
- URL, email, file path, JSON, code detection
- Shared between content detection and regex search

### 3. Database Migration System

**Current:** Ad-hoc migrations in `db.rs`
**Needed:** Structured migration for multiple schema changes

```rust
// src/db.rs additions
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", "CREATE TABLE entries ..."),
    ("002_last_used", "ALTER TABLE entries ADD COLUMN last_used ..."),
    ("003_usage_count", "ALTER TABLE entries ADD COLUMN usage_count INTEGER DEFAULT 0"),
    ("004_notes", "ALTER TABLE entries ADD COLUMN notes TEXT"),
    ("005_collections", "CREATE TABLE collections ..."),
    ("006_detected_type", "ALTER TABLE entries ADD COLUMN detected_type TEXT"),
];
```

---

## Phase 1: Infrastructure

### Feature #12: Custom Keybindings

**Description:** User-configurable keybindings via config file.

**Database Changes:** None

**Config Changes (`config.rs`):**
```rust
#[derive(Deserialize, Default)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub bindings: HashMap<String, String>,  // "ctrl+d" -> "delete"
}

// In Config struct:
pub keybindings: KeybindingsConfig,
```

**Example config.toml:**
```toml
[keybindings]
# Format: "key" = "action"
"p" = "toggle_preview"      # Changed from Tab
"ctrl+x" = "delete"         # Alternative delete
"ctrl+s" = "toggle_pin"     # Alternative pin
```

**New Files:**
- `src/actions.rs` - Action enum and dispatch
- `src/keybindings.rs` - Key parsing, binding resolution

**Implementation Steps:**
- [ ] Create Action enum with all current actions
- [ ] Create key parser (handles ctrl+, alt+, shift+)
- [ ] Create KeybindingResolver that maps KeyEvent → Action
- [ ] Load custom bindings from config
- [ ] Refactor `ui/mod.rs` to use action dispatch instead of direct key matching
- [ ] Add keybinding validation (no duplicates, valid actions)
- [ ] Update help overlay to show current bindings

**Keybinding Format:**
```
Simple: "q", "j", "/", "?"
Modified: "ctrl+u", "alt+d", "shift+g"
Special: "enter", "esc", "tab", "space", "backspace"
Arrow: "up", "down", "left", "right"
Function: "f1", "f2", ... "f12"
```

---

### Feature #8: Usage Statistics

**Description:** Track and display clipboard usage patterns.

**Database Changes:**
```sql
-- Migration 003_usage_count
ALTER TABLE entries ADD COLUMN usage_count INTEGER DEFAULT 0;

-- Update existing entries
UPDATE entries SET usage_count = 0 WHERE usage_count IS NULL;
```

**New CLI Command:**
```bash
ditox stats [--json]
```

**Output:**
```
Ditox Usage Statistics
══════════════════════
Total entries:     342
  Text:            298 (87%)
  Images:           44 (13%)

Storage:
  Database:        2.4 MB
  Images:         48.2 MB

Most copied (top 5):
  1. https://github.com/...     (copied 47 times)
  2. my-password-123           (copied 23 times)
  3. SELECT * FROM users...    (copied 18 times)

Activity:
  Today:            12 copies
  This week:        67 copies
  This month:      234 copies

Busiest hour:      2-3 PM (avg 8.2 copies)
```

**Implementation Steps:**
- [ ] Add `usage_count` column via migration
- [ ] Modify `db.touch()` to also increment usage_count
- [ ] Add `db.get_stats()` method returning Stats struct
- [ ] Add `stats` subcommand to CLI (`main.rs`)
- [ ] Create Stats struct with computed fields
- [ ] Optional: Add `S` keybind in TUI to show stats popup

**Stats Struct:**
```rust
pub struct Stats {
    pub total_entries: usize,
    pub text_count: usize,
    pub image_count: usize,
    pub db_size_bytes: u64,
    pub images_size_bytes: u64,
    pub top_entries: Vec<(Entry, u32)>,  // (entry, usage_count)
    pub copies_today: usize,
    pub copies_week: usize,
    pub copies_month: usize,
}
```

---

## Phase 2: Database Schema Extensions

### Feature #13: Entry Annotations

**Description:** Add personal notes/annotations to entries.

**Database Changes:**
```sql
-- Migration 004_notes
ALTER TABLE entries ADD COLUMN notes TEXT;
```

**Entry Struct Update:**
```rust
pub struct Entry {
    // ... existing fields
    pub notes: Option<String>,
}
```

**TUI Changes:**
- `n` keybind opens annotation editor (modal)
- Notes displayed in preview pane below content
- Notes are searchable (included in fuzzy search)

**UI Modal:**
```
┌─ Edit Note ─────────────────────────┐
│                                     │
│ > This is my note about this entry_ │
│                                     │
│ Enter: Save  Esc: Cancel            │
└─────────────────────────────────────┘
```

**Implementation Steps:**
- [ ] Add `notes` column via migration
- [ ] Update Entry struct and from_row()
- [ ] Add `db.update_notes(id, notes)` method
- [ ] Create `ui/note_editor.rs` - modal input widget
- [ ] Add `n` keybind to open editor
- [ ] Include notes in search (append to content for matching)
- [ ] Show notes in preview pane (below content, muted color)
- [ ] Add `--notes` flag to `ditox get` command

---

### Feature #1: Favorites/Collections

**Description:** Organize entries into named collections.

**Database Changes:**
```sql
-- Migration 005_collections
CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT,              -- Optional hex color
    keybind TEXT,            -- Optional quick access (1-9)
    position INTEGER DEFAULT 0,
    created_at TEXT NOT NULL
);

ALTER TABLE entries ADD COLUMN collection_id TEXT REFERENCES collections(id);
CREATE INDEX idx_collection ON entries(collection_id);
```

**New Structs:**
```rust
pub struct Collection {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub keybind: Option<char>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
```

**CLI Commands:**
```bash
ditox collection list                    # List all collections
ditox collection create "Work"           # Create collection
ditox collection delete "Work"           # Delete (entries become uncategorized)
ditox collection add <entry> "Work"      # Add entry to collection
ditox collection remove <entry>          # Remove from collection
ditox list --collection "Work"           # Filter by collection
```

**TUI Changes:**
- `c` keybind opens collection picker for selected entry
- Collections appear as tabs (see #20 integration)
- Collection indicator in list: `[Work]` prefix or colored dot

**Implementation Steps:**
- [ ] Create collections table via migration
- [ ] Add `collection_id` column to entries
- [ ] Create Collection struct and CRUD methods
- [ ] Add collection CLI subcommands
- [ ] Create `ui/collection_picker.rs` - modal selector
- [ ] Add `c` keybind for collection assignment
- [ ] Integrate with tabs (#20) for filtering
- [ ] Show collection indicator in list items

---

## Phase 3: Search & Content Analysis

### Feature #10: Regex Search Mode

**Description:** Toggle between fuzzy and regex search.

**App State Changes:**
```rust
pub enum SearchMode {
    Fuzzy,   // Current default (nucleo)
    Regex,   // New regex mode
}

// In App struct:
pub search_mode: SearchMode,
```

**UI Changes:**
- `/` enters fuzzy search (current)
- `:` enters regex search (new)
- Or: `ctrl+r` toggles mode while in search
- Search bar shows mode: `Search (fuzzy):` or `Search (regex):`

**Regex Features:**
- Case-insensitive by default (prefix `(?-i)` for case-sensitive)
- Highlight all matches in preview
- Show match count
- Invalid regex shows error message

**Implementation Steps:**
- [ ] Add SearchMode enum to App
- [ ] Add `regex` crate (already a dependency of other crates)
- [ ] Create `apply_regex_filter()` method in app.rs
- [ ] Modify `filter_entries()` to dispatch based on mode
- [ ] Add `:` keybind for regex search entry
- [ ] Or add `ctrl+r` toggle while searching
- [ ] Update search bar to show current mode
- [ ] Handle regex compilation errors gracefully
- [ ] Highlight regex matches in list and preview

**Regex Match Highlighting:**
```rust
// Store all match ranges for highlighting
pub regex_matches: HashMap<usize, Vec<(usize, usize)>>,  // entry_idx -> [(start, end)]
```

---

### Feature #3: Smart Content Detection

**Description:** Detect content types and offer contextual actions.

**New Module:** `src/patterns.rs`
```rust
use regex::Regex;
use once_cell::sync::Lazy;

pub enum ContentType {
    Url,
    Email,
    FilePath,
    Json,
    Xml,
    Code(CodeLanguage),
    ColorHex,
    ColorRgb,
    Uuid,
    IpAddress,
    PhoneNumber,
    Plain,
}

pub enum CodeLanguage {
    Rust,
    Python,
    JavaScript,
    Shell,
    Sql,
    Unknown,
}

static PATTERNS: Lazy<Vec<(ContentType, Regex)>> = Lazy::new(|| vec![
    (ContentType::Url, Regex::new(r"^https?://[^\s]+$").unwrap()),
    (ContentType::Email, Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()),
    (ContentType::FilePath, Regex::new(r"^[/~][\w./-]+$").unwrap()),
    (ContentType::Json, Regex::new(r"^\s*[\[{]").unwrap()),
    (ContentType::ColorHex, Regex::new(r"^#[0-9A-Fa-f]{6}$").unwrap()),
    (ContentType::Uuid, Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()),
    // ... more patterns
]);

pub fn detect_content_type(content: &str) -> ContentType { ... }
```

**Database Changes:**
```sql
-- Migration 006_detected_type
ALTER TABLE entries ADD COLUMN detected_type TEXT;
```

**Action Menu:**
Press `a` on selected entry to show contextual actions:
```
┌─ Actions ───────────────────────┐
│ Detected: URL                   │
│                                 │
│ [1] Open in browser             │
│ [2] Copy domain only            │
│ [3] Copy without protocol       │
│ [4] Copy as markdown link       │
│                                 │
│ 1-4: Select  Esc: Cancel        │
└─────────────────────────────────┘
```

**Actions by Type:**

| Type | Actions |
|------|---------|
| URL | Open browser, copy domain, copy path, markdown link |
| Email | Open mailto, copy username, copy domain |
| FilePath | Open file manager, copy filename, copy directory |
| JSON | Pretty print, minify, copy key (prompt) |
| ColorHex | Show color swatch, convert to RGB, copy |
| Code | Copy with syntax highlighting (for paste in rich text) |

**Implementation Steps:**
- [ ] Create `src/patterns.rs` with content detection
- [ ] Add `detected_type` column (optional, can be computed on-the-fly)
- [ ] Detect type on entry creation in watcher
- [ ] Create `ui/actions_menu.rs` - modal action picker
- [ ] Add `a` keybind to open actions menu
- [ ] Implement actions for each content type
- [ ] Show detected type in list (icon or abbreviated)
- [ ] Show detected type in preview header

**Content Type Icons (for list):**
```
🔗 URL
📧 Email
📁 File
{} JSON
🎨 Color
< > Code
📝 Plain
```

---

## Phase 4: UI Components

### Feature #18: Preview Pane Modes

**Description:** Multiple display modes for the preview pane.

**Preview Modes:**
```rust
pub enum PreviewMode {
    Wrap,      // Current default - text wraps at pane width
    Scroll,    // Horizontal scroll for long lines
    Truncate,  // First N lines, "... X more lines" indicator
    Hex,       // Hex dump view for binary/special content
    Raw,       // Show escape sequences, control chars visible
}
```

**UI Changes:**
- `p` cycles through preview modes (Wrap → Scroll → Truncate → Hex → Raw)
- Mode indicator in preview border: `─ Preview (wrap) ─`
- Mode persisted per-session (not saved to config)

**Mode Behaviors:**

| Mode | Behavior |
|------|----------|
| Wrap | Text wraps at pane width (current) |
| Scroll | Single line shows full width, arrow keys scroll horizontally |
| Truncate | First 20 lines, footer shows "+ N more lines" |
| Hex | `00000000  48 65 6c 6c 6f 20  Hello ` format |
| Raw | Shows `\n`, `\t`, `\x1b[` literally |

**Implementation Steps:**
- [ ] Add PreviewMode enum to App state
- [ ] Add `p` keybind to cycle modes (remap current Tab if needed)
- [ ] Modify `ui/preview.rs` to render based on mode
- [ ] Implement horizontal scrolling state for Scroll mode
- [ ] Implement hex dump renderer
- [ ] Implement raw/escaped renderer
- [ ] Show mode indicator in preview border title
- [ ] Add left/right arrow handling in Scroll mode

---

### Feature #20: TUI Tabs (Filter Bar)

**Description:** Tab bar for quick filtering at top of screen.

**Tab Types:**
```rust
pub enum TabFilter {
    All,                      // No filter
    Text,                     // entry_type = "text"
    Images,                   // entry_type = "image"
    Pinned,                   // pinned = true
    Collection(String),       // collection_id = X
    Today,                    // created_at within 24h
    Custom(String, String),   // (name, filter_query) - future
}
```

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│ [All] [Text] [Images] [★Pinned] [Today] [Work] [Personal]  │ ← Tabs row
├─────────────────────────────────────────────────────────────┤
│ Search: query█                                              │
```

**Navigation:**
- `[` / `]` or `Tab` / `Shift+Tab` - cycle tabs
- `1-9` - jump to tab by number (if not conflicting with snippets)
- Mouse click on tab to select

**App State:**
```rust
pub struct App {
    // ... existing
    pub tabs: Vec<TabFilter>,
    pub active_tab: usize,
}
```

**Implementation Steps:**
- [ ] Add TabFilter enum
- [ ] Add tabs Vec and active_tab to App
- [ ] Create `ui/tabs.rs` - tab bar widget
- [ ] Modify layout.rs to include tabs row (height 1)
- [ ] Add `[` / `]` keybinds for tab cycling
- [ ] Modify `filter_entries()` to apply tab filter
- [ ] Integrate collections as tabs (from #1)
- [ ] Handle tab overflow (show `[+N]` or scroll)
- [ ] Add mouse click support for tabs
- [ ] Persist last active tab to config (optional)

**Tab Rendering:**
```rust
// Active tab: inverted colors
// Inactive: normal
// With counts: [All (342)] [Text (298)] [Images (44)]
```

---

### Feature #9: Quick Snippets Bar

**Description:** Horizontal bar showing frequently used entries with 1-9 quick access.

**Integration with Tabs:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ [All] [Text] [Images] [Pinned]  │  1:https://  2:pass  3:SELECT    │
│ ← Tabs                          │  ← Snippets (1-9 to copy)        │
```

**Snippet Sources:**
1. **Auto:** Top N entries by `usage_count` (from #8)
2. **Manual:** User-assigned slots via `ditox snippet set 1 <entry>`

**App State:**
```rust
pub struct App {
    // ... existing
    pub snippet_slots: [Option<String>; 9],  // entry IDs for slots 1-9
}
```

**Database/Config:**
```toml
[snippets]
# Manual slot assignments (entry UUIDs)
slot_1 = "abc-123-..."
slot_2 = "def-456-..."
# Unassigned slots auto-fill from most-used
```

**Keybinds:**
- `1-9` in Normal mode - copy snippet from slot (if assigned)
- `alt+1-9` - assign current entry to slot (alternative)

**CLI Commands:**
```bash
ditox snippet list              # Show all slots
ditox snippet set 1 <entry>     # Assign entry to slot 1
ditox snippet clear 1           # Clear slot 1
ditox snippet auto              # Reset all to auto (most-used)
```

**Implementation Steps:**
- [ ] Add snippet_slots to App state
- [ ] Add snippets config section
- [ ] Load manual assignments from config
- [ ] Auto-fill empty slots from top usage_count entries
- [ ] Create `ui/snippets.rs` - snippets bar widget
- [ ] Integrate into tabs row (right side)
- [ ] Add `1-9` keybinds for quick copy
- [ ] Add snippet CLI subcommands
- [ ] Show truncated preview in snippet slot (8-10 chars)
- [ ] Handle narrow terminals (hide snippets if width < 100)

**Snippet Display:**
```
1:https://  2:my-pass  3:SELECT  4:{"key"  5:~/Doc...
  ↑ slot     ↑ truncated content preview (10 chars)
```

---

### Feature #11: Watch Status Indicator

**Description:** Show real-time watcher daemon status in TUI.

**Status Display Location:**
```
┌─────────────────────────────────────────────────────────────┐
│ [All] [Text] ...                           ● Watching (2s) │
│                                            ↑ Status        │
```
Or in status bar (bottom):
```
│ j/k:Move  ..  q:Quit │ 342 entries │ ● Watching (2s ago) │
```

**Status States:**
- `● Watching (Xs ago)` - Green dot, watcher running, X seconds since last check
- `○ Idle` - Gray dot, no watcher detected
- `! Error` - Red, watcher error (if detectable)

**Detection Methods:**
1. **PID file:** Watcher writes PID to `~/.local/share/ditox/watcher.pid`
2. **Heartbeat file:** Watcher touches `~/.local/share/ditox/watcher.heartbeat` every poll
3. **Process check:** Look for `ditox watch` process

**Implementation Steps:**
- [ ] Modify watcher.rs to write PID file on start
- [ ] Modify watcher.rs to touch heartbeat file each poll
- [ ] Add `watch_status()` function to check watcher state
- [ ] Add status indicator to layout (top-right or status bar)
- [ ] Create `ui/status_indicator.rs` widget
- [ ] Poll status every 2s in TUI refresh loop
- [ ] Clean up PID file on watcher exit (signal handler)
- [ ] Add `ditox status` output for watcher state

**Watcher Files:**
```
~/.local/share/ditox/watcher.pid        # Contains PID
~/.local/share/ditox/watcher.heartbeat  # mtime = last poll time
```

---

## Implementation Order Summary

```
Phase 1: Infrastructure (Week 1-2)
├── #12 Custom Keybindings ──────────┐
│   └── Action system foundation     │
└── #8 Usage Statistics              │
    └── usage_count column           │
                                     │
Phase 2: Schema Extensions (Week 2-3)│
├── #13 Entry Annotations            │
│   └── notes column                 │
└── #1 Favorites/Collections         │
    └── collections table            │
                                     ├── All depend on keybindings
Phase 3: Search & Content (Week 3-4) │
├── #10 Regex Search Mode            │
│   └── patterns.rs shared module    │
└── #3 Smart Content Detection       │
    └── Uses patterns.rs             │
                                     │
Phase 4: UI Components (Week 4-5)    │
├── #18 Preview Pane Modes           │
├── #20 TUI Tabs ────────────────────┤
│   └── Integrates #1 collections    │
├── #9 Quick Snippets Bar ───────────┤
│   └── Uses #8 usage data           │
└── #11 Watch Status Indicator       │
```

---

## Database Migration Summary

| Migration | Feature | Changes |
|-----------|---------|---------|
| 003 | #8 Usage Stats | `usage_count INTEGER DEFAULT 0` |
| 004 | #13 Annotations | `notes TEXT` |
| 005 | #1 Collections | `collections` table, `collection_id` FK |
| 006 | #3 Content Detection | `detected_type TEXT` (optional) |

---

## New Files Summary

| File | Features | Purpose |
|------|----------|---------|
| `src/actions.rs` | #12 | Action enum, dispatch |
| `src/keybindings.rs` | #12 | Key parsing, binding resolution |
| `src/patterns.rs` | #3, #10 | Content detection, regex patterns |
| `src/stats.rs` | #8 | Statistics computation |
| `src/collection.rs` | #1 | Collection model and CRUD |
| `src/ui/tabs.rs` | #20 | Tab bar widget |
| `src/ui/snippets.rs` | #9 | Snippets bar widget |
| `src/ui/note_editor.rs` | #13 | Note editing modal |
| `src/ui/actions_menu.rs` | #3 | Contextual actions modal |
| `src/ui/collection_picker.rs` | #1 | Collection assignment modal |
| `src/ui/status_indicator.rs` | #11 | Watcher status widget |

---

## Config Changes Summary

```toml
# config.toml additions

[keybindings]
# Custom keybindings (optional overrides)
"p" = "toggle_preview"
"ctrl+r" = "toggle_regex_search"

[snippets]
# Manual snippet slot assignments
slot_1 = "entry-uuid-here"
# Empty slots auto-fill from most-used

[ui]
# Existing...
preview_mode = "wrap"  # wrap, scroll, truncate, hex, raw
show_tabs = true
show_snippets = true
```

---

## Requirements Checklist

### Phase 1: Infrastructure
- [ ] #12: Action enum with all actions
- [ ] #12: Key parser (ctrl+, alt+, special keys)
- [ ] #12: KeybindingResolver
- [ ] #12: Config loading for custom bindings
- [ ] #12: Refactor ui/mod.rs to use actions
- [ ] #12: Update help overlay
- [ ] #8: Add usage_count column
- [ ] #8: Increment on copy
- [ ] #8: Stats struct and computation
- [ ] #8: CLI `ditox stats` command

### Phase 2: Schema Extensions
- [ ] #13: Add notes column
- [ ] #13: Note editor modal
- [ ] #13: Include notes in search
- [ ] #13: Display in preview
- [ ] #1: Collections table
- [ ] #1: Collection CRUD
- [ ] #1: Collection picker modal
- [ ] #1: CLI collection commands
- [ ] #1: List indicator

### Phase 3: Search & Content
- [ ] #10: SearchMode enum
- [ ] #10: Regex filter implementation
- [ ] #10: Mode toggle keybind
- [ ] #10: Error handling
- [ ] #3: patterns.rs module
- [ ] #3: Content type detection
- [ ] #3: Actions menu modal
- [ ] #3: Type-specific actions
- [ ] #3: Type indicator in list

### Phase 4: UI Components
- [ ] #18: PreviewMode enum
- [ ] #18: Mode cycling keybind
- [ ] #18: All mode renderers
- [ ] #20: TabFilter enum
- [ ] #20: Tab bar widget
- [ ] #20: Tab navigation
- [ ] #20: Collection integration
- [ ] #9: Snippet slots state
- [ ] #9: Auto-fill from usage
- [ ] #9: Quick copy keybinds
- [ ] #9: CLI snippet commands
- [ ] #11: Watcher PID/heartbeat files
- [ ] #11: Status detection
- [ ] #11: Status indicator widget

---

## Testing

### Unit Tests
- Action parsing and dispatch
- Key string parsing
- Content type detection patterns
- Statistics computation
- Collection CRUD operations

### Integration Tests
- Keybinding customization flow
- Search mode switching
- Tab filtering
- Multi-feature interactions

### Manual Testing
- All keybinds work with custom mappings
- Tabs filter correctly
- Snippets copy correct entries
- Watcher status updates in real-time
- Preview modes render correctly

---

## Work Log

### 2025-11-27
- Created comprehensive implementation plan
- Analyzed conflicts and dependencies
- Defined implementation order
- Documented all schema changes, new files, and config additions
