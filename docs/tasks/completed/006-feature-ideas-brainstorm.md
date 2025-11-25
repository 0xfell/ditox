# Task: Feature Ideas Brainstorm

> **Status:** completed
> **Priority:** medium
> **Created:** 2025-11-27
> **Completed:** 2025-11-27

## Description

A collection of 20 feature ideas and improvements for ditox, organized by category. Each idea includes a brief description, potential implementation notes, and estimated complexity.

---

## Feature Ideas

### 1. Favorites/Collections System
**Category:** Organization | **Complexity:** Medium

Go beyond pinning with named collections (e.g., "Work", "Code Snippets", "Passwords"). Users could:
- Create custom collections via CLI or TUI
- Move entries between collections
- Filter TUI view by collection
- Each collection could have its own keybind (1-9)

**Implementation:** Add `collection_id` column to entries table, new `collections` table.

---

### 2. Snippet Templates with Placeholders
**Category:** Productivity | **Complexity:** High

Save entries as templates with `{{placeholder}}` syntax:
- When copying a template, prompt user to fill placeholders
- Useful for email templates, code boilerplate, form responses
- Support default values: `{{name:John}}`

**Implementation:** New `template` flag on entries, placeholder parser, input modal in TUI.

---

### 3. Smart Content Detection & Actions
**Category:** UX | **Complexity:** Medium

Detect content types and offer contextual actions:
- **URLs:** Open in browser, copy domain only, preview link metadata
- **Email:** Open mailto, extract address
- **File paths:** Open file manager, copy filename only
- **JSON/Code:** Pretty print, syntax highlight
- **Color codes:** Show color preview swatch

**Implementation:** Pattern matching on content, action menu popup (press `a` for actions).

---

### 4. Entry Merging
**Category:** Productivity | **Complexity:** Low

In multi-select mode, merge selected text entries:
- Configurable separator (newline, comma, space)
- Creates new entry from merged content
- Useful for combining related snippets

**Implementation:** New keybind in multi-select mode, simple string concatenation.

---

### 5. Clipboard History Timeline View
**Category:** TUI | **Complexity:** Medium

Alternative view showing entries on a visual timeline:
- Group by day/hour
- Expandable day sections
- Visual density indicator (busy periods)
- Quick jump between days

**Implementation:** New view mode, date grouping logic, collapsible sections.

---

### 6. Export/Import Functionality
**Category:** Data | **Complexity:** Medium

Export clipboard history for backup or migration:
- Export formats: JSON, CSV, plaintext
- Import from other clipboard managers (GPaste, CopyQ format)
- Selective export (by date range, type, collection)
- Include/exclude images option

**CLI:** `ditox export --format json --since 2024-01-01 > backup.json`

---

### 7. Encryption for Sensitive Entries
**Category:** Security | **Complexity:** High

Mark entries as "sensitive" for encrypted storage:
- Entries encrypted at rest with user-provided key
- Require unlock (password/keyfile) to view
- Auto-lock after timeout
- Never show in plain `list` output

**Implementation:** SQLCipher or manual AES encryption, secure memory handling.

---

### 8. Usage Statistics Dashboard
**Category:** Insights | **Complexity:** Low

Show clipboard usage patterns:
- Most copied entries (by usage count)
- Copy frequency by hour/day
- Text vs image ratio
- Average entry length
- Total data stored

**CLI:** `ditox stats` or TUI dashboard view with charts

---

### 9. Quick Snippets Bar
**Category:** TUI | **Complexity:** Medium

Horizontal bar at top showing frequently used entries:
- 1-9 keybinds for instant access
- Auto-populated from most-used or manually curated
- Configurable slot count
- Visual indicator showing assigned entries

**Implementation:** New UI component, usage tracking, keybind routing.

---

### 10. Regex Search Mode
**Category:** Search | **Complexity:** Medium

Toggle between fuzzy and regex search:
- `/` for fuzzy (current)
- `:` or `/r` for regex mode
- Highlight all matches in preview
- Support capture groups in replace operations

**Implementation:** Add regex engine (already have `regex` crate), mode toggle in search bar.

---

### 11. Watch Mode Status Indicator
**Category:** TUI | **Complexity:** Low

Show real-time watcher status in TUI:
- Green dot when watcher is running
- Last capture timestamp
- Live update when new entry arrives
- Option to pause/resume watcher from TUI

**Implementation:** IPC or file-based status, periodic polling in TUI.

---

### 12. Custom Keybindings
**Category:** Configuration | **Complexity:** Medium

User-configurable keybindings via config file:
```toml
[keybindings]
copy = "Enter"
delete = "d"
search = "/"
toggle_preview = "p"  # changed from Tab
```

**Implementation:** Keybinding parser, remappable action system.

---

### 13. Entry Annotations/Notes
**Category:** Organization | **Complexity:** Low

Add personal notes to entries:
- Searchable notes field
- Displayed in preview panel
- Useful for remembering context
- Quick edit with `n` keybind

**Implementation:** New `notes` column, edit modal in TUI.

---

### 14. Duplicate Detection Threshold
**Category:** Watcher | **Complexity:** Low

Smarter duplicate handling:
- Detect near-duplicates (whitespace differences)
- Configurable similarity threshold
- Option to merge similar entries
- Show "similar entries" count in status

**Implementation:** Levenshtein distance or normalized comparison before insert.

---

### 15. Cross-Device Sync
**Category:** Data | **Complexity:** High

Sync clipboard history across machines:
- Self-hosted sync server option
- End-to-end encryption
- Conflict resolution (newest wins or manual)
- Selective sync (exclude sensitive collections)

**Implementation:** REST API server, sync protocol, conflict handling.

---

### 16. Image OCR & Text Extraction
**Category:** Images | **Complexity:** High

Extract text from images:
- OCR on paste using Tesseract
- Store extracted text as searchable metadata
- Copy extracted text directly
- Language detection

**Implementation:** Optional Tesseract dependency, background OCR processing.

---

### 17. Entry Source Tracking
**Category:** Metadata | **Complexity:** Medium

Track where clipboard content came from:
- Application name (browser, terminal, etc.)
- Window title at copy time
- Filter by source application
- Useful for finding "that thing I copied from Firefox"

**Implementation:** Wayland compositor integration for window info, new metadata column.

---

### 18. Preview Pane Modes
**Category:** TUI | **Complexity:** Low

Multiple preview display options:
- **Wrap:** Current behavior, text wraps
- **Scroll:** Horizontal scroll for long lines
- **Truncate:** Show first N chars with expand option
- **Hex:** Hex dump for binary content
- Cycle with `p` keybind

**Implementation:** Preview mode state, mode-specific renderers.

---

### 19. Clipboard Transformation Pipelines
**Category:** Productivity | **Complexity:** High

Apply transformations before pasting:
- Built-in: lowercase, uppercase, trim, base64, URL encode
- Custom shell command pipeline
- Save as named transformations
- Preview result before pasting

**Example:**
```toml
[transforms]
json_pretty = "jq ."
slug = "tr '[:upper:]' '[:lower:]' | tr ' ' '-'"
```

**Implementation:** Transform definitions, shell execution, preview modal.

---

### 20. TUI Tabs for Quick Filters
**Category:** TUI | **Complexity:** Medium

Tab bar at top for quick filtering:
- `[All]` `[Text]` `[Images]` `[Pinned]` `[Today]`
- Navigate with `[` and `]` or number keys
- Customizable tab filters in config
- Persist last active tab

**Implementation:** Tab component, filter presets, state persistence.

---

## Priority Recommendations

### Quick Wins (Low complexity, high value)
1. **#4 Entry Merging** - Simple addition to multi-select
2. **#8 Usage Statistics** - Fun insights feature
3. **#11 Watch Status Indicator** - Helpful feedback
4. **#13 Entry Annotations** - Adds organization depth
5. **#18 Preview Pane Modes** - Flexibility for power users

### Medium Effort, High Impact
1. **#1 Favorites/Collections** - Major organization upgrade
2. **#3 Smart Content Detection** - Makes ditox feel intelligent
3. **#6 Export/Import** - Essential for backup/migration
4. **#10 Regex Search** - Power user must-have
5. **#20 TUI Tabs** - UX improvement for filtering

### Ambitious but Valuable
1. **#7 Encryption** - Security-conscious users need this
2. **#15 Cross-Device Sync** - Game changer for multi-device users
3. **#16 Image OCR** - Unique differentiator
4. **#19 Clipboard Transformations** - Productivity powerhouse

---

## Implementation Notes

These ideas are meant as inspiration. Each would need its own task file when selected for implementation. Consider:
- User feedback on which features are most wanted
- Maintaining ditox's simplicity and speed
- Not adding features that conflict with the "one thing well" philosophy

## Testing

N/A - This is a brainstorming document.

## Work Log

### 2025-11-27
- Initial brainstorm of 20 feature ideas
- Categorized by type and complexity
- Added priority recommendations
- Selected 10 features for implementation planning (task 007)
- Marked as completed
