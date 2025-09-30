# Ditox Clipboard — Product Requirements Document (as of 2025‑09‑29)

## Overview

- Working title: Ditox (enterprise‑friendly clipboard history for developers and teams).
- Vision: a secure, fast, cross‑platform clipboard history with powerful search, favorites, and optional sync — designed for Linux first, then macOS and Windows.
- Scope (current): Linux‑first CLI with a reusable core library. Text clips are fully supported; image clips are implemented locally (store, list, copy). Optional remote sync (libSQL/Turso) is available for text when built with the `libsql` feature and is selected at runtime via settings; images remain local‑only.

## Goals (v0.1.x)

- Reliable local clipboard history on Linux with pragmatic fallbacks.
- Durable local storage with efficient search and simple favorites.
- Clean, scriptable CLI for add/list/search/copy/favorite/prune/migrate/config.
- Enterprise‑minded defaults: explicit opt‑in for sync, privacy controls, clear data model, SQL migrations, and tests.

## Non‑Goals (v0.1.x)

- No GUI, tray, or hotkeys pop‑up UI.
- No cross‑device sync by default; remote is opt‑in (runtime selection).
- No cross‑device image sync yet; images are local‑only.

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

- `ditox add [TEXT]` — add text (argument or STDIN). Images: `--image-path <file>` or `--image-from-clipboard` (Linux).
- `ditox list` — list text; `--images` lists image clips; `--json` for scripting; `--favorites` filter.
- `ditox search <query>` — substring or FTS5 (when available); `--json` optional.
- `ditox copy <id>` — put text or image back on the system clipboard (Linux adapter).
- `ditox favorite <id>` / `ditox unfavorite <id>` — toggle pin.
- `ditox prune [--max-items N] [--max-age 30d] [--keep-favorites]` — retention.
- `ditox migrate --status|--backup` — SQL migrations with optional on‑disk backup.
- `ditox config [--json]` — show effective configuration and paths.
- `ditox init-db` — initialize local store.
- `ditox doctor` — environment/store check (clipboard probes, FTS capability). Wayland‑specific tests (e.g., `wl-clipboard`) only run when Wayland is detected to avoid blocking headless CI.
- `ditox export <dir> [--favorites] [--images] [--tag <t>]` — write JSONL + image blobs to directory.
- `ditox import <dir|clips.jsonl> [--keep-ids]` — import previously exported data.

Notes

- Output can be `--json` for scripting.
- Consider `--fzf`/`--interactive` in a later minor to launch an interactive picker via `skim`/`fzf`.

---

## Architecture

- Workspace with two crates:
    - `crates/ditox-core` (lib): domain types, storage abstraction, search, clipboard bridges, migrations, local image blob store, optional sync engine.
    - `crates/ditox-cli` (bin): thin CLI; depends on the core only.
- OS clipboard adapters behind features; Linux uses `arboard` today. Non‑Linux builds stub clipboard IO.
- Isolation: the core is suitable for future GUI/daemon crates.

### Clipboard Access (Linux)

- Adapter based on `arboard` for read/write text and images.
- Manual capture (`ditox add`, `ditox copy`) provides a consistent path even where watchers are restricted.

### Data Model

- `clips` table
    - `id` TEXT primary key (sortable hex id; ULID planned)
    - `kind` TEXT ('text'|'image')
    - `text` TEXT
    - `created_at` INTEGER (unix seconds)
    - `last_used_at` INTEGER NULL (unix seconds; updated when selected/copied)
    - `is_favorite` INTEGER (0/1)
    - `deleted_at` INTEGER NULL
    - Image columns: `is_image` INTEGER (0/1), `image_path` TEXT NULL
    - Sync columns: `updated_at` INTEGER NULL, `lamport` INTEGER DEFAULT 0, `device_id` TEXT NULL
- `images` table keyed by `clip_id` with `format`, `width`, `height`, `size_bytes`, `sha256`, `thumb_path`.
- Indices: `(created_at DESC)`, FTS5 virtual table `clips_fts(text)` when available.

### Storage Options

- Baseline: SQLite with WAL; embedded SQL migrations.
- Search: FTS5 when present; LIKE fallback otherwise.
- Remote (opt‑in, feature‑gated): libSQL/Turso for text rows.

### Export/Import Layout

- Export writes `clips.jsonl` and, for images, content‑addressed blobs at `objects/aa/bb/<sha256>` under the chosen export directory.
- Import accepts either the export directory or `clips.jsonl` (with `objects/` next to it). Image round‑trip relies on the blob files being present under `objects/` using the real sha256.

---

## Quality & Testing Strategy (v0.1.x)

- Scope
  - Black‑box CLI tests (primary), focused core tests (SQLite store, migrations, image blob path), and migration status/backup.
- Harness
  - Each test uses a temp SQLite DB and isolated `XDG_CONFIG_HOME`; no shared state. Clipboard is never required; Linux E2E image copy is gated by `DITOX_E2E_CLIPBOARD=1`.
  - Tests force the local SQLite backend (`--store sqlite`) to avoid accidental remote/libsql interference.
- Budgets
  - Suite runtime ≤ 15s on a dev laptop; practical runs ~5s.
  - No flaky tests; `doctor` probes clipboard tools only when appropriate (e.g., Wayland session).
- Coverage highlights
  - CLI: add/list/search/copy/favorite/unfavorite/delete/info/prune/migrate/doctor/config/export/import.
  - Core: insert/list/search, favorites, last_used_at recency, image meta + blob round‑trip, migration versioning.
  - Migrations: `--status`, `--backup` creates `ditox.bak.<yyyyMMddHHmmss>` next to the DB; idempotent apply.
  - Export/Import: images export using real sha256; import resolves `objects/aa/bb/<sha256>`.

Trade‑offs

- SQLite is simple to ship and battle‑tested; FTS5 gives good substring/fuzzy support. A raw JSON file is fragile for concurrent writes and search.
- For enterprises, SQLite also eases audit/backup, and enables SQL migrations.

### Search

- Substring/LIKE and optional FTS5; auto‑rebuild FTS on first migration.

### Sync (runtime‑selectable; off by default)

- Engine: local‑first with push/pull to libSQL/Turso when configured.
- Scope today: text only. Image rows are excluded from remote writes.
- Conflict policy: last‑writer‑wins using tuple `(lamport, updated_at, device_id)`.
- Status: `sync status` reports last push/pull, pending local rows, local text/image counts, remote reachability, last error.

### Privacy & Security

- Clipboard contents may include secrets; CLI avoids logging content and supports pruning/export via JSON output from commands.
- Remote sync is opt‑in and feature‑gated; settings are local TOML; scripts harden perms to 0600 when possible.
- No telemetry.

---

## Packaging & Distribution

- Linux: build via Cargo; CI builds release binaries and uploads artifacts on tags `rust-v*`.
- NixOS: flake with `packages.ditox`, `apps.ditox`, and a dev shell (Rust + headers). CI also exercises flake builds.
- Systemd: user timers for prune/sync provided via `scripts/install_*_timer.sh` and `contrib/systemd` templates.

---

## Roadmap

- 0.1.x (current)
    - Core library (SQLite + migrations + FTS5 when available)
    - CLI: add/list/search/favorite/unfavorite/copy/delete/info/prune/init-db/doctor/migrate/config
    - Images: local store/list/info/copy; content‑addressed blobs on disk
    - Sync (feature‑gated): text push/pull; status reporting; systemd timer installers
- 0.2.x
    - Optional background daemon (`clipd`) and interactive picker (maybe we could make a TUI)
    - Import/export tools; richer search; tags
    - Image thumbnails and metadata filters (maybe we could save the image filename, as a searchable field)
- 0.3.x
    - Multi‑device merge improvements and conflict tooling
- 0.4.x
    - macOS and Windows adapters and packages

---

## Open Questions / Risks

- Clipboard watcher ergonomics vary on Wayland; manual capture keeps flows reliable.
- Older SQLite builds may lack FTS5; LIKE fallback remains acceptable for small/medium datasets.
- Large histories: prune policies and limits (defaults in settings) mitigate growth.
- Remote: text only today; images remain local‑only until a safe/object‑store strategy is defined.

---

## References

- CopyQ, Ditto, Maccy (feature prior art)
- Wayland data‑control protocol, wl‑clipboard‑rs, arboard/copy‑pasta
- SQLite FTS5 docs, Turso/libSQL docs

---

## Images (current)

### Goals

- Capture images from file or system clipboard; store locally; copy back to clipboard.
- Keep DB lean: store image bytes as content‑addressed PNG blobs under the DB directory; metadata in SQLite.

### Design

- Clipboard API: `arboard` on Linux for image read/write (RGBA in, PNG out).
- Storage layout:
    - `clips.kind = 'image'`; flags `is_image` and optional `image_path` (when using file‑path mode).
    - `images(clip_id, format, width, height, size_bytes, sha256, thumb_path)`.
    - Blob store path: `objects/aa/bb/<sha256>` adjacent to the DB file.
- Dedupe: exact via SHA‑256 digest of encoded PNG bytes.

### CLI

- Add from file: `ditox add --image-path <file>`
- Add from clipboard (Linux): `ditox add --image-from-clipboard`
- List images: `ditox list --images [--json]`
- Inspect: `ditox info <id>`
- Copy back: `ditox copy <id>`

### Sync Considerations

- Images are local‑only and excluded from remote sync by design at this stage.
