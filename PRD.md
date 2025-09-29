# Ditox Clipboard — Product Requirements Document (v0.1.0)

## Overview
- Working title: Ditox (enterprise‑friendly clipboard history for developers and teams).
- Vision: a secure, fast, cross‑platform clipboard history with powerful search, favorites, and optional sync — designed for Linux first, then macOS and Windows.
- v0.1.0 scope: Linux (X11 + Wayland) and NixOS only. Core library and CLI; no GUI yet. Text content only; images and rich text are out of scope for this first release.

## Goals (v0.1.0)
- Reliable local capture of text clipboard history on Linux (X11 and Wayland) with pragmatic fallbacks.
- Durable local storage with efficient search and simple favorites.
- Clean, scriptable CLI for add/list/search/copy/favorite/export.
- Enterprise‑minded defaults: explicit opt‑in for sync, privacy controls, clear data model, and testable architecture.

## Non‑Goals (v0.1.0)
- No GUI, tray, or hotkeys pop‑up UI.
- No cross‑device sync by default; optional remote integration will be prototyped behind a feature flag.
- No binary attachments (images, files) or formatting; text only.

---

## Competitive Landscape (summary)
What established clipboard managers do well:
- CopyQ (Linux/macOS/Windows): searchable history, favorites/pins, rules, commands, scripting, images, tabs, and sync options.
- Ditto (Windows): robust history, favorites, search, encrypted sync over LAN/TCP, groups, and hotkeys.
- Maccy (macOS): super‑fast, minimal UI, fuzzy search, pins, and shortcuts.

Linux platform constraints that shape our design:
- Wayland intentionally limits global clipboard listeners; capturing reliably often requires compositor protocols such as `wlr-data-control` or external helpers (e.g., `wl-clipboard`). X11 is permissive and supports selection change events.

References (for deeper reading; see end notes).

---

## Users and Key Jobs
- Developers/SREs: keep recent commands, tokens, or snippets; search/paste quickly; pin critical items.
- Security‑sensitive users: retain history locally with control and auditability; disable capturing from specific apps or patterns.
- Ops/IT: package and deploy a predictable, auditable binary; configure retention and exclusions.

---

## Core Experience (CLI‑first)
- `ditox add [TEXT]` — manually capture current text (or from STDIN).
- `ditox list` — show recent items (id, age, preview, favorite flag).
- `ditox search <query>` — substring and optional FTS search; filter by favorites.
- `ditox copy <id>` — copy an entry back to the system clipboard.
- `ditox favorite <id>` / `ditox unfavorite <id>` — toggle pin.
- `ditox export --format json|ndjson|csv` — dump items for backup or audit.
- `ditox prune --max-items N --max-age 30d` — retention management.
- `ditox init-db` — initialize local store.
- `ditox doctor` — self‑check for X11/Wayland helpers and database features.

Notes
- Output can be `--json` for scripting.
- Consider `--fzf`/`--interactive` in a later minor to launch an interactive picker via `skim`/`fzf`.

---

## Architecture
- Workspace with two crates:
  - `ditox-core` (lib): domain types, storage abstraction, search, clipboard bridges, and (later) sync.
  - `ditox-cli` (bin): user interface; depends only on the core.
- OS adapters behind traits and feature flags (`x11`, `wayland`, `macos`, `windows`).
- Linux capture modes (v0.1.0):
  - X11 watcher: event‑driven when available.
  - Wayland: helper‑based (e.g., `wl-clipboard`) or polling fallback; explicit and documented limitations.
- Isolation: the core must be usable from future GUI/daemon crates without change.

### Clipboard Access (Linux v0.1.0)
- X11: use `x11-clipboard` or `copypasta` backends for read/write and event‑based change detection when possible.
- Wayland: prefer `wl-clipboard`/`wl-clipboard-rs` or the `wlr-data-control` protocol where supported; otherwise provide manual capture (`ditox add` and `ditox copy`) as a consistent fallback.

### Data Model (text‑only)
- `clip` table (or collection):
  - `id` (string/UUID ULID)
  - `text` (TEXT)
  - `created_at` (timestamp)
  - `is_favorite` (bool, default false)
  - `deleted_at` (nullable) — support soft delete and future sync/conflict resolution
- Indices:
  - `(created_at DESC)` for recency
  - Full‑text index on `text` (FTS5) for fast search

### Storage Options
- Baseline: SQLite for reliability, indexing, and future FTS. It scales better than a JSON file and avoids whole‑file rewrites.
- JSON (ndjson) for export/import only, not primary storage.
- Remote: libSQL/Turso for optional sync — online/offline friendly replication while keeping SQLite semantics.

Trade‑offs
- SQLite is simple to ship and battle‑tested; FTS5 gives good substring/fuzzy support. A raw JSON file is fragile for concurrent writes and search.
- For enterprises, SQLite also eases audit/backup, and enables SQL migrations.

### Search
- Default: substring/LIKE plus optional FTS5 virtual table for fast searches.
- Future: add ranking, fuzzy matching, and tag filters.

### Sync (behind a feature flag, off by default)
- libSQL/Turso replicator (opt‑in):
  - Local first: operate on local SQLite/libSQL; background sync when configured.
  - Encryption: recommend end‑to‑end via application‑level encryption (AEAD) on `text` column before it hits remote; or SQLCipher locally.
  - Conflict policy: last‑writer‑wins per item with timestamps; server time drift mitigated by monotonic sequence per device.
- Enterprise: document how to self‑host libSQL or run Turso; provide migration path from pure SQLite.

### Privacy & Security (v0.1.0)
- Defaults: text only; opt‑in to capture; redact via ignore rules (regex/domain/process name when available).
- Commands: `prune`, `clear`, `export`; explicit retention caps (by age/count).
- Telemetry: none by default.

---

## Packaging & Distribution (v0.1.0)
- Linux:
  - Static binary or musl build when feasible; otherwise glibc minimal.
  - Provide `.deb` and `.rpm` in CI; consider AppImage.
  - Systemd user unit template for a future daemon (`clipd`) but not required in 0.1.
- NixOS:
  - Provide a Nix derivation and optional flake for `ditox-cli`.

---

## Roadmap
- 0.1.0 (this release)
  - Core library with storage trait and SQLite backend
  - CLI: add/list/search/favorite/copy/export/prune/init-db/doctor
  - Linux support (X11 + Wayland helpers); text only
- 0.2.x
  - Daemon (`clipd`) with event listeners and configurable rules
  - Interactive picker (`--fzf`) and tags
  - Basic import from common managers (CopyQ JSON, Ditto CSV)
  - Images: capture/store images, thumbnails, dedupe, and copy-out
- 0.3.x
  - Optional sync (libSQL/Turso) + E2EE
  - Multi‑device merge/conflict policy and device IDs
- 0.4.x
  - macOS and Windows OS adapters and packages
  - Images/rich text (depending on demand)

---

## Open Questions / Risks
- Wayland clipboard capture parity varies by compositor; we’ll document exact support and ship reliable fallbacks (manual capture + polling).
- If FTS5 is unavailable (older SQLite), search falls back to LIKE; `doctor` will surface capability.
- Performance for very large histories: we’ll default to a max history size (e.g., 10k items) and prune strategy.
- Enterprise encryption policies differ: provide E2EE at the app layer to avoid dependence on platform crypto.

---

## References
- CopyQ features: https://copyq.readthedocs.io/en/latest/ and https://github.com/hluk/CopyQ
- Ditto features: https://sourceforge.net/projects/ditto-cp/
- Maccy (macOS) features: https://maccy.app/ and https://github.com/p0deje/Maccy
- Wayland data‑control protocol: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/staging/data-control/data-control-v1.xml
- wl-clipboard-rs crate: https://crates.io/crates/wl-clipboard-rs
- arboard and copypasta crates: https://crates.io/crates/arboard, https://crates.io/crates/copypasta
- SQLite FTS5: https://www.sqlite.org/fts5.html
- Turso/libSQL: https://turso.tech/ and https://docs.turso.tech/libsql/overview

---

## Images (planned for v0.2)
### Goals
- Capture images from the system clipboard and store them efficiently.
- Support favorites, metadata search (format/size/dimensions), and copy‑out back to clipboard.
- Keep DB lean by storing image bytes as content‑addressed files with metadata in SQLite.

### Design
- Clipboard API: use `arboard` for cross‑platform image read/write (RGBA in, PNG/WebP out).
- Storage layout:
  - Table `clips` gains `kind` (Text|Image).
  - Table `images` keyed by `clip_id` with fields: `format`, `width`, `height`, `size_bytes`, `sha256`, `thumb_path`.
  - Blob store in `objects/aa/bb/<sha256>`; optional per‑file AEAD encryption.
- Dedupe: exact via SHA‑256; optional perceptual hash (pHash) for near‑dupes.
- Thumbnails: small PNG/WebP for fast rendering in future UI.
- Search:
  - Text clips: FTS5.
  - Image clips: metadata filters (format/size/dimensions/favorites/date) and optional similarity via pHash.

### CLI Additions
- `ditox list --images` and `ditox info <id>` show image metadata.
- `ditox export --images` exports metadata + blobs.
- `ditox prune --images` respects size/age quotas.

### Sync Considerations
- Sync only metadata and blob refs by default; optionally sync blobs to object storage (S3/MinIO). Small images can be stored as DB BLOBs if configured.

