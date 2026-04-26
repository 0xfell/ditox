# Ditox v0.4 → v1.0 Master Plan

> Written 2026-04-26. Sets direction for ~10 months of work toward Ditto
> feature parity with cross-platform reach (Linux first-class on Hyprland +
> Sway, Windows first-class, macOS in final phase).
>
> Companion docs:
> - `docs/notes/ditto-comparison.md` — feature inventory and tradeoffs.
> - `docs/notes/ui-replication.md` — Ditto UX replication design.
> - `docs/notes/hyprland-setup.md` — end-user Hyprland setup.

---

## Decisions register

These are the answers to the open questions raised in the comparison doc,
with reasoning. "Best implementation, more time is fine" was the brief.

### D1 — Phase ordering: keep as proposed

`Foundation → Multi-format → Paste-back → Power-user → Ditto UX → Hotkeys/IPC/Scripting → LAN sync → Distribution/i18n → macOS`.

Rationale: Multi-format must precede paste-back because paste-back wants
the format aggregator infrastructure to emit *everything* the source app
published. Doing paste-back first against text-only would mean a partial
feature shipped early and rebuilt later — exactly the "quick win"
trajectory we're avoiding. Foundation first is non-negotiable; it pays
down debt and introduces the abstractions every later phase relies on.

### D2 — GUI architecture: revert to long-running

Decision: **revert task `013`'s one-shot launcher to a long-running
process with IPC**. Repurpose its visual design (420×520 floating panel)
but back it with a single instance + flock + Unix socket on Linux,
single-instance mutex + named pipe on Windows.

Rationale: per-clip global hotkeys, modifier-held cycling, foreground
snapshot accuracy, and sub-100 ms summon latency all require persistent
state. The one-shot model also breaks the "Ditto-like" UX target: every
launch starts cold, can't track foreground changes between summons, and
can't register hotkeys for individual clips.

### D3 — Scripting language: Rhai

Decision: **Rhai** (`rhai` crate, Rust-native).

Rationale:
- Pure Rust; no native dependency, no FFI surface, smaller binary,
  cross-compiles cleanly to all our targets.
- Sandbox-by-default: `Engine::set_max_operations`,
  `Engine::set_max_call_levels`, `Engine::set_max_string_size` give us
  resource bounds without writing C glue.
- No file/network builtins to disable.
- `mlua`/Lua wins on ecosystem familiarity but loses on safety,
  build-system complexity, and binary size. Lua scripts in clipboard
  managers tend to be small enough that ecosystem isn't decisive.

### D4 — Capture default: all formats with per-format size caps

Decision: **capture every format the OS publishes** with a configurable
size cap per format and a global cap per clip. Allowlist not required to
unlock HTML/RTF — those work out of the box.

Rationale: matches Ditto's behaviour (which is what users expect from a
"clipboard manager"). Multi-format storage is what makes
paste-rich-format actually work. Per-format size caps prevent runaway DB
growth (default 10 MiB per format, 25 MiB per clip — tunable).

Provide a `[capture] mode = "all" | "minimal" | "custom"` switch where
`minimal` = text + canonical-image (current ditox behaviour), `custom` =
user allowlist.

### D5 — Linux paste-back synthesis: detect-and-degrade chain

Decision: **strategy chain at runtime** with no hard dependency:
`hyprctl sendshortcut` (if Hyprland) → `wtype` → `ydotool` → manual.

Rationale: forcing `ydotool` requires uinput access setup that varies
wildly between distros and triggers user surprise. `wtype` works on most
wlroots compositors with no daemon. `hyprctl sendshortcut` is the
cleanest path on Hyprland. Falling back to "manual paste" is honest and
shipping-acceptable; users opt in to a synthesizer as they wish.

Config:
```toml
[paste]
synthesize = "auto"  # "auto" | "hyprctl" | "wtype" | "ydotool" | "off"
```

`auto` selects the first available in the order above.

### D6 — Sync scope: LAN-only in v1.0

Decision: **LAN peer-to-peer only**. Cloud relay is sketched in protocol
docs but not built.

Rationale: a relay is a separate product (server, hosting, ToS, abuse,
key rotation, account model). LAN is self-contained — no third party,
zero attack surface beyond the LAN itself, and ships in a single binary.
Wire format will be designed with relay-readiness so a future relay can
forward frames unchanged.

### D7 — macOS priority: keep last (Phase 8)

Decision: **macOS stays at the end**.

Rationale: every cross-platform abstraction we introduce in Phases 0-7
is an opportunity to learn what macOS will need. By Phase 8, the
`Capture` trait, `ForegroundTracker` trait, paste-synthesis chain, and
hotkey backend will all have stable shapes — a third platform implements
known interfaces instead of re-deriving them. Apple Developer ID
($99/yr) and notarization are also non-trivial; deferring them keeps the
Linux/Windows release cadence unblocked.

This does mean Mac users wait. We will accept PRs from macOS contributors
earlier if abstractions are ready.

### D8 — Telemetry: zero, ever, without explicit user action

Decision: **no telemetry of any kind**, including version pings and
anonymous usage stats. Crash reporting is local-file only (`human-panic`
writes a report to `/tmp` and tells the user to share manually).

Rationale: a clipboard manager has the most sensitive content on the
machine. Trust matters more than analytics. We learn from GitHub stars,
issues, and explicit user feedback.

### D9 — Branding: keep "collections" flat + add "tags"

Decision: **collections stay flat** (one entry → at most one collection).
Add a separate **tags** system (many-to-many).

Rationale: nesting collections is a premature complication. A user who
wants `Work/Email/Drafts` can have `Work` as a collection and `email,
drafts` as tags. Tags compose orthogonally and search/filter trivially.
If users still want nesting after release we can add it later — but tags
typically subsume the demand.

### D10 — Stretch features (Ditto features explicitly rejected)

| Feature | Decision | Replacement |
|---|---|---|
| Native DLL add-ins | Rejected | Rhai scripting (D3) + IPC for out-of-process plugins |
| MSHTML preview in tooltip | Rejected | Sanitised HTML rendering via a Rust crate (`ammonia` for sanitisation, fragment rendering as styled text) — no embedded browser, no CVE surface |
| MAPI send-via-email | Rejected | `mailto:` URL with body + attachment hint, opened via `open` / `xdg-open` / `start`. Cross-platform, no MAPI dependency |
| Save animation (shrinking focus rect to tray) | Rejected | Subtle iced opacity tween in the list when a new entry appears. Cheaper, less disorienting |
| U3 USB stick autorun helper | Rejected | U3 is dead. No replacement. |

---

### H1 — Compositor target priority: Hyprland & Sway first-class; KDE & GNOME best-effort

Decision: **Hyprland and Sway are first-class** (CI tested, all UX
features supported); **KDE Wayland is best-effort** (works, fewer
features); **GNOME Wayland is degraded** (works for capture/storage/UI,
several Ditto-style features unavailable due to GNOME's Wayland posture).

Rationale: Hyprland and Sway both speak the wlroots protocol set
(`wlr-layer-shell`, `wlr-foreign-toplevel-management`,
`wlr-virtual-keyboard` if we use it). One implementation covers both
nearly for free. KDE has its own ecosystem (KGlobalAccel, KWin scripting,
Plasma integration) and we'll engage with it in a later phase. GNOME
deliberately doesn't ship most wlr-* protocols; users get a working but
limited experience.

### H2 — Layer-shell vs window rules: layer-shell, properly

Decision: **invest in proper `wlr-layer-shell` integration** (option
B.1.a in the addendum). No Hyprland window-rule hack.

Rationale: layer-shell is the correct architecture for a launcher
(non-tiling, no taskbar, configurable anchors, no compositor-specific
config). The 3-4 week timeline cost is acceptable given the brief.
Implementation path: vendor or use `iced_layershell` (or its successor)
to bridge iced 0.14 → `sctk` → `wlr-layer-shell`. If that crate doesn't
fit, we write a thin custom shell using `sctk` + `tiny-skia` in a new
crate `ditox-layershell`.

### H3 — GUI architecture reversal: confirmed

Decision matches D2 above. Repurpose `013` artifacts.

### H4 — `hyprctl` dependency: optional, used only for niceties

Decision: **`hyprctl` is optional**. The always-on Linux path uses
`wlr-foreign-toplevel-management` (cross-compositor) for foreground
tracking and `wtype`/`ydotool` for keystroke synthesis.

`hyprctl` is shelled out to *only* for:
- Cursor position queries (no portable Wayland API).
- `sendshortcut` paste-back when on Hyprland and configured.

If `hyprctl` is missing on a Hyprland system we degrade gracefully —
foreground tracking and paste still work via the wlr-protocol path.

### H5 — Auto-installing Hyprland config: explicit only, never auto

Decision: **never auto-write user dotfiles**. Provide
`ditox-gui --install-hyprland-config` that writes
`~/.config/hypr/conf.d/ditox.conf` with clearly-marked sections, *only*
when the user runs it. `--help` output and `docs/notes/hyprland-setup.md`
both contain copy-pasteable snippets for users who prefer manual.

### H6 — Caret-position UX: Windows-only, ship as-is

Decision: **caret-position is a Windows-only feature**. On Wayland
(including Hyprland), the closest equivalent is "centre on active
window" — also user-selectable.

Rationale: there is no Wayland protocol for "give me the active text
caret position" and there isn't one on the horizon. We ship what works
where it works and document.

### H7-H10 — Carried over

H7 = D3 (Rhai). H8 = D4 (all formats). H9 = D6 (LAN-only). H10 = D7
(macOS last).

---

## Phase plan summary

| Phase | Theme | Duration | Schema |
|---|---|---|---|
| 0 | Foundation hardening | ~3 wk | v1 → v2 |
| 1 | Multi-format capture | ~6-8 wk | v2 → v3 |
| 2 | Paste-back UX (cross-platform) | ~3 wk | none |
| 3 | Power-user features | ~4 wk | v3 → v4 |
| 4 | Ditto UX replication (long-running, layer-shell) | ~5-6 wk | none |
| 4b | GUI parity (settings, collections, multi-select, tags) | ~3-4 wk | v4 → v5 |
| 5 | Hotkeys, IPC, Rhai scripting | ~4 wk | v5 → v6 |
| 6 | LAN sync (TOFU + Noise) | ~6-8 wk | v6 → v7 |
| 7 | Distribution & i18n | ~3 wk | none |
| 8 | macOS port | ~3-4 wk | none |

Each phase produces a release. v0.4 = Phase 0 done; v0.5 = Phase 1 done;
… v0.11 = Phase 7 done; v1.0 = Phase 8 done.

---

## Phase 0 — Foundation hardening

**Goal:** prepare substrate. No new user-visible features.

Tasks (numbered to avoid collision with existing 004, 010):
- `014-foundation-data-dir-fix.md` — honour `Config.storage.data_dir`.
- `015-foundation-docs-reconciliation.md` — fix ROADMAP, AGENTS.md, legacy IPC notes.
- `016-foundation-watcher-daemon-hardening.md` — flock PID file, signal handlers, systemd unit, `--stop`/`--status`.
- `017-foundation-async-db-actor.md` — DbActor with `mpsc::Sender<DbCommand>` + `oneshot::Sender<Result<…>>`.
- `018-foundation-capture-trait.md` — `CaptureSource` trait; refactor watcher to consume sources.
- `019-foundation-schema-v2.md` — defensive `ALTER TABLE` for `entry_kind`, `format_count`, `source_app`, indexes; bump `schema_meta.version` to 2.
- `020-foundation-tracing-logging.md` — `tracing` everywhere, `RUST_LOG` documented, no `eprintln!`.
- `021-foundation-compositor-detection.md` — `Compositor::{Hyprland, Sway, Kde, Gnome, Other, Windows, Macos}` enum + detection.
- `022-foundation-layer-shell-spike.md` — research spike: prototype `sctk` + `wlr-layer-shell` rendering a static iced widget tree. Decide on `iced_layershell` vendor vs custom crate. Output: ADR in `docs/notes/`.

Phase exit criteria:
- `cargo test --workspace` green on Linux + Windows.
- `cargo clippy --workspace -- -D warnings` clean.
- New schema v2 migrates fresh and v1-existing DBs successfully (snapshot test).
- Watcher: `ditox watch && ditox watch` second invocation refuses cleanly.
- Watcher: SIGTERM removes PID file.
- Layer-shell ADR merged.

---

## Phase 1 — Multi-format capture

**Goal:** capture every format the OS publishes; persist with stable
hashing; aggregate on multi-clip paste.

Single epic task: `023-phase-1-multi-format-capture.md`.

Subtasks (in epic body):

1. **Schema v2 → v3.** New `entry_formats` table; migration moves
   existing single-content rows to one `entry_formats` row each.
2. **Format naming convention.** MIME types preferred (`text/plain;charset=utf-8`,
   `text/html`, `image/png`, `application/x-files`). Win32-specific
   formats prefixed `win32:` (`win32:CF_DIB`, `win32:CF_HDROP`).
3. **Linux capture surface.** Replace `wl-paste` shell-out with
   `wl-clipboard-rs` library. Enumerate offered MIME types. Capture all
   below per-format size cap.
4. **Windows capture surface.** Direct `windows-rs` calls:
   `OleGetClipboard` → `IDataObject::EnumFormatEtc`. Use
   `AddClipboardFormatListener` against a hidden message-only window for
   event-driven capture (no more polling).
5. **Per-format hashing & dedup.** SHA-256 per format; entry hash = SHA-256
   of `(sorted format-name : format-hash)`. Stable canonicalisation:
   - RTF: strip `{\*\datastore}`, `\rsidN`, `\insrsidN`.
   - HTML Format: parse envelope, hash fragment only.
   - Text: trim trailing `\0` padding.
6. **Aggregator trait.** `FormatAggregator` with implementations for
   plain text, HTML envelope round-trip, RTF, file lists, image stack
   (horizontal/vertical via `image` crate).
7. **Search across formats.** FTS5 indexes `entry_formats.content` for
   inline text formats. Search mode prefixes (`/h `, `/r `, `/p `).
8. **Sentinel handling.** Honour Windows `Clipboard Viewer Ignore`,
   `ExcludeClipboardContentFromMonitorProcessing`,
   `CanIncludeInClipboardHistory==0`.
9. **Limits & quotas.** `[capture] max_format_size`, `max_clip_size`,
   `mode = all|minimal|custom`, `[capture.formats] include`/`exclude`.

Phase exit criteria:
- Capture HTML from a browser, paste into Word; formatting preserved.
- Capture file list from File Explorer / Files; paste into another
  manager; files appear.
- Capture RTF from LibreOffice; bytes stable across copies (dedup
  works).
- Migration v2→v3 round-trips without data loss; tested with snapshot
  DBs.

---

## Phase 2 — Paste-back UX

**Goal:** click an entry, the launcher closes, Ctrl-V is synthesised
into the previously-focused app. Cross-platform.

Single epic task: `024-phase-2-paste-back.md`.

Subtasks:

1. **Foreground tracker abstraction.** `ditox-core/src/foreground.rs`
   with `ForegroundTracker` trait. Snapshot includes hwnd/xid,
   process_basename, title, captured_at.
2. **Windows tracker.** `GetForegroundWindow` +
   `QueryFullProcessImageNameW`. Restore via `AttachThreadInput` +
   `BringWindowToTop` + `SetForegroundWindow` (existing
   `force_restore_window` logic, refactored).
3. **Wayland tracker.** Subscribe to
   `wlr-foreign-toplevel-management-unstable-v1` via `sctk`. Cache the
   most recent non-ditox toplevel. `hyprctl activewindow -j` enrichment
   on Hyprland.
4. **Keystroke synthesis chain (Linux).** `hyprctl sendshortcut` →
   `wtype` → `ydotool` → off. `Config.paste.synthesize`.
5. **Keystroke synthesis (Windows).** `SendInput` with stuck-modifier
   pre-flight (release every down VK).
6. **Per-app keystroke override.** `[paste.keystrokes]` map, basename →
   sequence, default `ctrl+v`.
7. **`Clipboard Viewer Ignore` sentinel emission** during paste.
8. **GUI integration.** Rename `Message::CopyEntry` → `Message::PasteEntry`.
   Snapshot foreground on launcher show; restore + synthesize on entry
   activation.
9. **Modifier-held cycling.** Detect global-hotkey re-fire while popup
   visible; advance selection without dismiss.

Phase exit criteria:
- Windows: Ctrl+Shift+V → click entry → text appears in the previous app.
- Hyprland: bind `ctrl+grave, exec, ditox-gui --toggle` → click entry →
  text appears in the previous app via `hyprctl sendshortcut`.
- Per-app override demonstrably changes behaviour for at least vim
  (`"+gp` instead of `ctrl+v`).

---

## Phase 3 — Power-user features

Single epic task: `025-phase-3-power-user.md`.

Subtasks:
1. **Special paste menu.** `Transform` trait + 20+ implementations
   (case transforms, slugify, GUID, datetime prepend/append, line-feed
   manipulation, ASCII-only, posixify, image stacking).
2. **Per-app capture exclusion.** `[capture.exclude] processes = [...]`
   glob list. Wired to `ForegroundTracker::snapshot()` from Phase 2.
3. **Color swatch detection** in list rendering.
4. **Filter rules table.** Schema v3 → v4. Regex/glob patterns,
   per-process scope, drop/transform/tag actions.
5. **Suspend/resume awareness.** `WM_POWERBROADCAST` /
   logind `PrepareForSleep`. Drop and reopen DB connection.
6. **Search mode prefixes** (`/q`, `/f`, `/r`, `/h`, `/p`).
7. **Per-resolution window state.** Migrate `window_state.json` to
   `HashMap<String, WindowGeometry>`.
8. **Translate / web-search URL templates.** Right-click → "Translate"
   / "Search web".

---

## Phase 4 — Ditto UX replication (long-running GUI + layer-shell)

Single epic task: `026-phase-4-ditto-ux.md`.

This is the **architecture reversal** — long-running GUI process + IPC.
Task `013` is partially reverted (visual design retained).

Subtasks:

1. **Long-running GUI.** Single-instance via flock + Unix socket
   (Linux) or named mutex + named pipe (Windows). First launch becomes
   the daemon; subsequent launches send IPC commands and exit.
2. **IPC protocol restoration.** Commands: `SHOW`, `HIDE`, `TOGGLE`,
   `QUIT`, `EMIT <id>`, `STATUS`, `CAPTURE`, `CYCLE-NEXT`, `CYCLE-PREV`.
   Newline-delimited text.
3. **Layer-shell rendering on Linux.** `iced_layershell` integration or
   custom `ditox-layershell` crate (decision from `022` ADR). Window
   becomes a `wl_layer_surface` on `Top` layer with `OnDemand` keyboard
   interactivity.
4. **Configurable popup position.** `Config.gui.position`:
   `at_caret` (Windows only), `at_cursor` (Windows + Hyprland via
   `hyprctl cursorpos`), `at_previous`, `at_active_window_center`,
   `fixed`.
5. **Custom non-client area.** Caption drawable on top/bottom/left/right.
   Already on Windows; extend to Linux post layer-shell.
6. **Always-on-top toggle.** Pin button. iced supports it on Windows;
   on Linux/Wayland, layer-shell already gives us this for free.
7. **Modifier-held cycling.** Implementation from Phase 2.9 wired to
   global-hotkey-or-IPC re-fire.
8. **Hide-on-blur with grace period** (configurable).
9. **Tooltip-as-preview.** Hover an entry → enlarged preview tooltip
   with text/image/sanitized-HTML.
10. **Inline list extras.** Color swatches, hotkey numbers, group/sticky
    glyphs.
11. **`ditox-gui --install-hyprland-config`.** Writes managed file
    `~/.config/hypr/conf.d/ditox.conf` with `exec-once`, `bind`, and
    `windowrule` lines guarded by markers.
12. **Per-resolution window state** (folded in from Phase 3.7 if not
    already done).

---

## Phase 4b — GUI feature parity

Single epic task: `027-phase-4b-gui-parity.md`.

1. Settings window with all tunables.
2. Collections in GUI (tab strip, create/delete, drag-and-drop).
3. Multi-select.
4. Per-row favorite toggle.
5. Image zoom in side panel.
6. Theming via `[ui.theme]` (currently TUI-only, hot-reload via
   `notify`).
7. Customizable keybindings in GUI.
8. Tags system (schema v4 → v5: `tags`, `entry_tags`).
9. Time-window filter chips (Today / Yesterday / This Week / This
   Month / Older).

---

## Phase 5 — Hotkeys, IPC, scripting

Single epic task: `028-phase-5-hotkeys-ipc-scripting.md`.

1. **Per-clip global hotkey.** Schema v5 → v6 adds `entries.global_hotkey`,
   `entries.local_hotkey`. Windows registers via `global-hotkey`. Linux:
   write hyprland binds to managed config file (Hyprland), or document
   manual setup (Sway/KDE/GNOME).
2. **IPC protocol completion.** All commands from Phase 4.2 + script
   runtime hooks.
3. **Rhai scripting.** Two hook points: `on_capture` and `on_paste`.
   Sandbox with `Engine::set_max_*`. Scripts under
   `~/.config/ditox/scripts/*.rhai`.
4. **Save-clipboard hotkey.** `ditox save` CLI + optional global hotkey
   that force-bumps a counter even on dedup hit.

---

## Phase 6 — LAN sync (TOFU + Noise)

Single epic task: `029-phase-6-lan-sync.md`.

1. mDNS-SD discovery (`mdns-sd` crate). Service type
   `_ditox._tcp.local.`.
2. ed25519 identity keypair, chmod 600.
3. TOFU pinning UI; never auto-trust.
4. Noise_XX_25519_ChaChaPoly_SHA256 transport via `snow`.
5. Length-prefixed protobuf frames over TCP.
6. Pull-based sync; entries content-addressed; pinned/last_used =
   last-write-wins on RFC3339 timestamp.
7. Image blob transfer with resumable partial-hash chunks.
8. Sync settings UI (per-peer auto-sync toggle, send-all flag).
9. Inno Setup firewall-rule prompt on install.
10. Schema v6 → v7: `peers` table (id, name, public_key, last_sync,
    trust_state).

---

## Phase 7 — Distribution & i18n

Single epic task: `030-phase-7-distribution-i18n.md`.

1. Portable Windows build (marker file).
2. Code signing via SignPath Foundation OSS.
3. Chocolatey + Winget + Scoop submissions.
4. AUR PKGBUILD (`ditox-bin`).
5. Microsoft Store (MSIX).
6. Flatpak.
7. i18n via `fluent-rs`. Initial 5 locales: en-US, es-ES, fr-FR, de-DE,
   ja-JP.
8. Crash dumps via `human-panic` + WER LocalDumps registration.
9. Stats GUI page.
10. QR code export of clip text (`qrcode` crate).
11. Subtle iced opacity tween on new-entry insert (replaces Ditto's
    save animation).

---

## Phase 8 — macOS port

Single epic task: `031-phase-8-macos.md`.

1. `MacosCapture` via `arboard` (already supports macOS) wrapped in
   `CaptureSource`.
2. Tray, hotkey, auto-launch verified on macOS.
3. Accessibility permission flow for keystroke synthesis.
4. `.app` bundle, DMG, Homebrew Cask, notarization.
5. Apple Developer ID code signing.

---

## Cross-cutting

### Schema migration testing
- One snapshot DB per version under `tests/migrations/snapshots/`.
- For every version bump, test fresh-install → vN, vN-1 → vN, … v1 → vN.
- Assert entry count + per-entry hash preservation.

### Performance budget
- Capture-to-store latency: < 50 ms text, < 200 ms image.
- GUI launch-to-visible: < 250 ms (long-running summon < 50 ms).
- List render 10k entries: < 50 ms (requires virtualization).
- Search 10k entries: < 100 ms.
- Memory idle: < 50 MB GUI / < 20 MB watcher.

### Security model
- No network without explicit user action.
- Sync is opt-in.
- Identity keys chmod 600.
- Process exclusion list ships with sensible password-manager defaults.
- Scripting sandboxed.

### Testing targets
- `ditox-core`: > 85% line coverage.
- `ditox-tui`: > 60%.
- `ditox-gui`: best-effort.
- `proptest` for dedup, transforms, format aggregators.
- Migration tests every version.

### CI matrix expansion
- Current: linux-gnu, linux-musl, linux-arm64, windows.
- Phase 0: add Hyprland headless smoke (when available).
- Phase 8: add macos-x64, macos-arm64.

---

## Open work for the user / community

When phases 0-3 are done we will have a v0.7 that significantly exceeds
ditox v0.3.1 and rivals Ditto on Windows for core capture/paste. Phase 4
brings the launcher UX to parity. Phase 5 enables scripting and per-clip
hotkeys. Phase 6 adds sync. Phase 7 polishes distribution. Phase 8 adds
macOS.

Each phase ships a release. Users get value continuously.
