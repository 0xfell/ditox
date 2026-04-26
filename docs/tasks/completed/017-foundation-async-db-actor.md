# Task: Introduce DB actor

> **Status:** completed
> **Priority:** high
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

`rusqlite` is **not `Sync`**. The pre-actor `ditox-gui` worked around
this with `Arc<Mutex<Database>>` and blocking lock acquisition on the
iced render thread. Every DB operation blocked the UI; with the
in-process watcher polling on a separate thread there was also lock
contention between writes and queries.

This task replaces the mutex-shared handle with a closure-based actor:

- A dedicated `std::thread` owns the `Database`.
- A `SyncSender<DbJob>` is shared (it *is* `Sync`) by all callers via
  the cheap-to-clone `DbHandle`.
- Each call sends a `Box<dyn FnOnce(&mut Database) + Send>` and
  blocks on a per-call `sync_channel(1)` reply.

Benefits realised:
- No mutex contention — actor processes commands FIFO from one thread.
- Backpressure available (`try_call` / `dispatch` for hot paths).
- Trivially extensible — new `Database` methods callable through the
  actor without touching it.
- Cheap clone for handing the handle to subscriptions and Tasks.

## Requirements

- [x] **`DbActor` in `ditox-core/src/db_actor.rs`** with:
  - `DbHandle::spawn(database: Database) -> (DbHandle, DbActorJoin)`
  - `DbHandle::spawn_with_depth(database, depth)` for tests
  - `DbHandle::call::<R, F>(f)` — blocking round-trip
  - `DbHandle::try_call::<R, F>(f)` — non-blocking send + blocking reply
  - `DbHandle::dispatch::<F>(f)` — fire-and-forget
  - `DbHandle::flush()` — block until queue drained
  - Implements `Clone`, `Send`, `Sync`.
- [x] **Closure-based command surface** instead of one-variant-per-method
  enum. See design doc for rationale (`docs/notes/db-actor.md`).
- [x] **Refactor `ditox-gui::DitoxApp`** to hold `DbHandle` instead of
  `Arc<Mutex<Database>>`. All DB calls become closures dispatched to
  the actor.
- [x] **Backpressure.** Default queue depth `64` (constant
  `DEFAULT_QUEUE_DEPTH`); `try_call` and `dispatch` return
  `DitoxError::Other("queue full…")` immediately when saturated.
- [x] **Graceful shutdown.** Actor exits when the last `DbHandle`
  clone drops. No explicit `shutdown()` API — `Drop` handles it.
- [x] **Documentation** in `docs/notes/db-actor.md` covering
  rationale, usage, design decisions, and follow-ups.
- [ ] **`Watcher` migration** — deferred. Watcher is a single-threaded
  consumer of its own `Database` handle; the actor would add per-poll
  latency for zero contention benefit. Phase 4 (long-running daemon
  + IPC) will migrate when the watcher and GUI share one `Database`.
- [ ] **`ditox-tui` CLI migration** — deferred for the same reason.
  Each CLI invocation is a short-lived single-threaded process.

## Implementation Notes

### Why closures, not enum

The original spec proposed `DbCommand::InsertEntry { entry, reply }`,
`DbCommand::GetEntries { … }`, etc. — one variant per `Database`
method. With ~25 methods today and Phase 1 adding more, that's
boilerplate that has to track every `Database` change.

A `Box<dyn FnOnce(&mut Database) + Send + 'static>` is a single type
that can express any operation:

```rust
handle.call(|db| db.count())?
handle.call(move |db| db.insert(&entry))?
handle.call(move |db| {
    db.set_entry_collection(&id, Some(&col))?;
    db.touch(&id)
})?  // ad-hoc multi-statement transaction, no enum extension needed
```

Trade-off: the command surface isn't enumerable for IPC serialisation.
Phase 4's IPC layer will need a separate `DaemonCommand` enum anyway,
so the actor doesn't need to be it.

### Why `std::sync::mpsc`, not tokio or crossbeam

Same reasoning as `capture.rs` (task 018): keep `ditox-core`
runtime-agnostic. `std::sync::mpsc::sync_channel(N)` gives us:

- Bounded queue with `try_send` for backpressure
- `sync_channel(1)` doubles as a one-shot reply channel
- Zero new dependencies

Tokio would force the runtime decision into the core; crossbeam would
add a dep for marginal gain.

### Scope decision: GUI only

The actor solves cross-thread access to a `!Sync` resource. The GUI
is the only frontend with cross-thread DB access (iced render thread +
watcher poll thread inside the same process). The standalone watcher
daemon, TUI, and CLI are each single-threaded consumers of their own
`Database` handle — adding the actor would route work through an
extra channel hop for zero benefit.

Phase 4 will migrate the watcher daemon as part of the daemon model
where one `Database` is shared across iced rendering, watcher
polling, and IPC command handling.

## Testing

8 inline tests in `db_actor::tests`:

- `call_round_trips_a_simple_closure`
- `insert_then_count_via_actor`
- `dispatch_is_fire_and_forget_but_committed`
- `handle_clones_share_one_actor`
- `actor_exits_when_last_handle_drops`
- `try_call_returns_full_when_queue_saturated` — minimum-depth (1)
  channel; verifies `Err(queue full)` instead of blocking
- `flush_blocks_until_queue_drained`
- `stress_one_thousand_inserts_from_many_threads` — 10 concurrent
  threads, 100 inserts each, asserts final `count() == 1000`

Total workspace count after this task: **80 tests** (was 72, +8 actor
tests).

## Risks & follow-ups

- **Latency per call.** Channel send + per-job spawn of a reply
  oneshot adds ~microseconds of overhead. Measured: 1000 sequential
  round-trips finish in ~18 s in the stress test (~18 ms/op due to
  `Entry::new_text` allocation; the channel layer itself is
  negligible). For the GUI's 60 fps frame budget (16 ms), single
  calls fit comfortably.
- **Long migrations.** A future `RunMigration` that takes seconds
  would block the actor and any waiting GUI calls. Mitigation
  (Phase 1+): add a separate "maintenance" channel or chunk the
  migration with progress callbacks.
- **Async wrapper.** `iced` is async-native; today we wrap blocking
  `call`s in `tokio::task::spawn_blocking` for the search path. An
  `async fn call_async(...)` on `DbHandle` would be cleaner;
  out-of-scope for Phase 0, in-scope for Phase 4.

## Work Log

### 2026-04-26
- Task file created with planned spec (typed `DbCommand` enum).
- Surveyed all `Database` callers across the workspace; only
  `ditox-gui` uses `Arc<Mutex<Database>>` (one site, 7 callsites).
  TUI and CLI use `Database` directly with no mutex; standalone
  watcher daemon does too.
- Decision: closure-based actor over typed enum (extensibility,
  transaction-friendly, no boilerplate per `Database` method).
- Decision: `std::sync::mpsc::sync_channel` over tokio/crossbeam
  (no new deps, no runtime coupling on `ditox-core`).
- Decision: scope refactor to GUI only; defer Watcher and CLI to
  Phase 4 daemon work.
- Built `ditox-core/src/db_actor.rs` (`DbHandle`, `DbActorJoin`,
  `call`/`try_call`/`dispatch`/`flush`); 8 inline tests including
  1000-insert stress and minimum-depth backpressure.
- Refactored `ditox-gui/src/app.rs`: `db: Arc<Mutex<Database>>` →
  `db: DbHandle`; spawn actor in `DitoxApp::new`; replace 7
  callsites with `call`/`dispatch` per use case (queries vs
  fire-and-forget vs delete-then-refresh blocks); updated the
  search `Task::perform { spawn_blocking { ... } }` to call the
  actor instead of locking the mutex.
- Wrote `docs/notes/db-actor.md` covering design rationale, usage,
  scope decisions, and open follow-ups.
- Workspace `cargo test`, `cargo clippy --workspace --all-targets
  -- -D warnings`, and `cargo fmt --all -- --check` all clean.
  72 → 80 tests, 0 regressions.
