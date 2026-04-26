# Ditox systemd user units

## `ditox-watcher.service`

User-mode systemd unit that runs `ditox watch` continuously while a
graphical session is active.

### Install

```sh
mkdir -p ~/.config/systemd/user
cp ditox-watcher.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ditox-watcher.service
```

### Status

```sh
systemctl --user status ditox-watcher.service
journalctl --user -u ditox-watcher.service -f
```

### Stop / disable

```sh
systemctl --user stop ditox-watcher.service
systemctl --user disable ditox-watcher.service
```

### Notes

- The unit assumes `ditox` is in `~/.local/bin/`. If installed
  elsewhere (e.g. via `cargo install`), edit `ExecStart=` accordingly.
- `Restart=on-failure` — the daemon will restart if it crashes, but
  not if you stop it manually.
- `--journal` makes ditox route logs through the journald layer (full
  integration arrives in a future release; it currently falls back to
  stderr which systemd still captures).
- Hardening directives are conservative; if you see clipboard backend
  errors related to permissions, try removing `ProtectHome=read-only`.
