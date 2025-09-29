Timers, Turso PRAGMA, Remote E2E, and Sync Doctor
=================================================

This note captures the changes, commands, and results for the requested work.

## Scope

- Add combined maintenance timer installer and README note.
- Guide/apply one‑time Turso PRAGMA bump and run full remote E2E; share results.
- Add a `sync doctor` subcommand for remote diagnostics.
- Remove ad‑hoc helper bins; keep small contrib scripts instead.

## Changes (by file)

- `README.md` — document both systemd timers and list `sync doctor` in the CLI reference.
- `crates/ditox-core/src/lib.rs`
  - Fix SQLite migration runner (remove stray async/libsql calls), keep using `PRAGMA user_version` locally.
  - Ensure image inserts set `is_image = 1` and migration 0003 backfills existing images.
- `crates/ditox-core/migrations/0003_images_path.sql` — contains backfill: `UPDATE clips SET is_image = 1 WHERE kind = 'image' AND is_image = 0;` (confirmed).
- `crates/ditox-cli/src/main.rs` — add `sync doctor` subcommand.
- Removed helper bins (replaced by `sync doctor`):
  - `crates/ditox-cli/src/bin/remote_pragma.rs`
  - `crates/ditox-cli/src/bin/remote_pragma_set.rs`
  - `crates/ditox-cli/src/bin/local_add.rs`
- New contrib scripts:
  - `contrib/scripts/turso_user_version.sh` — show/set `PRAGMA user_version` via Turso CLI.
  - `contrib/scripts/dev_local_add.sh` — convenience local row insert for dev.

## Build, Lint, Test

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Status: clippy clean; all tests pass.

## Timers

- Combined installer: `scripts/install_maintenance_timers.sh` (calls prune + sync installers).
- Individual installers: `scripts/install_prune_timer.sh`, `scripts/install_sync_timer.sh`.

## Remote E2E (libSQL/Turso)

Prereqs (env):

```bash
export TURSO_URL="libsql://<your-db>.turso.io"
export TURSO_AUTH_TOKEN="<token>"
```

Build and run:

```bash
cargo build -p ditox-cli --features libsql
cargo test -p ditox-core --features libsql,sqlite --test sync_libsql -- --nocapture
cargo run -p ditox-cli --features libsql -- sync status
cargo run -p ditox-cli --features libsql -- add "hello-remote-$(date +%s)"
cargo run -p ditox-cli --features libsql -- sync run
cargo run -p ditox-cli --features libsql -- sync status
```

Observed (example) outputs:

```
sync: pushed=0 pulled=1
last_push_updated_at=Some(0)
last_pull_updated_at=Some(1759134128)
pending_local=1
local_text=1
remote_ok=Some(true)

sync: pushed=1 pulled=0
pending_local=0
```

## `sync doctor`

Diagnostics command (feature‑gated with `libsql`):

```bash
cargo run -p ditox-cli --features libsql -- sync doctor
```

Example output:

```
remote_ok=true
remote_user_version=0
has_clips=true
clips_columns=id, kind, text, created_at, is_favorite, deleted_at, is_image, image_path, updated_at, lamport, device_id
clips_count=4
```

## PRAGMA `user_version` (remote)

- libSQL/Turso (Hrana) blocks setting `PRAGMA user_version` programmatically; leaving it at `0` is safe because migrations are idempotent and schema is validated by `sync doctor`.
- If you want the flag updated for bookkeeping, use the Turso CLI (or the helper script):

```bash
contrib/scripts/turso_user_version.sh <db-name>               # show
contrib/scripts/turso_user_version.sh <db-name> set 4         # set, then show
```

## Notes

- Image rows: the SQLite path sets `is_image = 1` for new images; migration 0003 backfills legacy image rows.
- Remote backend intentionally excludes images from sync.

## Next

- Optional: add a short README subsection illustrating typical `sync doctor` output.
- Optional: add Makefile targets for timers and doctor.

