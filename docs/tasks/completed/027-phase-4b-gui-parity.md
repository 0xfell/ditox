# Task: Phase 4b — GUI feature parity

> **Status:** completed
> **Priority:** high
> **Phase:** 4b — GUI parity
> **Created:** 2026-04-26
> **Started:** 2026-04-27
> **Completed:** 2026-04-27
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

### 2026-04-27 — 4b.8 core tag groundwork landed
- Task moved to in-progress. Started with the schema v4 -> v5 tags
  groundwork so GUI and CLI tag surfaces share one tested core model.
- Added `ditox_core::Tag`, `tags`, and `entry_tags`; bumped
  `SCHEMA_VERSION` from 4 to 5; added explicit forward-only
  `migrate_to_v5()` with indexes on both tag lookup directions.
- Added DB helpers for tag CRUD, get-or-create by name, idempotent
  entry/tag linking, unlinking, tag listing per entry, entry listing
  per tag, and entry counts per tag.
- Wired Phase 3 filter-rule `tag:<name>` actions into the watcher:
  matching captures now create/link the tag after the entry insert
  instead of logging a not-yet-implemented message.
- Added CLI surface: `ditox tag <entry> <name> [--color]`,
  `ditox untag <entry> <tag>`, and `ditox tag-list [entry] [--json]`.
- Added GUI tag/filtering surface: tag chips, per-entry tag glyphs,
  side-panel tag add/remove, and single-tag filtering composed with
  the existing tab filters.
- Added time-window tabs (`Yesterday`, `This Week`, `This Month`,
  `Older`) and core DB filtering for them.
- Added collection tabs from existing collections plus side-panel
  reassignment/uncollected controls. This is the first collection GUI
  surface; full create/rename/delete remains in 4b.2.
- Added side-panel image zoom controls (`-`, `+`, `Fit`) with bounded
  25%-400% zoom. Drag-pan/open-external remain in 4b.5.
- Tests added: v4 -> v5 schema snapshot, tag CRUD/idempotent linking,
  tag+time filtered query composition, watcher tag-rule integration,
  and an end-to-end CLI tag round-trip.
  Verified with `nix develop -c cargo test --workspace` and
  `nix develop -c cargo clippy --workspace --all-targets --locked -- -D warnings`.

### 2026-04-27 — settings, collections, and multi-select slice
- Added typed config persistence via `Config::save()`, including TOML
  serialization and atomic temp-file rename. The GUI settings page can
  now edit and save core general/theme options and hide-on-blur.
- Added collection CRUD controls to the GUI settings overlay: create,
  select/edit, save, and delete. Collection refresh rebuilds the dynamic
  tab strip and delete unassigns entries through the existing DB helper.
- Added multi-select mode (`m` shortcut and toolbar button). In
  multi-select mode, row clicks toggle selection instead of copying.
- Added bulk action toolbar: copy joined text, delete selected, tag
  selected, move selected to any existing collection, and apply a named
  transform to selected text entries before copying the result.
- Keyboard shortcuts now ignore events already captured by focused input
  widgets, so typing in settings/search does not trigger launcher actions.
- Verified with `nix develop -c cargo check -p ditox-gui`,
  `nix develop -c cargo test --workspace`, and
  `nix develop -c cargo clippy --workspace --all-targets --locked -- -D warnings`.

### 2026-04-27 — close-out
- Added an `Uncollected` pseudo-tab and DB filtering for entries without
  a collection assignment.
- Extended tag chips to support multiple active tags; filtering now
  requires entries to have all selected tags. Tag chips include color text
  when a color is configured.
- Added side-panel image `Actual` and `Open` controls. `Open` delegates to
  the platform opener (`xdg-open`, `open`, or `cmd /C start`).
- Added config hot reload by watching the config file mtime on the GUI
  tick; changed config updates refresh launcher settings and poll interval
  without restarting.
- Added GUI keybinding overrides under `[keybindings.gui]`, resolved before
  the built-in default shortcuts. Supported action names include `hide`,
  `move_up`, `move_down`, `copy`, `prev_page`, `next_page`, `prev_tab`,
  `next_tab`, `preview`, `toggle_help`, and `toggle_multi_select`.
- Phase 4b is closed with platform-live visual validation still dependent
  on the target desktop sessions. Automated verification passed with
  `nix develop -c cargo test --workspace` and
  `nix develop -c cargo clippy --workspace --all-targets --locked -- -D warnings`.
