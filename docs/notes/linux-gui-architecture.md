# Linux GUI architecture

Notes on how `ditox-gui` runs on Linux. For the corresponding Windows
decisions see `ditox-gui/src/app.rs` and `docs/notes/win-d-problem.md`.

## Process model

```
┌────────────────────────────────────────────────────────────┐
│ ditox-gui (iced main thread — winit event loop)            │
│  ├─ iced app, watcher poll, search, DB access              │
│  ├─ ipc_sub subscription — drains ipc_bridge mpsc          │
│  └─ tray_sub subscription — drains tray_icon::MenuEvent    │
├────────────────────────────────────────────────────────────┤
│ ditox-ipc-server (background thread, spawned per client)   │
│  listens on $XDG_RUNTIME_DIR/ditox-gui-$UID.sock           │
│  reads one line, pushes IpcCommand into ipc_bridge mpsc    │
├────────────────────────────────────────────────────────────┤
│ ditox-tray (dedicated GTK thread)                          │
│  gtk::init() → owns TrayIcon → gtk::main()                 │
│  menu events travel via tray_icon's global MenuEvent chan  │
└────────────────────────────────────────────────────────────┘
```

### Why three threads?

- **iced/winit** wants the main thread.
- **tray-icon** on Linux requires a GTK event loop on the same thread as the
  `TrayIcon`. Trying to use `gtk::init()` on the winit thread causes
  duplicated event-loop initialisation (winit owns the wayland/x11 loop).
  A dedicated thread is the only sane option.
- **IPC** could live on the iced thread as a `tokio` task, but we already
  have synchronous `std::os::unix::net` + `flock` code paths and the small
  helper thread pattern is simpler than pulling tokio into the socket
  lifecycle.

## Single-instance lock

File layout (per UID):

```
$XDG_RUNTIME_DIR/ditox-gui-$UID.lock    # flock(LOCK_EX | LOCK_NB)
$XDG_RUNTIME_DIR/ditox-gui-$UID.sock    # Unix domain socket
```

- Second launches get `EWOULDBLOCK`, fall through to the "forward action
  over the socket and exit" path.
- The socket file is removed before bind (covers a crashed previous
  instance — the flock is gone so it's safe).
- The `InstanceLock` guard lives for the whole run; `Drop` removes both
  files. flock is released automatically when the file handle closes.
- Fallback directory for `XDG_RUNTIME_DIR` unset/missing is `/tmp`.

## IPC wire format

Plaintext, newline-delimited. Server reads one line per connection, pushes
an `IpcCommand`, responds `OK\n` or `ERR <msg>\n`, reads the next line,
repeats until EOF.

| Command | Iced message | Effect |
|---------|---|---|
| `TOGGLE` | `ToggleWindow` | Show/hide based on visibility check |
| `SHOW` | `IpcShow` | Always show |
| `HIDE` | `HideWindow` | Hide if shown, no-op otherwise |
| `QUIT` | `QuitApp` | Save window state + `std::process::exit(0)` |

Unknown commands reply `ERR unknown command`.

## Why not D-Bus?

- No user-level D-Bus session on minimal setups (headless, some tiling
  WMs not started via session manager).
- Per-user Unix sockets are trivial, zero-config, and already what bigger
  apps like wofi/rofi/anyrun use.

## Compositor keybind pattern

Recommended user setup:

```
# Hyprland (~/.config/hypr/hyprland.conf)
exec-once = ditox-gui --hide
bind = SUPER, V, exec, ditox-gui --toggle

# Sway (~/.config/sway/config)
exec ditox-gui --hide
bindsym $mod+v exec ditox-gui --toggle

# GNOME / KDE
# Use the DE's keyboard-shortcut UI, command = `ditox-gui --toggle`.
# Autostart is handled by the tray "Run at login" toggle.
```

## XDG autostart details

When the tray "Run at login" checkbox is on:

```
~/.config/autostart/ditox-gui.desktop
```

Contents:

```ini
[Desktop Entry]
Type=Application
Name=Ditox
Comment=Clipboard manager
Exec=/nix/store/.../bin/ditox-gui --hide
Icon=ditox
Terminal=false
Categories=Utility;
X-GNOME-Autostart-enabled=true
```

`--hide` makes the initial iced window come up with `settings.visible =
false`, so the user only sees the tray icon until they trigger the
keybind.

## Known non-issues (intentional)

- The custom dark title bar we draw on Windows also renders on Linux on
  top of the compositor's decorations. Functional but visually
  redundant; follow-up polish.
- The "Show" tray menu entry label on Linux says "Show" (no hotkey
  hint) because there's no global hotkey on Wayland.
- `Message::ForceWindowFocus` runs on Linux but calls into a stub
  `force_restore_window` — completely a no-op, no behaviour difference.
