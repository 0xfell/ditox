#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   contrib/scripts/turso_user_version.sh <db-name>            # show user_version
#   contrib/scripts/turso_user_version.sh <db-name> set <ver>  # set then show
#
# Requires the Turso CLI (`turso`) logged in and configured.

if ! command -v turso >/dev/null 2>&1; then
  echo "error: turso CLI not found in PATH" >&2
  exit 1
fi

DB_NAME=${1:-}
ACTION=${2:-show}
VER=${3:-}

if [[ -z "${DB_NAME}" ]]; then
  echo "usage: $0 <db-name> [show|set <ver>]" >&2
  exit 2
fi

case "${ACTION}" in
  show)
    turso db shell "${DB_NAME}" --execute "PRAGMA user_version;" ;;
  set)
    if [[ -z "${VER}" ]]; then
      echo "usage: $0 ${DB_NAME} set <ver>" >&2
      exit 2
    fi
    turso db shell "${DB_NAME}" --execute "PRAGMA user_version = ${VER};"
    turso db shell "${DB_NAME}" --execute "PRAGMA user_version;" ;;
  *)
    echo "unknown action: ${ACTION}" >&2
    exit 2 ;;
esac

