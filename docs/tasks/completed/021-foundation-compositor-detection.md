# Task: Compositor / OS detection module

> **Status:** completed
> **Priority:** high
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

Phases 2, 4, 5, and 6 all branch on the runtime environment:

- Hyprland gets `hyprctl` shortcuts, layer-shell, generated config.
- Sway gets `swaymsg` foreground queries.
- KDE Wayland gets KGlobalAccel for hotkeys (Phase 5).
- GNOME Wayland skips features that need wlr-* protocols.
- Windows gets registry hotkeys, MAPI, etc.
- macOS (Phase 8) gets NSPasteboard, LaunchAgents.

Centralise detection in one place to avoid `if let Ok(_) = env::var(...)`
sprinkled throughout.

## Requirements

- [ ] **`Platform` enum** in `ditox-core/src/platform.rs`:
      ```rust
      pub enum Platform {
          Linux(LinuxCompositor),
          Windows(WindowsVersion),
          Macos(MacosVersion),
          Other,
      }

      pub enum LinuxCompositor {
          Hyprland { version: Option<Version> },
          Sway     { version: Option<Version> },
          Kde      { version: Option<Version>, wayland: bool },
          Gnome    { version: Option<Version>, wayland: bool },
          Wlroots  { name: String },          // generic wlr-* compositor
          X11Only  { name: Option<String> },  // pre-Wayland desktop
          Unknown,
      }
      ```
- [ ] **Detection at startup**, cached in a `OnceLock<Platform>`:
      - **Hyprland:** `$HYPRLAND_INSTANCE_SIGNATURE` set, OR
        `$XDG_CURRENT_DESKTOP=Hyprland`, OR `hyprctl version` succeeds.
      - **Sway:** `$SWAYSOCK` set OR
        `$XDG_CURRENT_DESKTOP=sway` OR `swaymsg -t get_version` succeeds.
      - **KDE:** `$XDG_CURRENT_DESKTOP=KDE` AND
        `$WAYLAND_DISPLAY` set → `Kde { wayland: true }`.
      - **GNOME:** `$XDG_CURRENT_DESKTOP=GNOME`.
      - **Windows:** compile-time `cfg(windows)` plus runtime
        `RtlGetVersion` for major.minor.build.
      - **macOS:** compile-time `cfg(target_os = "macos")` plus runtime
        Gestalt or `system_profiler`.
- [ ] **Capability flags** derived from Platform:
      ```rust
      impl Platform {
          pub fn supports_layer_shell(&self) -> bool;
          pub fn supports_wlr_foreign_toplevel(&self) -> bool;
          pub fn supports_global_hotkey_in_app(&self) -> bool;
          pub fn supports_hyprctl(&self) -> bool;
          pub fn paste_synthesizer_chain(&self) -> Vec<&'static str>;
      }
      ```
- [ ] **`ditox status` exposes the detection** in JSON output:
      ```
      {
        "platform": "Hyprland",
        "version": "0.42.0",
        "capabilities": {
          "layer_shell": true,
          "wlr_foreign_toplevel": true,
          "global_hotkey_in_app": false,
          "hyprctl": true
        }
      }
      ```
- [ ] **Unit tests** with environment-variable mocking via
      `temp_env::with_vars`.

## Implementation Notes

Detection should be cheap and **never panic**. Each probe wrapped in
`Result`; on any error, fall back to `Unknown` and log a warning.

The `paste_synthesizer_chain` returns the ordered list of synthesizers
to try. For Hyprland: `["hyprctl", "wtype", "ydotool"]`. For Sway:
`["wtype", "ydotool"]`. For GNOME Wayland: `["ydotool"]` (with a
warning that the user may need to set up uinput).

`supports_global_hotkey_in_app` is `true` only on Windows and macOS.
On every Linux compositor it's false; the user binds via the
compositor.

## Testing

- Unit tests for each compositor by mocking env vars.
- Manual: run `ditox status --json` on Hyprland, Sway (Arch live ISO?),
  KDE Wayland (live ISO), GNOME Wayland (live ISO), Windows 11.
  Capture screenshots in `docs/notes/platform-detection-matrix.md`.

## Work Log

### 2026-04-26
- Task file created.
- Created `ditox-core/src/platform.rs` with `Platform`, `LinuxCompositor`, `WindowsVersion`, `MacosVersion` types.
- `detect()` returns a cached `&'static Platform` via `OnceLock`. Detection is cheap and never panics.
- Linux detection probes (in order): `HYPRLAND_INSTANCE_SIGNATURE`, `XDG_CURRENT_DESKTOP=Hyprland`, `SWAYSOCK`, `XDG_CURRENT_DESKTOP=sway`, `XDG_CURRENT_DESKTOP=KDE` (+ Wayland), `XDG_CURRENT_DESKTOP=GNOME`, generic wlroots fallback for unrecognised wayland, X11Only for `DISPLAY`-only sessions, Unknown otherwise.
- Capability flags: `supports_layer_shell()`, `supports_wlr_foreign_toplevel()`, `supports_global_hotkey_in_app()`, `supports_hyprctl()`, and `paste_synthesizer_chain()` returning the ordered try-list.
- Windows / macOS version probes are stubs returning `(0,0,0)` — to be filled in by Phase 1 / 8 when version-specific behaviour matters.
- 8 unit tests covering Hyprland (signature + XDG paths), Sway, KDE Wayland, GNOME Wayland (degraded), X11-only, Unknown, and the cached-detect smoke test. Each mocks env vars via a sequenced `with_env` helper to avoid interference.
- Wired `ditox status` to print platform slug + capability flags + paste chain.
- All 52 workspace tests pass. Build green.
