# Ditox Keyboard Shortcuts

A complete reference of all keyboard shortcuts available in ditox TUI.

## Navigation

| Key | Alternative | Action |
|-----|-------------|--------|
| `j` | `Down` | Move down |
| `k` | `Up` | Move up |
| `g` | `Home` | Go to top |
| `G` | `End` | Go to bottom |
| `Ctrl+U` | `PageUp` | Page up (half screen) |
| `Ctrl+D` | `PageDown` | Page down (half screen) |
| `h` | `Left` | Previous page |
| `l` | `Right` | Next page |

## Actions

| Key | Action |
|-----|--------|
| `Enter` | Copy selected entry and quit |
| `y` | Copy selected entry to clipboard |
| `d` | Delete selected entry |
| `D` | Clear all entries (with confirmation) |
| `s` | Toggle favorite status |
| `r` | Refresh entries from database |
| `n` | Edit note/annotation for entry |

## Search

| Key | Action |
|-----|--------|
| `/` | Start search (fuzzy mode) |
| `Ctrl+R` | Start search (regex mode) |
| `Ctrl+T` | Toggle between fuzzy/regex search |
| `Esc` | Exit search / Clear query |

## Multi-Select

| Key | Action |
|-----|--------|
| `m` | Toggle multi-select mode |
| `Space` | Select/deselect current entry |
| `v` | Select all / Deselect all |
| `d` | Delete selected entries (in multi-select) |
| `y` | Copy selected entries (in multi-select) |

## View & Display

| Key | Action |
|-----|--------|
| `Tab` | Toggle preview pane |
| `t` | Toggle expanded (fullscreen) preview |
| `p` | Cycle preview mode (Wrap/Scroll/Truncate/Hex/Raw) |
| `L` | Toggle line numbers in preview |
| `?` | Toggle help overlay |

## Tabs

| Key | Action |
|-----|--------|
| `[` | Previous tab |
| `]` | Next tab |

Available tabs: All, Text, Images, Favorites, Today

## Quick Snippets

| Key | Action |
|-----|--------|
| `1-9` | Copy snippet from slot 1-9 |

Quick snippets are favorite entries assigned to number keys for fast access.

## System

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Esc` | Quit (when not in search mode) |
| `Ctrl+C` | Force quit |

## Mouse Support

| Action | Effect |
|--------|--------|
| Click | Select entry |
| Double-click | Copy entry |
| Scroll wheel | Navigate up/down |

## Preview Modes

| Mode | Description |
|------|-------------|
| **Wrap** | Text wraps at pane width (default) |
| **Scroll** | Horizontal scroll for long lines (use `h`/`l`) |
| **Truncate** | First N lines with "...X more lines" indicator |
| **Hex** | Hexdump view showing bytes and ASCII |
| **Raw** | Shows escape sequences and control characters |

## Customization

Keybindings can be customized in `~/.config/ditox/config.toml`:

```toml
[keybindings]
move_up = "k"
move_down = "j"
copy = "y"
delete = "d"
# ... etc
```

Supported modifiers: `ctrl+`, `alt+`, `shift+`

Special keys: `enter`, `esc`, `tab`, `space`, `backspace`, `delete`, `home`, `end`, `pageup`, `pagedown`, `up`, `down`, `left`, `right`, `f1`-`f12`
