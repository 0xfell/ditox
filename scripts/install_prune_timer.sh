#!/usr/bin/env bash
set -euo pipefail

CFG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/ditox"
SETTINGS="$CFG_DIR/settings.toml"
SYS_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$SYS_DIR"

# Determine cadence from settings.prune.every (default 7d)
EVERY="7d"
if [[ -f "$SETTINGS" ]]; then
  VAL=$(awk -F '[=" ]+' '/^\s*every\s*=/{print $3; exit}' "$SETTINGS" || true)
  if [[ -n "${VAL:-}" ]]; then EVERY="$VAL"; fi
fi

# Validate EVERY as <int><unit> where unit in s/m/h/d/w
if [[ ! "$EVERY" =~ ^[0-9]+[smhdw]$ ]]; then
  echo "[warn] Invalid prune.every '$EVERY' in settings; defaulting to 7d" >&2
  EVERY="7d"
fi

# Write service
install -m 0644 "$(dirname "$0")/../contrib/systemd/ditox-prune.service" "$SYS_DIR/ditox-prune.service"

# Generate timer from template
TPL="$(dirname "$0")/../contrib/systemd/ditox-prune.timer.template"
sed "s/@EVERY@/$EVERY/g" "$TPL" > "$SYS_DIR/ditox-prune.timer"

systemctl --user daemon-reload
systemctl --user enable --now ditox-prune.timer
echo "Installed user timer with cadence: $EVERY"
systemctl --user status ditox-prune.timer --no-pager || true

# Harden settings permissions
if [[ -f "$SETTINGS" ]]; then chmod 600 "$SETTINGS" || true; fi
