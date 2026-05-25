#!/usr/bin/env bash
set -euo pipefail

if [[ "${XDG_CURRENT_DESKTOP:-}" != *sway* && -z "${SWAYSOCK:-}" ]]; then
  echo "smoke-gui-sway: requires a live Sway session" >&2
  exit 2
fi

if ! command -v swaymsg >/dev/null 2>&1; then
  echo "smoke-gui-sway: swaymsg is required" >&2
  exit 2
fi

bin="${DITOX_GUI_BIN:-target/debug/ditox-gui}"
if [[ ! -x "$bin" ]]; then
  echo "smoke-gui-sway: $bin is not executable; run cargo build -p ditox-gui first" >&2
  exit 2
fi

if ! swaymsg -t get_version >/dev/null 2>&1; then
  echo "smoke-gui-sway: swaymsg cannot reach the running compositor" >&2
  exit 2
fi

existing="$(pgrep -x ditox-gui || true)"
if [[ -n "$existing" ]]; then
  echo "smoke-gui-sway: ditox-gui is already running: $existing" >&2
  exit 2
fi

tmp="$(mktemp -d /tmp/ditox-gui-sway-smoke.XXXXXX)"
pid=""

cleanup() {
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    "$bin" --quit >/dev/null 2>&1 || true
    sleep 0.2
    kill "$pid" >/dev/null 2>&1 || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT

mkdir -p "$tmp/data" "$tmp/config"

RUST_LOG="${RUST_LOG:-ditox_gui=info,ditox_core=warn}" \
  XDG_DATA_HOME="$tmp/data" \
  XDG_CONFIG_HOME="$tmp/config" \
  "$bin" --hide >"$tmp/ditox-gui.log" 2>&1 &
pid="$!"

wait_for_status() {
  local expected="$1"
  local deadline=$((SECONDS + 8))
  local status

  while (( SECONDS < deadline )); do
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "smoke-gui-sway: daemon exited early" >&2
      cat "$tmp/ditox-gui.log" >&2 || true
      exit 1
    fi

    status="$("$bin" --status 2>/dev/null || true)"
    if [[ "$status" == *"visible=$expected"* ]]; then
      return 0
    fi
    sleep 0.1
  done

  echo "smoke-gui-sway: timed out waiting for visible=$expected" >&2
  echo "last status: ${status:-none}" >&2
  cat "$tmp/ditox-gui.log" >&2 || true
  exit 1
}

wait_for_status false
"$bin" --show >/dev/null 2>&1
wait_for_status true
"$bin" --hide >/dev/null 2>&1
wait_for_status false
"$bin" --toggle >/dev/null 2>&1
wait_for_status true
"$bin" --toggle >/dev/null 2>&1
wait_for_status false
"$bin" --quit >/dev/null 2>&1

for _ in {1..40}; do
  if ! kill -0 "$pid" 2>/dev/null; then
    pid=""
    echo "smoke-gui-sway: ok"
    exit 0
  fi
  sleep 0.1
done

echo "smoke-gui-sway: daemon did not exit after --quit" >&2
cat "$tmp/ditox-gui.log" >&2 || true
exit 1
