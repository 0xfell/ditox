# Task: TUI Polish & Refinements

> **Status:** completed
> **Priority:** medium
> **Created:** 2025-11-27
> **Completed:** 2025-11-27

## Description

Additional TUI polish and refinements remaining from the original UI improvements wishlist (Task 004). These are smaller enhancements that improve the overall user experience and visual clarity.

## Requirements

### Visual Improvements
- [x] Better visual distinction for entry types (icons instead of "txt/img")
  - Text: `T` icon
  - Image: `I` icon
- [x] Line numbers in text preview (optional, toggleable with `L` key)
- [x] Image dimensions display in preview (e.g., "1920x1080")

### Terminal Handling
- [x] Minimum terminal size detection (warn if too small)
  - Minimum: 60 columns x 10 rows
  - Show friendly message instead of panic/crash
- [x] Better narrow terminal handling
  - Graceful degradation when width < 80 (hide tabs)
  - Auto-hide preview pane when width <= 60
  - Hide snippet hints when width < 100

### Status & Messages
- [x] Message timeout (auto-clear after 2-3 seconds)
  - Store message timestamp
  - Clear on next tick if expired
- [x] Last refresh timestamp in status bar
  - "Updated: Xs ago" format

### Help System
- [x] Auto-generate help from keybinding config
  - Dynamically reads from keybinding resolver
  - Reflects custom keybindings if configured
  - Group by action category

## Implementation Summary

### Files Modified

1. **src/entry.rs**
   - Added `icon()` method to `EntryType` for compact type display

2. **src/app.rs**
   - Added `message_time: Option<Instant>` for message timeout
   - Added `last_refresh: Instant` for status bar timestamp
   - Added `show_line_numbers: bool` for line number toggle
   - Added `set_message()`, `is_message_expired()`, `time_since_refresh()`, `toggle_line_numbers()` methods
   - Updated all message assignments to use `set_message()`

3. **src/actions.rs**
   - Added `ToggleLineNumbers` action

4. **src/keybindings.rs**
   - Added `L` keybinding for `ToggleLineNumbers`

5. **src/ui/mod.rs**
   - Updated message clearing to use timeout check
   - Added action handler for `ToggleLineNumbers`
   - Updated `layout::draw` call to pass keybindings

6. **src/ui/layout.rs**
   - Added minimum terminal size constants (60x10)
   - Added `draw_size_warning()` function for small terminal display
   - Added narrow terminal handling (auto-hide tabs at <80, snippets at <100)
   - Added refresh timestamp to status bar
   - Updated `draw()` to accept `KeybindingResolver`

7. **src/ui/preview.rs**
   - Added `get_image_dimensions()` helper function
   - Updated image info to show dimensions
   - Added line number support to `render_wrap_mode()`, `render_scroll_mode()`, `render_truncate_mode()`

8. **src/ui/list.rs**
   - Changed entry type display from `short()` (3 chars) to `icon()` (1 char)
   - Adjusted fixed width calculation

9. **src/ui/help.rs**
   - Complete rewrite to generate help dynamically from `KeybindingResolver`
   - Uses `get_primary_key()` to show current keybindings
   - Grouped by action category

## Testing

All 150 existing tests pass:
- 18 unit tests
- 35 CLI tests
- 15 clipboard tests
- 28 database tests
- 54 entry tests

## Work Log

### 2025-11-27
- Created task from remaining Task 004 items
- Excluded items already implemented in Task 007 (tabs, preview modes, watcher status)

### 2025-11-27 (Implementation)
- Implemented entry type icons (T/I)
- Implemented line numbers toggle (L key)
- Implemented image dimensions display
- Implemented minimum terminal size detection
- Implemented narrow terminal handling (auto-hide tabs/snippets)
- Implemented message timeout (2 seconds)
- Implemented last refresh timestamp in status bar
- Implemented auto-generated help from keybindings
- All tests passing
