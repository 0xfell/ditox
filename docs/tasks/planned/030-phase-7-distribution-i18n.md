# Task: Distribution & i18n

## Status

planned

## Summary

Package the TUI/CLI cleanly across supported channels and prepare user-facing
strings for localization.

## Scope

- Linux tarballs for glibc, musl, and aarch64.
- Windows zip for the terminal binary when Windows support is validated.
- Nix flake and Home Manager module.
- Optional distro packages such as AUR, Winget, Chocolatey, and Homebrew.
- Translation infrastructure for terminal messages, help text, and docs.

## Acceptance Criteria

- Release workflow produces only supported terminal artifacts.
- Install docs match actual artifacts.
- User-visible strings can be extracted or audited for localization.

