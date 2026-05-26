# Clipse Parity And TUI Customization Plan

Ditox should keep its Zig plus OpenTUI architecture, but the user-facing behavior should converge on the parts of Clipse that make it feel complete: fast history selection, rich previews, configurable shortcuts, safe destructive actions, and full visual customization.

## Current Parity Snapshot

Implemented today:

- Text history with SQLite persistence.
- Explicit SQLite schema versioning and a tested v1-to-v2 migration path.
- Method-specific JSON-RPC request/result contracts with generated TypeScript discriminated unions and backend param validation.
- FTS-backed search with escaped literal substring fallback, boundary-aware fuzzy subsequence scoring, and relevance-tiered ordering for exact, prefix, substring, and fuzzy matches.
- Metadata-first image capture and storage.
- Favorites/pins in storage and list ordering.
- Basic filters for all, text, images, favorites, and today.
- Configurable filter cycling order, including omitting filters from the cycle.
- Multi-select toggle plus bulk copy/output.
- Clipse-style search-match yank/copy from search mode through configurable `searchCopyMatches`.
- Live search refreshes results while typing by default, with configurable debounce and query open/cancel behavior for users who prefer explicit Enter-only search.
- Visible literal and fuzzy-subsequence search matches are highlighted in row previews using the configured row search tone, with a config switch for quieter unhighlighted search.
- Copy-only and Hyprland paste-back paths.
- Clipse-style CLI aliases for supported command flows: `-a`, `-c`, `-p`, `--wl-store`, `--auto-paste`, `-clear`, `-clear-all`, `-clear-text`, `-clear-images`, `--output-all`, `-clean`, `-kill`, `-pause`, `-listen`, `-listen-shell`, and `-enable-real-time`; platform-specific `-listen-x11` / `-listen-darwin` aliases are recognized and report that only the Wayland listener is implemented.
- `ditox` with no arguments, `ditox launch`, `ditox launch --keep`, `ditox keep`, `ditox -enable-real-time`, and `ditox tui --enable-real-time` capture the active Hyprland window and forward it to the TUI as `DITOX_TARGET_WINDOW` through the child environment; keep-open launch variants set `DITOX_TUI_EXIT_AFTER_PASTE=false` for the child, and realtime launch variants set `DITOX_TUI_REFRESH_MS=250`.
- `ditox --auto-paste` sends the configured paste keybind through Hyprland; the direct paste-back path uses the same configurable keybind after writing the selected entry to the clipboard.
- Paste/choose exits the TUI by default like Clipse, with file/env behavior toggles for keep-open workflows.
- Clear all/text/images confirmations.
- OpenTUI visual shell with semantic theme tokens.
- File-backed TUI config with six built-in theme presets, per-token theme overrides, per-surface style, layout, chrome, label, and keybinding overrides.
- A checked `tui-config.schema.json` is linked from the example config for editor completion and tested against runtime surface, layout, chrome, label, behavior, theme, status-tone, and keybinding names.
- File-backed keybinding config accepts common Clipse aliases including `choose`, `filter`, `clearSelected`, `togglePin`, `togglePinned`, and `yankFilter`; `clearSelected` clears marked rows instead of deleting entries.
- File-backed key display labels can rename normalized key names in help/status hints without changing the actual bindings.
- File-backed `helpOrder` can reorder or hide help-overlay rows for compact or team-specific picker layouts.
- File-backed `helpOrder` can opt into non-default help rows for quit, output, preview scrolling/back, search editing, and confirmation choices.
- File-backed status hint templates can reference paste, copy, preview, search, filter, pinned, delete, output, help, and quit key groups, with separate search, preview, and confirm-mode templates for apply, back, scroll, and confirm actions.
- File-backed status tone matchers can map localized or theme-specific status text to success, warning, and error colors.
- File-backed list content tone routing can assign explicit tones to row markers, metadata, preview text, search matches, empty-state copy, and scrollbar cells.
- File-backed preview content tone routing can assign explicit tones to split/full preview borders, empty states, image fallback/notice text, gutters, semantic content lines, and metadata.
- File-backed config accepts common Clipse top-level TUI aliases for `maxEntryLength`, `pollInterval`, `enableMouse`, `enableDescription`, and `imageDisplay.type`; `maxEntryLength` caps row preview text, Clipse `imageDisplay.scaleX` / `scaleY` / `heightCut` map to block preview sizing hints, and Kitty/Sixel image display aliases are preserved as explicit modes that currently render through capability-aware labeled block fallbacks until OpenTUI exposes a stable protocol renderer path.
- File-backed config accepts Clipse-style `themeFile` values and maps `custom_theme.json` colors onto Ditox theme tokens and per-surface styles, while explicit Ditox style overrides keep precedence.
- Row metadata templates now support the same practical entry placeholders as preview metadata, including id, source app, MIME, dimensions, blob path, short hash, full hash, age, size, and pin slots, with configurable row hash length.
- TUI history load limit is file/env configurable and is sent to the backend `entries.list` RPC request.
- Startup filter, pinned-only mode, and initial query are file/env configurable before the first backend refresh.
- Page up/down, home/end, select-single, and range select navigation.
- Delete confirmation for the selected set, including pinned-entry warning text.
- Pinned-only toggle view backed by the favorites filter.
- Scrollable full preview mode with independent preview navigation and copy/paste bindings.
- Configurable full-preview metadata visibility for cleaner reader layouts.
- Configurable split preview pane visibility for list-only picker layouts.
- Pinned-preserving clear semantics through backend/RPC, CLI `--keep-pinned`, and safe TUI clear confirmations.
- Configurable live polling while browsing, skipped during search, preview, and confirmations.
- Mouse row selection, Ctrl-click marking, Shift-click range selection, list wheel scrolling, and full-preview wheel scrolling.
- Conflict-free default browse keymap: `space` opens Clipse-style full preview, while `x` / `s` handle mark/select workflows.
- Configurable mouse enablement for users who do not want terminal mouse capture.
- Configurable terminal alternate-screen behavior through Ditox `terminal.altScreen`, Grok-style `terminal.alt_screen`, direct `terminal.screenMode`, renderer `terminal.backgroundColor`, split-footer `terminal.footerHeight`, shutdown clearing, terminal title/cursor presentation, Kitty keyboard protocol, renderer FPS, render debounce, stdin parser buffer limits, and env overrides.
- Watcher live/paused/stale/stopped state in the status line with configurable labels.
- Hyprland watcher capture exclusions for app class and window title, with Clipse-style case-insensitive substring matching and default password-manager app exclusions.
- Theme presets (`ditoxDark`, `ditoxLight`, `groknight`, `grokday`, `tokyonight`, `rosepine`), terminal screen/background/title/cursor behavior, compact-mode dense layout defaults, row kind labels, compact pinned markers, markerless rows, selected+marked row markers, alternate-row striping, row metadata templates/visibility/bounded fitting, row marker/metadata/preview/search-match/empty/scrollbar tone routing, preview border/empty/fallback/gutter/content/metadata tone routing, entry ID prefixes, filter names/order, startup filter/query state, action exit behavior, search/query prompts/templates/cursor, search-match highlighting, delete/clear/confirm prompt templates, clear-kind names, empty previews, header selection templates, header/status line templates, header-line and status-line placeholder tone routing, overlay border/content tone routing including search prompt/query/cursor colors, preview/list title templates, preview metadata/gutters/separators/templates/placeholders, split/full preview gutter widths/separators, image metadata preview field ordering/visibility, header separators, key display separators, row field widths/gaps/vertical spacing, row/list/preview/full-preview/split-pane spacer surfaces, history load limit, structural header/status/overlay heights/placement and visibility, header/status/overlay vertical padding, independent list/split-preview/full-preview panel padding, independent search/danger/help overlay padding with legacy global fallbacks, independent header/list/split-preview/full-preview/search/danger/help border visibility/title visibility/border style/title alignment with legacy global fallbacks, list-position and preview bottom-title visibility/alignment, pane width minima/insets, split/full preview gutter visibility, split/full preview text width insets, split/full image mode/renderer/block glyph/background/notice/max sizing/row insets/source notices/fallback prefixes and separators/line spacing, split/full preview metadata height and horizontal/vertical padding, full-preview metadata styling, scrollbar width/glyphs, text truncation markers, whitespace replacement, title padding, help key width, default and opt-in help rows, byte and age units, image fallback/protocol notice reasons, watcher error separators, extended and mode-aware status hint templates, operation/view/entry-count status copy/templates, runtime error copy, and per-surface semantic tone colors and text attributes are configurable.
- PNG, JPEG, GIF first frames, WebP, and uncompressed BMP image entries render as configurable OpenTUI-supersampled or text-span block previews with configurable cell glyphs, metadata fallback, capability-aware Kitty/Sixel fallback notices, and row budgeting that reserves image/notice/fallback rows before text metadata is windowed.
- Image entry selection restores stored image bytes to the clipboard with the entry MIME type.
- Delete-after TTL cleanup removes old non-pinned entries and prunes expired image blobs.
- Repair/clean sanitizes persisted text rows, recomputes previews/hashes/byte lengths, removes image entries whose blob files are missing, and drains pending blob-prune records.
- The watcher records its PID in storage on startup so `ditox -kill` can terminate the exact known watcher process without broad command matching.
- `ditox -pause <duration>` accepts Clipse-style `ms`, `s`, `m`, and `h` durations while keeping the existing `pause <milliseconds>` command compatible.
- Backend config accepts TOML and JSON files, including Clipse-style `configuration.json` top-level aliases for `maxHistory`, `deleteAfter`, `allowDuplicates`, `pollInterval`, `maxEntryLength`, `historyFile`, `tempDir`, `excludedApps`, `excludedWindows`, and `autoPaste`, with smoke coverage for duplicate history, max-history pruning, config-relative storage/image paths, JSON config loading, auto-paste settings, and watcher capture exclusions.
- Search ranking is covered against realistic clipboard-history samples for path acronyms, camel-case symbols, stack traces, command snippets, and environment/config snippets.
- The CLI smoke suite verifies Clipse-style aliases for add, copy-input, print-clipboard, wl-store text/image capture, auto-paste, output-all, clean, kill, pause durations, unsupported platform listener messages, no-argument launch, keep-open launch, realtime launch, and pinned-safe/inclusive clear flows.
- The CLI smoke suite verifies image copy uses MIME-tagged image bytes rather than the stored hash.
- The root CLI smoke suite verifies the TUI render path emits image block cells and file-based visual/key-hint customization.
- The TUI unit suite verifies the main shell/list/preview/status composition, search/delete/clear/help overlays, full preview, empty states, image block preview rendering, compact light-theme terminal bounds, and a wide/narrow/full-preview viewport matrix with OpenTUI frame snapshots.
- The TUI unit suite verifies persisted golden text frames for representative shell, help-overlay, and full-preview states.
- `DITOX_TUI_ARTIFACTS=1` exports ignored text-frame, OpenTUI span JSON, SVG visual, and deterministic PNG bitmap artifacts for review.
- The TUI unit suite verifies custom row pinned markers, markerless rows, alternate-row styling, selected+marked row styling/markers, bounded row metadata fitting, list content tone routing, preview content tone routing, terminal title/cursor config, entry ID prefixes, preview separators, header/preview metadata separators, header/status line templates and visibility, header-line and status-line placeholder tone routing, overlay border/content tone routing including search prompt/query/cursor tones, overlay placement, independent panel/overlay padding, independent panel/overlay chrome and title alignment, title chrome visibility/alignment, empty-state styling, split/full preview gutter visibility/styling/widths/separators, preview line spacing, spacer surface styling, preview metadata height/placeholders/padding, image metadata field ordering/visibility, full-preview metadata styling and metadata padding, split/full image mode/rendering/max sizing/source and fallback copy, image preview row budgeting, preview block glyphs/background/protocol notices, configurable filter cycle order, default and opt-in help row ordering/visibility, search-match row highlighting/toggling, row spacing, scrollbar width/glyphs, key display labels/separators, size/age units, image fallback separators, watcher error separators, extended and mode-aware status hint templates, operation/entry-count status templates, status tone matchers, per-surface semantic color overrides, per-surface text attributes, and mode-specific overlay surface styling.

Not yet 1:1 with Clipse:

- No native Kitty or Sixel image protocol rendering. Ditox accepts and preserves those Clipse config values and sizing hints, reads OpenTUI `kitty_graphics`/`sixel` capability flags for fallback notices, and uses labeled OpenTUI-supersampled or text-span block fallbacks for now because the current Solid/core surface does not expose a stable inline protocol image placement API.
- Command parity is not exact for platform-specific Clipse listener implementations; the X11/macOS aliases are recognized with explicit unsupported messages, and the Wayland/Hyprland command path covers the common supported aliases listed above, no-argument TUI launch, keep-open launch, realtime launch, text/image repair-clean, watcher kill, pause durations, and configurable `--auto-paste`.
- Platform parity is narrower: current Ditox is Linux/Wayland/Hyprland-focused, while Clipse keeps separate Wayland, X11, and macOS listener/display paths.
- No terminal-native screenshot capture workflow yet; OpenTUI currently exposes stable text and span captures, but not a screenshot API. Ditox exports deterministic SVG and PNG visual artifacts from span captures until that exists.

## Phase 1: File-Backed TUI Customization

Goal: every visible TUI surface reads from a resolved config object instead of hardcoded constants.

Status: implemented for the current TUI surface. Keep expanding this config as new surfaces are added.

Deliverables:

- Load JSON config from `DITOX_TUI_CONFIG`, falling back to `$XDG_CONFIG_HOME/ditox/tui.json` or `~/.config/ditox/tui.json`.
- Keep defaults when the file is missing or partially specified.
- Support terminal alternate-screen/main-screen/split-footer behavior with Ditox `terminal.altScreen` and Grok-style `terminal.alt_screen`, direct `terminal.screenMode`, renderer `terminal.backgroundColor`, footer height, shutdown clearing, terminal title/cursor presentation, Kitty keyboard protocol, renderer FPS, render debounce, stdin parser buffer limits, plus env overrides.
- Support theme preset plus per-token color overrides.
- Support per-surface style overrides for shell, header, list, alternate rows, selected row, selected+marked row, marked row, empty states, preview, preview gutter, preview metadata, full preview, full preview gutter, full preview metadata, base/search/danger/help overlays, status line, and scrollbar, including secondary, success, warning, error, search, favorite/pinned, and image tone slots plus standard terminal text attributes.
- Support layout knobs: compact mode with dense defaults, list width, history load limit, alternate-row striping, search-match highlighting, header/status line visibility, split preview pane visibility, preview length, image metadata preview field ordering/visibility, row metadata visibility, split/full preview metadata and gutter visibility, scrollbar visibility/width, legacy panel/overlay padding fallbacks, independent list/split-preview/full-preview panel padding, independent search/danger/help overlay padding, spacer surfaces, structural header/status/overlay heights and overlay placement, pane width minima/insets, split-pane gap, title padding, help key width, row spacing, row preview reserved/max width with bounded metadata fitting, split/full preview text width insets, split/full image mode/renderer/block glyph/background/notice/max sizing/row insets and line spacing, split/full preview metadata heights and horizontal/vertical padding, split/full preview gutter widths, preview hash length, text truncation marker, whitespace replacement, and status separator spacing.
- Support either `listWidthPercent` or `previewWidthPercent` for split sizing, with list width taking precedence when both are present.
- Support chrome knobs: legacy panel/overlay border, title, and title-alignment fallbacks; independent header/list/split-preview/full-preview/search/danger/help border visibility, title visibility, border style, and title alignment; list-position title visibility/alignment, preview bottom-title visibility/alignment, row markers, scrollbar glyphs, and status separator, including empty glyph values.
- Support label overrides for header, filter names, search/query prompts and cursor, clear-kind names, panels, empty states, overlays, preview copy/gutters, split/full image source notices and fallback prefixes/separators, header/preview metadata separators, byte and age units, image fallback/protocol notice reasons, header/status line templates, status hints, operation/view/entry-count status copy, and runtime error copy.
- Support label overrides for key display separators, help key grouping, and status hint composition, including extended status hint placeholders for filter, pinned view, delete, output, and quit actions plus mode-specific search/preview/confirm hint templates.
- Support help-overlay row ordering/visibility through named help actions, including opt-in rows for less common but active commands.
- Support key display label overrides for normalized key names shown in help and status hints.
- Support status tone matcher overrides for success/warning/error coloring of custom status copy.
- Support header-line placeholder tone overrides for brand, filter, query, mode, label, and separator segments.
- Support status-line placeholder tone overrides for operation, watcher, hint, and separator segments.
- Support overlay-border tone overrides for search, danger, and help overlay chrome.
- Support overlay-content tone overrides for search prompt/query/cursor text, delete/clear prompts, confirmation hints, and help key/action text.
- Support list-content tone overrides for row markers, metadata, preview text, search matches, empty-state copy, and scrollbar cells.
- Support preview-content tone overrides for split/full preview borders, empty states, image fallback/notice text, gutters, semantic content lines, and metadata.
- Support behavior overrides for live search, search debounce, query clearing/restoration, and whether paste, copy, bulk-copy, and search-copy actions exit the TUI after success.
- Support label/layout/chrome overrides for row metadata templates, row field widths, list-position titles, split-preview metadata, and full-preview bottom titles.
- Support rich preview metadata placeholders for source application, age, short hash, full hash via `hashFull`, size, dimensions, blob path, and pinned state.
- Support label overrides for header selection text, search input prompt/query/cursor text, delete prompts, clear prompts, and confirm hints.
- Support keybinding overrides for all current actions.
- Support common Clipse top-level TUI aliases when they map cleanly onto Ditox behavior: `maxEntryLength`, `pollInterval`, `enableMouse`, `enableDescription`, `imageDisplay.type`, and `clearSelected`.
- Support Clipse-style `themeFile` loading relative to the TUI config file, including `custom_theme.json` color mapping for title, list, selection, status, filter, help, scrollbar, and preview surfaces.
- Add unit tests for defaults, file/env overrides, and invalid value clamping.
- Keep the JSON schema in sync with runtime config names and the example config.

Acceptance:

- A user can alter the visual identity and key hints without editing source.
- Components receive all strings/colors/layout decisions through config props.

## Phase 2: Navigation And Selection Parity

Goal: match the core Clipse list ergonomics.

Status: implemented for the current TUI. Keep refining as new views are added.

Deliverables:

- Keep home/end navigation.
- Keep page up/page down navigation based on visible row count.
- Keep select-single action.
- Keep range select up/down actions.
- Keep search-mode yank/copy for all current matches.
- Keep configurable filter cycle order.
- Keep literal and fuzzy-subsequence search-match highlighting in row previews.
- Keep delete operating on selected entries or current entry.
- Keep destructive confirmation text that reflects item count and warns when pinned entries are involved.
- Keep pinned-only toggle view.
- Keep clear variants that preserve pinned entries where intended.

Acceptance:

- The list can be driven efficiently without repeated single-row movement.
- Multi-select can copy, output, and delete the selected set.

## Phase 3: Pinned View And Clear Semantics

Goal: make pins behave like first-class protected items.

Status: implemented.

Deliverables:

- Keep the pinned-only toggle action in the TUI.
- Preserve current favorite filter, but keep the one-key pinned interaction like Clipse.
- Keep clear modes that preserve pinned items where intended.
- Keep explicit confirmation text for pinned deletes and clears.
- Keep delete-after TTL semantics for non-pinned entries only.

Acceptance:

- Users can treat pinned items as saved snippets instead of ordinary history.

## Phase 4: Preview Mode

Goal: add a full-screen or dominant scrollable preview mode.

Status: implemented for text, metadata, and PNG/JPEG/GIF/BMP half-block previews. Image protocol rendering still belongs to Phase 5.

Deliverables:

- Keep preview mode toggled from the list.
- Keep independent preview scrolling.
- Keep copy/paste from preview mode.
- Keep current split-pane metadata preview for normal browsing.
- Keep tests for preview line windowing.

Acceptance:

- Long text is inspectable before selection without breaking the list layout.

## Phase 5: Image Rendering

Goal: move beyond metadata-only image support.

Status: PNG, JPEG, GIF first-frame, WebP, and uncompressed BMP half-block rendering is implemented. OpenTUI capability flags are used for Kitty/Sixel fallback notices. Native Kitty and Sixel protocol rendering remain planned.

Deliverables:

- Detect terminal image protocol support where feasible and surface it through configurable fallback notices.
- Keep configured image preview mode and glyph selection for metadata or half-block previews.
- Add configured image preview mode for kitty or sixel when the renderer path is stable.
- Keep metadata fallback for unsupported terminals.
- Add dimensions and size constraints to avoid layout corruption.
- Add fixtures and tests for renderer selection and fallback behavior.
- Keep image copy/paste behavior MIME-aware so preview/rendering support does not affect clipboard fidelity.

Acceptance:

- Images can be visually inspected in supported terminals and never corrupt unsupported terminals.

## Phase 6: Realtime Updates

Goal: make the TUI feel attached to the running watcher.

Deliverables:

- Keep configurable polling while browsing.
- Keep the Clipse-style `-enable-real-time` launch alias wired to a live TUI polling interval.
- Add a lightweight daemon status/update channel later if the stdio RPC model becomes a bottleneck.
- Keep refreshing history while preserving selection when possible.
- Keep watcher paused/stale/healthy status in the status line.
- Keep watcher PID recording and `-kill` exact-PID shutdown covered by smoke tests.
- Avoid jumping selection while the user is searching or previewing.

Acceptance:

- New clipboard entries appear without restarting the TUI.

## Phase 7: Mouse And Accessibility

Goal: optional pointer support without weakening keyboard-first use.

Deliverables:

- Add configurable mouse enablement if users need to disable terminal mouse capture.
- Keep row click/select and wheel scrolling.
- Keep all actions keyboard-accessible.
- Maintain color-plus-text indicators for state, not color alone.

Acceptance:

- Mouse users get convenience while keyboard users remain first-class.

## Phase 8: Test And Quality Gates

Goal: prevent regressions while the TUI becomes more configurable.

Deliverables:

- Unit tests for config resolution, keybinding mapping, presentation helpers, and selection behavior.
- CLI smoke coverage for selected-entry delete and clear semantics.
- PTY runtime smoke for renderer boot.
- PTY runtime smoke for file-based TUI customization.
- OpenTUI component frame snapshots for the main shell/list/preview/status composition, search/delete/clear/help overlays, full preview, empty states, and image block previews.
- Keep expanding OpenTUI frame snapshots across more viewport sizes, theme combinations, and input states.
- Keep persisted golden text-frame snapshots under `tui/src/__goldens__`.
- Keep `DITOX_TUI_ARTIFACTS=1` text-frame/span JSON/SVG/PNG exports for review.
- Add terminal-native screenshot artifacts if OpenTUI exposes a stable screenshot path.

Acceptance:

- `bun run check` remains the required gate.
- Every new behavior has either unit, smoke, or runtime coverage.
