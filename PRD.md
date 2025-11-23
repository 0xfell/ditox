# Product Requirement Document: Ditox v1.0

## 1. Introduction

**Ditox** is a lightweight, terminal-based clipboard manager built for NixOS and Wayland. It captures your clipboard history in the background and provides a fast TUI for retrieval.

**Primary targets:** NixOS, Wayland (Hyprland), Linux
**Architecture:** Background service + on-demand TUI (single binary, two modes)

## 2. Tech Stack

| Component       | Technology                                         |
| --------------- | -------------------------------------------------- |
| **Language**    | Rust (2021 edition)                                |
| **TUI**         | Ratatui + Crossterm                                |
| **Database**    | SQLite (via `rusqlite`, bundled)                   |
| **Clipboard**   | `wl-clipboard-rs` (Wayland) / `arboard` (fallback) |
| **Config**      | TOML via `serde`                                   |
| **Hashing**     | SHA256 via `sha2`                                  |
| **CLI**         | `clap`                                             |

## 3. Why This Architecture?

A clipboard manager **must** capture history even when the UI isn't open. Two approaches:

| Approach | Pros | Cons |
|----------|------|------|
| Always-on TUI | Simple | Must keep terminal open, impractical |
| Daemon + TUI | Captures everything | Slightly more complex |

**Chosen: Daemon + TUI in single binary**

```
ditox watch    # Background daemon (captures clipboard)
ditox          # TUI (reads from shared DB)
```

The daemon is lightweight (~5MB RAM) and designed for systemd user services on NixOS.

## 4. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         ditox watch                              │
│                      (background daemon)                         │
│  ┌─────────────────┐                                            │
│  │ wl-clipboard-rs │──► Poll clipboard ──► Dedupe ──► SQLite    │
│  └─────────────────┘                                   │        │
└────────────────────────────────────────────────────────┼────────┘
                                                         │
┌────────────────────────────────────────────────────────┼────────┐
│                           ditox                        │        │
│                        (TUI client)                    ▼        │
│  ┌─────────────┐    ┌──────────┐    ┌─────────────────────────┐│
│  │   Ratatui   │◄───│   App    │◄───│        SQLite           ││
│  │     TUI     │    │  State   │    │   (shared database)     ││
│  └─────────────┘    └──────────┘    └─────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

**Key insight:** Both modes share the same SQLite database. No IPC needed.

## 5. Usage

```bash
# Start background watcher (run once, or via systemd)
ditox watch

# Open TUI to browse/search/paste
ditox

# CLI utilities
ditox list              # Print recent entries
ditox list --json       # JSON output for scripts
ditox copy 1            # Copy entry #1 to clipboard
ditox clear             # Clear all history
ditox status            # Check if watcher is running
```

## 6. NixOS Integration (Primary Goal)

### 6.1 Flake Structure

```
ditox/
├── flake.nix
├── flake.lock
├── nix/
│   ├── package.nix      # Derivation
│   └── module.nix       # Home Manager module
├── Cargo.toml
├── Cargo.lock
└── src/
```

### 6.2 flake.nix

```nix
{
  description = "Ditox - Terminal clipboard manager for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      in {
        packages.default = pkgs.callPackage ./nix/package.nix { };

        apps.default = {
          type = "program";
          program = "${self.packages.${system}.default}/bin/ditox";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
            pkg-config
            openssl
          ];
        };
      }
    ) // {
      homeManagerModules.default = import ./nix/module.nix;
      overlays.default = final: prev: {
        ditox = self.packages.${prev.system}.default;
      };
    };
}
```

### 6.3 Home Manager Module

```nix
# nix/module.nix
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.programs.ditox;
  tomlFormat = pkgs.formats.toml { };
in {
  options.programs.ditox = {
    enable = mkEnableOption "ditox clipboard manager";

    package = mkOption {
      type = types.package;
      default = pkgs.ditox;
      description = "The ditox package to use.";
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          general = {
            max_entries = 1000;
            poll_interval_ms = 300;
          };
          ui.theme = "tokyo-night";
        }
      '';
      description = "Configuration for ditox.";
    };

    systemd.enable = mkOption {
      type = types.bool;
      default = true;
      description = "Enable systemd user service for clipboard watching.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."ditox/config.toml" = mkIf (cfg.settings != { }) {
      source = tomlFormat.generate "ditox-config" cfg.settings;
    };

    systemd.user.services.ditox = mkIf cfg.systemd.enable {
      Unit = {
        Description = "Ditox clipboard watcher";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };

      Service = {
        ExecStart = "${cfg.package}/bin/ditox watch";
        Restart = "on-failure";
        RestartSec = 5;
      };

      Install = {
        WantedBy = [ "graphical-session.target" ];
      };
    };
  };
}
```

### 6.4 User Configuration Example

```nix
# In your home.nix or flake
{ inputs, ... }: {
  imports = [ inputs.ditox.homeManagerModules.default ];

  programs.ditox = {
    enable = true;
    systemd.enable = true;  # Auto-start watcher
    settings = {
      general = {
        max_entries = 1000;
        poll_interval_ms = 300;
      };
      ui = {
        show_preview = true;
        date_format = "relative";
      };
    };
  };

  # Hyprland keybinding
  wayland.windowManager.hyprland.settings = {
    bind = [ "SUPER, V, exec, kitty --class ditox -e ditox" ];
    windowrulev2 = [
      "float, class:^(ditox)$"
      "size 800 600, class:^(ditox)$"
      "center, class:^(ditox)$"
      "animation slide, class:^(ditox)$"
    ];
  };
}
```

## 7. Functional Requirements

### 7.1 Clipboard Watcher (`ditox watch`)

| Feature            | Description                                              |
| ------------------ | -------------------------------------------------------- |
| **Text capture**   | Store copied text                                        |
| **Image capture**  | Save images to disk, store path in DB                    |
| **Deduplication**  | SHA256 hash, skip duplicates                             |
| **Polling**        | Default 500ms, configurable                              |
| **Wayland-native** | Uses `wl-clipboard-rs` for reliable Wayland support      |
| **Lightweight**    | < 10MB RAM when idle                                     |

### 7.2 TUI (`ditox`)

```
┌─ Ditox ─────────────────────────────────────────────────────┐
│ Search: _______________________________________________ [?] │
├─────────────────────────────────┬───────────────────────────┤
│  #  │ Type │ Content       │ Age│                           │
│─────┼──────┼───────────────┼────│  Preview                  │
│ > 1 │ txt  │ Hello world   │ 2m │                           │
│   2 │ txt  │ fn main() {   │ 5m │  Hello world              │
│   3 │ img  │ screenshot    │ 1h │                           │
│   4 │ txt  │ https://...   │ 2h │  Full content of the      │
│   5 │ txt  │ nix develop   │ 3h │  selected entry appears   │
│     │      │               │    │  here with wrapping.      │
├─────────────────────────────────┴───────────────────────────┤
│ j/k:Move  Enter:Copy+Exit  y:Copy  d:Del  /:Search  q:Quit  │
└─────────────────────────────────────────────────────────────┘
```

### 7.3 Keybindings

| Key       | Action                     |
| --------- | -------------------------- |
| `j`/`k`   | Navigate up/down           |
| `g`/`G`   | Go to top/bottom           |
| `Enter`   | Copy to clipboard and exit |
| `y`       | Copy (stay open)           |
| `d`       | Delete entry               |
| `D`       | Clear all                  |
| `/`       | Search                     |
| `Esc`     | Clear search               |
| `Tab`     | Toggle preview             |
| `?`       | Help                       |
| `q`       | Quit                       |

### 7.4 Search

- Fuzzy matching via `nucleo` or `fuzzy-matcher`
- Real-time filtering
- Case-insensitive

## 8. Storage

**Locations (XDG compliant):**
- Database: `~/.local/share/ditox/ditox.db`
- Images: `~/.local/share/ditox/images/`
- Config: `~/.config/ditox/config.toml`

**Schema:**

```sql
CREATE TABLE entries (
    id          TEXT PRIMARY KEY,
    entry_type  TEXT NOT NULL,
    content     TEXT NOT NULL,
    hash        TEXT NOT NULL UNIQUE,
    byte_size   INTEGER NOT NULL,
    created_at  TEXT NOT NULL,
    pinned      INTEGER DEFAULT 0
);

CREATE INDEX idx_created_at ON entries(created_at DESC);
CREATE INDEX idx_pinned ON entries(pinned DESC, created_at DESC);
```

## 9. Configuration

```toml
# ~/.config/ditox/config.toml

[general]
max_entries = 500          # Max history size
poll_interval_ms = 500     # Clipboard poll rate

[storage]
# Uses XDG defaults if not specified
# data_dir = "~/.local/share/ditox"

[ui]
show_preview = true
date_format = "relative"   # "relative" | "iso"

[ui.theme]
selected = "#7aa2f7"
border = "#565f89"
text = "#c0caf5"
muted = "#565f89"
```

## 10. Performance Targets

| Metric            | Target              |
| ----------------- | ------------------- |
| Watcher RAM       | < 10MB              |
| TUI RAM           | < 30MB              |
| Startup           | < 50ms              |
| Binary size       | < 10MB (stripped)   |
| Poll → DB write   | < 20ms              |

## 11. Non-Goals (v1.0)

- Windows/macOS support
- Cloud sync
- Rich text/HTML
- OCR
- GUI
- X11-first (Wayland is primary)

## 12. Dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
wl-clipboard-rs = "0.8"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
sha2 = "0.10"
uuid = { version = "1", features = ["v4"] }
directories = "5"
chrono = "0.4"
nucleo = "0.5"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"

[target.'cfg(not(target_os = "linux"))'.dependencies]
arboard = "3"  # Fallback for non-Wayland
```

## 13. Project Structure

```
ditox/
├── flake.nix
├── flake.lock
├── Cargo.toml
├── Cargo.lock
├── nix/
│   ├── package.nix
│   └── module.nix
├── src/
│   ├── main.rs
│   ├── cli.rs            # Clap definitions
│   ├── app.rs            # TUI app state
│   ├── watcher.rs        # Clipboard daemon
│   ├── db.rs             # SQLite operations
│   ├── entry.rs          # Entry model
│   ├── config.rs         # Config loading
│   ├── clipboard.rs      # Clipboard abstraction
│   └── ui/
│       ├── mod.rs
│       ├── layout.rs
│       ├── list.rs
│       ├── preview.rs
│       ├── search.rs
│       └── theme.rs
├── README.md
└── LICENSE
```

## 14. Roadmap

| Version | Features                                  |
| ------- | ----------------------------------------- |
| 0.1     | Core watcher + TUI + NixOS module         |
| 0.2     | Fuzzy search, theming                     |
| 1.0     | Stable, polished, documented              |
| 1.1     | Pinned entries, categories                |
| 1.2     | Sixel/Kitty image preview                 |
| 2.0     | Cross-platform (arboard backend), sync    |
