# Task: Content-Addressed Image Storage + Refcount Prune

> **Status:** completed
> **Priority:** high
> **Created:** 2026-04-24
> **Completed:** 2026-04-25

## Description

The image store on disk (`~/.local/share/ditox/images/`) grew without bound.
Four independent bugs caused leaks. This task rewrites the storage layer to
be content-addressed with a refcount-backed prune queue and adds a
`ditox repair` subcommand for reconciliation.

## Bugs fixed

1. **Write-before-dedup.** `Clipboard::get_image` unconditionally wrote a new
   timestamped PNG, THEN the watcher checked `exists_by_hash` and skipped
   the insert on a dup. The file was orphaned.
   - Repro (pre-fix): 3× same image copy → 3 files, 1 row.
2. **Deletes never unlinked files.** `Database::delete`, `clear_all`,
   `cleanup_old`, `App::delete_selected`, `App::delete_selected_multi`,
   `App::clear_all`, and the GUI `ConfirmDeleteEntry` all only removed the
   row. The `ditox clear` CLI did `remove_dir_all` which masked it for that
   one path but clobbered quarantined/pinned files too.
3. **`cleanup_old` leaked at scale.** With `max_entries=3` and 6 captures:
   2 rows but 9 files on disk.
4. **Watcher restart re-wrote current image.** `initialize_hash` only
   primed from text. On restart with an image still on the clipboard, the
   next poll re-captured it and wrote a duplicate file.

## Design

- **Content-addressed layout:** `images/{hash[..2]}/{hash}.{ext}`. The
  2-char fan-out directory keeps any one dir bounded even at tens of
  thousands of entries. Same bytes ⇒ same path ⇒ writer is a no-op.
- **Schema v1:** new `entries.image_extension TEXT` column (bare ext,
  no dot). Image rows' `content` column becomes the bare hash. Migration
  from v0 is automatic on first open, idempotent, rewrites the layout in
  a single transaction.
- **Atomic writes:** `tmp-write → fsync(file) → rename → fsync(parent)`.
  Temp filenames include PID to avoid racing concurrent writers. Stale
  `.tmp` files older than 60 s are swept on every startup.
- **Refcount prune via queue:** deletion sites INSERT into
  `pending_blob_prunes (hash, extension, queued_at)` inside the same SQL
  transaction that removes the row. `drain_pending_blob_prunes` runs on
  every startup AND right after each delete/clear/cleanup_old. Crash
  between row-gone and file-gone just means the next startup finishes
  the job.
- **Watcher:** `Clipboard::read_image()` returns `ClipboardImage { bytes,
  hash, extension }` without touching disk. `poll_internal` short-circuits
  on `last_hash` match OR `exists_by_hash` before calling
  `Database::store_image_blob`. `initialize_hash` now primes from the
  image side first, then text.
- **`ditox repair`:** orphan files (on disk but no row) → removed;
  dangling rows (blob missing) → removed. With `--fix-hashes`: verifies
  each referenced file's SHA-256 matches the DB hash; mismatches move to
  `images/.quarantine/{db_hash}_{actual_hash}.{ext}` for manual review.
  `--dry-run` reports without mutating.

## Files changed

### Core (`ditox-core/src/`)

- `db.rs` — new constants (`SCHEMA_VERSION`, `QUARANTINE_DIR`,
  `TMP_SUFFIX`, `TMP_SWEEP_AGE_SECS`); helpers `sweep_one`,
  `is_tmp_leftover`; methods `image_path`, `store_image_blob`,
  `queue_blob_prune_tx`, `drain_pending_blob_prunes`,
  `sweep_stale_tmp_files`, `scan_image_files`, `referenced_image_blobs`,
  `image_rows_with_paths`, `delete_dangling_row`, `quarantine_file`,
  `read_schema_version`, `write_schema_version`,
  `migrate_image_store_to_v1`; `delete`/`clear_all`/`cleanup_old` now
  take `&mut self` and use SQL transactions + the prune queue.
- `entry.rs` — `Entry.image_extension: Option<String>`; `new_image(hash,
  size, extension)` signature; `image_path()` convenience method;
  `preview()` synthesises `image-<hash8>.<ext>`.
- `clipboard.rs` — new `pub struct ClipboardImage`; new pure
  `Clipboard::read_image()` on both Linux & Windows; old `get_image`
  removed.
- `watcher.rs` — rewritten `poll_internal` and `initialize_hash` with
  content-addressed store flow.
- `app.rs` — `copy_selected`/`copy_snippet` call sites use
  `entry.image_path()` to derive the real path for `Clipboard::set_image`.

### TUI (`ditox-tui/src/`)

- `cli.rs` — new `Repair { dry_run, fix_hashes }` subcommand.
- `main.rs` — `cmd_delete` + `cmd_clear` now take `&mut Database`;
  redundant `std::fs::remove_file` / `remove_dir_all` removed; `cmd_copy`
  uses `entry.image_path()`; new `cmd_repair` handler.
- `ui/preview.rs` — resolves `entry.image_path()` before feeding
  ratatui-image.

### GUI (`ditox-gui/src/`)

- `app.rs` — `CopyEntry`/`CopyFromPreview`/`view_thumbnail` and
  `update_image_cache` resolve `entry.image_path()` instead of reading
  `entry.content` as a filesystem path.

### Tests

- `ditox-core/tests/image_store.rs` — 6 unit tests covering atomic
  idempotent store, delete prunes, clear prunes, cleanup_old prunes,
  startup drain of pending queue, scan ignores quarantine/`.tmp`.
- `ditox-tui/tests/cli_image_store.rs` — 6 CLI integration tests
  covering `ditox delete`, `ditox clear`, `ditox repair`,
  `--dry-run`, and `--fix-hashes` quarantine.
- Existing `fts_test.rs` and `search_benchmark.rs` updated for new
  `Entry.image_extension` field.

## Testing

```bash
cargo test --workspace        # 33 tests pass
cargo build --release         # clean
```

End-to-end verification on live Hyprland/Wayland via
`/tmp/ditox-bughunt/hunt-fixed.sh` — all 17 assertions pass:

- Bug #1: 3× same image copy → 1 row, 1 file. ✓
- Bug #2: `ditox delete` → rows=0, files=0. ✓
- Bug #3: `cleanup_old` eviction → rows == files. ✓
- Bug #4: watcher restart with image still on clipboard → no new rows,
  no new files. ✓
- `ditox repair --dry-run` → reports, doesn't mutate. ✓
- `ditox repair` → removes 1 orphan, deletes 1 dangling row. ✓
- Hash integrity check of stored blobs: all pass. ✓

## Work Log

### 2026-04-24
- Phase 0: built a CLI-based bug-hunt harness (`/tmp/ditox-bughunt/hunt.sh`).
- Reproduced all four bugs live on Hyprland.
- Confirmed Hyprland's clipboard is byte-stable for the same content
  (wl-paste twice → identical SHA-256), so content-hash dedup works on
  this compositor.

### 2026-04-25
- Implemented schema v1, migration, atomic store, prune queue.
- Added `Clipboard::read_image()` pure reader.
- Rewrote watcher to dedup BEFORE disk write.
- Updated all TUI/GUI/CLI call sites for new `Entry.image_extension` /
  `image_path()` contract.
- Added `ditox repair` + `--dry-run` + `--fix-hashes`.
- Added 12 tests (6 unit, 6 integration); all workspace tests green.
- Ran the updated `hunt-fixed.sh` against the live build — 17/17
  assertions pass.
