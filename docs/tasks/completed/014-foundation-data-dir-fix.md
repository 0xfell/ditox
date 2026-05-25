# Task: Honour storage.data_dir

## Status

completed

## Completed

2026-04-26

## Summary

Fixed parsed-but-ignored `storage.data_dir` configuration so all database,
image-store, watcher, and repair paths use the configured data directory.

## Work Log

- Added process-wide data-dir override handling.
- Applied the override before opening the database.
- Added soft warnings when an override starts a fresh history while a legacy DB
  exists elsewhere.
- Added tests for override path resolution.

