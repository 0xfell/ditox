# Repository Guidelines

## Project Structure & Module Organization
- Root: `Cargo.toml` (workspace, edition 2021), `.gitignore`, `PRD.md`.
- Core lib `crates/ditox-core/`: domain types, `Store` + SQLite backend, clipboard adapters, SQL migrations in `migrations/`, file‑backed blobs in `objects/aa/bb/<sha256>`; image groundwork via `ClipKind::Image` and `ImageMeta`.
- CLI `crates/ditox-cli/`: commands `add`, `list`, `search`, `copy`, `favorite`, `migrate`, `doctor`.
- Tests live under `crates/*/tests/`.

## Build, Test, and Development Commands
- Build: `cargo build` (or `cargo build -p ditox-cli`).
- Run CLI: `cargo run -p ditox-cli -- list`.
- Format/Lint: `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` (must be clean).
- Test: `cargo test --all`.
- Migrations (SQLite):
  - Status: `cargo run -p ditox-cli -- migrate --status`
  - Apply with backup: `cargo run -p ditox-cli -- migrate --backup`
  - Useful flags: `--db <path>`, `--auto-migrate=false` for managed rollouts.

## Coding Style & Naming Conventions
- Rustfmt defaults; keep Clippy at zero warnings.
- Modules/files `snake_case`; types/enums `CamelCase`; functions/fields `snake_case`.
- Errors: CLI uses `anyhow`; core uses `thiserror` + `Result`; avoid `unwrap()`/`expect()` in library code.
- Migrations: name `NNNN_description.sql` (4‑digit, ascending); one concern per file.

## Testing Guidelines
- Prefer black‑box CLI tests with `assert_cmd` + `predicates` in `crates/ditox-cli/tests`.
- For DB tests, create a temp SQLite file per test and clean up after.
- Add doc tests for examples in public APIs.

## Commit & Pull Request Guidelines
- Conventional commits, e.g., `feat(core): sqlite FTS search`, `fix(cli): handle empty stdin`, `docs(prd): add image plan`.
- PRs must describe intent, link issues, list commands run, and note DB migrations + CLI flags touched.
- Before opening: run fmt, clippy, tests, and `migrate --status`.

## Security & Configuration Tips
- Clipboard data is sensitive—avoid logging secrets; redact in examples/tests.
- Default DB path follows XDG; review retention/pruning before enabling sync (future work).
- v0.1.0 targets Linux (X11/Wayland via `arboard`); other OS adapters land later.
