# Ditox Roadmap

> **Current Version:** 0.3.1
> **Target:** v1.0 — full Ditto feature parity, cross-platform first-class
> **Plan:** see [`notes/master-plan-v1.md`](notes/master-plan-v1.md)

## Status Overview

| Category | Count |
|----------|-------|
| Completed | 23 |
| In Progress | 3 |
| Planned (Phase 0 — Foundation) | 0 |
| Planned (Phase 1-8 epics + carry-overs) | 9 |

---

## Plan summary

The v0.4 → v1.0 master plan is in [`notes/master-plan-v1.md`](notes/master-plan-v1.md).
Architectural comparison with Ditto in [`notes/ditto-comparison.md`](notes/ditto-comparison.md).
Hyprland-specific user setup in [`notes/hyprland-setup.md`](notes/hyprland-setup.md).
Internal UI design in [`notes/ui-replication.md`](notes/ui-replication.md).

| Phase | Theme | Tasks | Schema |
|---|---|---|---|
| 0 | Foundation hardening | `014`-`022` | v1 → v2 |
| 1 | Multi-format capture | `023` | v2 → v3 |
| 2 | Paste-back UX (cross-platform) | `024` | none |
| 3 | Power-user features | `025` | v3 → v4 |
| 4 | Ditto UX replication (long-running, layer-shell) | `026` | none |
| 4b | GUI parity (settings, collections, multi-select, tags) | `027` | v4 → v5 |
| 5 | Hotkeys, IPC, Rhai scripting | `028` | v5 → v6 |
| 6 | LAN sync (TOFU + Noise) | `029` | v6 → v7 |
| 7 | Distribution & i18n | `030` | none |
| 8 | macOS port | `031` | none |

---

## In Progress

| Task | Description |
|------|-------------|
| [Windows installer & distribution](tasks/in-progress/004-windows-installer-distribution.md) | Inno Setup installer + code-signing (signing pending cert) |
| [Windows 11 support](tasks/in-progress/010-windows-11-support.md) | Largely complete; CLI-test gaps remain |
| [GUI improvements](tasks/in-progress/gui-improvements.md) | Pre-Phase-4 punch list; some items folded into Phase 4 |

---

## Planned

### Phase 0 — Foundation (immediate)

_All Phase 0 tasks complete (014-022). Workspace is at the v0.4 quality bar; ready to tag and start Phase 1._

### Phase 1-8 — Epics (one task per phase)

| Task | Description | Schema |
|------|-------------|--------|
| [024 Paste-back UX](tasks/planned/024-phase-2-paste-back.md) | Foreground tracker + keystroke synthesis chain (Win/Hyprland/Sway) | none |
| [025 Power-user features](tasks/planned/025-phase-3-power-user.md) | Special paste, exclusion, color swatch, filter rules, suspend/resume | v3 → v4 |
| [026 Ditto UX replication](tasks/planned/026-phase-4-ditto-ux.md) | Long-running daemon + layer-shell + position modes + tooltip preview | none |
| [027 GUI feature parity](tasks/planned/027-phase-4b-gui-parity.md) | Settings, collections, multi-select, tags, time chips, theming | v4 → v5 |
| [028 Hotkeys, IPC, Rhai scripting](tasks/planned/028-phase-5-hotkeys-ipc-scripting.md) | Per-clip hotkeys, full IPC, sandboxed scripting hooks | v5 → v6 |
| [029 LAN peer-to-peer sync](tasks/planned/029-phase-6-lan-sync.md) | mDNS + ed25519 TOFU + Noise transport, content-addressed | v6 → v7 |
| [030 Distribution & i18n](tasks/planned/030-phase-7-distribution-i18n.md) | Choco/Winget/AUR/Flatpak/MSIX, signing, 5 locales, crash reporting | none |
| [031 macOS port](tasks/planned/031-phase-8-macos.md) | NSPasteboard, Accessibility flow, .app bundle, Cask, notarisation | none |
| [032 Windows multi-format capture](tasks/planned/032-phase-1-windows-multi-format.md) | Spun out from 023 sub-task 1.4: `AddClipboardFormatListener` event-driven capture; needs Windows-side validation | none |

---

## Recently Completed

| Task | Date | Description |
|------|------|-------------|
| [023 Multi-format clipboard capture](tasks/completed/023-phase-1-multi-format-capture.md) | 2026-04-26 | Phase 1: 8/9 sub-tasks. Schema v2→v3 with `entry_formats` table + `format_content_fts`; `FormatId` + Wayland/Win32 canonicalisation; HTML envelope + RTF \rsid stripping; per-format hashing; multi-format `Database::insert_multi` with rollback-on-failure; multi-format FTS5 search; `CaptureConfig` mode/size caps; `WaylandLibraryCapture` via `wl-clipboard-rs` (live-tested on Hyprland); `FormatAggregator` trait + 5 impls (PlainText/HtmlEnvelope/Rtf/UriList/ImageStack). Sub-task 1.4 (Windows event-driven capture) spun out as task 032. +87 tests, 153 total. |
| [022 Layer-shell research spike](tasks/completed/022-foundation-layer-shell-spike.md) | 2026-04-26 | Built A1 prototype (`spike/a1-iced-layershell/`, 179 LOC, builds in 1.71s); decision: adopt `iced_layershell = "=0.17.1"` over hand-rolled SCTK + tiny-skia. ADR at `docs/notes/adr/0001-layer-shell-strategy.md`. Hyprland verified by user; Sway/KDE/GNOME/longevity tests pending. |
| [017 DB actor](tasks/completed/017-foundation-async-db-actor.md) | 2026-04-26 | Replaced GUI's `Arc<Mutex<Database>>` with closure-based actor on a dedicated thread; cheap-clone `DbHandle` exposes `call`/`try_call`/`dispatch`/`flush`; bounded queue (64) for backpressure; 8 tests including 1000-insert stress. |
| [018 `CaptureSource` trait](tasks/completed/018-foundation-capture-trait.md) | 2026-04-26 | Generalised watcher to consume `Vec<Box<dyn CaptureSource>>`; sync trait + `RawClip`/`RawFormat` model + `PollingCaptureSource` adapter + `MockCaptureSource`; 13 new tests (7 unit + 6 integration). |
| [021 Compositor / OS detection](tasks/completed/021-foundation-compositor-detection.md) | 2026-04-26 | `Platform` enum with cached `OnceLock` detect, capability flags, paste-chain heuristic, `ditox status` integration; 8 tests. |
| [019 Schema v1 → v2 migration](tasks/completed/019-foundation-schema-v2.md) | 2026-04-26 | `entry_kind`/`format_count`/`source_app`/`captured_at` + `idx_entries_source_app` index; idempotent forward-only migration; 4 tests. |
| [020 `tracing` everywhere](tasks/completed/020-foundation-tracing-logging.md) | 2026-04-26 | Replaced `eprintln!` with structured `tracing` events; `logging::init(Mode::{Stderr,File,Journald})` shared across binaries. |
| [016 Watcher daemon hardening](tasks/completed/016-foundation-watcher-daemon-hardening.md) | 2026-04-26 | `fs2` flock + atomic heartbeat + `ctrlc` SIGTERM/SIGINT; `WatcherStatus`, `stop_watcher`; CLI flags `--stop --status --json --journal`; systemd user unit; 6 tests. |
| [015 Reconcile docs with reality](tasks/completed/015-foundation-docs-reconciliation.md) | 2026-04-26 | Brought ROADMAP/AGENTS/notes in line with the post-013 GUI model and pruned legacy IPC references. |
| [014 Honour `storage.data_dir`](tasks/completed/014-foundation-data-dir-fix.md) | 2026-04-26 | Fixed parsed-but-ignored config key; process-wide override via `set_data_dir_override`; 7 tests. |
| [Floating-launcher GUI redesign](tasks/completed/013-floating-launcher-redesign.md) | 2026-04-26 | One-shot GUI: each launch opens a 420×520 floating panel at bottom-left; copy/Esc/unfocus/close exits the process. **Note:** Phase 4 (`026`) reverts the one-shot model to a long-running daemon with IPC; visual design retained. |
| [Release Infrastructure](tasks/completed/012-release-infra.md) | 2026-04-25 | CI + release workflows (GitHub Actions), prebuilt Linux/Windows binaries (TUI tarball, musl static, AppImage, Windows zip), Cachix push, README rewrite, versions bumped to 0.3.0 |
| [Image Storage Bug Fix](tasks/completed/011-image-storage-bug.md) | 2026-04-25 | Content-addressed image store, refcount prune queue, schema v1 migration, `ditox repair` command. Fixes 4 disk-leak bugs. |
| [Linux GUI](tasks/completed/010-linux-gui.md) | 2026-04-24 | Cross-platform `ditox-gui` (Wayland/X11) with tray, `--toggle` IPC, XDG autostart |
| [Delete Confirmation in TUI](tasks/completed/009-delete-confirmation-tui.md) | 2025-12-02 | Add confirmation dialogs for delete operations (`d` and `D`) |
| [TUI Pagination](tasks/completed/005-tui-pagination.md) | 2025-11-27 | Lazy loading & pagination for 126x faster startup, 500x memory reduction |
| [TUI Polish & Refinements](tasks/completed/008-tui-polish.md) | 2025-11-27 | Entry type icons, line numbers, terminal size handling, message timeout, auto-help |
| [Feature Bundle Implementation](tasks/completed/007-feature-bundle-implementation.md) | 2025-11-27 | Implementation of 10 selected features (notes, stats, collections, etc.) |
| [Feature Ideas Brainstorm](tasks/completed/006-feature-ideas-brainstorm.md) | 2025-11-27 | 20 feature ideas for future development |
| [TUI UI Improvements](tasks/completed/004-tui-ui-improvements.md) | 2025-11-27 | UI/UX enhancements: scrollbar, mouse support, multi-select, search highlighting |
| [Tab Crash in Ghostty](tasks/completed/003-tab-crash-ghostty.md) | 2025-11-27 | Fix Tab key crash when using Ghostty terminal |
| [CLI Parity](tasks/completed/002-cli-parity.md) | 2024-11-27 | Add missing CLI commands (get, search, delete, pin, count) |
| [Core Implementation](tasks/completed/001-core-tui-cli.md) | 2024-11 | Initial TUI, watcher, basic CLI, NixOS integration |

---

## Quick Reference

### What's Working (v0.3.1)

**TUI (`ditox`):** Full feature set — list, search, copy, delete, pin, preview,
pagination, notes, stats, collections.

**GUI (`ditox-gui`):** One-shot floating launcher (420×520, bottom-left).
Each launch opens a fresh window; click an entry / Enter / Esc / unfocus
exits the process. Tab opens a side inspector panel.
- **Windows:** system tray, Ctrl+Shift+V global hotkey, auto-start via
  registry, Win32 focus recovery for Win+D.
- **Linux (Wayland + X11):** system tray (StatusNotifierItem via
  libappindicator); compositor keybind launches a fresh process per press.
  `--toggle` / `--show` / `--hide` retained as no-op compatibility shims.

**CLI (`ditox`):**
- `ditox` — TUI
- `ditox watch` — Watcher daemon
- `ditox list [--limit N] [--json] [--pinned]`
- `ditox get <target> [--json]` — Get full content
- `ditox search <query> [--limit N] [--json]` — Fuzzy search
- `ditox copy <target>` — Copy to clipboard
- `ditox delete <target>` — Delete entry
- `ditox favorite <target>` — Toggle favorite
- `ditox count` — Print entry count
- `ditox clear [--confirm]` — Clear history
- `ditox status` — Show status
- `ditox stats` — Show usage statistics
- `ditox collection …` — Manage collections
- `ditox repair [--dry-run] [--fix-hashes]` — Reconcile image store with DB

**GUI CLI (`ditox-gui`):** `--toggle`, `--show`, `--hide`, `--quit`, `--help`,
`--version`.

### Performance (v0.3.0)

| Metric | Result |
|--------|--------|
| First page load (10k entries) | 0.19ms |
| Startup speedup | 126.8x faster |
| Memory reduction | ~500x |
| Page navigation | ~0.25ms/page |
| Search (10k entries) | <2ms |

### Cross-platform support matrix (target after Phase 4)

| Feature | Windows | Hyprland | Sway | KDE Wayland | GNOME Wayland | macOS |
|---|---|---|---|---|---|---|
| Capture | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (Phase 8) |
| Long-running launcher | ✅ | ✅ via layer-shell | ✅ via layer-shell | ✅ | 🟡 xdg_toplevel | ✅ |
| Foreground tracking | ✅ | ✅ | ✅ | 🟡 | ❌ | ✅ |
| Paste-back synthesis | ✅ SendInput | ✅ hyprctl | ✅ wtype | ✅ wtype | 🟡 ydotool | ✅ CGEvent |
| Global hotkey (in-app) | ✅ | ❌ → compositor bind | ❌ → compositor bind | ❌ → KGlobalAccel (later) | ❌ → manual | ✅ |
| Tray icon | ✅ | ✅ via waybar/hyprpanel | ✅ via waybar | ✅ Plasma | 🟡 needs extension | ✅ |
| Run-at-login | ✅ registry | ✅ exec-once | ✅ exec | ✅ autostart | ✅ autostart | ✅ LaunchAgent |
| Per-clip global hotkey | ✅ | ✅ via managed binds | ✅ via managed binds | ❌ (later) | ❌ (later) | ✅ |

### File Locations

**Linux:**
- Tasks: `docs/tasks/{completed,in-progress,planned}/`
- Notes: `docs/notes/`
- Config: `~/.config/ditox/config.toml`
- Data: `~/.local/share/ditox/`
- Identity (Phase 6): `~/.config/ditox/identity.{key,pub}`
- Hyprland helper output (Phase 4): `~/.config/hypr/conf.d/ditox.conf`,
  `~/.config/hypr/conf.d/ditox-binds.conf`
- IPC socket (Phase 4): `$XDG_RUNTIME_DIR/ditox-gui-${UID}.sock`
- Watcher PID: `~/.local/share/ditox/watcher.pid`
- Watcher heartbeat: `~/.local/share/ditox/watcher.heartbeat`

**Windows:**
- Config: `%APPDATA%/ditox/config.toml`
- Data: `%APPDATA%/ditox/`
- IPC named pipe (Phase 4): `\\.\pipe\ditox-gui-{Username}`
- WER dumps: `%APPDATA%/ditox/Dumps/`

**macOS (Phase 8):**
- Config: `~/Library/Application Support/ditox/config.toml`
- Data: `~/Library/Application Support/ditox/`
