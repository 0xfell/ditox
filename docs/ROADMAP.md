# Ditox Roadmap & Docs Index

Status: terminal-first clipboard manager (Zig backend + OpenTUI/Bun frontend).
See `AGENTS.md` (repo root) for the authoritative architecture overview.

## Documents

| File | Purpose |
|------|---------|
| [`TODO.md`](./TODO.md) | Active worklist — what remains after the OpenTUI + Zig scaffold. |
| [`AGENTS.md`](../AGENTS.md) | Authoritative architecture overview. |

## Current Scope (shipped)

- SQLite schema/migrations, FTS5 search (literal + fuzzy fallback).
- Image capture with magic-byte MIME verification, content-addressed image
  blobs (atomic writes, liveness-checked prune queue, orphan-blob GC in
  repair).
- `entries.get_image` RPC: the TUI fetches preview bytes over RPC and never
  reads blob files from disk.
- Native Kitty/Sixel protocol image rendering (capability-gated, placement
  diffed) with OpenTUI-supersampled / text half-block fallbacks.
- Method-specific JSON-RPC contracts over Content-Length stdio; the TUI keeps
  one persistent `ditoxd serve --stdio` session per process.
- `ditoxd daemon` watcher (single long-lived DB owner, capture errors logged +
  surfaced via `watcher.status.last_error`) + Hyprland paste-back.
- File-backed OpenTUI customization, configurable keymap, multi-select,
  pinned-only view, full preview (soft-wrapped, arrow-key toggle), mouse
  selection, live polling.
- Recency-ordered history: copy/paste records `last_used_at_ms` so re-used
  clips move to the top; row age reflects most-recent activity.
- Short/compatibility CLI aliases and config aliases for easy migration.

## Deferred (intentionally not built yet)

- Compile-time dispatch ↔ schema enforcement for RPC methods (sync is manual).
- Daemon sockets / event stream (RPC is stdio-only, one session per client).
- LAN sync, scripting, Windows, X11, macOS.

## Distribution

- Nix flake; CI builds `x86_64-linux` + `aarch64-linux` and pushes to
  `https://0xfell.cachix.org`. See `README.md` → Install.
