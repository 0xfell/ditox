# Ditox

A terminal-first clipboard manager for Wayland desktops.

- **TUI** (`ditox`) - keyboard-driven terminal UI and full CLI.
- **Watcher** - lightweight clipboard daemon with SHA-256 deduplication.
- **Native core** - Rust storage, clipboard capture, paste-back, image handling, and sync.

Ditox is now a TUI-only project. The next frontend generation is planned around
Bun, TypeScript, SolidJS, and OpenTUI, with Rust kept for native OS services.

## Features

- Text and image capture, including browser "Copy image" cases where the clipboard also contains a URL.
- Content-addressed image store with atomic writes and refcount-based pruning.
- Full-text search, fuzzy search, and regex search modes.
- Named collections, favorites, notes, quick-snippet slots, tags, and multi-select.
- Pagination-aware lists; large histories stay fast.
- Inline terminal image previews through kitty, sixel, iTerm2, or half-block rendering.
- Wayland clipboard integration via `wl-clipboard-rs`.
- Systemd user service and Home Manager module.
- `ditox repair` for image-store reconciliation.

## Install

### Nix

```sh
nix run github:0xfell/ditox
nix profile install github:0xfell/ditox
```

With Home Manager:

```nix
{
  inputs.ditox.url = "github:0xfell/ditox";

  outputs = { self, nixpkgs, ditox, home-manager, ... }: {
    homeConfigurations.you = home-manager.lib.homeManagerConfiguration {
      modules = [
        ditox.homeManagerModules.default
        {
          programs.ditox = {
            enable = true;
            systemd.enable = true;
            settings = {
              general.max_entries = 1000;
              general.poll_interval_ms = 250;
              ui.show_preview = true;
            };
          };
        }
      ];
    };
  };
}
```

Release builds are pushed to [cachix.org/ditox](https://app.cachix.org/cache/ditox):

```sh
nix run --option extra-substituters https://ditox.cachix.org \
        --option extra-trusted-public-keys ditox.cachix.org-1:kVMmDqje/wWu/ChZfMzWdqduZlBptE7LoHZ3lTFdxg8= \
        github:0xfell/ditox
```

### Prebuilt Binaries

Artifacts live at <https://github.com/0xfell/ditox/releases/latest>.

```sh
curl -L https://github.com/0xfell/ditox/releases/latest/download/ditox-x86_64-linux-musl.tar.gz | tar xz
sudo install ditox /usr/local/bin/
```

Install `wl-clipboard` from your distro for shell interoperability:

```sh
sudo apt install wl-clipboard
sudo pacman -S wl-clipboard
sudo dnf install wl-clipboard
```

### Build From Source

```sh
git clone https://github.com/0xfell/ditox
cd ditox
nix develop
cargo build --release --workspace
```

The release binary is `target/release/ditox`.

## Usage

```sh
ditox                 # browse history
ditox watch           # start the clipboard watcher
```

Common TUI keys:

| Key | Action |
|---|---|
| `j`/`k`, `up`/`down` | Move selection |
| `g` / `G` | Top / bottom |
| `Enter` | Copy and quit |
| `y` | Copy and stay open |
| `Tab` | Toggle preview pane |
| `/` | Fuzzy search |
| `Ctrl+R` | Regex search |
| `d` / `D` | Delete / clear with confirmation |
| `n` | Edit note |
| `m`, `Space`, `v` | Multi-select mode |
| `?` | Help overlay |
| `q` | Quit |

CLI examples:

```sh
ditox list [--limit N] [--json] [--favorites]
ditox get <n|id> [--json]
ditox search <query> [--limit N] [--json]
ditox copy <n|id>
ditox delete <n|id>
ditox favorite <n|id>
ditox clear [--confirm]
ditox status
ditox stats [--json]
ditox repair [--dry-run] [--fix-hashes]
ditox collection list|create|delete|rename|add|remove|show
ditox sync discover|pull|peers|log|trust|reject|untrust|auto-send
```

## Configuration

`~/.config/ditox/config.toml`:

```toml
[general]
max_entries = 500
poll_interval_ms = 250

[ui]
show_preview = true
date_format = "relative"
# graphics_protocol = "kitty" # kitty | sixel | iterm2 | halfblocks

[ui.theme]
selected = "#7aa2f7"
border = "#565f89"
text = "#c0caf5"
muted = "#565f89"
```

## Data Locations

| Item | Linux path |
|---|---|
| Database | `~/.local/share/ditox/ditox.db` |
| Images | `~/.local/share/ditox/images/` |
| Config | `~/.config/ditox/config.toml` |
| Watcher PID | `~/.local/share/ditox/watcher.pid` |

## Project Docs

- [docs/ROADMAP.md](docs/ROADMAP.md) - current roadmap and OpenTUI pivot.
- [docs/features.md](docs/features.md) - current feature surface.
- [docs/notes/image-storage.md](docs/notes/image-storage.md) - image storage protocol.
- [docs/RELEASING.md](docs/RELEASING.md) - release process.

## Contributing

1. `nix develop` or install Rust and Wayland development packages manually.
2. Run `scripts/check-no-root-tests.sh`.
3. Run `cargo fmt --all -- --check`.
4. Run `cargo clippy --workspace --all-targets --locked -- -D warnings`.
5. Run `cargo test --workspace --locked`.
6. Larger features should add or update a task file under `docs/tasks/`.

## License

MIT - see [LICENSE](LICENSE).
