# Task: Linux Final Product

## Status

in-progress

## Summary

Harden the Linux release as a TUI-only clipboard manager. This task verifies the
current Rust TUI, CLI, watcher daemon, Wayland capture, paste-back paths, image
storage, sync, packaging, and documentation.

## Checklist

- [ ] Verify `cargo test --workspace --locked`.
- [ ] Verify `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- [ ] Verify `cargo fmt --all -- --check`.
- [ ] Verify `nix build .#default`.
- [ ] Exercise TUI browse/search/copy/delete/favorite/notes/preview workflows.
- [ ] Exercise `ditox watch --status --json`, stop/start behavior, and systemd unit.
- [ ] Exercise Wayland image and text capture on Hyprland and at least one wlroots compositor.
- [ ] Document degraded GNOME behavior for foreground tracking and paste-back.

## Acceptance Criteria

- Linux users can install and run one terminal binary, `ditox`.
- The watcher daemon can be managed from CLI and systemd.
- TUI workflows are keyboard-first, reliable, and documented.
- Packaging ships no removed desktop assets or dependencies.

