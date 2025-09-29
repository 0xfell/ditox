#!/usr/bin/env bash
set -euo pipefail

MSG=${1:-"hello-local-$(date +%s)"}

if command -v ditox-cli >/dev/null 2>&1; then
  exec ditox-cli --store sqlite add "$MSG"
else
  echo "[info] 'ditox-cli' not in PATH; using cargo run"
  exec cargo run -p ditox-cli -- --store sqlite add "$MSG"
fi

