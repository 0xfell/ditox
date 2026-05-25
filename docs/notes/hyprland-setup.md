# Ditox on Hyprland

> Status: Linux final-product setup guide. Hyprland is a first-class target.

Hyprland is a first-class target. This document explains how to set
ditox up and what to expect.

---

## TL;DR

The recommended setup is:

```bash
ditox-gui --install-hyprland-config
```

This writes `~/.config/hypr/conf.d/ditox.conf` with everything you need
(autostart, hotkey, window rules, per-clip hotkey source). Add one line
to your main `hyprland.conf`:

```
source = ~/.config/hypr/conf.d/ditox.conf
```

Reload Hyprland (`hyprctl reload`) and press `Ctrl+~` to open the
launcher.

---

## What works on Hyprland

| Feature | Status |
|---|---|
| Capture (text, images, HTML, RTF, files) | First-class (Phase 1) |
| Floating launcher via `wlr-layer-shell` | First-class |
| Foreground tracking (paste-back to previous app) | First-class via `wlr-foreign-toplevel-management` + `hyprctl activewindow` |
| Paste-back synthesis | First-class via `hyprctl dispatch sendshortcut` |
| Tray icon | Works with `waybar` (StatusNotifierItem) and `hyprpanel` |
| Position at cursor | Works via `hyprctl cursorpos` |
| Position at active-window centre | Works via `hyprctl monitors` + `activeworkspace` |
| Position at caret (text input position) | **Not available** — Wayland has no protocol for this |
| Global hotkey (in-app) | **Not available** — Wayland security model forbids it; use compositor binding instead |
| Per-clip global hotkeys | First-class via generated Hyprland binds |
| Run at login | First-class via `exec-once` in `~/.config/hypr/conf.d/ditox.conf` |

---

## Manual setup (no helper)

If you don't want ditox to manage a config file for you, copy the
following into `~/.config/hypr/hyprland.conf` (or any sourced file):

```hyprlang
# ditox autostart (long-running daemon, hidden)
exec-once = ditox-gui --hide

# Show / hide the launcher
bind = CTRL, grave, exec, ditox-gui --toggle

# Optional: dedicated bind to capture current selection (force-save)
# bind = CTRL_SHIFT, c, exec, ditox save

# When the launcher window opens it should float, pin, and not animate.
# The layer-shell path makes most of these unnecessary, but they're harmless
# if your compositor falls back to xdg_toplevel:
windowrulev2 = float, class:^(ditox-gui)$
windowrulev2 = pin, class:^(ditox-gui)$
windowrulev2 = noborder, class:^(ditox-gui)$
windowrulev2 = noshadow, class:^(ditox-gui)$
windowrulev2 = noanim, class:^(ditox-gui)$
```

If you want a specific size and position rather than ditox's
auto-positioning:

```hyprlang
windowrulev2 = size 420 520, class:^(ditox-gui)$
windowrulev2 = move 20 100%-540, class:^(ditox-gui)$
```

---

## Tray icon

Hyprland doesn't ship a status bar by default. ditox publishes the tray
icon via `StatusNotifierItem` (libappindicator), which the following
bars consume:

- **`waybar`** — works out of the box. Add a `tray` module:

  ```jsonc
  // ~/.config/waybar/config
  {
      "modules-right": ["tray", ...],
      "tray": { "spacing": 8 }
  }
  ```

- **`hyprpanel`** — works out of the box.

If you don't run a bar, ditox still works — just rely on the global
keybind to summon it.

---

## Paste-back behaviour

When you click an entry in the launcher, ditox needs to:

1. Hide / dismiss the launcher.
2. Switch focus back to the previously-active window.
3. Write the clip to the OS clipboard.
4. Synthesize `Ctrl+V` into the now-focused window.

Step 4 is the tricky one on Wayland. ditox tries the following in order:

1. **`hyprctl dispatch sendshortcut , ctrl+v, address:<addr>`** — preferred
   on Hyprland. No daemons, no extra setup.
2. **`wtype`** — works on most wlroots compositors. Install with
   `pacman -S wtype` / `apt install wtype` / equivalent.
3. **`ydotool`** — requires `ydotoold` running and your user in the
   `input` group (or another mechanism for `/dev/uinput` access).
4. **Manual paste** — ditox writes the clip and tells you "click your
   target window and press Ctrl+V."

Force a specific synthesizer via config:

```toml
# ~/.config/ditox/config.toml
[paste]
synthesize = "hyprctl"   # or "wtype" | "ydotool" | "off" | "auto" (default)
```

---

## Position modes

Configure where the launcher appears via:

```toml
[gui]
position = "at_cursor"   # one of:
# "at_cursor"            — at the mouse cursor (Hyprland: hyprctl cursorpos)
# "at_active_window_center" — centred on the previously-focused window
# "at_previous"          — wherever it last closed
# "fixed"                — anchored to a specific monitor edge (default; bottom-left)
# "at_caret"             — Windows only; falls back to "at_active_window_center" on Wayland
```

---

## Per-clip hotkeys

You can bind a clip to a global hotkey from the launcher's side panel. Behind
the scenes ditox writes:

```hyprlang
# ~/.config/hypr/conf.d/ditox-binds.conf  (managed by ditox; do not edit)
# >>> ditox-managed binds — auto-generated, do not edit between markers <<<
bind = CTRL_ALT, 1, exec, ditox-gui paste-clip <uuid-1>
bind = CTRL_ALT, 2, exec, ditox-gui paste-clip <uuid-2>
# <<< end ditox-managed binds >>>
```

You add `source = ~/.config/hypr/conf.d/ditox-binds.conf` once to your
main config. ditox keeps the file in sync as you add/remove per-clip
hotkeys; the markers prevent it from touching anything else.

---

## Troubleshooting

### Launcher tiles instead of floating

You're on a build older than Phase 4 (no layer-shell). Add the window
rules listed in *Manual setup* above, or run
`ditox-gui --install-hyprland-config`.

### Paste does nothing after clicking an entry

Check synthesizer detection:

```
ditox-gui --paste-debug
```

Usually one of:

- `hyprctl` returned non-zero (Hyprland version too old? — needs >= 0.39).
- No address could be resolved for the previous window (the previous
  window was ditox itself).
- `wtype`/`ydotool` not installed and `synthesize = "auto"` could not
  find a fallback.

Workaround: set `[paste] synthesize = "off"` and paste manually with
Ctrl+V.

### Tray icon doesn't appear

Hyprland alone doesn't render trays. Install `waybar` or `hyprpanel`
and add the tray module. Alternative: use the keyboard summon and
ignore the tray.

### Foreground tracker reports the wrong app

ditox tracks the most recent non-ditox toplevel. If you have a quirky
setup (dock, popup, transient window) the tracker can occasionally
target the wrong app. Adjust:

```toml
[foreground]
ignore_classes = ["wofi", "fuzzel", "rofi", "dunst", "your-bar"]
```

---

## Security note

Wayland deliberately prevents one app from reading another app's
clipboard or injecting input. ditox uses three privileged channels:

1. The standard `wl-clipboard` data-control protocol (read clipboard
   content). Allowed by the compositor's policy.
2. `wlr-foreign-toplevel-management` (read window list). Requires the
   compositor to support it. Hyprland and Sway do; GNOME does not.
3. Keystroke synthesis via `hyprctl sendshortcut` / `wtype` / `ydotool`.
   This is the only sensitive operation; it runs as you, not as root.

ditox never sends your clipboard over the network unless you explicitly
enable LAN sync (Phase 6) and pin a peer's public key.
