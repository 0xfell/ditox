# Task 4 — Distribution and Platform Support

This task focuses on production packaging and platform fit-and-finish, with a priority on Windows binaries and CI artifacts. It builds on Task 3 (clipd, picker, tags, xfer, thumbs).

---

## Goals

- Windows: provide signed (or sign-ready) `.exe` binaries for `ditox.exe` and `clipd.exe`.
- CI: matrix builds for Linux, macOS, Windows; attach release artifacts (zip/tar.gz) per target.
- Picker: integrate daemon pagination for smooth large-history scrolling (done); ensure responsiveness at 50k+ items.
- clipd: test-friendly modes (done: `--exit-after-ms`, `--health-once`).
- Clipboard UX: improved doctor diagnostics on macOS/Windows (done) and platform-specific read/write adapters.

Non-goals in this task: code signing automation, installers (MSI/deb/rpm); these can follow in 4.x.

---

## Windows Binary Plan

- Toolchain: `x86_64-pc-windows-msvc`.
- Strategy options:
  - Native runner on Windows in GitHub Actions with MSVC toolchain.
  - Cross-compile from Linux using `cargo-xwin` (optional fallback).
- Artifacts: `ditox-cli` and `ditox-clipd` renamed on upload to `ditox.exe` and `clipd.exe`.
- Packaging: zip with `README-WINDOWS.txt`, `LICENSE`, and example `settings.toml`.

Notes
- arboard works with Windows clipboard; doctor prints detailed errors if access fails.
- TUI runs in Windows Terminal/ConHost; recommend UTF-8 code page and a font with box drawing characters.

---

## CI (GitHub Actions)

- Matrix: `ubuntu-latest`, `windows-latest`, `macos-latest`.
- Steps:
  - Checkout with submodules.
  - Rust toolchain stable + cache.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`.
  - Build release for each:
    - Linux: `cargo build --release -p ditox-cli -p ditox-clipd`.
    - macOS: same as above.
    - Windows: `cargo build --release -p ditox-cli -p ditox-clipd --target x86_64-pc-windows-msvc`.
  - Upload artifacts:
    - `ditox.exe`, `clipd.exe` (Windows)
    - `ditox`, `clipd` (Linux/macOS)

Future
- Add `cargo-xwin` lane for reproducible cross builds.
- Incorporate code signing (EV optional) through repository secrets.

---

## User Docs (Windows)

- Place config under `%APPDATA%\ditox\settings.toml` (directories crate maps accordingly).
- Start `clipd` at login via Task Scheduler (ship a `.xml` example).
- Known caveat: some clipboard managers lock access; doctor prints a hint when errors occur.

---

## Verification

- Smoke tests on Windows runner:
  - `ditox.exe add "hello"`, `ditox.exe list`, `ditox.exe pick --no-daemon`.
  - `clipd.exe --port 0 --health-once` (returns one health response and exits).
  - Export/import round-trip.

---

## Checklist

- [ ] GitHub Actions workflow for matrix build + artifacts.
- [ ] Windows artifact zips with `.exe` + README + LICENSE.
- [ ] Optional cargo-xwin job.
- [ ] Task Scheduler templates for `clipd`.
- [ ] Release notes including Windows usage tips.

