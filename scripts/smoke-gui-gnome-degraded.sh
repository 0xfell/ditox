#!/usr/bin/env bash
set -euo pipefail

if [[ "${XDG_SESSION_TYPE:-}" != "wayland" || "${XDG_CURRENT_DESKTOP:-}" != *GNOME* ]]; then
  echo "smoke-gui-gnome-degraded: requires a live GNOME Wayland session" >&2
  exit 2
fi

bin="${DITOX_GUI_BIN:-target/debug/ditox-gui}"
if [[ ! -x "$bin" ]]; then
  echo "smoke-gui-gnome-degraded: $bin is not executable; run cargo build -p ditox-gui first" >&2
  exit 2
fi

existing="$(pgrep -x ditox-gui || true)"
if [[ -n "$existing" ]]; then
  echo "smoke-gui-gnome-degraded: ditox-gui is already running: $existing" >&2
  exit 2
fi

tmp="$(mktemp -d /tmp/ditox-gui-gnome-smoke.XXXXXX)"
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
      echo "smoke-gui-gnome-degraded: daemon exited early" >&2
      cat "$tmp/ditox-gui.log" >&2 || true
      exit 1
    fi

    status="$("$bin" --status 2>/dev/null || true)"
    if [[ "$status" == *"visible=$expected"* ]]; then
      return 0
    fi
    sleep 0.1
  done

  echo "smoke-gui-gnome-degraded: timed out waiting for visible=$expected" >&2
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
    echo "smoke-gui-gnome-degraded: ok"
    exit 0
  fi
  sleep 0.1
done

echo "smoke-gui-gnome-degraded: daemon did not exit after --quit" >&2
cat "$tmp/ditox-gui.log" >&2 || true
exit 1
