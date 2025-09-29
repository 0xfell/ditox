#!/usr/bin/env bash
set -euo pipefail

"$(dirname "$0")/install_prune_timer.sh"
"$(dirname "$0")/install_sync_timer.sh"
echo "Installed prune + sync timers per settings.toml"
