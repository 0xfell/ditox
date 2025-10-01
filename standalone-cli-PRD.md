# Ditox Standalone CLI (Embedded Daemon) — PRD

- Doc: standalone-cli-PRD.md
- Owner: 0xfell
- Status: Draft (v0.1)
- Target release: v1.1.0 (scope can be split if needed)

## Summary

Ship a “standalone” Ditox experience where the `ditox` CLI/TUI runs with an embedded clipboard watcher (daemon) while the TUI is open. Users install and run only `ditox-cli`; it will capture clipboard changes in the background for the lifetime of the TUI session. The separate `ditox-clipd` binary remains available for long‑running, headless use, but regular users no longer need to manage a daemon explicitly.

## Goals

- Zero‑setup experience: `cargo install ditox-cli` → `ditox` captures clipboard while the UI is open.
- Embedded daemon (“managed daemon”) lifecycle is tied to the TUI: start at TUI launch; stop when TUI exits.
- No data loss vs external daemon: same capture fidelity, same store, image support.
- Sensible defaults on Linux (X11/Wayland) with `arboard`.
- Safe coexistence with the external daemon: if `ditox-clipd` is already running, do not double‑capture.
- Keep power‑user paths intact (systemd timers, headless daemon, remote backends).

## Non‑Goals (Initial)

- Replacing the external daemon for long‑running/background use across reboots.
- Implementing Windows/macOS native clipboard watchers (current adapters remain no‑op or limited).
- Cross‑process IPC beyond what’s required to detect an existing daemon.

## User Stories / UX

- As a first‑time user, I run `ditox` and immediately see history populate as I copy text/images in other apps. When I quit the TUI, background capture stops.
- As an existing user with `ditox-clipd` running (e.g., via systemd), launching `ditox` uses the existing history; no duplicate captures or lock errors occur.
- As a privacy‑conscious user, I can start the TUI with capture disabled (`--daemon=off`) or pause/resume capture from inside the TUI.
- As a power user, I can still run `ditox-clipd` headless for continuous capture and use the TUI only as a viewer.

## CLI Surface (Proposed)

Defaults target simplicity. Flags are additive and remain backward compatible.

- `ditox` (alias: `ditox tui`)
  - Starts TUI.
  - Default capture mode: `--daemon=managed` (embedded watcher).
  - New flags:
    - `--daemon <managed|external|off>` (default `managed`)
      - `managed`: spawn embedded watcher inside the same process.
      - `external`: do not spawn; expect external `ditox-clipd` or none.
      - `off`: do not capture; view‑only.
    - `--daemon-sample <dur>` sampling interval (default: `200ms`).
    - `--daemon-images <on|off>` (default: `on`).
    - `--no-auto-migrate` (mirrors existing, passthrough to store).
  - TUI hotkeys (visible in help):
    - `p`: pause/resume capture.
    - `D`: toggle image capture on/off for session.

- `ditox doctor`
  - Reports “Managed Daemon: active|paused|off”, backend (X11/Wayland), sample interval, lock owner (pid), and conflict detection (external daemon present: yes/no).

- `ditox daemon …` (existing external daemon remains; docs point casual users to just `ditox`).

## Architecture & Design

### High‑Level

- Embed a clipboard watcher (“managed daemon”) into `ditox-cli` when in TUI mode.
- Watcher runs in the same process as the TUI and writes to the same SQLite store via `ditox-core::Store` with identical semantics to `ditox-clipd`.
- On startup:
  1) Detect external daemon (pidfile/lockfile probe).
  2) If found and `--daemon=managed`, switch to `external` mode silently (no embedded watcher) to avoid double capture.
  3) If none and `--daemon=managed`, spawn a background task (Tokio) that:
     - polls clipboard at configured interval;
     - deduplicates consecutive identical content (hash, timestamp);
     - persists entries via the store; supports images behind feature gate.
  4) When TUI exits, task shuts down gracefully.

### Process/Task Model

- Single OS process, multiple async tasks:
  - UI task (ratatui + crossterm) — existing.
  - Managed watcher task (Tokio) — new.
  - Optional: background maintenance (pruning, thumbs) — reused from existing paths.
- Runtime: enable `tokio` for the CLI by default (we already use it behind `libsql`). UI remains synchronous with a small bridge to spawn tasks.

### Concurrency & Safety

- Database access: reuse `ditox-core` connection management; one writer task (watcher) + UI readers.
- Single‑instance capture guard:
  - File lock in XDG state dir (e.g., `~/.local/state/ditox/managed-daemon.lock`) containing pid + start time.
  - If lock exists and pid alive → assume external/other capture active; do not start embedded watcher.
  - If stale lock → clean up and continue.

### Clipboard Backends

- Linux: `arboard` with Wayland data‑control and X11; identical to existing daemon behavior.
- Non‑Linux: watcher task becomes a no‑op; UI works without capture (documented).

### Images

- Honor existing `ClipKind::Image` support from core, storing blobs under `objects/aa/bb/<sha256>`.
- Session‑toggle for image capture to mitigate performance on slow systems.

### Configuration

- Use existing config file (`~/.config/ditox/settings.toml`). New keys:
  - `[daemon]
     mode = "managed" | "external" | "off"
     sample = "200ms"
     images = true`
- CLI flags override config, config overrides defaults.

### Compatibility & Coexistence

- If `ditox-clipd` runs (systemd, user session), `ditox` detects it via pid/lock and does not spawn an embedded watcher.
- If both start simultaneously (race), file lock resolves to a single active capturer.
- The `ditox-clipd` crate remains published for headless use; we may later convert its internals into a library crate to share code (see Migration).

## Migration / Refactor Plan

Phase 0 (prep)
- Extract reusable watcher logic from `ditox-clipd` into a new internal module (`crates/ditox-cli/src/managed_daemon.rs`) by copy‑move first, then deduplicate with `ditox-clipd` in a follow‑up.

Phase 1 (MVP)
- Add managed daemon to CLI TUI with defaults above.
- Add locking, sampling, image toggle, and pause/resume in TUI.
- Update `doctor` and docs.

Phase 2 (Consolidation)
- Refactor `ditox-clipd` into a small bin that calls a new `clipd-lib` (shared between CLI managed daemon and external daemon) to eliminate duplication.

Phase 3 (Optional)
- Provide `--detach` to keep the managed daemon running after TUI exit (off by default; not in MVP scope).

## Security & Privacy

- No secret logging; redact clip contents in logs and tests.
- Keep “incognito” toggle (do not persist) as a future enhancement.
- Respect image data handling and storage budget as today.

## Performance Targets

- CPU: < 1% on idle with 200ms sampling on typical Linux desktop.
- Memory: < 50 MiB for TUI + watcher combined in steady state.
- DB I/O: avoid excessive writes via dedup + backoff for bursty changes.

## Failure Modes & Handling

- Clipboard backend unavailable → surface non‑fatal warning in TUI status bar and `doctor`.
- DB locked/busy → retry with exponential backoff; drop frames rather than freeze UI.
- External daemon detected → do not start managed watcher; show status “External daemon active”.

## Testing

- Unit tests for watcher dedup, sampling, and store writes (using temp SQLite).
- CLI tests (assert_cmd) for mode negotiation and lock behavior.
- Doctor output assertions.
- Manual smoke on X11 and Wayland (GitHub Actions can’t capture clipboard; rely on local CI matrix guidance).

## Documentation

- README: simplify Quick Start — “Run `ditox` and start copying; no separate daemon required.”
- Update help for new flags and TUI shortcuts.
- Keep a section for advanced users describing the external daemon.

## Rollout

- Ship as a minor release v1.1.0 (behavioral change but backward compatible).
- Keep `ditox-clipd` crate published. After Phase 2, mark it as “optional for power users” in docs.

## Open Questions

1) Should `managed` be the default for `ditox tui` only, or for any `ditox` command that keeps the process running? (Proposal: TUI only.)
2) Do we want a keybind to toggle between `external` and `managed` live? (Likely no for MVP.)
3) Should we support `--detach` to keep capture after TUI exit? (Out of MVP, possibly Phase 3.)
4) Where to host the shared daemon code long‑term: inside `ditox-core` or a new `clipd-lib` crate? (Proposal: `clipd-lib`.)

## Acceptance Criteria

- Running `ditox` on Linux starts the TUI and begins capturing clipboard changes without any external daemon.
- Closing the TUI stops capture (no background processes left running).
- If an external daemon is running, `ditox` does not start an embedded watcher and clearly indicates “External daemon active”.
- `doctor` shows correct managed daemon status and environment details.
- No regression to list/search/copy/image flows; tests pass; CI green.

