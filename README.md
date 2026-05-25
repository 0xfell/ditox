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

Run the TUI with a built backend on `PATH` or through `DITOXD`:

```sh
DITOXD=./zig-out/bin/ditoxd bun run --cwd tui start
./zig-out/bin/ditox tui
```

## Hyprland Shape

The target launch model is:

```ini
exec-once = ditoxd watch
bind = SUPER, V, exec, ditox launch
```

`ditox launch` captures the active Hyprland window before opening the terminal.
Paste-back writes the selected entry to the clipboard with `wl-copy`, refocuses
the captured window with `hyprctl`, and dispatches `sendshortcut "CTRL,V,"`.

The watcher remains a long-running process (`ditoxd watch`). JSON-RPC stays a
short-lived stdio command (`ditoxd serve --stdio`) for now, which keeps the
OpenTUI process simple while storage and capture semantics settle.

The TUI shipping format is bundled JavaScript for normal builds
(`tui/dist/index.js`, run with `bun`). Source execution through
`bun run --cwd tui start` remains the fallback for development checkouts, and a
compiled Bun executable is deferred until the TUI API stops moving.

## Current Scope

The current implementation includes SQLite schema migrations, FTS5 search,
metadata-first image capture under `images-v2`, JSON-RPC method contracts,
OpenTUI keymap bindings, multi-select bulk copy/output, pause/resume watcher
state, and Hyprland paste-back targeting. Inline terminal image rendering,
daemon sockets, sync, scripting, Windows, and X11 are intentionally deferred.

For clipboard-safe tests or demos, set `DITOX_CLIPBOARD_MOCK=/tmp/ditox-clip`
before running `ditox copy` or `ditox paste`.
