#!/usr/bin/env bash
set -euo pipefail

# Publishes crates to crates.io in order: core -> clipd -> cli
# Requires env var CARGO_REGISTRY_TOKEN to be set in the environment.

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "CARGO_REGISTRY_TOKEN is not set" >&2
  exit 1
fi

publish_crate() {
  local package_name="$1"
  local display_name="$2"

  echo "Publishing ${display_name}..."

  set +e
  local publish_output
  publish_output=$(cargo publish -p "${package_name}" 2>&1)
  local publish_status=$?
  set -e

  printf '%s\n' "${publish_output}"

  if [[ ${publish_status} -eq 0 ]]; then
    echo "${display_name} submitted. crates.io index may take a few minutes to update."
    return 0
  fi

  if grep -q "has already been uploaded" <<<"${publish_output}"; then
    echo "${display_name} version already published; skipping."
    return 0
  fi

  echo "Publishing ${display_name} failed with status ${publish_status}." >&2
  return ${publish_status}
}

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/ditox-core/Cargo.toml | head -n 1)
if [[ -z "${VERSION}" ]]; then
  echo "Failed to determine ditox-core version from Cargo.toml" >&2
  exit 4
fi

publish_crate "ditox-core" "ditox-core"

echo
echo "Check and publish ditox-clipd..."
if rg -n "ditox-core\s*=\s*\{\s*path\s*=\s*\"\..*\"" crates/ditox-clipd/Cargo.toml >/dev/null; then
  echo "ditox-clipd depends on ditox-core via path. Update it to 'version = \"${VERSION}\"' before publishing." >&2
  exit 2
fi
publish_crate "ditox-clipd" "ditox-clipd"

echo
echo "Check and publish ditox-cli..."
if rg -n "ditox-core\s*=\s*\{\s*path\s*=\s*\"\..*\"" crates/ditox-cli/Cargo.toml >/dev/null; then
  echo "ditox-cli depends on ditox-core via path. Update it to 'version = \"${VERSION}\"' before publishing." >&2
  exit 3
fi
publish_crate "ditox-cli" "ditox-cli"

echo "Done."
