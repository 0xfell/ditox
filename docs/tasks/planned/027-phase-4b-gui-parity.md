# Task: Phase 4b — GUI feature parity

> **Status:** planned
> **Priority:** high
> **Phase:** 4b — GUI parity
> **Created:** 2026-04-26
> **Estimated:** 3-4 weeks

## Description

Bring the GUI up to TUI feature parity, plus add the missing
organisational features (tags, time-window filters) that make the GUI
useful for daily power use.

Schema bump: v4 → v5 (tags, entry_tags).

Builds on Phase 4's long-running daemon — these features assume the GUI
keeps state across summons.

## Sub-tasks

### 4b.1 Settings window

Iced page accessible via gear icon. Sections:

- **General** — `max_entries`, `poll_interval_ms`, theme, language.
- **Capture** — mode (all/minimal/custom), format allowlist, exclude
  process patterns, max sizes.
- **Paste** — synthesise on/off, per-app keystroke overrides table.
- **Hotkeys** — global hotkey, in-launcher key bindings, list of
  per-clip hotkeys (managed in Phase 5).
- **Filters** — Phase 3.4 filter rules.
- **Storage** — data_dir, vacuum button, repair button, prune queue
  status.
- **Sync** — placeholder until Phase 6.
- **About** — version, license, links.

Each section is a separate `iced::Element` view. Settings persist to
`config.toml` via `Config::save()`.

### 4b.2 Collections in GUI

Tab strip extends with user-defined collections. UX:

- "+" button opens a modal: name, color picker, optional keybind.
- Right-click on collection tab: rename, delete, change color.
- Entry context menu: "Move to collection ▶".
- Drag entry onto a collection tab to move.
- "Uncollected" pseudo-tab for entries with no collection.

CRUD via existing `ditox-core::collection` module. Collection list
loaded on launcher show.

### 4b.3 Multi-select

- `m` toggles multi-select mode (or a checkbox "Select multiple").
- Click toggles row selection in the mode.
- Shift-click range select.
- Bottom action bar: "Copy joined" (Phase 1 aggregator),
  "Delete", "Move to collection", "Add tag", "Apply transform".

### 4b.4 Per-row favorite toggle

Star icon visible on hover (and always for already-favorite rows).
Click toggles via existing `Database::toggle_favorite`.

### 4b.5 Image zoom in side panel

In the Tab side panel, when entry is an image:

- Mouse wheel: zoom in/out (10% increments, 25%-400% range).
- Click-and-drag: pan.
- "Fit" / "Actual size" buttons.
- Double-click: open in default image viewer (`open` crate).

### 4b.6 Theming via TOML

Currently `[ui.theme]` is TUI-only. Wire the GUI to read it too:

```toml
[ui.theme]
selected_bg     = "#7aa2f7"
selected_fg     = "#1a1b26"
border          = "#565f89"
text            = "#c0caf5"
muted           = "#565f89"
search_match_bg = "#bb9af7"
favorite_glyph  = "#ffd700"
collection_default_bg = "#414868"
```

Hot-reload via `notify` crate watching `config.toml`. Live update
without restart.

### 4b.7 Customizable keybindings in GUI

Reuse `ditox-tui/src/keybindings.rs` resolver. iced `keyboard::Event`
matched against the resolved table.

Defaults preserved; user overrides in `[keybindings.gui]` section
(separate from `[keybindings]` for TUI to avoid coupling).

### 4b.8 Tags system

Schema v4 → v5:

```sql
CREATE TABLE tags (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    color      TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE entry_tags (
    entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    tag_id   TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);

CREATE INDEX idx_entry_tags_tag ON entry_tags(tag_id);
```

Many-to-many. Orthogonal to collections.

UX:
- Side panel "Tags" section: chip-style tag editor.
- Type-ahead: existing tags suggested.
- Enter creates a new tag if not found.
- Click a chip in the tag list (top of side panel) to filter.
- Multiple tag chips selected = "show entries with all these tags".

CLI: `ditox tag <entry> <tag-name>`, `ditox untag`, `ditox tag-list`.

### 4b.9 Time-window filter chips

Tab strip gains a secondary row of time chips:

- Today (existing)
- Yesterday
- This Week
- This Month
- Older

Mutually exclusive with each other (but can combine with collection
filter and tag filter).

`TabFilter` enum extended:

```rust
enum TimeFilter { All, Today, Yesterday, ThisWeek, ThisMonth, Older }
```

Resolved into a SQL `created_at >= ? AND created_at < ?` clause.

## Acceptance criteria

- [ ] All GUI features work on Hyprland (post-Phase 4) and Windows.
- [ ] Settings changes persist and take effect without restart.
- [ ] Collections fully manageable from GUI; CLI behaviour preserved.
- [ ] Multi-select works with all bulk actions (copy, delete, tag,
      transform).
- [ ] Theming hot-reloads via `notify`.
- [ ] Tags survive a v4 → v5 migration.
- [ ] Time chips accurately filter (Today shows entries from last 24h,
      Yesterday shows 24-48h ago, etc.).

## Implementation Notes

The schema migration is small (two tables, one index). Add to existing
migration table.

For tag chip UX, consider iced `Container` widgets with custom style.
Color picker for tag/collection: a small palette (Material colors)
plus hex input.

## Work Log

### 2026-04-26
- Task file created (epic).
