#!/usr/bin/env bash
set -euo pipefail

# Publishes crates to crates.io in order: core -> clipd -> cli
# Requires env var CARGO_REGISTRY_TOKEN to be set in the environment.

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "CARGO_REGISTRY_TOKEN is not set" >&2
  exit 1
fi

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/ditox-core/Cargo.toml | head -n 1)
if [[ -z "${VERSION}" ]]; then
  echo "Failed to determine ditox-core version from Cargo.toml" >&2
  exit 4
fi

echo "Publishing ditox-core..."
cargo publish -p ditox-core
echo "ditox-core submitted. crates.io index may take a few minutes to update."

echo
echo "Check and publish ditox-clipd..."
if rg -n "ditox-core\s*=\s*\{\s*path\s*=\s*\"\..*\"" crates/ditox-clipd/Cargo.toml >/dev/null; then
  echo "ditox-clipd depends on ditox-core via path. Update it to 'version = \"${VERSION}\"' before publishing." >&2
  exit 2
fi
cargo publish -p ditox-clipd || true

echo
echo "Check and publish ditox-cli..."
if rg -n "ditox-core\s*=\s*\{\s*path\s*=\s*\"\..*\"" crates/ditox-cli/Cargo.toml >/dev/null; then
  echo "ditox-cli depends on ditox-core via path. Update it to 'version = \"${VERSION}\"' before publishing." >&2
  exit 3
fi
cargo publish -p ditox-cli || true

echo "Done."
