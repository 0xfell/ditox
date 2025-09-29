# Repository Guidelines

## Project Structure & Module Organization
- Root workspace: `Cargo.toml` (edition 2021), `.gitignore`, `PRD.md`.
- Core library: `crates/ditox-core/` — domain types, storage (`Store`), SQLite backend, clipboard adapters, and embedded SQL migrations in `migrations/`.
- CLI: `crates/ditox-cli/` — user commands (`add`, `list`, `search`, `copy`, `favorite`, `migrate`, `doctor`).
- Image groundwork (v0.2): `ClipKind::Image`, `ImageMeta`, and file‐backed blob store (`objects/aa/bb/<sha256>`).

## Build, Test, and Development Commands
- Build (default features): `cargo build` (or `-p ditox-cli`).
- Run CLI: `cargo run -p ditox-cli -- list`.
- Format/lint (required before PR): `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`.
- Tests: `cargo test --all` (add tests under `crates/*/tests/`).
- Migrations (SQLite):
  - Status: `cargo run -p ditox-cli -- migrate --status`
  - Apply with backup: `cargo run -p ditox-cli -- migrate --backup`
- Useful flags: `--db <path>` to choose DB, `--auto-migrate=false` for managed rollouts.

## Coding Style & Naming Conventions
- Rustfmt defaults; keep Clippy clean (no warnings).
- Naming: modules `snake_case`, types/enums `CamelCase`, functions/fields `snake_case`.
- Errors: use `anyhow` in CLI and `thiserror`/`Result` in core; avoid `unwrap()`/`expect()` in library code.
- Migrations: `NNNN_description.sql` (4‑digit, ascending). One concern per file.

## Testing Guidelines
- Prefer black‑box tests in `crates/ditox-cli/tests` using `assert_cmd` and `predicates`.
- For DB tests, create a temp SQLite file per test and clean up.
- Add doc tests for examples in public APIs.

## Commit & Pull Request Guidelines
- Conventional commits: `feat(core): sqlite FTS search`, `fix(cli): handle empty stdin`, `docs(prd): add image plan`.
- PRs must: describe intent, link issues, list commands run, and include notes on DB migrations and CLI flags.
- Before opening: run format, clippy, tests, and `migrate --status`.

## Security & Configuration Tips
- Clipboard data is sensitive. Do not log secrets; prefer redaction in examples/tests.
- Default DB path follows XDG; review retention and pruning before enabling sync (future).
- v0.1.0 targets Linux (X11/Wayland via `arboard`); other OS adapters land later.
