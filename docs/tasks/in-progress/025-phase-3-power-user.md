# Task: Phase 3 — Power-user features

> **Status:** in-progress (7/8 sub-tasks done — 3.1, 3.2, 3.4, 3.5 (Linux), 3.6, 3.7, 3.8)
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

### 2026-04-26 — sub-task 3.6 landed

**3.6 — Search-mode prefixes.**

`ditox-core/src/search.rs`:
- New `SearchScope` enum: `Default`/`Plain`/`Html`/`Rtf`/`Notes`/`FullText`.
  Each non-default scope has a single-letter prefix code:
  `p`/`h`/`r`/`q`/`f`. `format_name()` returns the canonical MIME for
  the format-restricted scopes (`Plain` → `text/plain;charset=utf-8`,
  etc.).
- `ParsedQuery { scope, query }` returned by `parse(input)`.
- `parse(input)` recognises strict `'/' SCOPE_LETTER ' ' BODY`. Anything
  else (no slash, lone `/`, unknown letter, leading whitespace, double
  slash) falls through to `Default` with the literal input — fail-soft
  so unfortunate clip contents starting with `/` aren't silently
  re-routed.
- `dispatch(db, parsed, limit, filter, collection_id)` helper routes
  to the right `Database` method based on scope. `Default`/`FullText`
  honour the tab/collection filter via `search_entries_filtered`;
  `Plain`/`Html`/`Rtf` use `search_entries_in_format` (filter
  intentionally ignored — power-user mode); `Notes` uses
  `search_notes_only`.
- 25 unit tests covering: prefix-char round-trip; case-insensitive
  parse; empty body (`"/p "` → `Plain { query: "" }`); fail-soft
  paths (`"/pfoo"`, `"/p"` alone, `"/x foo"`, `"/3 foo"`, `"//foo"`,
  `" /p hello"`); `format_name` correctness; `Display` impl.

`ditox-core/src/app.rs::App::load_search_results`:
- Calls `search::parse` first; routes the actual SQL to
  `search_entries` / `search_entries_in_format` / `search_notes_only`
  per scope. After loading, briefly swaps `search_query` to the
  stripped post-prefix query so `apply_fuzzy_filter` /
  `apply_regex_filter` highlight only the actual search term, not
  the prefix; restores afterward so the search bar still shows the
  user's input verbatim.

`ditox-gui/src/app.rs`:
- Both search call sites (the live `Message::SearchTriggered`
  tokio-spawn path and the `refresh_entries` path) now call
  `ditox_core::search::parse` then `dispatch(...)` — single
  routing helper, identical behaviour across paths.

**Default vs FullText semantics.** Both currently route to the same
DB method (`search_entries_filtered` for the GUI, `search_entries`
for the TUI's App). `FullText` is reserved as a future-extension
point in case `Default` is ever narrowed to text+notes; documented
in the doc comment.

**Tab-filter limitation.** The format-restricted scopes ignore the
tab filter today — `/h hello` on the Images tab returns
HTML-format hits across all entry types, not zero. Documented in
the module-level doc comment; Phase 3 polish or Phase 4 may
revisit if the combination proves useful in practice.

**Workspace test count after this session: 347 tests** (was 322;
+25 search.rs unit tests). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 3.7 landed (data-model + key MVP)

**3.7 — Per-resolution window state.**

Migrated `window_state.json` from a single flat `{x, y, width,
height}` to a multi-key map keyed by monitor resolution. The
in-memory `WindowState` shape stays unchanged so the 11 read sites
in `DitoxApp` (`self.window_state.{x, y, width, height}`)
continue to work without churn — only the persistence layer
evolved.

`ditox-gui/src/app.rs`:
- New `WindowStateFile { version: 2, geometries: HashMap<String,
  PersistedGeometry>, last_resolution_key: Option<String> }`
  on-disk shape; matches the spec from
  `docs/notes/ui-replication.md::A10`.
- `PersistedGeometry { x, y, width, height, last_used: String }`
  carries an ISO-8601 timestamp for the LRU fallback path.
- `parse_persisted_shape(content)` probes the JSON via
  `serde_json::Value`: `geometries` key → new format;
  `x`+`y`+`width`+`height` numeric keys → legacy → migrate under
  `LEGACY_RESOLUTION_KEY = "legacy"`. The probe is necessary
  because every `WindowStateFile` field is `#[serde(default)]`
  so a legacy file would otherwise parse as an empty new-format
  file and silently lose the user's saved geometry. (The first
  iteration of this code shipped with that bug; caught by the
  live smoke test on Hyprland — log showed `Loaded ... at (100,
  100)` instead of the saved `(150, 250)`. Fixed before commit.)
- `WindowState::load`:
  1. Parse via `parse_persisted_shape`.
  2. Pick the geometry: current monitor key → `last_resolution_key`
     → first available → default.
  3. Validate (`x`, `y` not absurd; size at least `MIN_WINDOW_SIZE`).
- `WindowState::save`: read-modify-write so saving under one
  resolution doesn't drop entries for other monitors.
- Resolution key from
  `iced::window::Position::SpecificWith` callback: captures
  `monitor_size` into a `OnceLock<String>` formatted as
  `"<width>x<height>"`. Idempotent — first launch wins.
  Multi-monitor moves within a session are not yet detected;
  Phase 4's daemon-mode rework will replace this with per-event
  monitor tracking. Documented limitation in the doc comment.

Phase 4 will:
- Extend the resolution key with the monitor model + serial
  (e.g. `"1920x1080@DP-1:LG_DISPLAY_ABC"`) per the A10 spec —
  format is forward-compat (just longer keys).
- Replace the `OnceLock<String>` with a per-event monitor tracker
  so multi-monitor moves Save/Load under the right key.

`ditox-gui/Cargo.toml`: pull in workspace `chrono` for the
`last_used` timestamp.

12 unit tests in a new `window_state_tests` module covering:
legacy-shape parser; new-shape parser; parse_persisted_shape
detection (legacy migration / new-format / unknown / partial /
corrupt JSON); make_resolution_key truncation; PersistedGeometry
↔ WindowState round-trip; LEGACY_RESOLUTION_KEY constant
stability; current_monitor_key fallthrough.

**Live verification on Hyprland 2026-04-26:** wrote a legacy
`{"x":150.0,"y":250.0,"width":420.0,"height":520.0}` file. Log
showed:

```
INFO ditox_gui::app: migrating legacy single-geometry window_state.json to multi-key format
INFO ditox_gui::app: Loaded window state: 420x520 at (150, 250)
```

— migration banner emitted, geometry correctly picked up from the
legacy entry. (The on-disk file isn't rewritten until a save event
fires; non-destructive migration on read keeps things safe.)

**Workspace test count after this session: 359 tests** (was 347;
+12 window_state_tests). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 3.8 landed

**3.8 — Translate / web-search URL templates.**

`ditox-core/src/url_template.rs`:
- `ActionsConfig { translate_url, web_search_url }` with the
  spec defaults: `https://translate.google.com/?text={q}` and
  `https://duckduckgo.com/?q={q}`.
- `substitute(template, query)` — replaces every `{q}` with the
  URL-encoded query; passes other tokens (including lone `{`)
  through verbatim.
- `url_encode(input)` — RFC 3986 unreserved-set encoder
  (`[A-Za-z0-9-_.~]` literal, everything else `%XX` uppercase
  hex). ~25 LoC, no new workspace dep (avoided `urlencoding` /
  `percent-encoding` for dependency hygiene).
- `UrlAction { Translate, WebSearch }` enum with
  `from_name(&str)` parser accepting canonical names
  (`translate`/`search`) plus synonyms (`tr`/`trans`/`web`/
  `websearch`/`web-search`).
- `open_in_browser(url)` — cross-platform shell-out:
  - Linux:   `xdg-open <url>`
  - macOS:   `open <url>`
  - Windows: `cmd /C start "" <url>` (the empty `""` is the
    mandatory window-title arg for `start`)

  Errors surface the launcher's stderr — typical failure modes
  are "no default browser registered" or "xdg-open not on PATH".

`ditox-core/src/config.rs`: `Config.actions: ActionsConfig`
nested under the `[actions]` TOML section.

`ditox-tui/src/cli.rs`: new `Open { target, action,
print_only }` subcommand.

`ditox-tui/src/main.rs::cmd_open`:
- Resolves the entry via the existing `resolve_target` helper
  (1-based index or UUID).
- Refuses image entries (URL templates need a text query —
  pasting a bare SHA-256 into Google Translate is useless).
- Looks up the action via `UrlAction::from_name`; exit code 2
  on unknown action.
- `--print-only` (`-p`) dumps the resolved URL to stdout
  instead of launching the browser. Useful for piping into a
  custom browser script or sanity-checking templates.
- Otherwise calls `open_in_browser` and propagates errors.

15 unit tests in url_template.rs covering: url_encode unreserved
pass-through / space / specials / unicode (multi-byte UTF-8 →
multi-byte percent encoding) / empty / uppercase hex (RFC 3986
§2.1); substitute happy-path / encoding / multi-occurrence /
no-placeholder / lone braces; ActionsConfig defaults and
helpers; UrlAction resolution and from_name with all synonyms.

**Live verification on Hyprland 2026-04-26 with `--print-only`:**

```
$ ditox open 1 translate -p
https://translate.google.com/?text=Diablo%C2%AE%20IV%3A%20Lord%20of%20Hatred%E2%84%A2%20-%20Ultimate%20Edition

$ ditox open 1 search -p
https://duckduckgo.com/?q=Diablo%C2%AE%20IV%3A%20Lord%20of%20Hatred%E2%84%A2%20-%20Ultimate%20Edition

$ ditox open 1 trans -p          # synonym
https://translate.google.com/?text=...

$ ditox open 1 web-search -p     # synonym
https://duckduckgo.com/?q=...

$ ditox open 1 garbage -p
ditox open: unknown action 'garbage'. Valid: translate, search (synonyms: tr, trans, web, websearch, web-search).
$ echo $?
2
```

Unicode round-trip works: ™ → `%E2%84%A2`, ® → `%C2%AE`, space →
`%20`, `:` → `%3A` — all uppercase hex per RFC 3986 §2.1.

GUI integration deferred — iced 0.14 has no built-in context
menu support, and adding right-click handling is its own
refactor. Phase 4's Ditto-UX-replication task (026) will
introduce the broader context-menu UI; 3.8's data model + CLI
surface is the foundation it'll build on.

**Workspace test count after this session: 374 tests** (was 359;
+15 url_template tests). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 3.1 landed (full Tier 1+2+3, 21 transforms)

**3.1 — Special paste / transforms.**

Shipped 21 of the spec's 22 transforms (image-stitching deferred —
needs multi-entry-selection UX from Phase 4). All text transforms
operate via a clean `Transform` trait and a registry-based lookup.

`ditox-core/src/transforms/` directory module:

- `mod.rs`: `Transform` trait + `registry()` + `get(id)` lookup.
  6 unit tests on registry hygiene (unique IDs, kebab-case form,
  count, lookup case-insensitivity).
- `case.rs`: `UpperCase`, `LowerCase`, `TitleCase`, `SentenceCase`,
  `InvertCase`, `CamelCase`, `PascalCase`, `SnakeCase`, `KebabCase`.
  Tokeniser splits on whitespace, ASCII punctuation (excluding
  `'`), lowercase→uppercase transitions, and the
  `acronym→camel`-followed-by-lowercase boundary so `HTTPRequest`
  splits as `["http", "request"]` rather than `["h","t","t","p","request"]`.
  Tracks original case in a `Vec<char>` accumulator so the
  boundary-detection still has access to the un-lowered chars.
  19 unit tests including edge cases (acronyms, apostrophes,
  multi-separator runs, empty input).
- `whitespace.rs`: `TrimWhitespace`, `CollapseWhitespace`,
  `RemoveLineFeeds`, `AddLineFeed`. The spec's
  parameterised `AddLineFeeds(n)` was reduced to a fixed
  `AddLineFeed` (n=1, idempotent on input already ending in
  `\n`); chaining with `printf '\n\n'` etc. gives n>1 from the
  shell. Collapse leaves newlines verbatim, only coalescing ASCII
  space + tab runs into a single space (matches `tr -s ' '`).
  10 unit tests.
- `string.rs`: `PlainTextOnly` (placeholder identity for
  single-format text entries; activates with Phase 4's
  multi-format Entry), `Slugify` (NFKD + drop combining marks +
  custom symbol map for ©/®/™/°/×/÷/en-em-dash/smart-quotes/…/→/←;
  `unicode-normalization` workspace dep), `AsciiOnly`,
  `PosixifyPaths`, `Typoglycemia` (Fisher-Yates + LCG seeded from
  system clock, preserves first/last letters, ≥4-char-word gate).
  20 unit tests including symbol-map round-trip, smart-quote
  handling, idempotent slug, length preservation in typoglycemia.
- `meta.rs`: `PrependDateTime`, `AppendDateTime` (chrono `%Y-%m-%d
  %H:%M:%S` local time), `InsertGuid` (UUIDv4 via existing `uuid`
  workspace dep). 5 unit tests including UUID parseability and
  uniqueness.

**Slugify implementation hygiene:** the spec is explicit that we
must NOT read or copy Ditto's `Slugify.h` table. The custom symbol
map ships from-scratch from RFC 3986 + commonly-encountered glyphs
the user noted in their daily clipboard. Combining-mark detection
uses hard-coded ranges (Mn/Mc blocks from BMP) rather than pulling
in `unicode-properties` for one function.

`ditox-core/Cargo.toml`: `unicode-normalization = "0.1.24"`
workspace dep added.

`ditox-tui/src/cli.rs::Commands::Transform`:

- `--list` flag (mutually exclusive with `target`/`transform`)
  prints the registry as a 2-line-per-transform table or as JSON
  with `--json`.
- Otherwise: `ditox transform <target> <transform-id> [-p]`
  resolves the entry (1-based index or UUID), applies the named
  transform, and either copies the result to the clipboard or
  prints to stdout (`-p` / `--print-only`).
- Refuses image entries (transforms operate on text).
- Exit code 2 on unknown transform id with a helpful pointer
  to `--list`.

**Live verification on Hyprland 2026-04-26:**

```
$ ditox transform --list | head -8
ID                     NAME / DESCRIPTION
────────────────────────────────────────────────────────────────────────────────
plain-text-only        Plain text only
                         Drop non-text formats (HTML/RTF/etc.). Currently a no-op...
upper-case             UPPER CASE
                         Convert all letters to UPPERCASE.
lower-case             lower case
                         Convert all letters to lowercase.
... (21 total transforms)

$ ditox transform 1 upper-case -p
DIABLO® IV: LORD OF HATRED™ - ULTIMATE EDITION

$ ditox transform 1 slugify -p
diablo-r-iv-lord-of-hatredtm-ultimate-edition

$ ditox transform 1 kebab-case -p
diablo®-iv-lord-of-hatred™-ultimate-edition

$ ditox transform 1 typoglycemia -p
Dbioal® IV: Lrod of Headrt™ - Uiatmlte Eoiitdn

$ ditox transform 1 unknown-transform -p
ditox transform: unknown transform 'unknown-transform'. Run 'ditox transform --list' for available IDs.
$ echo $?
2
```

End-to-end verified: case conversions preserve Unicode marks (®, ™
pass through `to_uppercase`); slugify strips diacritics + applies
symbol map (® → r via NFKD, ™ → tm via custom map); typoglycemia
preserves first/last letters and overall length;
the `kebab-case` non-ASCII pass-through is intentional (only
boundaries are split, not character set restricted — use
`slugify` for ASCII-clean output).

**Image transforms (`ImagesHorizontal` / `ImagesVertical`)
deferred.** They take multiple entries as input rather than a
single text string and warrant their own trait when
multi-entry-selection UX lands in Phase 4. The Phase 1
`ImageStackAggregator` already implements the underlying
PNG-stack composition.

**Workspace test count after this session: 441 tests** (was 374;
+67 transform tests). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 3.5 landed (Linux logind; Windows deferred)

**3.5 — Suspend/resume awareness.**

ditox-core/src/power.rs:
- `PowerEvent { Suspending, Resumed }` enum.
- `PowerMonitor` trait — same shape as `ForegroundTracker` /
  `CaptureSource` (sync, `Send`, worker thread + `mpsc::Receiver`).
- `NoopPowerMonitor` for fallback (channel sender dropped
  immediately so subscribers see `Disconnected`).
- `build_default_monitor()` factory: tries logind on Linux,
  Noop on every other platform. Never errors.

ditox-core/src/power/logind.rs:
- `LogindPowerMonitor::new()` opens
  `zbus::blocking::Connection::system()` and pings the logind
  DBus peer (`org.freedesktop.login1` / `Ping`) to confirm it's
  registered. Returns `Err` on non-systemd Linux distros (Alpine,
  Devuan with sysvinit, Void runit) so `build_default_monitor`
  falls through to Noop.
- `subscribe()` spawns a `ditox-logind` worker thread that owns
  the connection and iterates the `PrepareForSleep` signal
  stream. The signal carries a single `bool`:
  `true` = about-to-suspend → `PowerEvent::Suspending`;
  `false` = just-resumed → `PowerEvent::Resumed`.
- `shutdown()` flips an `AtomicBool` flag and joins the worker.
  `Drop` impl calls `shutdown` for safety.
- `signal_loop` is tolerant of malformed payloads (logs +
  continues) and exits cleanly when the subscriber drops the rx.

`Cargo.toml`: `zbus = { version = "5", features = ["blocking-api"] }`
workspace dep, gated behind `cfg(unix)` in `ditox-core`. Pure-Rust
implementation — no system `libdbus` dep. `cargo build` on a
non-Linux Unix (BSDs, macOS) still pulls in zbus but
`build_default_monitor` returns Noop.

ditox-core/src/watcher.rs:
- `Watcher::run_loop` subscribes to a power monitor on entry,
  drains pending events on each poll iteration via
  `try_recv`, calls `handle_power_event` on each.
- `handle_power_event(Resumed)`: clears `last_hash` and
  re-initialises from the active capture source so the next poll
  either captures a genuinely-new clip or correctly skips an
  unchanged one. Without this, anything the user copied during
  sleep that hashes identically to the pre-sleep clipboard is
  silently skipped.
- `handle_power_event(Suspending)`: logged at info level today;
  Phase 4 may extend to flush pending DB writes pre-sleep.
- Power-monitor `shutdown()` called before `run_loop` returns so
  the worker thread doesn't outlive the watcher.

8 unit tests on `power::tests`: NoopPowerMonitor disconnected-channel
behaviour; idempotent shutdown; name; PowerEvent equality;
build_default_monitor no-panic; trait object safety. Plus 2 unit
tests on `power::logind::tests`: bus-less constructor sanity (test
runs even on CI hosts without logind — accepts either Ok or Err);
name returns `"logind"` independent of bus availability.

**Live verification deferred** because it requires literal machine
suspend (DBus signal only fires on `systemctl suspend` /
laptop-lid-close / `loginctl suspend`). The unit tests + clean
compile + trait-shape parity with verified `ForegroundTracker` /
`CaptureSource` impls give us high confidence; runtime
verification can happen the first time the user's machine sleeps
with `RUST_LOG=ditox_core::power=info ditox watch` running.

**Windows path deferred.** The Windows equivalent is
`WM_POWERBROADCAST` with `PBT_APMRESUMEAUTOMATIC`, which requires
either a message-only window with a wndproc or a service-style
power-notification registration via
`RegisterPowerSettingNotification`. Both are non-trivial Win32
integration that needs a Windows test machine. Spawning that
work is on the same path as task 033 (Windows paste-back) and
should land alongside or in a Phase-2-style follow-up task.

**Workspace test count after this session: 449 tests** (was 441;
+8 power module tests). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 3.4 landed

**3.4 — Filter rules.**

User-managed pattern rules evaluated at capture time. Matches drop /
transform / tag the clip; first matching rule wins. Schema bumped
v3 → v4. UI deferred to Phase 4 (a Settings page); CLI surface
ships now.

`ditox-core/src/db.rs`:
- `SCHEMA_VERSION` bumped 3 → 4.
- `migrate_to_v4()`: creates `filter_rules` table with columns
  `(id, name, pattern, pattern_kind, process_glob, action,
  enabled, position, created_at)` plus indexes on `position`
  and `(enabled, position)`.
- New CRUD methods: `add_filter_rule`, `list_filter_rules`,
  `get_filter_rule`, `delete_filter_rule`,
  `set_filter_rule_enabled`, `set_filter_rule_position`,
  `max_filter_rule_position`.
- `row_to_filter_rule()` translates the canonical TEXT
  `pattern_kind` and `action` columns back into typed
  `PatternKind` and `FilterAction` enums.

`ditox-core/src/filter.rs`:
- `PatternKind { Regex, Glob, Contains }`. Glob reuses the
  existing `crate::config::glob_match` helper; Contains is plain
  ASCII-case-insensitive substring match; Regex compiles via
  `regex::RegexBuilder` with `case_insensitive(true)` and a
  4 MiB `dfa_size_limit` cap (defence-in-depth against runaway
  matches).
- `FilterAction { Drop, Transform(String), Tag(String) }`.
  Canonical TEXT round-trip via `to_storage()` /
  `from_storage()`: `"drop"` / `"transform:<id>"` /
  `"tag:<name>"`.
- `FilterRule` struct with `new_now()` constructor (UUIDv4 +
  ISO-8601 timestamp).
- `FilterEngine`: compiles rules once, sorts by position, evaluates
  on demand. Disabled rules are dropped at compile time. Invalid
  regex patterns log `warn` and are skipped without breaking the
  engine.
- `MatchedRule<'a>` returned from `evaluate()` carries a borrow
  into the engine — callers don't pay clone cost on the hot path.
- 18 unit tests covering kind round-trip, action serialisation,
  rule construction, engine compile-time skip of disabled rules,
  first-match-wins by position, contains/glob/regex matching,
  invalid-regex graceful skip, process-scope restriction.

`ditox-core/src/watcher.rs`:
- `Watcher` gains a `filters: FilterEngine` field built at
  construction (`db.list_filter_rules()` then
  `FilterEngine::from_rules`). Construction failures log warn
  and start with an empty engine.
- New `Watcher::reload_filters()` for the future "edit rules
  while daemon runs" workflow (caller decides cadence).
- `process_clip` snapshots the foreground basename **once** and
  reuses it for both the existing `[capture.exclude]` check and
  the new filter-rule evaluation. Exclusion runs first; rules
  run after.
- For text clips, `FilterEngine::evaluate(text, basename)`
  returns the first matching rule. Action handling:
  - `Drop` → log + `return Ok(false)` without advancing
    `last_hash` (so identical content from a non-matching
    context still captures).
  - `Transform(id)` → log + capture as-is (full transform
    application requires more plumbing — Phase 3 follow-up).
  - `Tag(name)` → log + capture as-is (tags Phase 4b).

`ditox-tui/src/cli.rs::Commands::Rules`:
- `ditox rules list [--json]`
- `ditox rules add --name <n> --pattern <p> [--kind regex|glob|contains] [--process <glob>] [--action <a>]`
- `ditox rules show <id> [--json]`
- `ditox rules delete <id>`
- `ditox rules enable <id>` / `ditox rules disable <id>`
- `ditox rules reorder <id> <position>`

Add appends at `max_position + 1`. Show / disable / enable / delete
exit 1 on unknown id. Add exits 2 on bad `--kind` or `--action`.

4 new integration tests in
`ditox-core/tests/watcher_capture_integration.rs`:
- Watcher drops clip matching a filter rule.
- Non-matching clip passes through.
- First match wins by position.
- Process-glob scope restricts the rule to a specific
  foreground (with a `MockForegroundTracker`).

**Live verification on Hyprland 2026-04-26:**

```
$ ditox rules list
No filter rules configured.

$ ditox rules add --name "drop pwds" --pattern "(?i)password" --kind regex --action drop
Added rule 9335cfe2 "drop pwds" (drop) at position 0

$ ditox rules add --name "scoped drop" --pattern "secret" --kind contains --process "*KeePass*"
Added rule 9a6dfde2 "scoped drop" (drop) at position 1

$ ditox rules list
POS   ENABLED    KIND   ID                     PROCESS    ACTION         NAME / PATTERN
────────────────────────────────────────────────────────────────────────────────────────────────────
0     yes        regex  9335cfe2               -          drop           drop pwds
                                                                           (?i)password
1     yes        contains 9a6dfde2               *KeePass*  drop           scoped drop
                                                                           secret

$ ditox rules disable <full-uuid>
Rule <uuid> disabled
$ ditox rules show <full-uuid>
ID:           <uuid>
Name:         drop pwds
Pattern:      (?i)password
Pattern kind: regex
Enabled:      false
...
$ ditox rules reorder <full-uuid> 99
Rule <uuid> moved to position 99
$ ditox rules add --kind invalid --name x --pattern y
ditox rules add: unknown --kind 'invalid'. Valid: regex, glob, contains.
$ echo $?
2
```

Schema migration log: `applying schema migration v3 -> v4 (filter
rules)` fired once on first DB access; subsequent runs no-op.

**Workspace test count after this session: 471 tests** (was 449;
+18 filter unit tests + +4 watcher integration tests). All clippy
`-D warnings` + fmt clean.
