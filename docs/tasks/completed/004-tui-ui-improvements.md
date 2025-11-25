# Task: TUI UI Improvements and Ideas

> **Status:** completed
> **Priority:** medium
> **Created:** 2025-11-27
> **Completed:** 2025-11-27

## Description

A collection of UI/UX improvements for the Ditox TUI to enhance usability, visual appeal, and functionality. This task documents potential enhancements identified from analyzing the current implementation.

## Requirements

### Navigation & Scrolling
- [x] Add vertical scrolling support for entries beyond terminal height (already handled by Ratatui)
- [x] Show scroll indicator/scrollbar in the list panel
- [x] Add page up/down navigation (Ctrl+U/Ctrl+D or PgUp/PgDn)
- [ ] Jump-to-entry by number (e.g., type "42" to jump to entry 42)

### Layout & Responsive Design
- [ ] Configurable list/preview split ratio (default 50/50, allow 60/40, 70/30, etc.)
- [ ] Collapsible preview panel with configurable default state
- [ ] Minimum terminal size detection with friendly error message
- [ ] Better handling of very narrow terminals (< 40 cols)

### Visual Enhancements
- [ ] Syntax highlighting for code snippets in preview
- [ ] Better visual distinction between entry types (icons: 📋 text, 🖼️ image)
- [ ] Line numbers in text preview for long entries
- [ ] Truncation indicator showing "...N more lines" at bottom of preview
- [x] Entry count in status bar (e.g., "42 entries" or "5/42 filtered")

### Search Improvements
- [ ] Search history (up/down arrows to cycle previous searches)
- [x] Search result count display ("3 matches")
- [x] Highlight search matches in list and preview
- [ ] Filter by type toggle (e.g., only images, only text)
- [ ] Regex search mode toggle

### Theming & Customization
- [ ] Built-in theme presets (dark, light, high-contrast, catppuccin, dracula, etc.)
- [ ] Per-widget color overrides in config
- [ ] Option to show/hide specific columns (pin, index, type, timestamp)
- [ ] Configurable timestamp format (relative vs absolute)

### Mouse Support
- [x] Click to select entries in list
- [x] Scroll wheel support for navigation
- [ ] Click on preview to expand
- [x] Double-click to copy entry

### Status & Feedback
- [ ] Watcher daemon status indicator (● connected / ○ disconnected)
- [ ] Last refresh timestamp
- [ ] Clipboard operation feedback with timeout (message auto-clears after 2s)
- [ ] Loading indicator for database operations

### Image Handling
- [ ] Image gallery mode (arrow keys to cycle through image entries)
- [ ] Image zoom controls in expanded view
- [ ] Show image dimensions in preview metadata
- [ ] Thumbnail grid view option for image-heavy histories

### Advanced Features
- [x] Multi-select mode for batch operations (delete, copy multiple)
- [ ] Undo last delete action
- [ ] Quick filters (today, this week, pinned only, images only)
- [ ] Sort options (by date, by type, by usage count)
- [ ] Entry tagging/categorization

### Help & Discoverability
- [ ] Auto-generate help from keybinding config
- [ ] Command palette (fuzzy searchable commands like VS Code's Ctrl+Shift+P)
- [ ] First-run tutorial/tips
- [ ] Keyboard shortcut hints on hover (if mouse enabled)

## Implementation Notes

### Priority Recommendations

**High impact, moderate effort:**
1. Vertical scrolling - essential for large histories
2. Entry count display - quick context
3. Search match highlighting - better UX
4. Built-in theme presets - popular request

**High impact, higher effort:**
1. Mouse support - makes TUI more accessible
2. Multi-select mode - batch operations are common
3. Image gallery mode - image users need this

**Low hanging fruit:**
1. Entry count in status bar
2. Timestamp format config
3. Search result count
4. Page up/down navigation

### Technical Considerations

- Scrolling requires tracking `scroll_offset` in `App` state
- Mouse support uses Crossterm's `EnableMouseCapture`
- Syntax highlighting can use `syntect` crate
- Theme presets can be embedded TOML or hard-coded Theme structs
- Multi-select needs `selected: Vec<usize>` instead of single `selected: usize`

## Testing

- Manual testing for each visual change
- Verify rendering in multiple terminals (Ghostty, Kitty, WezTerm, Alacritty)
- Test edge cases: 0 entries, 1000+ entries, very long entries
- Test image handling with various formats and sizes

## Work Log

### 2025-11-27
- Created task from codebase analysis
- Documented current limitations and improvement opportunities
- Prioritized features by impact and effort
- Implemented the following improvements:
  - **Entry count in status bar**: Shows "42 entries" or "5/42 filtered" when searching
  - **Search result count**: Shows "(3 matches)" in search bar
  - **Page up/down navigation**: PgUp/PgDn and Ctrl+U/Ctrl+D support
  - **Scroll indicator**: Scrollbar visible in list when entries exceed height
  - **Search match highlighting**: Matched characters highlighted in yellow in list and preview
  - **Mouse support**: Click to select, scroll wheel navigation, double-click to copy
  - **Multi-select mode**: Press 'm' to enter, Space to toggle selection, 'v' to select all, batch delete/copy
- Updated help menu with new keybindings
- All 132 tests passing
