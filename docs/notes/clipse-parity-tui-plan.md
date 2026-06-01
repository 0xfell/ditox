# Clipse Parity And TUI Customization Plan

Ditox should keep its Zig plus OpenTUI architecture, but the user-facing behavior should converge on the parts of Clipse that make it feel complete: fast history selection, rich previews, configurable shortcuts, safe destructive actions, and full visual customization.

## Current Parity Snapshot

Implemented today:

- Text history with SQLite persistence.
- Explicit SQLite schema versioning and a tested v1-to-v2 migration path; normal backend opens no longer rebuild the FTS index, so TUI refresh RPCs and the watcher can share SQLite without unnecessary write-lock contention.
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
- Wayland clipboard writes do not wait on `wl-copy` stderr, because `wl-copy` may fork a clipboard-owner process that keeps inherited file descriptors open after the command has already accepted the clipboard payload.
- TUI Enter/choose bindings normalize OpenTUI's runtime `return` key event to the user-facing `enter` action, including search apply and preview-mode copy/paste.
- Clipse-style CLI aliases for supported command flows: `-v`, `--version`, `version`, `-a`, `-c`, `-p`, `--wl-store`, `--auto-paste`, `-clear`, `-clear-all`, `-clear-text`, `-clear-images`, `--output-all`, `-clean`, `-kill`, `-pause`, `-listen`, `-listen-shell`, and `-enable-real-time`; platform-specific `-listen-x11` / `-listen-darwin` aliases are recognized and report that only the Wayland listener is implemented.
- `ditox` with no arguments, `ditox launch`, `ditox launch --keep`, `ditox keep`, `ditox -enable-real-time`, and `ditox tui --enable-real-time` capture the active Hyprland window and forward it to the TUI as `DITOX_TARGET_WINDOW` through the child environment; keep-open launch variants set `DITOX_TUI_EXIT_AFTER_PASTE=false` for the child, and realtime launch variants set `DITOX_TUI_REFRESH_MS=250`.
- `ditox tui` resolves bundled JavaScript from an install layout next to the executable before falling back to the development checkout path.
- `bun run build` builds the TUI bundle before the Zig install step; when `tui/dist/index.js` exists, `zig build` installs it under `share/ditox/tui/dist` along with the TUI config example, schema, and custom-theme example.
- `ditox --auto-paste` sends the configured paste keybind through Hyprland; the direct paste-back path uses the same configurable keybind after writing the selected entry to the clipboard.
- TUI paste/choose falls back to copy-only when no target window was captured or when Hyprland paste-back fails after clipboard write, so text and image entries still land on the clipboard.
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
- File-backed header brand/filter/query/mode width caps can keep long searches or localized labels from crowding narrow terminals.
- File-backed overlay prompt, query, cursor, hint, and help-action width caps can keep custom overlay copy from crowding narrow terminals.
- File-backed status operation, watcher, and key-hint width caps can keep long errors or watcher states from crowding narrow terminals.
- File-backed status tone matchers can map localized or theme-specific status text to success, warning, and error colors.
- Pin/unpin actions preserve a configurable success status after the history refresh instead of falling back to the generic entry count.
- File-backed list content tone routing can assign explicit tones to row markers, metadata, preview text, search matches, empty-state copy, and scrollbar cells.
- File-backed preview content tone routing can assign explicit tones to split/full preview borders, empty states, image fallback/notice text, gutters, semantic content lines, and metadata.
- File-backed config accepts common Clipse top-level TUI aliases for `maxEntryLength`, `pollInterval`, `enableMouse`, `enableDescription`, and `imageDisplay.type`; `maxEntryLength` caps row preview text, Clipse `imageDisplay.scaleX` / `scaleY` / `heightCut` map to preview sizing hints, and Kitty/Sixel image display aliases select native terminal image protocols when the terminal reports support and pixel resolution, falling back to labeled text-span blocks otherwise.
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
- Mouse row selection, right-click marking, Ctrl-click marking, Shift-click range selection, list wheel scrolling, and full-preview wheel scrolling.
- Conflict-free default browse keymap: `space` opens Clipse-style full preview, while `x` / `s` handle mark/select workflows.
- Configurable mouse enablement for users who do not want terminal mouse capture.
- Configurable terminal alternate-screen behavior through Ditox `terminal.altScreen`, Grok-style `terminal.alt_screen`, direct `terminal.screenMode`, renderer `terminal.backgroundColor`, split-footer `terminal.footerHeight`, shutdown clearing, terminal title/cursor presentation, Kitty keyboard protocol, renderer FPS, render debounce, stdin parser buffer limits, and env overrides.
- Watcher live/paused/stale/stopped state in the status line with configurable labels and status composition templates.
- SQLite connections use a busy timeout and avoid routine FTS rebuilds, preventing transient watcher/TUI lock contention from leaking into the status line as raw `SQLiteFailure` errors during normal browsing.
- Help overlay rows now lay out as responsive columns based on available width and row capacity, with dynamic key-column sizing and shorter default action labels.
- Hyprland watcher capture exclusions for app class and window title, with Clipse-style case-insensitive substring matching and default password-manager app exclusions.
- Theme presets (`ditoxDark`, `ditoxLight`, `groknight`, `grokday`, `tokyonight`, `rosepine`), terminal screen/background/title/cursor behavior, compact-mode dense layout defaults, row kind labels, compact pinned markers, markerless rows, selected+marked row markers, marker slot width/alignment, alternate-row striping, row metadata templates/visibility/bounded fitting/alignment, row metadata age/size/pinned slot alignment, row content/preview alignment, row marker/metadata/preview/search-match/empty/scrollbar tone routing, preview border/empty/fallback/gutter/content/metadata tone routing, entry ID prefixes, filter names/order, startup filter/query state, action exit behavior, search/query prompts/templates/cursor, search-match highlighting, delete/clear/confirm prompt templates, clear-kind names, empty previews, empty-state helper visibility/title alignment/help alignment/vertical alignment/line spacing, header selection templates, header/status line templates/horizontal/vertical body alignment, header-line and status-line placeholder tone routing, overlay border/content tone routing including search prompt/query/cursor colors, preview/list title templates, preview metadata/gutters/separators/templates/placeholders, text preview gutter templates, split/full preview gutter widths/alignment/separators, image metadata preview field ordering/visibility, header separators, key display separators, row field widths/gaps/vertical spacing, row/list/preview/full-preview/split-pane spacer surfaces, history load limit, shell padding, structural header/status/overlay heights/placement and visibility, header/status/overlay vertical padding, independent list/split-preview/full-preview panel padding, independent search/danger/help overlay padding, overlay row spacing, and horizontal/vertical body alignment with legacy global fallbacks, independent header/list/split-preview/full-preview/status/search/danger/help border visibility/title visibility/border style/title alignment with legacy global fallbacks, list-position and preview bottom-title visibility/alignment, pane width minima/insets, split/full preview gutter visibility, split/full preview text width insets and horizontal/vertical body alignment, split/full image mode/renderer/alignment/block glyph/background/notice/max sizing/row insets/source notices/source labels/fallback prefixes and separators/notice spacing/line spacing, split/full preview metadata height, header/detail line spacing, horizontal/vertical padding, horizontal/vertical body alignment, and hash lengths, full-preview metadata header/detail templates and tone routing, scrollbar width/placement/glyphs/alignment, text truncation markers, whitespace replacement, symmetric/asymmetric title padding, help key width/alignment, default and opt-in help rows, byte and age units, image fallback/protocol notice reasons and display names, watcher error separators/status templates, extended and mode-aware status hint templates, header brand/filter/query/mode width caps, overlay prompt/hint/action width caps, status operation/watcher/hint width caps, operation/view/pin/entry-count status copy/templates, runtime error templates including unknown exit-status text, and per-surface semantic tone colors and text attributes are configurable.
- PNG, JPEG, GIF first frames, WebP, and uncompressed BMP image entries render through a native-first `auto` image renderer by default: Kitty graphics is preferred, Sixel is the second native choice, and text-span half blocks remain the safe fallback. Native rendering computes image placement from OpenTUI terminal capabilities, terminal pixel resolution, pane cell geometry, and the configured split/full preview bounds; text fallback and the explicit OpenTUI supersampled renderer remain available through config. These paths support configurable split/full alignment, area-averaged downsampling/scaling, larger split/full default preview budgets, cell glyphs, source labels, notice spacing, metadata fallback, capability-aware Kitty/Sixel fallback notices/protocol names, and row budgeting that reserves image/notice/fallback rows before text metadata is windowed. Kitty rendering transmits image data without implicit display, clears stale visible placements before each explicit placement, and clears/frees native image state when the preview leaves native-image mode so terminal graphics cannot overlay later text frames.
- Image entry selection restores stored image bytes to the clipboard with the entry MIME type.
- Delete-after TTL cleanup removes old non-pinned entries and prunes expired image blobs.
- Repair/clean sanitizes persisted text rows, recomputes previews/hashes/byte lengths, removes image entries whose blob files are missing, and drains pending blob-prune records.
- The watcher records its PID in storage on startup so `ditox -kill` can terminate the exact known watcher process without broad command matching.
- `ditox -pause <duration>` accepts Clipse-style `ms`, `s`, `m`, and `h` durations while keeping the existing `pause <milliseconds>` command compatible.
- Backend config accepts TOML and JSON files, including Clipse-style `configuration.json` top-level aliases for `maxHistory`, `deleteAfter`, `allowDuplicates`, `pollInterval`, `maxEntryLength`, `historyFile`, `tempDir`, `excludedApps`, `excludedWindows`, and `autoPaste`, with smoke coverage for duplicate history, max-history pruning, config-relative storage/image paths, JSON config loading, auto-paste settings, and watcher capture exclusions.
- Search ranking is covered against realistic clipboard-history samples for path acronyms, camel-case symbols, stack traces, command snippets, and environment/config snippets.
- The CLI smoke suite verifies Clipse-style aliases for version, add, copy-input, print-clipboard, wl-store text/image capture, auto-paste, output-all, clean, kill, pause durations, unsupported platform listener messages, no-argument launch, keep-open launch, realtime launch, and pinned-safe/inclusive clear flows.
- The CLI smoke suite verifies image copy uses MIME-tagged image bytes rather than the stored hash.
- The CLI smoke suite verifies copy commands return after a Wayland clipboard owner inherits stderr, so pressing Enter in the TUI cannot freeze on a completed `wl-copy` handoff.
- The root CLI smoke suite verifies the TUI render path emits image block cells and file-based visual/key-hint customization.
- The TUI unit suite verifies the main shell/list/preview/status composition, search/delete/clear/help overlays, full preview, empty states, image block preview rendering, compact light-theme terminal bounds, and a wide/narrow/full-preview viewport matrix with OpenTUI frame snapshots.
- The TUI unit suite verifies Enter/return key normalization and paste-to-copy fallback behavior for no-target and paste-back-failure launch paths.
- The TUI unit suite verifies persisted golden text frames for representative shell, help-overlay, and full-preview states.
- `DITOX_TUI_ARTIFACTS=1` exports ignored text-frame, OpenTUI span JSON, SVG visual, and deterministic PNG bitmap artifacts for review.
- The TUI unit suite verifies mouse row gesture mapping, custom row pinned markers, markerless rows, row marker slot width/alignment, alternate-row styling, selected+marked row styling/markers, bounded row metadata fitting/alignment, row metadata age/size/pinned slot alignment, row content/preview alignment, list content tone routing, preview content tone routing, terminal title/cursor config, entry ID prefixes, preview separators, header/preview metadata separators, header/status line templates/horizontal/vertical body alignment and visibility, header-line and status-line placeholder tone routing, overlay border/content tone routing including search prompt/query/cursor tones, overlay placement/horizontal/vertical body alignment/row spacing, shell padding, independent panel/overlay padding, independent panel/overlay chrome, status-line chrome, title alignment, narrow header segment fitting, narrow status segment fitting, and narrow overlay text fitting, title chrome visibility/alignment and optional status-line chrome, empty-state styling/horizontal/vertical alignment/line spacing, text preview gutter templates, split/full preview gutter visibility/styling/widths/alignment/separators, preview horizontal/vertical body alignment, preview line spacing, spacer surface styling, preview metadata height/placeholders/hash length/padding/line spacing/horizontal/vertical alignment, image metadata field ordering/visibility, full-preview metadata header/detail templates, legacy fallbacks, styling, and metadata hash length/padding/line spacing/alignment, split/full image mode/rendering/alignment/max sizing/source labels/source and fallback copy/notice spacing, image preview row budgeting, preview block glyphs/background/protocol notices and display names, configurable filter cycle order, default and opt-in help row ordering/visibility/key-column alignment, search-match row highlighting/toggling, row spacing, scrollbar width/placement/glyphs/alignment, key display labels/separators, size/age units, image fallback separators, watcher error separators/status templates, extended and mode-aware status hint templates, operation/pin/entry-count status templates, status tone matchers, per-surface semantic color overrides, per-surface text attributes, and mode-specific overlay surface styling.
- The TUI unit suite verifies split/full preview body vertical placement separately from preview text alignment and metadata placement.

Not yet 1:1 with Clipse:

- Native Kitty and Sixel image protocol rendering is implemented for terminals that report both protocol support and pixel resolution through OpenTUI. Remaining image-preview gaps are hardening against more terminal multiplexers, adding broader real-terminal smoke coverage, and potentially adding optional external renderer fallbacks for terminals that support images but do not expose them through OpenTUI capability detection.
- Command parity is not exact for platform-specific Clipse listener implementations; the X11/macOS aliases are recognized with explicit unsupported messages, and the Wayland/Hyprland command path covers the common supported aliases listed above, version output, no-argument TUI launch, keep-open launch, realtime launch, text/image repair-clean, watcher kill, pause durations, and configurable `--auto-paste`.
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
- Support layout knobs: compact mode with dense defaults, list width, history load limit, alternate-row striping, search-match highlighting, empty-state helper visibility/title alignment/help alignment/vertical alignment/line spacing, shell padding, header/status line visibility and horizontal/vertical body alignment, split preview pane visibility, preview length, image metadata preview field ordering/visibility, row metadata visibility, row content/metadata/preview alignment, row marker/age/size/pinned slot alignment, split/full preview metadata and gutter visibility, scrollbar visibility/width/placement/alignment, legacy panel/overlay padding and body-alignment fallbacks, independent list/split-preview/full-preview panel padding, independent search/danger/help overlay padding, overlay row spacing, and horizontal/vertical body alignment, spacer surfaces, structural header/status/overlay heights and overlay placement, pane width minima/insets, split-pane gap, symmetric/asymmetric title padding, help key width/alignment, row spacing, row preview reserved/max width with bounded metadata fitting, split/full preview text width insets and horizontal/vertical body alignment, split/full image mode/renderer/alignment/block glyph/background/notice/max sizing/row insets, notice spacing, and line spacing, split/full preview metadata heights, header/detail line spacing, horizontal/vertical padding, horizontal/vertical body alignment, and hash lengths, split/full preview gutter widths and alignment, text truncation marker, whitespace replacement, optional header brand/filter/query/mode width caps, optional overlay prompt/hint/action width caps, symmetric or asymmetric status separator spacing, and optional status operation/watcher/hint width caps.
- Support split/full preview body vertical placement separately from preview text alignment and metadata alignment.
- Support either `listWidthPercent` or `previewWidthPercent` for split sizing, with list width taking precedence when both are present.
- Support chrome knobs: legacy panel/overlay border, title, and title-alignment fallbacks; independent header/list/split-preview/full-preview/status/search/danger/help border visibility, title visibility, border style, and title alignment; list-position title visibility/alignment, preview bottom-title visibility/alignment, row markers and marker slot width/alignment, scrollbar glyphs/alignment, and status separator, including empty glyph values.
- Support label overrides for header, filter names, search/query prompts and cursor, clear-kind names, panels, empty states, overlays, preview copy/gutters, text preview gutter templates, split/full image source notices/source labels and fallback prefixes/separators, header/preview metadata separators, split/full preview metadata header/detail templates, byte and age units, image fallback/protocol notice reasons and display names, header/status line templates, status hints, operation/view/pin/entry-count status copy, and runtime error templates including unknown exit-status text.
- Support label overrides for key display separators, help key grouping, and status hint composition, including extended status hint placeholders for filter, pinned view, delete, output, and quit actions plus mode-specific search/preview/confirm hint templates.
- Support help-overlay row ordering/visibility through named help actions, including opt-in rows for less common but active commands.
- Support responsive help-overlay columns with dynamic key/action widths so compact shortcuts stay readable across narrow and wide terminals.
- Support key display label overrides for normalized key names shown in help and status hints.
- Support status tone matcher overrides for success/warning/error coloring of custom status copy.
- Support header-line placeholder tone overrides for brand, filter, query, mode, label, and separator segments.
- Support status-line placeholder tone overrides for operation, watcher, hint, and separator segments.
- Support overlay-border tone overrides for search, danger, and help overlay chrome.
- Support overlay-content tone overrides for search prompt/query/cursor text, delete/clear prompts, confirmation hints, and help key/action text.
- Support list-content tone overrides for row markers, metadata, preview text, search matches, empty-state copy, and scrollbar cells.
- Support preview-content tone overrides for split/full preview borders, empty states, image fallback/notice text, gutters, semantic content lines, and metadata.
- Support behavior overrides for live search, search debounce, query clearing/restoration, and whether paste, copy, bulk-copy, and search-copy actions exit the TUI after success.
- Support label/layout/chrome overrides for row metadata templates, row field widths and slot alignment, list-position titles, split/full preview metadata, and full-preview bottom titles.
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
- Keep configurable pin/unpin status feedback after the mutation reloads history.
- Keep clear modes that preserve pinned items where intended.
- Keep explicit confirmation text for pinned deletes and clears.
- Keep delete-after TTL semantics for non-pinned entries only.

Acceptance:

- Users can treat pinned items as saved snippets instead of ordinary history.

## Phase 4: Preview Mode

Goal: add a full-screen or dominant scrollable preview mode.

Status: implemented for text, metadata, block image previews, and native Kitty/Sixel image previews where the terminal reports support.

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

Status: PNG, JPEG, GIF first-frame, WebP, and uncompressed BMP rendering is implemented. The default image renderer is native-first: Kitty graphics when available, Sixel when available, and text-span blocks when native protocols or pixel resolution are unavailable. The older OpenTUI supersampled buffer renderer remains explicit opt-in.

Deliverables:

- Detect terminal image protocol support where feasible and surface it through configurable fallback notices.
- Keep configured image preview mode and glyph selection for metadata or half-block previews.
- Keep configured image preview mode for Kitty or Sixel and route it through the native protocol renderer when support and pixel resolution are available.
- Keep metadata fallback for unsupported terminals.
- Keep dimensions and size constraints to avoid layout corruption.
- Keep fixtures and tests for protocol selection, terminal pixel geometry, Kitty/Sixel encoding, native render queuing, and fallback behavior.
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
- Keep backend storage opens read-friendly: schema migration may rebuild FTS when needed, but ordinary TUI RPC connections must not perform write-heavy maintenance work.
- Avoid jumping selection while the user is searching or previewing.

Acceptance:

- New clipboard entries appear without restarting the TUI.

## Phase 7: Mouse And Accessibility

Goal: optional pointer support without weakening keyboard-first use.

Deliverables:

- Add configurable mouse enablement if users need to disable terminal mouse capture.
- Keep row click/select, right-click marking, and wheel scrolling.
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
- CLI smoke coverage for packaged TUI bundle/config installation and installed-layout TUI launch resolution.
- OpenTUI component frame snapshots for the main shell/list/preview/status composition, search/delete/clear/help overlays, full preview, empty states, and image block previews.
- Keep expanding OpenTUI frame snapshots across more viewport sizes, theme combinations, and input states.
- Keep persisted golden text-frame snapshots under `tui/src/__goldens__`.
- Keep `DITOX_TUI_ARTIFACTS=1` text-frame/span JSON/SVG/PNG exports for review.
- Add terminal-native screenshot artifacts if OpenTUI exposes a stable screenshot path.

Acceptance:

- `bun run check` remains the required gate.
- Every new behavior has either unit, smoke, or runtime coverage.
