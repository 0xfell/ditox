# Ditox

A lightweight, terminal-based clipboard manager for Wayland and NixOS.

## Features

- **Wayland-native** - Uses `wl-clipboard-rs` for reliable clipboard monitoring
- **Terminal UI** - Fast, keyboard-driven interface built with Ratatui
- **CLI** - Command-line interface for scripting and automation
- **Image preview** - Inline image display (Kitty, iTerm2, Sixel protocols)
- **NixOS-first** - Includes Home Manager module and systemd integration
- **Fuzzy search** - Quickly find entries with fuzzy matching in TUI
- **JSON output** - Machine-readable output via `ditox list --json`
- **Deduplication** - SHA256 hashing prevents duplicate entries

## Installation

### NixOS (Recommended)

Add to your flake inputs:

```nix
{
  inputs.ditox.url = "github:oxfell/ditox";
}
```

Then in your Home Manager configuration:

```nix
{ inputs, ... }: {
  imports = [ inputs.ditox.homeManagerModules.default ];

  programs.ditox = {
    enable = true;
    systemd.enable = true;  # Auto-start watcher
    settings = {
      general.max_entries = 1000;
      ui.show_preview = true;
    };
  };
}
```

### Cargo

```bash
cargo install ditox
```

### From Source

```bash
git clone https://github.com/oxfell/ditox
cd ditox
cargo build --release
```

## Usage

### Interactive TUI

```bash
# Open the TUI to browse clipboard history
ditox
```

### Background Watcher

```bash
# Start the clipboard watcher (run in background or as systemd service)
ditox watch
```

### CLI Commands

CLI access for scripting and automation:

```bash
# List entries
ditox list                      # List recent entries (default: 10)
ditox list --limit 50           # List more entries
ditox list --json               # JSON output for scripts
ditox list --pinned             # Show only pinned entries

# Copy to clipboard
ditox copy 1                    # Copy entry #1 to clipboard

# Clear history
ditox clear --confirm           # Clear all history

# Info
ditox status                    # Show statistics and paths
```

> **Note:** Some operations (search, delete, pin) are currently TUI-only. Use the interactive TUI for the full feature set.

### Scripting Examples

```bash
# Export history as JSON
ditox list --limit 1000 --json > backup.json

# Copy a specific entry to clipboard
ditox copy 1

# Use jq to extract content from JSON output
ditox list --json | jq -r '.[0].content'
```

## Keybindings (TUI)

| Key       | Action                     |
| --------- | -------------------------- |
| `j`/`k`   | Navigate up/down           |
| `g`/`G`   | Go to top/bottom           |
| `Enter`   | Copy to clipboard and exit |
| `y`       | Copy (stay open)           |
| `d`       | Delete entry               |
| `D`       | Clear all                  |
| `s`       | Toggle pin                 |
| `Tab`     | Toggle preview panel       |
| `/`       | Search                     |
| `?`       | Help                       |
| `q`       | Quit                       |

## Configuration

Configuration file: `~/.config/ditox/config.toml`

```toml
[general]
max_entries = 500
poll_interval_ms = 250  # Fast polling for quick copy-paste

[ui]
show_preview = true
date_format = "relative"
# Graphics protocol for image preview (auto-detected by default)
# Options: kitty, sixel, iterm2, halfblocks
# graphics_protocol = "kitty"  # Uncomment for Ghostty/Kitty/WezTerm

[ui.theme]
selected = "#7aa2f7"
border = "#565f89"
text = "#c0caf5"
```

**Note for Ghostty users:** Image preview should auto-detect, but if it doesn't work, add:
```toml
[ui]
graphics_protocol = "kitty"
```

## Hyprland Integration

Add to `~/.config/hypr/hyprland.conf`:

```conf
bind = SUPER, V, exec, kitty --class ditox -e ditox
windowrulev2 = float, class:^(ditox)$
windowrulev2 = size 800 600, class:^(ditox)$
windowrulev2 = center, class:^(ditox)$
```

## License

MIT
