# Task: Layer-shell research spike

> **Status:** completed
> **Priority:** high
> **Phase:** 0 — Foundation
> **Created:** 2026-04-26
> **Completed:** 2026-04-26

## Description

Phase 4 will rewrite the Linux GUI to use `wlr-layer-shell` instead
of `xdg_toplevel`. This task is a research spike to decide between
two implementation paths and produce an Architecture Decision Record
(ADR).

## Requirements

- [x] **Path A1 prototype** — `iced_layershell` against our pinned
  iced 0.14: `spike/a1-iced-layershell/`, 179 LOC, builds clean
  (release, 1.71s incremental, 23 MB binary). Renders 5 hard-coded
  entries with ↑↓ navigation and Esc/Enter handling.
- [x] **Path A2 reference** — `smithay-client-toolkit`'s own
  `examples/simple_layer.rs` (462 LOC for layer surface boilerplate
  alone, no widgets). Documented in the ADR as reference for the LOC
  comparison; not built as a separate spike since the comparison is
  already overwhelming.
- [x] **ADR** at [`docs/notes/adr/0001-layer-shell-strategy.md`](../../notes/adr/0001-layer-shell-strategy.md)
  documenting the decision, experimental data, risks, and migration
  plan. Includes a verification checklist for visual confirmation on
  real Hyprland and Sway sessions before Phase 4 lands.
- [x] **Phase 4 task file (`026-phase-4-ditto-ux.md`) updated** with
  the chosen path (A1, with concrete integration steps and pin
  recommendation `iced_layershell = "=0.17.1"`).

## Decision

**Path A1 (`iced_layershell`)** — see ADR 0001.

Headline rationale:
- 3-5× less code than A2 for the same UI.
- Drop-in: replace one `iced::application(...)` call site; preserve
  all 1100+ LOC of existing iced widget code in `ditox-gui/src/app.rs`.
- Active maintenance (62 releases, latest landed within a week of
  this spike).
- License compatible (MIT).
- Compositor coverage matches our target matrix (Hyprland, Sway,
  River, Wayfire, KDE; GNOME falls back to xdg_toplevel — same
  limitation as A2 since wlr-layer-shell is a protocol-level boundary).

Pin to `=0.17.1` initially to avoid the in-flight v0.18 beta churn;
revisit when v0.18 stabilises.

## Visual verification

**Hyprland: confirmed working by project lead 2026-04-26**
(`cd spike/a1-iced-layershell && nix develop -c cargo run --release`,
reported "works well").

**Sway / KDE / GNOME / longevity tests: pending.** ADR carries the
remaining checklist; verify before Phase 4 task 026 begins
implementation.

```sh
cd spike/a1-iced-layershell
nix develop -c cargo run --release
# Expect: 420x520 panel at bottom-left of active monitor.
# Esc → exit. ↑↓ → move highlight. Enter → print + exit.
```

If any Sway check later fails, escalate to Path A2 before proceeding
with Phase 4.

## Implementation Notes

The spike is a separate, non-workspace Cargo project at
`spike/a1-iced-layershell/`. Keeping it out of the main workspace
means the spike's deps don't pollute production `Cargo.lock`, and we
can iterate on the spike without re-resolving the whole workspace
graph.

The only API friction encountered was that
`iced::keyboard::on_key_press` (used in the iced_layershell upstream
example) doesn't exist in iced 0.14; we used
`event::listen_with(...)` instead. This is a minor iced 0.14 detail
and works equivalently.

## Work Log

### 2026-04-26
- Surveyed `iced_layershell` (waycrate/exwlshelleventloop): v0.17.1
  stable from 2026-03-25, v0.18.0-beta4 in dev, MIT, tracks iced
  0.14, 131 stars, 62 releases historical.
- Surveyed `smithay-client-toolkit` 0.20: well-maintained, but
  reference example for a single layer-shell window is 462 LOC and
  carries the comment "This example is horrible. Please make a
  better one soon."
- Built A1 prototype at `spike/a1-iced-layershell/` (179 LOC,
  builds release in 1.71s incremental). Renders 5 entries, anchors
  bottom-left, handles ↑↓/Esc/Enter via `event::listen_with`.
- Decided to skip building a separate A2 prototype — the LOC and
  feature comparison was already conclusive without the redundant
  build, and the SCTK reference is publicly available for anyone
  wanting to verify the cost.
- Wrote ADR at `docs/notes/adr/0001-layer-shell-strategy.md` with
  decision, experimental data, risks, migration plan, and a
  verification checklist for the human running real compositors.
- Updated Phase 4 task `026-phase-4-ditto-ux.md` section 4.3 with
  the chosen path, concrete integration steps, and pin
  recommendation `iced_layershell = "=0.17.1"`.
- Added `spike/` to git tracking.
- **Hyprland verification done by user.** Project lead ran the
  spike on a real Hyprland session and reported "works well".
  ADR Hyprland boxes ticked. Sway / KDE / GNOME / longevity
  tests remain pending; tracked in ADR for whoever brings up
  Phase 4.
