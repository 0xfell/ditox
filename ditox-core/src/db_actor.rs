//! Actor wrapping `Database` so callers can share a `Send + Sync +
//! Clone` handle while the underlying `rusqlite::Connection` (which
//! is `!Sync`) lives on a single dedicated thread.
//!
//! # Why an actor?
//!
//! Today's GUI works around `!Sync` by holding `Arc<Mutex<Database>>`
//! and locking on the iced render thread for every query. Two threads
//! contend on the lock (iced thread for UI queries, the in-process
//! watcher for inserts), which serialises everything onto the lock
//! anyway. Worse, the Mutex blocks the GUI on slow queries — a 50 ms
//! `count()` against a 100k-row DB hitches the frame.
//!
//! The actor shifts that contention onto a bounded channel:
//!
//! - One dedicated `std::thread` owns the `Database`.
//! - Callers send `Box<dyn FnOnce(&mut Database) + Send>` closures.
//! - A reply `mpsc::sync_channel(1)` carries the result back.
//!
//! The handle is just a wrapper around the channel `SyncSender`, which
//! is itself `Send + Sync + Clone`. When the last clone drops the
//! sender disconnects and the actor thread exits — no explicit
//! shutdown required.
//!
//! # Why closures, not a typed `DbCommand` enum?
//!
//! The original Phase-0 spec proposed one variant per `Database`
//! method (`InsertEntry`, `GetEntries`, `Search`, …). With ~25 public
//! methods today and Phase 1 multi-format adding more, that's a lot of
//! boilerplate that has to be kept in lockstep with the `Database`
//! API. Closures are:
//!
//! - **Trivially extensible.** Need a new query? Pass a new closure.
//! - **Transaction-friendly.** A single closure can call multiple
//!   methods; the actor processes it as one unit of work, so future
//!   `db.transaction(|t| { ... })` semantics drop in cleanly.
//! - **Type-safe.** Each `call::<R, F>` is monomorphised on the
//!   return type; no `match` on a response enum at the callsite.
//!
//! The trade-off is that the command surface isn't enumerable for,
//! say, IPC serialisation. Phase 4 (daemon + IPC) will need a
//! separate `DaemonCommand` enum for that anyway, so we lose nothing.
//!
//! # Backpressure
//!
//! The channel is `sync_channel(64)` by default. Callers that *must
//! not* block (the watcher hot-path) use [`DbHandle::try_call`] /
//! [`DbHandle::dispatch`] which `try_send` and surface
//! `DitoxError::Other("queue full…")` instead of blocking.
//!
//! # Panic handling
//!
//! If a closure panics inside the actor, the actor thread unwinds and
//! drops the receiver. All in-flight reply channels disconnect. New
//! `call`s return `DitoxError::Other("channel closed")`. Callers
//! observe a clear error and can degrade or re-spawn. This is
//! deliberately blunter than `Mutex` poisoning — DB code shouldn't
//! panic in steady state and a crashed actor is a clearer signal than
//! silently-poisoned state.

use crate::db::Database;
use crate::error::{DitoxError, Result};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use tracing::debug;

/// A boxed closure that runs against the actor's `Database`.
type DbJob = Box<dyn FnOnce(&mut Database) + Send + 'static>;

/// Default queue depth for the command channel. Sized so the GUI
/// never blocks on bursty user input while still applying
/// backpressure on pathological cases.
pub const DEFAULT_QUEUE_DEPTH: usize = 64;

/// Cheap-to-clone `Send + Sync` handle. Cloning shares the same
/// underlying actor; dropping the last clone shuts the actor down.
#[derive(Clone)]
pub struct DbHandle {
    tx: SyncSender<DbJob>,
}

/// Returned by [`DbHandle::spawn`] alongside the handle. Holds the
/// `JoinHandle` for the actor thread so callers can wait for clean
/// shutdown if they want it. Most callers can drop this and rely on
/// the actor exiting when the last `DbHandle` clone drops.
pub struct DbActorJoin(JoinHandle<()>);

impl DbActorJoin {
    /// Block until the actor thread exits. Useful in tests.
    pub fn join(self) -> std::thread::Result<()> {
        self.0.join()
    }
}

impl DbHandle {
    /// Spawn an actor on a dedicated thread with the default queue
    /// depth. Returns the handle plus a join wrapper for callers that
    /// want explicit shutdown semantics.
    pub fn spawn(db: Database) -> (Self, DbActorJoin) {
        Self::spawn_with_depth(db, DEFAULT_QUEUE_DEPTH)
    }

    /// As [`spawn`] but with an explicit channel depth. Tests use a
    /// small depth (e.g. 1) to exercise backpressure.
    pub fn spawn_with_depth(db: Database, depth: usize) -> (Self, DbActorJoin) {
        let (tx, rx) = sync_channel::<DbJob>(depth);
        let join = thread::Builder::new()
            .name("ditox-db-actor".into())
            .spawn(move || run_actor(db, rx))
            .expect("spawn db actor thread");
        (Self { tx }, DbActorJoin(join))
    }

    /// Send a closure to the actor and block until it returns a
    /// result.
    ///
    /// Errors:
    /// - actor channel closed (last clone dropped or actor panicked)
    /// - reply channel disconnected (closure panicked mid-execution)
    pub fn call<R, F>(&self, f: F) -> Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut Database) -> R + Send + 'static,
    {
        let (rtx, rrx) = sync_channel::<R>(1);
        let job: DbJob = Box::new(move |db: &mut Database| {
            let result = f(db);
            // If the receiver was dropped before we replied (e.g. the
            // caller timed out and walked away), the send fails
            // silently — that's the right behaviour, our work is done.
            let _ = rtx.send(result);
        });
        self.tx
            .send(job)
            .map_err(|_| DitoxError::Other("db actor: channel closed".into()))?;
        rrx.recv()
            .map_err(|_| DitoxError::Other("db actor: closure panicked or reply dropped".into()))
    }

    /// Like [`call`] but returns `DitoxError::Other("queue full…")`
    /// immediately if the actor's queue is at capacity, instead of
    /// blocking. Reply is still received synchronously once the
    /// closure has been queued.
    ///
    /// Use this from the watcher hot-path so a backed-up actor
    /// (mid-`vacuum`, say) never stalls clipboard polling.
    pub fn try_call<R, F>(&self, f: F) -> Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut Database) -> R + Send + 'static,
    {
        let (rtx, rrx) = sync_channel::<R>(1);
        let job: DbJob = Box::new(move |db: &mut Database| {
            let result = f(db);
            let _ = rtx.send(result);
        });
        match self.tx.try_send(job) {
            Ok(()) => rrx.recv().map_err(|_| {
                DitoxError::Other("db actor: closure panicked or reply dropped".into())
            }),
            Err(TrySendError::Full(_)) => Err(DitoxError::Other(
                "db actor: queue full, command dropped".into(),
            )),
            Err(TrySendError::Disconnected(_)) => {
                Err(DitoxError::Other("db actor: channel closed".into()))
            }
        }
    }

    /// Fire-and-forget. The closure runs against the database; the
    /// caller doesn't wait for it. `try_send` is used so this never
    /// blocks; full-queue is surfaced as an error so the caller can
    /// decide whether to retry or drop.
    pub fn dispatch<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Database) + Send + 'static,
    {
        let job: DbJob = Box::new(f);
        match self.tx.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(DitoxError::Other("db actor: queue full".into())),
            Err(TrySendError::Disconnected(_)) => {
                Err(DitoxError::Other("db actor: channel closed".into()))
            }
        }
    }

    /// Block until all currently-queued jobs have been processed.
    /// Useful in tests and at shutdown to flush before exiting.
    pub fn flush(&self) -> Result<()> {
        // Round-trip a no-op closure. The actor processes commands in
        // FIFO order, so when our reply arrives, every command queued
        // before us has finished.
        self.call(|_db| ())
    }
}

fn run_actor(mut db: Database, rx: Receiver<DbJob>) {
    debug!("db actor: starting");
    while let Ok(job) = rx.recv() {
        job(&mut db);
    }
    debug!("db actor: channel closed, exiting");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{set_data_dir_override, Database};
    use crate::Entry;
    use std::sync::Arc;
    use std::sync::Mutex as StdMutex;
    use tempfile::TempDir;

    // `set_data_dir_override` is process-wide. Serialize tests.
    static OVERRIDE_LOCK: StdMutex<()> = StdMutex::new(());

    fn fresh_actor() -> (TempDir, DbHandle, DbActorJoin) {
        let tmp = TempDir::new().unwrap();
        set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();
        let db = Database::open().unwrap();
        db.init_schema().unwrap();
        let (handle, join) = DbHandle::spawn(db);
        (tmp, handle, join)
    }

    #[test]
    fn call_round_trips_a_simple_closure() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, _join) = fresh_actor();
        let n = handle.call(|db| db.count().unwrap()).unwrap();
        assert_eq!(n, 0);
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn insert_then_count_via_actor() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, _join) = fresh_actor();
        let entry = Entry::new_text("hello".to_string());
        handle.call(move |db| db.insert(&entry)).unwrap().unwrap();
        let n = handle.call(|db| db.count().unwrap()).unwrap();
        assert_eq!(n, 1);
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn dispatch_is_fire_and_forget_but_committed() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, _join) = fresh_actor();
        let entry = Entry::new_text("dispatched".to_string());
        handle
            .dispatch(move |db| {
                let _ = db.insert(&entry);
            })
            .unwrap();
        // Round-trip a flush to guarantee the dispatched job ran.
        handle.flush().unwrap();
        let n = handle.call(|db| db.count().unwrap()).unwrap();
        assert_eq!(n, 1);
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn handle_clones_share_one_actor() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, _join) = fresh_actor();
        let h2 = handle.clone();
        let h3 = handle.clone();

        let e1 = Entry::new_text("from-h1".to_string());
        let e2 = Entry::new_text("from-h2".to_string());
        let e3 = Entry::new_text("from-h3".to_string());
        handle.call(move |db| db.insert(&e1)).unwrap().unwrap();
        h2.call(move |db| db.insert(&e2)).unwrap().unwrap();
        h3.call(move |db| db.insert(&e3)).unwrap().unwrap();

        let n = handle.call(|db| db.count().unwrap()).unwrap();
        assert_eq!(n, 3);
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn actor_exits_when_last_handle_drops() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, join) = fresh_actor();
        // Take a clone so we can verify it ALSO needs to drop.
        let clone = handle.clone();
        drop(handle);
        // Actor still alive: clone holds the sender.
        clone.call(|_db| ()).unwrap();
        drop(clone);
        // Now the channel disconnects and the actor exits.
        join.join().unwrap();
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn try_call_returns_full_when_queue_saturated() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();
        let db = Database::open().unwrap();
        db.init_schema().unwrap();
        // Queue depth 1 — minimum bounded sync_channel capacity.
        let (handle, _join) = DbHandle::spawn_with_depth(db, 1);

        // Block the actor with a long-running job.
        let blocker_tx = handle.clone();
        let blocker = std::thread::spawn(move || {
            blocker_tx
                .call(|_db| {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                })
                .unwrap();
        });

        // Give the blocker time to start running on the actor.
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Saturate the queue: send sync (queue depth 1) so the next
        // `try_send` must fail. Race-free because the blocker is
        // currently executing (not in queue), so the queue has 1 slot
        // free — fill it from a thread:
        let filler_tx = handle.clone();
        let filler = std::thread::spawn(move || {
            filler_tx
                .call(|_db| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                })
                .unwrap();
        });
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Now: actor is busy (blocker), queue holds 1 (filler).
        // try_call must return Full immediately.
        let r = handle.try_call(|_db| 42);
        assert!(r.is_err(), "expected queue-full error, got {:?}", r.is_ok());
        let msg = format!("{:?}", r.err().unwrap());
        assert!(msg.contains("queue full"), "wrong error variant: {}", msg);

        blocker.join().unwrap();
        filler.join().unwrap();
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn flush_blocks_until_queue_drained() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, _join) = fresh_actor();

        // Dispatch 10 inserts.
        for i in 0..10 {
            let entry = Entry::new_text(format!("flush-{}", i));
            handle
                .dispatch(move |db| {
                    let _ = db.insert(&entry);
                })
                .unwrap();
        }
        handle.flush().unwrap();
        let n = handle.call(|db| db.count().unwrap()).unwrap();
        assert_eq!(n, 10);
        let _ = set_data_dir_override(None);
    }

    #[test]
    fn stress_one_thousand_inserts_from_many_threads() {
        let _g = OVERRIDE_LOCK.lock().unwrap();
        let (_tmp, handle, _join) = fresh_actor();
        let mut threads = Vec::new();
        let counter = Arc::new(StdMutex::new(0u32));
        for _ in 0..10 {
            let h = handle.clone();
            let c = counter.clone();
            threads.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    let mut idx = c.lock().unwrap();
                    *idx += 1;
                    let val = *idx;
                    drop(idx);
                    let entry = Entry::new_text(format!("entry-{}", val));
                    h.call(move |db| db.insert(&entry)).unwrap().unwrap();
                }
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
        let n = handle.call(|db| db.count().unwrap()).unwrap();
        assert_eq!(n, 1000);
        let _ = set_data_dir_override(None);
    }
}
