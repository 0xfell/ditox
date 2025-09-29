#!/usr/bin/env bash
set -euo pipefail

CFG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/ditox"
SETTINGS="$CFG_DIR/settings.toml"
SYS_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$SYS_DIR"

# Determine cadence from settings.sync.interval (default 5m)
EVERY="5m"
if [[ -f "$SETTINGS" ]]; then
  VAL=$(awk -F '[=" ]+' '/^\s*\[sync\]/{flag=1;next} /^\[/{flag=0} flag && /^\s*interval\s*=/{print $3; exit}' "$SETTINGS" || true)
  if [[ -n "${VAL:-}" ]]; then EVERY="$VAL"; fi
fi

# Validate format <int><unit>
if [[ ! "$EVERY" =~ ^[0-9]+[smhdw]$ ]]; then
  echo "[warn] Invalid sync.interval '$EVERY' in settings; defaulting to 5m" >&2
  EVERY="5m"
fi

install -m 0644 "$(dirname "$0")/../contrib/systemd/ditox-sync.service" "$SYS_DIR/ditox-sync.service"
TPL="$(dirname "$0")/../contrib/systemd/ditox-sync.timer.template"
sed "s/@EVERY@/$EVERY/g" "$TPL" > "$SYS_DIR/ditox-sync.timer"

systemctl --user daemon-reload
systemctl --user enable --now ditox-sync.timer
echo "Installed user sync timer with cadence: $EVERY"
systemctl --user status ditox-sync.timer --no-pager || true
