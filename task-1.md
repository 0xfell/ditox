Task 1 — Single Binary, Runtime Backend Selection (Local or Turso)
=================================================================

Owner: TBD
Status: Proposed
Date: 2025-09-29

Problem
-------
- Today, remote support (Turso/libSQL) is behind a compile-time feature flag. We ship a binary that may or may not include remote support.
- README shows a split world where using Turso requires building with `--features libsql`.
- Desired: one binary that always supports both local-only and hybrid local/remote modes, selected by config at runtime: `backend = "localsqlite" | "turso"`.

Goals
-----
- Produce a single CLI binary that:
  - Always uses local SQLite as the primary, authoritative store.
  - When `backend = "turso"`, enables remote sync (push/pull) using the configured `url` and `auth_token`.
  - When `backend = "localsqlite"`, runs offline without any remote dependency.
- Retain all current commands and behavior, especially `sync status/run` via the local-first engine.

Non-Goals
---------
- Implementing remote image sync (images remain local-only for now).
- Converting the primary store to remote (we remain local-first; Turso is a sync peer).

Design Overview
---------------
- Build both local (SQLite) and remote (libSQL) support into the same binary.
- Runtime selection is driven solely by settings:
  - `[storage]
    backend = "localsqlite" | "turso"
    # when turso
    url = "libsql://<db>.turso.io"
    auth_token = "<token>"
    `
- The CLI will always open the local SQLite store for normal operations. The `sync` subcommands will consult `[storage]` to determine whether to talk to remote.
- Remove the code path that replaces the Store with a remote `LibsqlStore` when backend = turso; keep SQLite Store and use `SyncEngine` for remote.

Changes (Code)
--------------
- Enable both backends at build time by default:
  - `crates/ditox-cli/Cargo.toml`: make the `libsql` feature on by default or remove the toggle and depend on `ditox-core` with features `sqlite, clipboard, libsql`.
  - `crates/ditox-core/Cargo.toml`: keep `sqlite` and `libsql` as optional features but ensure the CLI enables both.

- Always return a local SQLite Store in the CLI:
  - File: `crates/ditox-cli/src/main.rs:515`
    - Function `build_store`: delete the `Storage::Turso` branch that constructs `LibsqlStore`; instead, always resolve a local DB path and return `StoreImpl` (SQLite).
  - File: `crates/ditox-cli/src/main.rs:555`
    - Function `build_store_readonly`: same change as above (keep local read-only access where applicable).

- Keep `SyncEngine` logic as-is (already runtime-conditional):
  - File: `crates/ditox-cli/src/main.rs:405-469`
    - `Sync { cmd }` builds the engine with `local_db`, and `url/token` only when `backend = "turso"`.
  - File: `crates/ditox-core/src/lib.rs` (module `sync`)
    - With both `sqlite` and `libsql` compiled in, `run()` pushes/pulls; otherwise it no-ops. After this task, our single binary will compile both, so push/pull work when configured.

Config & UX
-----------
- Keep current TOML schema but update docs to clarify runtime selection:

```toml
# Storage backend (runtime selection)
[storage]
backend = "localsqlite"   # or "turso"
# db_path = "/custom/path/ditox.db"   # optional; defaults to XDG path when omitted

# Remote (only when backend = "turso")
# url = "libsql://<your-db>.turso.io"
# auth_token = "<turso-token>"
```

- Behavior:
  - All commands operate on the local SQLite DB.
  - When backend = `turso`, `ditox sync run` and timers can push/pull remote text clips.
  - `ditox migrate` remains local-only and unmodified.
  - `ditox doctor` may optionally ping remote (nice-to-have follow-up).

CI & Packaging
--------------
- Ensure the default build includes both features:
  - GitHub Actions: no extra flags if we set CLI default features to include `libsql`.
  - Nix flake `flake.nix`: no change needed if default features compile in both backends.

Testing
-------
- Unit/Integration:
  - Ensure `build_store()` returns SQLite even when `[storage.backend] = "turso"` (assert via `config --json` and a quick list/add).
  - Ensure `sync status` shows `remote_ok: Some(true)` when `TURSO_URL/AUTH_TOKEN` are set and the binary is built with both backends.
  - Existing env-gated sync smoke test (`crates/ditox-core/tests/sync_libsql.rs`) continues to pass.

Docs to Update
--------------
- `README.md`
  - Remove instructions that require `--features libsql` to enable remote.
  - Clarify runtime selection and that the primary store is always local SQLite.
- `PRD.md`
  - Reflect single-binary runtime selection and keep images local-only.

Risks
-----
- Binary size increase (tokio/libsql included). Acceptable for Linux-first target.
- Additional dynamic linking considerations for libSQL; test in CI.

Rollout Plan
------------
1) Flip CLI features to include `libsql` by default and pin core dependency features (`sqlite, clipboard, libsql`).
2) Simplify `build_store()`/`build_store_readonly()` to always use SQLite.
3) Verify unit + CLI tests; update docs and CI artifacts.
4) Tag `rust-v0.1.1` with release notes (out of scope for this task file).

Acceptance Criteria
-------------------
- A single binary supports both modes at runtime:
  - With default `[storage.backend = "localsqlite"]` → all operations work offline against local DB.
  - With `[storage.backend = "turso"]` and valid credentials → `ditox sync run` pushes/pulls; day-to-day commands still operate against local DB.
- No compile-time flags are required by users to enable remote mode.
