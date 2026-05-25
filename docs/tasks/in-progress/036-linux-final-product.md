# Task: Linux Final Product

> **Status:** in-progress
> **Priority:** high
> **Created:** 2026-05-25
> **Completed:** YYYY-MM-DD

## Description

Turn Ditox into a final Linux product, following the root `final.md` plan.
The release target is Linux: Hyprland and Sway are first-class, generic
wlroots/KDE are supported where protocols allow, and GNOME is degraded mode.
Windows and macOS remain future-port work and are not blockers for this final
Linux release.

## Requirements

- [x] Ensure all Rust tests are owned by workspace packages and run under
  `cargo test --workspace --locked`.
- [x] Complete multi-format watcher capture through `Database::insert_multi`.
- [x] Wire filter `transform:<id>` actions into the capture pipeline.
- [x] Replace TUI no-op actions with real actions or remove unreachable actions.
- [x] Finish Linux foreground subscription paths and paste-back validation hooks.
- [x] Validate GUI daemon, IPC, and layer-shell behavior on live Hyprland.
- [ ] Validate Sway layer-shell behavior and GNOME degraded-mode paths in live
  Sway/GNOME sessions.
- [ ] Expand TUI, GUI, and packaging tests to final-product coverage.
- [x] Keep Linux support claims accurate in README and docs.
- [x] Pass the final release gate:
  `cargo fmt --all -- --check`,
  `scripts/check-no-root-tests.sh`,
  `nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `nix develop --command cargo test --workspace --locked`,
  and `nix build --no-link .#default`.

## Implementation Notes

- Root-level integration tests were orphaned by the virtual workspace and did
  not run under `cargo test --workspace`.
- The first implementation slice migrates those tests into package-owned
  `tests/` directories and adds a guard to prevent regression.

## Testing

- Run the package-specific migrated test targets after moving them.
- Run the root-test guard.
- Run `scripts/smoke-gui-hyprland.sh` inside a live Hyprland session after
  building `target/debug/ditox-gui`.
- Run `scripts/smoke-gui-sway.sh` inside a live Sway session after building
  `target/debug/ditox-gui`.
- Run `scripts/smoke-gui-gnome-degraded.sh` inside a live GNOME Wayland session
  after building `target/debug/ditox-gui`.
- Run the full workspace gates before marking the task complete.

## Work Log

### 2026-05-25

- Created root `final.md` with the Linux final-product implementation and
  verification plan.
- Moved orphaned root integration tests into package-owned test directories.
- Added `scripts/check-no-root-tests.sh` and wired it into CI/release docs.
- Implemented watcher multi-format persistence with canonical entry selection
  plus non-canonical `entry_formats` extras.
- Implemented filter `transform:<id>` action handling for captured text.
- Replaced TUI `ShowActions` and `ShowStats` no-op handlers with live status
  summaries.
- Replaced Hyprland foreground `subscribe()` stub with socket2-backed focus
  event subscription and replaced Wayland capture `subscribe()` stub with a
  polling-backed subscription.
- Fixed GUI tray/window toggle behavior so hidden daemon windows can be shown
  again through the shared toggle path.
- Fixed Hyprland layer-shell hide/show behavior by moving hidden surfaces
  off-screen at 1x1 and restoring configured geometry on show/toggle.
- Added and verified `scripts/smoke-gui-hyprland.sh` for live Hyprland
  show/hide/toggle/quit IPC and layer geometry.
- Exposed `ditox-gui --status` through the public CLI and added
  `scripts/smoke-gui-sway.sh` plus `scripts/smoke-gui-gnome-degraded.sh` for
  live Sway/GNOME validation. This host is Hyprland-only, so those scripts were
  syntax-checked here but not run against Sway/GNOME.
- Verified `ditox-gui --status` appears in CLI help and returns `not-running`
  with exit code `1` when no daemon is active.
- Verified Sway/GNOME smoke scripts fail safely with exit code `2` and clear
  live-session requirement messages on this Hyprland host.
- Verified migrated core tests:
  `nix develop --command cargo test -p ditox-core --test db_tests --test entry_tests --test clipboard_tests --test pagination_benchmark_tests --locked`.
- Verified migrated CLI tests:
  `nix develop --command cargo test -p ditox-tui --test cli_tests --locked`.
- Verified core package:
  `nix develop --command cargo test -p ditox-core --locked`.
- Verified GUI package:
  `nix develop --command cargo test -p ditox-gui --locked`.
- Verified GUI binary build:
  `nix develop --command cargo build -p ditox-gui --locked`.
- Verified Hyprland smoke:
  `scripts/smoke-gui-hyprland.sh`.
- Verified workspace tests:
  `nix develop --command cargo test --workspace --locked`.
- Verified strict clippy:
  `nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Verified Nix package build:
  `nix build --no-link .#default`.
- Fixed GUI icon rendering on Hyprland layer-shell:
  corrected stale Bootstrap icon codepoints, explicitly loaded the bundled
  Bootstrap font at startup, and added a layer-shell-only system-symbol fallback
  because the custom icon font renders as missing-glyph boxes in the
  `iced_layershell` path on this host.
- Visually verified via Hyprland screenshots that the layer-shell header/search
  action icons render as symbols instead of square placeholders. Also verified
  the forced xdg-toplevel path still renders Bootstrap Icons.
- Re-verified focused GUI gates after the icon fix:
  `cargo fmt --all -- --check`,
  `nix develop --command cargo test -p ditox-gui --locked`,
  `nix develop --command cargo clippy -p ditox-gui --all-targets --locked -- -D warnings`,
  and `scripts/smoke-gui-hyprland.sh`.
