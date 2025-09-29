Ditox Sync — Local‑First Online/Offline PRD

Overview
- Local-first: always read/write to local SQLite; sync to Turso/libSQL when configured and online. If offline or Turso disabled, everything continues to work locally. When online again, reconcile changes automatically.

Goals
- Seamless offline → online transitions with no user intervention.
- Eventual consistency across devices, deterministic conflict resolution.
- Low overhead: background sync with batching and backoff.

Non-Goals (v1)
- Real-time bidirectional streaming; per-field CRDTs; image sync.

Architecture
- Local DB is authoritative for UX. A SyncEngine runs in background (daemon later; CLI on-demand now).
- Remote adapter (Turso) implements push/pull. FTS optional; LIKE fallback.
- Operations are logged locally (ops table) and applied to remote as idempotent UPSERTs; remote rows are pulled and merged into local with LWW.

Data Model (local)
- clips: add updated_at INTEGER NOT NULL, lamport INTEGER NOT NULL DEFAULT 0 (keep deleted_at, is_favorite).
- ops: op_id ULID PK, entity ('clip'), entity_id TEXT, op ('upsert'|'delete'|'favorite'), payload JSON, device_id TEXT, lamport INTEGER, ts INTEGER, applied INTEGER DEFAULT 0.
- devices: device_id ULID PK, created_at.
- sync_state: last_pull_lamport INTEGER, last_pull_ts INTEGER, last_push_op TEXT.
(remote) clips mirrors columns (updated_at, lamport, device_id), FTS optional.

Identity & Ordering
- device_id persisted in settings. Each local write: lamport += 1; updated_at=now(). Pull: lamport=max(local, max(remote))+1.
- Conflict: LWW by (lamport, updated_at, device_id) — deterministic tiebreaker.

Sync Protocol
- Push: transform local ops → remote conditional UPSERT guarded by LWW tuple. Batched, idempotent.
- Pull: SELECT rows with (lamport, updated_at) > checkpoint, page by created_at/lamport, apply locally with same LWW.
- Deletes: use tombstones (deleted_at != NULL) propagated as normal updates.

Connectivity & Scheduling
- Online probe: remote SELECT 1. Backoff exponential (5s→5m). Trigger: manual `ditox sync run` or periodic timer (systemd) later.

CLI & Settings
- settings.toml: [storage] backend, url, auth_token, device_id; [sync] enabled=true, interval="5m", batch_size=500, backoff_max="5m".
- Commands: `ditox sync status` (enabled, last sync, pending ops, last error), `ditox sync run` (one iteration, optional --push-only/--pull-only).

Security
- Transport via Turso tokens; settings file 0600. v1 no E2EE; v1.2 consider encrypting `text` per-device key.

Failure & Retry
- Never block local writes. Partial failures OK; resume from last checkpoint. Detect schema drift; surface in doctor/status.

Testing
- Unit: LWW merge/idempotency; op serialization. Integration: offline → ops → reconnect → converge. Property: commutativity of merges.

Milestones
- v1: text-only sync, ops log, LWW merges, manual CLI run/status, retries/backoff.
- v1.1: background daemonized sync + better progress.
- v1.2: optional image sync, E2EE for `text`.

Risks / Open Questions
- Lamport misuse/clock skew; we prioritize lamport over timestamps. Remote FTS variance (fallback ready). Scale under large histories (need paging+limits). Multi-user per DB (scope to per-account for now).
