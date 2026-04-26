# Task numbering convention

## Current state (as of 2026-04-26)

The task IDs in `docs/tasks/` have a historical conflict:

- Numbers 001-013 in `completed/` form the original work.
- Numbers `004` (`004-windows-installer-distribution.md`) and `010`
  (`010-windows-11-support.md`) in `in-progress/` were created
  later without checking the completed range.
- The third in-progress task (`gui-improvements.md`) has no number.

To avoid further collision, **all new tasks start at `014`** and are
allocated by reading `ls docs/tasks/{completed,in-progress,planned}/`
and picking the next free number.

## Rules

1. **A task's number doesn't change** when it moves between
   `planned/`, `in-progress/`, and `completed/`. The file is renamed
   only if the title changes meaningfully.
2. **Phase 0 tasks** are `014`–`022` (foundation hardening).
3. **Phase 1-8 epic tasks** are `023`–`031`. Each phase epic spawns
   sub-tasks numbered after the highest-allocated number when
   actually started (e.g. Phase 1 epic `023` may spawn `032`,
   `033`, `034` when `023` is moved to `in-progress`).
4. Future tasks discovered out-of-band (bug reports, design
   refactors) take the next free number, not a sub-numbering scheme.

## Allocation log

| Range | Phase | Status |
|---|---|---|
| 014-022 | Phase 0 — Foundation | completed |
| 023 | Phase 1 — Multi-format capture (epic) | completed (8/9; 1.4 → 032) |
| 024 | Phase 2 — Paste-back | planned |
| 025 | Phase 3 — Power-user features | planned |
| 026 | Phase 4 — Ditto UX | planned |
| 027 | Phase 4b — GUI parity | planned |
| 028 | Phase 5 — Hotkeys/IPC/scripting | planned |
| 029 | Phase 6 — LAN sync | planned |
| 030 | Phase 7 — Distribution & i18n | planned |
| 031 | Phase 8 — macOS | planned |
| 032 | Phase 1 carry-over — Windows multi-format capture (`AddClipboardFormatListener`) | planned (spawned from 023 sub-task 1.4) |
| 033+ | Future | unallocated |
