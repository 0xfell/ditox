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
bun run --cwd tui contracts
zig build
bun run --cwd tui test
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

## Current Scope

The scaffold implements the contract boundary, SQLite-backed text history,
basic JSON-RPC, operational CLI commands, a `wl-clipboard` adapter, and an
OpenTUI Solid shell. Image terminal previews, daemon sockets, sync, scripting,
Windows, and X11 are intentionally deferred.
