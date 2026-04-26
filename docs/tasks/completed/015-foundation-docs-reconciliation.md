# Task: Reconcile docs with reality

> **Status:** completed
> **Priority:** medium
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

Several docs drifted out of sync with the v0.3.1 codebase:

1. `docs/ROADMAP.md` reports `In Progress: 0` while
   `docs/tasks/in-progress/` has 3 task files.
2. `docs/notes/linux-gui-architecture.md` documents the flock + Unix
   socket IPC model that was removed in task `013`.
3. `AGENTS.md` references `ipc_bridge.rs` which no longer exists.
4. The numbering `004-` and `010-` exists in both `completed/` and
   `in-progress/`. Future planning uses `014+` to avoid collision.

We will keep the legacy IPC notes (they describe the model we'll revert
to in Phase 4) but mark them as "historical / forward-looking."

## Requirements

- [ ] Update `docs/ROADMAP.md`:
      - Correct In-Progress / Planned counts.
      - Add the new phases (0..8) from `master-plan-v1.md`.
      - Reference `master-plan-v1.md` from "Quick Reference."
- [ ] Update `AGENTS.md`:
      - Remove the `ipc_bridge.rs` reference.
      - Update GUI section to reflect post-013 one-shot model AND note
        Phase 4 will revert to long-running daemon.
- [ ] Update `docs/notes/linux-gui-architecture.md`:
      - Add a header banner: "Historical & forward-looking — IPC was
        removed in task 013; will be reintroduced in Phase 4 per
        `master-plan-v1.md`."
- [ ] Add `docs/notes/numbering.md` documenting the conflict and the
      `014+` rule for new tasks.

## Implementation Notes

Don't delete the legacy IPC documentation — it's the design we're
returning to. Just tag it.

The numbering doc explains:
- IDs 001-013 in `completed/` are historical.
- IDs 004 and 010 in `in-progress/` are pre-existing (predate the
  collision check).
- New tasks start at 014.
- A task's number doesn't change when it moves between dirs.

## Testing

Manual review:

- `cd docs/tasks/in-progress && ls | wc -l` matches the count in
  `ROADMAP.md` In-Progress row.
- `grep -r ipc_bridge AGENTS.md` returns nothing.
- `head -3 docs/notes/linux-gui-architecture.md` shows the historical
  banner.

## Work Log

### 2026-04-26
- Task file created.
- Updated `docs/notes/linux-gui-architecture.md` with historical/forward-looking banner.
- Updated `AGENTS.md`: removed `ipc_bridge.rs` reference; documented post-013 one-shot model AND the Phase 4 reversal plan; updated GUI summoning section.
- Created `docs/notes/numbering.md` documenting the task ID conflict and the `014+` allocation rule.
- ROADMAP.md was already updated when phases were planned.
