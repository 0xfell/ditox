# Task: Honour `Config.storage.data_dir`

> **Status:** completed
> **Priority:** high
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

`Config.storage.data_dir` is parsed from `config.toml` but ignored —
`Database::get_data_dir` always resolves via `directories::ProjectDirs`.
Documented as a bug in `docs/notes/ditto-comparison.md` baseline §4.

Fix it so users can keep their database and image store in any directory
they choose (e.g. an encrypted volume, a synced folder, a portable
USB).

## Requirements

- [ ] `ditox-core/src/db.rs::Database::get_data_dir` accepts an optional
      override and uses it when `Config.storage.data_dir` is `Some`.
- [ ] `ditox-core/src/db.rs::Database::get_db_path` accepts the same
      override.
- [ ] All call sites pass the config-resolved `data_dir`. Audit the
      following:
      - `ditox-core::Database::open()` — current entry point.
      - `ditox-tui` watcher daemon launch.
      - `ditox-tui` CLI `repair`, `clear`, `status`, etc.
      - `ditox-gui` boot in `app.rs::create_app`.
- [ ] Tilde expansion (`~/path` → `$HOME/path`) supported.
- [ ] Environment variable expansion (`$XDG_DATA_HOME/foo`) supported.
- [ ] Path is created on demand if missing (`fs::create_dir_all`); error
      bubble-up if creation fails.
- [ ] Migration safety: if `data_dir` was previously unset and a DB
      exists at the default path, do **not** silently start a new empty
      DB — surface a clear error or refuse to switch.

## Implementation Notes

Approach:

1. Introduce `Database::resolve_paths(config: &Config) -> Paths` where
   `Paths { db_path, images_dir }`.
2. Replace `get_db_path()` and `get_data_dir()` with the resolver
   everywhere.
3. Add a unit test: temp `Config` with `storage.data_dir = Some(<temp>)`,
   open a DB, insert a row, close, reopen, assert row present.
4. Add a unit test for tilde + env expansion.
5. Update `docs/ROADMAP.md` "Quick Reference" file locations to
   mention the override.

The migration-safety check: if `Config.storage.data_dir` is `Some(P)`
and `P/ditox.db` does not exist, but the legacy default path *does*
exist, log a warning. Don't auto-migrate; that's a user-driven
operation.

## Testing

- Unit tests in `ditox-core/tests/data_dir_override.rs`.
- Manual smoke: `DITOX_CONFIG=/tmp/foo.toml ditox status` with a config
  pointing data_dir to `/tmp/ditox-alt/`.

## Work Log

### 2026-04-26
- Task file created.
- Added `expand_path()` to `ditox-core/src/config.rs` supporting `~`, `~/...`, `$VAR`, and `${VAR}` (unknown vars left literal, no per-user `~bob` expansion).
- Added `StorageConfig::resolved_data_dir()` and `Config::apply_storage_override()`.
- Added `Config::legacy_db_exists_outside()` helper to warn when an override starts a fresh history while a default DB exists.
- Added process-wide `DATA_DIR_OVERRIDE: RwLock<Option<PathBuf>>` in `ditox-core/src/db.rs` with `set_data_dir_override()` and `data_dir_override()` accessors.
- Refactored `Database::get_db_path()` to be public and route through `get_data_dir()`.
- Refactored `Database::get_data_dir()` to consult the override first, fall back to `ProjectDirs`.
- All path resolvers (`get_data_dir`, `get_images_dir`, `image_path`, `store_image_blob`) automatically pick up the override.
- Wired `Config::apply_storage_override()` into both `ditox-tui/src/main.rs` and `ditox-gui/src/main.rs` startup paths with a soft-warn when a legacy DB exists.
- Wrote `ditox-core/tests/data_dir_override.rs` with 7 tests covering: tilde expansion, `$VAR` expansion, `${VAR}` expansion, unknown-var literal handling, override redirects DB, override creates missing dir, TOML parsing of `[storage].data_dir = "~/path"`, legacy-DB-detection helper.
- All 40 workspace tests pass (was 33). Build green.
