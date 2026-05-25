# Task: Paste-Back UX

## Status

completed

## Completed

2026-04-26

## Summary

Added foreground tracking, paste synthesis, and sentinel logic so TUI selection
can write a clip and paste it into the previously focused app when supported.

## Work Log

- Added `ForegroundTracker`, `ForegroundSnapshot`, and self-filtering.
- Added Hyprland foreground tracking.
- Added paste synthesizer chain: `hyprctl`, `wtype`, `ydotool`, and `off`.
- Added per-app keystroke parsing and overrides.
- Added `PasteSentinel` to prevent self-recapture after paste-back.
- Added `SelectionCursor` groundwork for rapid cycling.
- Verified Hyprland text paste-back manually.

