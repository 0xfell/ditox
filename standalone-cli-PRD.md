# Ditox Standalone CLI (Embedded Daemon) — PRD

- Doc: standalone-cli-PRD.md
- Owner: 0xfell
- Status: Review‑Ready (v1.0)
- Target release: v1.1.0 (scope can be split if needed)
  - Cut as 1–2 PRs if helpful: (1) managed watcher MVP, (2) consolidation/refactor of shared code with `ditox-clipd`.

## Summary

Ship a “standalone” Ditox experience where the `ditox` CLI/TUI runs with an embedded clipboard watcher (daemon) while the TUI is open. Users install and run only `ditox-cli`; it will capture clipboard changes in the background for the lifetime of the TUI session. The separate `ditox-clipd` binary remains available for long‑running, headless use, but regular users no longer need to manage a daemon explicitly.

## Goals

- Zero‑setup experience: `cargo install ditox-cli` → `ditox` captures clipboard while the UI is open.
- Embedded daemon (“managed daemon”) lifecycle is tied to the TUI: start at TUI launch; stop when TUI exits.
- No data loss vs external daemon: same capture fidelity, same store, image support.
- Sensible defaults on Linux (X11/Wayland) with `arboard`.
- Safe coexistence with the external daemon: if `ditox-clipd` is already running, do not double‑capture.
- Keep power‑user paths intact (systemd timers, headless daemon, remote backends).
- Config layering is predictable: CLI flags > env vars > config file > defaults.

## Non‑Goals (Initial)

- Replacing the external daemon for long‑running/background use across reboots.
- Implementing Windows/macOS native clipboard watchers (current adapters remain no‑op or limited).
- Cross‑process IPC beyond what’s required to detect an existing daemon.
 - Telemetry or network calls beyond existing optional sync.

## User Stories / UX

- As a first‑time user, I run `ditox` and immediately see history populate as I copy text/images in other apps. When I quit the TUI, background capture stops.
- As an existing user with `ditox-clipd` running (e.g., via systemd), launching `ditox` uses the existing history; no duplicate captures or lock errors occur.
- As a privacy‑conscious user, I can start the TUI with capture disabled (`--daemon=off`) or pause/resume capture from inside the TUI.
- As a power user, I can still run `ditox-clipd` headless for continuous capture and use the TUI only as a viewer.

### UX Details (TUI)

- Status bar shows capture state continuously:
  - Examples: `capture: managed (200ms, images:on)` · `capture: paused` · `capture: external (pid 1234)` · `capture: off`.
- Hotkeys surface in help overlay: `p` pause/resume, `D` toggle images for session.
- On startup, if an external daemon is detected and `--daemon=managed` is requested, show a one‑line notice: `external daemon detected; using external mode` (no modal).
- If clipboard backend is unavailable, show a non‑fatal warning icon in status with tooltip in help overlay.

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
    - Environment overrides (optional): `DITOX_DAEMON=managed|external|off`, `DITOX_DAEMON_SAMPLE=200ms`, `DITOX_DAEMON_IMAGES=0|1`.
  - TUI hotkeys (visible in help):
    - `Ctrl+P`: pause/resume capture (managed mode).
    - `D`: toggle image capture on/off for session (managed mode).

- `ditox doctor`
  - Reports capture status, backend (X11/Wayland), sample interval, lock owner (pid), and conflict detection (external daemon present: yes/no).
  - Example output (text):
    ```
    capture:
      mode: managed
      status: active
      sample: 200ms
      images: on
      backend: wayland (arboard)
      lock: ~/.local/state/ditox/managed-daemon.lock (pid=43210, alive)
      external_daemon: not detected
    ```

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

Notes

- `tokio` is already enabled by default via `libsql` feature in `ditox-cli`; the managed watcher uses the same runtime. When building without `libsql`, enable a small `tokio` runtime feature for the CLI.

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
 - Lock format (text): `pid=<u32>\nstarted_at_unix=<i64>\nowner=managed|external`.
 - External daemon detection also checks `~/.config/ditox/clipd.json` (already used by CLI) when present.

### Clipboard Backends

- Linux: `arboard` with Wayland data‑control and X11; identical to existing daemon behavior.
- Non‑Linux: watcher task becomes a no‑op; UI works without capture (documented).

### Images

- Honor existing `ClipKind::Image` support from core, storing blobs under `objects/aa/bb/<sha256>`.
- Session‑toggle for image capture to mitigate performance on slow systems.
 - Per‑session caps: configurable max image bytes (default 8 MiB) to avoid spikes; larger images are skipped with a one‑line notice in status bar and `doctor`.

### Configuration

- Use existing config file (`~/.config/ditox/settings.toml`). New keys:
  - `[daemon]
     mode = "managed" | "external" | "off"
     sample = "200ms"
     images = true`
- CLI flags override config, config overrides defaults.
 - Env vars (optional) override config as noted above.

### Compatibility & Coexistence

- If `ditox-clipd` runs (systemd, user session), `ditox` detects it via pid/lock and does not spawn an embedded watcher.
- If both start simultaneously (race), file lock resolves to a single active capturer.
- The `ditox-clipd` crate remains published for headless use; we may later convert its internals into a library crate to share code (see Migration).

### Observability & Logging

- Log level controlled by `RUST_LOG=ditox_cli=info` etc.; redact clip contents by default.
- Structured events:
  - `capture.started { mode, sample_ms, images }`
  - `capture.paused { reason }`
  - `capture.persisted { id, kind, bytes? }` (omit bytes for text; size only for images)
  - `capture.error { category, message }`

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
 - Threat model notes
   - Trust boundary: local user session only; no network unless optional sync is configured.
   - Attack surface: reading/writing clipboard; local SQLite writes; lockfile parsing.
   - Mitigations: validate lockfile contents; handle symlink tricks by opening with `O_NOFOLLOW`; ensure config and state dirs are user‑owned (0700 preferred) and files are 0600 when created.

## Performance Targets

- CPU: < 1% on idle with 200ms sampling on typical Linux desktop.
- Memory: < 50 MiB for TUI + watcher combined in steady state.
- DB I/O: avoid excessive writes via dedup + backoff for bursty changes.
 - Startup: first TUI frame within 120ms on a modern laptop; watcher begins within 300ms after TUI init.

## Failure Modes & Handling

- Clipboard backend unavailable → surface non‑fatal warning in TUI status bar and `doctor`.
- DB locked/busy → retry with exponential backoff; drop frames rather than freeze UI.
- External daemon detected → do not start managed watcher; show status “External daemon active”.
 - Lockfile present but pid not alive → treat as stale; remove and continue (log info).
 - Image exceeds per‑session cap → skip and notify; do not crash.

## Risks & Mitigations

- Double capture across processes
  - Mitigation: single lockfile guard in XDG state dir; verify pid liveness; fast‑fail if another capturer holds the lock.
- Wayland/X11 backend variance
  - Mitigation: fallback logic and clear `doctor` diagnostics; allow `--daemon=external|off` to bypass managed capture.
- UI stalls from DB contention
  - Mitigation: isolate watcher writes on a dedicated task; use backoff + bounded queues; never block UI thread.
- Image memory pressure on large clips
  - Mitigation: session toggle for images; size caps; stream to file under `objects/` rather than holding full images in RAM.
- Stale locks after crashes
  - Mitigation: include monotonic start time + pid in lock; on startup, detect and clean stale locks.
- Test flakiness in CI (no clipboard)
  - Mitigation: gate integration tests; use unit tests with mocked clipboard; mark platform‑specific tests as ignored on CI.
 - Optional libsql runtime + tokio interaction
   - Mitigation: keep watcher isolated; avoid blocking `libsql` futures; prefer single runtime, no nested reactors.

## Testing

- Unit tests for watcher dedup, sampling, and store writes (using temp SQLite).
- CLI tests (assert_cmd) for mode negotiation and lock behavior.
- Doctor output assertions.
- Manual smoke on X11 and Wayland (GitHub Actions can’t capture clipboard; rely on local CI matrix guidance).

Test Matrix

- Linux (X11): local manual; CI runs unit/black‑box only (no real clipboard).
- Linux (Wayland): local manual on wlroots and GNOME; document compositor quirks.
- Non‑Linux: ensure TUI runs with `capture: off` without panics.

KPIs / Success Metrics

- 90%+ of new users see live captures within first 5s of launching `ditox` on Linux.
- No increase in reported “double capture” or DB lock issues vs v1.0.x.
- Runtime budgets above are met on dev laptops.

## Work Breakdown & Checklists

- CLI (crates/ditox-cli)
  - [ ] Add `--daemon` flag (`managed|external|off`) with default `managed` for `tui`.
  - [ ] Implement `managed_daemon` task and lifecycle wiring.
  - [ ] Add pause/resume and image toggle bindings in TUI.
  - [ ] Update `doctor` to report managed daemon status and env.
  - [ ] Add env var parsing for `DITOX_DAEMON*`.
- Core (crates/ditox-core)
  - [ ] Ensure `Store` is safe for concurrent reader/writer usage.
  - [ ] Confirm image pipeline writes to `objects/aa/bb/<sha256>` with dedup.
- Daemon (ditox-clipd)
  - [ ] Extract shared logic to `clipd-lib` (Phase 2).
- Config & Docs
  - [ ] Add `[daemon]` settings to config; wire overrides.
  - [ ] Update README quick start and help text.
- Tests
  - [ ] Unit tests for dedup, sampling, and lock guard.
  - [ ] CLI black‑box tests for mode negotiation and doctor output.
  - [ ] Document manual smoke checklist for Wayland/X11.

## Documentation

- README: simplify Quick Start — “Run `ditox` and start copying; no separate daemon required.”
- Update help for new flags and TUI shortcuts.
- Keep a section for advanced users describing the external daemon.
 - Add a short “Troubleshooting capture” page linked from `doctor` output.

## Rollout

- Ship as a minor release v1.1.0 (behavioral change but backward compatible).
- Keep `ditox-clipd` crate published. After Phase 2, mark it as “optional for power users” in docs.
 - Feature flag guard: initial PR wires the code under an internal feature `managed-daemon` toggled on in release builds; can be flipped off quickly if regressions appear.
 - Announce behavior in release notes; call out how to disable via `--daemon=off` or config.

## Resolved Questions

1) Default `managed` scope — Decision: only for `ditox` (TUI). Other short‑lived commands remain unchanged.
2) Live toggle between `external` and `managed` — Decision: no for MVP. Support pause/resume only.
3) `--detach` support — Decision: out of MVP; evaluate post‑MVP (Phase 3) with explicit opt‑in and clear UX.
4) Shared daemon code location — Decision: new `clipd-lib` crate in Phase 2; `ditox-clipd` depends on it; CLI uses a subset for managed mode.

## Acceptance Criteria

- Running `ditox` on Linux starts the TUI and begins capturing clipboard changes without any external daemon.
- Closing the TUI stops capture (no background processes left running).
- If an external daemon is running, `ditox` does not start an embedded watcher and clearly indicates “External daemon active”.
- `doctor` shows correct managed daemon status and environment details.
- No regression to list/search/copy/image flows; tests pass; CI green.
 - Status bar correctly reflects `managed|external|off|paused` states and image on/off.
 - Lockfile is written and cleaned up as specified; stale lock handling verified.
 - Config layering precedence verified: flags > env > config > defaults.
