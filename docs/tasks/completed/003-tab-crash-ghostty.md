# Task: Fix Tab Key Crash in Ghostty Terminal

> **Status:** completed
> **Priority:** high
> **Created:** 2025-11-27
> **Completed:** 2025-11-27

## Description

Ditox TUI crashes when pressing the Tab key in Ghostty terminal. This is likely a terminal compatibility issue with how Tab key events are being handled or processed by the TUI.

## Requirements

- [x] Reproduce the crash in Ghostty terminal
- [x] Identify the source of the crash (keybinding handler, event processing, etc.)
- [x] Fix the Tab key handling to prevent crash
- [x] Ensure Tab key behavior works correctly across terminals (Ghostty, Alacritty, etc.)
- [x] Add defensive handling for unexpected key events

## Implementation Notes

### Root Cause Analysis

The crash was traced to cursor position calculation in `src/ui/search.rs`. When the cursor position exceeded terminal bounds (possible when toggling preview panel or with long search queries), Ghostty's stricter terminal validation caused a crash.

### Changes Made

1. **Cursor position bounds checking** (`src/ui/search.rs:36-49`)
   - Added `saturating_add()` to prevent u16 overflow
   - Added bounds checking to ensure cursor stays within search bar area
   - Only sets cursor position if within valid bounds

2. **Explicit Tab handling in search mode** (`src/ui/mod.rs:210-211`)
   - Added explicit `KeyCode::Tab` handler in search mode
   - Tab now toggles preview panel in both Normal and Search modes consistently

### Key Code Changes

**search.rs** - Defensive cursor positioning:
```rust
// Calculate cursor X position: border(1) + " Search: "(9) + query length
let cursor_offset = 10u16.saturating_add(app.search_query.len() as u16);
let x = area.x.saturating_add(cursor_offset);
let y = area.y.saturating_add(1);

// Only set cursor if within bounds (prevent crash on small terminals or Ghostty)
let max_x = area.x.saturating_add(area.width.saturating_sub(1));
let max_y = area.y.saturating_add(area.height.saturating_sub(1));
if x <= max_x && y <= max_y {
    frame.set_cursor_position(Position::new(x, y));
}
```

**mod.rs** - Tab handling in search mode:
```rust
// Handle Tab in search mode - toggle preview without exiting search
KeyCode::Tab => app.show_preview = !app.show_preview,
```

## Testing

- [x] Test Tab key in Ghostty terminal - should not crash
- [x] Test Tab key in other terminals (if available)
- [x] Verify normal TUI functionality still works after fix (all 132 tests pass)

## Work Log

### 2025-11-27
- Task created based on user report
- Investigated codebase to identify crash source
- Found cursor position calculation in search.rs without bounds checking
- Implemented fixes:
  1. Added bounds checking to cursor position calculation
  2. Added explicit Tab handling in search mode
- All 132 tests pass
- User confirmed fix works in Ghostty - task complete
