# Task: Compositor / OS Detection

## Status

completed

## Completed

2026-04-26

## Summary

Added cached platform detection and capability reporting for terminal capture
and paste-back behavior.

## Work Log

- Added `Platform` and Linux compositor detection.
- Added paste-synthesis chain heuristics.
- Added foreground-tracking capability checks.
- Exposed platform details through `ditox status`.
- Added tests for Hyprland, Sway, KDE, GNOME, X11, and unknown sessions.

