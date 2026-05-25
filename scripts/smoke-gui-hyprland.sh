#!/usr/bin/env bash
set -euo pipefail

if [[ "${XDG_CURRENT_DESKTOP:-}" != *Hyprland* ]]; then
  echo "smoke-gui-hyprland: requires a live Hyprland session" >&2
  exit 2
fi

if ! command -v hyprctl >/dev/null 2>&1; then
  echo "smoke-gui-hyprland: hyprctl is required" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "smoke-gui-hyprland: jq is required for hyprctl JSON parsing" >&2
  exit 2
fi

bin="${DITOX_GUI_BIN:-target/debug/ditox-gui}"
if [[ ! -x "$bin" ]]; then
  echo "smoke-gui-hyprland: $bin is not executable; run cargo build -p ditox-gui first" >&2
  exit 2
fi

existing="$(pgrep -x ditox-gui || true)"
if [[ -n "$existing" ]]; then
  echo "smoke-gui-hyprland: ditox-gui is already running: $existing" >&2
  exit 2
fi

tmp="$(mktemp -d /tmp/ditox-gui-hypr-smoke.XXXXXX)"
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

layer_geometry() {
  hyprctl layers -j | jq -er '
    [
      .. | objects | select(.namespace? == "ditox-gui") |
      [.x, .y, .w, .h] | map(floor | tostring) | @tsv
    ][0]
  '
}

wait_for_geometry() {
  local mode="$1"
  local deadline=$((SECONDS + 8))
  local geom x y w h

  while (( SECONDS < deadline )); do
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "smoke-gui-hyprland: daemon exited early" >&2
      cat "$tmp/ditox-gui.log" >&2 || true
      exit 1
    fi

    if geom="$(layer_geometry 2>/dev/null)"; then
      read -r x y w h <<<"$geom"
      case "$mode" in
        hidden)
          if (( ${w%.*} <= 1 && ${h%.*} <= 1 && ${x%.*} < -1000 )); then
            return 0
          fi
          ;;
        shown)
          if (( ${w%.*} == 420 && ${h%.*} == 520 && ${x%.*} >= 0 )); then
            return 0
          fi
          ;;
      esac
    fi
    sleep 0.1
  done

  echo "smoke-gui-hyprland: timed out waiting for $mode geometry" >&2
  echo "last geometry: ${geom:-none}" >&2
  cat "$tmp/ditox-gui.log" >&2 || true
  exit 1
}

wait_for_status() {
  local expected="$1"
  local deadline=$((SECONDS + 8))
  local status

  while (( SECONDS < deadline )); do
    status="$("$bin" --status 2>/dev/null || true)"
    if [[ "$status" == *"visible=$expected"* ]]; then
      return 0
    fi
    sleep 0.1
  done

  echo "smoke-gui-hyprland: timed out waiting for status visible=$expected" >&2
  echo "last status: ${status:-none}" >&2
  cat "$tmp/ditox-gui.log" >&2 || true
  exit 1
}

wait_for_geometry hidden
wait_for_status false
"$bin" --show >/dev/null 2>&1
wait_for_geometry shown
wait_for_status true
"$bin" --hide >/dev/null 2>&1
wait_for_geometry hidden
wait_for_status false
"$bin" --toggle >/dev/null 2>&1
wait_for_geometry shown
wait_for_status true
"$bin" --toggle >/dev/null 2>&1
wait_for_geometry hidden
wait_for_status false
"$bin" --quit >/dev/null 2>&1

for _ in {1..40}; do
  if ! kill -0 "$pid" 2>/dev/null; then
    pid=""
    echo "smoke-gui-hyprland: ok"
    exit 0
  fi
  sleep 0.1
done

echo "smoke-gui-hyprland: daemon did not exit after --quit" >&2
cat "$tmp/ditox-gui.log" >&2 || true
exit 1
