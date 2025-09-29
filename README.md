```
DDDDDDDDDDDDD          iiii          tttt
D::::::::::::DDD      i::::i      ttt:::t
D:::::::::::::::DD     iiii       t:::::t
DDD:::::DDDDD:::::D               t:::::t
D:::::D    D:::::D iiiiiiittttttt:::::ttttttt       ooooooooooo xxxxxxx      xxxxxxx
D:::::D     D:::::Di:::::it:::::::::::::::::t     oo:::::::::::oox:::::x    x:::::x
D:::::D     D:::::D i::::it:::::::::::::::::t    o:::::::::::::::ox:::::x  x:::::x
D:::::D     D:::::D i::::itttttt:::::::tttttt    o:::::ooooo:::::o x:::::xx:::::x
D:::::D     D:::::D i::::i      t:::::t          o::::o     o::::o  x::::::::::x
D:::::D     D:::::D i::::i      t:::::t          o::::o     o::::o   x::::::::x
D:::::D     D:::::D i::::i      t:::::t          o::::o     o::::o   x::::::::x
D:::::D    D:::::D  i::::i      t:::::t    tttttto::::o     o::::o  x::::::::::x
DDD:::::DDDDD:::::D  i::::::i     t::::::tttt:::::to:::::ooooo:::::o x:::::xx:::::x
D:::::::::::::::DD   i::::::i     tt::::::::::::::to:::::::::::::::ox:::::x  x:::::x
D::::::::::::DDD     i::::::i       tt:::::::::::tt oo:::::::::::oox:::::x    x:::::x
DDDDDDDDDDDDD        iiiiiiii         ttttttttttt     ooooooooooo xxxxxxx      xxxxxxx
```

# Ditox — Clipboard History for Developers (CLI + Core)

Note: Docs below use the command name `ditox` for readability. If you installed from source without a wrapper, your binary name may be `ditox-cli` — use that instead (e.g., `ditox-cli list`).

Ditox is a fast, scriptable clipboard history with a focus on reliability, privacy, and great CLI ergonomics. It targets Linux first (X11/Wayland via `arboard`) and is designed as a Rust workspace with a reusable core library and a small command‑line tool.

- Core features:
    - Add/list/search text clips; JSON output for scripting.
    - Favorites and retention pruning by age or count.
    - Image groundwork: add/list/info/copy image clips; file‑backed blobs.
    - SQLite store with FTS5 when available; LIKE fallback otherwise.
    - Optional remote backend (libSQL/Turso) behind a feature flag.
    - Self‑check (`doctor`) and explicit migrations (`migrate`).

- Status: v0.1.x, Linux‑first. Clipboard adapters for other OSes will land later; the CLI builds but clipboard IO is a no‑op outside Linux.

## Quick Start

- Build the CLI:
    - `cargo build -p ditox-cli` (debug)
    - `cargo build --release -p ditox-cli` (release)
- Run it:
    - `cargo run -p ditox-cli -- list`

The default database lives at `~/.config/ditox/db/ditox.db` (XDG). You can override with `--db <path>`.

## Installation

- Cargo (local checkout):
    - `cargo install --path crates/ditox-cli`
- Nix (flake):
    - `nix build -L .#ditox` → binary at `result/bin/ditox-cli`
    - `nix run .#ditox -- --help`
- CI artifacts: see GitHub Actions workflows under `.github/workflows/` for prebuilt tarballs produced on tags.

Optional features:

- libSQL/Turso remote store: build CLI with `--features libsql` on `ditox-cli` to enable the remote backend path (`[storage.backend = "turso"]`).

## Usage

Initialize a database (explicit, safe to run multiple times):

- `ditox init-db`

Add text (argument or STDIN):

- `echo "hello world" | ditox add`
- `ditox add "token: abc123"`

List recent entries:

- `ditox list`
- `ditox list --json` (machine‑readable)
- `ditox list --favorites`

Search by substring or FTS5 if available:

- `ditox search error`
- `ditox search error --json`

Favorite / unfavorite:

- `ditox favorite <id>`
- `ditox unfavorite <id>`

Copy back to clipboard:

- `ditox copy <id>`
    - Linux only. On non‑Linux builds the clipboard adapter is a no‑op.

Show details for an entry:

- `ditox info <id>`

Prune history (by count, age; favorites kept by default):

- `ditox prune --max-items 2000`
- `ditox prune --max-age 30d`
- `ditox prune --max-items 0 --keep-favorites` (keep only favorites)

Migrations (SQLite):

- `ditox migrate --status`
- `ditox migrate --backup` (copy `.db` → `.bak.<timestamp>` then apply pending migrations)

Doctor (environment/store check):

- `ditox doctor`

Images (scaffolded, Linux):

- Add from file: `ditox add --image-path ./foo.png`
- Add from clipboard: `ditox add --image-from-clipboard`
- List images: `ditox list --images [--json]`
- Copy image to clipboard: `ditox copy <id>` (for image clips)

## Command Reference

Global options (apply to all subcommands):

| Option                | Default                           | Purpose                       |
| --------------------- | --------------------------------- | ----------------------------- | -------------------------------------------------------------------------- |
| `--store <sqlite      | mem>`                             | `sqlite`                      | Choose backend (SQLite on disk, or in‑memory `mem`).                       |
| `--db <path>`         | XDG `~/.config/ditox/db/ditox.db` | Path to SQLite database file. |
| `--auto-migrate[=true | false]`                           | `true`                        | Apply pending DB migrations on startup (set `false` for managed rollouts). |

Subcommands:

| Command      | Synopsis                                                         | Description                                                                                     |
| ------------ | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `init-db`    | `ditox init-db [--db <path>]`                                    | Initialize the local database (no‑op if already initialized).                                   |
| `add`        | `ditox add [TEXT] [--image-path <file>                           | --image-from-clipboard]`                                                                        | Add a new entry. Without `TEXT`, reads from STDIN. Use image flags to store an image clip.                       |
| `list`       | `ditox list [--favorites] [--images] [--limit N] [--json]`       | List recent text clips (default) or images with `--images`. Use `--json` for scripting.         |
| `search`     | `ditox search <QUERY> [--favorites] [--json]`                    | Search text clips (FTS5 if available, otherwise `LIKE`).                                        |
| `favorite`   | `ditox favorite <ID>`                                            | Mark an entry as favorite.                                                                      |
| `unfavorite` | `ditox unfavorite <ID>`                                          | Remove favorite flag.                                                                           |
| `copy`       | `ditox copy <ID>`                                                | Copy the entry back to the system clipboard (text or image; Linux only).                        |
| `delete`     | `ditox delete [ID]`                                              | Delete a specific entry; with no `ID`, clear all entries.                                       |
| `info`       | `ditox info <ID>`                                                | Show details about an entry (for images: format, size, dimensions, sha256).                     |
| `prune`      | `ditox prune [--max-items N] [--max-age 30d] [--keep-favorites]` | Prune by count and/or age. Favorites are kept by default; `--keep-favorites` makes it explicit. |
| `migrate`    | `ditox migrate [--status                                         | --backup]`                                                                                      | `--status` prints current/latest/pending; `--backup` makes a `.bak.<timestamp>` then applies pending migrations. |
| `doctor`     | `ditox doctor`                                                   | Self‑check for clipboard access and search capability.                                          |
| `config`     | `ditox config [--json]`                                          | Print effective configuration and paths.                                                        |

## Configuration

Settings live at `~/.config/ditox/settings.toml` by default. The CLI reads this file and merges it with flags.

Example:

```toml
# ~/.config/ditox/settings.toml

# Storage backend (default: local sqlite)
[storage]
backend = "localsqlite"
# db_path = "/custom/path/ditox.db"   # optional override of the XDG default

# Alternative remote backend (requires building CLI with --features libsql)
# [storage]
# backend = "turso"
# url = "libsql://<your-db>.turso.io"
# auth_token = "<turso-token>"

# Retention policy
[prune]
# every = "7d"            # used by the optional systemd timer installer
keep_favorites = true
max_items = 10000
max_age = "90d"

# Optional storage budget (future use)
max_storage_mb = 512
```

Effective paths/config can be printed with:

- `ditox config` (pretty)
- `ditox config --json`

## Data Layout

- Database: `~/.config/ditox/db/ditox.db` by default.
- Image blobs: content‑addressed files under the database directory: `~/.config/ditox/db/objects/aa/bb/<sha256>`.
- Migrations: embedded SQL files in `crates/ditox-core/migrations/` (`NNNN_description.sql`).

FTS5 is used when available (see `0002_fts.sql`). If not available, search uses a `LIKE` fallback. `ditox doctor` reports capability.

## Systemd Timer (optional prune)

A convenience script installs a per‑user systemd timer to run `ditox prune` on a cadence derived from `settings.toml`:

- `scripts/install_prune_timer.sh`
    - Generates `~/.config/systemd/user/ditox-prune.*`, enables and starts the timer.
    - Uses `[prune].every` (e.g., `7d`) from your settings; defaults to `7d`.
    - Inspect with `systemctl --user status ditox-prune.timer`.

See `contrib/systemd/README.md` for details.

## Building From Source

- Workspace build: `cargo build`
- CLI only: `cargo build -p ditox-cli`
- With all features for testing: `cargo build --workspace --all-features`
- Lint and format: `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
- Tests: `cargo test --all`
    - Clipboard E2E image test runs on Linux only and is guarded by `DITOX_E2E_CLIPBOARD=1`.

Nix dev shell (with Rust, clippy, rustfmt, X11/Wayland headers):

- `nix develop -c $SHELL`

## Architecture

- `crates/ditox-core/` (library)
    - Domain types (`Clip`, `ClipKind::{Text, Image}`, `ImageMeta`).
    - `Store` trait with in‑memory and SQLite implementations.
    - SQLite backend (WAL, migrations, optional FTS5), content‑addressed blob store for images.
    - Clipboard adapters behind features (Linux uses `arboard`).
    - Optional libSQL/Turso backend behind `libsql` feature.
- `crates/ditox-cli/` (binary)
    - Thin CLI over the core: `add`, `list`, `search`, `copy`, `favorite`/`unfavorite`, `info`, `delete`, `prune`, `migrate`, `doctor`, `config`.
    - XDG paths for config/DB; TOML settings.

## Security & Privacy

- Clipboard history can contain secrets. Ditox avoids logging content and provides JSON export and pruning tools.
- Default store is local SQLite; remote sync is off by default and gated by a feature + explicit configuration.
- Settings are plain‑text TOML; the installer script hardens permissions to `0600` when present.

## Contributing

We welcome issues and PRs. Before opening a PR:

- Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all`.
- For DB changes, add a new migration in `crates/ditox-core/migrations/` named `NNNN_description.sql`.
- Use Conventional Commits (e.g., `feat(core): sqlite FTS search`, `fix(cli): handle empty stdin`).
- Mention any CLI flags or migrations you touched in the PR description.

## Troubleshooting

- FTS not working: Your SQLite may lack FTS5. Searching will fall back to `LIKE`. Check with `ditox doctor`.
- Wayland/X11 clipboard issues: Some environments restrict clipboard access. `ditox doctor` prints a quick probe. For images, ensure a running graphical session.
- DB path: Use `--db /path/to/ditox.db` to isolate tests or try out data in a temp dir.

## License

Dual‑licensed under either of:

- MIT license
- Apache License, Version 2.0

as specified in the crate manifests.

## Project Layout

- Workspace root: `Cargo.toml`, `.gitignore`, `PRD.md`.
- Core library: `crates/ditox-core/` (migrations in `migrations/`).
- CLI: `crates/ditox-cli/`.
- Tests: `crates/*/tests/`.
- Nix: `flake.nix`, `flake.lock` (see `nix.md`).
- Systemd integration: `contrib/systemd/` and `scripts/install_prune_timer.sh`.
