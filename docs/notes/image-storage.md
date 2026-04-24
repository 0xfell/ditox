# Image Storage Architecture

> **Applies to:** schema v1+
> **Introduced by:** task `011-image-storage-bug`

## Layout on disk

```
<data_dir>/images/
├── 00/
│   ├── 00e5…a7c2.png
│   └── 00ff…2241.png
├── 1a/
│   └── 1a3d…b901.jpg
├── …
├── 4f/
│   └── 4ff6ab670a58c14270e034e2090d9a432caa263a14e0a25785386b0c12f880b5.png
└── .quarantine/
    └── {db_hash}_{actual_hash}.{ext}
```

- Every file is named by its full SHA-256 hash + a lowercase extension.
- The two-character prefix directory is the first two hex chars of the hash.
  This fans the tree out so any one subdir stays under a few hundred files
  even if a user captures tens of thousands of images.
- `.quarantine/` is populated only by `ditox repair --fix-hashes` when
  an on-disk file's hash doesn't match its DB row. We never delete from
  there — it's for human inspection.
- `.tmp` files (with PID suffix) are transient during an atomic write.
  Anything older than 60 seconds is swept on the next startup.

## Database columns

```sql
entries.content          TEXT  -- for images: the bare 64-char hex hash
entries.hash             TEXT  -- same value (UNIQUE)
entries.image_extension  TEXT  -- "png" / "jpg" / … (NULL for text entries)
entries.byte_size        INTEGER
```

The path is derived via `Database::image_path(&hash, &ext)`; code never
stores absolute paths in the DB. `Entry::image_path()` is the single
public helper for resolving a row to its blob path.

## Atomic write protocol

```
store_image_blob(hash, ext, bytes):
    path = images_dir / hash[..2] / "{hash}.{ext}"
    if path.exists(): return (path, False)      # content-addressed no-op
    tmp  = "{path}.{pid}.tmp"
    File::create(tmp).write_all(bytes).sync_all()   # fsync contents
    fs::rename(tmp, path)                           # atomic on same FS
    fsync(path.parent())                            # make rename durable
    return (path, True)
```

- PID in the temp name prevents two concurrent writers from clobbering
  each other's tmp on the way to the same destination.
- If rename fails but the destination now exists, another writer won
  the race — we delete our tmp and return success.
- `fsync` on both the file and parent means a power loss can never
  expose a half-written blob.

## Refcount prune via persistent queue

`entries.hash` has a `UNIQUE` constraint, so a blob's refcount is always
0 or 1. We still model it as a refcount for future extensibility.

On delete paths (`Database::delete`, `clear_all`, `cleanup_old`) we do:

```rust
let tx = conn.transaction()?;
// Queue (hash, ext) for every image row we're deleting.
tx.execute("INSERT OR IGNORE INTO pending_blob_prunes …");
tx.execute("DELETE FROM entries WHERE …");
tx.commit()?;             // row gone AND queued atomically
drain_pending_blob_prunes();  // unlinks files outside the SQL tx
```

`drain_pending_blob_prunes` scans the queue, verifies refcount == 0,
unlinks the file, and removes the queue row. If the unlink fails (e.g.
EBUSY, EACCES), the queue entry is left alone for the next startup.

Startup always runs `drain_pending_blob_prunes()` + `sweep_stale_tmp_files()`.
Cost is milliseconds even on a large tree.

## Watcher flow

The watcher reads image bytes into memory via `Clipboard::read_image()`
but does NOT write to disk until it has decided the entry is new:

```
1. bytes = Clipboard::read_image()
2. if bytes.hash == last_hash: return            # unchanged
3. if db.exists_by_hash(bytes.hash): return      # dup
4. Database::store_image_blob(hash, ext, bytes)
5. db.insert(Entry::new_image(hash, size, ext))
6. db.cleanup_old(max_entries)                   # may evict + prune
```

This is the fix for the original write-before-dedup bug: steps 2 and 3
short-circuit BEFORE any disk IO. `initialize_hash()` also primes from
the image side first so a watcher restart doesn't re-capture a still-
present clipboard image.

## Invariants

1. **Every image row has a backing file** (except transiently, during a
   crashed delete — healed at next startup).
2. **Every file on disk is referenced by some row**, except:
   - files under `.quarantine/`,
   - `.tmp` files younger than `TMP_SWEEP_AGE_SECS`,
   - orphans created by external writers (cleaned by `ditox repair`).
3. **Row hash always matches on-disk bytes** for referenced files (unless
   the OS corrupted bits — `ditox repair --fix-hashes` quarantines those).

## `ditox repair` subcommand

```
ditox repair             # apply: remove orphans + dangling rows
ditox repair --dry-run   # report only
ditox repair --fix-hashes         # also verify and quarantine mismatches
ditox repair --dry-run --fix-hashes
```

Exit code is 0 on success even when fixes were applied; non-zero only
on unrecoverable errors (e.g. DB lock failure).

## Migration from schema v0

`init_schema` reads `schema_meta.version`. If < 1 it runs
`migrate_image_store_to_v1()`:

1. Snapshot every `entry_type = 'image'` row.
2. For each: skip if already in the new layout (`image_extension` set
   and `content` looks like a 64-hex hash).
3. Read the legacy file (`content` = old path).
4. Verify the on-disk bytes hash to the row's declared `hash`.
5. If they match: `store_image_blob`, update the row
   (`content = hash`, `image_extension = ext`), remove the legacy file.
6. If they don't match: log a warning, leave the legacy file for manual
   review. `ditox repair --fix-hashes` will quarantine it later.

The migration is idempotent (safe to re-run) and single-transaction per
row.
