# Ditox

Ditox is a **terminal-first clipboard manager** for Linux/Wayland (first target:
Hyprland). There is no desktop GUI, tray, or floating window — just a fast CLI
and a keyboard-driven TUI picker.

- **Zig backend** owns clipboard access, SQLite storage, the watcher daemon,
  config, paste-back, and repair.
- **OpenTUI/Solid frontend** (Bun + TypeScript) owns rendering and the keyboard
  workflow only.
- The two halves talk over **JSON-RPC 2.0** (Content-Length framed stdio); the
  TUI never touches the database directly.

See [`AGENTS.md`](./AGENTS.md) for the authoritative architecture overview and
[`docs/ROADMAP.md`](./docs/ROADMAP.md) for status.

## Features

- History stored in SQLite with FTS5 search (literal substring + boundary-aware
  fuzzy fallback) and relevance ranking.
- Image capture with metadata-first priority — copying an image from a browser
  stores the image, not the URL. Blobs are content-addressed and deduplicated.
- A long-lived `ditoxd daemon` watcher (single DB owner) plus Hyprland
  paste-back that refocuses the source window and dispatches the paste shortcut.
- A highly customizable TUI: themes, keymap, multi-select, pinned-only view,
  full preview, mouse support, live polling, and block image previews.
- **Migration-friendly**: common short CLI aliases and config keys are accepted
  so existing clipboard-manager setups migrate with little or no change.

## Contents

- [Install (Nix / NixOS)](#install-nix--nixos)
- [Quick Start (development)](#quick-start-development)
- [CLI Usage](#cli-usage)
- [TUI Customization](#tui-customization)
- [Backend Config](#backend-config)
- [Hyprland Setup](#hyprland-setup)
- [Current Scope](#current-scope)

## Install (Nix / NixOS)

Ditox ships as a Nix flake with prebuilt closures on a public
[Cachix](https://cachix.org) cache, so installing does **not** require building
Zig or bundling the TUI locally. Prebuilt for `x86_64-linux` and `aarch64-linux`.

Trust the binary cache once (otherwise Nix will rebuild from source):

```nix
# NixOS / nix.conf
nix.settings = {
  substituters = [ "https://0xfell.cachix.org" ];
  trusted-public-keys = [
    "0xfell.cachix.org-1:0VSPKbe/Eilt+WTT/0faSQeQnnhDOH7PxkUvoRtvPPo="
  ];
};
```

Or, with the Cachix CLI: `cachix use 0xfell`.

Then run or install:

```sh
nix run github:0xfell/ditox            # try it without installing
nix profile install github:0xfell/ditox
```

The flake also advertises the cache via `nixConfig.extra-substituters`, so
`nix run github:0xfell/ditox` will offer to use it (accept the prompt or pass
`--accept-flake-config`).

### Home Manager

```nix
{
  inputs.ditox.url = "github:0xfell/ditox";

  # in your home configuration:
  imports = [ inputs.ditox.homeManagerModules.default ];
  programs.ditox = {
    enable = true;
    systemd.enable = true;   # run `ditoxd daemon` as a user service
  };
}
```

## Quick Start (development)

```sh
nix develop
bun install
bun run check
```

Build the binaries, then exercise the backend CLI:

```sh
./zig-out/bin/ditox add "hello"
./zig-out/bin/ditox list
./zig-out/bin/ditox print 1
./zig-out/bin/ditoxd serve --stdio < contracts/fixtures/health.check.rpc
```

## CLI Usage

Common short aliases are accepted for supported operations:

```sh
# Capture / output
./zig-out/bin/ditox -a "save this"          # add an entry
./zig-out/bin/ditox -c "copy only"          # copy input to clipboard
./zig-out/bin/ditox -p                       # print current clipboard
./zig-out/bin/ditox -v                       # version
./zig-out/bin/ditox --output-all unescaped
wl-paste --watch ./zig-out/bin/ditox --wl-store

# Watcher lifecycle
./zig-out/bin/ditox -listen                  # start the Wayland watcher
./zig-out/bin/ditox -pause 5m                # pause capture temporarily
./zig-out/bin/ditox -kill                    # stop stored watcher PID

# Maintenance
./zig-out/bin/ditox -clear                   # clear, keep pinned
./zig-out/bin/ditox -clear-all               # clear, include pinned
./zig-out/bin/ditox -clean                   # sanitize text, reconcile images

# Paste-back
./zig-out/bin/ditox --auto-paste
./zig-out/bin/ditox keep                     # picker stays open after paste
```

Launch the TUI (with a built backend on `PATH` or via `DITOXD`):

```sh
DITOXD=./zig-out/bin/ditoxd bun run --cwd tui start
./zig-out/bin/ditox                          # no args -> open the picker
./zig-out/bin/ditox tui
./zig-out/bin/ditox launch --keep
```

## TUI Customization

The TUI loads optional JSON config from `DITOX_TUI_CONFIG`, then falls back to
`$XDG_CONFIG_HOME/ditox/tui.json` or `~/.config/ditox/tui.json`. Missing files
and keys use built-in defaults.

The fastest way to customize is to copy `tui/tui-config.example.json` and edit
it. It declares `"$schema": "./tui-config.schema.json"`, so editors offer
completion and flag unsupported keys. **The schema and example file are the
source of truth for the full option list** — the categories below are an
overview, not an exhaustive reference.

What you can configure:

- **Themes** — presets (`ditoxDark`, `ditoxLight`, `groknight`, `grokday`,
  `tokyonight`, `rosepine`), per-token colors, and a `themeFile` pointing at a
  `custom_theme.json` (start from `tui/custom_theme.example.json`).
- **Terminal** — alt-screen / screen mode, renderer background, footer height,
  clear-on-shutdown, title and cursor style, opt-in Kitty keyboard protocol, and
  OpenTUI timing/buffer tuning (FPS, debounce, stdin buffer). Most have
  `DITOX_TUI_*` env overrides for one-off runs.
- **Layout** — split width (`listWidthPercent` / `previewWidthPercent`), compact
  mode, history load limit, pane minimum widths, header/status/overlay
  structure, alignment, and width caps for narrow terminals.
- **Surfaces** — per-surface styles (shell, header, list, rows, preview,
  overlays, status line, scrollbar) including semantic colors and text
  attributes (bold, dim, italic, underline, inverse, strikethrough).
- **Previews** — split/full preview length and metadata, line-number gutters,
  and image previews (OpenTUI block vs. text renderer, sizing, alignment,
  Kitty/Sixel capability-aware fallback notices).
- **Labels & glyphs** — every visible label, prompt, marker, status/header
  template, metadata placeholder, unit, and separator is overridable; glyphs can
  be set to empty strings for quieter layouts.
- **Keybindings** — every TUI action is rebindable. Defaults keep `space` for
  preview and `x` / `s` for mark/select. Common action names (`choose`,
  `filter`, `togglePin`, `clearSelected`, `yankFilter`, …) are accepted as
  aliases, and `keyLabels` controls how keys appear in help/hints.
- **Behavior** — `filterOrder` and `startup` choose which filters cycle and the
  opening view; `behavior` controls live search (on by default, debounced),
  whether opening/cancelling search keeps the query, and which actions exit the
  TUI after success.
- **Compatibility keys** — `maxEntryLength`, `pollInterval`, `enableMouse`,
  `enableDescription`, and `imageDisplay.type = basic | kitty | sixel` map onto
  the equivalent Ditox layout behavior.

Tone (`headerLineTones`, `statusTones`, `overlayContentTones`, `listContentTones`,
`previewContentTones`, …) routes individual placeholders to explicit surface
colors; see the schema for the full set.

### Environment overrides

```sh
DITOX_TUI_THEME=ditoxLight DITOX_TUI_LIST_WIDTH=60 bun run --cwd tui start
DITOX_TUI_LIVE_SEARCH=false bun run --cwd tui start          # non-live search
DITOX_TUI_EXIT_AFTER_PASTE=false bun run --cwd tui start     # keep-open debug
```

Keep-open is also available from the CLI: `ditox keep` or
`ditox launch --keep`.

### Golden frames & review artifacts

Golden TUI text frames live in `tui/src/__goldens__`. Refresh them only after
reviewing the visual change:

```sh
cd tui
DITOX_UPDATE_GOLDENS=1 bun test src/tui-golden.test.tsx
```

To export review artifacts (text frame, OpenTUI span data, SVG, and PNG) without
touching goldens:

```sh
cd tui
DITOX_TUI_ARTIFACTS=1 bun test src/tui-golden.test.tsx
```

Artifacts land in `tui/artifacts/frames/` (Git-ignored). Set
`DITOX_TUI_ARTIFACT_DIR=/tmp/ditox-frames` to redirect them.

## Backend Config

The backend reads TOML or JSON from `DITOX_CONFIG`, then falls back to
`$XDG_CONFIG_HOME/ditox/config.toml` or `~/.config/ditox/config.toml`. JSON is
detected when the file starts with `{`, so an existing JSON
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

Key behaviors:

- `delete_after_seconds` enables delete-after retention: old non-pinned entries
  are removed automatically while pinned entries remain. Image blobs removed by
  retention, clear, or delete are pruned from disk.
- `[auto_paste]` configures paste-key behavior. `ditox --auto-paste` sends the
  configured keybind through Hyprland, and normal paste-back reuses it after
  writing the selected entry to the clipboard.
- `[capture]` configures excluded-app capture rules. The watcher matches the
  active window class against `excluded_apps` and the title against
  `excluded_windows` (case-insensitive substring) before storing anything.

### Config aliases

For migrations, the backend also accepts these top-level TOML/JSON aliases:
`maxHistory`, `deleteAfter`, `allowDuplicates`, `pollInterval`, and
`maxEntryLength`. `historyFile` maps to the SQLite database path and `tempDir` to
the image blob directory (relative values resolve from the config file's
directory). `~/`, `$HOME`, `$XDG_CONFIG_HOME`, and `$XDG_DATA_HOME` prefixes are
expanded. `excludedApps` / `excludedWindows` map to the capture exclusion lists,
and `autoPaste.enabled` / `autoPaste.keybind` / `autoPaste.buffer` map to the
auto-paste settings.

## Hyprland Setup

The target launch model:

```ini
exec-once = ditoxd daemon
bind = SUPER, V, exec, ditox
```

`ditox` with no arguments — and `ditox launch`, `ditox launch --keep`,
`ditox keep` — capture the active Hyprland window before opening the terminal and
pass it to the TUI as `DITOX_TARGET_WINDOW`. The `keep` variants also set
`DITOX_TUI_EXIT_AFTER_PASTE=false` for that launched TUI.
`ditox -enable-real-time` (and the `launch` / `tui` `--enable-real-time`
variants) launch the picker with `DITOX_TUI_REFRESH_MS=250` for live polling.

Paste-back writes the selected entry with `wl-copy`, refocuses the captured
window with `hyprctl`, and dispatches the configured paste shortcut (`ctrl+v` by
default).

The watcher is the `ditoxd daemon`: a single long-lived DB-owner process running
the full capture loop. This is the structural fix for DB-lock / stale-watcher
issues; JSON-RPC stays short-lived stdio for now.

### Shipping format

Normal builds ship the TUI as bundled JavaScript (`tui/dist/index.js`, run with
`bun`); source execution via `bun run --cwd tui start` remains the dev fallback.
An installed `ditox` binary looks for bundled TUI entrypoints next to itself
first (including `../share/ditox/tui/dist/index.js`) before falling back to a
development checkout. `bun run build` builds the TUI bundle before the Zig
install step, so `zig-out/share/ditox/tui/dist/` holds the runnable bundle and
`zig-out/share/ditox/tui/` holds the example config, schema, and custom-theme
example.

## Current Scope

Shipped today:

- SQLite schema migrations and relevance-ranked FTS5 search (literal substring +
  boundary-aware fuzzy subsequence fallback).
- Metadata-first image capture under `images-v2`; selecting an image writes the
  stored blob back to the clipboard with its MIME type.
- Method-specific JSON-RPC contracts, file-backed OpenTUI customization, and a
  configurable keymap.
- Multi-select bulk copy/output, search-match yank/copy, highlighted literal and
  fuzzy matches, range selection, pinned-only view, and full preview mode.
- Mouse selection / right-click marking / scrolling (disable via config for
  keyboard-only workflows).
- Pinned-safe clear flows, live TUI polling, watcher health in the status line,
  pause/resume, delete-after TTL cleanup for non-pinned entries, and
  storage clean/repair (sanitizes text rows, removes image rows with missing
  blobs).
- Short CLI aliases (version/add/copy-input/print-clipboard/wl-store/
  auto-paste/output-all/listen/clear/clean/kill/pause/realtime-launch) with
  explicit "unsupported" messages for X11/macOS listener aliases.
- PNG, JPEG, GIF (first frame), WebP, and uncompressed BMP previews as
  OpenTUI-supersampled or text-span block renders, with capability-aware
  Kitty/Sixel fallback notices.

Intentionally deferred: native Kitty/Sixel protocol rendering, daemon sockets,
sync, scripting, Windows, and X11.

For clipboard-safe tests or demos, set `DITOX_CLIPBOARD_MOCK=/tmp/ditox-clip`
before running `ditox copy` or `ditox paste`.
