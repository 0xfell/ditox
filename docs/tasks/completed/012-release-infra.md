# Task: Release Infrastructure (CI + Prebuilt Binaries + NixOS Install)

> **Status:** completed
> **Priority:** high
> **Created:** 2026-04-25
> **Completed:** 2026-04-25

## Description

Make Ditox installable without cloning the repo or running a Rust
toolchain. Two outcomes from one pipeline:

1. **NixOS users** can `nix run github:0xfell/ditox` or pin the flake in
   their home-manager config — no local compilation thanks to the
   `cachix.org/ditox` binary cache.
2. **Non-Nix Linux / Windows users** can grab prebuilt artifacts from
   GitHub Releases: TUI tarballs (glibc + musl static), GUI AppImages
   (x86_64 + aarch64), and a Windows zip with both `.exe`s.

## Deliverables

### CI (`.github/workflows/ci.yml`)

Runs on every push/PR:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo build --workspace --locked`
- `cargo test --workspace --locked`
- Same build + test on `windows-latest`
- `nix flake check` + `nix build .#default`
- On master only: push Nix closure to `cachix.org/ditox`

### Release (`.github/workflows/release.yml`)

Runs on `v*.*.*` tag push. Six parallel build jobs + one publish job:

| Job | Runner | Output |
|---|---|---|
| linux-gnu-tui | ubuntu-24.04 | `ditox-$V-x86_64-linux.tar.gz` |
| linux-musl-tui | ubuntu-24.04 | `ditox-$V-x86_64-linux-musl.tar.gz` (static) |
| linux-gnu-gui-appimage | ubuntu-24.04 | `ditox-gui-$V-x86_64-linux.AppImage` |
| linux-arm64 | ubuntu-24.04 + cross | `ditox-$V-aarch64-linux.tar.gz` + `ditox-gui-$V-aarch64-linux.AppImage` |
| windows | windows-latest | `ditox-$V-x86_64-windows.zip` |
| nix-cache | ubuntu-24.04 | push closure to cachix |
| publish | ubuntu-24.04 | aggregates all + `SHA256SUMS`, creates GitHub Release with auto-generated notes |

macOS is intentionally out of scope (clipboard backend would be a
separate effort). Apple-silicon Linux users are covered by the aarch64
artifacts.

### Packaging files (new)

- `packaging/linux/ditox-gui.desktop` — freedesktop entry for AppImage.
- `packaging/linux/ditox-gui.png` — 256×256 icon for AppImage and
  freedesktop icon cache.

### Docs

- `README.md` rewritten with three Nix install paths, curl one-liners
  for tarball/AppImage, Windows zip instructions, full keybinds + CLI
  command reference, and a Cachix substituter snippet.
- `docs/RELEASING.md` — step-by-step checklist for cutting a release,
  Cachix setup, dry-run mode, troubleshooting.
- `CLAUDE.md` — added "Release Process" section with the short-form
  checklist for future contributors.
- `docs/ROADMAP.md` — current version bumped to 0.3.0.

## Clippy cleanup

Set `-D warnings` in CI as requested. This required:

- 21 auto-fixable warnings resolved via `cargo clippy --fix` (manual
  clamp, redundant closures, `match_result_ok`, `manual_div_ceil`,
  `unused_mut`, etc.).
- 6 structural/harmless warnings gated with `#[allow(...)]` + a
  justifying comment:
  - `EntryType::from_str` — named for symmetry with `as_str`; not a
    `FromStr` impl.
  - Three `too_many_arguments` on internal render helpers
    (`format_entry_row`, `render_text_preview`, `run_loop`).
  - One `field_reassign_with_default` on `iced::window::Settings` where
    the `#[cfg(windows)]` decorations toggle makes a struct literal
    awkward.

Result: `cargo clippy --workspace --all-targets --locked -- -D warnings`
exits clean.

## Files changed

```
Cargo.toml                                 version 0.2.1 → 0.3.0, repo URL fix
Cargo.lock                                 regenerated for 0.3.0
nix/package.nix                            version + homepage + longDescription + maintainer
flake.nix                                  packages.ditox-gui alias, formatter, checks
ditox-core/src/entry.rs                    #[allow] on from_str
ditox-core/tests/search_benchmark.rs       fmt + clippy autofix (no logic change)
ditox-gui/installer/setup.iss              MyAppVersion 0.2.0 → 0.3.0, URL fix
ditox-gui/src/app.rs                       clippy autofixes + scoped field_reassign #[allow]
ditox-tui/src/keybindings.rs               unneeded late init
ditox-tui/src/ui/list.rs                   #[allow(too_many_arguments)]
ditox-tui/src/ui/mod.rs                    #[allow(too_many_arguments)]
ditox-tui/src/ui/note_editor.rs            manual_clamp → clamp
ditox-tui/src/ui/preview.rs                clippy autofixes + #[allow(too_many_arguments)]
tests/common/mod.rs                        repo URL in sample data
CLAUDE.md                                  Release Process section
README.md                                  full rewrite
docs/ROADMAP.md                            v0.3.0 / task 012
docs/RELEASING.md                          NEW — release checklist
docs/tasks/completed/012-release-infra.md  NEW — this file
packaging/linux/ditox-gui.desktop          NEW
packaging/linux/ditox-gui.png              NEW (copy of ditox.png)
.github/workflows/ci.yml                   NEW
.github/workflows/release.yml              NEW
```

## Testing

Local verification (all green before commit):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked           # 33 tests, all pass
nix build .#default                       # builds both binaries
```

Live verification of the release pipeline will happen when the user
pushes the first `v0.3.0` tag. The `workflow_dispatch` trigger allows a
dry run that builds artifacts without cutting a release.

## Outstanding items for the user

1. **Create the Cachix cache** at <https://app.cachix.org/cache/new>
   named `ditox`. Public (OSS tier, free, 5 GB).
2. **Generate auth token** with write scope → add as
   `CACHIX_AUTH_TOKEN` repo secret.
3. **Copy the public key** from the Cachix dashboard and replace the
   `REPLACE_ME=` placeholder in `README.md` (Binary cache section).
4. **Verify Actions permissions** — Settings → Actions → General →
   Workflow permissions: "Read and write" (needed for release upload).
5. **Rotate the Cachix auth token** you pasted in chat earlier —
   assume it's compromised.
6. **Push master + tag** when ready:
   ```sh
   git push origin master          # triggers CI
   git tag -a v0.3.0 -m v0.3.0
   git push origin v0.3.0          # triggers release pipeline
   ```

## Work Log

### 2026-04-25
- Designed three-track plan: NixOS consumption, prebuilt binaries,
  release automation.
- Fixed metadata: bumped versions to 0.3.0, corrected `oxfell` → `0xfell`
  URL across Cargo.toml / nix/package.nix / ditox-gui/installer/setup.iss
  / tests/common/mod.rs.
- Added `packages.ditox-gui` alias, `formatter`, and `checks.build` to
  flake.nix.
- Wrote README.md from scratch covering all install paths.
- Cleaned up 42 clippy warnings so `-D warnings` CI flag is viable.
- Wrote `ci.yml` (4 jobs: linux test, windows test, nix build, cachix
  push on master).
- Wrote `release.yml` (7 jobs: 5 build targets + nix cache + publish).
- Wrote `RELEASING.md` checklist with Cachix setup, dry-run guidance,
  troubleshooting.
- `cargo fmt/clippy/test/nix build` all green.
