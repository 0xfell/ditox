# Task: Structured Tracing

## Status

completed

## Completed

2026-04-26

## Summary

Moved internal diagnostics to `tracing` while preserving structured CLI output
through `println!`.

## Work Log

- Added shared logging initialization.
- Added stderr, file, and journald modes.
- Replaced diagnostic `eprintln!` calls with structured events.
- Documented `RUST_LOG` usage.

