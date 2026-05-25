# Task: Multi-Format Clipboard Capture

## Status

completed

## Completed

2026-04-26

## Summary

Added multi-format clipboard storage, search, and Wayland capture.

## Work Log

- Added `entry_formats` and format-content FTS.
- Added canonical format IDs for text, HTML, RTF, URI lists, and image data.
- Added per-format hashing and rollback-safe multi-format inserts.
- Added Wayland library capture through `wl-clipboard-rs`.
- Added capture size limits and format allow/deny policy.
- Added format aggregation for multi-entry copy/paste workflows.
- Added migration and search tests.

