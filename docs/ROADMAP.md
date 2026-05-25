# Ditox Roadmap

> **Current version:** 0.3.1
> **Product direction:** TUI-only clipboard manager
> **Frontend direction:** OpenTUI/Solid terminal UI, backed by the Rust native core

## Status Overview

| Category | Count |
|----------|-------|
| Completed | 24 |
| In Progress | 3 |
| Planned | 5 |

## Product Shape

Ditox is focused on one excellent terminal experience:

- Rust remains responsible for clipboard capture, paste-back, SQLite, image storage, sync, config, and platform integration.
- The current Rust/Ratatui TUI stays as the working production interface while the next frontend is designed.
- The next frontend target is Bun + TypeScript + SolidJS + OpenTUI, following the opencode-style terminal UI stack.
- No desktop windowing or visual desktop surface is part of the product scope.

## In Progress

| Task | Description |
|------|-------------|
| [010 Windows 11 support](tasks/in-progress/010-windows-11-support.md) | Validate and finish CLI/native clipboard support on Windows. |
| [034 wlr-foreign-toplevel subscription](tasks/in-progress/034-phase-2-wlr-foreign-toplevel.md) | Generic Wayland foreground tracker for non-Hyprland wlroots compositors. |
| [036 Linux final product](tasks/in-progress/036-linux-final-product.md) | TUI-only Linux release hardening and verification. |

## Planned

| Task | Description | Schema |
|------|-------------|--------|
| [030 Distribution & i18n](tasks/planned/030-phase-7-distribution-i18n.md) | Package the TUI/CLI, add localization hooks, and document supported channels. | none |
| [031 macOS port](tasks/planned/031-phase-8-macos.md) | NSPasteboard capture/write, accessibility paste-back, and terminal-first packaging. | none |
| [032 Windows multi-format capture](tasks/planned/032-phase-1-windows-multi-format.md) | Event-driven Windows capture with Win32 clipboard format enumeration. | none |
| [033 Windows paste-back](tasks/planned/033-phase-2-windows-paste-back.md) | Win32 foreground tracking and SendInput paste synthesis. | none |
| [038 OpenTUI terminal frontend](tasks/planned/038-opentui-terminal-frontend.md) | Build the next-generation TUI with Bun, TypeScript, SolidJS, and OpenTUI. | TBD |

## Recently Completed

| Task | Date | Description |
|------|------|-------------|
| [037 Remove desktop frontend](tasks/completed/037-remove-desktop-frontend.md) | 2026-05-25 | Removed the desktop crate, packaging, release jobs, runtime dependencies, and docs; pivoted the roadmap to TUI-only with OpenTUI as the next frontend direction. |
| [029 LAN peer-to-peer sync](tasks/completed/029-phase-6-lan-sync.md) | 2026-04-27 | Opt-in LAN sync with local identity, mDNS discovery, trust controls, Noise transport, trusted TCP pulls, metadata sync, and image chunk transfer. |
| [028 Hotkeys, IPC, Rhai scripting](tasks/completed/028-phase-5-hotkeys-ipc-scripting.md) | 2026-04-28 | Per-clip hotkey metadata, scripting hooks, force-capture command, script starter examples, and expanded command surfaces for automation. |
| [025 Power-user features](tasks/completed/025-phase-3-power-user.md) | 2026-04-26 | Special-paste transforms, per-app capture exclusion, color swatches, filter rules, suspend/resume awareness, search prefixes, and URL templates. |
| [024 Paste-back UX](tasks/completed/024-phase-2-paste-back.md) | 2026-04-26 | Foreground tracking, Linux paste synthesis chain, paste sentinel, and selection cursor groundwork. |
| [023 Multi-format clipboard capture](tasks/completed/023-phase-1-multi-format-capture.md) | 2026-04-26 | Multi-format storage/search, Wayland capture, canonical format handling, and format aggregation. |
| [016 Watcher daemon hardening](tasks/completed/016-foundation-watcher-daemon-hardening.md) | 2026-04-26 | Flock PID file, heartbeat, signal handling, status/stop commands, journald mode, and systemd user unit. |
| [011 Image Storage Bug Fix](tasks/completed/011-image-storage-bug.md) | 2026-04-25 | Content-addressed image store, refcount prune queue, schema migration, and repair command. |

## Quick Reference

### Working Surface

- `ditox` - interactive TUI.
- `ditox watch` - watcher daemon.
- `ditox list|get|search|copy|delete|favorite|clear|count|status|stats|repair` - CLI operations.
- `ditox collection ...` - collection management.
- `ditox sync ...` - LAN sync operations.

### Linux Support Matrix

| Feature | Hyprland | Sway | KDE Wayland | GNOME Wayland |
|---|---|---|---|---|
| Capture | yes | yes | yes | yes |
| TUI | yes | yes | yes | yes |
| Foreground tracking | hyprctl + wlr | wlr | protocol-dependent | no |
| Paste-back synthesis | hyprctl | wtype | wtype where available | ydotool/manual |
| Global hotkey | compositor bind | compositor bind | compositor bind/manual | manual |

### File Locations

**Linux:**
- Tasks: `docs/tasks/{completed,in-progress,planned}/`
- Notes: `docs/notes/`
- Config: `~/.config/ditox/config.toml`
- Data: `~/.local/share/ditox/`
- Identity: `~/.config/ditox/identity.{key,pub}`
- Watcher PID: `~/.local/share/ditox/watcher.pid`
- Watcher heartbeat: `~/.local/share/ditox/watcher.heartbeat`

**Windows:**
- Config: `%APPDATA%/ditox/config.toml`
- Data: `%APPDATA%/ditox/`

**macOS:**
- Config: `~/Library/Application Support/ditox/config.toml`
- Data: `~/Library/Application Support/ditox/`
