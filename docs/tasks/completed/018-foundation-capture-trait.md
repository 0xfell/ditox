# Task: Generalise the watcher with a `CaptureSource` trait

> **Status:** completed
> **Priority:** high
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

Today's watcher (`ditox-core/src/watcher.rs`) hard-codes a polling
strategy and platform-specific clipboard reads. To support:

- Event-based Windows clipboard listener (Phase 1).
- Wayland event-based capture if the protocol matures (post-v1.0).
- Multiple clipboards (X11 selection + clipboard).
- macOS clipboard (Phase 8).
- Tests with mock capture sources.

… we need an abstraction. Introduce a `CaptureSource` trait that the
watcher consumes, with one implementation per platform/strategy.

## Requirements

- [x] **`CaptureSource` trait** in `ditox-core/src/capture.rs` —
      synchronous (no `async fn`) so `ditox-core` stays
      runtime-agnostic; async/event-driven backends sit a thread away
      behind the `mpsc::Receiver` returned by `subscribe()`.
- [x] **`RawClip` / `RawFormat` structs** with `captured_at`,
      optional `source_app`, and a `Vec<RawFormat>` so Phase 1
      multi-format work doesn't have to redo the model.
- [x] **`PollingCaptureSource<F>` adapter** — wraps a synchronous
      "read clipboard now" closure, spawns a named background thread,
      hash-dedups, idempotent shutdown, propagates receiver-drop as
      thread exit. Used by `Watcher::new` to wrap the existing
      `Clipboard::read_image` / `Clipboard::get_text` reads.
- [x] **`Watcher` consumes `Vec<Box<dyn CaptureSource>>`.** New
      `Watcher::with_sources(db, config, sources)` constructor for
      tests and Phase 1 multi-source setups; `Watcher::new` continues
      to install a single platform-default source so existing call
      sites are unchanged.
- [x] **`MockCaptureSource`** for unit tests in `capture.rs` plus a
      richer `QueueSource` test helper in
      `tests/watcher_capture_integration.rs` for the watcher-level
      flow tests.
- [x] **Watcher refactoring done in this task.** No new capture
      backends yet — those land with Phase 1 (`023`) and Windows
      `AddClipboardFormatListener`.

## Implementation Notes

**Why synchronous, not async/streams.** The original spec called for
`#[async_trait]`, `tokio::select!`, and a `StreamMap`. We rejected
that for Phase 0 because:

1. `ditox-core` has no `tokio` dependency today and adding one to a
   library that's consumed by both async (potential GUI iced runtime)
   and sync (CLI, watcher daemon) callers couples the core to a
   runtime decision better made per-frontend.
2. The current `Watcher` runs on its own thread with a sleep loop —
   forcing it through tokio's runtime adds machinery without solving
   any real problem until Phase 1 introduces multiple sources whose
   events need merging.
3. Event-driven backends (`AddClipboardFormatListener`,
   `wlr-data-control-v1` events) don't actually need async/await —
   they need a way to push values into a channel from whatever thread
   the OS calls them on. `mpsc::Receiver` is exactly that.

The trait surface (`name`, `current_snapshot`, `subscribe`,
`shutdown`) covers both polling and event-driven consumers without
either dictating the runtime.

**Current Watcher loop** uses `current_snapshot()` (synchronous) per
poll cycle. `subscribe()` is exercised by the `PollingCaptureSource`
unit tests but isn't yet consumed by the watcher itself. Phase 1 may
add a separate `EventLoopWatcher` that consumes streams via
`crossbeam-channel::Select` for true event-driven backends; the
trait already supports both modes.

**Image-format priority preserved.** The original watcher checked
images first, then text (so browser "Copy image" captures the image,
not the URL). The refactor keeps this in two places:
1. The default `legacy_clipboard_snapshot` closure reads
   `Clipboard::read_image` first, falls back to `Clipboard::get_text`.
2. `Watcher::process_clip` walks `RawClip.formats` for `image/*`
   first then `text/plain`.

**Dedup model.** `Watcher.last_hash` now stores `clip_hash(&RawClip)`
(SHA-256 over sorted formats) instead of the per-payload hash. This
gives format-set-aware dedup for free — Phase 1 multi-format clips
where the underlying image bytes are identical but the text caption
changes will correctly produce a new entry. Persistent dedup against
the DB still uses the inner content hash via `Database::exists_by_hash`.

**Source priority.** When multiple sources are registered, the first
to return `Some` from `current_snapshot()` wins for that poll. This
matches X11's CLIPBOARD-vs-PRIMARY semantics that Phase 1 will need.

## Testing

- 7 unit tests in `capture.rs` (`raw_clip_*`, `clip_hash_*`,
  `polling_source_emits_on_change_and_dedups`,
  `mock_source_drains_queued_then_accepts_injections`,
  `shutdown_is_idempotent`).
- 6 integration tests in `tests/watcher_capture_integration.rs`:
  - `watcher_captures_text_from_mock_source`
  - `watcher_dedups_repeated_clip_via_last_hash`
  - `watcher_prefers_image_over_text_in_same_clip`
  - `watcher_falls_through_to_second_source_when_first_empty`
  - `watcher_first_source_wins_when_both_have_content`
  - `watcher_initialize_hash_primes_from_first_nonempty_source`
    (regression test for bug #4 — content already on clipboard at
    startup must not be re-captured on the first poll).

Total workspace count after this task: **72 tests** (was 59 at task
start, +7 capture unit tests, +6 watcher integration tests).

## Work Log

### 2026-04-26
- Task file created with planned spec (async/streams).
- Built `capture.rs` synchronously with `RawClip`/`RawFormat`/
  `CaptureSource` trait + `PollingCaptureSource<F>` adapter +
  `MockCaptureSource` + `clip_hash` helper. 7 unit tests.
- Refactored `Watcher` to hold `Vec<Box<dyn CaptureSource>>`. Added
  `with_sources` constructor. Default `new` constructor wraps the
  legacy `Clipboard` reads in a `PollingCaptureSource`.
- Replaced `poll_internal` body with source-iterating
  `process_clip(RawClip)`. Replaced `initialize_hash` to prime from
  source snapshots.
- Wrote 6 integration tests against `Watcher::with_sources` with mock
  sources covering image priority, dedup, source fall-through, source
  priority, and startup-content non-recapture.
- Fixed clippy warnings introduced/exposed: `truncate(false)` on
  `acquire_lock` and the matching test helper, doc-list overindent in
  `db.rs`. Workspace `cargo test`, `cargo clippy -- -D warnings`,
  `cargo fmt --check` all clean.
