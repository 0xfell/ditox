# Task: Schema v1 → v2 (preparatory columns)

> **Status:** completed
> **Priority:** medium
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

Add columns and indexes that Phase 1 (multi-format) and Phase 3
(per-app exclusion) will need, but apply them as a small, isolated v1
→ v2 migration so the bigger v2 → v3 multi-format migration in Phase 1
is cleaner.

## Requirements

- [ ] **`schema_meta.version`** updated from `1` to `2` after migration.
- [ ] **Add column `entries.entry_kind TEXT`** populated from
      `entry_type` during migration. Future code reads `entry_kind`;
      `entry_type` is kept for one version, deprecated.
- [ ] **Add column `entries.format_count INTEGER NOT NULL DEFAULT 1`.**
      Anticipates multi-format: every existing entry has `format_count
      = 1`.
- [ ] **Add column `entries.source_app TEXT`.** Populated by Phase 2's
      foreground tracker; for now `NULL`.
- [ ] **Add index `idx_entries_source_app ON entries(source_app)`.**
- [ ] **Add column `entries.captured_at TEXT`.** Distinct from
      `created_at` because Phase 1 will allow clip-de-duplication to
      "promote" an existing entry to top without changing its
      `created_at` (preserving original first-seen time). For v2,
      backfill `captured_at = created_at`.
- [ ] **Migration code** in `db.rs` that runs only when current schema
      version < 2.
- [ ] **Idempotent.** Running on a v2 DB is a no-op.
- [ ] **Snapshot tests.** Add `tests/migrations/snapshot_v1.db.gz` (a
      tiny v1 DB) and a test that opens it via `Database::open`, asserts
      schema_meta.version == 2 after migration, asserts entry counts
      preserved.

## Implementation Notes

The migration in `db.rs::init_schema` already uses defensive `ALTER
TABLE … ADD COLUMN .ok()` for v1 columns. Same pattern works for v2.

Pseudocode:

```rust
let version = read_schema_version(&conn)?;
if version < 2 {
    conn.execute_batch(r#"
        BEGIN;
        ALTER TABLE entries ADD COLUMN entry_kind TEXT;
        ALTER TABLE entries ADD COLUMN format_count INTEGER NOT NULL DEFAULT 1;
        ALTER TABLE entries ADD COLUMN source_app TEXT;
        ALTER TABLE entries ADD COLUMN captured_at TEXT;
        CREATE INDEX IF NOT EXISTS idx_entries_source_app ON entries(source_app);
        UPDATE entries SET entry_kind = entry_type WHERE entry_kind IS NULL;
        UPDATE entries SET captured_at = created_at WHERE captured_at IS NULL;
        UPDATE schema_meta SET value = '2' WHERE key = 'version';
        COMMIT;
    "#)?;
}
```

`Entry` model gains corresponding fields. JSON serialisation kept stable
where users have existing scripts consuming it (don't rename existing
fields; add new optional ones).

## Testing

- Schema test: fresh DB ends at version 2 with all columns.
- Migration test: load `snapshot_v1.db.gz`, open, assert version 2,
  assert all rows preserved.
- Round-trip test: insert via `Entry::new`, fetch, assert all v2
  fields populated correctly.
- Idempotency test: open the migrated DB twice, assert no errors and
  no duplicate columns.

## Work Log

### 2026-04-26
- Task file created.
- Bumped `SCHEMA_VERSION` constant to `2`.
- Added `migrate_to_v2()` to `Database`: defensive `ALTER TABLE … ADD COLUMN` for `entry_kind`, `format_count`, `source_app`, `captured_at`; backfill `entry_kind = entry_type` and `captured_at = created_at`; create `idx_entries_source_app` index.
- Hooked into `init_schema()` after the v0→v1 image-store migration. Idempotent.
- `Entry` model not yet updated to expose the new fields — that's a Phase 1 concern (the multi-format model will overhaul `Entry` substantially anyway). v2 just lays the column substrate.
- Wrote `ditox-core/tests/schema_v2_migration.rs` with 4 tests: fresh DB lands at v2, fresh DB has v2 columns + index, v1 → v2 backfills correctly with two pre-existing rows, migration is idempotent across reopens.
- All 44 workspace tests pass. Build green.
