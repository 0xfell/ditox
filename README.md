# Ditox

Ditox is a fresh terminal-first clipboard manager.

This rebuild follows the preserved `migration-fresh-start.md` blueprint:

- no desktop GUI, tray, or floating app window
- Zig backend owns clipboard, storage, paste-back, config, and repair
- OpenTUI Solid frontend owns rendering and keyboard workflow only
- frontend and backend communicate through JSON-RPC 2.0 over Content-Length stdio
- first target is Linux Wayland on Hyprland

## Quick Start

```sh
nix develop
bun install
bun run check
```

Run the backend CLI after building:

```sh
./zig-out/bin/ditox add "hello"
./zig-out/bin/ditox list
./zig-out/bin/ditox print 1
./zig-out/bin/ditoxd serve --stdio < contracts/fixtures/health.check.rpc
```

Common Clipse-style aliases are accepted for supported operations:

```sh
./zig-out/bin/ditox -a "save this"
./zig-out/bin/ditox -c "copy only"
./zig-out/bin/ditox -p
./zig-out/bin/ditox -v
wl-paste --watch ./zig-out/bin/ditox --wl-store
./zig-out/bin/ditox --auto-paste
./zig-out/bin/ditox --output-all unescaped
./zig-out/bin/ditox -clear      # keep pinned
./zig-out/bin/ditox -clear-all  # include pinned
./zig-out/bin/ditox -clean      # sanitize text and reconcile images
./zig-out/bin/ditox -kill       # stop stored watcher PID
./zig-out/bin/ditox -pause 5m   # pause capture temporarily
./zig-out/bin/ditox -listen     # start the Wayland watcher process
./zig-out/bin/ditox keep        # open picker and keep it open after paste
```

Run the TUI with a built backend on `PATH` or through `DITOXD`:

```sh
DITOXD=./zig-out/bin/ditoxd bun run --cwd tui start
./zig-out/bin/ditox
./zig-out/bin/ditox tui
./zig-out/bin/ditox launch --keep
```

## TUI Customization

The TUI reads optional JSON config from `DITOX_TUI_CONFIG`, then falls back to
`$XDG_CONFIG_HOME/ditox/tui.json` or `~/.config/ditox/tui.json`. Missing files
and missing keys use built-in defaults.

Start from `tui/tui-config.example.json` to customize. The example declares
`$schema: "./tui-config.schema.json"` so editors can offer completion and catch
unsupported config keys.

- theme presets (`ditoxDark`, `ditoxLight`, `groknight`, `grokday`,
  `tokyonight`, `rosepine`) and per-token colors
- Clipse-style `themeFile` pointing to a `custom_theme.json` file; start from
  `tui/custom_theme.example.json` when migrating an existing Clipse theme
- terminal renderer behavior through `terminal.altScreen` or Grok-style
  `terminal.alt_screen`, direct `terminal.screenMode`, and renderer
  `terminal.backgroundColor` (`auto`, `transparent`, or `#rrggbb`),
  plus `terminal.footerHeight` for split-footer mode and
  `terminal.clearOnShutdown`; `terminal.title` and `terminal.cursor` can tune
  terminal identity and cursor style/blink/color; opt-in Kitty keyboard
  protocol settings live under `terminal.kittyKeyboard`, and optional OpenTUI
  tuning can set `terminal.targetFps`, `terminal.maxFps`,
  `terminal.debounceDelay`, and `terminal.stdinParserMaxBufferBytes`;
  `DITOX_TUI_ALT_SCREEN`, `DITOX_TUI_SCREEN_MODE`, `DITOX_TUI_BACKGROUND`,
  `DITOX_TUI_FOOTER_HEIGHT`, `DITOX_TUI_CLEAR_ON_SHUTDOWN`,
  `DITOX_TUI_TITLE`, `DITOX_TUI_CURSOR_STYLE`,
  `DITOX_TUI_CURSOR_BLINKING`, `DITOX_TUI_CURSOR_COLOR`,
  `DITOX_TUI_KITTY_KEYBOARD`, `DITOX_TUI_TARGET_FPS`,
  `DITOX_TUI_MAX_FPS`, `DITOX_TUI_RENDER_DEBOUNCE_MS`, and
  `DITOX_TUI_STDIN_BUFFER_BYTES` can override this for one-off runs
- per-surface styles for shell, header, list, alternate rows, selected row,
  selected+marked row, marked row,
  row spacers, empty states, preview, preview gutter, preview metadata,
  split-preview spacers, full preview, full preview gutter, full preview metadata,
  full-preview spacers, base/search/danger/help overlays, status line, scrollbar, and split-pane gap,
  including per-surface semantic colors for secondary text, success, warning,
  error, search, favorite/pinned, and image accents plus terminal text
  attributes such as bold, dim, italic, underline, inverse, and strikethrough
- split width through `listWidthPercent` or `previewWidthPercent`, compact mode
  with denser default padding, preview, metadata, gutter, and empty-state spacing,
  history load limit, shell padding, structural header/status/overlay heights and overlay placement,
  header/status line visibility and horizontal/vertical body alignment,
  split/full preview length, live refresh interval,
  split/full image preview mode, sizing, OpenTUI-vs-text renderer, block glyph, transparent-pixel hex background, and protocol notice visibility, mouse enablement and scroll rows, split/full preview
  metadata and gutter visibility, row metadata visibility, bounded row metadata
  slots for long custom templates, split preview pane
  visibility, image metadata preview field ordering/visibility, search-match highlighting, scrollbar visibility/width/placement/alignment, pane minimum widths, split-pane gap, split/full preview width insets,
  empty-state helper visibility, line spacing, and vertical alignment, split/full preview text width insets, preview body horizontal/vertical alignment, split/full image max sizing, split/full image row insets, notice spacing, and line spacing, split/full preview line-number gutter widths and alignment,
  split/full preview metadata heights, header/detail line spacing, horizontal/vertical padding, body alignment, and hash lengths,
  symmetric or asymmetric title padding, legacy panel/overlay padding and overlay body-alignment fallbacks, independent list/split-preview/full-preview panel padding, independent search/danger/help overlay padding, overlay row spacing, and horizontal/vertical body alignment, help key width/alignment, row field widths and marker/age/size/pinned slot alignment, row content/metadata/preview alignment, row metadata hash length,
  row gaps and vertical spacing, spacer surfaces, alternate-row striping, row preview reserved width, row preview max width, empty-state
  padding, title/help/vertical alignment, and line spacing, header brand/filter/query/mode width caps, overlay prompt/hint/action width caps, symmetric or asymmetric status separator spacing, and status operation/watcher/hint width caps
- legacy panel/overlay border, title, and title-alignment fallbacks; independent header/list/split-preview/full-preview/status/search/danger/help border visibility, title visibility, border style, and title alignment,
  list-position and preview bottom-title visibility and alignment, row markers including the
  selected+marked marker and marker slot width/alignment, scrollbar
  glyphs/alignment, status separator; row/status/scrollbar glyphs can be set to empty
  strings for quieter layouts
- all visible labels, filter names, search/query prompts/cursor, clear-kind names,
  empty states, preview metadata/gutters, operation/view status copy, overlay
  copy, header selection templates, row pinned markers, row metadata templates,
  entry ID prefixes, preview/list title templates, and split/full preview
  metadata header/detail templates,
  metadata placeholders for age/source/dimensions/blob/hash/full-hash/MIME/size/id,
  split/full preview separators, text preview gutter templates, search/delete/clear/confirm prompt templates, split/full image
  source notices, source labels, fallback prefixes/separators, image fallback/protocol notice reasons,
  Kitty/Sixel protocol display names, text truncation markers, whitespace replacement,
  header/preview metadata separators, byte and age units, watcher error separators,
  watcher status templates,
  key display separators, header/status line templates, status hint templates,
  operation, pin/unpin, and entry-count status templates, runtime error templates including unknown exit-status text, and help action text
- keybindings for every current TUI action; defaults keep `space` for preview
  and use `x` / `s` for mark/select workflows. Search mode also exposes a
  Clipse-style `ctrl+s` yank action that copies every current search match.
  Common Clipse key names such as `choose`, `filter`, `togglePin`,
  `togglePinned`, `clearSelected`, and `yankFilter` are accepted as config
  aliases. `clearSelected` maps to clearing marked rows, matching Clipse.
- `keyLabels` for changing how normalized key names appear in help and status
  hints without changing the actual bindings
- status hint templates can reference paste/copy/preview/search/filter/pinned/
  delete/output/help/quit key groups, and separate search/preview/confirm-mode
  hint templates can expose mode-specific apply/back/scroll/confirm actions
- `headerLineTones` for routing header placeholders such as brand, filter,
  query, mode, labels, and separators to explicit header surface tones while
  keeping `auto` for the pinned-filter color
- `statusTones` for controlling which status text fragments render with the
  success, warning, or error colors
- `statusLineTones` for routing status-line placeholders such as operation,
  watcher, hint, and separator to explicit status surface tones while keeping
  `auto` for dynamic semantic colors
- `layout.headerContentAlign`, `layout.statusContentAlign`,
  `layout.headerVerticalAlign`, and `layout.statusVerticalAlign` for placing the
  header/status body text independently from border title alignment
- `layout.headerBrandMaxWidth`, `headerFilterMaxWidth`,
  `headerQueryMaxWidth`, and `headerModeMaxWidth` for keeping long header
  segments bounded in narrow terminals
- `layout.searchOverlayPromptMaxWidth`, `searchOverlayQueryMaxWidth`,
  `searchOverlayCursorMaxWidth`, `dangerOverlayPromptMaxWidth`,
  `dangerOverlayHintMaxWidth`, and `helpOverlayActionMaxWidth` for keeping
  custom overlay labels and help copy bounded in narrow terminals
- `layout.overlayContentAlign`, `searchOverlayContentAlign`,
  `dangerOverlayContentAlign`, `helpOverlayContentAlign`, the matching overlay
  vertical-alignment fields, and overlay line-spacing fields for placing overlay
  body rows independently from overlay title alignment
- `overlayBorderTones` for routing search, danger, and help overlay border
  colors to explicit overlay surface tones
- `overlayContentTones` for routing search input prompt/query/cursor,
  delete/clear prompts, confirmation hints, and help key/action text to explicit
  overlay surface tones
- `listContentTones` for routing row markers, metadata, preview text, search
  matches, empty-state copy, and scrollbar cells to explicit list/empty/scrollbar
  surface tones
- `previewContentTones` for routing split/full preview borders, empty states,
  image fallback/notice text, gutters, semantic content lines, and split/full
  preview metadata header/detail rows
  to explicit preview surface tones
- `filterOrder` for changing which filters the cycle action visits, and in what
  order. Supported values are `all`, `text`, `images`, `favorites`, and `today`.
- `helpOrder` for reordering or hiding help-overlay rows, useful for compact
  picker layouts that only show the commands a team actually uses. Besides the
  default rows, it can opt into rows for quit, output, preview scrolling/back,
  search editing, and confirmation choices.
- `startup` for choosing the opening filter, pinned-only mode, and initial query
  before the first backend refresh. The same filter values are supported.
- `behavior` for Clipse-like search and action lifecycle: live search is enabled
  by default with configurable debounce, search can keep or clear the previous
  query when opened, cancelled search can restore or keep the typed query, and
  paste/choose exits the TUI by default while copy, bulk copy, and search-copy
  actions can independently opt into exiting after success.
- common Clipse top-level TUI config keys map onto Ditox layout behavior:
  `maxEntryLength`, `pollInterval`, `enableMouse`, `enableDescription`, and
  `imageDisplay.type = basic | kitty | sixel`. `maxEntryLength` caps the row
  preview text, and Clipse `imageDisplay.scaleX`, `scaleY`, and `heightCut`
  become block preview sizing hints unless native Ditox image sizing is
  set. Kitty/Sixel modes are preserved explicitly and render through labeled
  block fallbacks until OpenTUI exposes a stable protocol renderer path.
  `layout.imagePreviewRenderer = auto | opentui | text` chooses between the
  OpenTUI supersampled block renderer and the text-span renderer; `auto` uses
  the OpenTUI path for the default glyph and keeps the text path for custom
  block glyph themes. `layout.imagePreviewAlign` and
  `layout.fullPreviewImageAlign` place rendered block previews at `left`,
  `center`, or `right` inside their split/full preview text area; the matching
  `previewContentAlign`, `fullPreviewContentAlign`, `previewBodyVerticalAlign`,
  `fullPreviewBodyVerticalAlign`, `previewMetaContentAlign`,
  `fullPreviewMetaContentAlign`, `previewMetaVerticalAlign`, and
  `fullPreviewMetaVerticalAlign` knobs align preview text, fallback notices,
  empty-preview copy, preview body placement, and metadata independently.
  OpenTUI capability flags are observed, so fallback notices can distinguish
  unknown, unsupported, and detected-but-unrenderable protocol states.

Environment overrides remain available for quick experiments:

```sh
DITOX_TUI_THEME=ditoxLight DITOX_TUI_LIST_WIDTH=60 bun run --cwd tui start
```

For non-live search while tuning backend matching:

```sh
DITOX_TUI_LIVE_SEARCH=false bun run --cwd tui start
```

For Clipse-style keep-open debugging:

```sh
./zig-out/bin/ditox keep
./zig-out/bin/ditox launch --keep
DITOX_TUI_EXIT_AFTER_PASTE=false bun run --cwd tui start
```

Golden TUI text frames live in `tui/src/__goldens__`. Refresh them only after
reviewing the visual change:

```sh
cd tui
DITOX_UPDATE_GOLDENS=1 bun test src/tui-golden.test.tsx
```

For review artifacts without changing goldens, export the rendered text frame,
OpenTUI span data, an SVG visual render, and a PNG bitmap render:

```sh
cd tui
DITOX_TUI_ARTIFACTS=1 bun test src/tui-golden.test.tsx
```

Artifacts are written under `tui/artifacts/frames/` and are ignored by Git.
Set `DITOX_TUI_ARTIFACT_DIR=/tmp/ditox-frames` to redirect them.

## Backend Config

The backend reads TOML or JSON from `DITOX_CONFIG`, then falls back to
`$XDG_CONFIG_HOME/ditox/config.toml` or `~/.config/ditox/config.toml`. JSON
files are detected when the file starts with `{`, so an existing Clipse
`configuration.json` can be pointed at directly.

```toml
[history]
max_entries = 1000
delete_after_seconds = 0 # 0 disables TTL cleanup
allow_duplicates = false

[watch]
poll_interval_ms = 250

[paste]
enabled = true
buffer_ms = 120

[auto_paste]
enabled = false
keybind = "ctrl+v"
buffer_ms = 10

[capture]
excluded_apps = ["1Password", "Bitwarden", "KeePassXC", "LastPass", "Dashlane", "Password Safe", "Keychain Access"]
excluded_windows = []
```

For Clipse migrations, the backend also accepts these top-level TOML or JSON aliases:
`maxHistory`, `deleteAfter`, `allowDuplicates`, `pollInterval`, and
`maxEntryLength`. `historyFile` maps to Ditox's SQLite database path and
`tempDir` maps to the image blob directory; relative values resolve from the
config file's directory. `~/`, `$HOME`, `$XDG_CONFIG_HOME`, and
`$XDG_DATA_HOME` path prefixes are expanded. `excludedApps` and
`excludedWindows` map to the Hyprland capture exclusion lists, and Clipse-style
`autoPaste.enabled`, `autoPaste.keybind`, and `autoPaste.buffer` map to Ditox's
auto-paste settings.

`delete_after_seconds` mirrors Clipse's delete-after behavior: old non-pinned
entries are removed automatically while pinned entries remain saved. Image blobs
deleted by retention, clear, or delete are pruned from disk.

`[auto_paste]` mirrors Clipse's paste-key settings. `ditox --auto-paste`
sends the configured keybind through Hyprland, and the regular paste-back path
uses the same keybind after writing the selected entry to the clipboard.

`[capture]` mirrors Clipse's excluded-app workflow for the Hyprland watcher.
The watcher compares the active window class against `excluded_apps` and the
active window title against `excluded_windows` with case-insensitive substring
matching before storing text or images.

## Hyprland Shape

The target launch model is:

```ini
exec-once = ditoxd daemon
bind = SUPER, V, exec, ditox
```

`ditox` with no arguments, `ditox launch`, `ditox launch --keep`, and
`ditox keep` capture the active Hyprland window before opening the terminal and
pass it to the TUI as `DITOX_TARGET_WINDOW` through the child environment. The
keep variants also set `DITOX_TUI_EXIT_AFTER_PASTE=false` for that launched
TUI. `ditox -enable-real-time`, `ditox launch --enable-real-time`, and
`ditox tui --enable-real-time` launch the picker with
`DITOX_TUI_REFRESH_MS=250` for Clipse-style live polling. Paste-back writes the selected entry to the clipboard with `wl-copy`,
refocuses the captured window with `hyprctl`, and dispatches the configured
paste shortcut, `ctrl+v` by default.

The watcher is now the `ditoxd daemon` (single long-lived DB owner process running the full capture loop). This is the structural fix for DB lock / stale watcher. JSON-RPC stays short-lived stdio for now (per plan).
OpenTUI process simple while storage and capture semantics settle.

The TUI shipping format is bundled JavaScript for normal builds
(`tui/dist/index.js`, run with `bun`). Source execution through
`bun run --cwd tui start` remains the fallback for development checkouts, and a
compiled Bun executable is deferred until the TUI API stops moving. An installed
`ditox` binary looks for bundled TUI entrypoints next to itself first, including
`../share/ditox/tui/dist/index.js`, before falling back to the development
checkout path. `bun run build` builds the TUI bundle before the Zig install step,
so `zig-out/share/ditox/tui/dist/` contains the runnable bundle and
`zig-out/share/ditox/tui/` contains the example config, schema, and custom-theme
example.

## Current Scope

The current implementation includes SQLite schema migrations, relevance-ranked
FTS5 search with literal substring and boundary-aware fuzzy subsequence fallback,
metadata-first image capture under `images-v2`, method-specific JSON-RPC contracts,
file-backed OpenTUI customization, configurable keymap bindings, multi-select
bulk copy/output, search-match yank/copy, highlighted visible literal and fuzzy search matches,
range selection, pinned-only view, full preview mode,
mouse selection/right-click marking/scrolling, pinned-safe clear flows, live TUI polling,
watcher health in the status line, pause/resume watcher state, delete-after TTL
cleanup for non-pinned entries, storage clean/repair that sanitizes persisted
text rows and removes image rows with missing blobs, no-argument TUI launch, stored watcher-PID
shutdown, case-insensitive excluded app/window capture rules, Clipse-style CLI aliases for version/add/copy-input/print-clipboard/wl-store/
auto-paste/output-all/listen/clear/clean/kill/pause/realtime-launch flows, explicit unsupported messages for X11/macOS listener aliases, and Hyprland paste-back targeting. Mouse capture
can be disabled from config for terminal workflows that prefer pure keyboard
input, while terminal title, cursor style, Kitty keyboard protocol, and renderer
timing/buffer settings can be opted into per file or per launch. PNG, JPEG, GIF first frames, WebP, and uncompressed BMP image entries can
render as configurable OpenTUI-supersampled or text-span block previews with configurable source labels,
source-notice spacing, metadata fallback, capability-aware Kitty/Sixel fallback notices/protocol names,
and bounded row budgeting before text metadata is windowed; selecting an
image writes the stored blob back to the clipboard with its MIME type. Native Kitty/Sixel protocol rendering,
daemon sockets, sync, scripting, Windows, and X11 are intentionally deferred.

For clipboard-safe tests or demos, set `DITOX_CLIPBOARD_MOCK=/tmp/ditox-clip`
before running `ditox copy` or `ditox paste`.
