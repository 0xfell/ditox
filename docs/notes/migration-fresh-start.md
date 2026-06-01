# Fresh-Start Clipboard Manager Plan

Date: 2026-05-25

Purpose: this is a standalone blueprint for rebuilding the project from zero.
It assumes the old repository may be deleted. It captures the product lessons,
core behavior, data model, platform details, known problems, and suggested
implementation order without depending on any other file.

## 1. Product Decision

Build a terminal-first clipboard manager.

Do not rebuild a desktop GUI. Do not keep old GUI compatibility flags, tray
logic, layer-shell behavior, AppImage-specific GUI packaging, or desktop-window
process models.

The project can use any frontend stack. The important boundary is this:

- The terminal UI owns rendering, keyboard interaction, focus inside the app,
  selection state, and visual workflow.
- The native/backend layer owns clipboard access, storage, image blobs,
  foreground tracking, paste-back, sync, repair, and platform integration.
- The terminal UI must not read or write the database directly.
- The terminal UI should talk to the backend through a stable command/event API.

This makes the frontend stack replaceable. The UI could be OpenTUI, Ratatui,
Ink, Bubble Tea, Textual, a custom terminal renderer, or another future stack.
The product architecture should not depend on that choice.

## 2. Target Product Scope

The product should be one excellent terminal clipboard manager with:

- Clipboard watcher daemon.
- Terminal app for browsing and acting on history.
- Scriptable CLI or command bridge.
- Local storage with SQLite or an equivalent embedded database.
- Content-addressed image blob storage.
- Text, image, HTML, RTF, URI-list, and platform-specific format capture.
- Search across content, formats, notes, tags, and collections.
- Copy selected history item back to the clipboard.
- Optional paste-back into the previously focused app.
- Repair tooling for database/blob consistency.
- Clear platform degradation behavior.

Non-goals for the fresh start:

- Desktop GUI.
- Tray icon.
- Floating launcher window.
- Desktop window focus hacks.
- GUI installers as a primary product surface.
- Frontend direct database coupling.
- Carrying historical migrations into the new database.

## 3. Recommended High-Level Architecture

Use four conceptual layers.

### 3.1 Terminal UI

Responsibilities:

- Draw the app.
- Manage UI focus and modes.
- Handle keymaps and mouse input if supported.
- Maintain ephemeral state:
  - selected row.
  - current query.
  - active filter/tab.
  - preview focus.
  - multi-selection.
  - open dialogs.
  - status messages.
- Request data/actions from backend API.
- Subscribe to backend events when available.

Non-responsibilities:

- Opening SQLite.
- Resolving image blob paths without backend permission.
- Implementing platform clipboard APIs.
- Implementing paste-back.
- Running sync protocol.
- Running repair logic.

### 3.2 Backend API

Responsibilities:

- Stable JSON command/event contract.
- Validation of user requests.
- Authorization of local-only operations if a daemon/socket is used.
- Translation between UI concepts and core services.

Start simple:

- JSON request/response over stdio is acceptable for the first version.

Upgrade when needed:

- Long-running daemon over Unix socket or named pipe.
- Event stream for new captures, watcher state, sync state, and config reloads.

Avoid:

- HTTP server by default.
- Network API by default.
- UI-specific database queries.

### 3.3 Core Services

Responsibilities:

- Storage.
- Capture/watch.
- Clipboard read/write.
- Image blob store.
- Search.
- Paste-back.
- Foreground tracking.
- Filter rules.
- Text transforms.
- Sync.
- Config.
- Logging.
- Repair.

### 3.4 Platform Backends

Responsibilities:

- Wayland clipboard capture/write.
- Windows clipboard capture/write.
- macOS pasteboard capture/write.
- Foreground app/window tracking.
- Keystroke synthesis.
- Suspend/resume events.
- Platform-specific data/config paths.

Each platform backend should expose a small trait/interface to the core. The
core should own policy. Platform backends should only report facts and perform
native operations.

## 4. Process Model

### 4.1 Minimal First Version

```text
terminal UI
  -> spawn backend command with JSON request
  -> backend opens storage, executes command, prints JSON response
```

Good for:

- listing entries.
- search.
- get entry.
- copy entry.
- delete/favorite/update notes.
- early contract tests.

Weakness:

- repeated process startup cost.
- no live event stream.
- not ideal for auto-refresh or watcher control.

### 4.2 Mature Version

```text
terminal UI
  -> local socket / named pipe
  -> daemon
       -> storage
       -> watcher
       -> sync runtime
       -> clipboard/native APIs
       -> event stream
```

Good for:

- live updates.
- lower latency.
- one owner for long-running watcher and sync runtime.
- easier config reload.
- fewer concurrent database opens.

Recommendation:

- Design the command API first.
- Implement stdio transport first.
- Keep the API transport-agnostic so it can move to a daemon later.

## 5. Backend Command Contract

Use a request/response envelope.

Request:

```json
{
  "id": "request-id",
  "method": "entries.list",
  "params": {}
}
```

Success:

```json
{
  "id": "request-id",
  "ok": true,
  "result": {}
}
```

Failure:

```json
{
  "id": "request-id",
  "ok": false,
  "error": {
    "code": "not_found",
    "message": "entry not found",
    "details": {}
  }
}
```

Rules:

- Every response includes the request id.
- Every method has versioned params and result schemas.
- Error codes are stable machine-readable strings.
- User-facing error text is separate from error codes.
- The UI never parses human CLI output.
- The backend may add fields, but should not remove or rename fields without a
  bridge version bump.

Core first-version methods:

- `health.check`
- `config.get`
- `watcher.status`
- `entries.list`
- `entries.search`
- `entries.get`
- `entries.copy`
- `entries.delete`
- `entries.favorite`
- `entries.update_notes`
- `collections.list`
- `tags.list`
- `stats.get`

Second-version methods:

- `watcher.start`
- `watcher.stop`
- `entries.copy_and_paste`
- `entries.transform`
- `entries.aggregate`
- `collections.create`
- `collections.rename`
- `collections.delete`
- `collections.assign_entry`
- `tags.create`
- `tags.assign_entry`
- `tags.remove_from_entry`
- `rules.list`
- `rules.create`
- `rules.update`
- `rules.delete`
- `repair.check`
- `repair.apply`

Future event names:

- `entry.captured`
- `entry.updated`
- `entry.deleted`
- `watcher.started`
- `watcher.stopped`
- `watcher.unhealthy`
- `sync.peer_discovered`
- `sync.log_appended`
- `config.changed`

## 6. Core Data Model

### 6.1 Entry Summary

The UI mostly needs summaries:

```json
{
  "id": "uuid",
  "kind": "text",
  "canonical_format": "text/plain;charset=utf-8",
  "preview": "short display text",
  "hash": "sha256-hex",
  "byte_size": 123,
  "created_at": "2026-05-25T10:00:00Z",
  "last_used": "2026-05-25T10:00:00Z",
  "usage_count": 0,
  "favorite": false,
  "notes": null,
  "collection": null,
  "tags": [],
  "source_app": null,
  "format_count": 1
}
```

Rules:

- `id` is stable and opaque to the UI.
- `hash` is not a display label; use it for dedup/debug only.
- `preview` is sanitized and safe for terminal display.
- `canonical_format` is one of the backend's canonical format names.
- `kind` is coarse: `text` or `image`.
- Rich content lives in formats, not directly in the summary.

### 6.2 Full Entry

```json
{
  "entry": {},
  "formats": [
    {
      "format_name": "text/plain;charset=utf-8",
      "storage": "inline",
      "byte_size": 123,
      "format_hash": "sha256-hex",
      "canonical": true,
      "content": "full text only when requested"
    }
  ]
}
```

Rules:

- Full text is returned only by `entries.get` or preview-specific methods.
- Image bytes are not embedded in normal responses.
- Image previews should be separate:
  - terminal protocol payload.
  - safe temp path.
  - dimensions plus placeholder.
  - or backend-rendered preview bytes, depending on UI stack.

### 6.3 Canonical Format Names

Use stable names from day one.

Text/rich formats:

- `text/plain;charset=utf-8`
- `text/html`
- `text/rtf`
- `text/uri-list`
- `x-special/gnome-copied-files`
- `application/json`
- `application/xml`
- `application/xhtml+xml`

Image formats:

- `image/png`
- `image/jpeg`
- `image/gif`
- `image/webp`
- `image/bmp`
- `image/tiff`

Windows-specific formats:

- `win32:CF_TEXT`
- `win32:CF_UNICODETEXT`
- `win32:CF_OEMTEXT`
- `win32:CF_BITMAP`
- `win32:CF_DIB`
- `win32:CF_DIBV5`
- `win32:CF_TIFF`
- `win32:CF_HDROP`
- `win32:HTML Format`
- `win32:Rich Text Format`
- custom registered Windows formats as `win32:<registered-name>`

Rules:

- The frontend must not invent format names.
- Format names are part of the database, bridge, sync, and import contract.
- Changing a canonical name is a breaking change.

## 7. Storage Design

Use an embedded database. SQLite with FTS5 is proven and recommended.

Do not carry historical migrations into the fresh database. Start with schema
version 1 that already contains the desired model. If old data must be kept,
write an importer from the old database into the new schema.

### 7.1 Tables

Minimum tables:

- `schema_meta`
- `entries`
- `entry_formats`
- `format_content_fts`
- `entries_fts` or equivalent notes/content FTS table
- `collections`
- `tags`
- `entry_tags`
- `filter_rules`
- `pending_blob_prunes`
- `peers`
- `sync_log`

### 7.2 Entries Table

Recommended shape:

```sql
CREATE TABLE entries (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('text', 'image')),
  canonical_format TEXT NOT NULL,
  display_content TEXT NOT NULL,
  hash TEXT NOT NULL UNIQUE,
  byte_size INTEGER NOT NULL,
  captured_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_used TEXT NOT NULL,
  usage_count INTEGER NOT NULL DEFAULT 0,
  favorite INTEGER NOT NULL DEFAULT 0,
  notes TEXT,
  collection_id TEXT,
  source_app TEXT,
  global_hotkey TEXT UNIQUE,
  local_hotkey TEXT
);
```

Notes:

- `display_content` is sanitized display/source text for the primary row.
- Image entries use display content like `image-<hash-prefix>.<ext>`.
- `hash` is the dedup key for the canonical user-visible payload.
- `captured_at` is when the clipboard was observed.
- `created_at` is when the database row was created.
- `last_used` changes when the user copies/pastes the entry.

### 7.3 Entry Formats Table

Recommended shape:

```sql
CREATE TABLE entry_formats (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  format_name TEXT NOT NULL,
  storage TEXT NOT NULL CHECK (storage IN ('inline', 'blob_file')),
  content TEXT,
  blob_hash TEXT,
  blob_ext TEXT,
  byte_size INTEGER NOT NULL,
  format_hash TEXT NOT NULL,
  canonical INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  UNIQUE(entry_id, format_name)
);
```

Rules:

- Text-like formats use `storage = 'inline'` and store canonical text/content.
- Image-like formats use `storage = 'blob_file'`.
- `blob_hash` and `blob_ext` resolve into the blob store.
- Exactly one format per entry should be canonical.
- `format_hash` is SHA-256 of canonicalized bytes for that format.

### 7.4 Collections

Collections are single-membership groups.

Recommended fields:

- `id`
- `name`
- `color`
- `keybind`
- `position`
- `created_at`

Rules:

- Entry has at most one `collection_id`.
- Collection name is unique.
- Deleting a collection should unassign entries or cascade according to product
  decision. Unassigning is safer.

### 7.5 Tags

Tags are many-to-many labels.

Recommended fields:

- `id`
- `name`
- `color`
- `created_at`

Join table:

- `entry_id`
- `tag_id`

Rules:

- Tag name is unique.
- Tags are independent of collections.
- An entry can have multiple tags.

### 7.6 Filter Rules

Filter rules run at capture time.

Recommended fields:

- `id`
- `name`
- `pattern`
- `pattern_kind`: `regex`, `glob`, `contains`
- `process_glob`
- `action`: `drop`, `transform:<id>`, `tag:<name>`
- `enabled`
- `position`
- `created_at`

Rules:

- Enabled rules are evaluated by ascending `position`.
- First match wins.
- If `process_glob` is set and foreground app is unknown, skip the rule.
- Invalid regex should disable or skip that rule, not crash the watcher.

## 8. Image Blob Store

Keep the proven content-addressed design.

### 8.1 Layout

```text
<data_dir>/images/
  00/
    00e5...a7c2.png
  1a/
    1a3d...b901.jpg
  ...
  .quarantine/
    <db_hash>_<actual_hash>.<ext>
```

Rules:

- File name is full SHA-256 hex plus lowercase extension.
- Directory is the first two hex chars of the hash.
- Database stores hash and extension, never absolute paths.
- Quarantine is for human inspection and should not be auto-deleted.
- Temporary files end in `.tmp` and are swept when stale.

### 8.2 Atomic Write Protocol

```text
put_blob(hash, ext, bytes):
  path = images / hash[0..2] / (hash + "." + ext)
  if path exists:
    return already_present

  tmp = path + "." + pid + ".tmp"
  create tmp
  write all bytes
  fsync tmp
  rename tmp -> path
  fsync parent directory
  return created
```

Rules:

- Never write image bytes before dedup checks.
- Temp path includes process id to avoid concurrent writer collisions.
- If rename fails but final path exists, another writer won; remove temp and
  return success.
- Startup sweeps stale temp files.

### 8.3 Prune Queue

Use `pending_blob_prunes`.

Delete flow:

```text
begin transaction
  queue hash/ext for deleted image rows
  delete rows
commit
drain queue outside transaction
```

Drain rules:

- If no live image row references hash/ext, remove file.
- If removal fails, keep queue row for retry.
- If file is missing, remove queue row.
- Run drain at startup and after delete/cleanup operations.

### 8.4 Repair

Repair modes:

- Dry run.
- Apply orphan cleanup.
- Apply dangling row cleanup.
- Optional hash verification.

Repair actions:

- Remove files on disk not referenced by DB.
- Delete or mark DB rows whose blob is missing.
- Quarantine files whose bytes hash differently from the DB hash.
- Report counts in machine-readable output.

## 9. Capture and Watcher Design

### 9.1 Raw Clip Model

```json
{
  "captured_at": "timestamp",
  "source_app": "optional-process-basename",
  "formats": [
    {
      "mime": "text/plain;charset=utf-8",
      "bytes": "raw bytes in backend memory"
    }
  ]
}
```

Implementation type can be native structs, but this is the concept.

Rules:

- A clip can contain multiple formats.
- Formats are raw at capture boundary.
- Canonicalization happens before hashing/storage.
- Capture backend reads OS state.
- Watcher policy decides what to keep.

### 9.2 Capture Source Interface

Each backend should support:

- `name()`
- `current_snapshot() -> optional RawClip`
- `subscribe() -> stream of RawClip`
- `shutdown()`

This works for both polling and event-driven backends.

### 9.3 Watcher Pipeline

Order matters:

1. Get snapshot from highest-priority source.
2. Run capture scripts, if enabled.
3. Compute whole-clip hash for in-memory dedup.
4. If whole-clip hash equals last processed hash, skip.
5. Get foreground app once.
6. Apply per-app capture exclusion.
7. Apply filter rules.
8. Canonicalize and prepare formats.
9. Enforce per-format size caps.
10. Enforce per-clip total size cap.
11. Choose canonical display format.
12. Check paste sentinel.
13. Check database dedup by canonical hash.
14. Store image blob only if new.
15. Insert entry and all formats transactionally.
16. Apply rule-generated tags.
17. Cleanup old entries.
18. Update last processed hash only for accepted or intentionally skipped
    self-paste/uncapturable clips, not for security exclusions or drop rules.

Important behavior:

- Image formats take priority over text for canonical display.
- For browser copy-image cases, image wins over URL text.
- Text/html/RTF and other formats can still be stored as non-canonical formats.
- Security exclusions and drop rules do not advance `last_hash`, so the same
  bytes copied later from another app can still be captured.

### 9.4 Capture Limits

Recommended defaults:

- `max_format_size_bytes = 10 MiB`
- `max_clip_size_bytes = 25 MiB`
- `poll_interval_ms = 250`
- `max_entries = 500`

Behavior:

- Oversized format can be dropped while keeping other formats.
- Oversized total clip should drop the entire clip.
- Empty canonicalized formats should be skipped.
- Duplicate canonical format names should keep the first one.

## 10. Format Canonicalization

Purpose: dedup equivalent clipboard data without merging distinct user-visible
content.

Rules:

- Be conservative.
- False positives are worse than false negatives.
- If parsing fails, hash original bytes.

Canonicalization behaviors to keep:

- `text/plain` and `UTF8_STRING` normalize to `text/plain;charset=utf-8`.
- HTML:
  - Detect Windows HTML clipboard envelope.
  - Extract fragment between `StartFragment` and `EndFragment`.
  - Preserve `SourceURL` as metadata if needed.
  - Raw HTML without envelope remains raw.
- RTF:
  - Only canonicalize valid RTF starting with `{\rtf`.
  - Strip volatile RSID control words.
  - Strip known volatile destination groups like RSID tables and internal
    data/theme groups.
  - Avoid unbounded brace stripping.
- Images:
  - Hash actual image bytes used for storage.
  - Prefer PNG when converting raw platform image buffers.

## 11. Search Design

Search modes to keep:

- Broad search across all text-bearing formats and notes.
- Fuzzy search for interactive UI.
- Regex search with clear invalid-pattern feedback.
- Format-restricted search:
  - `/p query` plain text.
  - `/h query` HTML.
  - `/r query` RTF.
  - `/q query` notes.
  - `/f query` full text.

Backend should own:

- Search query parsing.
- FTS routing.
- Result pagination.
- Optional highlight ranges.
- Filter composition.

UI should own:

- Search input rendering.
- Search mode indicator.
- Keyboard behavior.
- Highlight display, using backend-provided ranges if available.

Response shape:

```json
{
  "items": [],
  "total": 123,
  "limit": 20,
  "offset": 0,
  "query": "term",
  "scope": "default"
}
```

## 12. Terminal UI Workflows to Preserve

Core workflows:

- Browse clipboard history.
- Move up/down.
- Jump top/bottom.
- Page up/down.
- Search.
- Toggle fuzzy/regex.
- Preview entry.
- Copy selected entry.
- Copy selected entry and exit.
- Delete selected entry with confirmation.
- Clear all with confirmation.
- Toggle favorite.
- Edit notes.
- Switch filters/tabs.
- Multi-select entries.
- Batch delete.
- Batch copy/merge text entries.
- Show help.
- Show status messages.
- Refresh automatically when watcher captures new entries.

Useful filters/tabs:

- All.
- Text.
- Images.
- Favorites.
- Today.
- Yesterday.
- This week.
- This month.
- Older.
- Collection.
- Uncollected.
- Tag.

Preview modes:

- Wrapped text.
- Horizontal scroll.
- Truncated.
- Hex view.
- Raw/escaped control view.
- Image preview or image metadata fallback.

Terminal constraints:

- Must work in narrow terminals.
- Must not overflow text inside controls.
- Must not assume image preview protocol exists.
- Must degrade image display to metadata/placeholder.
- Must keep key handling responsive under auto-refresh.

## 13. Clipboard Write and Paste-Back

### 13.1 Clipboard Write

Copy behavior:

- Text entry writes text to clipboard.
- Image entry writes image bytes to clipboard with correct MIME where possible.
- Copy increments usage count.
- Copy updates last-used timestamp.
- Copy records paste sentinel if paste-back may trigger self-recapture.

### 13.2 Foreground Tracking

Keep a platform-agnostic foreground snapshot:

```json
{
  "id_kind": "hypr | wlr | win32 | x11 | macos | unknown",
  "process_basename": "firefox",
  "title": "Window title",
  "captured_at": "timestamp",
  "restore_supported": true
}
```

Backends to support or plan:

- Hyprland:
  - snapshot via compositor IPC.
  - restore by window address.
- wlroots/Sway:
  - snapshot via wlr-foreign-toplevel where available.
  - restore through activate request, compositor may ignore.
- GNOME Wayland:
  - degraded, usually no foreground protocol.
- Windows:
  - planned foreground window handle and process basename.
- macOS:
  - planned active app/pid via native APIs and accessibility permissions.
- X11:
  - possible future support.

### 13.3 Keystroke Model

Keep a typed parser for per-app paste keys.

Examples:

- `ctrl+v`
- `ctrl+shift+v`
- `shift+insert`
- `enter`
- `ctrl+enter`
- `"+gp` for Vim register paste.

Rules:

- Sequence is one or more chords separated by whitespace.
- Modifiers: ctrl, shift, alt, super.
- Special keys: enter, tab, escape, space, backspace, delete, insert, home,
  end, pageup, pagedown, arrows, f1-f24.
- If a token starts with a modifier plus `+`, parse as chord.
- Otherwise parse each character as a literal chord.
- Comparisons are case-insensitive for key names.

### 13.4 Synthesizer Chain

Linux behavior to keep:

- Hyprland:
  - first try compositor-specific shortcut dispatch.
- wlroots/KDE:
  - try `wtype`.
- fallback:
  - try `ydotool`.
- final:
  - `off`, meaning clipboard was written but user must paste manually.

Known improvements:

- Enforce subprocess timeout.
- Avoid one process spawn per chord where possible.
- Prefer a backend that supports Unicode literal input.
- Return structured outcome to UI.

Suggested copy-and-paste response:

```json
{
  "copied": true,
  "foreground_restore": "attempted",
  "paste_synthesizer": "wtype",
  "paste_succeeded": true,
  "manual_paste_required": false,
  "message": "Pasted into firefox"
}
```

## 14. Configuration

Use a human-editable config file. TOML worked well, but the format can change
if the new stack has a better standard. The schema should be explicit.

Core config sections:

- general:
  - max entries.
  - delete-after retention for non-pinned entries, with `0` meaning disabled.
  - poll interval.
- storage:
  - data directory override.
- ui:
  - preview defaults.
  - date format.
  - theme.
  - image preview behavior.
- keybindings:
  - action-to-key mapping.
- capture:
  - mode.
  - size caps.
  - include/exclude formats.
  - excluded processes.
- paste:
  - disabled flag.
  - synthesizer chain override.
  - per-app keystrokes.
  - sentinel TTL.
  - rapid-refire window.
- actions:
  - URL templates.
- sync:
  - enabled flag.
  - port range.
  - display name.
  - digest limit.

Config behavior:

- Load once on command startup for stdio bridge.
- For daemon mode, support reload or restart.
- Unknown fields should either be preserved or clearly warned about. Do not
  silently discard user config if the app writes config back to disk.
- Path expansion should support `~`, `$VAR`, and `${VAR}`.

## 15. Filters, Transforms, and Scripts

### 15.1 Transforms

Text transforms that already proved useful:

- plain text only.
- upper case.
- lower case.
- title case.
- sentence case.
- invert case.
- camel case.
- pascal case.
- snake case.
- kebab case.
- slugify.
- remove line feeds.
- add line feed.
- trim whitespace.
- collapse whitespace.
- prepend date/time.
- append date/time.
- insert GUID/UUID.
- POSIX path conversion.
- ASCII only.
- typoglycemia.

Rules:

- Original entry is never mutated.
- Transform output is copied or pasted for the current operation.
- Transform IDs should be stable kebab-case strings.

### 15.2 Filter Rules

Keep filter rules because they solve capture hygiene without requiring scripts.

Actions:

- drop.
- transform with a transform id.
- tag with a tag name.

Important behavior:

- Drop rules must not advance watcher dedup state.
- Transform rules apply only to text content.
- Image clips that match text rules should be captured as-is or skipped
  according to explicit rule semantics.
- Rule errors should be logged and skipped, not crash the watcher.

### 15.3 Scripts

Scripting is optional in the fresh product.

What worked:

- Capture scripts can mutate text.
- Capture scripts can return `drop`.
- Operation, string, call-level, array, and map limits prevent runaway scripts.
- Dangerous dynamic operations like import/eval can be disabled.

Reasons to defer:

- Increases support surface.
- Harder to make portable.
- Filter rules may cover most automation needs.

If kept:

- Scripts run only in backend.
- Scripts never run in UI process.
- Scripts must be sandboxed and bounded.
- Script errors should not crash watcher.

## 16. Sync

Sync is valuable but should not block the first rebuild.

Existing design worth keeping:

- Local identity with ed25519 keypair.
- Public-key fingerprint for display/trust.
- mDNS service discovery.
- Trust states:
  - untrusted.
  - trusted/pinned.
  - rejected.
- Noise XX encrypted transport.
- Identity proof binds long-term ed25519 identity to Noise static key.
- Pull-based sync from trusted peers.
- Entry digests avoid transferring everything.
- Entry payload includes:
  - entry metadata.
  - formats.
  - notes.
  - collection.
  - tags.
  - image blob chunks.
- Blob chunks around 64 KiB.
- Sync log table.

Known issue:

- The name `auto_send` is confusing if behavior is actually auto-pull. Rename
  before reimplementing.

Fresh-start guidance:

- Build sync after storage and bridge contracts are stable.
- Treat sync protocol as versioned.
- Keep trust explicit.
- Do not enable sync by default.

## 17. Platform Support

### 17.1 Linux Wayland

First-class initial target.

Expected support:

- Text capture.
- Image capture.
- Multi-format capture.
- Clipboard write.
- Terminal UI.
- Hyprland foreground tracking and paste-back.
- Sway/wlroots foreground tracking where protocol exists.
- wtype/ydotool fallback.
- systemd user service.
- Nix/Home Manager or equivalent packaging if desired.

Known degradation:

- GNOME Wayland usually lacks the foreground protocol needed for reliable
  paste-back targeting. Capture and manual paste still work.

### 17.2 Windows

Planned.

Needed:

- Event-driven capture with `AddClipboardFormatListener`.
- Enumerate Win32 clipboard formats.
- Map formats to canonical names.
- Use native APIs for clipboard write where needed.
- Foreground tracker using focused window handle and process name.
- Paste synthesis with `SendInput`.
- Stuck-modifier protection.
- Clear permission/error messages.

### 17.3 macOS

Planned.

Needed:

- NSPasteboard capture/write.
- Text, images, rich text, file URLs.
- Accessibility permission flow for paste-back.
- Active app tracking.
- Homebrew or equivalent packaging.
- Clear permission/error messages.

## 18. CLI and Automation

Keep a scriptable command surface independent of the UI.

Human CLI commands:

- watch start/stop/status.
- list.
- get.
- search.
- copy.
- save current clipboard.
- delete.
- favorite.
- clear.
- count.
- status.
- stats.
- repair.
- transform.
- rules.
- collections.
- tags.
- sync.

Machine API commands:

- Prefer stable JSON methods over human output.
- Keep all backend methods accessible from CLI for testability.
- Avoid requiring a terminal UI for any core operation.

Exit-code rules:

- 0 on success.
- 1 on runtime/user-visible failure.
- 2 on invalid usage.
- Repair should return 0 when it successfully applied fixes; non-zero only on
  unrecoverable errors.

## 19. What Worked

Keep these decisions:

- Terminal-first product direction.
- Native backend for platform behavior.
- SQLite with FTS5.
- Content-addressed image blobs.
- Atomic image writes.
- Persistent prune queue.
- Image-over-text capture priority.
- Multi-format clip model.
- Conservative canonicalization.
- Watcher pipeline ordering.
- Startup dedup priming.
- Capture source abstraction.
- Foreground tracker abstraction.
- Paste sentinel.
- Per-app keystroke overrides.
- Transform registry.
- Filter rules.
- Explicit repair command.
- Scriptable CLI.
- Tests using isolated temp data directories.
- JSON output for automation.

## 20. What Did Not Work

Do not repeat these:

- Desktop GUI as product direction.
- Tray icon as core UX.
- Layer-shell window experiments as primary workflow.
- Frontend direct database access.
- Monolithic storage module.
- Historical phase comments embedded in long-lived code.
- Mixing UI state, DB access, search, and clipboard operations in one app
  state object.
- Building packaging around removed desktop assets.
- Letting documentation describe future work as if it is current behavior.

## 21. Recommended Module Boundaries

Use names appropriate to the chosen language, but keep these boundaries.

Backend:

- `api`: command envelope, method routing, response/error types.
- `config`: load, validate, save, path expansion.
- `storage/connection`: database open and pragmas.
- `storage/schema`: fresh schema.
- `storage/entries`: entry CRUD and pagination.
- `storage/formats`: format rows.
- `storage/search`: FTS and search routing.
- `storage/blobs`: image blob paths, atomic writes, prune queue.
- `storage/collections`: collections.
- `storage/tags`: tags.
- `storage/rules`: filter rule persistence.
- `storage/sync`: peer and sync log persistence.
- `storage/repair`: repair operations.
- `capture`: raw clip model and capture source interface.
- `capture/platform`: OS-specific capture backends.
- `clipboard`: OS clipboard write/read helpers.
- `formats`: canonical format IDs and canonicalization.
- `watcher`: capture policy and daemon loop.
- `foreground`: foreground tracker interface.
- `paste`: keystroke parser, synthesizers, sentinel, cursor.
- `transforms`: transform registry.
- `rules`: filter rule engine.
- `sync`: protocol and runtime.
- `logging`: structured logs.

Frontend:

- `bridge`: API client and schemas.
- `state`: UI stores/state machines.
- `keymap`: key parsing and action resolution.
- `components`: terminal components.
- `theme`: visual tokens.
- `preview`: text/image preview rendering.
- `commands`: command palette actions.

## 22. Testing Strategy

Backend tests:

- schema creation.
- old database import if supported.
- entry CRUD.
- pagination.
- FTS search.
- format canonicalization.
- image store atomic write behavior.
- prune queue.
- repair dry-run/apply.
- capture policy.
- watcher dedup.
- filter rules.
- transforms.
- paste keystroke parser.
- synthesizer command generation.
- foreground filtering.
- sync protocol frames.
- sync identity verification.
- API contract fixtures.

Frontend tests:

- bridge client handles success/error.
- keymap resolves actions.
- search mode state.
- selection state.
- multi-select state.
- dialog state.
- rendering snapshots if the chosen stack supports them.

Integration tests:

- CLI/API list/search/copy with temp database.
- watcher with mock capture source.
- repair against temp blob tree.
- terminal UI smoke test with seeded backend.
- live platform tests gated behind environment variables.

Manual live tests:

- Wayland text capture.
- Wayland browser copy-image.
- image preview fallback.
- Hyprland paste-back.
- Sway/wlroots paste-back.
- GNOME degraded behavior.
- suspend/resume watcher behavior.

## 23. Fresh MVP Milestones

### Milestone 0: Contract Skeleton

Build:

- empty backend.
- command envelope.
- `health.check`.
- empty terminal shell.
- frontend bridge client.
- contract fixture tests.

Exit:

- UI starts.
- UI can call backend and render health result.

### Milestone 1: Storage and Read-Only UI

Build:

- fresh schema.
- seed command.
- `entries.list`.
- `entries.get`.
- `entries.search`.
- list UI.
- search bar.
- preview pane.
- status bar.

Exit:

- UI browses seeded entries.
- UI searches seeded entries.
- UI previews text.

### Milestone 2: Clipboard Write

Build:

- `entries.copy`.
- text clipboard write.
- image clipboard write.
- usage count.
- last-used update.
- paste sentinel write.

Exit:

- User can copy from history into OS clipboard.

### Milestone 3: Watcher

Build:

- Wayland capture backend.
- watcher loop.
- dedup.
- image priority.
- format policy.
- blob writes.
- cleanup.
- watcher status.
- UI refresh.

Exit:

- New clipboard text/image entries appear in terminal UI.

### Milestone 4: Management

Build:

- delete.
- clear.
- favorite.
- notes.
- tags.
- collections.
- multi-select.

Exit:

- Daily-use history management works.

### Milestone 5: Paste-Back

Build:

- foreground tracker.
- keystroke parser.
- Linux synthesizer chain.
- structured paste outcome.
- degraded mode messages.

Exit:

- Copy-and-paste-back works on primary Linux target.
- Manual fallback is clear elsewhere.

### Milestone 6: Power Features

Build:

- transforms.
- filter rules.
- URL templates.
- aggregation/merge.
- stats.
- repair UI/API.
- optional scripts.

Exit:

- Advanced old-product workflows are covered.

### Milestone 7: Sync

Build:

- identity.
- discovery.
- trust states.
- encrypted transport.
- pull/import.
- sync status/log UI.

Exit:

- Trusted peer sync works.

### Milestone 8: Ports and Packaging

Build:

- Linux packages.
- Windows capture/write.
- Windows paste-back.
- macOS capture/write.
- macOS paste-back.
- optional localization.

Exit:

- Supported platforms have tested terminal-first installs.

## 24. Import From Old Project

If old data matters, write an importer. Do not keep old migrations in the main
fresh schema.

Importer requirements:

- Read old entries.
- Read old entry formats.
- Read old image blob references.
- Verify image file existence.
- Copy blobs into new content-addressed store.
- Import collections.
- Import tags.
- Import notes.
- Import favorites.
- Import usage counts.
- Import peers only if sync protocol remains compatible.
- Produce a report:
  - imported entries.
  - skipped duplicates.
  - missing blobs.
  - quarantined blobs.
  - unsupported records.

## 25. Open Questions

Decide before implementation:

- Which terminal UI stack is first?
- Is the first bridge stdio or daemon socket?
- Is old database import required?
- Is scripting part of v1 or deferred?
- Is sync pull-based, push-based, or both?
- Does the frontend render images, or does backend provide terminal-ready image
  previews?
- Should config writes preserve comments/unknown fields?
- Are global hotkeys in scope for terminal-only product, or only compositor/user
  config?
- Is Windows/macOS support part of v1 or post-v1?

## 26. First Commit Sequence

Recommended order:

1. Create fresh repository/workspace.
2. Add backend command envelope.
3. Add frontend bridge client.
4. Add `health.check` fixture test.
5. Add storage schema v1.
6. Add seed data command.
7. Add `entries.list`.
8. Render terminal list.
9. Add `entries.get`.
10. Render preview.
11. Add `entries.search`.
12. Add `entries.copy`.
13. Add watcher.
14. Add image blobs.
15. Add delete/favorite/notes.
16. Add tags/collections.
17. Add paste-back.
18. Add repair.
19. Add transforms/filter rules.
20. Add sync.

## 27. Non-Negotiable Invariants

- UI never opens the database directly.
- Image blobs are content-addressed.
- Image writes happen only after dedup says the entry is new.
- Deleting image rows queues blob prune transactionally.
- Watcher never crashes because one format is unreadable.
- Security exclusions do not advance dedup state.
- Drop rules do not advance dedup state.
- Format names are stable and backend-owned.
- CLI/API output used by machines is JSON.
- Desktop GUI code does not return.

## 28. Final Summary

The fresh project should keep the hard-won native behavior and discard the old
frontend shape.

Keep:

- capture policy.
- storage lessons.
- image blob protocol.
- format canonicalization.
- watcher ordering.
- search model.
- paste-back abstractions.
- repair logic.
- scriptable command surface.

Change:

- make the frontend stack replaceable.
- isolate native behavior behind a stable API.
- split storage into focused modules.
- start with a fresh schema.
- rebuild UI behavior with explicit state machines.

Build the next version around a small, stable backend contract. The terminal UI
can change; the native contract and data invariants should not.
