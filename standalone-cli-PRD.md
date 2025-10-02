# Ditox Standalone CLI (Embedded Daemon) — PRD

- Doc: standalone-cli-PRD.md
- Owner: 0xfell
- Status: Draft (as of 2025-10-02)
- Target release: v1.1.0 (scope can be split if needed)
- Platforms: Linux primary (X11/Wayland via `arboard`); other OS builds run without capture

## Summary

Ship a “standalone” Ditox experience where the `ditox` CLI/TUI runs with an embedded clipboard watcher (daemon) while the TUI is open. Users install and run only `ditox-cli`; it will capture clipboard changes in the background for the lifetime of the TUI session. The separate `ditox-clipd` binary remains available for long‑running, headless use, but regular users no longer need to manage a daemon explicitly.

Invocation policy: when `ditox` is invoked with no subcommand or arguments, launch the TUI (picker). Subcommands like `list`, `search`, etc., are unaffected. Keep `ditox pick` and add `ditox tui` as a synonym for clarity.

## Goals

- Zero‑setup experience: `cargo install ditox-cli` → `ditox` captures clipboard while the UI is open.
- Embedded daemon (“managed daemon”) lifecycle is tied to the TUI: start at TUI launch; stop when TUI exits.
- No data loss vs external daemon: same capture fidelity, same store, image support.
- Sensible defaults on Linux (X11/Wayland) with `arboard`.
- Safe coexistence with the external daemon: if `ditox-clipd` is already running, do not double‑capture.
- Keep power‑user paths intact (systemd timers, headless daemon, remote backends).
- Zero migrations required: no DB schema changes for MVP.

## Non‑Goals (Initial)

- Replacing the external daemon for long‑running/background use across reboots.
- Implementing Windows/macOS native clipboard watchers (current adapters remain no‑op or limited).
- Cross‑process IPC beyond what’s required to detect an existing daemon.

## Terminology

- Managed/embedded daemon: in‑process clipboard watcher owned by the `ditox` TUI session.
- External daemon: the separate `ditox-clipd` process (systemd/headless).
- Off/view‑only: no capture; the TUI acts as a read‑only viewer.

## User Stories / UX

- As a first‑time user, I run `ditox` and immediately see history populate as I copy text/images in other apps. When I quit the TUI, background capture stops.
- As an existing user with `ditox-clipd` running (e.g., via systemd), launching `ditox` uses the existing history; no duplicate captures or lock errors occur.
- As a privacy‑conscious user, I can start the TUI with capture disabled (`--daemon=off`) or pause/resume capture from inside the TUI.
- As a power user, I can still run `ditox-clipd` headless for continuous capture and use the TUI only as a viewer.
- As a script author, `ditox` with a subcommand behaves exactly as before (no surprise TUI launch).

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
        - `--no-alt-screen` (quality‑of‑life; renders in place, helps tmux/screen).
    - TUI hotkeys (visible in help):
        - `p`: pause/resume capture.
        - `D`: toggle image capture on/off for session.
        - `?`: help overlay; shows current capture mode and toggles.

- `ditox doctor`
    - Reports “Managed: active|paused|off”, backend (X11/Wayland), sample interval, lock owner (pid), capture mode (`managed|external|off`), DB path, and conflict detection (external daemon present: yes/no).

- `ditox daemon …` (existing external daemon remains; docs point casual users to just `ditox`).

## Architecture & Design

### High‑Level

- Embed a clipboard watcher (“managed daemon”) into `ditox-cli` when in TUI mode.
- Watcher runs in the same process as the TUI and writes to the same SQLite store via `ditox-core::Store` with identical semantics to `ditox-clipd`.
- On startup:
    1. Detect external daemon (pidfile/lockfile probe).
    2. If found and `--daemon=managed`, switch to `external` mode silently (no embedded watcher) to avoid double capture.
    3. If none and `--daemon=managed`, spawn a background task (Tokio) that:
        - polls clipboard at configured interval;
        - deduplicates consecutive identical content (hash, timestamp);
        - persists entries via the store; supports images behind feature gate.
    4. When TUI exits, task shuts down gracefully.

Assumptions & constraints
- Linux target first; non‑Linux builds compile but watcher is a no‑op for MVP.
- Polling is acceptable for MVP; event‑driven hooks may be explored later.
- No DB schema changes; reuse `Store` and image blob layout.

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

Lifecycle state machine (managed mode)
- States: `inactive` → `starting` → `active` ↔ `paused` → `stopping` → `inactive`.
- Triggers: TUI enter/exit; pause/resume hotkey; external daemon detected (transition to `inactive` with status `external`).

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

Config precedence
- Flags > env vars > config file > built‑in defaults.
- Environment (new): `DITOX_DAEMON`, `DITOX_DAEMON_SAMPLE`, `DITOX_DAEMON_IMAGES`.

### Compatibility & Coexistence

- If `ditox-clipd` runs (systemd, user session), `ditox` detects it via pid/lock and does not spawn an embedded watcher.
- If both start simultaneously (race), file lock resolves to a single active capturer.
- The `ditox-clipd` crate remains published for headless use; we may later convert its internals into a library crate to share code (see Migration).

Edge cases
- DB path not writable → surface error in footer; allow TUI read‑only; `doctor` explains remediation.
- Lock path not writable (e.g., custom XDG) → disable managed mode with a warning; continue in `external|off`.
- Wayland/X11 mismatch or missing helpers → show non‑fatal warning; capture may be limited.

## Migration / Refactor Plan

Phase 0 (prep)

- Extract reusable watcher logic from `ditox-clipd` into a new internal module (`crates/ditox-cli/src/managed_daemon.rs`) by copy‑move first, then deduplicate with `ditox-clipd` in a follow‑up.

Phase 1 (MVP)

- Add managed daemon to CLI TUI with defaults above.
- Add locking, sampling, image toggle, and pause/resume in TUI.
- Update `doctor` and docs.
    - Add explicit “managed/external/off” section; print lock path and owner PID.

Phase 2 (Consolidation)

- Refactor `ditox-clipd` into a small bin that calls a new `clipd-lib` (shared between CLI managed daemon and external daemon) to eliminate duplication.

Phase 3 (Optional)

- Provide `--detach` to keep the managed daemon running after TUI exit (off by default; not in MVP scope).

## Security & Privacy

- No secret logging; redact clip contents in logs and tests.
- Keep “incognito” toggle (do not persist) as a future enhancement.
- Respect image data handling and storage budget as today.
 - Telemetry: none. No network calls unless remote sync is explicitly configured (feature‑gated).

## Performance Targets

- CPU: < 1% on idle with 200ms sampling on typical Linux desktop.
- Memory: < 50 MiB for TUI + watcher combined in steady state.
- DB I/O: avoid excessive writes via dedup + backoff for bursty changes.
 - Startup: TUI open ≤ 150 ms over baseline on a typical dev laptop.

## Failure Modes & Handling

- Clipboard backend unavailable → surface non‑fatal warning in TUI status bar and `doctor`.
- DB locked/busy → retry with exponential backoff; drop frames rather than freeze UI.
- External daemon detected → do not start managed watcher; show status “External daemon active”.
 - Lock contention with unknown PID → if PID not alive, remove stale lock; else disable managed mode and show guidance.
 - Unhandled runtime error in watcher → stop watcher, mark status `error`, leave TUI usable.

## Testing

- Unit tests for watcher dedup, sampling, and store writes (using temp SQLite).
- CLI tests (assert_cmd) for mode negotiation and lock behavior.
- Doctor output assertions.
- Manual smoke on X11 and Wayland (GitHub Actions can’t capture clipboard; rely on local CI matrix guidance).
 - Non‑Linux builds: ensure TUI launches, status shows `off|external`, and no watcher work happens.

QA checklist (CI + local)
- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`.
- Linux Wayland/X11 manual: pause/resume, images on/off, dedupe, and exit cleanup.
- Run with external `ditox-clipd` active: verify no double capture.
- Run under `NO_COLOR`, small terminal width, and ASCII mode: UI remains legible.

## Documentation

- README: simplify Quick Start — “Run `ditox` and start copying; no separate daemon required.”
- Update help for new flags and TUI shortcuts.
- Keep a section for advanced users describing the external daemon.
 - Add a “Troubleshooting managed mode” section (locks, DB path, Wayland helpers).

## Rollout

- Ship as a minor release v1.1.0 (behavioral change but backward compatible).
- Keep `ditox-clipd` crate published. After Phase 2, mark it as “optional for power users” in docs.

Backout plan
- If issues arise, default `--daemon` to `external` via a minor patch release; keep flags available for opt‑in managed mode.
- Revert `ditox` (no‑args) to print help instead of launching the TUI, if needed.

## Open Questions

1. Should `managed` be the default for `ditox tui` only, or for any `ditox` command that keeps the process running? (Proposal: TUI only.)
2. Do we want a keybind to toggle between `external` and `managed` live? (Likely no for MVP.)
3. Should we support `--detach` to keep capture after TUI exit? (Out of MVP, possibly Phase 3.)
4. Where to host the shared daemon code long‑term: inside `ditox-core` or a new `clipd-lib` crate? (Proposal: `clipd-lib`.)
5. Should `ditox` (no args) launching the TUI be gated behind a config flag initially to reduce surprises? (Default ON in v1.1.0 unless feedback suggests otherwise.)

## Acceptance Criteria

- Running `ditox` on Linux starts the TUI and begins capturing clipboard changes without any external daemon.
- Closing the TUI stops capture (no background processes left running).
- If an external daemon is running, `ditox` does not start an embedded watcher and clearly indicates “External daemon active”.
- `doctor` shows correct managed daemon status and environment details.
- No regression to list/search/copy/image flows; tests pass; CI green.
 - `ditox` with subcommands behaves exactly as before; no interactive mode is entered when a subcommand is present.
 - Lock file is cleaned on exit and stale locks are recovered on next start.

# Ditox Standalone TUI and Managed Capture — Implementation Notes

This document captures everything we added for the standalone TUI picker, the embedded managed capture, and the customization system so it can be re‑implemented or ported cleanly.

## Goals

- Fast, single‑binary interactive picker (no separate server needed).
- Optional embedded capture daemon with pause/images control and single‑instance locking.
- Customizable TUI (themes, glyphs, layouts; ASCII/no‑color fallbacks).
- Debounced auto‑refresh, selection preservation, and “New” badges.
- Compatible with existing local SQLite and optional remote libsql/Turso.

## High‑Level Architecture

- Entry: `crates/ditox-cli/src/main.rs`
    - Parses CLI flags (`Pick` command) and prepares a “lazy” store.
    - Optionally starts an embedded capture daemon (managed mode) if no external clipd is detected.
    - Passes capture status + settings to the picker (`picker::run_picker_default`).
- TUI: `crates/ditox-cli/src/picker.rs`
    - Draws frames with ratatui (list, title, footer, help overlay).
    - Reads theme, glyphs, layout packs at startup.
    - Debounced refresh + search + selection restore + “New” badges.
- Customization helpers: `crates/ditox-cli/src/theme.rs`
    - Theme loader (`load_tui_theme`) with color caps and border types.
    - Glyph packs (`load_glyphs`) and layout packs (`load_layout`).
- Embedded capture: `crates/ditox-cli/src/managed_daemon.rs`
    - Managed watcher thread, lockfile handling, pause/images toggles.
- Optional tray (feature `tray`): `crates/ditox-cli/src/tray.rs`
    - Pause/Images/QUIT menu wired to managed control.

## Modules and Key Types

- `picker.rs`
    - `run_picker_default`, `run_picker_with` (TUI core loop)
    - `CaptureMode` (Managed | External | Off)
    - `CaptureStatus { mode, managed: Option<ManagedControl> }`
    - `EventSource`/`RealEventSource` for easy testing
    - Helper: `selected_id(...)` to preserve cursor selection
- `managed_daemon.rs`
    - `DaemonConfig { sample: Duration, images: bool, image_cap_bytes: Option<usize> }`
    - `ManagedHandle` (lifetime + lock cleanup) and `ManagedControl` (pause/images/sample getters + toggles)
    - `try_create_lock()` and `detect_external_clipd()`
- `theme.rs`
    - `Caps { color_depth, unicode, no_color }` (`detect_caps`)
    - `TuiTheme { ... colors ... border_type }` and `load_tui_theme()`
    - `Glyphs` + `load_glyphs()` (ASCII/Unicode/built‑ins + user packs)
    - `LayoutPack` + `load_layout()` (border styles, line heights, templates)

## CLI Surface (Pick)

File: `crates/ditox-cli/src/main.rs`

- Managed capture and refresh
    - `--daemon {managed|external|off}` (default: managed)
    - `--daemon-sample <duration>` (e.g., 200ms, 1s)
    - `--daemon-images {true|false}`
    - `--no-daemon` (bypass daemon IPC)
    - `--refresh-ms <u64>` (debounced auto‑refresh interval, overrides config)
    - `--no-alt-screen` (draw in main screen buffer)
- Backend and filters
    - `--remote` (force libsql/Turso; disables daemon)
    - `--tag <string>` (filter by tag)
    - `--favorites`, `--images`
- TUI customization
    - `--theme <name|path>`; `--ascii`; `--color {auto|always|never}`
    - Discovery/preview: `--themes`, `--glyphsets`, `--layouts`, `--preview <theme>`, `--dump-caps`
    - `--glyphs <name|path>`, `--layout <name|path>`

Other commands: `doctor` (prints capture status), `thumbs`, `config`, etc., were preserved.

## Settings (TOML)

File: `crates/ditox-cli/src/config.rs`

- Table `[tui]`
    - `page_size: Option<usize>` — default 10
    - `auto_apply_tag_ms: Option<u64>` — auto apply `#tag` after idle typing
    - `absolute_times: Option<bool>` — show absolute instead of relative
    - `refresh_ms: Option<u64>` — debounced auto‑refresh interval (default 1500)
    - `sound_on_new: Option<bool>` — reserved future toggle (default false)
    - `theme: Option<String>` — name or path (e.g., "dark", or `~/.config/ditox/themes/custom.toml`)
    - `color: Option<String>` — `auto|always|never`
    - `box_chars: Option<String>` — `unicode|ascii` (ASCII disables borders)
    - `alt_screen: Option<bool>` — use alternate screen buffer
    - `live_reload: Option<bool>` — reserved (no‑op placeholder)
    - `date_format: Option<String>` — date tokens for absolute mode
    - `auto_recent_days: Option<u32>` — threshold days for auto → absolute
    - `glyphs: Option<String>` — name or file path
    - `layout: Option<String>` — name or file path

- Table `[storage]` remains unchanged (`LocalSqlite`/`Turso`).

## Environment Variables

These supplement CLI/config for fast overrides:

- `DITOX_TUI_THEME` — theme name or path
- `DITOX_TUI_COLOR` — `auto|always|never`
- `DITOX_TUI_ASCII` — `1` to force ASCII draw
- `DITOX_TUI_ALT_SCREEN` — `1|0` to enable/disable alternate screen
- `DITOX_TUI_DATE_FMT` — date format string
- `DITOX_TUI_AUTO_DAYS` — integer recent threshold
- `DITOX_TUI_GLYPHS` — glyph pack name or path
- `DITOX_TUI_LAYOUT` — layout pack name or path

## Rendering & Theming

- Color/no‑color and Unicode/ASCII detection via `detect_caps()`.
- Theme (`TuiTheme`) provides:
    - Title, status, badge, muted, border colors; search match colors; `border_type`.
- Glyphs (`Glyphs`): favorite on/off, selected/unselected marks, kind icons, `enter_label`.
- Layout (`LayoutPack`):
    - Where to draw search bar (top/bottom), item line height (1 or 2),
    - Optional templating for list title/footer/items/meta/help,
    - Per‑section borders and a compact pager ("{page}/{page_count}").

## TUI Behavior

- Modes: Normal and Query (`/` toggles). Query text applied live.
- Tag input: when Query text starts with `#`, auto‑apply tag after `auto_apply_tag_ms` idle.
- Paging: dynamic rows per page based on viewport height and `list_line_height`.
- Selection: restored via stable ID on refresh/filter changes (keeps cursor where possible).
- “New” badges: new IDs get a badge for ~2.5s after appearance.
- Footer: shows shortcuts (customizable), optional capture status (
  `managed (N ms, images:on|off, paused|active)` | `external` | `off`).
- Help overlay: centered modal with multi‑column hotkeys; respects theme/layout/borders.

## Key Bindings (default)

- Navigation: `↑/k`, `↓/j`, `→/l/PgDn`, `←/h/PgUp`, `Home/g`, `End/G`
- Filter/Search: `/` toggle, `t` apply `#tag`, `r` refresh
- Favorites/Images: `Tab` favorites toggle, `i` images view toggle
- Managed capture (when available): `D` images capture toggle, `Ctrl+P` pause/resume
- Selection & actions: `s` select, `S` clear selection, `Enter` copy, `x` delete, `p` fav/unfav
- Help/Exit: `?` help, `q` or `Esc` quit

## Embedded Managed Capture

File: `crates/ditox-cli/src/managed_daemon.rs`

- Lockfile: `${state_dir}/managed-daemon.lock`
    - Linux: stale lock detection via `/proc/<pid>`; removes stale and continues.
    - Non‑Linux (macOS/Windows): fail fast if a lock exists (prevents double capturers).
- Sampling loop:
    - Poll text; trim trailing `\n`; dedupe via recent list; `Store::add` or `touch_last_used`.
    - Optional image capture (bytes cap check) via `add_image_rgba`.
- Controls exposed through `ManagedControl`:
    - `toggle_pause()`, `is_paused()`, `toggle_images()`, `images_on()`, `sample()`.
- External detection: `config/clipd.json` with TCP connect confirmation.
 - Observability: emit `trace!` with timings (no content); TUI footer shows succinct status.

## Optional Tray (feature `tray`)

File: `crates/ditox-cli/src/tray.rs`

- Menu entries: Pause/Resume, Images on/off, Quit.
- Hooks into `ManagedControl` if managed capture is active.
- Small icon builder, optional PNG fallback.
 - Not part of the MVP default build; behind feature flag to keep dependencies out of CI by default.

## Backend Policy (Picker)

- If `--remote`: always use libsql/Turso, bypass daemon.
- Otherwise: local SQLite for picker and managed capture, with DB path:
    - `[storage.local_sqlite].db_path` or default `${config_dir}/db/ditox.db`.
- Daemon start policy:
    - `--daemon` + no external clipd detected → start embedded daemon.
    - Environment override: `DITOX_DAEMON={managed|external|off}`.
 - No args policy: `ditox` with no subcommand launches the TUI; any provided subcommand runs headless as today.

## Diagnostics

- Doctor (`ditox config` / `ditox doctor`): prints capture status and clipboard checks.
- Errors from TUI copy operations are deferred and printed after exit.
 - `ditox doctor` includes the lock path, PID, backend, and notes if status is managed/external/off.

## CI/Policy Notes

- `deny.toml`: allow `"Apache-2.0 WITH LLVM-exception"`.
- Advisories ignored (tray‑only transitive GTK3/GLib):
    - RUSTSEC‑2024‑0412/0413/0415/0416/0418/0419/0420, and glib VariantStrIter unsoundness (RUSTSEC‑2024‑0429),
    - `proc-macro-error` unmaintained (RUSTSEC‑2024‑0370).
- These are ignored because tray is optional and only used for desktop UI; CLI/core paths are unaffected.
 - Default CI build excludes `tray`; enable it in a dedicated job if/when we wire the tray.

## Re‑Implementation Checklist

1. CLI flags (Pick)
    - Add to `crates/ditox-cli/src/main.rs` under `Commands::Pick` the flags listed above and the env‑wiring for theme/color/ascii/layout/glyphs.
2. Managed daemon
    - Restore `crates/ditox-cli/src/managed_daemon.rs` with lockfile policy and controls.
    - In `main.rs`, compute effective daemon mode and optionally call `start_managed(...)`.
3. TUI entry
    - Call `theme::load_tui_theme()`, `theme::load_glyphs()`, `theme::load_layout()` at the top of `run_picker_with`.
    - Respect caps: `detect_caps()` → ascii/no‑color fallbacks; apply borders via `border_type` where present.
4. Picker loop
    - Add debounced refresh (`refresh_ms`) and `last_key_ts` typing debounce.
    - Preserve selection via ID (`pending_restore_id` + `selected_id()` helper).
    - Track `last_ids` + `new_until` to render “New” badges.
5. Rendering
    - Use theme colors for: title, list highlight, borders, footer, help overlay, match highlights.
    - If `layout.*_template` exists, expand placeholders; else render defaults.
    - Optional pager at bottom‑right of list area using `pager_template`.
    - Respect ASCII/no‑color fallbacks; avoid wide glyphs when `--ascii`.
6. Keybindings
    - Implement bindings listed above; ensure help overlay and footer reflect them.
7. Backend policy
    - Build lazy store with remote/local rules; bypass daemon when remote.
8. Tray (optional)
    - Guard under `#[cfg(feature = "tray")]` and keep GTK3 dev‑deps out of default build.
9. Doctor/config outputs
    - Show capture status; surface errors non‑fatally.

## Test Matrix (manual)

- Linux tty: ASCII and Unicode; color `auto|never|always`.
- With and without daemon; with external clipd running.
- `--remote` (libsql) disables daemon and still renders badges.
- Themes: built‑in `dark` and `high-contrast`; user theme in `~/.config/ditox/themes/foo.toml`.
- Glyph packs: `ascii` and `unicode`.
- Layout packs: `default` plus a custom pack testing templates/borders/pager/help/footer.

## Non‑Linux Locking (Important)

- Managed lock refusal on macOS/Windows is deliberate to avoid multiple capture threads touching the same SQLite DB.
- If you want parity with Linux, add a platform‑specific liveness probe (e.g., `kill(pid, 0)` via nix or `OpenProcess` on Windows) and update `is_pid_alive` accordingly.
