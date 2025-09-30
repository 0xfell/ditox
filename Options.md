# Ditox Configuration Options

This document lists every configurable option, what it does, the expected type, defaults, and practical bounds. All options live in `~/.config/ditox/settings.toml`.

Notes
- “Default” means the value used when the key is omitted.
- “Min/Max” list validated bounds where the code enforces them; when not enforced, practical recommendations are given.
- Durations accept a number followed by a unit: `s`, `m`, `h`, `d`, `w` (seconds/minutes/hours/days/weeks).

## Storage

```toml
[storage]
backend = "localsqlite"  # or "turso"
# db_path = "/path/to/ditox.db"          # localsqlite only
# url = "libsql://<host>"                # turso only (required)
# auth_token = "<token>"                 # turso optional
```

- backend (string)
  - What: Selects the store implementation.
  - Allowed: `"localsqlite"` (default), `"turso"`.
- db_path (string, path)
  - What: SQLite file path for local backend.
  - Default: `$XDG_CONFIG_HOME/ditox/db/ditox.db`.
- url (string)
  - What: Remote LibSQL/Turso URL.
  - Required when backend=`"turso"`. Must start with `libsql://`.
- auth_token (string)
  - What: Remote authentication token for Turso/LibSQL.
  - Default: unset (some deployments allow anonymous read/write with appropriate DB policy).

## Prune (optional)

```toml
[prune]
# every = "1d"
# keep_favorites = true
# max_items = 10000
# max_age = "90d"
```

- every (duration string)
  - What: Intended cadence for a background prune (informational for CLI; timers/scripts may use it).
  - Default: unset.
- keep_favorites (bool)
  - What: Protect favorites from pruning.
  - Default: true when unset.
- max_items (integer)
  - What: Keep at most N most recent entries (older ones may be removed).
  - Default: unset (no count-based pruning).
  - Min/Max: ≥1; practical upper bound depends on disk space.
- max_age (duration string)
  - What: Remove entries older than this age.
  - Default: unset (no age-based pruning).

## Sync (optional)

```toml
[sync]
# enabled = true
# interval = "5m"
# batch_size = 500
# device_id = "my-device"
```

- enabled (bool)
  - What: Enables periodic sync (your scheduler/runner controls cadence; CLI `sync run` is on-demand).
  - Default: unset/disabled.
- interval (duration string)
  - What: Intended cadence for background sync (informational; your timer should use it).
  - Default: unset.
- batch_size (integer)
  - What: Rows pushed/pulled per batch.
  - Default: 500.
  - Min/Max: ≥1; practical max 10_000.
- device_id (string)
  - What: Identifier used for tie‑breaking/metadata.
  - Default resolution order: `DITOX_DEVICE_ID` env → hostname → "local".

## Images (optional)

```toml
[images]
# local_file_path_mode = false
# dir = "/path/to/imgs"
# encoding = "png"
```

- local_file_path_mode (bool)
  - What: Store images by file path instead of embedding blobs (used when adding images).
  - Default: false.
- dir (string, path)
  - What: Base directory for image files when `local_file_path_mode = true`.
  - Default: `$XDG_CONFIG_HOME/ditox/data/imgs`.
- encoding (string)
  - What: Preferred on-disk/image copy encoding (currently informational; PNG is used internally).
  - Default: `"png"`.

## TUI

```toml
[tui]
page_size = 10
# auto_apply_tag_ms = 400
absolute_times = true
```

- page_size (integer)
  - What: Items per page in the picker.
  - Default: 10.
  - Min: 1. Practical max: 200 (UI readability/perf).
- auto_apply_tag_ms (integer, milliseconds)
  - What: When typing `#tag` in the search box, auto‑apply the tag after this idle time.
  - Default: unset (disabled).
  - Min: 1 ms. Practical range: 100–2000 ms.
- absolute_times (bool)
  - What: Picker shows absolute timestamps (ns precision) instead of relative "x ago".
  - Default: true.

## Global

- max_storage_mb (integer)
  - Location: top level (not in a table).
  - What: Advisory cap for local storage usage; used by tools/scripts to decide when to prune.
  - Default: unset.
  - Min: 1. Practical max: depends on disk space.

## Example (safe defaults + comments)

```toml
[storage]
backend = "localsqlite"
# db_path = ""

# [prune]
# every = "1d"
# keep_favorites = true
# max_items = 10000
# max_age = "90d"

# [sync]
# enabled = true
# interval = "5m"
# batch_size = 500
# device_id = "my-device"

# [images]
# local_file_path_mode = false
# dir = ""
# encoding = "png"

[tui]
page_size = 10
# auto_apply_tag_ms = 400
absolute_times = true

# max_storage_mb = 1024
```

If you need any option enforced at load time (e.g., clamp out‑of‑range values or reject invalid durations), I can add validation and friendly error messages in the CLI.
