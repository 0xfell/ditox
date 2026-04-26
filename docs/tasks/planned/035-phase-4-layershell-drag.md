# Task: Phase 4 — wlr-layer-shell drag handle

> **Status:** planned
> **Priority:** medium (UX polish; not blocking)
> **Phase:** 4 — Ditto UX (carry-over)
> **Created:** 2026-04-26
> **Spawned from:** task 026 sub-task 4.5
> **Estimated:** 1 week

## Why this is its own task

Task 026 closed at 11/12 sub-tasks on 2026-04-26 with the daemon
model + layer-shell launcher fully functional. Sub-task 4.5 (drag
handle for layer-shell windows) was deferred because:

1. The existing `Message::StartDrag` → `iced::window::drag` path
   already works for xdg_toplevel platforms (Windows, GNOME
   Wayland, X11). Layer-shell users on Hyprland / Sway / wlroots
   have a config-driven alternative via `[gui.position]` (sub-task
   4.4) — they can pin the launcher to a fixed corner + offset
   without runtime drag.
2. Implementing drag for layer-shell requires bespoke glue: window-
   local cursor tracking via `iced::event::listen_with`, manual
   delta math + `MarginChange` emission per mouse-move while
   dragging, and anchor-aware delta-to-margin translation.
3. Visual testing is essential to make the feature feel right
   (cursor discontinuity, accidental drags from list scrolls, ESC
   to cancel partway through, etc.).

Spinning it out as a focused task keeps the Phase 4 close clean.

## Description

Implement an internal drag handle for layer-shell windows. Drag the
title bar → update the layer-surface margin via `MarginChange`. The
visible effect is the launcher repositioning under the user's
pointer, just like dragging an xdg_toplevel.

## Architecture

`ditox-gui/src/app.rs`:

```rust
struct DragState {
    /// Window-local cursor position at the moment of press.
    press_local: Point,
    /// Margin tuple at the moment of press (top, right, bottom, left).
    margin_at_press: (i32, i32, i32, i32),
    /// Anchor flags at the moment of press; controls how delta
    /// translates to margin update.
    anchor: iced_layershell::reexport::Anchor,
}

struct DitoxApp {
    // ... existing fields ...

    /// Phase 4 sub-task 4.5: layer-shell drag-by-margin state.
    /// `None` when not dragging.
    drag_state: Option<DragState>,

    /// Last known cursor position (window-local). Updated by the
    /// `Event::Mouse(CursorMoved)` subscription. We need to know
    /// where the cursor was AT the moment of `TitleDragStart`,
    /// since `mouse_area::on_press` doesn't carry the position.
    last_cursor_local: Option<Point>,

    /// Cached layer-shell margin so we can compute deltas without
    /// querying iced_layershell. Initialised from
    /// `layer_anchor_and_margin_for(config.gui.position).1`.
    /// Updated on every `MarginChange` we emit.
    current_margin: (i32, i32, i32, i32),
}
```

New messages:

```rust
enum Message {
    // ... existing variants ...

    /// Mouse pressed on the draggable title strip.
    TitleDragStart,
    /// Cursor moved (in window-local coords). Subscription emits
    /// continuously; handler is a no-op when not dragging.
    CursorMovedLocal(Point),
    /// Mouse released anywhere. Subscription emits unconditionally;
    /// handler clears `drag_state` if set.
    DragEnd,
}
```

Subscription additions:

```rust
let mouse_sub = event::listen_with(|event, _, _| match event {
    Event::Mouse(mouse::Event::CursorMoved { position }) => {
        Some(Message::CursorMovedLocal(position))
    }
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
        Some(Message::DragEnd)
    }
    _ => None,
});
```

Handlers:

```rust
Message::TitleDragStart => {
    let press = self.last_cursor_local.unwrap_or(Point::ORIGIN);
    self.drag_state = Some(DragState {
        press_local: press,
        margin_at_press: self.current_margin,
        anchor: self.current_anchor, // tracked alongside current_margin
    });
}

Message::CursorMovedLocal(p) => {
    self.last_cursor_local = Some(p);
    if let Some(state) = self.drag_state.as_ref() {
        let delta = (p.x - state.press_local.x, p.y - state.press_local.y);
        let new_margin = compute_dragged_margin(state.anchor, state.margin_at_press, delta);
        if new_margin != self.current_margin {
            self.current_margin = new_margin;
            return Task::done(Message::MarginChange(new_margin));
        }
    }
}

Message::DragEnd => {
    self.drag_state = None;
}
```

`compute_dragged_margin(anchor, margin_at_press, delta_xy)` does the
anchor-aware translation:

- `Anchor::Left` set → `new_margin.3 (left) = margin_at_press.3 + delta.x`.
- `Anchor::Right` set → `new_margin.1 (right) = margin_at_press.1 - delta.x`.
- `Anchor::Top` set → `new_margin.0 (top) = margin_at_press.0 + delta.y`.
- `Anchor::Bottom` set → `new_margin.2 (bottom) = margin_at_press.2 - delta.y`.
- Anchors not set → corresponding margin unchanged (centred axis
  doesn't participate in drag math).

## Acceptance criteria

- [ ] On Hyprland: press + drag the launcher's title bar → window
      follows the cursor smoothly. Release → window stays at the
      drop position.
- [ ] On Sway: same.
- [ ] Window-local cursor coords match user expectations: dragging
      right moves the window right (Bottom-Left anchor: increases
      `left` margin → window moves right).
- [ ] Drag from a non-title-bar area (e.g. the entry list) does
      NOT initiate a drag. Only the title-bar `mouse_area`'s
      `on_press` triggers `TitleDragStart`.
- [ ] Esc during drag cancels and restores the pre-drag margin.
- [ ] After drag end, `Config.gui.position = at_previous` saves
      the new geometry to `window_state.json` so subsequent
      summons reuse it.
- [ ] xdg_toplevel platforms unaffected — the existing
      `Message::StartDrag` → `iced::window::drag` path stays for
      Windows / GNOME Wayland / X11.

## Risks

- **Risk:** Wayland clients can't query screen-global cursor
  coords, only window-local. The drag math relies on window-local
  delta = screen-local delta (true while the window itself isn't
  moving during the drag-update sequence; iced_layershell's
  `MarginChange` round-trips through the compositor before the
  window position actually updates, so the next CursorMoved event
  reflects the correct delta from the press point).
- **Risk:** Some compositors throttle MarginChange round-trips.
  Mitigation: debounce per-frame (16 ms) so we don't flood the
  socket with identical-or-near-identical margin updates.
- **Risk:** Drag math is wrong for centred anchors. Mitigation:
  document that drag is only meaningful for corner-anchored
  layouts; centre-anchored windows ignore drag (margin unchanged).

## Implementation Notes

iced 0.14's `mouse_area::on_press` doesn't carry the press
position. We work around this by tracking the last-known
window-local cursor position in `last_cursor_local` (updated
continuously by the subscription) and reading it inside the
`TitleDragStart` handler.

`current_margin` and `current_anchor` mirror the layer-shell
state. They're updated whenever we emit a `MarginChange` /
`AnchorChange`. Initial values come from
`layer_anchor_and_margin_for(config.gui.position)` at boot.

The `MarginChange` message variant is auto-generated by the
`#[iced_layershell::to_layer_message]` attribute on
`Message`. Catch-all `_ => {}` already soaks it up; we just need
to construct it via `Task::done(Message::MarginChange(...))`.

A small visual cue in the cursor (e.g. switch to `move` cursor
shape) on hover over the title bar would help users discover
the drag affordance — out of scope for the initial commit.
