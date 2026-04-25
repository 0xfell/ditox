# Ditox

A cross-platform clipboard manager for Linux (Wayland) and Windows.

- **TUI** (`ditox`) — keyboard-driven terminal UI + full CLI.
- **GUI** (`ditox-gui`) — iced + system tray, with Ctrl+Shift+V global hotkey on Windows and a `--toggle` IPC flag for compositor keybinds on Linux.
- **Daemon** — lightweight clipboard watcher with SHA-256 dedup.

## Features

- Text and image capture, including browser "Copy image" with URL/image ambiguity resolved.
- Content-addressed image store (atomic writes, refcount-based pruning) — no orphan files.
- Full-text search (FTS5), fuzzy and regex modes.
- Named collections, pinned favorites, quick-snippet slots (1–9).
- Pagination-aware lists; 10k+ entries load in milliseconds.
- Wayland clipboard integration via `wl-clipboard`; Windows via arboard.
- System tray (libappindicator on Linux, Win32 on Windows).
- Home Manager module + systemd user service.
- `ditox repair` for image-store reconciliation.

## Install

### Linux — NixOS / Nix

**Ad-hoc run:**
```sh
nix run github:0xfell/ditox              # launch the TUI
nix run github:0xfell/ditox#ditox-gui    # launch the GUI
```

**Install into a profile:**
```sh
nix profile install github:0xfell/ditox
```

**Declarative (flake + Home Manager):**
```nix
{
  inputs.ditox.url = "github:0xfell/ditox";      # track master
  # inputs.ditox.url = "github:0xfell/ditox/v0.3.0";   # pin to a release

  outputs = { self, nixpkgs, ditox, home-manager, ... }: {
    homeConfigurations.you = home-manager.lib.homeManagerConfiguration {
      # ...
      modules = [
        ditox.homeManagerModules.default
        {
          programs.ditox = {
            enable = true;
            systemd.enable = true;        # auto-start the watcher
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

**Binary cache:** release builds are pushed to
[cachix.org/ditox](https://app.cachix.org/cache/ditox). To skip local
compilation:
```sh
# One-off:
nix run --option extra-substituters https://ditox.cachix.org \
        --option extra-trusted-public-keys ditox.cachix.org-1:kVMmDqje/wWu/ChZfMzWdqduZlBptE7LoHZ3lTFdxg8= \
        github:0xfell/ditox

# Or, once, as root on NixOS:
cachix use ditox
```

### Linux — prebuilt binaries (non-Nix)

All artifacts live at
<https://github.com/0xfell/ditox/releases/latest>.

**TUI only — static, no deps:**
```sh
curl -L https://github.com/0xfell/ditox/releases/latest/download/ditox-x86_64-linux-musl.tar.gz | tar xz
sudo install ditox /usr/local/bin/
```

**GUI — AppImage (x86_64 and aarch64):**
```sh
curl -LO https://github.com/0xfell/ditox/releases/latest/download/ditox-gui-x86_64-linux.AppImage
chmod +x ditox-gui-x86_64-linux.AppImage
./ditox-gui-x86_64-linux.AppImage
```

You still need `wl-clipboard` on the TUI side for clipboard operations:
```sh
sudo apt install wl-clipboard         # Debian/Ubuntu
sudo pacman -S wl-clipboard           # Arch
sudo dnf install wl-clipboard         # Fedora
```

### Windows

Download `ditox-x86_64-windows.zip` from the
[releases page](https://github.com/0xfell/ditox/releases/latest), unzip,
and run `ditox-gui.exe`. Ctrl+Shift+V summons the window. The tray icon
offers a "Run at login" toggle that writes to the `HKCU…\Run` registry key.

### Build from source

```sh
git clone https://github.com/0xfell/ditox
cd ditox
cargo build --release --workspace
# binaries: target/release/ditox  target/release/ditox-gui
```

On Linux the GUI needs GTK 3, libayatana-appindicator, Vulkan loader,
fontconfig, and friends — see `flake.nix devShells.default` for the
exact dependency list. Easiest path: `nix develop`.

## Usage

### TUI

```sh
ditox                 # browse history
ditox watch           # start the clipboard watcher
```

Key bindings (TUI):

| Key | Action |
|---|---|
| `j`/`k`, `↑`/`↓` | Move selection |
| `g` / `G` | Top / bottom |
| `Enter` | Copy and quit |
| `y` | Copy, stay open |
| `Tab` | Toggle preview pane |
| `/` | Fuzzy search (Ctrl+R toggles regex) |
| `f` | Toggle favorite |
| `d` | Delete (with confirmation) |
| `D` | Clear all (with confirmation) |
| `n` | Edit note |
| `1`…`9` | Switch tab (All/Text/Images/Favorites/Today/…) |
| `v` | Multi-select mode |
| `?` | Help overlay |
| `q` | Quit |

### CLI

```sh
ditox list [--limit N] [--json] [--favorites]
ditox get <n|id> [--json]          # print raw content
ditox search <query> [--limit N] [--json]
ditox copy <n|id>                  # push entry onto the clipboard
ditox delete <n|id>
ditox favorite <n|id>
ditox clear [--confirm]
ditox count
ditox status
ditox stats [--json]
ditox repair [--dry-run] [--fix-hashes]
ditox collection list|create|delete|rename|add|remove|show
```

Entry targets are either 1-based indices (from `list`) or UUIDs.

`ditox repair` reconciles the image store with the database — removes
orphan files, prunes dangling rows, and (with `--fix-hashes`) quarantines
any file whose SHA-256 disagrees with the row it's supposed to back.
See [`docs/notes/image-storage.md`](docs/notes/image-storage.md) for the
full storage protocol.

### GUI

- **Linux:** run `ditox-gui`. The tray icon offers show/hide/quit/"Run at
  login". Summon from a compositor keybind with `ditox-gui --toggle`:
  ```conf
  # Hyprland
  bind = SUPER, V, exec, ditox-gui --toggle
  ```
  Single-instance coordination uses `flock` + a Unix socket under
  `$XDG_RUNTIME_DIR`; a second launch just toggles the running instance.
- **Windows:** Ctrl+Shift+V is registered globally. Tray icon → Quit.

## Configuration

`~/.config/ditox/config.toml` (Linux) or `%APPDATA%/ditox/config.toml`
(Windows):

```toml
[general]
max_entries = 500
poll_interval_ms = 250

[ui]
show_preview = true
date_format = "relative"
# graphics_protocol = "kitty"     # override auto-detection: kitty | sixel | iterm2 | halfblocks

[ui.theme]
selected = "#7aa2f7"
border   = "#565f89"
text     = "#c0caf5"
```

The Home Manager module (`programs.ditox.settings`) renders this file
declaratively — see the install example above.

## Data locations

| | Linux | Windows |
|---|---|---|
| Database | `~/.local/share/ditox/ditox.db` | `%APPDATA%\ditox\ditox.db` |
| Images | `~/.local/share/ditox/images/` | `%APPDATA%\ditox\images\` |
| Config | `~/.config/ditox/config.toml` | `%APPDATA%\ditox\config.toml` |

## Project docs

- [`docs/ROADMAP.md`](docs/ROADMAP.md) — status, version, what's next.
- [`docs/notes/`](docs/notes/) — architecture notes (image storage, Linux GUI).
- [`docs/tasks/`](docs/tasks/) — per-feature work logs.
- [`CLAUDE.md`](CLAUDE.md) — contributor guide for AI agents working on this repo.
- [`docs/RELEASING.md`](docs/RELEASING.md) — release process.

## Contributing

1. `nix develop` (or install Rust + system deps manually).
2. `cargo build --workspace && cargo test --workspace` — all 33 tests must pass.
3. Follow Conventional Commits (`fix:`, `feat:`, `chore:`, etc.).
4. Larger features: add a task file under `docs/tasks/in-progress/` and
   update `docs/ROADMAP.md` when done.

## License

MIT — see [LICENSE](LICENSE) (or this file header for now).
