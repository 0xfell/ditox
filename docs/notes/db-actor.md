# DB Actor

> **Status:** active (introduced by task 017, Phase 0)
> **Module:** `ditox-core/src/db_actor.rs`
> **Public types:** [`DbHandle`], [`DbActorJoin`], [`DEFAULT_QUEUE_DEPTH`]

## What it is

A small actor that owns a single `Database` (and its underlying
`rusqlite::Connection`) on a dedicated thread. Callers interact via
`DbHandle`, a cheap-to-clone `Send + Sync` wrapper around a bounded
`std::sync::mpsc::SyncSender`.

```
                +---------------+        +--------------+
                |  iced thread  |  call  |              |
                +-------+-------+  ----> |              |
                        |               |              |
+---------+   +---------v-------+       |  actor       |
| watcher |   | other DbHandle  | call  |  thread      |
| poller  |-->|     clones      | ----> |  owns        |
+---------+   +-----------------+       |  Database    |
                                        |              |
                                        +--------------+
                                              | sync_channel(64)
                                              | reply: sync_channel(1)
```

## Why

`rusqlite::Connection` is `!Sync`. The pre-actor pattern was
`Arc<Mutex<Database>>`, which:

1. **Serialises everything** onto the single mutex — multiple readers
   can't run concurrently even though SQLite supports it.
2. **Blocks the iced render thread** for the duration of any query.
   A 50 ms `count()` against a 100k-row DB hitches the frame.
3. **Conflates two concerns**: cross-thread access (Rust ownership)
   and cross-connection coordination (SQLite locking). They have
   different right answers.

The actor decouples them:

- **Cross-thread access** is solved by the actor pattern: callers
  send closures, the actor runs them on its dedicated thread.
- **Cross-connection coordination** continues to be SQLite's job (we
  have one connection per process today, so this is moot).

## How

### Spawning

```rust
use ditox_core::{Database, DbHandle};

let db = Database::open()?;
db.init_schema()?;
let (handle, _join) = DbHandle::spawn(db);
```

`DbHandle::spawn` starts a `std::thread::Builder` named
`ditox-db-actor`. The returned `DbActorJoin` wraps the `JoinHandle`;
most callers can drop it and rely on the actor exiting when the last
`DbHandle` clone drops.

### Calling

Three flavours, picking by what the caller needs:

| Method        | Send mode  | Reply mode      | Use when                                          |
|---------------|-----------|-----------------|---------------------------------------------------|
| `call`        | blocking  | blocking        | Normal query/mutation. Default choice.           |
| `try_call`    | non-block | blocking        | Watcher hot-path: never want to stall the poll.  |
| `dispatch`    | non-block | none            | Fire-and-forget mutation (`touch` after copy).   |
| `flush`       | blocking  | blocking        | Tests / shutdown — wait for all queued work.     |

Closure signature: `FnOnce(&mut Database) -> R + Send + 'static`.

```rust
// Query
let n = handle.call(|db| db.count().unwrap())?;

// Mutation that may need its result
let id_to_delete = entry.id.clone();
let was_deleted = handle.call(move |db| db.delete(&id_to_delete))??;

// Fire-and-forget bump on copy
let id = entry.id.clone();
handle.dispatch(move |db| { let _ = db.touch(&id); })?;

// Hot path: drop on backpressure rather than block the watcher
match handle.try_call(move |db| db.insert(&entry)) {
    Ok(Ok(())) => {}
    Ok(Err(e)) => warn!("db insert failed: {}", e),
    Err(e) if format!("{e}").contains("queue full") => {
        warn!("db actor saturated, dropping clip");
    }
    Err(e) => error!("db actor unreachable: {}", e),
}
```

### Shutdown

The actor exits when the last `DbHandle` clone is dropped. There is
no explicit `shutdown()` method.

For tests, holding the `DbActorJoin` returned from `spawn` lets you
`.join()` to wait for the thread to finish flushing.

## Design decisions

### Closures vs typed enum

The original Phase-0 spec proposed one variant per `Database` method
(`InsertEntry`, `GetEntries`, …). With ~25 public methods and Phase 1
multi-format adding more, that's a lot of boilerplate that has to
move in lockstep with the `Database` API.

Closures sidestep this: any new `Database` method is callable through
the actor without touching the actor itself. The downside is that the
command surface isn't enumerable — Phase 4's IPC layer will need a
separate `DaemonCommand` enum for serialised wire commands anyway, so
the actor doesn't lose any expressiveness it would otherwise have.

### `std::sync::mpsc` vs `crossbeam_channel` vs `tokio::sync::mpsc`

We picked `std::sync::mpsc::sync_channel(N)` because:

- It's stdlib — no extra dep.
- `sync_channel(N)` gives us `try_send` for backpressure.
- The reply channel `sync_channel(1)` doubles as a one-shot.
- No async runtime requirement on `ditox-core`.

`tokio::sync::mpsc` would have forced `tokio` into `ditox-core`,
which we explicitly rejected for `capture.rs` (task 018) — same
reasoning applies here.

### Bounded queue (default 64)

A bounded channel is required for `try_call` / `dispatch` to surface
backpressure. 64 commands lets the GUI tolerate burst input
(typing fast in the search box → ~30 commands/sec) without any
real-world risk of `queue full`. The watcher's worst case is one
`try_call` per `poll_interval_ms` (default 100 ms), so 64 covers
6.4 s of complete actor stall — more than enough margin for the GUI
to recover or surface an error.

### Panic handling

If a closure panics, the actor thread unwinds and drops the receiver.
All in-flight reply channels disconnect. New `call`s return
`DitoxError::Other("channel closed")`. Callers observe a clear error
and can degrade or re-spawn.

This is deliberately blunter than `Mutex` poisoning:

- DB code shouldn't panic in steady state.
- A crashed actor is a clearer signal than silently-poisoned state.
- Phase 6 (sync) may add automatic restart with a small command
  buffer; for now, frontends show an error.

### Scope of the Phase 0 refactor

Only the GUI was migrated to `DbHandle`. The standalone watcher
daemon (`ditox watch`) and the TUI/CLI were left direct, because:

- They're single-threaded users of their own `Database` handles —
  no cross-thread access, so the actor would add latency without
  removing real contention.
- The contention the actor is solving exists *only* in the GUI
  process, where iced and the in-process watcher share state.

Phase 4 (long-running daemon + IPC) will revisit and migrate the
watcher daemon onto the actor as part of the daemon-spawned-by-GUI
model. The CLI invocations will stay direct (each invocation is its
own short-lived process; the actor would just add startup cost).

## Adding a new operation

Just call it as a closure:

```rust
let result = handle.call(|db| db.your_new_method(args))?;
```

No actor changes needed. The closure runs on the actor thread,
reply travels back through the per-call oneshot.

## Testing the actor

`ditox-core/src/db_actor.rs` has 8 inline tests covering:

- `call` round-trip
- `dispatch` is fire-and-forget but committed
- Handle clones share one actor
- Actor exits when last handle drops
- `try_call` returns `queue full` when saturated
- `flush` blocks until queue drained
- 1000-insert stress from 10 concurrent threads

Run with: `nix develop -c cargo test -p ditox-core --lib db_actor`.

## Open questions / follow-ups

- **Async wrapper.** `iced` is async-native; today we wrap blocking
  `call`s in `tokio::task::spawn_blocking` for the search path. A
  proper `async fn call_async(...)` on `DbHandle` would be cleaner;
  out-of-scope for Phase 0, in-scope for Phase 4.
- **Cancellation.** Long-running queries (large `vacuum`) can't
  currently be cancelled. Phase 4 may add `oneshot::Sender<bool>` for
  cancellation.
- **Metrics.** The actor doesn't currently emit `tracing` spans
  per-command. Add a `with_tracing(true)` builder option in Phase 1 if
  observability becomes a need.
