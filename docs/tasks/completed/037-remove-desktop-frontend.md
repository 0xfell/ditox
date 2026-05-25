# Task: Remove Desktop Frontend

## Status

completed

## Completed

2026-05-25

## Summary

Removed the desktop frontend and reset the project direction to TUI-only.
Ditox now ships only the `ditox` terminal binary, with Rust kept for native
clipboard, storage, paste-back, sync, and platform integration.

## Work Log

- Deleted the desktop crate, installer files, Linux desktop assets, smoke
  scripts, and obsolete desktop-focused notes/tasks.
- Removed desktop dependencies from the Rust workspace, Nix package, dev shell,
  CI, and release workflows.
- Removed desktop-only config and helper types from `ditox-core`.
- Updated README, roadmap, release docs, and feature docs for a TUI-only product.
- Added a planned OpenTUI frontend task for the next terminal UI generation.

## Verification

- `cargo generate-lockfile`
- `cargo fmt --all -- --check`
- `cargo test --workspace --locked`

