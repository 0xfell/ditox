# Task: Phase 3 — Power-user features

> **Status:** in-progress (1/8 sub-tasks done — 3.2)
> **Priority:** medium
> **Phase:** 3 — Power-user features
> **Created:** 2026-04-26
> **Started:** 2026-04-26
> **Estimated:** 4 weeks

## Description

Port the long tail of useful Ditto features that aren't structural:
special-paste transforms, per-app capture exclusion, color swatches,
filter rules, suspend/resume awareness, search mode prefixes, and
per-resolution window state.

Schema bump: v3 → v4 (filter rules table).

## Sub-tasks

### 3.1 Special paste / transforms

`ditox-core/src/transforms.rs`:

```rust
pub trait Transform: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn apply(&self, formats: &mut Vec<FormatBlob>) -> Result<()>;
}
```

Implementations (each in its own file under `transforms/`):

| Transform | Effect |
|---|---|
| `PlainTextOnly` | Drop all formats except `text/plain` |
| `UpperCase` | Title-case-aware uppercase |
| `LowerCase` | Lowercase |
| `TitleCase` | Title-case |
| `SentenceCase` | First letter capitalised, rest lowered |
| `InvertCase` | Swap upper ↔ lower per char |
| `CamelCase` | `helloWorld` |
| `PascalCase` | `HelloWorld` |
| `SnakeCase` | `hello_world` |
| `KebabCase` | `hello-world` |
| `Slugify` | URL-safe; built on `unicode-normalization` + custom punctuation map (do NOT copy Ditto's `Slugify.h`) |
| `RemoveLineFeeds` | Replace `\n`/`\r\n` with single space |
| `AddLineFeeds(n)` | Append n newlines |
| `TrimWhitespace` | `.trim()` |
| `CollapseWhitespace` | Multi-space → single space |
| `PrependDateTime { format }` | Configurable strftime |
| `AppendDateTime { format }` | Same |
| `InsertGuid` | `uuid::Uuid::new_v4()` |
| `PosixifyPaths` | `\\` → `/` |
| `AsciiOnly` | Strip non-ASCII |
| `Typoglycemia` | Letter-shuffle inner chars (joke) |
| `ImagesHorizontal` | Phase 1 ImageStackAggregator (multi-clip) |
| `ImagesVertical` | Same, vertical |

Surface:
- TUI: `T` opens transform menu.
- GUI: side panel "Transforms" submenu.
- CLI: `ditox transform <id> <transform-id>`.

### 3.2 Per-app capture exclusion

```toml
[capture.exclude]
processes = [
    "*KeePass*",
    "*1Password*",
    "*Bitwarden*",
    "*pass*",          # password-store CLI
    "ydotoold",        # avoid feedback loops
]
```

Glob patterns evaluated against
`ForegroundTracker::snapshot().process_basename` at capture time. Match
→ skip the capture entirely (no DB row, no log).

Defaults ship with the password-manager list above. User can edit
freely.

### 3.3 Color swatch detection

In list rendering, regex-match (in priority order):

- `#[0-9a-fA-F]{8}` (RGBA8)
- `#[0-9a-fA-F]{6}` (RGB)
- `#[0-9a-fA-F]{3}` (short)
- `rgba?\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*(,\s*[\d.]+)?\s*\)`
- `hsla?\(\s*\d+(deg)?\s*,\s*\d+%\s*,\s*\d+%\s*(,\s*[\d.]+)?\s*\)`

Render a 12×12 pixel filled square before the entry text, using the
parsed color.

GUI: iced `Container` with custom `style`.
TUI: `Span::styled` with `bg` set to the parsed color.

### 3.4 Filter rules

Schema v3 → v4:

```sql
CREATE TABLE filter_rules (
    id           TEXT PRIMARY KEY,            -- uuid
    name         TEXT NOT NULL,
    pattern      TEXT NOT NULL,
    pattern_kind TEXT NOT NULL,              -- 'regex' | 'glob'
    process_glob TEXT,                        -- optional process scope
    action       TEXT NOT NULL,              -- 'drop' | 'transform:<id>' | 'tag:<tag-id>'
    enabled      INTEGER NOT NULL DEFAULT 1,
    position     INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL
);

CREATE INDEX idx_filter_rules_position ON filter_rules(position);
```

Rules evaluated in `position` order during capture pipeline, after
dedup. First match wins. Supported actions:

- `drop` — discard the clip (don't insert).
- `transform:<id>` — apply a Phase 3.1 transform before insert.
- `tag:<tag-id>` — apply a tag (Phase 4b) on insert.

UI: Settings → Filters page. List, create, edit, reorder, enable/disable.

### 3.5 Suspend/resume awareness

- **Linux:** subscribe to logind `org.freedesktop.login1.Manager`
  `PrepareForSleep` signal via `zbus`.
- **Windows:** register for `WM_POWERBROADCAST`, watch
  `PBT_APMRESUMEAUTOMATIC`.

On resume:
- Drop the `Database` connection in the `DbActor`.
- Reopen.
- Restart capture sources (subscribe again).

This handles VMware host-resume, suspend-after-long-idle, and similar
edge cases.

### 3.6 Search mode prefixes

Implementation lands in TUI + GUI search bars. Parser:

```
"/p hello"  → Search { mode: Plain,    query: "hello" }
"/h hello"  → Search { mode: Html,     query: "hello" }
"/r hello"  → Search { mode: Rtf,      query: "hello" }
"/q hello"  → Search { mode: Notes,    query: "hello" }
"/f hello"  → Search { mode: FullText, query: "hello" }
"hello"     → Search { mode: Default,  query: "hello" }
```

`Default` searches plain text + notes (current behaviour).

### 3.7 Per-resolution window state

Migrate `window_state.json` from a single `{x, y, w, h}` to a map keyed
by resolution + monitor identifier. See
`docs/notes/ui-replication.md::A10` for the schema.

### 3.8 Translate & web-search URL templates

Right-click on a text entry → context menu with:

- "Translate" → opens
  `Config.actions.translate_url.replace("{q}", url_encoded_text)`
  in default browser.
- "Search web" → opens
  `Config.actions.web_search_url.replace("{q}", ...)`.

Defaults:

```toml
[actions]
translate_url  = "https://translate.google.com/?text={q}"
web_search_url = "https://duckduckgo.com/?q={q}"
```

Cross-platform launch via `open` crate.

## Acceptance criteria

- [ ] Special paste menu surfaces all 22 transforms; each has unit tests.
- [ ] `[capture.exclude] processes = ["*KeePass*"]` actually skips
      KeePassXC clipboard activity.
- [ ] Color swatch appears for `#ff5500`, `rgb(255,85,0)`, `hsl(20,100%,50%)`.
- [ ] Filter rule "drop entries matching `(?i)password`" works in CI test.
- [ ] System suspend → resume → ditox keeps working (manual test on a
      laptop).
- [ ] `/p hello` returns only plain-text matches; `/h hello` only HTML.
- [ ] Window position remembered separately for 1080p vs 4K monitors.
- [ ] Right-click "Translate" opens browser to translate.google.com.

## Implementation Notes

For Slugify, do **NOT** read or copy Ditto's `Slugify.h`. Build our
own table from:
- `unicode-normalization` for NFD decomposition + diacritic removal.
- A small custom punctuation/symbol map (©→(c), ™→tm, …).

For Typoglycemia: simple shuffle of inner letters of each word ≥ 4
chars, preserving first and last. Seed with system time so consecutive
applications produce different results (intended).

`logind` integration via `zbus`; document optional dependency for
non-systemd Linux distros.

## Risks

- **Risk:** Transform on rich-format clip mangles formatting.
  Mitigation: each transform documents its applicable
  `format_kinds`; UI greys out inapplicable transforms.
- **Risk:** Filter rule regex backtracking attacks.
  Mitigation: use `regex` crate (no backtracking), enforce
  `RegexBuilder::size_limit` and `dfa_size_limit`.

## Work Log

### 2026-04-26 — task moved to in-progress; sub-task 3.2 landed

Phase 3 begins. Started with sub-task 3.2 because it's small,
high-value, and a natural closure on Phase 2's `ForegroundTracker`
work — the first real consumer of the abstraction.

**3.2 — Per-app capture exclusion.**

`ditox-core/src/config.rs`:
- New `CaptureExcludeConfig { processes: Vec<String> }` nested under
  `CaptureConfig.exclude`. Defaults ship with a conservative
  password-manager list: `*KeePass*`, `*1Password*`, `*Bitwarden*`,
  `ydotoold` (the last one to break feedback loops with our own
  paste-back synthesis).
- `CaptureExcludeConfig::excludes(basename) -> bool` walks the
  patterns and returns true on first glob match.
- New module-local `glob_match(pattern, input) -> bool` —
  ASCII-case-insensitive, supports `*` (zero-or-more chars) and `?`
  (exactly one char). Standard two-pointer + last-star backtrack
  algorithm; no recursion. ~25 LoC. No new workspace dep (would
  have been `wildmatch` or `globset`; both overkill for the
  use case).
- 9 glob unit tests + 7 `CaptureExcludeConfig` tests, including TOML
  round-trip and the "missing `[capture.exclude]` block must still
  pick up the password-manager defaults" security check.

`ditox-core/src/watcher.rs`:
- `Watcher` gains `foreground_tracker: Box<dyn ForegroundTracker>`.
- `Watcher::new` calls `crate::foreground::build_default_tracker()`
  to pick the per-platform tracker (Hyprland → hyprctl, others →
  Noop). Existing call sites (ditox-gui's in-process watcher,
  ditox-tui's daemon path) get exclusion for free.
- New `Watcher::with_sources_and_tracker(db, config, sources,
  tracker)` 4-arg constructor for tests that want to inject a
  `MockForegroundTracker`.
- Existing 3-arg `Watcher::with_sources(db, config, sources)`
  preserved as a thin wrapper that uses `NoopForegroundTracker` —
  back-compat for the 6 existing tests in
  `tests/watcher_capture_integration.rs`.
- `process_clip` snapshots the foreground BEFORE the sentinel/dedup
  checks. On glob match: log `debug!`, return `Ok(false)`, and
  intentionally do NOT advance `last_hash`. The latter is
  load-bearing — see the dedicated test
  `excluded_clip_does_not_advance_last_hash`. Rationale: a future
  clip with the same bytes from a NON-excluded app must still be
  capturable; if we'd advanced `last_hash`, the second poll would
  short-circuit on dedup before we even consulted the tracker.
- Tracker-error case (e.g. hyprctl binary missing while platform
  reported Hyprland): log `debug!` and fall through to capture.
  Fail-open is the right default — exclusion is an
  extra-conservative feature, not a correctness gate.

`ditox-core/tests/watcher_capture_integration.rs`:
- 5 new integration tests using `MockForegroundTracker`:
  1. excluded foreground → clip dropped, no DB row.
  2. allowed foreground → clip captured normally.
  3. tracker returns None (GNOME Wayland scenario) → fail-open
     captures.
  4. excluded clip does not advance `last_hash` (the load-bearing
     case described above).
  5. empty `processes` list short-circuits the whole foreground
     check.
- Test 4 introduces a small `MockForegroundTrackerHandle` adapter
  that wraps an `Arc<MockForegroundTracker>` so the test body and
  the watcher-owned `Box<dyn ForegroundTracker>` can share the
  same underlying mock and the test can flip the basename
  mid-test.

**Workspace test count after this session: 322 tests** (was 301;
+9 glob_tests + +7 capture_exclude_tests + +5 watcher integration
tests). All clippy `-D warnings` + fmt clean.

**Live smoke test (Hyprland):** `RUST_LOG=ditox=debug
./target/release/ditox-gui` from a brave-browser context starts
cleanly (no panic from the new `Watcher::new` → tracker init path).
Default exclusions don't fire because foreground is `brave-browser`,
which doesn't match the password-manager globs.
