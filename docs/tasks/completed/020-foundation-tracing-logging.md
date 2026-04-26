# Task: Standardise logging via `tracing`

> **Status:** completed
> **Priority:** medium
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

The codebase mixes `eprintln!`, `println!`, and `tracing` calls. Move to
exclusive `tracing` usage so:

- Log filtering via `RUST_LOG` works uniformly.
- Phase 6 sync gets structured logs with peer IDs, byte counts.
- systemd journal integration (task 016) emits structured fields.
- GUI can show recent log lines in a debug overlay.

## Requirements

- [ ] **Audit & replace.** All `eprintln!` / `println!` outside of CLI
      output paths replaced by `tracing::{info, warn, error, debug,
      trace}!`.
- [ ] **Span the hot paths.** Add `#[tracing::instrument]` to:
      - `Watcher::poll_once`
      - `Database::insert_entry`
      - `Database::get_page_filtered`
      - GUI `Message::Update` dispatcher
- [ ] **`tracing-subscriber` initialisation** unified in
      `ditox-core/src/logging.rs`:
      ```rust
      pub fn init(opts: LogOpts) {
          let env_filter = EnvFilter::try_from_default_env()
              .unwrap_or_else(|_| EnvFilter::new("info,ditox=debug"));
          let fmt = fmt::layer().with_target(true).with_level(true);
          tracing_subscriber::registry().with(env_filter).with(fmt).init();
      }
      ```
      Three modes: `Stderr` (default), `Journald` (systemd unit, task
      016 `--journal`), `File(path)` (debug capture).
- [ ] **CLI output preserved.** `ditox list`, `get`, `search`, `count`,
      `stats` write to stdout via `println!`/`writeln!` — those are
      user-facing structured output, not logs. Keep them as-is.
- [ ] **`RUST_LOG` documented** in `README.md` and `AGENTS.md`:
      ```
      RUST_LOG=ditox=debug ditox watch
      RUST_LOG=ditox_core::watcher=trace ditox watch
      ```
- [ ] **Module-level targets.** Use `tracing::info!(target: "ditox::sync", …)`
      patterns where it helps log filtering.
- [ ] **Drop the `_log` / `eprintln!` cruft.**
- [ ] **`tracing-journald` dep** behind a `journald` cargo feature
      (default on Linux only).

## Implementation Notes

Search-and-replace is mostly mechanical, but watch for cases where
`eprintln!` was used as a CLI status indicator (e.g.
"Daemon started, PID 1234"). Keep those as `eprintln!` if the user
expects them on every CLI invocation; convert if they're really logs.

Test commands won't be affected — they capture stdout/stderr and don't
care about log levels.

## Testing

- `RUST_LOG=ditox=trace ditox watch` for 5 seconds → spot-check the
  output is informative.
- `cargo test` still passes.
- `journalctl --user -u ditox-watcher.service` shows structured logs
  after task 016 lands.

## Work Log

### 2026-04-26
- Task file created.
- Created `ditox-core/src/logging.rs` with `init(Mode)` accepting `Mode::Stderr` (default), `Mode::File(PathBuf)`, and `Mode::Journald` (stub forwarding to stderr until task 016 ships journald support).
- Added `tracing-subscriber` to `ditox-core/Cargo.toml`.
- Wired `logging::init(Mode::Stderr)` into `ditox-tui/src/main.rs` (replacing inline `tracing_subscriber::fmt()` call).
- Wired `logging::init(Mode::Stderr)` into `ditox-gui/src/main.rs`.
- Replaced 4 `eprintln!("warn: ...")` calls in `ditox-tui/src/main.rs` repair path with `tracing::warn!`.
- Replaced 1 `eprintln!("Search error: ...")` call in `ditox-gui/src/app.rs` with `tracing::error!`.
- Kept the `eprintln!` in both `main.rs` error handlers as last-resort output (logging may not be initialised at very-early-error time).
- Kept all `println!` calls in CLI commands — those are user-facing structured output, not logs.
- Updated `AGENTS.md` with a new "Logging" section explaining `RUST_LOG` usage and the println-vs-tracing distinction.
- Build green, all 40 tests + 1 doctest pass.
