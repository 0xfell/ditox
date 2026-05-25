# Ditox Linux Final Product Plan

This document is intended to be handed to another AI agent or engineer without
additional conversation context. It defines the target, current repo state,
implementation work, test work, and release gates needed to turn Ditox into a
final Linux product rather than an MVP.

## Target

Ship Ditox as a polished Linux clipboard manager with both TUI and GUI support.

Supported final-product platforms:

- Hyprland on Wayland: first-class.
- Sway on Wayland: first-class.
- Generic wlroots compositors such as river and Wayfire: supported where the
  compositor exposes the required protocols.
- KDE Wayland: supported where the required protocols are available; gracefully
  degrade unavailable features.
- GNOME Wayland: supported in degraded mode because Mutter does not expose the
  wlroots protocols needed for layer-shell and foreground tracking.
- X11: best-effort only, not a final-product promise unless explicitly added
  later.

Deferred platforms:

- Windows.
- macOS.

Windows and macOS code may remain in the repository, but final Linux release
claims must not depend on them. Any Windows/macOS tasks in roadmap docs should
be marked as deferred or future-port work, not final-product blockers.

## Release Bar

This is a strict release gate. A final release is blocked unless all of these
pass:

```sh
cargo fmt --all -- --check
nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings
nix develop --command cargo test --workspace --locked
nix build --no-link .#default
```

Additionally, Linux end-to-end tests must pass for:

- CLI workflows.
- TUI workflows that can be tested headlessly.
- GUI summon/select/paste/hide smoke flows on Hyprland.
- GUI summon/select/paste/hide smoke flows on Sway.
- Degraded-mode behavior on GNOME Wayland, if automated in CI; otherwise a
  scripted reproducible manual check must be completed before release.

The release is not final if advertised Linux features are implemented only as
stubs, no-op handlers, or documentation promises.

## Current State From Audit

This section records the initial audit that produced this plan. See
"Implementation Progress" below for work already completed from the plan.

The repository root is:

```text
/home/friend/dev/personal/ditox
```

Workspace crates:

- `ditox-core`: shared database, clipboard, watcher, config, transforms,
  foreground tracking, paste synthesis, sync, scripting.
- `ditox-tui`: `ditox` binary, CLI, terminal UI.
- `ditox-gui`: `ditox-gui` binary, iced GUI, tray, IPC, layer-shell path.

Validation already observed:

- `cargo fmt --all -- --check`: passed.
- `nix develop --command cargo test --workspace --locked`: passed.
- `nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `nix build --no-link .#default`: passed.
- Bare host `cargo test --workspace --locked` failed because the base shell was
  missing `pkg-config` and `glib-2.0.pc`; the repo-provided Nix shell supplies
  these.

Important discovered gaps:

- Root-level `tests/*.rs` are orphaned by the virtual workspace and are not run
  by `cargo test --workspace`.
- `cargo test --test cli_tests --locked` fails with "no test target named
  `cli_tests`".
- There are 147 `#[test]` functions under root `tests/` that must be moved into
  package-owned integration tests.
- The DB supports multi-format storage, but the watcher still collapses a clip
  to one canonical format in `ditox-core/src/watcher.rs`.
- Filter rule `transform:<id>` is logged but not applied.
- TUI actions `ShowActions` and `ShowStats` are currently no-op handlers.
- Wayland capture `subscribe()` is a stub; watcher polling is still the active
  path.
- Hyprland foreground tracker `subscribe()` is a stub; snapshot-on-demand works.
- Windows foreground tracking and paste synthesis are no-op/deferred. This is
  acceptable only after docs clearly scope the final product to Linux.
- Non-Unix GUI IPC is unsupported. This is acceptable only after docs clearly
  scope the final product to Linux.

## Implementation Progress

Completed on 2026-05-25:

- Created `docs/tasks/in-progress/036-linux-final-product.md` and updated
  roadmap/release docs for the Linux final-product task.
- Moved orphaned root integration tests into package-owned test directories:
  `ditox-core/tests/` and `ditox-tui/tests/`.
- Added `scripts/check-no-root-tests.sh` and wired it into CI and release
  checks.
- Implemented watcher multi-format capture end to end:
  `Watcher::process_clip` now prepares all allowed formats, chooses a canonical
  display entry, and calls `Database::insert_multi` for extras.
- Implemented capture filter `transform:<id>` wiring for text clips.
- Replaced TUI `ShowActions` and `ShowStats` no-op handlers with real status
  summaries.
- Replaced Hyprland foreground `subscribe()` empty-channel stub with a
  socket2-backed subscription.
- Replaced Wayland capture `subscribe()` empty-channel stub with a
  polling-backed subscription.
- Fixed GUI tray/window toggle behavior so a hidden daemon window can be shown
  again through the shared toggle path.
- Fixed Hyprland layer-shell hide/show behavior: hide now moves the layer
  surface off-screen at 1x1 and drops exclusive keyboard interactivity; show
  restores the configured geometry and interactivity.
- Added `scripts/smoke-gui-hyprland.sh`, a reproducible CLI smoke test for the
  live Hyprland daemon IPC path.
- Exposed `ditox-gui --status` through the public CLI so compositor smoke
  scripts can assert daemon visibility through IPC.
- Added `scripts/smoke-gui-sway.sh` and
  `scripts/smoke-gui-gnome-degraded.sh` as reproducible live-session smoke
  gates for Sway and GNOME degraded mode.
- Fixed GUI icon rendering in the Hyprland layer-shell path. The xdg-toplevel
  path renders Bootstrap Icons correctly, but the layer-shell backend shows
  missing-glyph boxes for the custom icon font, so layer-shell now uses
  standard system-font symbols while non-layer-shell windows keep Bootstrap
  Icons. Stale Bootstrap codepoints were also corrected to match the bundled
  `iced_fonts 0.3` font.

Verified on 2026-05-25:

```sh
cargo fmt --all -- --check
scripts/check-no-root-tests.sh
bash -n scripts/smoke-gui-hyprland.sh scripts/smoke-gui-sway.sh scripts/smoke-gui-gnome-degraded.sh
nix develop --command cargo test -p ditox-core --locked
nix develop --command cargo test -p ditox-gui --locked
nix develop --command cargo test -p ditox-tui --test cli_tests --locked
nix develop --command cargo test --workspace --locked
nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings
nix develop --command cargo build -p ditox-gui --locked
target/debug/ditox-gui --help | rg -- --status
nix build --no-link .#default
scripts/smoke-gui-hyprland.sh
nix develop --command cargo clippy -p ditox-gui --all-targets --locked -- -D warnings
```

Additional smoke-script behavior verified on this Hyprland host:

- `target/debug/ditox-gui --status` exits `1` with `not-running` when no daemon
  is active.
- `scripts/smoke-gui-sway.sh` exits `2` with a clear "requires a live Sway
  session" message outside Sway.
- `scripts/smoke-gui-gnome-degraded.sh` exits `2` with a clear "requires a
  live GNOME Wayland session" message outside GNOME.
- Manual screenshot validation on Hyprland confirmed the layer-shell GUI no
  longer renders square missing-glyph placeholders for the header/search action
  icons. Masking Hyprland detection to force the xdg-toplevel path confirmed
  Bootstrap Icons still render there.

Still required before marking the final-product goal complete:

- Run `scripts/smoke-gui-sway.sh` inside a live Sway session.
- Run `scripts/smoke-gui-gnome-degraded.sh` inside a live GNOME Wayland
  session.
- Decide whether the live compositor smokes can run in CI or remain documented
  pre-release manual gates.

## Phase 0: Create Tracking Task And Reconcile Scope

Follow the repository workflow in the existing instructions:

- Create a task file in `docs/tasks/in-progress/`, for example:
  `docs/tasks/in-progress/036-linux-final-product.md`.
- Base it on `docs/tasks/TEMPLATE.md`.
- Update `docs/ROADMAP.md` status counts and the in-progress table.

Required doc changes:

- `docs/ROADMAP.md`:
  - State that the final-product release target is Linux only.
  - Move Windows and macOS tasks to a "Deferred / Future Ports" section.
  - Keep Linux tasks as release blockers.
  - Remove or rewrite stale quick-reference text that claims one-shot GUI
    behavior if the code is currently long-running daemon mode.
- `docs/features.md`:
  - List only Linux-supported final features as shipping.
  - Mark GNOME limitations clearly.
  - Remove final-product claims for Windows/macOS.
- `README.md`:
  - Document Linux install/build/run paths.
  - Make `nix develop` the recommended development environment.
  - Document required non-Nix system packages for non-Nix Linux users:
    `pkg-config`, GLib, GTK3, Wayland, libxkbcommon, libappindicator or
    Ayatana AppIndicator, X11 fallback libraries, Vulkan/OpenGL/fontconfig.
- `docs/notes/hyprland-setup.md` and any Sway docs:
  - Ensure keybinding and startup instructions match the final daemon/IPC
    behavior.

Acceptance criteria:

- A new in-progress task exists for this final-product work.
- Roadmap counts are correct.
- Linux final-product scope is unambiguous.
- Windows/macOS are not advertised as release-blocking supported platforms.

## Phase 1: Make All Tests Actually Run

Problem:

- Root-level integration tests under `tests/` are not owned by any package in
  this virtual workspace.
- They are invisible to `cargo test --workspace`.

Implementation:

1. Move root tests to package-owned integration tests.

   Suggested mapping:

   ```text
   tests/cli_tests.rs                    -> ditox-tui/tests/cli_tests.rs
   tests/db_tests.rs                     -> ditox-core/tests/db_tests.rs
   tests/entry_tests.rs                  -> ditox-core/tests/entry_tests.rs
   tests/clipboard_tests.rs              -> ditox-core/tests/clipboard_tests.rs
   tests/pagination_benchmark_tests.rs   -> ditox-core/tests/pagination_benchmark_tests.rs
   tests/common/mod.rs                   -> move helpers into the crate test tree that uses them
   ```

   If a test needs both `ditox-core` and the `ditox` binary, put it under
   `ditox-tui/tests` because `assert_cmd::Command::cargo_bin("ditox")` is tied
   to the `ditox-tui` package.

2. Update imports and helper module paths.

   Examples:

   - Replace `mod common;` with a package-local helper module.
   - Make test fixtures use isolated `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, and
     `XDG_RUNTIME_DIR`.
   - Avoid reading or writing the user's real clipboard DB.

3. Add a root-test guard.

   Create a small script:

   ```text
   scripts/check-no-root-tests.sh
   ```

   Required behavior:

   - Fails if any `tests/*.rs` file exists at repository root.
   - Allows `tests/README.md` only if a root tests directory is kept for
     documentation.

4. Wire the guard into CI and local release checks.

   Add it to GitHub Actions and `docs/RELEASING.md`.

Acceptance criteria:

- `cargo test --workspace --locked` runs the migrated tests.
- `cargo test -p ditox-tui --test cli_tests --locked` works.
- `cargo test -p ditox-core --test db_tests --locked` works.
- `cargo test -p ditox-core --test entry_tests --locked` works.
- `cargo test -p ditox-core --test clipboard_tests --locked` works.
- The root `tests/` directory no longer contains Rust test files.
- The total number of executed tests increases by roughly the 147 tests that
  were previously orphaned, adjusted only for duplicate or obsolete tests.

## Phase 2: Complete Multi-Format Capture End To End

Problem:

- Schema v3 has `entry_formats` and `Database::insert_multi`.
- `WaylandLibraryCapture` can return a `RawClip` with multiple formats.
- `Watcher::process_clip` still picks one format:
  - image first if any `image/*` exists,
  - otherwise `text/plain`,
  - otherwise skip.
- Extra formats are not persisted during normal watcher capture.

Target behavior:

- A clipboard event with multiple formats produces one `entries` row and all
  allowed formats in `entry_formats`.
- The canonical entry remains what the UI displays by default:
  - image if an image format is present,
  - otherwise plain text if present,
  - otherwise the best text-like supported format.
- Extra formats are stored as `ExtraFormat` rows through
  `Database::insert_multi`.
- Rich text search can find content in stored text/html and text/rtf formats.
- Size caps and capture allow/exclude rules continue to apply before DB insert.

Implementation details:

1. Add a conversion helper in `ditox-core/src/watcher.rs` or a new core module.

   Suggested shape:

   ```rust
   struct PreparedClip {
       entry: Entry,
       canonical_hash: String,
       extras: Vec<ExtraFormat>,
       clip_hash: String,
       matched_text_for_rules: Option<String>,
   }
   ```

   Required logic:

   - Compute `clip_hash(&RawClip)` once for dedup against `last_hash`.
   - Select canonical format deterministically:
     - First image by preferred MIME order:
       `image/png`, `image/jpeg`, `image/webp`, `image/gif`, `image/bmp`,
       `image/tiff`, then any other `image/*`.
     - Else first `text/plain`.
     - Else first text-like format recognized by `FormatId`.
   - Build `Entry::new_image` or `Entry::new_text` for the canonical content.
   - Build `ExtraFormat` for every non-canonical format that should be stored.
   - Do not duplicate the canonical row in extras.

2. Use `Database::insert_multi(&entry, &extras)` for watcher inserts.

   Do not call plain `db.insert(&entry)` for multi-format clips.

3. Blob handling:

   - For canonical image entries, preserve current content-addressed image
     storage behavior.
   - For extra image formats, rely on `insert_multi` and `ExtraFormat` storage
     semantics.
   - Ensure rollback behavior is tested if an extra image blob is stored and DB
     insertion fails.

4. Dedup behavior:

   - Use canonical inner hash for `db.exists_by_hash`.
   - If the canonical content already exists, do not create a duplicate entry.
   - Still update `last_hash` for a seen clipboard event after all skip checks
     that are supposed to suppress only this event.
   - Preserve current intentional behavior for excluded apps and drop rules:
     they must not advance `last_hash`.

5. Sync behavior:

   - Confirm `sync.rs` payload import/export already uses `entry_formats`.
   - Add or update tests so a synced multi-format entry arrives with its extra
     formats intact.

Tests:

- Unit test canonical selection:
  - image plus URL text selects image canonical and stores URL/text as extra.
  - plain text plus HTML selects plain text canonical and stores HTML extra.
  - HTML-only selects a text-like canonical if supported.
- Integration test watcher capture with a mock `RawClip` containing plain text,
  HTML, RTF, and image data.
- Assert `entries.format_count` equals stored format count.
- Assert `entry_formats` contains the expected format names.
- Assert `format_content_fts` contains searchable text for inline text formats.
- Assert duplicate canonical content does not create a second entry.
- Assert root migrated DB tests still pass.

Acceptance criteria:

- Normal watcher capture persists all allowed formats.
- Search prefixes such as `/h`, `/r`, `/p`, and full text search work against
  captured multi-format entries.
- Browser "copy image" still displays as image, not copied URL text.

## Phase 3: Complete Capture Transform Rules

Problem:

- Filter action `transform:<transform-id>` exists.
- `Watcher::process_clip` logs that transform is not wired and captures as-is.

Target behavior:

- Transform rules mutate the captured text payload before hashing, dedup, and
  DB insert.
- Transform rules affect only text formats they are designed to transform.
- Image-only clips are unaffected unless future image transforms are added.

Implementation details:

1. In `Watcher::process_clip`, after matching a rule with
   `FilterAction::Transform(id)`:

   - Resolve the transform from `ditox-core/src/transforms`.
   - If the clip has `text/plain`, transform that payload.
   - If transform resolution fails, log a warning and capture original content
     only if this is the desired existing behavior. Prefer failing closed for
     malformed user rules: skip the transform but do not crash.

2. Recompute all hashes after mutation:

   - Recompute `clip_hash`.
   - Recompute canonical inner hash.
   - Ensure `last_hash` stores the post-transform clip hash.

3. Interaction with tag/drop rules:

   - Keep first-match-wins rule behavior.
   - A transform action should not also tag unless the rule engine is extended
     to support multiple actions. Do not invent multi-action behavior in this
     task.

Tests:

- Rule `transform:lower-case` changes captured text before insert.
- Rule `transform:slugify` changes captured text before insert.
- Duplicate detection uses transformed content.
- A transform rule scoped to a process only applies when foreground basename
  matches.
- Image-only clip with transform rule does not panic and does not corrupt the
  image entry.
- CLI `ditox rules add --action transform:<id>` plus watcher mock path is
  covered if practical.

Acceptance criteria:

- No log line remains saying transform action is not wired.
- Transform rule behavior is documented in CLI help and feature docs.

## Phase 4: Replace TUI No-Ops With Real Product Behavior

Problem:

- `ShowActions` in `ditox-tui/src/ui/mod.rs` is a TODO no-op.
- `ShowStats` in `ditox-tui/src/ui/mod.rs` is a TODO no-op.

Target behavior:

- No advertised keybinding silently does nothing.
- Either implement the behavior or remove the binding from user-visible docs and
  defaults.

Implementation options:

1. Preferred: implement both.

   Command palette:

   - Modal overlay listing available actions for current context.
   - Search/filter actions by name.
   - Execute selected action with Enter.
   - Close with Esc.
   - Must include actions such as copy, delete, favorite, edit note, toggle
     preview, transform, open URL action, tag, collection operations where
     already supported.

   Stats overlay:

   - Reuse existing stats logic from CLI if possible.
   - Show total entries, text/image counts, favorites, collections, total DB
     size, image storage size, top copied entries, recent activity.
   - Close with Esc.

2. Acceptable only if explicitly chosen in docs: remove both.

   - Remove keybindings.
   - Remove help text.
   - Remove feature docs mentioning them.
   - Add tests confirming no stale action appears in help.

Use option 1 for final product unless there is a strong reason not to.

Tests:

- Unit tests for action availability per mode.
- TUI rendering tests for command palette and stats overlay, if the current TUI
  test style supports snapshots or buffer assertions.
- Key handling tests:
  - palette opens,
  - filtering changes selection,
  - Enter dispatches selected action,
  - Esc closes.
- Stats overlay test with temp DB fixture.

Acceptance criteria:

- `rg "TODO: Implement" ditox-tui/src` has no product-surface TODOs.
- Help/docs accurately reflect implemented keybindings.

## Phase 5: Finish Linux Foreground Tracking And Paste-Back

Current state:

- Hyprland snapshot/restore exists through `hyprctl`.
- Hyprland subscription is a stub.
- `WlrForegroundTracker` exists for generic wlroots.
- `build_default_tracker` uses:
  - Hyprland tracker for Hyprland,
  - WLR tracker for Sway/generic wlroots/KDE where available,
  - noop fallback otherwise.
- Paste synthesis chain supports `hyprctl`, `wtype`, `ydotool`, and `off`.

Target behavior:

- Hyprland and Sway paste-back are reliable enough for final product.
- GNOME degrades honestly to clipboard write plus manual paste, without
  pretending paste-back is supported.
- Foreground tracking does not capture Ditox itself as target.

Implementation:

1. WLR tracker hardening:

   - Verify `WlrForegroundTracker` activation and snapshot behavior on Sway.
   - Add robust handling for toplevel close, handle ID reuse, missing seat, and
     compositor disconnect.
   - Ensure `ForegroundId::supports_restore()` is true only when restore is
     actually attempted and meaningful.

2. Hyprland tracker:

   - Keep `hyprctl` snapshot/restore as preferred Hyprland path.
   - Optionally implement subscription by tailing Hyprland socket2 events, but
     do not block final release on subscription if summon-time snapshot is
     proven reliable.

3. Paste synthesis:

   - Confirm `wtype` path works on Sway.
   - Confirm `hyprctl sendshortcut` path works on Hyprland.
   - Confirm `ydotool` fallback reports clear status when unavailable.
   - Enforce timeout for spawned paste commands. `SPAWN_TIMEOUT` is currently
     documented but not enforced. Implement timeout so a hung external command
     cannot hang the GUI.

4. GUI flow:

   - On summon, capture previous foreground before Ditox gains focus.
   - On selection, write clipboard, hide window, restore previous foreground if
     supported, synthesize paste sequence.
   - If restore or synthesis fails, leave clipboard set and show/log a clear
     degraded outcome.

Tests:

- Unit tests for WLR state transitions.
- Unit tests for process self-filtering.
- Unit tests for paste command timeout.
- Mock integration test for GUI paste-and-hide decision flow.
- Automated compositor E2E for Hyprland and Sway:
  - Launch a text input app.
  - Put known entry in DB.
  - Summon `ditox-gui`.
  - Select entry.
  - Assert text appears in original app.
  - Assert no self-recapture entry was added.

Acceptance criteria:

- Hyprland paste-back works in automated E2E.
- Sway paste-back works in automated E2E.
- GNOME degraded mode is tested or manually scripted with exact expected output.

## Phase 6: Finish Layer-Shell GUI Polish

Current state:

- Linux layer-shell is used for wlroots/KDE-ish platforms where configured.
- Layer-shell drag handle is still a planned task.

Target behavior:

- Launcher behaves like a polished desktop utility:
  - fast summon,
  - stable position,
  - no taskbar pollution where compositor protocols allow,
  - pin/hide behavior reliable,
  - keyboard and mouse interaction complete,
  - geometry persists as configured.

Implementation:

1. Implement layer-shell drag handle for layer-shell windows.

   Required state:

   ```rust
   struct DragState {
       press_local: iced::Point,
       margin_at_press: (i32, i32, i32, i32),
       anchor: iced_layershell::reexport::Anchor,
   }
   ```

   Add to app state:

   - `drag_state: Option<DragState>`
   - `last_cursor_local: Option<Point>`
   - `current_margin: (i32, i32, i32, i32)`
   - `current_anchor`

   Required messages:

   - `TitleDragStart`
   - `CursorMovedLocal(Point)`
   - `DragEnd`
   - Optional `DragCancel`

   Behavior:

   - Drag starts only from title bar.
   - Dragging updates layer-shell margin through `MarginChange`.
   - Releasing stops drag.
   - Esc during drag restores margin from `margin_at_press`.
   - `at_previous` persists the new geometry.
   - Non-layer-shell platforms keep existing `iced::window::drag` behavior.

2. Harden blur/pin behavior:

   - Hide-on-blur should honor grace period.
   - Pin should keep launcher visible across blur.
   - Esc should close/hide consistently.

3. Visual and interaction polish:

   - Ensure text does not overlap in narrow launcher width.
   - Ensure image thumbnails do not reflow list rows unpredictably.
   - Ensure keyboard focus is predictable after summon.
   - Ensure tag/collection/settings panels remain usable in 420 x 520 and
     reasonable larger sizes.

Tests:

- Unit tests for anchor-aware drag margin math.
- State tests for drag cancel and persist.
- GUI E2E screenshots on Hyprland and Sway.
- Pixel/screenshot checks:
  - launcher visible,
  - nonblank content,
  - no obvious overlap,
  - modal overlays fit within window.

Acceptance criteria:

- Hyprland and Sway layer-shell drag works.
- xdg_toplevel fallback remains unaffected.
- `window_state.json` persists and reloads expected geometry.

## Phase 7: Complete CLI Final-Product Coverage

Target:

Every supported CLI command must have tests for success, failure, and JSON
where applicable.

Commands to cover:

```text
ditox
ditox watch --status
ditox watch --stop
ditox watch --json
ditox list --limit N --json --favorites
ditox get <target> --json
ditox search <query> --limit N --json
ditox copy <target>
ditox delete <target>
ditox favorite <target>
ditox count
ditox clear --confirm
ditox stats --json
ditox collection list/create/delete/rename/add/remove/show
ditox tag list/create/delete/add/remove/show or the current tag command shape
ditox rules list/add/show/delete/enable/disable/reorder
ditox transform --list
ditox transform <target> <transform>
ditox open <target> <action>
ditox save
ditox repair --dry-run --fix-hashes
ditox sync discover/pull/peers/log/trust/reject/untrust/auto-send
```

Implementation:

- Migrate existing root CLI tests into `ditox-tui/tests`.
- Add missing cases for newer commands.
- Make helper fixture create:
  - temp `XDG_DATA_HOME`,
  - temp `XDG_CONFIG_HOME`,
  - temp `XDG_RUNTIME_DIR`,
  - initialized DB,
  - optional seeded entries, collections, tags, rules, image blobs.
- All CLI tests must avoid the real clipboard except tests specifically marked
  ignored/manual/live.
- For commands that must touch the system clipboard, abstract or isolate:
  - Prefer DB-only command tests where possible.
  - For live clipboard tests, mark `#[ignore]` and document the exact command
    to run.

Acceptance criteria:

- `cargo test -p ditox-tui --tests --locked` covers all CLI commands.
- JSON outputs parse as JSON in tests.
- Failure modes assert useful stderr and nonzero exit status.

## Phase 8: Linux Automated E2E Infrastructure

Target:

Automated tests prove the app works in realistic Linux desktop sessions.

Preferred CI matrix:

- Nix build/test job.
- Hyprland nested compositor job.
- Sway nested compositor job.
- Optional GNOME/KDE compatibility jobs if stable in CI.

Implementation approach:

1. Add E2E scripts under:

   ```text
   tests/e2e/
   ```

   Suggested scripts:

   ```text
   tests/e2e/hyprland-smoke.sh
   tests/e2e/sway-smoke.sh
   tests/e2e/gnome-degraded-smoke.sh
   tests/e2e/common.sh
   ```

2. Each E2E script should:

   - Create temp data/config/runtime dirs.
   - Build or use existing debug/release binaries.
   - Start nested compositor.
   - Start `ditox watch` or seed DB directly depending on scenario.
   - Start a simple test target app with a text input.
   - Run `ditox-gui` summon path.
   - Trigger selection through IPC/keyboard/mouse automation.
   - Assert pasted content appears in the test app.
   - Assert no duplicate self-recapture was created.
   - Dump logs and screenshots on failure.

3. Tooling choices:

   Use the most reliable available Linux automation stack:

   - compositor command tools: `hyprctl`, `swaymsg`;
   - input tools: `wtype`, `ydotool` only where expected;
   - screenshot tools: `grim`, compositor-specific screenshots, or
     browser/desktop automation if available;
   - test target app: a small purpose-built GTK/iced text window, `foot`,
     `alacritty`, or another deterministic app available in CI.

4. Add ignored live Rust tests only for developer diagnostics, not as the
   primary final gate.

Acceptance criteria:

- Hyprland smoke runs in CI and fails on broken paste-back.
- Sway smoke runs in CI and fails on broken paste-back.
- Failure artifacts include logs and screenshots.

## Phase 9: Packaging And Runtime Validation

Target:

The Linux packaged artifact works, not just debug builds.

Implementation:

1. Keep Nix package as required gate.

   ```sh
   nix build --no-link .#default
   ```

2. Add package smoke tests:

   - Run built `ditox --version`.
   - Run built `ditox-gui --version`.
   - Run DB-only CLI commands with temp dirs.
   - Validate wrapper `LD_LIBRARY_PATH` includes required GUI runtime libs.

3. Address Cachix trust noise:

   The audit saw many warnings like:

   ```text
   ignoring substitute for ... from 'https://ditox.cachix.org',
   as it's not signed by any of the keys in 'trusted-public-keys'
   ```

   Choose one:

   - Document the required `ditox.cachix.org` public key and setup command.
   - Or remove `ditox.cachix.org` from default substituters if it is not meant
     to be trusted by normal contributors.

4. Add release checklist updates:

   - Include `nix develop` test commands.
   - Include E2E compositor scripts.
   - Include package smoke commands.

Acceptance criteria:

- Nix package builds.
- Packaged binaries run basic CLI smoke tests.
- Build documentation does not lead contributors into the bare-shell
  `pkg-config`/GLib failure without explanation.

## Phase 10: Documentation Accuracy Pass

Target:

Docs match the code and release scope exactly.

Files to review and update:

```text
README.md
docs/ROADMAP.md
docs/features.md
docs/shortcuts.md
docs/RELEASING.md
docs/notes/master-plan-v1.md
docs/notes/linux-gui-architecture.md
docs/notes/ui-replication.md
docs/notes/hyprland-setup.md
docs/notes/adr/0001-layer-shell-strategy.md
packaging/linux/*
nix/module.nix
```

Required content:

- Installation through Nix.
- Non-Nix dependency list.
- First-run setup.
- Hyprland keybinding setup.
- Sway keybinding setup.
- GNOME limitations.
- KDE limitations.
- Data/config paths.
- How to run watcher.
- How to run GUI daemon.
- How to bind summon command.
- How to troubleshoot paste-back.
- How to collect logs with `RUST_LOG`.
- Exact final release test commands.

Acceptance criteria:

- No docs claim Windows/macOS final support for this release.
- No docs describe one-shot GUI behavior if the current code is daemon/IPC.
- No docs mention features that are stubs or no-ops.

## Phase 11: CI Workflow

Target:

CI enforces the final release gate.

Required jobs:

1. Format:

   ```sh
   cargo fmt --all -- --check
   ```

2. Clippy:

   ```sh
   nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings
   ```

3. Tests:

   ```sh
   nix develop --command cargo test --workspace --locked
   ```

4. Orphan test guard:

   ```sh
   scripts/check-no-root-tests.sh
   ```

5. Nix package:

   ```sh
   nix build --no-link .#default
   ```

6. Hyprland E2E.

7. Sway E2E.

8. Optional non-blocking compatibility jobs:

   - KDE Wayland.
   - GNOME Wayland degraded mode.

Acceptance criteria:

- Branch cannot pass CI with orphan tests.
- Branch cannot pass CI with no-op advertised behavior.
- Branch cannot pass CI if package build fails.
- Final release tag must pass the full Linux gate.

## Final Acceptance Checklist

The work is complete only when all items below are true:

- [ ] Root test files are migrated into package-owned test targets.
- [ ] `cargo test --workspace --locked` runs the migrated tests.
- [ ] Multi-format capture stores all allowed formats through normal watcher
      operation.
- [ ] Filter transform actions mutate captured text before DB insert.
- [ ] TUI command palette is implemented or removed from product surface.
- [ ] TUI stats overlay is implemented or removed from product surface.
- [ ] Hyprland GUI paste-back passes automated E2E.
- [ ] Sway GUI paste-back passes automated E2E.
- [ ] GNOME degraded behavior is tested or manually scripted and documented.
- [ ] Layer-shell drag works on Hyprland and Sway.
- [ ] CLI integration tests cover every supported command.
- [ ] JSON CLI outputs are parsed and asserted in tests.
- [ ] Nix package builds.
- [ ] Packaged binaries pass smoke tests.
- [ ] Docs describe Linux final-product scope accurately.
- [ ] Windows/macOS are clearly deferred.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `nix develop --command cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- [ ] `nix develop --command cargo test --workspace --locked` passes.
- [ ] `nix build --no-link .#default` passes.
- [ ] `git status --short` is clean before final handoff.

## Suggested Execution Order

Use this order to minimize churn:

1. Create tracking task and reconcile docs scope.
2. Move orphan root tests into package-owned integration tests.
3. Add CI guard for root tests.
4. Complete multi-format watcher persistence.
5. Complete transform rule behavior.
6. Implement or remove TUI no-op actions.
7. Harden Linux foreground and paste-back paths.
8. Implement layer-shell drag.
9. Build CLI coverage to final-product level.
10. Add automated Hyprland and Sway E2E.
11. Add package smoke tests.
12. Final docs pass.
13. Run the full release gate.

Do not mark the final-product task complete until the final acceptance
checklist passes.
