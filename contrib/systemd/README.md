# Ditox systemd user timer (prune)

This example sets up a per-user systemd timer to run `ditox prune` periodically.
The install script reads `~/.config/ditox/settings.toml` and uses `prune.every` as cadence.

Quick install
- Run: `scripts/install_prune_timer.sh`
- It creates `~/.config/systemd/user/ditox-prune.service` and `.timer`, then enables the timer.
- Adjust behavior via `settings.toml` (`[prune] keep_favorites, max_items, max_age`).

Manual files (reference)
- `contrib/systemd/ditox-prune.service`
- `contrib/systemd/ditox-prune.timer.template` (with `@EVERY@` placeholder)

Inspect
- `systemctl --user status ditox-prune.timer`
- `journalctl --user -u ditox-prune.service --since today`

Note: To run while logged out, you may need lingering: `loginctl enable-linger "$USER"`.
