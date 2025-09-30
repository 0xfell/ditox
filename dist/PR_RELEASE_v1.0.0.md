Title: chore(release): v1.0.0

Summary
- Bump crate versions to 1.0.0 (`ditox-core`, `ditox-cli`, `ditox-clipd`).
- Update README badge to 1.0.0.
- Add CHANGELOG entry for 1.0.0 (2025-09-30) and fix tag note to use `v*`.
- Bump Nix flake package version to 1.0.0.

Release Rationale
- Stabilizes the core data model and CLI surface.
- Introduces the TUI picker and background daemon crate (`ditox-clipd`).
- Adds import/export for text and image blobs; improves systemd timer installers.

CI & Packaging
- Release workflow `.github/workflows/release.yml` triggers on tags like `v1.0.0`.
- Builds and uploads zips for Linux/macOS/Windows plus Nix tarball.

Checklist
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --all`
- [x] `cargo run -p ditox-cli -- --db /tmp/ditox-release.db migrate --status` (0 pending)

Next Steps (post-merge)
1) Tag: `git tag -a v1.0.0 -m "v1.0.0" && git push origin v1.0.0`.
2) Watch the “Release” workflow to complete asset uploads.
3) Optionally run the “Upload Assets” workflow to re-attach builds if needed.

Notes
- Images remain local-only by design; remote sync (libSQL/Turso) is feature-gated.
