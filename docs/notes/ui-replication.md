# Ditto UX Replication — Internal Design Notes

> Companion to `master-plan-v1.md` Phase 4. This document captures the
> internal design decisions for matching Ditto's launcher UX, with focus
> on Hyprland/Wayland constraints.

---

## Target behaviours (from Ditto)

The 13 launcher behaviours we're replicating, in order of importance:

1. Hidden until summoned (no taskbar entry, no startup window).
2. Global hotkey summons; same hotkey while visible cycles selection.
3. Position is contextual (caret / cursor / previous / centre / fixed).
4. Foreground app captured before show (paste-back targets it).
5. Frame is custom-drawn (caption configurable).
6. List is virtual + custom-drawn (thumbnails, color swatches, glyphs).
7. Search is incremental, mode prefixes (`/q `, `/f `).
8. Tooltip-as-preview on hover.
9. Enter pastes back; double-click pastes; Esc hides.
10. Hide-on-blur (configurable).
11. Snap to edges while dragging (Shift suppresses).
12. Per-resolution geometry.
13. Single-instance.

---

## Architecture decisions

### A1 — Long-running GUI process

**Decision:** revert task `013` partially. The GUI is a long-running
daemon. First launch creates the lock and IPC socket; subsequent
launches send commands and exit.

**Why:** behaviours 1, 2, 4, 9, 13 all require persistent state.
Foreground tracking before-show only works if the process is already
running with a tracker subscription. Modifier-held cycling needs an
in-memory selection cursor that survives between key-events. Per-clip
hotkeys (Phase 5) need persistent registration.

**Cost:** more memory at idle (target < 50 MB), slightly higher
implementation complexity for single-instance + IPC. We accept these
costs for the UX gain.

### A2 — `wlr-layer-shell` on Linux

**Decision:** ditox-gui's launcher window is a `wl_layer_surface` on
the `Top` layer with `OnDemand` keyboard interactivity, anchored
configurably.

**Why:**
- Layer-shell surfaces don't tile — Hyprland and Sway leave them where
  we put them, no window rules needed.
- `Top` layer means the launcher floats above normal apps but is hidden
  by fullscreen apps (configurable to `Overlay` if user prefers).
- `OnDemand` keyboard interactivity gives us focus while visible
  without permanently grabbing input.
- This is how every credible Wayland launcher works: `fuzzel`, `wofi`,
  `tofi`, `bemenu`, `anyrun`, `walker`.

**Implementation paths:**

- **A2a — `iced_layershell` (preferred):** the
  [waycrate/exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
  workspace ships an iced-compatible layer-shell shell. We vendor or
  depend on it directly. Accepts upstream PRs for our needs.
- **A2b — Custom `ditox-layershell` crate:** if A2a doesn't fit (iced
  version mismatch, missing features), we write a thin shell using
  `smithay-client-toolkit` + `tiny-skia` (CPU rasterizer for low
  resource use) or `wgpu` (GPU). This is more work — about 2000 LOC for
  the input/output/event-loop bridge.

The Phase 0.9 spike (`022-foundation-layer-shell-spike.md`) chooses
between A2a and A2b and produces an ADR.

### A3 — Foreground tracking strategy

**Decision:** stack two protocols.

```
     ┌────────────────────────────┐
     │ wlr-foreign-toplevel-mgmt  │  ← always on; primary
     └────────────┬───────────────┘
                  │
         on Hyprland only:
                  │
     ┌────────────▼───────────────┐
     │ hyprctl activewindow -j    │  ← enrichment
     │   - process basename       │
     │   - pid                    │
     │   - workspace, monitor     │
     │   - window address         │
     └────────────────────────────┘
```

- `wlr-foreign-toplevel-management` gives a stable async stream of
  toplevel events (created, focused, closed). We maintain a cache; on
  summon, "the most recent non-ditox toplevel" is the target.
- `hyprctl` adds the *address* (needed for `hyprctl dispatch
  sendshortcut`), process name (for per-app keystroke override), and
  workspace info (for monitor-aware positioning).

Compositor support:

| Compositor | wlr-foreign-toplevel | hyprctl | Notes |
|---|---|---|---|
| Hyprland | yes | yes | first-class |
| Sway | yes | no (use `swaymsg`) | first-class with shim |
| KDE Wayland | yes (since Plasma 5.20) | no | best-effort |
| GNOME Wayland | **no** | no | degraded; foreground unknown, paste targets last-known via Sticky Keys workarounds — document |

### A4 — Position modes

**Decision:** position math implemented in `ditox-core/src/position.rs`:

```rust
enum PositionMode {
    AtCaret,                  // Windows only; falls back to ActiveWindowCentre on Linux
    AtCursor,                 // Win: GetCursorPos; Hyprland: hyprctl cursorpos; otherwise: previous
    AtPrevious,               // Saved per-resolution
    ActiveWindowCentre,       // Win: window rect; Wayland: hyprctl/swaymsg or screen centre
    Fixed { anchor: Anchor, monitor: MonitorSelector, offset: (i32, i32) },
}

struct Anchor {
    horizontal: HAnchor,  // Left, Centre, Right
    vertical: VAnchor,    // Top, Middle, Bottom
}
```

The `WindowState` resolver in `ditox-gui` consults `PositionMode`,
queries the relevant subsystem, applies per-resolution clamp, and
returns the final geometry.

### A5 — Modifier-held cycling protocol

**Decision:** detect via the IPC layer, not via OS-level modifier
polling.

When the user presses the global hotkey:
- Compositor (Linux) or `global-hotkey` (Windows) fires.
- The launcher window is shown if hidden, OR the next entry is selected
  if already visible AND the previous show was less than `cycle_window_ms`
  ago (default 800 ms).

Implementation:
- A local atomic counter increments on each `--toggle` IPC reception.
- A timer resets it to "fresh" after `cycle_window_ms`.
- "Fresh" first toggle = show + reset selection.
- "Within window" subsequent toggles = advance selection by 1.
- Backwards cycling via `--cycle-prev` (or Shift + hotkey when feasible).

### A6 — Tooltip-as-preview

**Decision:** custom iced widget `EntryPreview` that renders next to
the hovered list row. Not a system tooltip — those are too small and
don't support rich content.

Content modes:
- Plain text: monospace, syntax highlighted via `syntect` if file
  extension hint is available.
- Image: large thumbnail with dimensions and size.
- HTML: sanitised via `ammonia`, rendered as simple styled text
  (limited tag support: `b`, `i`, `u`, `a`, `code`, `pre`, headers,
  lists). No JavaScript, no embedded resources, no MSHTML.
- RTF: stripped to plain text.
- Files: list of paths.

Triggered after `hover_delay_ms` (default 400 ms). Hidden on mouse-leave
or selection change.

### A7 — List virtualization

**Decision:** Phase 4b ships with iced's standard `Column` (works for
the typical < 500 entries). Phase 4b extension if perf is an issue:
write a `VirtualList` widget that only constructs row containers for
viewport-visible rows.

Inputs to the decision: profiling at 1k, 5k, 10k entries with iced
0.14. If < 50 ms render at 10k, defer the virtualization.

### A8 — Single-instance + IPC

**Linux (`ditox-gui` daemon):**

```
$XDG_RUNTIME_DIR/ditox-gui-${UID}.lock   # flock(LOCK_EX | LOCK_NB)
$XDG_RUNTIME_DIR/ditox-gui-${UID}.sock   # Unix socket, 0600
```

Wire protocol: newline-delimited UTF-8 commands. One command per
connection. Response: `OK\n` or `ERR <msg>\n`.

```
TOGGLE          # show if hidden, hide if shown
SHOW            # ensure visible
HIDE            # ensure hidden
QUIT            # daemon exit
EMIT <id>       # paste-clip <uuid> bypassing UI
STATUS          # returns "VISIBLE\n" or "HIDDEN\n" plus selected id
CYCLE-NEXT      # advance selection (modifier-held pattern)
CYCLE-PREV      # retreat selection
CAPTURE         # force-capture current OS clipboard
PASTE-CLIP <id> # like EMIT but for per-clip hotkey integration
```

**Windows:**

```
\\.\pipe\ditox-gui-{Username}     # named pipe with default ACL
```

Same wire protocol. Single-instance via `CreateMutexW(L"ditox-gui-{user}")`.

### A9 — Hide-on-blur

**Decision:** configurable, with a 250 ms grace period to ignore
spurious unfocus events that happen during the show animation.

```toml
[gui]
hide_on_blur = true
hide_on_blur_grace_ms = 250
```

When `hide_on_blur = false`, the launcher stays open until explicit
dismiss (Esc or pin toggle). Useful when comparing many clips
side-by-side with another window.

### A10 — Per-resolution geometry

**Decision:** `window_state.json` becomes:

```jsonc
{
  "geometries": {
    "1920x1080@DP-1:LG_DISPLAY_ABC": {
      "x": 20, "y": 540, "w": 420, "h": 520,
      "anchor": "bottom-left",
      "last_used": "2026-04-26T19:00:00Z"
    },
    "3840x2160@HDMI-1:DELL_U2720Q": { ... }
  },
  "last_resolution_key": "1920x1080@DP-1:LG_DISPLAY_ABC"
}
```

Resolution key construction:

- **Windows:** `EnumDisplayMonitors` + `MONITORINFOEX.szDevice` and
  `EnumDisplayDevicesW` for the display name.
- **Wayland:** monitor model + serial via `wl_output` events (sctk
  exposes these).

If no entry matches the current resolution, fall back to
`last_used` ordering, then default geometry.

---

## Phase ordering inside Phase 4

Suggested sub-phase ordering (each ~1 week):

1. **Long-running daemon scaffold + IPC** (no UX changes yet).
   Repurpose `013`'s window code; add lock+sock; reroute CLI flags.
2. **Layer-shell prototype** (driven by `022` ADR). Window swap from
   xdg_toplevel to layer_surface.
3. **Foreground tracker & position modes.** Plug into Phase 2's tracker
   abstraction.
4. **Modifier-held cycling + always-on-top + hide-on-blur grace.**
5. **Tooltip-as-preview + inline list extras** (color swatches, glyphs).
6. **`--install-hyprland-config` helper.**
7. **Per-resolution geometry + window state migration.**

Each sub-phase is mergeable independently and ships behind a feature
flag (`gui.experimental.layer_shell`, etc.) until the whole phase is
green.

---

## Testing strategy

- **Visual regression** via `iced` snapshot widgets. Render to a
  software canvas via `tiny-skia` and diff against checked-in PNGs.
- **IPC integration tests** that spawn two ditox-gui processes and
  verify the second sends commands to the first.
- **Hyprland headless smoke test** in CI: launch Hyprland in a nested
  Wayland session via `Hyprland --headless`, run ditox-gui, send
  `TOGGLE` over the socket, assert the layer surface is visible via
  compositor introspection. Optional — adds CI cost.
- **Manual test matrix** documented in `tests/manual/phase-4.md` with
  screenshots of expected behaviour on Hyprland, Sway, Windows 11.

---

## Open implementation questions (to resolve during Phase 4)

These are not blocking phase entry; they're acknowledged unknowns:

- **Cycling backwards UX.** Shift+hotkey is unreliable on most
  compositors (modifier state unknown to ditox). Options: dedicated
  `--cycle-prev` IPC command bound to a separate hotkey, or arrow keys
  while the popup is visible. Probably arrow keys.

- **Snap-to-edges on Wayland.** Compositor manages window movement when
  the user drags. Hyprland exposes `windowrulev2 = move`, but on-the-fly
  snapping during a drag is the compositor's job, not ours. We accept
  this limitation; on Wayland the launcher is layer-shell-anchored
  anyway, no manual move.

- **Tooltip positioning at screen edges.** Standard "flip if
  overflowing" logic. Implementer's choice.

- **Keyboard focus loss when a modal appears (e.g. confirmation
  dialog).** Layer-shell `OnDemand` interactivity should handle this;
  if not, we may need to grab keyboard explicitly while modal is open.
