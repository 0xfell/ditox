# Releasing Ditox

Ditox ships as a TUI/CLI binary.

Distribution channels:

1. **GitHub Releases** - prebuilt Linux and Windows archives cut by
   `.github/workflows/release.yml` on `v*.*.*` tag pushes.
2. **Nix flake** - `nix run github:0xfell/ditox`, with closures pushed to
   `cachix.org/ditox`.

## One-Time Setup

### Cachix

1. Log in at <https://app.cachix.org> with GitHub.
2. Create a public cache named `ditox`.
3. Generate a write token.
4. Add it to the GitHub repo as `CACHIX_AUTH_TOKEN`.
5. Keep the public cache key in `README.md` in sync with Cachix.

### GitHub

- Actions -> General -> Workflow permissions: **Read and write permissions**.
- The release workflow needs this to create releases and upload assets.

## Cutting A Release

```sh
export V=0.3.1

# Update hardcoded versions.
sed -i "s/^version = \".*\"$/version = \"$V\"/" Cargo.toml
sed -i "s/^  version = \".*\";/  version = \"$V\";/" nix/package.nix

# Refresh the lockfile.
cargo build --workspace

# Verify.
scripts/check-no-root-tests.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
nix build .#default

# Commit and tag.
git add Cargo.toml Cargo.lock nix/package.nix
git commit -m "chore(release): v$V"
git tag -a "v$V" -m "v$V"
git push origin master "v$V"
```

The tag push triggers `.github/workflows/release.yml`.

## Release Artifacts

Expected files:

- `ditox-$V-x86_64-linux.tar.gz`
- `ditox-$V-x86_64-linux-musl.tar.gz`
- `ditox-$V-aarch64-linux.tar.gz`
- `ditox-$V-x86_64-windows.zip`
- `SHA256SUMS`

Smoke test one artifact:

```sh
curl -L https://github.com/0xfell/ditox/releases/download/v$V/ditox-$V-x86_64-linux-musl.tar.gz \
  | tar -xz --strip-components=1
./ditox --version
```

Smoke test Nix:

```sh
nix run --option extra-substituters https://ditox.cachix.org \
        github:0xfell/ditox/v$V -- --version
```

## Manual Dry Run

1. Go to Actions -> Release -> Run workflow.
2. Pick `master`.
3. Download the workflow artifacts. No GitHub Release is created unless the run
   came from a version tag.

## Troubleshooting

**`Cargo.lock needs to be updated but --locked was passed`**

Run `cargo build --workspace`, commit `Cargo.lock`, and retry.

**Cachix push fails with 401**

Rotate or recreate `CACHIX_AUTH_TOKEN` and update the repository secret.

**Nix build fails with a Cargo lock mismatch**

Regenerate `Cargo.lock`, commit it, and rerun `nix build .#default`.

## Schema Migrations

If a release bumps the schema version, migration code lives in
`ditox-core/src/db.rs::init_schema` and must be:

1. Idempotent.
2. Crash-safe on next open.
3. Covered by a unit test under `ditox-core/tests/`.

See `docs/notes/image-storage.md` for the image-store migration protocol.
