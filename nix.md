# NixOS Build & Packaging PRD for Ditox

## Goals
- Reproducible builds and a dev shell via Flakes.
- Package `ditox-cli` for NixOS; future `clipd` daemon later.
- Keep multiple maintained versions side‑by‑side.

## Strategy
- Use Nix Flakes to pin inputs in `flake.lock` and expose `packages`, `apps`, and `devShells` for `x86_64-linux`. Flakes standardize pinning and workflows (`nix build`, `nix run`, `nix flake check`). citeturn1search0
- Prefer `crane` to build the Cargo workspace: incremental, sandboxed, automatic vendoring; add `clippy`/`fmt` hooks. Keep `Cargo.lock` in VCS. citeturn0search2
- Provide a fallback using `rustPlatform.buildRustPackage` (needs `cargoHash`/`cargoLock`) for downstream packagers without `crane`. citeturn4search3
- Toolchains: default to `nixpkgs` Rust; optionally expose inputs for `oxalica/rust-overlay` (pin specific stable/beta/nightly) or `nix-community/fenix` (monthly/nightly channels). citeturn3search0turn3search1

## Flake Layout (planned)
- `packages.<system>.ditox` → CLI derivation; `apps.<system>.ditox` → runnable app.
- `devShells.<system>.default` → Rust, clippy, rustfmt, pkg-config, and X11/Wayland headers.
- Versioned outputs: `packages.<system>.ditox_0_1` for 0.1.x while `packages.<system>.ditox` tracks latest. Multiple versions can co‑exist; overlays/flake outputs select which one a system consumes. citeturn2search0

## CI and Caching
- Add a Nix job that uses `cachix/install-nix-action` to enable flakes, then `nix build` and `nix flake check`. Optionally push to a Cachix cache (`cachix/cachix-action`) for faster PR builds. citeturn0search0turn0search1turn0search3

## Versioning & Backports
- Pin `nixpkgs` in `flake.lock`; update with `nix flake update` (audit changes in PRs). Flakes make input versions explicit and queryable. citeturn1search0
- Keep old Ditox lines as separate outputs (e.g., `ditox_0_1`, `ditox_0_2`) pointing at tags. Consumers choose via flake ref or overlay.
- For older NixOS releases, add additional `nixpkgs` inputs (e.g., `nixpkgs-24_05`) and build variant packages against each; use overlays to patch differences if required. citeturn2search0

## Risks & Notes
- Ensure deterministic builds: avoid non‑reproducible steps and keep timestamps out of artifacts; `crane` handles vendoring and common pitfalls. citeturn0search2
- Document `nix develop` for contributors; enable flakes where needed (`nix.settings.experimental-features = ["nix-command" "flakes"]`). citeturn1search0

