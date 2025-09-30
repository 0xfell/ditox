Title: Remove IDs from TUI list and add “Last Time Used” tracking

Owner: core+cli
Status: Draft
Target Version: 0.1.x

Summary
- Stop showing internal clip IDs in the TUI list.
- Add a per-clip field last_used_at updated each time a user selects an entry (text or image) in the picker.
- Surface last_used_at in the TUI as human‑readable (“Last used 2h ago”), and over the daemon JSON so mixed versions remain compatible.

Goals
- TUI list items no longer display the clip ID.
- Selecting an item (press Enter) writes last_used_at = now() for that item.
- TUI shows metadata line with last_used_at (or “never”).
- Works whether listing via local Store or the daemon.

Non‑Goals
- Changing headless/STDOUT behavior that prints the selected ID (kept for scripting compatibility).
- Re-sorting lists by last_used_at (we may add as a follow‑up; keep created_at ordering for now).
- Backfilling historical last_used_at values.

User‑Facing UX Changes
- List item layout (Text):
  - Before: "* <id> <preview>"
  - After:  "* <preview>" on line 1; line 2 (dim): "Created <rel time> • Last used <rel time|never>"
- List item layout (Image): similar, showing size/format on line 1 and dates on line 2.
- No ID appears anywhere in the TUI chrome. Footer/help remains unchanged.

Data Model & Storage
- Schema: add nullable INTEGER column clips.last_used_at (unix seconds).
- Index: CREATE INDEX IF NOT EXISTS idx_clips_last_used_at ON clips(last_used_at DESC);
- Migration: crates/ditox-core/migrations/0006_last_used.sql with the above. Leave NULL for existing rows.

Core API Changes (ditox-core)
- Type: extend Clip with last_used_at: Option<OffsetDateTime>.
- Trait: add fn touch_last_used(&self, id: &str) -> anyhow::Result<()> to Store.
- SqliteStore:
  - Populate last_used_at in SELECTs (list, get, list_images) with NULL handling.
  - Implement touch_last_used: UPDATE clips SET last_used_at=?, updated_at=?, lamport=? WHERE id=?.
- MemStore:
  - Track last_used_at per Clip and implement touch_last_used.

Daemon (ditox-clipd)
- JSON Item payloads: add optional last_used_at (i64|null) for both Text and Image.
- list_text/list_images: include last_used_at from Clip when serializing.
- (No write op required; the picker calls Store.touch_last_used directly.)

CLI/TUI (ditox-cli)
- picker.rs:
  - Remove ID from rendered ListItem strings.
  - Render two-line ListItem with preview on L1 and metadata on L2 using dim style; show created_at and last_used_at (relative).
  - After a successful copy (text or image), call store.touch_last_used(&id) before exiting.
  - Headless mode: still return/print the ID; also touch last_used_at before return.
- Add tiny helper for relative time formatting (e.g., “just now”, “5m ago”, “2d ago”).

Search/Filters
- No change to query semantics. Future: optional sort mode by last_used_at when requested.

Testing
- Core (sqlite + mem):
  - Add tests for touch_last_used setting non‑NULL and monotonic behavior.
  - Verify list/get return last_used_at correctly (NULL vs Some).
- CLI picker tests:
  - Update headless flow to assert last_used_at is set after Enter.
  - Golden snapshot: ensure rendered strings contain no IDs and include “Last used”.
- Daemon tests:
  - Round‑trip JSON includes last_used_at and remains backward compatible if field is missing.

Migrations & Rollout
- Add 0006_last_used.sql:
  - ALTER TABLE clips ADD COLUMN last_used_at INTEGER NULL;
  - CREATE INDEX IF NOT EXISTS idx_clips_last_used_at ON clips(last_used_at DESC);
- Ensure migrate commands continue to work:
  - cargo run -p ditox-cli -- migrate --status
  - cargo run -p ditox-cli -- migrate --backup
- No automatic backfill. Optionally add `doctor` subcommand advice to backfill (set to created_at) only if explicitly requested.

Compatibility
- JSON shape between CLI and daemon gains an optional field; old clients ignore it; new clients treat missing as “never”.
- Scripts relying on printed IDs after selection still work.

Performance/Telemetry
- Negligible cost: single UPDATE on selection; index supports future sorting.

Security/Privacy
- last_used_at is local metadata only; avoid logging it in debug output.

Acceptance Criteria
- TUI never shows IDs anywhere in the list UI.
- Selecting any item sets last_used_at and subsequent list renders show an updated relative time.
- All fmt/clippy/tests pass; migrations show current=latest.

Work Breakdown
1) Schema + migrations
   - Add 0006_last_used.sql and wire through.
2) Core types & Store trait
   - Add last_used_at to Clip; trait method touch_last_used.
   - SqliteStore/MemStore implementations and queries updated.
3) Daemon JSON
   - Add last_used_at to Item::Text/Image and serializers.
4) TUI rendering
   - Remove IDs; show preview + metadata lines; add relative‑time helper.
   - Call touch_last_used on selection (text/image, headless + interactive).
5) Tests
   - Core + CLI + daemon updates; refresh snapshots if any.
6) Docs
   - README: mention “Last used” in TUI section; PRD: record field.

File/Module Touch List (indicative)
- crates/ditox-core/migrations/0006_last_used.sql
- crates/ditox-core/src/lib.rs (Clip, Store trait, SqliteStore, MemStore)
- crates/ditox-clipd/src/main.rs (Item structs + list_* serializers)
- crates/ditox-cli/src/picker.rs (render + selection path + time formatting)
- crates/ditox-cli/tests/* (picker behavior)
- PRD.md/README.md (notes)

Dev Checklist (run locally)
- cargo fmt --all
- cargo clippy --all-targets -- -D warnings
- cargo test --all
- cargo run -p ditox-cli -- migrate --status

Open Questions
- Do we want an optional sort by last_used_at when no query is active? (default off in this task)
- Should “copy without exit” (future multi‑copy) also update last_used_at on each copy?
