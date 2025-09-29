# PRD: Local-Only Image Handling

## Summary
- Goal: Add an optional “local-only images” mode that never syncs image data to remote (Turso). Images are persisted as files under `~/.config/ditox/data/imgs/` and entries reference them by path.
- Default behavior: Keep storing image bytes locally in the entry (current behavior), but still do not sync image entries to remote.
- Optional behavior (via settings): When enabled, do not store image bytes in the entry at all; instead save an image file and persist only its file path in the entry. These entries are always excluded from remote sync.

## Motivation
- Large binary image blobs inflate the remote DB significantly, slow down sync, and provide limited utility if the primary use is local clipboard recall.
- Users should be able to choose between full in-entry storage (faster single-file backup, heavier DB) and file-path storage (lighter DB, easier external management), with both modes remaining local-only for images with respect to remote sync.

## Scope
- In scope:
  - Local filesystem storage for images under `~/.config/ditox/data/imgs/`.
  - Schema changes to mark and handle image entries, and to optionally store an image file path.
  - CLI and core behavior for capturing, listing, copying, and diagnosing image entries.
  - A settings flag to opt into local-file-path storage for images.
  - Migration to introduce new columns and preserve existing data.
- Out of scope (follow-ups):
  - Cross-device image sync, dedup, or remote object storage.
  - Non-Linux clipboard adapters (we target Linux first per v0.1.0).
  - Image content OCR or search.

## User Stories
- As a user, when I copy an image, Ditox should retain it locally and let me recall it later, without uploading it to remote.
- As a user, I can opt into storing images as files and keeping only a reference path in Ditox, to keep the DB small.
- As a user, I can still list and favorite image clips; `copy` should put the image back on my clipboard.
- As a user, `doctor` should detect missing image files and help me fix broken references.

## Terminology
- Image entry: A clip whose kind is image (leverages `ClipKind::Image` and `ImageMeta`).
- Local-only image: An image entry that is never synced to remote Turso.

## Behavior
- Capture:
  - Default mode: store image bytes in the entry (as today) for local persistence. Do not sync image entries to remote. `is_image = true` is tracked (see Schema) and `image_path` remains NULL.
  - Local-file-path mode (optional): do not keep image bytes in the entry. Save the image to `~/.config/ditox/data/imgs/` and store its absolute path in `image_path`. Mark the entry as image and exclude from remote sync.
- Copying to clipboard:
  - If `image_path` is set and the file exists, load from disk and copy to clipboard.
  - Else, if image bytes are present in the entry, use those.
  - If neither available, return a helpful error and suggest running `doctor`.
- Listing/searching:
  - `list` shows an image indicator (e.g., `🖼`) and filename or short path.
  - `search` continues to work on metadata; image bytes are not indexed. If we have `ImageMeta` fields, we may show dimensions/format.
- Favorites:
  - Works identically; does not change sync behavior (still never upload images).
- Sync:
  - Image entries are excluded from remote writes regardless of mode. In reads from remote, images simply won’t appear (consistent with current default policy).

## Filesystem
- Base dir: `~/.config/ditox/data/imgs/` (respect `XDG_CONFIG_HOME` if set; fallback to `~/.config`).
- Naming: `{entry_id}-{app_version}-{timestamp}.png` (example); we will finalize extension based on encoding.
  - `entry_id`: the UUID/row id assigned on insert.
  - `app_version`: semantic version string from the CLI.
  - `timestamp`: UTC RFC3339 or `YYYYMMDDTHHMMSSZ`.
  - Extension: default to `.png` for lossless stability; consider `.webp` if clipboard source is WebP; preserve original when possible.
- Ensure parent directories exist and set perms `0700` on `imgs/`.

## Settings
Add to `settings.toml` (names illustrative; final path may vary):

```
[images]
# When true, store image files on disk and only keep a file path in the entry.
local_file_path_mode = false

# Directory override for local image storage; when empty, use XDG path `~/.config/ditox/data/imgs/`.
# Accepts `~`, environment vars, and relative paths (resolved against config dir).
dir = ""

# Preferred encoding: "png" (default) | "webp" | "preserve".
encoding = "png"
```

Notes:
- Default `local_file_path_mode = false` retains current behavior (bytes in-entry), while still skipping remote sync for images.

## Schema & Migrations
- Table: `clip` (existing)
- New columns:
  - `is_image` INTEGER NOT NULL DEFAULT 0  -- boolean flag
  - `image_path` TEXT NULL                 -- absolute path to local image file when path mode is used

Rationale:
- We use `is_image` to clearly separate image entries for sync policy and UI.
- `image_path` is optional; only set when `local_file_path_mode = true`.
- Image bytes remain in existing content/blob fields when `local_file_path_mode = false`.

Migration plan:
- Create migration `00XX_images_local_only.sql`:
  - `ALTER TABLE clip ADD COLUMN is_image INTEGER NOT NULL DEFAULT 0;`
  - `ALTER TABLE clip ADD COLUMN image_path TEXT NULL;`
  - Optional: backfill `is_image = 1` where `kind = 'image'` if such a discriminator exists.
- Indexing: consider `CREATE INDEX clip_is_image_idx ON clip(is_image);` for faster queries.
- Rollback: `DROP COLUMN image_path; DROP COLUMN is_image;` (SQLite requires table rebuild; handled by our migration tooling if needed).

## Core API Changes (ditox-core)
- Model updates:
  - Extend entry/clip struct with `is_image: bool` and `image_path: Option<PathBuf>`.
- Store behavior:
  - On insert of an image clip:
    - If `local_file_path_mode` is true: write file, set `image_path`, set `is_image = true`, omit image bytes from entry storage.
    - Else: store bytes as today and set `is_image = true` (no `image_path`).
  - On fetch:
    - If `image_path` exists: prefer loading bytes from file for clipboard operations.
    - Else: use stored bytes when present.
- Clipboard adapters:
  - Continue to support `ClipKind::Image` + `ImageMeta`.
  - Ensure load-from-file path for `copy`.

## CLI Changes (ditox-cli)
- `add` (capture): detect image clips; apply storage mode; print where the image was stored (path or in-entry).
- `list`: show `🖼` tag; if `image_path` set, show basename; otherwise show `[image: in-entry]`.
- `copy`: handle path-first then in-entry fallback; error if neither available; suggest `doctor`.
- `search`: filter by `is_image` with `--images` (optional flag); content search does not include image bytes.
- `favorite`: unchanged.
- `migrate`: includes new migration; `--status` reflects columns.
- `doctor`: add checks:
  - Orphaned `image_path` (file missing on disk).
  - Files under `imgs/` not referenced by any entry.
  - Permissions on `imgs/` folder.

## Sync Policy
- Write (local → remote): never send rows where `is_image = 1`.
- Read (remote → local): image rows won’t appear remotely; no-op.
- Telemetry/metrics (if any): count skipped image entries, but do not log paths or content.

## Error Handling & Edge Cases
- Missing file at `image_path`: display actionable error and suggest `doctor` to repair; do not crash.
- Path collisions: filename includes `{entry_id}` to avoid conflicts; still check existence and append a suffix if needed.
- Unsupported formats: convert to PNG by default; fall back to raw bytes in-entry if conversion fails (still `is_image = 1`, still skip remote sync).
- Permissions: attempt to set `0700` on directory; warn if stricter perms cannot be set.

## Security & Privacy
- Never log image contents or absolute paths at info level; use debug with redaction.
- Image files live under user config; avoid world-readable perms.
- Tests should use temp dirs and never touch the real config path.

## Testing
- Core unit tests: path generation, file save, load, and fallback to in-entry bytes.
- CLI integration tests (`crates/ditox-cli/tests`):
  - Capture image → verify file created and DB row has `is_image` and `image_path` when path mode enabled.
  - Copy image from path; from in-entry.
  - `list` renders expected markers.
  - `doctor` reports missing file and orphaned file cases.
- Migration tests: create pre-migration DB, run migrate, confirm columns and defaults.

## Rollout
- Ship behind a config flag (`images.local_file_path_mode`).
- Default remains backward-compatible: images stored in-entry locally but not synced remotely.
- Provide `migrate --backup` guidance in release notes.

## Open Questions
- Exact filename format and extension policy (PNG-only vs preserve source format).
- Whether to move path storage under `XDG_DATA_HOME` instead of config; current spec follows `~/.config` for consistency with existing Ditox paths.
- Optional: store `ImageMeta` (width/height/mime) as separate columns for richer UX.

