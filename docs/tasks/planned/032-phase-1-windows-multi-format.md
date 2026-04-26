# Task: Phase 1 — Windows event-driven multi-format capture

> **Status:** planned
> **Priority:** high (blocks Windows Phase 1 parity)
> **Phase:** 1 — Multi-format capture (carry-over)
> **Created:** 2026-04-26
> **Spawned from:** task 023 sub-task 1.4
> **Estimated:** 1-2 weeks (Windows-side work)

## Why this is its own task

Task 023 sub-task 1.4 was deferred when Phase 1 closed at 8/9
(2026-04-26). It needs a Windows machine (or a Windows VM) for any
meaningful validation — every other Phase 1 sub-task either landed
or was verifiable from Linux. Spinning it out keeps the Windows
work visible and unblocked from Phase 2 progress on Linux.

The Linux equivalent (Wayland multi-format capture via
`wl-clipboard-rs`) landed as part of 023 in commit `f60b94b`.
This task is the Windows mirror.

## Description

Replace the polling `arboard`-based capture in
`ditox-core/src/clipboard.rs` (the `#[cfg(windows)]` block at lines
226-333) with an event-driven `AddClipboardFormatListener` source
that returns *every* offered clipboard format in one snapshot —
mirroring what `WaylandLibraryCapture` does on Linux today.

Implements the `CaptureSource` trait (already defined in
`ditox-core/src/capture.rs`); plugs into `Watcher::new` via the
`#[cfg(windows)]` arm that today still constructs a
`PollingCaptureSource::new("legacy-clipboard", interval,
legacy_clipboard_snapshot)` — see
`ditox-core/src/watcher.rs:289-321`.

## Architecture

```
ditox-gui / ditox watch  (windows)
    │
    ├── spawns: WindowsListenerCapture thread
    │       │
    │       ├── creates message-only window via
    │       │     CreateWindowExW(HWND_MESSAGE, ...)
    │       ├── calls AddClipboardFormatListener(hwnd)
    │       ├── runs message loop
    │       │     on WM_CLIPBOARDUPDATE:
    │       │       - OpenClipboard(hwnd)  (with retry on busy)
    │       │       - EnumClipboardFormats / IDataObject::EnumFormatEtc
    │       │       - read each format via GetClipboardData / IDataObject::GetData
    │       │       - canonicalise via FormatId::from_win32_cf
    │       │       - skip do-not-record sentinels (see below)
    │       │       - construct RawClip
    │       │       - send to mpsc::Sender<RawClip>
    │       │     on WM_DESTROY: RemoveClipboardFormatListener, exit
    │       └── ...
    └── consumer: Watcher reads from rx via subscribe()
            (UNLIKE the Wayland source which is polled via
             current_snapshot — Windows event-driven goes through
             the existing CaptureSource::subscribe contract.)
```

`current_snapshot()` on `WindowsListenerCapture` reads the current
clipboard once via the same OpenClipboard/EnumClipboardFormats path
*without* registering a listener — used for startup priming
(`Watcher::initialize_hash`). The listener thread only fires after
`subscribe()` is called.

## Sub-tasks

### Add `windows-rs` to ditox-core (Windows-only target)

```toml
[target.'cfg(windows)'.dependencies]
arboard.workspace = true   # keep for set_text/set_image paste-back
sysinfo.workspace = true   # keep for process detection
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_DataExchange",
    "Win32_System_Memory",
    "Win32_System_Ole",
    "Win32_UI_WindowsAndMessaging",
] }
```

`windows-rs` is huge; pull only the features actually used.

### `WindowsListenerCapture` skeleton

```rust
// ditox-core/src/capture/windows.rs
pub struct WindowsListenerCapture {
    config: CaptureConfig,
    thread: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl WindowsListenerCapture {
    pub fn new(config: CaptureConfig) -> Self { ... }
}

impl CaptureSource for WindowsListenerCapture {
    fn name(&self) -> &str { "windows-listener" }

    fn current_snapshot(&self) -> Result<Option<RawClip>> {
        // Synchronous one-shot. OpenClipboard + EnumClipboardFormats
        // + GetClipboardData. Used by Watcher::initialize_hash on
        // startup. Does NOT spawn a listener.
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<RawClip>> {
        // Spawn listener thread; return rx.
        // Thread owns the message-only window + AddClipboardFormatListener.
    }

    fn shutdown(&mut self) -> Result<()> {
        // Set shutdown flag; PostMessage(WM_QUIT) to listener thread;
        // join. Idempotent.
    }
}
```

### Per-format read pipeline

For each `EnumClipboardFormats` cf code:

1. Skip do-not-record sentinels (see below).
2. `FormatId::from_win32_cf(cf)` → canonical name; if `None` (custom
   format), call `GetClipboardFormatNameW(cf)` and store as
   `win32:<name>`.
3. Apply `CaptureConfig::should_capture_format(canonical)`.
4. `GetClipboardData(cf)` → `HANDLE`; `GlobalLock` → pointer;
   `GlobalSize` → byte length.
5. Bounded read: cap at `max_format_size_bytes + 1`. Drop with warn
   if exceeded.
6. `GlobalUnlock`. Push `RawFormat { mime: canonical, bytes }`.

After the loop: enforce `clip_size_ok(total)` — drop the WHOLE clip
on overflow (matches Linux behaviour).

### Do-not-record sentinels

Three Windows-defined ways for a source app to opt out of clipboard
history. Honour all three; this is the equivalent of the
`Clipboard Viewer Ignore` sentinel ditox itself sets to avoid
re-capturing its own paste-backs.

```rust
fn should_record(formats: &[u32]) -> bool {
    let cvi = RegisterClipboardFormatW(w!("Clipboard Viewer Ignore"));
    let exclude = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
    let cich = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));

    if formats.contains(&cvi) || formats.contains(&exclude) {
        return false;
    }
    if formats.contains(&cich) {
        // Single DWORD; 0 = exclude.
        let h = GetClipboardData(cich);
        let p = GlobalLock(h) as *const u32;
        let val = *p;
        GlobalUnlock(h);
        if val == 0 { return false; }
    }
    true
}
```

### Watcher wiring

`ditox-core/src/watcher.rs::Watcher::new`:

```rust
#[cfg(windows)]
{
    let mut source = crate::capture::windows::WindowsListenerCapture::new(
        config.capture.clone(),
    );
    // Watcher uses subscribe() for event-driven backends —
    // current_snapshot() only for startup priming.
    let rx = source.subscribe()?;  // ← currently no Watcher path consumes this
    Box::new(source)
}
```

**This requires a Watcher refactor.** Today `Watcher::poll_internal`
only ever calls `current_snapshot()`. To use the event-driven path,
`Watcher::run` needs to (a) call `subscribe()` once and (b) drain
the receiver in the poll loop instead of (or in addition to) calling
`current_snapshot()`. See "Watcher refactor" below.

### Watcher refactor (prerequisite)

`Watcher::poll_internal` (`watcher.rs:439-450`) currently iterates
`self.sources` and calls `current_snapshot()` on each. To consume an
event-driven source like `WindowsListenerCapture`:

- Call `subscribe()` once during `Watcher::run` startup; store the
  `Receiver<RawClip>` per source.
- In the poll loop: `try_recv()` from each receiver; on `Ok(clip)`,
  call `process_clip(clip)`. Fall back to `current_snapshot()` when
  the receiver is empty (for sources that don't push).
- A pure-event source (no polling) sets a long-poll interval but
  still wakes when a clip arrives. Use a `mpsc::Receiver::recv_timeout`
  scheme rather than try_recv to avoid busy-spin.

This refactor is **also** needed before the Linux Wayland source can
be event-driven (currently a stub). Schedule before or alongside
this task.

### Tests

- `name() == "windows-listener"` invariant.
- `shutdown_is_idempotent` — `Drop` contract.
- `should_record` honours each of the three sentinels (mock the
  format set; do not call `GetClipboardData`).
- `current_snapshot` smoke test: requires Windows runtime and a
  pre-populated clipboard; gated `#[ignore]` like the Wayland live
  test.
- `subscribe -> recv` end-to-end: runtime + manual clipboard
  copy; `#[ignore]`.
- Build-only check: `cargo build -p ditox-core --target
  x86_64-pc-windows-gnu` from Linux to catch type errors during
  blind authoring (CI already runs full Windows tests).

## Acceptance criteria

- [ ] **Multi-format capture:** copy in Word → snapshot has at
      minimum `text/plain`, `text/html`, `text/rtf`,
      `win32:CF_UNICODETEXT`, `win32:HTML Format`,
      `win32:Rich Text Format`.
- [ ] **Image priority preserved:** "Copy image" from Edge/Chrome
      adds an `image/png` (via `CF_DIB` conversion) and the watcher's
      `process_clip` picks it over the URL.
- [ ] **Do-not-record honoured:** copy via Microsoft Office's
      "Copy to clipboard but don't track" → no entry created.
- [ ] **No re-capture loop:** ditox's own paste-back doesn't
      generate a new entry (sets `Clipboard Viewer Ignore`).
- [ ] **No regressions:** Windows TUI / GUI / CLI tests still pass.
- [ ] Latency: clipboard change → entry written ≤ 100 ms (no
      polling delay).
- [ ] `Watcher::run` cleanly tears down the listener thread on
      Ctrl+C / SIGTERM.

## Risks

- **`OpenClipboard` busy:** another app may hold the clipboard
  briefly. Retry with exponential backoff (5×, 10ms→100ms);
  abandon snapshot on persistent failure.
- **Listener event drops under load:** Windows can collapse
  multiple `WM_CLIPBOARDUPDATE`s. Mitigation: Ditto's watchdog —
  every 5 min, write a custom format, expect to see it back; on
  miss, re-register. (Implement in v0.5+, not v0.4.)
- **`windows-rs` API churn:** pin to a specific version
  (`0.62.x`) and document the upgrade path. Version 0.62 matches
  the existing `ditox-gui` Win32 dependency.
- **GUI vs daemon double-listen:** `ditox-gui` and `ditox watch`
  both register an `AddClipboardFormatListener`. Both will fire
  on every change; both will write to the DB; dedup catches the
  duplicate but the work is wasted. Mitigation: GUI detects a
  running watcher (existing PID file) and skips its own listener.
  Track in the watcher refactor sub-task.

## Implementation Notes

- Use `windows-rs` `w!` macro for static UTF-16 strings.
- All Win32 calls return `BOOL` / `HANDLE` (zero = error). Wrap in
  `windows::core::Result` via `.ok()?` where possible.
- Message-only window class: register once with
  `RegisterClassExW`; `CreateWindowExW` with `HWND_MESSAGE` parent.
- Listener thread should `CoInitializeEx(COINIT_APARTMENTTHREADED)`
  if `IDataObject` paths are used (less common; basic
  `EnumClipboardFormats`/`GetClipboardData` doesn't need COM).
- `GlobalSize(handle)` is the upper bound, not the exact length —
  some formats embed their own length prefix. For text formats,
  trim trailing `\0` padding before hashing (the Windows
  `GlobalAlloc` quirk noted in 023 sub-task 1.5).

## References

- Task 023 sub-task 1.4 (the original epic — see work log).
- Task 023 sub-task 1.3 (`ditox-core/src/capture/wayland.rs`) —
  the Linux equivalent; mirror the structure where it makes
  sense (config plumbing, error mapping, dedup-by-canonical-MIME).
- Existing `ditox-gui` Win32 code in `ditox-gui/src/app.rs` —
  reference for `windows-rs` usage patterns already in the
  project.
- AGENTS.md "Windows-only (`#[cfg(windows)]`)" section —
  existing Windows dependencies and patterns.

## Work Log

### 2026-04-26 — task created (deferred from 023)

Phase 1 closed at 8/9 sub-tasks; this is sub-task 1.4 spun out into
its own task because it needs Windows for validation and the user
working on this iteration is on Linux only. The design above is
copy-paste from 023 with the additional "Watcher refactor"
prerequisite called out (the original 023 design assumed the
event-driven path could be added without touching `Watcher::run`).
