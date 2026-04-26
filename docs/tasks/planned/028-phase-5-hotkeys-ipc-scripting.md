# Task: Phase 5 — Hotkeys, IPC, Rhai scripting

> **Status:** planned
> **Priority:** medium
> **Phase:** 5 — Hotkeys, IPC, scripting
> **Created:** 2026-04-26
> **Estimated:** 4 weeks

## Description

Make ditox extensible: per-clip global hotkeys (Ditto's
"first-position" + buffer hotkeys), full IPC command set, and Rhai
embedded scripting for capture/paste hooks.

Schema bump: v5 → v6 (per-clip hotkey columns).

Decisions baked in:
- **Rhai scripting** (D3) — sandboxed, Rust-native, no FFI surface.
- **No native DLL add-ins** (D10).
- **Linux per-clip hotkeys via compositor binds**, written to a
  managed conf file (Hyprland) or documented manually
  (Sway/KDE/GNOME).

## Sub-tasks

### 5.1 Per-clip global hotkey

Schema v5 → v6:

```sql
ALTER TABLE entries ADD COLUMN global_hotkey TEXT;  -- e.g. "ctrl+alt+1"
ALTER TABLE entries ADD COLUMN local_hotkey  TEXT;  -- only valid in launcher
CREATE INDEX idx_entries_global_hotkey ON entries(global_hotkey)
    WHERE global_hotkey IS NOT NULL;
```

UI: side panel exposes "Bind global hotkey" button. Modal captures the
keystroke (Windows: `RegisterHotKey` test; Linux: ditox parses and
validates without registering).

**Windows registration:** existing `global-hotkey` crate. On daemon
boot, query all entries with `global_hotkey IS NOT NULL` and register
each.

**Hyprland:** generate `~/.config/hypr/conf.d/ditox-binds.conf`:

```hyprlang
# >>> ditox-managed (auto-generated, do not edit) >>>
bind = CTRL_ALT, 1, exec, ditox-gui paste-clip <uuid-of-entry-1>
bind = CTRL_ALT, 2, exec, ditox-gui paste-clip <uuid-of-entry-2>
# <<< end ditox-managed <<<
```

ditox writes this file every time a hotkey is added/removed/edited.
User must `source` it from `hyprland.conf` (the
`--install-hyprland-config` helper from Phase 4 already does this).

After write, ditox shells out `hyprctl reload` so changes take effect
without manual reload.

**Sway:** similar — write to `~/.config/sway/conf.d/ditox-binds.conf`,
reload via `swaymsg reload`. (Sway is best-effort per H1 but binds are
straightforward.)

**KDE / GNOME:** dbus-based hotkey APIs are out of scope for v1.0.
Document that per-clip hotkeys aren't available; user can rely on
in-launcher accelerators (5.2 EMIT command).

### 5.2 IPC protocol completion

All commands from Phase 4.2 plus:

```
PASTE-CLIP <id>     # write clip to clipboard, paste-back, no UI
EMIT <id>           # alias for PASTE-CLIP
COLLECTION-ADD <name> [color] [keybind]
TAG-ENTRY <entry-id> <tag-name>
SCRIPT-RUN <script-id> <entry-id>
SCRIPT-RELOAD       # re-read scripts dir
RELOAD-CONFIG       # re-read config.toml
GET-ENTRY <id>      # JSON dump of entry
LIST-ENTRIES [limit] [json]
```

Each command has a text and JSON response form. Default text;
`--json` flag on the CLI requests JSON.

### 5.3 Rhai scripting

`ditox-core/src/scripting.rs` embeds the `rhai` crate.

Hook points:

```rust
pub trait CaptureHook: Send + Sync {
    fn on_capture(&self, ctx: &mut CaptureContext) -> Decision;
}

pub trait PasteHook: Send + Sync {
    fn on_paste(&self, ctx: &mut PasteContext) -> Decision;
}

pub enum Decision {
    Continue,
    Drop,
    Replace(EntryFormats),
}
```

`CaptureContext` exposes:
- `clip.formats: Vec<RawFormat>` — read & mutate.
- `clip.text() -> String` — convenience.
- `clip.set_text(s: String)`.
- `clip.source_app -> Option<String>`.
- `clip.timestamp -> i64`.
- `clip.has_format(name: &str) -> bool`.
- `clip.remove_format(name: &str)`.

`PasteContext` adds:
- `target.process_basename`.
- `target.title`.

Rhai scripts live in `~/.config/ditox/scripts/{capture,paste}/*.rhai`.
Each file = one script. Convention: file name is the script ID.

Sandbox:
```rust
let mut engine = Engine::new();
engine.set_max_operations(1_000_000);
engine.set_max_string_size(64_000);
engine.set_max_call_levels(64);
engine.set_max_array_size(1024);
engine.set_max_map_size(1024);
engine.disable_symbol("eval");
```

No `import`, no file I/O, no network.

Example user script (`~/.config/ditox/scripts/capture/strip-tracking.rhai`):

```rhai
// Strip URL tracking parameters from copied URLs
let text = clip.text();
if text.starts_with("http") {
    let stripped = text.replace_all_regex("[?&](utm_[^=]+|fbclid)=[^&]*", "");
    clip.set_text(stripped);
}
```

Settings page lists all scripts with a per-script enable toggle.

### 5.4 Save-clipboard hotkey

CLI: `ditox save` — captures the current clipboard right now,
bypassing dedup. Useful when the user wants a "snapshot" even if the
content already exists in history.

Optional global hotkey (Phase 5.1): bind a key to `ditox save`.

Implementation: increments `entries.usage_count` if duplicate found
(distinguishing "force-save" from "first capture"). New flag column?
Probably overkill — just bump usage and mark `last_used = now`.

## Acceptance criteria

- [ ] Per-clip hotkey on Windows: `Ctrl+Alt+1` pastes the bound entry
      from anywhere.
- [ ] Per-clip hotkey on Hyprland: same after `hyprctl reload`.
- [ ] Rhai script with infinite loop terminated by sandbox limit.
- [ ] Rhai script can mutate clip text before insert.
- [ ] Rhai script can drop clip (return drop decision).
- [ ] `ditox save` captures even when current clipboard equals last
      entry.
- [ ] Removing a per-clip hotkey from the UI removes the bind from the
      managed conf file.

## Implementation Notes

For Hyprland bind regeneration: use a tempfile + atomic rename so a
crash mid-write doesn't leave a half-written file `source`d into the
compositor.

For `hyprctl reload`: spawn it; don't wait for output. If reload
fails, log a warning but don't error the API call (the user's binds
may stop working until manual reload, but the DB row is fine).

Rhai's `Engine` is `Sync`-safe via `Arc`; share one across all script
executions for performance.

Document a small library of "starter scripts" in `docs/notes/scripts/`:
- Strip URL tracking params.
- Lower-case all captures from a specific app.
- Drop captures shorter than 2 chars.
- Tag captures from `*Cargo.toml` source apps as `rust`.

## Risks

- **Risk:** A bad Rhai script crashes the daemon.
  Mitigation: sandbox limits enforced; script errors caught and logged
  per-execution; bad script flagged but doesn't disable others.
- **Risk:** `hyprctl reload` resets compositor state in a way the user
  doesn't expect (e.g. clears `windowrulev2`). Mitigation: only the
  managed file changes; reload is unavoidable but well-known.
- **Risk:** Per-clip hotkey storms (user binds 100 hotkeys).
  Mitigation: cap at 50 per-clip hotkeys; document `RegisterHotKey`'s
  ~16k atom limit on Windows.

## Work Log

### 2026-04-26
- Task file created (epic).
