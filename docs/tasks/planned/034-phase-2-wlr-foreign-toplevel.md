# Task: Phase 2 — `wlr-foreign-toplevel-management` subscription

> **Status:** planned
> **Priority:** medium (improves non-Hyprland Wayland support)
> **Phase:** 2 — Paste-back UX (carry-over)
> **Created:** 2026-04-26
> **Spawned from:** task 024 sub-task 2.3 (continuation)
> **Estimated:** 1 week

## Why this is its own task

Task 024 closed on 2026-04-26 with the Linux paste-back MVP working
end-to-end on **Hyprland**, courtesy of the
`HyprctlForegroundTracker` (Hyprland-specific IPC: `hyprctl
activewindow -j` + `hyprctl dispatch focuswindow`). The original 2.3
spec also called for a generic `wlr-foreign-toplevel-management-v1`
subscriber that works on any wlroots-based compositor (Sway, river,
Hyprland-as-fallback, Wayfire); that part was deferred because:

1. It needs a dedicated `wayland-client` event-loop thread, which is
   a non-trivial integration with the existing sync trait and bumps
   ditox-core's dependency footprint with an `sctk` (or
   `wayland-client` direct).
2. Hyprland users are already covered by the IPC-based tracker; the
   marginal user is a Sway / river user, and `wtype`-based synthesis
   already lands when `pick_chain` falls past the `hyprctl`
   synthesizer.
3. The subscription delivers `mpsc::Receiver<ForegroundSnapshot>` —
   useful for Phase 4's daemon mode, less critical for one-shot
   launchers (snapshot-on-demand is enough).

Tracking it as its own task keeps the option open without polluting
the Phase 2 close-out.

## Description

Implement a `WlrForegroundTracker` against
`wlr-foreign-toplevel-management-unstable-v1`. The protocol exposes:

- `zwlr_foreign_toplevel_manager_v1` (registry global) — emits a
  `zwlr_foreign_toplevel_handle_v1` for every existing toplevel and
  every new one.
- Per-handle events: `title`, `app_id`, `output_enter`,
  `output_leave`, `state` (where `state` includes the
  `Activated` flag — that's "this is the focused window").
- Per-handle requests: `activate(seat)` — non-privileged client can
  request focus; the compositor decides whether to honour. Sway
  honours it; Hyprland honours it; GNOME doesn't ship the protocol.

The tracker maintains a `HashMap<ZwlrForegroundToplevelHandle,
ToplevelInfo>` and updates the "currently focused" pointer on each
`state` event with `Activated`.

## Architecture

```rust
// ditox-core/src/foreground/wlr.rs
pub struct WlrForegroundTracker {
    /// Last-known focused snapshot. Updated by the wayland thread,
    /// read by `snapshot()`.
    current: Arc<Mutex<Option<ForegroundSnapshot>>>,
    /// Handle to the wayland thread for shutdown.
    thread: Option<JoinHandle<()>>,
    /// Send side of the subscription channel. The wayland thread
    /// pushes every new focused snapshot here; subscribers receive
    /// them via the channel returned from `subscribe()`.
    subscriber_tx: Option<mpsc::Sender<ForegroundSnapshot>>,
}

impl WlrForegroundTracker {
    pub fn new() -> Result<Self, ForegroundError> {
        let conn = Connection::connect_to_env()?;
        let (queue, manager) = bind_zwlr_foreign_toplevel_manager(&conn)?;
        let current = Arc::new(Mutex::new(None));
        let (tx, _rx_drop) = mpsc::channel();

        let current_clone = Arc::clone(&current);
        let thread = std::thread::Builder::new()
            .name("ditox-wlr-fg".into())
            .spawn(move || run_event_loop(queue, manager, current_clone, tx))?;

        Ok(Self {
            current,
            thread: Some(thread),
            subscriber_tx: None,
        })
    }
}

impl ForegroundTracker for WlrForegroundTracker {
    fn name(&self) -> &str { "wlr-foreign-toplevel" }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        Ok(self.current.lock().unwrap().clone())
    }

    fn restore(&self, snap: &ForegroundSnapshot) -> Result<()> {
        let ForegroundId::Wlr { handle_id, .. } = snap.identifier else {
            return Err(ForegroundError::WrongIdentifierVariant);
        };
        // Look up the live handle (handle_id is the protocol-level
        // wl_proxy id captured at snapshot time; if the toplevel
        // was destroyed in the interim, return Ok(()) silently).
        let Some(handle) = self.lookup_handle(handle_id) else {
            return Ok(());
        };
        let seat = self.first_seat()?;
        handle.activate(&seat);
        // Round-trip flush so the activate request is dispatched.
        self.connection.flush()?;
        Ok(())
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        let (tx, rx) = mpsc::channel();
        self.subscriber_tx = Some(tx);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        // Drop the connection; the wayland thread observes the
        // disconnect and exits.
        if let Some(t) = self.thread.take() {
            // Best-effort join; thread should exit within milliseconds
            // of Connection drop.
            let _ = t.join();
        }
        Ok(())
    }
}
```

The wayland thread runs a normal `wl_event_queue::dispatch` loop;
each `state` event with `Activated` flips the `current` snapshot and
notifies subscribers (if any). The handle's protocol-level id is
preserved in `ForegroundId::Wlr { handle_id, app_id, title }` so
`restore()` can look it up and call `activate(seat)`.

`ForegroundId::Wlr { handle_id }` already exists in
`ditox-core/src/foreground.rs` but holds an `String`; this task
should bump it to whatever opaque identifier the wayland-client
crate uses (an `ObjectId` or a strongly-typed proxy wrapper).
`ForegroundId::supports_restore()` flips to `true` for `Wlr` once
this lands (today it's `false`).

## Wiring

`ditox-core/src/foreground.rs::build_default_tracker`:

```rust
#[cfg(unix)]
{
    if matches!(detect_platform(), Platform::Hyprland) {
        return Box::new(ForegroundFilter::new(
            HyprctlForegroundTracker::new(),
            ForegroundFilter::default_self_names(),
        ));
    }
    if let Ok(tracker) = WlrForegroundTracker::new() {
        return Box::new(ForegroundFilter::new(
            tracker,
            ForegroundFilter::default_self_names(),
        ));
    }
    // Unsupported (GNOME without extension, etc.) — clipboard write
    // works, restore + paste-back synthesis don't.
    Box::new(NoopForegroundTracker::new())
}
```

The Hyprland tracker stays preferred because `hyprctl dispatch
focuswindow` is more reliable than `wlr-foreign-toplevel`'s
`activate` (which Hyprland still honours but goes through more
internal state).

## Acceptance criteria

- [ ] Sway: summon ditox-gui via a Sway keybind; click an entry;
      text appears in the previously-focused app.
- [ ] River: same.
- [ ] Hyprland fallback: when `hyprctl` binary is missing,
      `WlrForegroundTracker` provides the snapshot — verified via
      `RUST_LOG=ditox=debug` log line `using wlr foreign-toplevel
      tracker (Hyprland fallback)`.
- [ ] GNOME Wayland (no protocol): `WlrForegroundTracker::new()`
      returns `Err`; `build_default_tracker` falls through to
      `NoopForegroundTracker`; paste-back degrades to clipboard-only
      with a clear log message.
- [ ] Subscription channel emits each focus transition exactly
      once; verified by unit test using a mock `wayland-client` (or
      integration test against a headless `wlroots`).

## Dependencies

- `wayland-client = "0.31"` workspace dep (already present transitively
  via `wl-clipboard-rs`; verify or add explicitly).
- Optional: `smithay-client-toolkit = "0.18"` for higher-level
  bindings (nice-to-have; not strictly required since this protocol
  is small).
- Generated bindings for `wlr-foreign-toplevel-management-v1.xml`
  (use `wayland-scanner` build script in `ditox-core/build.rs`).

## Risks

- **Risk:** Sway's `activate` request is gated behind a security
  policy in some configurations; a failed activate is silent. The
  paste-back keystroke synthesis would land in the wrong window.
  Mitigation: the synthesis layer uses `wtype` (untargeted), so
  whichever app the compositor focuses gets the paste; document the
  failure mode.
- **Risk:** Stale `handle_id` after the toplevel is destroyed
  between `snapshot()` and `restore()`. Mitigation: `restore`
  silently no-ops when the lookup fails (the snapshot's window is
  gone; nothing to restore).
- **Risk:** GNOME Wayland doesn't ship the protocol; an attempt to
  bind the global returns "not advertised". Mitigation: the
  fall-through to `NoopForegroundTracker` already handles this; the
  tracker constructor returns a clear error.

## Implementation Notes

The wayland event-loop thread uses a regular `mpsc::channel()` to
push focused-snapshot events to subscribers. Subscribers are
expected to be lightweight consumers; if they fall behind, the send
fails (channel full / receiver dropped) and the tracker drops the
event silently — that's correct behaviour for foreground tracking
(stale events are noise).

For the launcher's `snapshot()` call, the `Mutex<Option<...>>` read
is the canonical "what's focused right now" — no need to poll the
subscription channel from the launcher.

When this lands, also bump
`ForegroundId::supports_restore()` to `true` for `Wlr` and update
the doc comment matrix in `ditox-core/src/foreground.rs:42-58`.
