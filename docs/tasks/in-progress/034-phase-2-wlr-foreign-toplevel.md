# Task: wlr-foreign-toplevel Subscription

## Status

in-progress

## Summary

Add a generic Wayland foreground tracker for wlroots compositors through
`wlr-foreign-toplevel-management-v1`. Hyprland can keep the `hyprctl` fast path;
Sway and generic wlroots sessions should use the protocol tracker when exposed.

## Checklist

- [ ] Maintain a live snapshot of the focused toplevel.
- [ ] Support focus restore through compositor activation requests where allowed.
- [ ] Keep GNOME and unsupported compositors on `NoopForegroundTracker`.
- [ ] Add unit tests for event handling and snapshot updates.
- [ ] Live-test Sway paste-back from the TUI.

## Acceptance Criteria

- `ditox status` reports wlr foreground capability accurately.
- Paste-back targets the previously focused app on supported wlroots compositors.
- Unsupported sessions degrade to clipboard-only/manual paste without errors.

