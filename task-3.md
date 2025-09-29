# Task 3 — 0.2.x Roadmap Features

This task groups the 0.2.x items from PRD.md into concrete deliverables, API/CLI changes, tests, and roll‑out steps. It follows the repo guidelines (fmt, clippy, tests, migrations naming, and CLI testing).

Scope is Linux first. All items are optional at runtime: users can keep the current CLI‑only workflow.

---

## Goals

- Optional background daemon `clipd` to watch the system clipboard, run prune/sync jobs, and serve fast local queries via IPC.
- Interactive picker integrated with CLI (fuzzy search + copy‑on‑select), with image preview/thumbnail support.
- Import/Export tools for full or filtered history (text and images).
- Richer search: FTS5 ranking, operators, favorites/images filters, and (later) tag filters.
- Tags: schema + CRUD, list/search by tags, export/import tags.
- Image thumbnails and metadata filters (width/height/format/size) usable in CLI and picker.

Non‑Goals: macOS/Windows adapters (tracked for 0.4.x), remote image sync, end‑to‑end encryption for images.

---

## Milestones

1) Clipd (daemon) foundation
2) Interactive picker MVP
3) Search upgrades (FTS rank + filters)
4) Tags schema + CLI
5) Import/Export tools
6) Thumbnails + image filters + picker preview

Release each milestone behind feature flags or CLI switches; keep backward‑compatible migrations and config.

---

## 1) Clipd (Daemon)

Design
- Binary name: `clipd` (crate `crates/ditox-clipd`).
- IPC: Unix domain socket at `${XDG_RUNTIME_DIR}/ditox/clipd.sock` (fallback to `${TMPDIR}`), JSON messages (serde). Simple request/response; optional subscribe stream later.
- Responsibilities:
  - Clipboard watch (Linux: `arboard` polling or Wayland‑friendly strategies) for text + images.
  - Background jobs: `prune` and optional `sync` at configured intervals.
  - Query service for `list/search/info` to reduce startup cost for the CLI/picker.
- Security: socket `0700` dir, `0600` socket; no remote access. Redact payloads in logs. Do not persist secrets.

CLI/Config
- `ditox config` gains `daemon` settings (intervals, socket path override, watch enable, max CPU/mem hints).
- `ditox doctor` reports: socket reachability, version match, watcher capability.
- Systemd user templates placed under `contrib/systemd/` and install scripts under `scripts/`:
  - `clipd.service` (Restart=on-failure), `clipd.socket` optional, `clipd-prune.timer`, `clipd-sync.timer`.

Acceptance
- Start/stop `clipd`; creating and deleting socket; healthy status endpoint returns JSON.
- When running, new clipboard items appear in `ditox list` within N seconds (configurable, default 1–2s).
- Background prune/sync runs at configured cadence; manual `ditox prune` is still supported.

Testing
- Black‑box CLI tests that speak JSON over a temporary UDS path.
- Fallback when daemon is down: CLI continues to hit SQLite directly.

---

## 2) Interactive Picker (TUI‑first MVP)

Design
- Command: `ditox pick` (new) or `ditox list --interactive`.
- Built‑in TUI (no external deps): `ratatui` + `crossterm`.
- Data source: by default via `clipd` IPC (paged); fallback to direct store when daemon is absent.
- Layout: left = list (virtualized), top bar = query + filters, right = preview pane (text preview, image info; later: ascii thumbnail).
- Behavior: type to filter; Up/Down or j/k to navigate; Enter copies and exits; Esc/Ctrl‑c cancels; Tab toggles filters.
- Large sets: incremental fuzzy match (top‑N) with background worker using `fuzzy-matcher` (SkimMatcherV2) when FTS is not used; FTS query path for advanced ops when available.

Keybindings
- Up/Down or j/k = move selection
- PgUp/PgDn = page
- Home/End = jump
- Enter = copy + exit
- Esc/Ctrl‑c = cancel
- Ctrl‑f = toggle favorites filter
- Ctrl‑i = images mode
- Ctrl‑t = filter by tag (opens small prompt)
- Ctrl‑r = toggle rank by FTS `bm25` when available

Flags
- `--images`, `--favorites`, `--limit N`, `--tag TAG`, `--no-daemon` (bypass IPC), `--fts` (force FTS query path), `--no-fuzzy` (disable fuzzy and use substring).

Acceptance
- Selecting an item performs `copy` and exits with code 0; cancel exits with code 130.
- Smooth interaction at 10k text clips on a typical laptop terminal (virtualized list, under ~30ms/frame).
- Works with both text and images; respects favorites and tag filters; degrades gracefully without FTS.

Testing
- Decouple input via an `EventSource` trait; provide a test implementation that feeds key sequences. Spawn the app in a headless mode and assert the copied id printed to stdout.
- Unit test fuzzy top‑N ranking; integration tests for filter toggles and cancel path.
- Snapshot small render regions via a terminal adapter trait (golden tests) without relying on actual terminal capabilities.

---

## 3) Richer Search

Design
- Continue using FTS5 (`clips_fts`) when available; fallback to `LIKE`.
- Add ranking: expose `bm25` rank in output and sort by it when `--rank` is chosen.
- Operators: phrase `"..."`, AND/OR, `*` suffix for prefix queries (FTS5), `tag:foo`, `is:image`, `is:fav`.
- CLI: `ditox search <query> [--json] [--limit N] [--rank] [--images] [--favorites] [--tag TAG]`.

Acceptance
- When FTS is present: ranking is stable and top‑N differs from naive recency where appropriate.
- With fallback: behavior matches current LIKE semantics for basic terms.

Testing
- Populate fixture data and assert result order for rank vs recency; include no‑FTS path.

---

## 4) Tags

Schema (new migrations)
- `NNNN_tags.sql` — create tables:
  - `tags(name TEXT PRIMARY KEY)`
  - `clip_tags(clip_id TEXT REFERENCES clips(id) ON DELETE CASCADE, name TEXT REFERENCES tags(name) ON DELETE CASCADE, PRIMARY KEY(clip_id, name))`
  - Index: `idx_clip_tags_name` on `(name)` for reverse lookups.

Core API (`ditox-core`)
- Extend `Clip` with an optional `tags: Option<Vec<String>>` in JSON outputs from CLI only (avoid breaking existing public lib types if desired), or add `Store::get_tags(id)` + `Store::set_tags(id, &[String])`.

CLI
- `ditox tag add <id> <tag>...`
- `ditox tag rm <id> <tag>...`
- `ditox tag ls <id>`
- Wire `--tag TAG` into `list/search/pick` to filter via join on `clip_tags`.

Acceptance
- Tags survive round‑trip export/import; filters combine with favorites/images.

Testing
- CLI black‑box tests covering add/remove/list and search by tag.

---

## 5) Import/Export

Format
- Default: JSON Lines (one object per line). Each object has:
  - Text clip: `{ id, kind: "text", text, created_at, is_favorite, tags?: [..] }`
  - Image clip: `{ id, kind: "image", created_at, is_favorite, tags?: [..], image: { sha256, format, width, height, size_bytes } }`
- Images: exported as files placed under `objects/aa/bb/<sha256>` within an export directory (mirrors current blob layout). For `image_path` entries, copy the original file if accessible; otherwise re‑encode from blob.

CLI
- `ditox export <dir> [--since 30d] [--limit N] [--images] [--favorites] [--tag TAG] [--stdout]`
- `ditox import <dir|file> [--dedupe sha|id|none] [--keep-ids] [--map-device <id>]`

Behavior
- Deduplicate by `id` (default keep existing) or by `sha256` for images. Preserve `created_at` when `--keep-ids` is set; otherwise generate new ids and treat as new clips.

Acceptance
- Export then import into a fresh DB recreates the same logical set of clips (count, text contents, basic image metadata, tags, favorites).

Testing
- Round‑trip tests using a temp dir; validate integrity and counts.

---

## 6) Thumbnails + Image Filters

Design
- Reuse existing `images` table column `thumb_path TEXT` to store a relative path for a generated thumbnail image (PNG/JPEG, e.g., 256px longest side).
- Generation: library function in `ditox-core` that writes `thumbs/aa/bb/<sha256>_256.png` under the same blob root, updates `images.thumb_path`.
- CLI flags for images:
  - `--min-width`, `--max-width`, `--min-height`, `--max-height`, `--format <png|jpeg|...>`, `--max-size <KB|MB>`.
- Picker: if `thumb_path` exists, render a small preview (TUI) or show dimensions + file name.

Acceptance
- Running a thumbnail job on an existing DB populates `thumb_path` for images and improves picker rendering without altering main image data.

Testing
- Unit test thumbnail generation on a sample RGBA; integration test listing with filters.

---

## Developer Notes

- Migrations: one concern per file; name `NNNN_description.sql` with ascending numbers. Include LIKE fallback for new queries when FTS is absent.
- Logging: never print clip contents in INFO/DEBUG; tests must avoid leaking real clipboard contents.
- Config paths: follow XDG; default DB path and blob root remain unchanged. New files live under `${CONFIG}/ditox/` and `${CONFIG}/ditox/data/`.
- Performance: paginate IPC list/search to avoid large payloads; consider `LIMIT 200` per page for picker.

---

## Rollout & Ops

- Provide `scripts/install_clipd_service.sh` and `scripts/install_clipd_timers.sh` that write user‑level systemd units with safe defaults (disabled by default, opt‑in).
- Document Wayland caveats and how to disable watcher while keeping picker/IPC online.
- Backward compatibility: older DBs work; thumbnails and tags are additive. Export tool works without daemon.

---

## Checklist

- [ ] `crates/ditox-clipd` crate (daemon) with health and list endpoints.
- [ ] IPC types crate or module shared with CLI.
- [ ] Picker command: `ditox pick` (+ fallback `--interactive`).
- [ ] Search: rank flag + operators; tests for FTS and LIKE.
- [ ] Tags migrations + CLI subcommands; store APIs.
- [ ] Export/Import CLIs with round‑trip tests.
- [ ] Thumbnails generator + filters + picker preview.
- [ ] Systemd templates + install scripts.
- [ ] Docs: PRD updates, README snippet, and man‑page‑style CLI help.
- [ ] CI: run picker tests in headless mode; ensure `cargo fmt`, `clippy -D warnings`, and `cargo test --all` stay green.
