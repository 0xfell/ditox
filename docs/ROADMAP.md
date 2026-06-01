# Ditox Roadmap & Docs Index

Status: terminal-first clipboard manager (Zig backend + OpenTUI/Bun frontend).
See `AGENTS.md` (repo root) for the authoritative architecture overview.

## Documents

| File | Purpose |
|------|---------|
| [`TODO.md`](./TODO.md) | Active worklist — what remains after the OpenTUI + Zig scaffold. |
| [`notes/migration-fresh-start.md`](./notes/migration-fresh-start.md) | Original fresh-start blueprint. Historical; "planned" sections are not current behavior. |
| [`notes/clipse-parity-tui-plan.md`](./notes/clipse-parity-tui-plan.md) | TUI parity plan vs Clipse. |

## Current Scope (shipped)

- SQLite schema/migrations, FTS5 search (literal + fuzzy fallback).
- Metadata-first image capture, content-addressed image blobs with prune queue.
- Method-specific JSON-RPC contracts over Content-Length stdio.
- `ditoxd daemon` watcher (single long-lived DB owner) + Hyprland paste-back.
- File-backed OpenTUI customization, configurable keymap, multi-select,
  pinned-only view, full preview, mouse selection, live polling.
- Clipse-style CLI aliases and config compatibility.

## Deferred (intentionally not built yet)

- Native Kitty/Sixel protocol image rendering (block/text fallback ships today).
- Daemon sockets / event stream (JSON-RPC is short-lived stdio).
- LAN sync, scripting, Windows, X11, macOS.

## Distribution

- Nix flake; CI builds `x86_64-linux` + `aarch64-linux` and pushes to
  `https://ditox.cachix.org`. See `README.md` → Install.
