# Repository Guidelines

## Project Structure & Module Organization

- Root: `Cargo.toml` (workspace, edition 2021), `.gitignore`, `PRD.md`.
- Core lib `crates/ditox-core/`: domain types; `Store` + SQLite backend; clipboard adapters; SQL migrations in `migrations/`; file‑backed blobs under `objects/aa/bb/<sha256>`; image groundwork via `ClipKind::Image` and `ImageMeta`.
- CLI `crates/ditox-cli/`: commands `add`, `list`, `search`, `copy`, `favorite`, `migrate`, `doctor`.
- Tests live under `crates/*/tests/`.

## Build, Test, and Development Commands

- Build all: `cargo build` — workspace build.
- CLI only: `cargo build -p ditox-cli`.
- Run CLI: `cargo run -p ditox-cli -- list` (example command).
- Format: `cargo fmt --all`.
- Lint: `cargo clippy --all-targets -- -D warnings` (do not use `cargo clippy -q -p ditox-cli -- -D`).
- Test: `cargo test --all`.
- Migrations (SQLite):
    - Status: `cargo run -p ditox-cli -- migrate --status`
    - Apply with backup: `cargo run -p ditox-cli -- migrate --backup`
    - Useful flags: `--db <path>`, `--auto-migrate=false`

## Coding Style & Naming Conventions

- Use `rustfmt` defaults; keep Clippy at zero warnings.
- Names: modules/files `snake_case`; types/enums `CamelCase`; functions/fields `snake_case`.
- Errors: CLI uses `anyhow`; core uses `thiserror` + `Result`; avoid `unwrap()`/`expect()` in library code.
- Migrations: `NNNN_description.sql` (4‑digit, ascending); one concern per file.

## Testing Guidelines

- Prefer black‑box CLI tests with `assert_cmd` + `predicates` in `crates/ditox-cli/tests/`.
- For DB tests, create a temp SQLite file per test and clean up.
- Add doc tests for examples in public APIs.
- Run all tests: `cargo test --all`.

## Commit & Pull Request Guidelines

- Conventional commits (examples): `feat(core): sqlite FTS search`, `fix(cli): handle empty stdin`.
- PRs: describe intent, link issues, list commands run, and note DB migrations + CLI flags touched; include relevant output/screenshots for UX‑affecting changes.

## Security & Configuration Tips

- Clipboard data is sensitive—avoid logging secrets; redact in examples/tests.
- Default DB path follows XDG; review retention/pruning before enabling sync (future work).
- v0.1.0 targets Linux (X11/Wayland via `arboard`); other OS adapters land later.

## Agent‑Specific Notes

- Scope: this file applies to the entire repository for AI assistants.
- Keep changes minimal and focused; update docs when touching migrations or CLI flags.

## Git Worktrees

- Location: use `./.worktree/` for all task‑specific checkouts. Ensure it exists with `mkdir -p ./.worktree` before creating any worktree.
- Branch naming: derive from the task type and a short, kebab‑case slug of the request. Use Conventional Commit types for the prefix:
    - Features: `feat/<slug>` (default if unspecified)
    - Fixes: `fix/<slug>`
    - Docs: `docs/<slug>`
    - Refactors/Chores/Perf/Tests: `refactor/<slug>` or `chore/<slug>` or `perf/<slug>` or `test/<slug>` as appropriate
- Worktree directory name: mirror the branch name but replace `/` with `__` to avoid nested folders; e.g., branch `feat/image-deduping` → dir `./.worktree/feat__image-deduping`.
- Create a new worktree from `origin/master`:
    - `git fetch origin`
    - `BRANCH="feat/<slug>"; DIR=".worktree/${BRANCH//\//__}"`
    - `git worktree add -b "$BRANCH" "$DIR" origin/master`
- Use the worktree:
    - `cd "$DIR"`
    - Implement changes; run builds/tests here, not in the master checkout.
- Push and open PR:
    - `git push -u origin "$BRANCH"`
    - Open a PR from `$BRANCH` into `master` (follow Conventional Commits in commit messages).
- Keep in sync with `master` while working:
    - `git fetch origin && git rebase origin/master` (inside the worktree). Avoid merge commits in feature branches.
- Clean up after merge:
    - `cd -` (leave the worktree dir)
    - `git worktree remove "$DIR"` (use `--force` only if necessary)
    - `git branch -d "$BRANCH"`
    - `git push origin --delete "$BRANCH"` (if the remote branch was pushed)
    - `git worktree prune`
- Rules of thumb:
    - One worktree per user‑visible task/feature; name slugs short and specific (e.g., `qr-decode`, `fts-search`, `migrate-status-ui`).
    - Do not commit directly to `master`; always work in a worktree branch and open a PR.
    - Never place other Git repos under `./.worktree/`; it is exclusively for worktrees.
    - If you must switch tasks, create a new branch/worktree rather than reusing an existing one.

Note: Avoid `cargo clippy -q -p ditox-cli -- -D` (can hang). Prefer `cargo clippy --all-targets -- -D warnings`.
