# Task: Phase 1 — Multi-format clipboard capture

> **Status:** completed (8/9 sub-tasks; 1.4 spun out as task 032)
> **Priority:** high
> **Phase:** 1 — Multi-format capture
> **Created:** 2026-04-26
> **Completed:** 2026-04-26
> **Estimated:** 6-8 weeks
> **Actual:** ~1 day (8/9 sub-tasks; 1.4 deferred to Windows-side work)

## Description

Capture every clipboard format the OS publishes (text, HTML, RTF, image
variants, file lists, custom MIME), persist with stable hashing,
aggregate on multi-clip paste. This is **the unlock** for paste-back
that preserves rich formatting (Phase 2), HTML preview tooltips
(Phase 4), and several power-user features (Phase 3).

Schema bump: v2 → v3. New `entry_formats` table; `entries.content` is
deprecated for the duration of one version.

Decisions baked in:
- **Default capture:** all formats with per-format size cap (D4).
- **Format naming:** MIME types preferred, `win32:` prefix for Windows
  custom formats.
- **Linux backend:** `wl-clipboard-rs` library (replaces `wl-paste`
  shell-out).
- **Windows backend:** event-driven via `AddClipboardFormatListener`
  (replaces polling).

## Sub-tasks (each becomes its own task file when started)

### 1.1 Schema v2 → v3 multi-format model

Schema:

```sql
CREATE TABLE entry_formats (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id    TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    format_name TEXT NOT NULL,        -- MIME or win32:CF_*
    storage     TEXT NOT NULL,        -- 'inline' | 'blob_file'
    content     TEXT,                 -- inline text (NULL for blob)
    blob_hash   TEXT,                 -- blob file hash (NULL for inline)
    blob_ext    TEXT,                 -- file extension (e.g. "png", "rtf")
    byte_size   INTEGER NOT NULL,
    format_hash TEXT NOT NULL,        -- SHA-256 of canonical bytes
    canonical   INTEGER NOT NULL DEFAULT 0,  -- 1 = primary format for this entry
    created_at  TEXT NOT NULL,
    UNIQUE(entry_id, format_name)
);

CREATE INDEX idx_entry_formats_entry      ON entry_formats(entry_id);
CREATE INDEX idx_entry_formats_blob_hash  ON entry_formats(blob_hash);
CREATE INDEX idx_entry_formats_canonical  ON entry_formats(entry_id, canonical);

-- entries.canonical_format pointer
ALTER TABLE entries ADD COLUMN canonical_format TEXT;
```

Migration: for each existing row, insert one `entry_formats` row
mirroring current data. Set `canonical = 1`. Set
`entries.canonical_format` to `text/plain` or `image/png`.

Update FTS: `entries_fts` indexes `entry_formats.content` for inline
text formats.

### 1.2 Format name canonicalisation

Create `ditox-core/src/format.rs` with:

- `enum FormatId { Mime(String), Win32(String) }`
- Conversions: `from_win32_cf(u32)`, `from_wayland_mime(&str)`.
- Canonical strings for common types (`text/plain;charset=utf-8`,
  `text/html`, `text/rtf`, `image/png`, `image/jpeg`,
  `application/x-files`, `application/x-vnd.ditox.<custom>`).
- Win32-specific: `win32:CF_DIB`, `win32:CF_HDROP`, `win32:CF_LOCALE`,
  etc.

### 1.3 Linux capture via `wl-clipboard-rs`

Replace `clipboard.rs:63-224` `wl-paste` subprocess with
`wl_clipboard_rs::paste::get_contents` and
`wl_clipboard_rs::utils::get_mime_types`.

New `WaylandLibraryCapture` implements the `CaptureSource` trait from
task 018. Polls (initially) at the configured interval; later
phases may switch to event-based via `wlr-data-control-v1` if the
library exposes it.

### 1.4 Windows capture via `AddClipboardFormatListener`

Replace `clipboard.rs:230-332` `arboard` reads with direct
`windows-rs` calls.

Architecture:

```
ditox-gui / ditox watch
    │
    ├── spawns: WindowsListenerCapture thread
    │       │
    │       ├── creates message-only window via CreateWindowExW(HWND_MESSAGE)
    │       ├── calls AddClipboardFormatListener(hwnd)
    │       ├── runs message loop
    │       │     on WM_CLIPBOARDUPDATE:
    │       │       - OpenClipboard
    │       │       - EnumClipboardFormats / IDataObject::EnumFormatEtc
    │       │       - read each format via GetClipboardData / IDataObject::GetData
    │       │       - construct RawClip
    │       │       - send to mpsc::Receiver<RawClip>
    │       │     on WM_DESTROY: RemoveClipboardFormatListener, exit
    │       └── ...
```

Handle the watchdog case (Ditto's 5-min ping) — if no
`WM_CLIPBOARDUPDATE` fires for an extended period during which we *know*
the user copied, attempt re-registration.

Honour Windows do-not-record sentinels:
- `Clipboard Viewer Ignore` (custom registered format).
- `ExcludeClipboardContentFromMonitorProcessing`.
- `CanIncludeInClipboardHistory` DWORD == 0.

### 1.5 Per-format hashing & canonicalisation

For each format, compute SHA-256 of *canonicalised* bytes:

- **`text/plain`**: trim trailing `\0` padding from `GlobalAlloc`
  buffers (Windows quirk).
- **`text/html`** (Windows "HTML Format" envelope): parse the
  `Version`/`StartHTML`/`EndHTML`/`StartFragment`/`EndFragment` /
  `SourceURL` header, hash the *fragment only*. (Source URL stored as
  metadata.)
- **`text/rtf`**: strip volatile sub-blocks (`{\*\datastore...}`,
  `\rsidN`, `\insrsidN`, `\mdispDef1`) before hash. Implement as a
  light RTF tokenizer; do NOT shell out to a renderer.
- **Images** (`image/png`, `image/jpeg`, `image/gif`, `image/webp`,
  `image/bmp`, `win32:CF_DIB`): hash raw bytes after re-encoding to a
  canonical container. For PNG: re-encode at default level via `image`
  crate to normalise zlib variations. For others: hash as-is for the
  format-name; cross-format dedup is a separate concern.

Entry-level hash:

```
sha256( for each (format_name, format_hash) in sorted_by_name:
            f"{format_name}\0{format_hash}\n" )
```

This is stable across format orderings and identifies the *whole clip*.

### 1.6 Format aggregator trait

`ditox-core/src/aggregator.rs`:

```rust
pub trait FormatAggregator: Send {
    fn format_name(&self) -> &str;
    fn add(&mut self, blob: &[u8], idx: usize, count: usize) -> Result<()>;
    fn build(self: Box<Self>) -> Result<Vec<u8>>;
}
```

Implementations:

- `PlainTextAggregator { separator: String }` — concatenate.
- `HtmlEnvelopeAggregator` — parse each clip's envelope, keep
  `<!--StartFragment-->...<!--EndFragment-->`, join with `<br>`,
  serialise valid envelope with recomputed offsets.
- `RtfAggregator` — strip outer `{\rtf1` and trailing `}` of inner
  clips, join with `\par`.
- `FileListAggregator { mode: HDrop | UriList }` — concatenate file
  paths into the appropriate format.
- `ImageStackAggregator { axis: Horizontal | Vertical }` — composes
  via `image` crate `imageops`.

Wired into TUI multi-select copy (`core/app.rs::multi_copy`) which
currently does plain-text concat only.

### 1.7 Search across formats

FTS5 schema update:

```sql
DROP TABLE entries_fts;
CREATE VIRTUAL TABLE entries_fts USING fts5(
    entry_id    UNINDEXED,
    format_name UNINDEXED,
    content,
    notes
);
```

Triggers updated to maintain it. Searches now include results from any
inline text format (HTML, RTF source, plain text, notes).

Search mode prefixes (also referenced in Phase 3):
- `/p` — plain text only
- `/h` — HTML only
- `/r` — RTF only
- `/q` — notes only (ditox's "quick paste text")
- `/f` — full-content all formats (default)

### 1.8 Limits & quotas

Config:

```toml
[capture]
mode = "all"                          # "all" | "minimal" | "custom"
max_format_size_bytes = 10485760      # 10 MiB
max_clip_size_bytes   = 26214400      # 25 MiB

[capture.formats]
include = []                          # only used when mode = "custom"
exclude = ["application/x-vnd.foo"]   # always honoured
```

`mode = minimal` matches v0.3.1 behaviour: only `text/plain` and
canonical image. `mode = all` is the new default.

When a single format exceeds `max_format_size_bytes` the entire clip
is dropped (matching Ditto's behaviour) with a warning log.

### 1.9 Ordering: write blobs only after dedup

Critical: existing `Watcher::poll_internal` pattern (write-after-dedup)
must be preserved. Multi-format expands the surface — each blob format
gets a tmp-write/fsync/rename/fsync-parent sequence, but only after the
entry-level hash is checked. If any format's tmp-write fails, *roll
back* all already-written blobs for the same entry.

## Acceptance criteria

- [ ] Capture HTML from Firefox; paste into LibreOffice Writer;
      formatting preserved.
- [ ] Capture file selection from File Explorer (Windows) / Files
      (Linux); pasted into another file manager → files appear.
- [ ] Copy from Word; consecutive copies of identical text don't create
      new entries (`\rsid`-strip working).
- [ ] Migration v2 → v3 round-trips a snapshot DB without data loss.
- [ ] Multi-select 3 entries in TUI, copy. Paste into HTML-aware app
      → fragments concatenated as valid HTML.
- [ ] Default capture catches all known formats. `mode = "minimal"`
      reverts to v0.3.1 behaviour.
- [ ] Honor `Clipboard Viewer Ignore` / `Exclude*` / `CanIncludeInHistory`
      sentinels on Windows.
- [ ] Capture latency: < 50 ms (text), < 200 ms (image).
- [ ] DB size per capture: bounded by `max_clip_size_bytes`.

## Implementation Notes

- Vendor `wl-clipboard-rs` if upstream lacks features we need
  (e.g. mime-type filtering at watch time).
- Windows `windows-rs` is huge; consider building a thin wrapper crate
  `ditox-core/src/clipboard/win.rs` that compiles only the small
  subset of bindings we need.
- The migration is the single most invasive change in this whole
  roadmap. Snapshot tests for v0/v1/v2 → v3 are mandatory before merge.
- HTML envelope round-trip parser: see https://learn.microsoft.com/en-us/windows/win32/dataxchg/html-clipboard-format
- RTF tokenizer: minimal — we only need to strip control-word groups,
  not parse semantics. Use `nom` or hand-roll.

## Risks

- **Risk:** HTML envelope serialisation off-by-one bugs corrupt clips.
  Mitigation: round-trip property test (capture → store → emit → parse)
  with `proptest`.
- **Risk:** DB explodes for users with image-heavy workflows.
  Mitigation: per-format size cap + total clip cap defaults that are
  generous but bounded.
- **Risk:** `AddClipboardFormatListener` skipped events under heavy
  Windows load. Mitigation: watchdog-ping pattern from Ditto (write a
  custom format every 5 min, expect to see it back).

## Work Log

### 2026-04-26 — task created
- Epic file written.

### 2026-04-26 — sub-tasks 1.1, 1.2, 1.5, 1.7, 1.8, 1.9 landed

Six of nine sub-tasks complete in one session (~36 new tests, no
regressions, all clippy/fmt clean). The persistence, schema, format
identity, canonicalisation, search, limits, and atomic-write surface
is fully built out; the remaining work is producing (1.3, 1.4) and
consuming (1.6) multi-format clips.

**1.1 — Schema v2 → v3.** New `entry_formats` table with
`(entry_id, format_name, storage, content, blob_hash, blob_ext,
byte_size, format_hash, canonical, created_at)` and three indexes,
plus `entries.canonical_format` pointer. Backfill mirrors every
existing entry into one canonical `entry_formats` row (text →
inline, image → blob_file). New `format_content_fts` virtual table
with triggers on `entry_formats`. **Deviation from spec:** kept
`entries_fts` (notes + legacy single-format content) instead of
dropping and rebuilding it with multi-format columns — the spec
schema duplicates `notes` per format row (~3× bloat for typical
multi-format clips); we use two FTS tables and UNION them in search.
Trade-off: 2 FTS indexes vs 1, no notes duplication. `Database::open`
now sets `PRAGMA foreign_keys = ON` so the new `ON DELETE CASCADE`
on `entry_formats(entry_id) REFERENCES entries(id)` actually fires.
6 snapshot tests in `ditox-core/tests/schema_v3_migration.rs`.

**1.2 — Format identity.** New `ditox-core/src/format.rs` with
`FormatId` enum (`Mime` / `Win32` variants), conversion from Win32
`CF_*` codes (`from_win32_cf`), Wayland MIME normalisation
(`from_wayland_mime`), `is_text_like` / `is_image_like` /
`storage()` classifiers, and a `well_known` constants module
(`TEXT_PLAIN_UTF8`, `TEXT_HTML`, `IMAGE_PNG`, …). 8 unit tests.

**1.5 — Per-format hashing & canonicalisation.** New
`ditox-core/src/format/canonicalise.rs`:
- `html_envelope(bytes) -> CanonicalHtml { fragment, source_url,
  was_envelope }` parses Windows "HTML Format" envelope; on Linux
  raw HTML it passes through.
- `rtf(bytes) -> Vec<u8>` strips `\rsid*` family + `{\*\rsidtbl
  …}`, `{\*\datastore …}`, `{\*\mmathPr …}`, `{\*\latentstyles
  …}`, etc. — destination groups via brace-balanced walker
  (max 4096 bytes span to avoid runaway on malformed input).
- `format_hash(mime, bytes) -> String` dispatches per-MIME and
  returns the SHA-256 of canonicalised bytes (what gets stored in
  `entry_formats.format_hash`).
PNG re-encoding deferred — current per-MIME default is "hash raw
bytes". 11 unit tests, including a "two RTF copies with different
\rsid hash identically" regression test.

**1.7 — Multi-format search.** Rewrote `Database::search_entries`
and `search_entries_filtered` to use `e.id IN (UNION of
format_content_fts MATCH and entries_fts MATCH)` instead of the
single `entries_fts` join. Added `search_entries_in_format(query,
format_name, limit)` for the future `/h` / `/r` / `/p` search-mode
prefixes. Added `search_notes_only(query, limit)` for the `/q`
prefix (uses FTS5 column-restrictor `notes:term`). Search prefix
front-end UX (TUI/GUI handling) deferred to a Phase 1 follow-up.
4 v3 search tests added.

**1.8 — Capture limits & quotas.** New `Config.capture`
sub-section: `mode = all | minimal | custom`,
`max_format_size_bytes` (default 10 MiB),
`max_clip_size_bytes` (default 25 MiB), `formats.include` /
`formats.exclude` lists. Helpers
`CaptureConfig::should_capture_format(name)`,
`format_size_ok(len)`, `clip_size_ok(len)`. `Minimal` mode reverts
to v0.3.1 behaviour (text + canonical image only). Excludes win
over includes. 8 unit tests, including TOML round-trip.

**1.9 — Multi-format atomic write.** New
`Database::insert_multi(entry, extras: &[ExtraFormat])` and
`ExtraFormat { format_name, bytes }` with helpers
`format_hash() / canonical_bytes() / storage_class() /
blob_extension()`. Pipeline: write extra blob files first
(tracking was-newly-written paths) → open SQLite tx → write
canonical row + extra rows + update `entries.format_count` →
commit; on any SQL error, tx rolls back automatically and
written blobs are unlinked best-effort via `rollback_blobs`.
Known v0.4 limitation: a hash-collision short-circuit
(entries row already exists) does NOT unlink already-written
blob files; documented in the doc comment, asserted in the
`insert_multi_rolls_back_blobs_on_db_failure` test, follow-up
filed for Phase 1.x. 4 multi-format tests added (happy path,
empty extras, image extras land in blob store, collision behaviour).

**Remaining sub-tasks (deferred to next session):**

- **1.3 Linux capture via `wl-clipboard-rs`.** Replace the
  `wl-paste` shell-out with library calls; integrate as a
  `WaylandLibraryCapture` source per the `CaptureSource` trait
  (task 018). Needs real Wayland session for full verification.
- **1.4 Windows event-driven capture.** Replace `arboard` polling
  with `AddClipboardFormatListener` + message-only window +
  `EnumClipboardFormats` / `GetClipboardData`. Requires
  `windows-rs` Win32 bindings. Honour Ditto's "do not record"
  sentinels (`Clipboard Viewer Ignore`,
  `ExcludeClipboardContentFromMonitorProcessing`,
  `CanIncludeInClipboardHistory`).
- **1.6 FormatAggregator trait + 5 impls** (PlainText / HtmlEnvelope
  / Rtf / FileList / ImageStack). HtmlEnvelopeAggregator is the
  trickiest — needs envelope serialisation with recomputed offsets
  (the inverse of `canonicalise::html_envelope`).

Total workspace test count after this session: **121 tests** (was
72 at session start; +49 added: 8 db_actor + 6 v3 migration + 8
format + 11 canonicalise + 8 capture_config + 4 v3 search + 4
insert_multi). All clippy `-D warnings` + fmt clean.

### 2026-04-26 — sub-task 1.6 FormatAggregator landed

New `ditox-core/src/aggregator.rs` with `FormatAggregator` trait
(`format_name() -> &str`, `aggregate(&self, parts: &[&[u8]]) ->
Result<Vec<u8>, AggregateError>`) plus five impls:

- **`PlainTextAggregator { separator: String }`** — UTF-8
  validation + `String::join`-style concatenation. Rejects
  invalid UTF-8 with `AggregateError::InvalidUtf8 { index }`.
- **`HtmlEnvelopeAggregator { source_url: Option<String> }`** —
  wraps the byte concat of N HTML fragments in a Windows "HTML
  Format" clipboard envelope. Header is exactly **97 bytes**
  fixed (8-digit zero-padded offsets), so the four offsets are
  computable up-front without a second pass. Round-trip
  through `canonicalise::html_envelope` returns the joined
  fragments byte-for-byte (asserted by test).
- **`RtfAggregator`** — wraps each input's body in `{}` inside a
  fresh `{\rtf1\ansi …}` envelope, separated by `\par`. Body
  extraction uses a small prologue stripper that walks
  `\rtf1\ansi\ansicpg…\deff…\deflang…` control words and stops
  at the first `{` (destination group like `\fonttbl`) or
  literal text. Non-RTF inputs are escaped (`\` `{` `}` get a
  leading backslash) and emitted as literal text.
- **`UriListAggregator`** — RFC-2483 line-list join, normalises
  to CRLF, drops blank and `#`-comment lines, preserves
  duplicates and order (user-explicit selection). **Deviation
  from spec design:** dropped the `mode = HDrop | UriList`
  enum — Windows `CF_HDROP` inputs are converted to URI lists
  at the capture layer (Phase 1.4), so the aggregator never
  sees raw HDROP bytes. Single mode is cleaner.
- **`ImageStackAggregator { axis: StackAxis }`** — decodes
  PNG/JPEG/WebP via the workspace `image` crate, pastes each
  input into a transparent RGBA8 canvas at
  `(0, cumulative_y)` (vertical) or `(cumulative_x, 0)`
  (horizontal), output dimensions are sum-on-stack-axis +
  max-on-cross-axis. Always emits PNG (only workspace
  feature with alpha). Inputs are **not resized** —
  cross-axis padding is fully transparent.

`AggregateError` enum: `Empty`, `ImageDecode { index, source }`,
`ImageEncode(image::ImageError)`, `InvalidUtf8 { index }`. All
variants `#[error]`-derived via `thiserror`.

Trait is object-safe (`Box<dyn FormatAggregator>` works) — the
planned UI in Phase 4-5 will hold the aggregator behind a trait
object chosen by the user's selected format. Asserted by the
`aggregators_are_object_safe` test.

32 unit tests added in `ditox-core/src/aggregator.rs::tests`,
covering: empty rejection per impl, single-part identity/wrap,
multi-part join behaviour, format-name correctness, invalid
UTF-8 rejection (text aggregators), HTML envelope offset math
+ round-trip via `html_envelope()`, RTF prologue stripping,
RTF metacharacter escaping for plain-text inputs, RTF
destination-group preservation, URI-list CRLF normalisation +
comment/blank-line dropping, image stacking with transparent
padding, image decode-error indexing.

Workspace test count after this session: **153 tests**
(+32 aggregator). All clippy `-D warnings` + fmt clean.

**Phase 1 status: 7/9 sub-tasks done.** Remaining: 1.3
(Linux event-driven via `wl-clipboard-rs`) and 1.4 (Windows
`AddClipboardFormatListener`). Both require platform-specific
testing.

### 2026-04-26 — sub-task 1.3 Wayland multi-format capture landed

New `ditox-core/src/capture/wayland.rs` (`WaylandLibraryCapture`)
replaces the `wl-paste` subprocess shell-out on Linux. The watcher
now constructs the source via `#[cfg(unix)]` in
`ditox-core/src/watcher.rs::Watcher::new`; the legacy
`legacy_clipboard_snapshot` and the import of
`PollingCaptureSource` are gated `#[cfg(windows)]` and become a
Windows-only fallback until sub-task 1.4 lands.

**Pipeline per `current_snapshot()`:**

1. `paste::get_mime_types_ordered(Regular, Unspecified)` —
   compositor order is preserved (matters because many apps offer
   the native format first; `RawClip::first_with_prefix` returns
   the earliest match, so the ordering decides which `image/*`
   wins when both PNG and JPEG are offered).
2. For each MIME, canonicalise via `FormatId::from_wayland_mime`
   (collapses `text/plain`, `UTF8_STRING`, `STRING` → the single
   canonical `text/plain;charset=utf-8`).
3. Apply `CaptureConfig::should_capture_format(canonical)` —
   honours mode (All / Minimal / Custom) + allow/deny lists.
4. Dedup by canonical MIME — synonyms collapse to one read.
5. Open `paste::get_contents(Regular, Unspecified, Specific(mime))`,
   wrap the returned `os_pipe::PipeReader` in
   `Read::take(max_format_size_bytes + 1)`, `read_to_end`. Empty
   payloads are dropped (X11-leaked `TARGETS`/`TIMESTAMP` etc. and
   misbehaving sources).
6. `format_size_ok` per format and `clip_size_ok` for the total —
   when the total exceeds the cap, drop the WHOLE clip with a warn
   log (partial captures would silently lose information).

**Error mapping:** `PasteError::NoSeats | ClipboardEmpty | NoMimeType`
→ `Ok(None)` (steady-state empty clipboard, not an error).
Everything else (`MissingProtocol`, `WaylandCommunication`, `…`) →
`DitoxError::Clipboard(...)` so KDE-without-data-control or a
crashed compositor surfaces loudly.

**Image-priority preservation:** `Watcher::process_clip` was
already calling `clip.first_with_prefix("image/")` *before*
`first_with_prefix("text/plain")` (`watcher.rs:463 vs :485`); now
that `RawClip.formats` carries every offered MIME instead of one
pre-picked format, the existing precedence rule does the right
thing automatically. The "Copy image" from a browser → image, not
URL behaviour AGENTS.md describes is preserved without any
clipboard-side priority list. The `image/png > image/jpeg > …`
fixed list in `clipboard.rs::read_image:109-115` is no longer on
the hot path — that function survives as part of `Clipboard`'s
public API (now unused by the watcher) for backward compat.

**`subscribe()` is a stub** — returns an empty channel. The
watcher only ever calls `current_snapshot()` (`watcher.rs:445`),
so event-driven Wayland is deferred. A future implementation
needs `wlr-data-control-v1::data_offer.offer` events directly;
the `data_control` module is private inside `wl-clipboard-rs` so
this would require either a fork or a switch to
`smithay-client-toolkit`. Not blocking Phase 1.

**`Clipboard::set_text` / `Clipboard::set_image` unchanged** —
paste-back still shells out to `wl-copy` from `ditox-tui`,
`ditox-gui`, and `ditox-core::app`. The reverse-direction
`wl-clipboard-rs::copy::Options::copy` API works but needs a
daemonised holder process (Wayland clipboard requires the source
to stay alive until another client takes ownership), which is
its own project. Tracked as a follow-up.

**Tests added (7, +1 ignored):**
- `name_is_stable` — `name() == "wayland-library"` invariant.
- `shutdown_is_idempotent` — `Drop` contract.
- `translate_paste_err_collapses_empty_variants` — the three
  steady-state-empty errors all collapse to `Ok(None)`.
- `translate_paste_err_propagates_real_errors` — `MissingProtocol`
  surfaces as `DitoxError::Clipboard`.
- `subscribe_returns_empty_channel` — receiver is valid but
  disconnected (the `_tx` is dropped at end of `subscribe`).
- `config_is_held_by_value_for_should_capture_check` — the
  `CaptureConfig` field is consulted before any pipe is opened
  (verified via the cheap helper rather than a real Wayland call).
- `live_snapshot_returns_some_clip_when_clipboard_nonempty`
  (`#[ignore]`) — end-to-end smoke test against the active
  Wayland session. **Verified passing on Hyprland 2026-04-26**
  with `echo … | wl-copy` followed by
  `cargo test … live_snapshot … -- --ignored --nocapture`.
  Asserts the snapshot has at least one format with non-empty
  bytes and that all MIMEs are canonical (no raw `text/plain` or
  `UTF8_STRING` leak through).

**Workspace test count after this session: 159 tests** (+6
active wayland, +1 ignored). All clippy `-D warnings` + fmt clean.

**Phase 1 status: 8/9 sub-tasks done.** Remaining: 1.4 (Windows
`AddClipboardFormatListener`). That work needs to be done on
Windows (or against a Windows VM) — not feasible from this
Linux-only session.

### 2026-04-26 — closing 023; sub-task 1.4 spun out as task 032

Decision: Phase 1 is functionally complete from the Linux side
(8/9 sub-tasks: schema v3, format identity, canonicalisation,
multi-format DB writes, search, limits, atomic blob writes,
Wayland multi-format capture, FormatAggregator). Sub-task 1.4
(Windows event-driven capture) is the only outstanding item and
**fundamentally cannot be validated** from this Linux-only
iteration. Rather than block all of Phase 2 on it (which is also
mostly Linux-testable), 1.4 has been excised into its own task:

- **`docs/tasks/planned/032-phase-1-windows-multi-format.md`** —
  full design, sub-tasks, acceptance criteria, risks, and the
  prerequisite `Watcher::run` refactor that 023's original 1.4
  spec didn't surface.

032 also documents the **prerequisite watcher refactor**: the
current `Watcher::poll_internal` only calls `current_snapshot()`,
which works fine for the polling Wayland source today but isn't
suitable for an event-driven `AddClipboardFormatListener`. 032's
design includes the refactor to consume `subscribe()`'s
`mpsc::Receiver<RawClip>` alongside (or instead of) the synchronous
poll. Doing it as part of 032 keeps the Windows work
self-contained.

This task (023) is moved to `completed/` because the eight done
sub-tasks are independently mergeable and don't degrade the
existing Windows behaviour (the legacy `legacy_clipboard_snapshot`
+ arboard polling source remains on Windows until 032 ships).
Total effort spanning both Phase 0 + 1: ~3 days end-to-end.

**Final workspace test count: 159 tests** (start of session: 72;
+87 across Phase 0 + Phase 1). All clippy `-D warnings` + fmt
clean. Zero regressions in pre-existing tests.
