# Task: Image Storage Bug Fix

## Status

completed

## Completed

2026-04-25

## Summary

Fixed image-store leaks and made image blobs content-addressed.

## Work Log

- Added `images/{hash[..2]}/{hash}.{ext}` content-addressed layout.
- Added `entries.image_extension`.
- Added atomic image writes with temp file, fsync, rename, and parent fsync.
- Added prune queue for row-delete/blob-delete crash recovery.
- Added `ditox repair` for dangling rows, orphan files, stale tmp files, and
  hash mismatch quarantine.
- Updated TUI and CLI call sites to resolve image paths through `Entry`.
- Added core and CLI integration tests.

