#!/usr/bin/env sh
set -eu

if find tests -maxdepth 1 -type f -name '*.rs' 2>/dev/null | grep -q .; then
    echo "root-level Rust tests are not run by this virtual workspace" >&2
    echo "move tests/*.rs into a package-owned tests/ directory" >&2
    find tests -maxdepth 1 -type f -name '*.rs' >&2
    exit 1
fi
