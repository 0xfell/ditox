# GUI Improvements Plan

**Status:** in-progress
**Created:** 2025-12-08
**Priority:** High

## Overview

Comprehensive improvement of the ditox-gui to match TUI features and fix visual issues.

## Current Issues

1. **Visual Issues (from screenshot)**
   - Missing/broken icons/characters (likely Unicode rendering)
   - Status bar icons showing as boxes or missing characters
   - Type badges may not render correctly

2. **Missing Features (compared to TUI)**
   - Tab filtering (All, Text, Images, Favorites, Today)
   - Toggle favorite functionality
   - Image preview
   - Settings window
   - Help overlay
   - Keyboard shortcuts display
   - Pagination
   - Preview pane

## Implementation Plan

### Phase 1: Fix Visual Issues
**Scope:** Icon and character rendering fixes

1. **Replace Unicode characters with proper icons/text**
   - Replace `⋮⋮` drag handle with a proper widget or ":::"
   - Replace `●` watcher indicator with text "ON" or colored dot widget
   - Replace `⟍` resize indicator with proper widget
   - Replace `★` favorite star with "FAV" or proper icon
   - Replace `×` delete button with "X" text
   - Ensure consistent font rendering

2. **Improve type badges**
   - Make "T" and "I" badges more visually distinct
   - Add proper icons or use well-supported Unicode

### Phase 2: Add Tab Bar (Filtering)
**Scope:** Implement tabbed filtering like TUI

1. **Add TabFilter enum to GUI**
   - Reuse `TabFilter` from ditox-core
   - Track `active_tab` in DitoxApp state

2. **Create tab bar widget**
   - Horizontal row of tabs: All | Text | Images | Favorites | Today
   - Clickable tabs with visual selection state
   - Keyboard navigation: `[` and `]` or arrow keys

3. **Wire up filtering**
   - Use `db.get_page_filtered()` for efficient DB-level filtering
   - Update entry count display per tab

### Phase 3: Add Toggle Favorite
**Scope:** Allow users to favorite/unfavorite entries

1. **Add favorite button to each entry row**
   - Star icon/button that toggles on click
   - Visual distinction for favorited entries

2. **Add keyboard shortcut**
   - `S` key to toggle favorite on selected entry

3. **Wire to database**
   - Call `db.toggle_favorite(id)`
   - Refresh entry list after toggle

### Phase 4: Add Settings Window
**Scope:** Modal settings dialog for configuration

1. **Create Settings view/state**
   - Toggle for "Run on Startup" (already exists in tray)
   - Poll interval setting
   - Max entries setting
   - Theme/appearance options (future)

2. **Add settings button/shortcut**
   - Settings icon in UI
   - `Ctrl+,` shortcut

3. **Persist settings**
   - Save to `~/.config/ditox/config.toml`
   - Use existing `Config` struct

### Phase 5: Add Help Overlay
**Scope:** Show keyboard shortcuts help

1. **Create help overlay widget**
   - Modal popup showing all shortcuts
   - Organized by category (Navigation, Actions, etc.)

2. **Add toggle**
   - `?` or `F1` key to show/hide
   - Close on Escape

### Phase 6: Improve Preview
**Scope:** Better content preview

1. **Add preview pane** (optional)
   - Side panel showing full content of selected entry
   - Toggle with `Tab` or `P` key

2. **Image preview support**
   - Show thumbnail for image entries
   - Use iced's image widget

### Phase 7: Pagination (Optional)
**Scope:** Handle large history efficiently

1. **Add page navigation**
   - Page indicator: "Page 1/5"
   - PgUp/PgDown for navigation
   - Limit displayed entries to 20 per page

## Technical Details

### File Changes

```
ditox-gui/src/
├── app.rs           # Main application (major changes)
├── main.rs          # Entry point (no changes)
├── startup.rs       # Auto-launch (no changes)
├── views/           # NEW: Separate view modules
│   ├── mod.rs
│   ├── main_view.rs
│   ├── settings.rs
│   └── help.rs
└── widgets/         # NEW: Reusable widgets
    ├── mod.rs
    ├── tab_bar.rs
    ├── entry_item.rs
    └── preview.rs
```

### Dependencies to Add

None required - iced has all needed widget support.

### State Changes

```rust
pub struct DitoxApp {
    // Existing fields...

    // New: Tab filtering
    tabs: Vec<TabFilter>,
    active_tab: usize,

    // New: View mode
    show_settings: bool,
    show_help: bool,
    show_preview: bool,

    // New: Pagination
    current_page: usize,
    total_count: usize,
}
```

### New Messages

```rust
pub enum Message {
    // Existing...

    // Tab operations
    SelectTab(usize),
    NextTab,
    PrevTab,

    // Entry operations
    ToggleFavorite(String),

    // View operations
    ShowSettings,
    HideSettings,
    ToggleHelp,
    TogglePreview,

    // Settings
    SetPollInterval(u64),
    SetMaxEntries(usize),

    // Pagination
    NextPage,
    PrevPage,
}
```

## Implementation Order

1. **Phase 1** - Fix visual issues (quick wins, improves immediate UX)
2. **Phase 2** - Tab bar (core feature, high value)
3. **Phase 3** - Toggle favorite (complements tab bar)
4. **Phase 4** - Settings window (user-requested)
5. **Phase 5** - Help overlay (good for discoverability)
6. **Phase 6** - Preview pane (nice to have)
7. **Phase 7** - Pagination (performance for large history)

## Work Log

- 2025-12-08: Created plan, analyzed codebase
- 2025-12-08: Implemented all major features:
  - **Phase 1**: Fixed Unicode rendering - replaced problematic Unicode with ASCII (⋮⋮→::, ★→*, ●→ON badge, ×→X, ⟍→removed)
  - **Phase 2**: Added tab bar with 5 tabs (All, Text, Images, Favorites, Today) with DB-level filtering
  - **Phase 3**: Added clickable favorite toggle button on each entry row
  - **Phase 4**: Added settings modal (startup toggle, poll interval display, max entries display)
  - **Phase 5**: Added help overlay with keyboard shortcuts organized by category
  - Added pagination support with page navigation (PgUp/PgDn)
  - Added keyboard shortcuts: [ ] for tab navigation, ? for help toggle
  - Improved status bar with ON indicator badge, page count
