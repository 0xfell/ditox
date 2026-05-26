# Ditox Fresh-Start TODO

This file tracks what remains after the initial OpenTUI + Zig scaffold.

## 1. Make The MVP Usable End-To-End

Implemented with fake-tool smoke coverage:

- `ditox launch` captures the active Hyprland window before opening the TUI.
- `ditox` with no arguments launches the configured TUI command with the same target-window handoff.
- `ditox keep` and `ditox launch --keep` launch the same TUI command while setting `DITOX_TUI_EXIT_AFTER_PASTE=false`.
- `ditox -enable-real-time`, `ditox launch --enable-real-time`, and `ditox tui --enable-real-time` launch the TUI with `DITOX_TUI_REFRESH_MS=250`.
- The captured target window is passed into the TUI process as `DITOX_TARGET_WINDOW` through the child environment, not shell-prefix string injection.
- `Enter` in the TUI copies the selected entry and pastes it into the captured app.
- `Ctrl+Y` remains copy-only without paste-back.
- `ditox --auto-paste` sends the configured paste keybind through Hyprland.
- `ditox -kill` terminates the watcher PID recorded in storage.
- `ditox -clean` aliases the storage repair/cleanup path, sanitizes stored text rows, and removes image rows whose blob file is missing.
- `ditox -pause <duration>` accepts Clipse-style `ms`, `s`, `m`, and `h` durations.
- Backend config accepts TOML and JSON files, including Clipse-style `configuration.json` top-level aliases for `maxHistory`, `deleteAfter`, `allowDuplicates`, `pollInterval`, `maxEntryLength`, `historyFile`, `tempDir`, `excludedApps`, `excludedWindows`, and `autoPaste`; relative storage paths resolve from the config file's directory, with home/XDG prefixes expanded.
- The smoke suite verifies:
  - Ditox writes selected text/image bytes with `wl-copy`.
  - Ditox refocuses the target window with `hyprctl dispatch focuswindow address:<target>`.
  - Ditox dispatches `hyprctl dispatch sendshortcut` with the configured paste shortcut.
  - `ditox launch`, `ditox keep`, `ditox launch --keep`, `ditox -enable-real-time`, `ditox tui --enable-real-time`, and no-argument `ditox` forward the captured target window into the launched TUI command.
  - Keep-open launch variants set `DITOX_TUI_EXIT_AFTER_PASTE=false` for the launched TUI.
  - Real-time launch variants set `DITOX_TUI_REFRESH_MS=250` for the launched TUI.
  - Clipse-style backend aliases enable duplicate history, max-history pruning, watcher poll intervals, preview length limits, relative SQLite history paths, relative image temp directories, and capture exclusion lists.

Remaining workflow validation:

- Manually verify the full loop inside a real Hyprland session:
  - user presses compositor keybind.
  - terminal opens with Ditox TUI.
  - user selects an entry.
  - previous application receives the pasted clipboard content.
  - TUI exits or shows a clear result.

## 2. Polish The OpenTUI Interface

Initial visual polish is implemented:

- Centralized semantic theme tokens with dark and light variants.
- Pager-style UI configuration for compact mode, dense spacing defaults, panel sizing, metadata, preview length, and scrollbar visibility.
- Structured OpenTUI components for shell, header, history list, preview pane, status line, and overlays.
- Improved layout sizing, list scrolling, row truncation, preview behavior, and empty states.
- Semantic status colors and clearer runtime error messages for `ditoxd`, `wl-copy`, `hyprctl`, and paste failures.
- `@opentui/keymap` is the default key handling layer.
- Search, help, delete, and clear confirmations now share overlay structure while each overlay mode can override its own surface styling.
- Presentation helpers have direct Bun test coverage.
- File-backed TUI customization is implemented through `DITOX_TUI_CONFIG` or `~/.config/ditox/tui.json`.
- `tui/tui-config.schema.json` documents the file-backed TUI config contract, including supported label keys, is linked from the example config for editor completion, and is covered by schema drift tests.
- Theme presets (`ditoxDark`, `ditoxLight`, `groknight`, `grokday`, `tokyonight`, `rosepine`), terminal alt-screen/screen-mode/background/title/cursor behavior, theme tokens, labels, filter names, search/query prompts/templates/cursor, clear-kind names, delete/clear/confirm prompt templates, header selection templates, header/status line templates, header-line and status-line placeholder tone routing, row pinned markers, row metadata templates/placeholders/visibility, row metadata hash length, bounded row metadata slots for long templates, entry ID prefixes, preview/list title templates, preview metadata/gutters/separators/templates/placeholders, image metadata preview field ordering/visibility, header separators, key display separators, row field widths/gaps/vertical spacing, row/list/preview/full-preview/split-pane spacer surfaces, compact-mode dense layout defaults, alternate-row striping, selected+marked row styling, history load limit, search-match highlighting, empty-state helper visibility, structural header/status/overlay heights/placement and visibility, header/status/overlay vertical padding, independent list/split-preview/full-preview panel padding, independent search/danger/help overlay padding with legacy global fallbacks, pane width minima/insets, split-pane gap, split preview visibility, split/full preview metadata and gutter visibility, split/full preview text width insets, split/full image mode/renderer/block glyph/background/notice/max sizing/row insets/source notices/fallback prefixes and separators/line spacing, split/full preview metadata heights and horizontal/vertical padding, scrollbar width/glyphs, text truncation markers, whitespace replacement, title padding, help key width, byte and age units, image fallback/protocol notice reasons, watcher error separators, status hint templates, operation/view/entry-count status copy/templates, runtime error copy, per-surface semantic tone colors and text attributes, independent header/list/split-preview/full-preview/search/danger/help border visibility/title visibility/border style/title alignment with legacy global fallbacks, list-position and preview bottom-title visibility and alignment, row markers, scrollbar glyphs, status separator including empty glyph values, layout, and keybindings are configurable.
- Search mode has a Clipse-style configurable `searchCopyMatches` / `ctrl+s` yank action for copying every current match.
- Search mode refreshes results live by default with configurable debounce, and file/env behavior controls whether opening search clears the current query and whether cancelling restores the previous query.
- Visible literal and fuzzy-subsequence search matches are highlighted in row previews with the configured list/selected-row search color, and highlighting can be disabled without disabling search.
- Filter cycling order is configurable with `filterOrder`, so users can reorder or omit filters from the cycle.
- Startup filter, pinned-only mode, and initial query are configurable through the `startup` block and env overrides.
- Paste/choose exits the TUI by default like Clipse, with file/env behavior toggles for keeping paste, copy, bulk-copy, or search-copy actions open after success.
- Clipse-style CLI aliases are available for supported operations: `-a`, `-c`, `-p`, `--wl-store`, `--auto-paste`, `-clear`, `-clear-all`, `-clear-text`, `-clear-images`, `--output-all`, `-clean`, `-kill`, `-pause`, `-listen`, `-listen-shell`, and `-enable-real-time`; platform-specific `-listen-x11` / `-listen-darwin` aliases are recognized with explicit unsupported messages; no-argument `ditox` launches the TUI like Clipse, and `ditox keep` / `ditox launch --keep` launch it without exiting after paste.
- TUI history load limit is configurable through `historyLimit` / `DITOX_TUI_HISTORY_LIMIT`.
- File config accepts common Clipse keybinding aliases such as `choose`, `filter`, `clearSelected`, `togglePin`, `togglePinned`, and `yankFilter`; `clearSelected` clears marked rows instead of deleting entries.
- Key display labels are configurable separately from keybindings, so help/status hints can rename keys such as space, escape, enter, and arrows.
- Help overlay row ordering and visibility are configurable with `helpOrder`, including opt-in rows for quit, output, preview scrolling/back, search editing, and confirmation choices.
- Status hint templates can reference paste/copy/preview/search/filter/pinned/delete/output/help/quit key groups, with separate search/preview/confirm-mode templates for apply/back/scroll/confirm actions.
- Header placeholder colors are configurable with `headerLineTones`, so brand, filter, query, mode, labels, and separators can use explicit header surface tones or dynamic `auto` colors.
- Status tone matchers are configurable, so localized or theme-specific status copy can still use success, warning, and error colors.
- Status-line placeholder colors are configurable with `statusLineTones`, so operation, watcher, hint, and separator segments can use explicit status surface tones or dynamic `auto` colors.
- Overlay border colors are configurable with `overlayBorderTones`, so search, danger, and help overlay chrome can use explicit overlay surface tones.
- Overlay content colors are configurable with `overlayContentTones`, so search prompt/query/cursor text, delete/clear prompts, confirmation hints, and help key/action text can use explicit overlay surface tones.
- List content colors are configurable with `listContentTones`, so row markers, metadata, preview text, search matches, empty-state copy, and scrollbar cells can use explicit semantic tones.
- Preview content colors are configurable with `previewContentTones`, so split/full preview borders, empty states, image fallback/notice text, gutters, semantic content lines, and metadata can use explicit semantic tones.
- File config accepts common Clipse top-level TUI aliases for `maxEntryLength`, `pollInterval`, `enableMouse`, `enableDescription`, and `imageDisplay.type`; `maxEntryLength` caps row preview text, Clipse `imageDisplay.scaleX` / `scaleY` / `heightCut` map to block preview sizing hints, and Kitty/Sixel aliases are preserved as explicit modes that render through capability-aware labeled block fallbacks until OpenTUI exposes a stable protocol renderer path.
- File config accepts Clipse-style `themeFile` values and maps `custom_theme.json` colors onto Ditox theme tokens and surface styles, while native Ditox style overrides keep precedence.
- Per-surface styles are configurable for shell, header, list, alternate rows, selected row, selected+marked row, marked row, row spacers, empty states, preview, preview gutter, preview metadata, preview spacers, full preview, full preview gutter, full preview metadata, full-preview spacers, base/search/danger/help overlays, status line, scrollbar, and split-pane gap, including per-surface secondary/success/warning/error/search/favorite/image tones and standard terminal attributes.
- Default browse-mode keybindings are conflict-free: `space` opens full preview, while `x` / `s` handle mark/select workflows.
- Page up/down, home/end, select-single, and range select are available from the keymap.
- Delete confirmation handles selected entries and warns when pinned entries are included.
- Pinned-only view is available from the keymap.
- Scrollable full-preview mode is available from the keymap and supports copy/paste.
- Row metadata visibility is configurable for content-first list layouts.
- Header and status line visibility are configurable for minimal picker layouts.
- Panel/overlay titles, list-position titles, and preview bottom titles are independently configurable and aligned.
- Search/help/confirm overlays can be placed below the header or at the bottom with `overlayPlacement`.
- Row markers, scrollbar glyphs, and status separators can be empty strings for quiet layouts.
- Scrollbar width is configurable for wider theme glyphs.
- Full-preview metadata visibility is configurable independently for cleaner reader layouts.
- Full-preview metadata styling is configurable independently from full-preview content.
- Split/full preview metadata height and horizontal/vertical padding are configurable for compact one-line or roomier metadata rows.
- Split/full preview gutter visibility, width, and separator text are configurable for cleaner reader layouts.
- Empty-state helper copy visibility is configurable for quieter empty/search-miss views.
- The split preview pane can be hidden from config for list-only picker layouts.
- Realtime TUI polling is configurable through `refreshIntervalMs` / `DITOX_TUI_REFRESH_MS`.
- Mouse row selection, Ctrl-click marking, Shift-click range selection, and wheel scrolling are available.
- Terminal mouse capture can be disabled through `mouseEnabled` / `DITOX_TUI_MOUSE`.
- Terminal alternate-screen behavior is configurable through `terminal.altScreen`
  or Grok-style `terminal.alt_screen`, with `DITOX_TUI_ALT_SCREEN` and
  `DITOX_TUI_SCREEN_MODE` env overrides; renderer background color is
  configurable through `terminal.backgroundColor` and `DITOX_TUI_BACKGROUND`;
  split-footer height, shutdown clearing, terminal title/cursor presentation,
  Kitty keyboard protocol, renderer FPS, render debounce, and stdin parser
  buffer limits are configurable too.
- TUI clear actions preserve pinned entries by default, with an explicit clear-everything binding.
- Watcher live/paused/stale/stopped state is visible in the TUI status line.
- Refreshed-history entry-count status text is template-configurable.
- PNG, JPEG, GIF first frames, WebP, and uncompressed BMP image entries can render as OpenTUI-supersampled or text-span block previews with metadata fallback; Kitty/Sixel requests keep their configured mode and currently use capability-aware labeled block fallbacks.
- Image preview rows, protocol/source notices, and fallback rows reserve space before text metadata is windowed, so image-heavy panes stay bounded instead of crowding later preview lines.
- The image preview renderer is configurable (`auto`, `opentui`, `text`); custom text block glyphs and transparent-pixel background colors remain configurable for terminals or themes that render alternate block cells more cleanly.
- Image copy/paste restores the stored blob to the clipboard with its MIME type instead of copying the storage hash as text.
- TUI smoke tests now verify the real OpenTUI render path outputs image block cells, including the OpenTUI supersampled renderer.
- TUI smoke tests now verify file-based visual/key-hint customization in the real OpenTUI render path.
- CLI smoke tests verify image entries are copied as image bytes through `wl-copy --type`.
- CLI smoke tests verify Clipse-style aliases for add, copy-input, print-clipboard, wl-store text/image capture, auto-paste, output-all, clean, kill, pause durations, unsupported platform listener messages, no-argument launch, keep-open launch, realtime launch, and clear flows.
- OpenTUI component frame snapshots cover the main shell/list/preview/status composition, search/delete/clear/help overlays, full preview, empty states, image block preview rendering, compact light-theme terminal bounds, and a wide/narrow/full-preview viewport matrix with custom styles.
- Persisted golden text-frame snapshots cover representative shell, help-overlay, and full-preview states under `tui/src/__goldens__`.
- TUI review artifacts can be exported with `DITOX_TUI_ARTIFACTS=1`, producing ignored text-frame, OpenTUI span JSON, SVG visual, and PNG bitmap files under `tui/artifacts/frames/` or `DITOX_TUI_ARTIFACT_DIR`.
- TUI tests cover custom compact pinned markers, markerless rows, alternate-row styling, selected+marked row styling/markers, row metadata templates/placeholders/hash length/bounded fitting, terminal title/cursor config, entry ID prefixes, header selection templates, header/status line templates, header-line and status-line placeholder tone routing, overlay border/content tone routing including search prompt/query/cursor tones, overlay placement, independent panel/overlay padding, independent panel/overlay chrome and title alignment, list content tone routing, preview content tone routing, header/status visibility, title chrome visibility/alignment, empty-state helper visibility/styling, configurable filter cycle order, default and opt-in help row ordering/visibility, search-match row highlighting/toggling, per-surface text attributes, search/delete/clear/confirm prompt templates, mode-specific overlay surface styling, preview/list title templates, split/full preview gutter visibility/styling/widths/separators, preview line spacing, spacer surface styling, preview metadata height/placeholders/padding, image metadata field ordering/visibility, full-preview metadata styling and metadata padding, split/full image mode/rendering/max sizing/source and fallback copy, image preview row budgeting, preview renderer/block glyphs/background/protocol notices, preview separators, header/preview metadata separators/templates, key display labels/separators, row field widths/spacing, scrollbar width/glyphs, size/age units, image fallback separators, watcher error separators, extended and mode-aware status hint templates, operation/entry-count status templates, and status tone matchers.
- Delete-after TTL cleanup removes old non-pinned entries and prunes expired image blobs.
- Storage repair/clean sanitizes persisted text content, recomputes previews/hashes/byte lengths, removes image rows with missing blob files, and drains pending blob-prune records.
- Search uses SQLite FTS plus escaped literal substring fallback and relevance-tiered ordering so exact and prefix matches beat incidental matches.
- Search also includes fuzzy subsequence matches through a SQLite scoring function, with contiguous, acronym, path, and camel-case boundary matches ranked above weaker fuzzy hits.
- SQLite now has explicit `PRAGMA user_version` schema versioning with a tested v1-to-v2 migration path.
- JSON-RPC contracts now expose method-specific request and success schemas, generated TypeScript discriminated unions, and backend method-specific param validation.

Remaining polish:

- Add native Kitty and Sixel image protocol support if OpenTUI exposes a stable inline image renderable/output path; OpenTUI 0.2.15 exposes capability flags and a supersampled block-buffer renderer that Ditox now uses, but not a stable protocol placement surface yet.
- Add true terminal-native screenshot artifacts if OpenTUI exposes a stable screenshot path; text-frame, span JSON, SVG visual, deterministic PNG bitmap, and golden artifacts are now in place.

## 3. Backend Correctness

- Keep realistic clipboard-history search fixtures for path acronyms, camel-case symbols, stack traces, command snippets, and environment/config snippets.
- Keep JSON Schema as the contract source and regenerate TypeScript types.
- Add temp-database integration tests for:
  - add.
  - list.
  - search.
  - get.
  - copy mock path.
  - delete.
  - favorite/unfavorite.
  - clear text/images/all.
  - repair.
- Keep pinned-preserving clear coverage in backend and CLI smoke tests.
- Keep delete-after TTL coverage in backend and CLI smoke tests.

## 4. Clipboard Watcher

Current watcher supports polling, image-first capture, case-insensitive excluded windows/apps,
self-write avoidance, pause/resume state, watcher health, and a stored watcher
PID used by `ditox -kill`. It still needs a stronger runtime coordination model.

- Decide whether watcher stays a long-running process only, or becomes part of a socket daemon.

## 5. Image Support

First implementation should remain metadata-first.

- Capture image data from Wayland.
- Hash image bytes.
- Store blobs content-addressed under `images-v2`.
- Write blobs atomically: temp file, fsync, rename, fsync parent.
- Store metadata in SQLite:
  - MIME.
  - byte length.
  - hash.
  - blob path.
  - dimensions when available.
- Show image metadata in the TUI preview.
- Copy and paste image entries back to the clipboard as MIME-tagged image bytes.
- Keep actual terminal rendering modes:
  - metadata fallback.
  - PNG, JPEG, GIF first-frame, WebP, and uncompressed BMP half-block preview.
- Add actual protocol rendering modes:
  - Kitty protocol.
  - Sixel protocol.
- Keep blob pruning wired to delete, clear, retention, and repair.
- Keep repair/clean text sanitization and missing-image reconciliation covered by storage tests.

## 6. Clipse-Like Power Features

- Multi-select entries in the TUI.
- Range select and select-single entries in the TUI.
- Bulk copy/output selected entries.
- Pinned-only toggle view.
- Scrollable full preview mode.
- Better pin/favorite workflow.
- Clear text, clear images, and clear all from the TUI with confirmation.
- Pinned-safe clear text/images/all from the TUI, plus explicit clear everything.
- Add operational CLI parity:
  - `ditox add`.
  - `ditox copy`.
  - `ditox print`.
  - `ditox clear`.
  - `ditox pause`.
  - `ditox repair`.
  - `ditox status`.
  - `ditox launch`.
  - `ditox keep`.
  - `ditox launch --keep`.
  - no-argument `ditox` TUI launch.
  - `ditox --auto-paste`.
  - `ditox -clean`.
  - `ditox -kill`.
  - `ditox -pause <duration>`.
- Add export/output commands once storage is stable.

## 7. Packaging And Dev Ergonomics

- Make `ditox tui` work outside the repo checkout.
- Decide TUI shipping format:
  - Bun source executed by `bun`.
  - bundled JS.
  - compiled Bun executable.
- Add a single check command that runs:
  - `zig build test`.
  - TypeScript typecheck.
  - Bun tests.
  - TUI build.
- Add install/dev scripts.
- Keep `.envrc` + Nix dev shell working as the primary local development path.
- Keep native commands usable for contributors who do not use Nix.

## Recommended Next Step

Manually verify Kitty/Sixel protocol support opportunities when OpenTUI exposes a stable inline image renderable/output path.
