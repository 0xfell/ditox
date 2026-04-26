# ADR 0001: Layer-shell strategy for the Linux GUI

> **Status:** Accepted (pending real-compositor visual verification)
> **Date:** 2026-04-26
> **Phase:** 0 — Foundation (task 022)
> **Deciders:** project lead + claude-opus-4-7
> **Implementing:** Phase 4 (task 026 — Ditto UX replication)

## Context

ditox v0.3.1 ships a Linux GUI built on `iced` 0.14 with iced's
default winit/`xdg_toplevel` window. To match Ditto's launcher
behaviour on Hyprland and Sway we need:

- A small, always-on-top, focused panel summoned by a hotkey.
- Anchored to a screen edge (bottom-left by default).
- Cooperates with tiling compositors *without* the user having to
  add window rules to make it float.
- Receives keyboard input on appearance (Esc to dismiss, arrows +
  Enter to navigate).
- Auto-dismisses on focus loss.

`xdg_toplevel` is the wrong tool: tiling compositors will tile it,
floating compositors lay it out via WM heuristics, and "always on
top" is a per-WM hack. The right protocol is `wlr-layer-shell-v1`
(supported by Hyprland, Sway, River, Wayfire, KDE Plasma 5.27+,
Mir, Cosmic). GNOME's Mutter does not implement it — that's a known
limitation of every wlr-layer-shell client and not something this
ADR can fix; GNOME users fall back to xdg_toplevel.

This ADR picks the implementation path.

## Options considered

### Path A1: `iced_layershell`

[Crate](https://crates.io/crates/iced_layershell) ·
[Repo](https://github.com/waycrate/exwlshelleventloop) ·
MIT license · maintained by waycrate.

A drop-in replacement for `iced::application(...)` that swaps iced's
winit backend for a wlr-layer-shell event loop. Our existing iced
widget tree, theme, image rendering, etc. continues to work
unmodified.

| Property                  | Value                                            |
|---------------------------|--------------------------------------------------|
| Latest stable             | v0.17.1 (2026-03-25)                             |
| Latest beta               | v0.18.0-beta4 (2026-04-22, sctk refactor)        |
| Releases in last 12 mo    | ~30                                              |
| Tracks iced version       | 0.14 (matches our pin)                           |
| GitHub stars / forks      | 131 / 46                                         |
| Open issues               | 11 (mostly feature requests)                     |
| License                   | MIT                                              |
| Codebase size             | 12-crate workspace, ~15k LOC                     |
| Compositor support        | Anything supporting `zwlr_layer_shell_v1`        |

**Maintenance signals.** 62 releases in repo history. Latest beta
landed last week (sctk refactor — they're keeping pace with the
ecosystem). Maintainer is responsive on GitHub issues; commits
multiple times a week.

**API surface.** Drop-in `application(state, namespace, update,
view).settings(LayerShellSettings { … }).run()` replaces
`iced::application(...)`. `LayerShellSettings` exposes:

- `size: Option<(u32, u32)>` — panel dimensions
- `anchor: Anchor` — bitflags for screen edges (`Anchor::Bottom |
  Anchor::Left`)
- `start_mode: StartMode` — `Active` (current output) or
  `TargetScreen(name)` (specific output)
- `keyboard_interactivity: KeyboardInteractivity` —
  `None` / `OnDemand` / `Exclusive`
- `margin: (top, right, bottom, left)` — anchored offsets
- `exclusive_zone: i32` — optional reserved space (Ditto does NOT
  reserve, so we leave at 0)
- `layer: Layer` — `Background` / `Bottom` / `Top` / `Overlay`
  (we want `Top` for a launcher)

The crate also provides `#[to_layer_message]` macro that extends
our `Message` enum with built-in layer-shell control messages
(anchor change, size change, hide, etc).

### Path A2: Custom `smithay-client-toolkit` shell

Roll our own event loop with [smithay-client-toolkit](https://crates.io/crates/smithay-client-toolkit)
0.20 + a software-renderer (`tiny-skia`) or GPU renderer (`wgpu`).
Replaces iced entirely.

| Property                  | Value                                            |
|---------------------------|--------------------------------------------------|
| Latest stable             | v0.20.0                                          |
| Maintained by             | Smithay org (multi-maintainer)                   |
| License                   | MIT                                              |
| Codebase size             | well-tested, official Wayland-rs toolkit         |

**Reference point.** SCTK's own `examples/simple_layer.rs`
(currently 462 LOC) gets a single layer-shell window mapped, accepts
keyboard + pointer input, and software-renders a coloured rectangle
that changes hue on mouse move. **It does not include any UI widgets,
text rendering, font loading, theming, or selection state.** Adding
those would push us to ~1500-2000 LOC for parity with the iced
launcher.

The example file itself starts with `//! This example is horrible.
Please make a better one soon.` — accurate self-description of how
much boilerplate the API requires.

### Path A0 (status quo): keep `xdg_toplevel`

Stay on iced's default winit backend. Ship Hyprland/Sway window
rules as part of `--install-hyprland-config` (planned for Phase 4)
to force the launcher to float and anchor at bottom-left.

This is what task 026 currently lists as the "fallback if both
A1 and A2 fail." We document it here for completeness.

## Experimental data

Built and verified on this branch:

```
spike/
└── a1-iced-layershell/
    ├── Cargo.toml         (4 deps: iced, iced_layershell, iced_runtime, iced_core)
    └── src/main.rs        (179 LOC — anchor, 5 entries, ↑↓/Enter/Esc)
```

```sh
$ cd spike/a1-iced-layershell
$ nix develop -c cargo build --release
   Compiling a1-iced-layershell-spike v0.0.1
    Finished `release` profile in 1.71s
$ du -h target/release/a1-spike
23M   target/release/a1-spike
```

| Metric                       | Path A1 (iced_layershell) | Path A2 (SCTK reference)   |
|------------------------------|---------------------------|----------------------------|
| LOC for spec deliverable     | **179**                   | ~500-1000 (estimated)      |
| Reuses ditox-gui widgets     | Yes (Column/Text/Image)   | No — full rewrite          |
| New dependencies in tree     | 1 crate (`iced_layershell`) | 1 crate (`smithay-client-toolkit`) + a renderer |
| Cold compile (release)       | 5min30s first build, 1.71s incremental | not measured |
| Stripped binary size         | 23 MB                     | likely smaller (no iced)   |
| Migration effort (Phase 4)   | Replace one `iced::application(…)` call site | Rewrite `view`, `update`, theming, image cache, scrollables, focus management |

> **Visual verification still pending.** The author of this ADR is
> running in a CI-style environment without a graphical Wayland
> session. The A1 spike compiles to a runnable binary, but
> "appears at bottom-left of Hyprland" / "Esc closes" must be
> confirmed by a human running it on a real compositor before
> Phase 4 starts. See "Verification checklist" below.

## Decision

**Adopt Path A1: `iced_layershell`.**

Rationale:

1. **3-5× less code** for the same UI than A2, because we keep iced
   for everything except window management.
2. **Zero churn to existing widget code.** Phase 4 task 026 swaps
   one `iced::application(...)` call site for
   `iced_layershell::build_pattern::application(...)`; the rest of
   `ditox-gui/src/app.rs` (1100+ LOC of `update`/`view`/state) is
   untouched.
3. **Active maintenance.** 62 releases, latest landed last week,
   tracks iced versions promptly.
4. **License compatible.** MIT — same as ditox.
5. **Compositor coverage matches our target matrix.** Hyprland,
   Sway, River, Wayfire, KDE all support `zwlr_layer_shell_v1`.
   GNOME doesn't, but neither would A2; this is a protocol-level
   limitation, not a library choice.

Path A2 remains viable as a fallback if iced_layershell's beta
churn causes us pain in Phase 4. The ADR will be revisited if so.

## Risks

1. **Beta churn.** v0.18.0 is currently in beta with an
   smithay-client-toolkit refactor (issue #368). If our Phase 4
   timing lands during the beta cycle we should pin to v0.17.1
   stable and migrate to v0.18 in v0.5. **Mitigation:** pin
   `iced_layershell = "=0.17.1"` initially.
2. **iced version coupling.** iced_layershell is reactive to iced
   releases; if iced 0.15 lands before we ship Phase 4 we may need
   to wait for an iced_layershell update. **Mitigation:** stay on
   iced 0.14 until iced_layershell ships its 0.15-compatible
   release; this is consistent with the rest of the workspace.
3. **GNOME/Mutter unsupported.** `zwlr_layer_shell_v1` is not in
   Mutter and likely never will be. **Mitigation:** runtime
   detection via task 021's `Platform` enum — fall back to
   xdg_toplevel on `Platform::GnomeWayland` or `Platform::Other`.
   Document in `docs/notes/hyprland-setup.md` and the GNOME
   section of the cross-platform support matrix.
4. **Tray icon ordering.** Phase 4 needs the tray icon (currently
   spawned on its own GTK thread) to coexist with the
   `iced_layershell` event loop. Pre-existing constraint: GTK
   pumps on its own thread already, so this should not change.
   **Verify in Phase 4.**
5. **Double-window state.** The iced_layershell `application()`
   currently maps a single layer surface; opening a popup (e.g.
   "preview" pane) requires the popup APIs that the upstream
   author flagged as "still a toy" in the README. **Mitigation:**
   v1.0 launcher fits in one panel; defer popups to post-v1.0.

## Migration plan (Phase 4)

When task 026 begins:

1. Add `iced_layershell = "=0.17.1"` to `ditox-gui/Cargo.toml`.
2. In `ditox-gui/src/app.rs`:
   - Replace `iced::application(...)` with
     `iced_layershell::build_pattern::application(...)`.
   - Wrap the existing `Message` enum with `#[to_layer_message]`.
   - Move window-position logic from current `WindowState` /
     `Position::SpecificWith` into `LayerShellSettings { anchor,
     margin, size, … }`.
   - Behind `#[cfg(unix)]` only — Windows continues to use
     iced's default backend.
3. Add a runtime branch using task 021's `Platform`:
   - `Platform::Hyprland | Platform::Sway | Platform::Kde |
     Platform::Wlroots` → layer-shell.
   - `Platform::GnomeWayland | Platform::Other` → existing
     xdg_toplevel.
4. Update `docs/notes/ui-replication.md` and
   `docs/notes/hyprland-setup.md` with the new compositor flow.
5. Remove the v0.3.1 "always-on-top floating-window-rule"
   workarounds from `--install-hyprland-config`.

## Verification checklist (before merging Phase 4)

The spike's compilation alone is not proof of correctness. Before
task 026 lands the following must be observed by a human on a real
compositor:

- [x] On Hyprland (current Arch master, verified 2026-04-26):
  - [x] `cd spike/a1-iced-layershell && nix develop -c cargo run --release`
  - [x] panel appears at bottom-left of the active monitor
  - [x] keyboard focus lands on the panel (no extra `hyprctl`
    commands needed)
  - [x] Esc closes the process; exit code 0
  - [x] ↑↓ moves the highlight; Enter prints + exits
- [ ] On Sway (latest stable) — unverified; expected to work since
  both Sway and Hyprland implement `zwlr_layer_shell_v1`. Check
  before Phase 4 lands.
- [ ] On KDE Plasma 5.27+ — unverified; same expectation.
- [ ] On GNOME (Mutter) — expected to fail gracefully (Mutter
  doesn't implement wlr-layer-shell). Verify the failure mode is
  recoverable so we can fall back to xdg_toplevel cleanly.
- [ ] No CPU spin or fd leak after 60 s of idle — unverified.
- [ ] No fd leak across 100 launches in a tight shell loop —
  unverified.

If any Hyprland or Sway check fails, escalate to Path A2 before
proceeding with Phase 4.

## References

- [zwlr_layer_shell_unstable_v1 protocol](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [iced_layershell crate docs](https://docs.rs/iced_layershell/0.17.1/iced_layershell/)
- [exwlshelleventloop README](https://github.com/waycrate/exwlshelleventloop)
- [smithay-client-toolkit `simple_layer.rs` example](https://github.com/Smithay/client-toolkit/blob/master/examples/simple_layer.rs)
- ditox spike: `spike/a1-iced-layershell/`
- task 022: `docs/tasks/completed/022-foundation-layer-shell-spike.md`
- Phase 4 epic: `docs/tasks/planned/026-phase-4-ditto-ux.md`
