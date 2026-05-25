# Task: Watcher Daemon Hardening

## Status

completed

## Completed

2026-04-26

## Summary

Hardened `ditox watch` into a managed daemon with locking, status, stop, and
heartbeat support.

## Work Log

- Added flock-based single-instance protection.
- Added atomic heartbeat writes and stale-process detection.
- Added SIGTERM/SIGINT cleanup.
- Added `ditox watch --stop`, `--status`, `--json`, and `--journal`.
- Added a systemd user unit.
- Added tests for lock/status behavior.

