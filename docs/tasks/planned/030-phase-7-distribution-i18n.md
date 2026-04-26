# Task: Phase 7 — Distribution polish, i18n, observability

> **Status:** planned
> **Priority:** medium
> **Phase:** 7 — Distribution & i18n
> **Created:** 2026-04-26
> **Estimated:** 3 weeks

## Description

Final polish before macOS. Get ditox into every package manager users
expect, ship in 5 languages, signed binaries, crash reporting, GUI
stats page.

No schema changes.

Decisions baked in:
- **Zero telemetry** without explicit user action (D8).
- **Crash reporting via `human-panic`** writes local file; user shares
  manually.

## Sub-tasks

### 7.1 Portable Windows build

Build profile that detects `portable.marker` next to the EXE and writes
config + DB to `./data/` instead of `%APPDATA%`.

Mirrors Ditto's marker-file pattern. Useful for USB sticks.

Build via `cargo build --release --features portable` and ship as
`ditox-portable-{version}-windows-x64.zip`.

### 7.2 Code signing

- **Apply to SignPath Foundation** OSS signing (free for verified open
  source).
- Wire into `release.yml`: sign `ditox.exe`, `ditox-gui.exe`,
  `ditox-setup-{ver}.exe`.
- Document the signing policy in `docs/notes/signing.md`.

### 7.3 Package manager submissions

**Chocolatey** (`packaging/chocolatey/`):
- `ditox.nuspec` — manifest with version, description, license URL.
- `tools/chocolateyinstall.ps1` — downloads installer, runs silent.
- `tools/chocolateyuninstall.ps1` — removes via uninstaller.
- Submit to Chocolatey Community Repository.

**Winget** (`packaging/winget/`):
- `manifests/d/Ditox/Ditox/{version}.installer.yaml`,
  `.locale.en-US.yaml`, `.yaml`.
- PR to `microsoft/winget-pkgs`.

**Scoop** (`packaging/scoop/ditox.json`):
- Self-hosted bucket initially. Submit to `scoop-extras` after 2-3
  versions of stability.

**AUR** (`packaging/aur/ditox-bin/PKGBUILD`):
- Binary package downloading from GitHub Releases.
- Self-host on AUR; publish maintainer key.
- Optional `ditox-git` source package later.

**Flatpak** (`packaging/flatpak/com.ditox.Ditox.json`):
- Build manifest for FlatHub.
- Sandbox permissions: `--socket=wayland`, `--socket=fallback-x11`,
  `--filesystem=xdg-data/ditox:create`,
  `--filesystem=xdg-config/ditox:create`,
  `--filesystem=xdg-run/ditox-gui-${UID}.sock`.
- Submit to FlatHub.

### 7.4 Microsoft Store (MSIX)

`packaging/msix/AppxManifest.xml`. Build via `msix-packaging` GitHub
Action. Identity certificate via Partner Center; submission flow.

May ship later than v1.0 due to certification time.

### 7.5 i18n via fluent-rs

Wrap user-facing strings in `t!()` macro:

```rust
#[macro_export]
macro_rules! t {
    ($key:expr, $fallback:expr) => {{
        $crate::i18n::translate($key, $fallback)
    }};
    ($key:expr, $fallback:expr, $($arg:tt)*) => {{
        $crate::i18n::translate_args($key, $fallback, fluent_args![$($arg)*])
    }};
}
```

Locale files at `locales/<lang>/<crate>.ftl`. Fluent format.

Initial 5 locales:
- `en-US` (source)
- `es-ES` (Spanish)
- `fr-FR` (French)
- `de-DE` (German)
- `ja-JP` (Japanese)

Locale detection via `sys-locale` crate; user override via
`[ui] language = "en-US"`.

Document community-contribution process in `docs/notes/i18n.md`.

### 7.6 Crash dumps & error reporting

**`human-panic` integration:**
- On panic: write a report to `<temp>/ditox-crash-{ts}.toml` with
  version, OS, message, backtrace.
- User-friendly stderr message: "Ditox crashed. Report at <path>.
  Please share this file when filing a bug."
- **No automatic upload.**

**Windows:** installer registers WER LocalDumps for `ditox.exe` and
`ditox-gui.exe`:

```
HKLM\Software\Microsoft\Windows\Windows Error Reporting\LocalDumps\ditox.exe
   DumpFolder = %APPDATA%\ditox\Dumps
   DumpType = 2 (full dump)
   DumpCount = 3
```

User-friendly: dumps expire after 3 keeps.

### 7.7 Stats GUI page

Settings → Stats:

- Total entries.
- Total bytes (DB + image store).
- Captures since install (`Total/CopyCount` from `db_meta` table).
- Captures this session.
- Most-pasted entries (top 10).
- Top source apps (requires Phase 2 source_app data).
- Daily capture chart (last 30 days, `iced_charts` or sparkline).

Privacy note in the page: "All stats are local-only. Nothing leaves
this machine."

### 7.8 QR code export

`qrcode` crate. Right-click a text entry → "Show QR". Modal renders
QR. "Copy QR as image" button writes a PNG to clipboard.

Useful for one-way mobile transfer. Pairs with Phase 6 sync down the
line.

### 7.9 New-entry insertion animation

Subtle iced opacity tween (0.0 → 1.0 over 200 ms) when a new entry
appears in the list. Replaces Ditto's tray-shrink-rect animation —
subtler, less distracting.

Toggleable via `[ui] animations = true`.

## Acceptance criteria

- [ ] Portable Windows zip extracts and runs without registry changes.
- [ ] Signed `ditox-setup.exe` shows "Verified publisher: SignPath
      Foundation" in UAC.
- [ ] `choco install ditox` installs from Chocolatey Community.
- [ ] `winget install Ditox.Ditox` installs from winget.
- [ ] `paru -S ditox-bin` installs from AUR.
- [ ] `flatpak install flathub com.ditox.Ditox` installs from FlatHub.
- [ ] All 5 locales render correctly; fallback to English for missing
      keys.
- [ ] Triggered panic produces a `ditox-crash-*.toml` and the message
      to the user.
- [ ] Stats page accurate (verified by manual entry inspection).

## Implementation Notes

`fluent-rs` API:

```rust
let mut bundle = FluentBundle::new(vec![locale]);
bundle.add_resource(resource).unwrap();
let msg = bundle.get_message("hello-world").unwrap();
let pattern = msg.value().unwrap();
let value = bundle.format_pattern(pattern, None, &mut errors);
```

Wrap in a small helper that loads all `.ftl` from `locales/<lang>/`
and provides a `translate(key, fallback)` function.

For Chocolatey/Winget/AUR: automate version-bump scripts so a release
just runs `scripts/release.sh v1.2.3` and the manifests update.

## Work Log

### 2026-04-26
- Task file created (epic).
