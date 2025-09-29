Changelog
=========

All notable changes to this project will be documented in this file.
The format is based on Keep a Changelog, and this project adheres to
Semantic Versioning where practical.

Unreleased
----------
- Planned: background daemon (`clipd`), interactive picker, import/export tools.
- Planned: image thumbnails/filters, remote image strategy, improved conflict tooling.

0.1.0 – 2025-09-29
------------------

Added
- Core library (`ditox-core`) with SQLite backend (WAL) and embedded SQL migrations.
- Full text search via FTS5 when available; automatic LIKE fallback otherwise.
- Image support (local‑only):
  - `ClipKind::Image`, `ImageMeta`, and RGBA handling in core.
  - Content‑addressed PNG blobs stored on disk under `objects/aa/bb/<sha256>`.
  - CLI commands: `add --image-path`, `add --image-from-clipboard` (Linux), `list --images`, `info <id>`, `copy <id>`.
- CLI commands for text flow: `add`, `list`, `search`, `copy`, `favorite`/`unfavorite`, `delete`, `info`.
- Retention management: `prune [--max-items N] [--max-age DUR] [--keep-favorites]`.
- Configuration: `~/.config/ditox/settings.toml`; `ditox config [--json]` to view effective config.
- Database migrations UX: `ditox migrate --status` and `--backup`.
- Doctor command: probes clipboard access and search capability.
- Optional remote sync (feature‑gated `libsql`):
  - Local‑first sync engine with push/pull to Turso/libSQL for text rows.
  - Conflict resolution via `(lamport, updated_at, device_id)` tuple.
  - `ditox sync status` (last push/pull, pending local, counts, remote ping, last error) and `ditox sync run`.
- Systemd integration: user timers and installers for prune and sync.
- Nix flake: package build (`.#ditox`) and a dev shell; CI workflow for Nix builds.
- Test suite: core and CLI smoke tests; image round‑trip; migration checks; env‑gated libsql sync smoke.

Changed
- CLI and core cleaned up for clippy; Linux clipboard behind `arboard`; non‑Linux clipboard is a no‑op adapter.
- README and PRD updated to reflect current image + sync capabilities.

Notes
- Images are intentionally excluded from remote sync in this release.
- Release workflow expects tags like `rust-v0.1.0` for packaging.

