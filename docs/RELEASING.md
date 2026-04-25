# Releasing Ditox

This project ships through three distribution channels:

1. **GitHub Releases** — prebuilt binaries for Linux (x86_64 + aarch64)
   and Windows, cut automatically by `.github/workflows/release.yml` on
   every `v*.*.*` tag push.
2. **Nix flake** — `nix run github:0xfell/ditox` pulls straight from the
   repo; closures are cached at `cachix.org/ditox` so users don't need to
   recompile.
3. **Inno Setup installer** (Windows, optional) — `ditox-gui/installer/`
   contains an `.iss` script for a traditional Windows installer. Built
   manually; not part of the CI pipeline yet.

## One-time setup (already done, documented for posterity)

### Cachix cache

1. Log in at <https://app.cachix.org> with GitHub.
2. Create a public cache named `ditox`.
3. Generate an auth token with "write" scope.
4. In the GitHub repo: Settings → Secrets and variables → Actions →
   New repository secret. Name: `CACHIX_AUTH_TOKEN`, value: the token.
5. Copy the cache's public key from the Cachix dashboard.
6. Replace the `REPLACE_ME=` placeholder in `README.md` (Binary cache
   section) with the real public key.

### GitHub repo settings

- Actions → General → "Workflow permissions": set to **Read and write
  permissions** (needed by the release workflow to create the GitHub
  Release and upload assets).
- Actions → General → "Allow GitHub Actions to create and approve pull
  requests" can stay off; we don't need it.

## Cutting a release

### 1. Prepare

```sh
# 1a. Pick a version following semver: PATCH for bug fixes, MINOR for new
#     features, MAJOR for breaking changes (schema migrations, public API
#     signatures). Current: 0.3.0.
export V=0.3.1

# 1b. Update every hardcoded version.
sed -i "s/^version = \".*\"$/version = \"$V\"/" Cargo.toml
sed -i "s/^  version = \".*\";/  version = \"$V\";/" nix/package.nix
sed -i "s/^  #define MyAppVersion \".*\"/  #define MyAppVersion \"$V\"/" \
    ditox-gui/installer/setup.iss

# 1c. Refresh Cargo.lock so the new version lands in the lockfile.
cargo build --workspace

# 1d. Run the full test suite.
cargo test --workspace --locked

# 1e. Run the pre-commit gauntlet (same as CI).
#     IMPORTANT: clippy lints change between toolchain versions. CI uses
#     `dtolnay/rust-toolchain@stable` which always tracks the latest stable
#     release; our `nix develop` shell pins to whatever rust-bin.stable.latest
#     resolves to (usually 1–2 versions behind). To guarantee parity with
#     CI, run clippy with the host stable explicitly:
cargo fmt --all -- --check
rustup run stable cargo clippy --workspace --all-targets --locked -- -D warnings
nix build .#default
```

If any step fails, fix it and restart from the failing step.

### 2. Commit and tag

```sh
git add Cargo.toml Cargo.lock nix/package.nix ditox-gui/installer/setup.iss
# Add any other version-touching files you updated (CHANGELOG, ROADMAP, …)

git commit -m "chore(release): v$V"
git tag -a "v$V" -m "v$V"
git push origin master "v$V"
```

The tag push triggers `.github/workflows/release.yml`. Track the run at
`https://github.com/0xfell/ditox/actions`.

### 3. Verify the release

Once the workflow completes (~15–25 minutes — the AppImage and cross
builds are slow):

1. Open the release at
   `https://github.com/0xfell/ditox/releases/tag/v$V`. Confirm all six
   artifacts + `SHA256SUMS` are attached:
   - `ditox-$V-x86_64-linux.tar.gz`
   - `ditox-$V-x86_64-linux-musl.tar.gz`
   - `ditox-gui-$V-x86_64-linux.AppImage`
   - `ditox-$V-aarch64-linux.tar.gz`
   - `ditox-gui-$V-aarch64-linux.AppImage`
   - `ditox-$V-x86_64-windows.zip`
   - `SHA256SUMS`

2. Smoke test at least one artifact on a real machine:
   ```sh
   curl -L https://github.com/0xfell/ditox/releases/download/v$V/ditox-$V-x86_64-linux-musl.tar.gz \
     | tar -xz --strip-components=1
   ./ditox --version        # should print: ditox 0.3.1
   ```

3. Smoke test the Nix path from a machine that has never pulled Ditox:
   ```sh
   nix run --option extra-substituters https://ditox.cachix.org \
           github:0xfell/ditox/v$V -- --version
   ```
   First run should download from cache, not compile.

### 4. Announce (optional)

Edit the auto-generated release notes on GitHub if needed. The
`generate_release_notes: true` flag in the workflow produces a
changelog from the commit log between the previous tag and the new one.

## Manual dry-run of the pipeline

To test the workflow without cutting a real release:

1. Go to Actions → Release → "Run workflow" → pick `master`.
2. Artifacts will appear as workflow-run artifacts (retention: 7 days).
   No GitHub Release is created (the `publish` job is gated on
   `is_tag == 'true'`).
3. Download the artifacts tab of the run to inspect the binaries
   locally.

## Troubleshooting

**`Cargo.lock needs to be updated but --locked was passed`**
Run `cargo build --workspace` locally to regenerate the lockfile after a
version bump, then commit the updated `Cargo.lock`.

**AppImage job fails with "no fuse support"**
GitHub Actions runners do support FUSE, but only if you install it
(`apt install fuse libfuse2t64`). The workflow already does this.

**`linuxdeploy-plugin-gtk` fails on the aarch64 job**
Expected — the plugin isn't aarch64-safe. The aarch64 AppImage is
built without it, so users need GTK3 + libayatana-appindicator
installed on their aarch64 system. Document this in the release notes
if not already.

**Cachix push fails with 401**
The `CACHIX_AUTH_TOKEN` secret was rotated or never set. Re-issue the
token from the Cachix dashboard and update the repo secret.

**Nix build fails on first run with "hash mismatch"**
Happens when `Cargo.lock` changed but the checked-in hash in
`nix/package.nix` is stale. We use `cargoLock.lockFile`, so this
normally doesn't happen — but if you switched to a vendored hash, run
`nix build` locally, copy the expected hash from the error, and
commit it.

## Schema migrations

If a release bumps the schema version (e.g. adds a column, changes the
image layout), the migration code lives in
`ditox-core/src/db.rs::init_schema` and must be:

1. **Idempotent** — re-running it on an already-migrated DB is a no-op.
2. **Single-transaction per row** — a crash mid-migration heals on next
   open.
3. **Verified** — a unit test under `ditox-core/tests/` must demonstrate
   the v(N−1) → v(N) upgrade produces the expected DB state.

See `docs/notes/image-storage.md` for the v0 → v1 migration as an
example.
