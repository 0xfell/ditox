# Task: Phase 8 — macOS port

> **Status:** planned
> **Priority:** low
> **Phase:** 8 — macOS
> **Created:** 2026-04-26
> **Estimated:** 3-4 weeks

## Description

Add macOS as a third first-class platform. By Phase 8 the
`CaptureSource`, `ForegroundTracker`, paste-synthesizer chain, and
hotkey backend traits are stable and battle-tested — implementing
macOS becomes "fill in known interfaces."

Decisions baked in:
- **macOS is last** (D7) — abstractions stable before third platform.

No schema changes.

## Sub-tasks

### 8.1 macOS clipboard backend

`MacosCapture` implements `CaptureSource`. Backed by `arboard` (which
already supports macOS). No event-based clipboard API on macOS —
polling via `NSPasteboard.changeCount` is the standard approach.

Implementation:
- Wrapper over `arboard::Clipboard`.
- Reads `text/plain`, `image/png`, `image/tiff`, `public.file-url`,
  `public.html`, `public.rtf`.
- Maps to MIME-style format names per Phase 1 convention.

### 8.2 Foreground tracker

`MacosForegroundTracker` uses Objective-C bindings via `objc2`:

- `NSWorkspace.shared.frontmostApplication` for active app PID +
  bundle ID.
- `AXUIElementCreateApplication(pid)` + `AXUIElementCopyAttributeValue`
  for window title (requires Accessibility permission).

`subscribe()` polls `NSWorkspaceDidActivateApplicationNotification`.

### 8.3 Paste synthesis

macOS requires Accessibility permission to send keystrokes.

`MacosSynthesizer`:
- Uses `CGEvent` with `kCGEventKeyDown`/`kCGEventKeyUp`.
- Sends Cmd+V (macOS uses Cmd, not Ctrl, for paste).
- Per-app override still works; default is `cmd+v` instead of `ctrl+v`
  on macOS.

Permission flow:
- First time synthesis attempted → check `AXIsProcessTrusted()`.
- If false: show dialog "Ditox needs Accessibility permission. Open
  System Settings to grant."
- Open System Settings → Privacy & Security → Accessibility via
  `open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"`.

### 8.4 Tray icon

`tray-icon` 0.22 already supports macOS via `NSStatusItem`. Verify it
works on Sequoia and earlier.

Menu items same as other platforms; "Show / Hide" item respects the
menubar conventions.

### 8.5 Global hotkey

`global-hotkey` 0.7 supports macOS via Carbon
`RegisterEventHotKey`. Default: `Cmd+Shift+V`.

### 8.6 Auto-launch

`auto-launch` 0.6 supports macOS via LaunchAgents. Writes
`~/Library/LaunchAgents/com.ditox.gui.plist`.

### 8.7 Window management

iced runs on macOS (wgpu/Metal backend). Custom title bar via macOS-
specific iced settings (or accept system title bar; both fine for v1).

Layer-shell concept doesn't exist on macOS; the launcher is a
floating `NSPanel`-style window. iced 0.14 may need a wrapper to
configure as a panel; investigate.

### 8.8 Distribution

**`.app` bundle:**

```
Ditox.app/
├── Contents/
│   ├── Info.plist
│   ├── MacOS/
│   │   ├── ditox            (TUI/CLI binary)
│   │   └── ditox-gui        (GUI binary)
│   └── Resources/
│       ├── ditox.icns
│       └── ...
```

Built via `cargo-bundle` or hand-rolled `Makefile.macos`.

**DMG:** via `create-dmg` (npm-distributed but standalone).

**Code signing:** Apple Developer ID ($99/yr Apple Developer Program).
Notarisation via `notarytool`. CI integration via GitHub Actions
`mcrouter/notarize-action` or similar.

**Homebrew Cask:**

```ruby
cask "ditox" do
  version "1.0.0"
  sha256 "..."
  url "https://github.com/0xfell/ditox/releases/download/v#{version}/Ditox-#{version}.dmg"
  name "Ditox"
  homepage "https://github.com/0xfell/ditox"
  app "Ditox.app"
end
```

Submit to `homebrew/homebrew-cask`.

### 8.9 CI matrix

`.github/workflows/ci.yml` and `release.yml` extended:

- `macos-13` runner for x86_64.
- `macos-14` runner for aarch64.
- Run `cargo test`, `cargo clippy`, build `.app` bundle.
- Notarisation step on tagged release only (requires secret).

## Acceptance criteria

- [ ] `Ditox.app` runs on macOS Ventura (13.x), Sonoma (14.x), Sequoia
      (15.x).
- [ ] Capture works on x86_64 and aarch64 Macs.
- [ ] Cmd+Shift+V summons the launcher; Cmd+V paste-back works.
- [ ] Tray icon visible in menubar.
- [ ] Run-at-login enables/disables via LaunchAgent plist.
- [ ] `brew install --cask ditox` installs and runs.
- [ ] Notarised binaries pass Gatekeeper without "unidentified developer"
      warnings.

## Implementation Notes

The `iced` macOS path uses Metal via wgpu by default; this works
out of the box. Some iced features (e.g. transparency) are platform-
dependent — verify the launcher visual matches Linux/Windows.

The `objc2` bindings are mature; use them rather than the older `objc`
crate.

For the `NSPanel` launcher style: iced may not expose the
`NSWindow.styleMask` configuration we want. If not, drop down to
direct `objc2` calls in a `cfg(target_os = "macos")` block in
`ditox-gui/src/macos.rs`.

## Risks

- **Risk:** Accessibility permission flow is annoying for users.
  Mitigation: clear in-app instructions; remember the prompt was shown;
  paste-back can be disabled and the user pastes manually.
- **Risk:** Apple rejects notarisation due to a missing entitlement.
  Mitigation: minimal entitlements; standard Hardened Runtime; test
  notarisation early on the v0.11 → v1.0-rc1 build.
- **Risk:** macOS Sequoia changes clipboard behaviour again (it
  introduced new privacy prompts in 15.x). Mitigation: test on the
  latest macOS each release.

## Work Log

### 2026-04-26
- Task file created (epic).
