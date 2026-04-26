# Ditto vs Ditox — Architectural Comparison

> Date: 2026-04-26
> Source under analysis: `sabrogden/Ditto` `master` (release 3.25.113.0), shallow clone at `/tmp/Ditto-reference`.
> Ditox revision: `v0.3.1` working tree.

This is a **read-only architectural comparison**. Ditto is GPL-3.0; ditox is MIT. No
GPL code may be copied verbatim or as a close paraphrase into ditox. All
references to Ditto below are at the design / behaviour level and exist to
inform ditox's own roadmap.

---

## Table of contents

1. [Project shape & licensing](#1-project-shape--licensing)
2. [Clipboard capture & content handling](#2-clipboard-capture--content-handling)
3. [Storage layer](#3-storage-layer)
4. [UI, window management, rendering](#4-ui-window-management-rendering)
5. [Hotkeys, IPC, single-instance, sync](#5-hotkeys-ipc-single-instance-sync)
6. [Configuration & extras](#6-configuration--extras)
7. [Side-by-side feature matrix](#7-side-by-side-feature-matrix)
8. [Architectural patterns worth borrowing (clean-room)](#8-architectural-patterns-worth-borrowing-clean-room)
9. [Anti-patterns to avoid](#9-anti-patterns-to-avoid)
10. [Concrete suggestions for the ditox roadmap](#10-concrete-suggestions-for-the-ditox-roadmap)

---

## 1. Project shape & licensing

| | Ditto | Ditox |
|---|---|---|
| Language | C++ (MFC, ATL) — 287 source files in `src/` | Rust 2021 workspace |
| License | **GPL-3.0** | **MIT** |
| Platforms | Windows only (x64 + ARM64) | Linux (Wayland) + Windows |
| Lines of code (rough) | ~250 k incl. bundled libs (sqlite, chaiscript, qrencode, zlib, tinyxml) | ~12 k (5.8 k frontends + 3.7 k core + tests) |
| Crate / module count | Single MFC project + 4 sister projects (DittoSetup, EncryptDecrypt, focusdll, FocusHighlight, ICU_Loader, U3Stop, Addins) | 3 crates (`ditox-core`, `ditox-tui`, `ditox-gui`) |
| Bundled deps | SQLite Multiple Ciphers, ChaiScript, libqrencode, zlib, ICU (lazy), TinyXml, cpp-httplib (unused) | None — uses crates.io |
| Build system | MSBuild solution `CP_Main_10.sln` + Inno Setup | Cargo + Nix flake + Inno Setup (installer only) |
| Distribution | Installer EXE, Portable ZIP, MSIX/Store, Chocolatey, Winget | GitHub Releases (Linux gnu/musl/AppImage/arm64, Windows zip), Nix flake, Inno Setup installer |
| Code-signing | SignPath Foundation (free for OSS) | None yet |
| Age / maturity | ~20 years old, 6.2 k stars | < 1 year |

**License implication.** GPL-3.0 is strong copyleft. Ditox under MIT cannot
incorporate Ditto code (verbatim or close-derivative) without relicensing the
entire ditox tree. We can read freely, learn patterns, and reimplement
independently — that's what this document supports. We cannot copy.

---

## 2. Clipboard capture & content handling

### 2.1 Detection model

| Aspect | Ditto | Ditox |
|---|---|---|
| Strategy | **Event-based**: `AddClipboardFormatListener` (Vista+) with fallback to `SetClipboardViewer` chain. `src/ClipboardViewer.cpp:54-103` | **Polling**: every 250 ms (`watcher.rs:118-129`); same for the GUI subscription |
| Self-heal | 5-min watchdog timer; pings own clipboard with a custom `"Ditto Ping Format"` and reconnects if not seen | n/a |
| Debounce | `TIMER_DRAW_CLIPBOARD` collapses bursts; configurable inter-copy gap | None |
| Honors "do-not-record" hints | `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` | **No** — ditox captures everything |
| Per-app filter | Wildcard whitelist + blacklist on owner process name (`ValidActiveWnd`) | **None** |

**Why this matters.** Polling on Linux/Wayland is currently the only
practical option (Wayland's `wlr-data-control` would require a native socket
listener). On Windows, ditox could move to event-based capture via `arboard`
or a thin Win32 listener — that would cut idle CPU to zero and remove
visible latency. See [§10 #1](#10-concrete-suggestions-for-the-ditox-roadmap).

### 2.2 Supported formats

Ditto saves **every** clipboard format published, by name, not by numeric ID
(critical: registered IDs are unstable across reboots). Default seeded list
(`CP_Main.cpp:679-688`):

- `CF_TEXT`, `CF_UNICODETEXT`, `CF_HDROP`, `CF_DIB`, `Rich Text Format`,
  `HTML Format`, `PNG`, plus user-extensible types.

Ditox captures only **text** and **image** (`EntryType` is a closed enum,
`entry.rs:6-46`). Notable gaps:

- No HTML capture → loses formatted clips from browsers.
- No RTF capture → loses Word/Outlook clips.
- No `CF_HDROP` / file-list capture → can't act as a file-paste history.
- No custom MIME types.

### 2.3 The aggregator pattern (Ditto-only)

When the user multi-selects clips and pastes, Ditto re-emits **a single
merged clipboard payload** that still preserves rich format. Per-format
aggregators (`IClipAggregator`):

- `CF_Text` / `CF_UnicodeText` — concatenate with separator.
- `CF_HDROP` — rebuild a real file-drop list (Explorer treats the paste as
  files).
- `HTML Format` — parses Version/StartHTML/EndHTML/StartFragment/EndFragment,
  keeps fragments, **re-serialises** a valid envelope with recomputed
  offsets (`HTMLFormatAggregator.cpp:73-205`).
- `Rich Text Format` — strips leading `{\rtf1` / trailing `}` of inner
  clips, joins with `\par`.
- Image — composes images side-by-side or stacked into a single DIB.

Ditox has multi-select copy in the TUI (`core/app.rs:783-792`) but it
**concatenates plain text only and skips images**. There's no rich-format
aggregation because we don't store rich formats in the first place.

### 2.4 Deduplication

| | Ditto | Ditox |
|---|---|---|
| Hash | **CRC-32** of all format blobs | **SHA-256** of canonical bytes |
| Dedup site | `Main.CRC` indexed; collision = silent merge | `entries.hash UNIQUE` |
| Special handling | RTF: strips `\rsid`, `\insrsid`, `\datastore` before hashing (Word/Outlook randomise these). Text: clamps to `strlen` to avoid trailing garbage in the GlobalAlloc buffer | n/a — text is captured raw, image is canonicalised to PNG on Windows |
| On dup | Promotes existing row's `clipOrder` to top | Updates `last_used` via `db.touch()` |

CRC-32 is fragile — collisions silently merge distinct clips. SHA-256 is
strictly stronger. Ditto's RTF/text canonicalisation tricks would only matter
once ditox starts capturing those formats; the lesson is **canonicalise
before hashing** when a format embeds volatile metadata.

### 2.5 Paste injection

Ditto separates two stages:

1. **Stage A — load the OS clipboard** with all the saved formats via a
   custom `COleDataSource` (`OleClipSource.cpp`). For files from a remote
   peer it uses `DelayRenderData(CF_HDROP)` so the network fetch happens
   lazily inside `OnRenderGlobalData`.
2. **Stage B — synthesise Ctrl-V** to the foreground app via
   `ExternalWindowTracker::SendPaste()`. Pre-flight: walks all 256 VK codes
   sending KEY-UP for any still pressed (avoids stuck modifiers). The
   keystroke string is **per-target-app configurable** — vim gets `"+gp`,
   old `cmd.exe` gets `% {Delay100}ep`. UAC-elevated targets get a
   ShellExecute-with-`runas` helper child process.
3. Adds the `Clipboard Viewer Ignore` sentinel before ownership transfer so
   Ditto's own viewer doesn't loop on the paste it just performed.

Ditox doesn't synthesize Ctrl-V at all — clicking an entry just `wl-copy`s
or `arboard.set_text`s and then exits the GUI. The user pastes themselves.
This is simpler but loses the "pop the launcher → click → it pastes back
into the previous app" UX that's Ditto's killer feature. See
[§10 #3](#10-concrete-suggestions-for-the-ditox-roadmap).

### 2.6 Threading

Ditto's `CCopyThread` is a worker `CWinThread` that owns the hidden
clipboard-viewer window so the OS posts `WM_DRAWCLIPBOARD` /
`WM_CLIPBOARDUPDATE` to **its** queue, not the UI thread's. Capture is on
the viewer thread; DB writes happen on the UI thread; LRU bumps run on a
lowest-priority worker. Sync between worker config and live config goes
through a `CCriticalSection`.

Ditox: capture, dedup and DB write all happen on the same poll thread
(`watcher.rs::poll_once`). With a 250 ms cadence and SHA-256 over text,
this is fine; for images at 5+ MB it could become visible (we re-encode to
PNG on Windows on the poll thread). Worth offloading the encode to a
background worker once we capture multiple formats per clip.

### 2.7 Format preservation tricks Ditto does

Things a typical clipboard manager drops that Ditto goes out of its way to
keep — useful checklist for future ditox work:

1. **Format-by-name persistence** in the DB (`Data.strClipBoardFormat`),
   not by numeric `CLIPFORMAT` ID. Custom formats survive reboots and
   cross-machine import/export.
2. **HTML envelope round-trip.** Stores `SourceURL`, recomputes byte
   offsets on re-emit.
3. **Stable hashing for volatile-metadata formats** (RTF, see above).
4. **All formats kept**, not just one. Receiving app picks its preferred
   representation as if from a real Ctrl-V.
5. **`CF_HDROP` ↔ text conversion** for cross-app paste.
6. **PNG as a first-class format** alongside DIB (browser image copies
   often publish PNG without DIB).
7. **Microsoft "exclude from history" hints** are honoured.

---

## 3. Storage layer

### 3.1 Schema

Ditto's `Main` table is a single-table self-referential tree (clips and
groups share the schema; `bIsGroup=1` flips the type, `lParentID` builds
hierarchy). Per-format blobs live in `Data.ooData` as **inline BLOBs**.
Soft-delete via `MainDeletes` journal + idle-time blob purge.

Ditox separates concerns:

- `entries` row carries metadata + (for images) the bare hash.
- Image bytes live in **content-addressed external files**
  `images/{hash[..2]}/{hash}.{ext}`, never in SQLite.
- `pending_blob_prunes` queue is the durable "row deleted, file pending"
  bridge.
- FTS5 virtual table `entries_fts` for full-text search, kept in sync via
  triggers.

| Aspect | Ditto | Ditox |
|---|---|---|
| Image bytes | Inline BLOB (often **2x**: DIB + PNG of the same image) | External file, content-addressed, deduped at write |
| Text body | Inline BLOB per format | Inline `TEXT` column |
| Schema versioning | None — migration by `try { SELECT } catch { ALTER }` | Explicit `schema_meta.version` (currently `1`) |
| Migrations | `ValidDB` 200-line probe list (`DatabaseUtilities.cpp:287-489`) | Defensive `ALTER TABLE … ADD COLUMN .ok()` |
| FTS | None (search via SQL `LIKE` + ICU REGEXP extension `ICU_Loader.dll`) | **FTS5 + triggers** for incremental indexing |
| Indexes | 13 indexes incl. 3 composite for sticky/group ordering | 4 (`created_at`, `last_used`, `hash`, `collection_id`) |
| Transactions on hot path | **None** — every clip insert is its own implicit transaction (fsync per copy) | `db.touch()` and prune-queue in single transactions |
| WAL mode | Not enabled | Not enabled (default rollback journal) |
| Soft-delete journal | Yes — `MainDeletes` + idle-time purge throttled to N rows/pass | n/a — pruning is immediate (file, not blob) |
| Encryption at rest | **No** (despite linking sqlite3mc) | No |
| Compression | Only in `.dto` export files (zlib per blob) | None |

**Floating-point ordering.** Ditto stores `clipOrder` and `clipGroupOrder`
as `REAL`, so inserting between two siblings is `(a+b)/2` — no
re-numbering. But there's no rebalance step, so eventually precision is
exhausted. Ditox uses simple `ORDER BY created_at DESC` which sidesteps
the problem entirely.

### 3.2 Groups vs collections

Ditto's groups are recursive (group inside group inside group). Ditox
collections are flat (a single `collection_id` FK, nullable). For ditox
this is a known gap — collection support is in core/CLI but **not in the
GUI** and the tab list doesn't include user-defined collections (see
`baseline §3`).

If we ever want nested collections, Ditto's single-table self-reference is
the cheapest model. We'd lose FK integrity but gain unlimited depth.

### 3.3 Image storage

Ditox's content-addressed external store is **strictly better** than
Ditto's "everything inline" approach for clipboard managers:

- Built-in dedup at write time (`store_image_blob` checks dest existence
  first).
- Atomic writes (tmp → fsync → rename → fsync parent).
- No DB bloat — a 5 MB screenshot doesn't make `entries` 5 MB heavier per
  row.
- Self-healing via `pending_blob_prunes` queue + `ditox repair` walk.
- Hash-mismatch quarantine.

This is one area where ditox's design is unambiguously cleaner than
Ditto's. Worth keeping intact and documenting (`image-storage.md` already
covers it).

### 3.4 Encryption

Ditto links sqlite3mc (multi-cipher SQLite) but the application **never
calls `sqlite3_key`** — the on-disk DB is plaintext. The
`EncryptDecrypt/Encryption.cpp` static library (AES-256 CBC + SHA-256 KDF
with 100k iterations) is used **only for the LAN sync wire protocol**, not
at rest.

Ditox has no encryption. Both are vulnerable to local-disk reads. If
ditox ever wants encrypted-at-rest history, sqlite3mc + SQLCipher PRAGMA
keying is the obvious path — and we'd be doing something Ditto only
attempted on the wire.

### 3.5 Import / export

- **Ditto:** `.dto` files = standalone SQLite DBs with zlib-compressed
  format blobs and a `lOriginalSize` column. Three-line schema. Cheap to
  produce, opaque to non-Ditto tooling.
- **Ditox:** none. No `import` / `export` CLI command yet.

If we add this, JSON-or-tar-with-images would be more interoperable than
copying Ditto's "another SQLite file" approach.

---

## 4. UI, window management, rendering

### 4.1 Frontend stacks

| | Ditto | Ditox |
|---|---|---|
| Toolkit | MFC + ATL + GDI/GDI+ + custom NC drawing | Ratatui (TUI) + iced (GUI) — both Rust |
| Rendering | GDI / GDI+ owner-draw | wgpu + tiny-skia (iced); crossterm + ratatui (TUI) |
| List control | `LVS_OWNERDATA` virtual list, owner-drawn rows | iced `Column` of `Container`s; ratatui `List` widget |
| HTML preview in tooltip | **Yes** — embeds `IWebBrowser2` (real MSHTML) inside the tooltip | No |
| RTF preview in list | **Yes** — `CFormattedTextDraw` via hidden `RichEdit` | No |
| Image preview | GDI+ in dedicated `ImageViewer` window (zoom, pan, gestures) | iced `Image` widget in side panel; ratatui-image with kitty/sixel/iterm2/halfblocks protocols in TUI |
| DPI awareness | PMv2, per-monitor DPI; ships PNGs at 11 sizes (24..84 px) | iced handles HiDPI automatically |
| Theming | XML files under `Themes/` (light + DarkerDitto) — re-read on mtime change | Hard-coded palette in GUI; TUI reads `[ui.theme]` from TOML |
| Localization | XML language files (`Language/*.xml`); 25 languages bundled in installer | None |

### 4.2 The "popup launcher" flow

Ditto's defining UX:

1. User presses Ctrl+` (or any of the 3 configured hotkeys).
2. `ExternalWindowTracker::TrackActiveWnd` (already running on a 1-Hz
   timer) has the foreground HWND + focus HWND captured.
3. Ditto's frame is shown (positioned at caret / cursor / previous /
   centred-on-active-window per option) and brought to top.
4. User types to filter, arrows to select, Enter to paste.
5. `ExternalWindowTracker::SendPaste` re-activates the saved foreground,
   loads the clipboard, simulates Ctrl-V, restores foreground state.
6. Holding Ctrl while tapping the hotkey again advances through items
   without dismissing (great for cycling through the last N copies).

Ditox's GUI now follows a similar model post-`013-floating-launcher-redesign`:
420×520 anchored bottom-left, exits on copy / Esc / unfocus. The big
missing piece is **Stage B** — ditox doesn't synthesize Ctrl-V; the user
has to alt-tab and paste manually.

### 4.3 List rendering

Ditto's virtual + owner-drawn list with lazy DIB/RTF caches is well-suited
to 100k+ row DBs. A few details worth remembering:

- Two caches per format: `m_cf_dibCache` (positive) and `m_cf_NO_dibCache`
  (negative) so rows without an image don't re-query.
- Inline thumbnail is **height-fitted once** by `GetDibFittingToHeight` and
  the small DIB replaces the original in memory.
- Row text uses a custom encoding (`<noautodelete><shortcut><group><sticky><ingroup><qpastetext><pasted>|<actual description>`) parsed during paint to render icons inline.
- Match highlighting is RTF markup injected into the description before
  draw — the rich-edit-based draw helper already handles colors and
  bolding.

Ditox's iced list (`view_entry_row`) is per-row containers with stateful
caches in the `DitoxApp` struct (`image_cache: HashMap<String, iced_image::Handle>`).
Performance is fine at 500 entries; would degrade above 5–10 k without
virtualisation. iced 0.14 doesn't have a virtual list out of the box —
worth investigating `iced_aw` or implementing a viewport-clipped column.

### 4.4 Search UX

| | Ditto | Ditox |
|---|---|---|
| Modes | Substring / Wildcard / Regex (ICU REGEXP via SQLite extension) | Fuzzy (`nucleo-matcher`) / Regex (TUI only) / FTS5 (DB pre-filter) |
| Scope toggles | Description / Full text / Quick-paste-text — three toggles, OR'd | Single mode covering content + notes |
| Inline prefixes | `/q `, `/f ` to override mode per-query | None |
| Multi-word | AND/OR/NOT keywords, `"quoted phrases"`, `*` wildcards (mapped to SQL `%`) | Fuzzy handles whitespace as separators |
| Highlighting | RTF-injected markup, themed color | Stored fuzzy-match positions, rendered with highlight style (TUI) |

Ditox's FTS5 + fuzzy combo is more modern and has better UX out of the
box than Ditto's mini-DSL. The one feature worth porting is the **search
mode prefix** (`/q `, `/f `) — keyboard-only mode switching is faster than
`Ctrl+T` to toggle.

### 4.5 Window-management Win32 tricks Ditto uses

- **Foreground lock workaround:** zero `SPI_SETFOREGROUNDLOCKTIMEOUT` →
  `AttachThreadInput` to current foreground thread → `BringWindowToTop` +
  `SetForegroundWindow` → restore lock timeout.
- **Tray-click safety:** the active-window tracker walks parent HWNDs and
  ignores Ditto's own windows / `Shell_TrayWnd` / overflow / notify-area
  HWNDs so clicking the tray doesn't overwrite the saved target.
- **Custom NC area:** every Ditto window draws its own caption (top /
  bottom / left / right) — that's how dark mode works without DWM
  `UseImmersiveDarkMode`.
- **`CDimWnd`:** layered, transparent, click-through "dim" overlay used to
  visually reduce the popup behind modal dialogs.
- **Auto-snap to monitor edges** on `WM_MOVING` (suppressed with Shift).

Ditox already implements similar Win32 workarounds in
`ditox-gui/src/app.rs:148-498` (force_restore_window, remove_topmost,
foreground-lock zeroing). Documented in `docs/notes/win-d-problem.md`.

### 4.6 Tray icon

Both apps use platform-native tray APIs:

- Ditto: PJ Naughter's `CTrayNotifyIcon` (NTray) — `Shell_NotifyIcon`,
  auto-detects struct version, handles `TaskbarCreated` for explorer
  restart.
- Ditox: `tray-icon` 0.22 crate — Win32 on Windows, libappindicator/SNI on
  Linux. Linux requires a dedicated GTK thread because winit doesn't pump
  GTK.

Ditto-only behaviours worth considering:

- Tooltip text reflects state ("Ditto Disconnected" when clipboard chain
  detached).
- One-shot balloon at startup ("Ditto is running minimized").

---

## 5. Hotkeys, IPC, single-instance, sync

### 5.1 Hotkeys

| | Ditto | Ditox |
|---|---|---|
| Global hotkeys | 3 "show" hotkeys, 10 position hotkeys (paste Nth recent), 5 copy-buffer triplets (copy/cut/paste each), text-only-paste, save-clipboard, copy-and-save, per-clip global hotkeys (DB-driven). All via `RegisterHotKey` + `GlobalAddAtom`. | Windows-only Ctrl+Shift+V (via `global-hotkey` crate). Linux relies on compositor binding to launch a fresh process. |
| In-window accelerators | `CAccels` map of ~150 actions (`ActionEnums.h`); two-key chord support | Hard-coded in TUI via `keybindings.rs`; iced GUI hard-codes too |
| Per-clip hotkey | Yes — local + global, stored on `Main.lShortCut` / `Main.globalShortCut` | No |
| Modifier-held cycling | Yes — Ctrl held + tap hotkey advances selection without hiding popup | No |

The "per-clip global hotkey" pattern is unusual and powerful: a user can
bind `Ctrl+Alt+1` to "paste my email signature" forever, decoupled from
clipboard order. Worth considering for ditox once snippets are wired up
(the dead-coded `ui/snippets.rs` module suggests this was planned).

### 5.2 Single-instance

| | Ditto | Ditox |
|---|---|---|
| Mechanism | Named mutex `"Ditto Is Now Running [exe-suffix]"`, second launch `SendMessage`s saved HWND from registry and exits | GUI: **none currently** (post-013 each launch is a new process). Watcher daemon: PID file with no flock. |
| Cross-instance commands | `/connect`, `/disconnect`, `/openWindow`, `/exit`, `/plainTextPaste`, `/pasteClip`, `/editClip`, `/uacpaste:<pid>` — all dispatched via `SendMessage` | CLI flags `--toggle/--show/--hide/--quit` exist as no-ops; legacy flock+sock IPC was removed in `013` |
| UAC elevation handoff | Mutex `DittoAdminPaste_<pid>` + named events `Global\UAC_*_<pid>` | No UAC integration |

The Ditto cross-process command pattern (HWND in registry +
`SendMessage`/`PostMessage`) is simpler than ditox's previous Unix-socket
approach but doesn't translate to Linux. If we re-introduce IPC, the
flock+Unix-socket path documented in `linux-gui-architecture.md` is still
the right design — we just need to put it back.

### 5.3 LAN sync ("Friends")

Ditto has **peer-to-peer LAN clipboard sync**, no central server, no TLS:

- **Default port 23443** TCP, configurable, optional bind IP.
- **Wire protocol:** framed RPC with fixed `CSendInfo` header (`enum eSendType { START, DATA, DATA_START, DATA_END, END, EXIT, REQUEST_FILES }`); per-clip handshake `START → DATA_START/DATA_END (per format) → END`.
- **Encryption:** AES (Rijndael) keyed by SHA-256 of pre-shared password; per-message random 16-byte IV. KDF uses 100 k AES-ECB transformation rounds. Header magic `0x139C5AFE` / `0xBF3562DA`.
- **File transfer (CF_HDROP):** receiver opens a separate connection back to `respondPort`; files MD5-checked.
- **Auto-broadcast:** up to 5 `SendClients` get every saved clip; manual "send to friend N" actions for 15 named friends.
- **Firewall:** installer runs `netsh advfirewall firewall add rule … localport=23443` for both directions when the user opts in.

Ditox has nothing in this space. **Adding it** would be a significant
feature — and the design space is bigger today (mDNS discovery, libp2p,
WebRTC, end-to-end encryption with TOFU keys instead of pre-shared
passwords). See [§10 #6](#10-concrete-suggestions-for-the-ditox-roadmap).

### 5.4 Other cross-process touchpoints

| Mechanism | Ditto use | Ditox equivalent |
|---|---|---|
| Mutex `Ditto Is Now Running …` | Single instance | none |
| Named events `Global\UAC_*` | Parent ↔ elevated child paste handoff | none |
| Atoms via `GlobalAddAtom` | Hotkey IDs | `global-hotkey` crate handles IDs internally |
| MAPI32 `MAPISendMailW` | Send-via-email | none |
| `powrprof.dll` callback | Suspend/resume — re-opens DB on resume (fix for VMware host-resume DB corruption) | none — would be worth adding for laptops |

---

## 6. Configuration & extras

### 6.1 Settings storage

Ditto picks **registry vs INI** per build flavour:

- Default: `HKCU\Software\Ditto`.
- Microsoft Store / Chocolatey / Portable / per-machine override (marker
  files next to exe): UTF-16LE `Ditto.Settings` INI.
- Per-resolution settings via `(WxH)_` key prefix so window position
  follows the active screen size.
- ~300 tunable keys exposed across the Options sheet + Advanced grid.

Ditox uses a single TOML at `~/.config/ditox/config.toml` (Linux) or
`%APPDATA%\ditox\config.toml`. ~10 tunable keys today. Notable gap:
`storage.data_dir` is parsed but ignored — a bug.

### 6.2 Power-user features Ditto has, ditox lacks

This is the long tail that 20 years of accretion has produced. Each item
is a candidate for ditox to consider — most are clearly out of scope, a
few are obvious wins.

| Feature | Ditto file | Worth porting? |
|---|---|---|
| **Special paste** (paste-as-plaintext, upper/lower/capitalize/sentence/camel/inverted, remove-line-feeds, add-LF, paste-with-datetime, typoglycemia, trim-whitespace, posixify-paths, slugify, ascii-only, paste-GUID, paste-images-horizontal/vertical) | `SpecialPasteOptions.cpp` | **Yes** for plain-text + case transforms; rest are nice-to-have |
| **Slugify** with 400+ unicode→ASCII map (latin/greek/cyrillic/CJK currency/symbols) | `Slugify.h` | Yes, as a "transform on copy" option |
| **SQL formatter** (mini-DSL → SQL `WHERE`) | `FormatSQL.cpp` | n/a — we use FTS5 |
| **RTF→text conversion** | `ConvertRTFToText.cpp` | Yes once we capture RTF |
| **Color swatch in list** for `#RRGGBB` / `rgb(...)` / `hsl(...)` | `QListCtrl.cpp::DrawCopiedColorCode` | Yes — small, high-perceived-quality |
| **QR code export** of clip text (libqrencode) | `CreateQRCodeImage.cpp`, `QRCode/` | Niche but cheap |
| **Statistics** (total/trip copy/paste counts, uptime, sent/received this session) | `OptionsStats.cpp` | Already partially there (`ditox stats`); polish UI |
| **Wildcard filters** + **regex filters** (15 slots) per process | `RegExFilterHelper.cpp` | Yes — privacy / password-manager exclusion |
| **HTML preview in tooltip** via `IWebBrowser2` | `SimpleBrowser.cpp` (1281 LOC) | No — heavy, security-sensitive |
| **ChaiScript on-copy / on-paste hooks** with `IClip*` mutator API | `ChaiScript*` files | Maybe — Rhai or Lua would fit Rust better; great extensibility hook |
| **Native add-in DLLs** (FunctionType::PRE_PASTE) | `DittoAddin*.cpp`, `Addins/DittoUtil/` | No — security/portability nightmare |
| **Send-via-email** (MAPI) | `SendMail.cpp` | No |
| **Diff app integration** (compare clip A vs B in external diff tool) | option `DIFF_APP` | Niche, low cost |
| **Save animation** (shrinking focus rect from copied region to tray) | `SaveAnimation.cpp` | No — distracting |
| **U3 USB stick autorun-stop** helper | `U3Stop/` | No — U3 is dead |
| **Per-app paste keystroke** (`gvim`: `"+gp`, old `cmd`: `% {Delay100}ep`) | registry `PasteStrings` | Yes once we synthesize keystrokes (§5.5) |
| **CryptProtectData** for password storage | commented-out in `Options.cpp` | n/a until we have sync passwords |
| **Windows Error Reporting LocalDumps** registration in installer | `DittoSetup_10.iss` | Yes — easy in Inno Setup |
| **CF_DIB ignore list** (Excel/OneNote/PowerPoint produce a useless DIB alongside text) | `SETTING_IGNORE_ANNOYING_CF_DIB` | Yes once we capture multiple formats |
| **Translate / web-search URL templates** (right-click "translate" or "search") | options `TranslateUrl` / `WebSearchUrl` | Cheap and useful |
| **Suspend/resume DB reopen** (PowerManager.cpp) | `PowerManager.cpp` | Yes — adds robustness on laptops |
| **Per-resolution window position** | `GetResolutionProfileLong` | Yes |

### 6.3 i18n

Ditto's XML-based i18n (`Language/<name>.xml`, 25 languages bundled) is
significantly more capable than what ditox has (none — all strings
hard-coded English). If we ever want translations:

- Keying by control ID + English fallback string is a good pattern (every
  call site stays in English-readable code).
- File-based localisation is preferable to gettext for this kind of app
  (no compilation, hot-reload-friendly).
- `fluent-rs` would be the modern Rust analogue.

---

## 7. Side-by-side feature matrix

Legend: ✅ present and complete · 🟡 partial / present but not exposed · ❌ absent

| Capability | Ditto | Ditox |
|---|---|---|
| **Capture** | | |
| Event-based clipboard listener (Windows) | ✅ | ❌ (polling) |
| Multi-format capture (text/image/HTML/RTF/files/custom) | ✅ all | 🟡 text + image only |
| Image format priority list | ✅ configurable | ✅ hard-coded PNG→JPEG→GIF→WebP→BMP |
| Format-by-name persistence | ✅ | n/a (closed enum) |
| Honors "do-not-record" hints | ✅ | ❌ |
| Per-app capture filter (include/exclude) | ✅ wildcard list | ❌ |
| Multi-clip aggregator (HTML/RTF/CF_HDROP/image) | ✅ | 🟡 plain text concat (TUI multi-select) |
| **Storage** | | |
| Inline blob storage | ✅ | ✅ for text |
| External content-addressed image store | ❌ | ✅ |
| Atomic write protocol | n/a | ✅ |
| Schema versioning | ❌ probe-based | ✅ `schema_meta.version` |
| FTS index | 🟡 ICU REGEXP via SQLite extension | ✅ FTS5 + triggers |
| Soft-delete journal | ✅ `MainDeletes` + idle purge | ✅ `pending_blob_prunes` |
| Encryption at rest | ❌ | ❌ |
| Compression at rest | ❌ | ❌ |
| Import/export | ✅ `.dto` files (zlib-compressed SQLite) | ❌ |
| Backup (gzipped DB) | ✅ | ❌ |
| Repair / compact | ✅ (UI button + idle-time) | ✅ `ditox repair` CLI |
| **Organization** | | |
| Groups / collections | ✅ recursive nested | 🟡 flat, CLI-only, not in GUI |
| Favorites / pinned | ✅ "sticky" | ✅ `pinned` column |
| Notes / annotations | 🟡 description text | ✅ `notes` column, FTS-indexed |
| Tags | ❌ | ❌ |
| Per-clip global hotkey | ✅ | ❌ |
| Quick-paste slots (1-9) | ✅ ten | 🟡 dead-coded `ui/snippets.rs` |
| **Search** | | |
| Substring search | ✅ | ✅ |
| Wildcard search | ✅ `*`, `?`, `[]`, AND/OR/NOT | 🟡 fuzzy (different paradigm) |
| Regex search | ✅ ICU | ✅ TUI only |
| Fuzzy search | ❌ | ✅ nucleo-matcher |
| Mode prefixes (`/q `, `/f `) | ✅ | ❌ |
| Search-result highlighting | ✅ RTF markup | ✅ TUI |
| **UX** | | |
| Floating launcher window | ✅ | ✅ (post-013) |
| Position at caret / cursor / previous / centred | ✅ | 🟡 bottom-left of monitor |
| Multi-select | ✅ | ✅ TUI only |
| Cycle items by holding modifier | ✅ | ❌ |
| Inline image thumbnails in list | ✅ | ✅ GUI, ✅ TUI (graphics protocols) |
| Inline RTF rendering in list | ✅ | n/a |
| Inline color swatch for hex/rgb/hsl strings | ✅ | ❌ |
| Image viewer (zoom/pan/gestures) | ✅ GDI+ | 🟡 GUI side panel, no zoom |
| HTML preview in tooltip | ✅ MSHTML | ❌ |
| Custom NC / dark mode | ✅ | ✅ Windows custom title bar |
| Theming | ✅ XML themes hot-reload | 🟡 hard-coded GUI / TUI TOML |
| HiDPI / per-monitor DPI | ✅ PMv2 + 11 PNG sizes | ✅ via iced |
| i18n | ✅ 25 languages | ❌ |
| **Paste** | | |
| Synthesize Ctrl-V to foreground app | ✅ + per-app keystroke + UAC handoff | ❌ |
| Paste-as-plaintext | ✅ | ❌ |
| Special paste transforms (case/slug/GUID/datetime/typo/etc.) | ✅ ~25 | ❌ |
| Paste-images-horizontal / vertical | ✅ | ❌ |
| **Hotkeys & IPC** | | |
| Global hotkeys | ✅ ~25 actions | 🟡 Windows only, single hotkey |
| Per-clip global hotkey | ✅ | ❌ |
| Single-instance | ✅ named mutex | ❌ (one-shot) |
| Cross-instance command dispatch | ✅ HWND+SendMessage | ❌ (was flock+sock, removed) |
| **Sync / network** | | |
| LAN peer-to-peer sync | ✅ AES + pre-shared password, port 23443 | ❌ |
| Cloud sync | ❌ | ❌ |
| File-transfer over wire | ✅ MD5-checked | ❌ |
| **Extensibility** | | |
| Native add-in DLLs | ✅ | ❌ |
| Embedded scripting (on-copy / on-paste hooks) | ✅ ChaiScript | ❌ |
| **Misc** | | |
| Statistics page | ✅ | ✅ `ditox stats` |
| QR code export | ✅ libqrencode | ❌ |
| Send-via-email (MAPI) | ✅ | ❌ |
| Suspend/resume awareness | ✅ reopen DB on resume | ❌ |
| Save animation | ✅ shrinking focus rect | ❌ |
| Crash dump registration | ✅ WER LocalDumps | ❌ |
| Per-process keystroke override | ✅ registry subkeys | ❌ |
| Per-resolution window state | ✅ | ❌ |
| **Distribution** | | |
| Windows installer | ✅ Inno Setup | ✅ Inno Setup |
| Portable build | ✅ | ❌ |
| Microsoft Store | ✅ | ❌ |
| Chocolatey / Winget | ✅ both | ❌ |
| AppImage | ❌ | ✅ |
| Nix flake | ❌ | ✅ |
| Code-signing | ✅ SignPath Foundation | ❌ |
| **Platforms** | | |
| Windows | ✅ | ✅ |
| Linux/Wayland | ❌ | ✅ |
| macOS | ❌ | ❌ |
| **License** | GPL-3.0 | MIT |

---

## 8. Architectural patterns worth borrowing (clean-room)

Patterns observed in Ditto that we can re-implement independently in Rust
without touching their code. None of these require copying — they're
ideas, not bytes.

1. **Format-by-name storage.** When we add multi-format capture, store the
   format **string** (e.g. `"text/html"`, `"text/rtf"`) rather than the
   numeric Win32 `CLIPFORMAT` ID. This makes export and cross-machine
   import tractable.

2. **Aggregators for multi-clip paste.** A trait `FormatAggregator { fn
   add(&mut self, blob: &[u8], idx: usize, count: usize); fn build(self)
   -> Vec<u8>; }` with per-format implementations. Much cleaner than a big
   match arm.

3. **`Clipboard Viewer Ignore` sentinel.** When ditox writes to the
   clipboard during paste, we should publish a sentinel that our own
   watcher recognises and skips. Avoids loop-back. Not relevant on Linux
   (where wl-copy daemonises) but critical on Windows.

4. **HTML envelope round-trip.** Parse + recompute byte offsets when
   we eventually capture HTML.

5. **Stable hashing for RTF.** Strip `\rsid` / `\insrsid` /
   `{\*\datastore}` before SHA-256.

6. **Soft-delete journal + idle-time purge.** Already have it
   (`pending_blob_prunes`); good to keep doing this.

7. **Per-app paste keystroke override.** Some apps need `Shift+Insert` or
   `Ctrl+Y` instead of `Ctrl+V`. Store as `HashMap<String, String>` in
   config keyed by process basename.

8. **Foreground-lock-defeating activate sequence.** Already implemented in
   `ditox-gui/src/app.rs::force_restore_window`.

9. **Suspend/resume DB reopen.** Listen for `WM_POWERBROADCAST` /
   `Resume` events on Windows; on Linux watch logind via dbus. On resume,
   close + reopen the DB connection so any host-VM weirdness doesn't
   stale-handle us.

10. **Watchdog ping for the Windows clipboard listener.** Once we move to
    `AddClipboardFormatListener`, periodically write our own custom
    format to verify membership; reconnect on miss.

11. **Per-app capture exclusion.** Store globs like `"*KeePass*",
    "*1Password*"` and skip captures whose foreground process matches.
    Critical for password-manager users.

12. **Color swatch detection.** Quick win: regex-match
    `#[0-9a-fA-F]{6}` / `rgb\(...\)` / `hsl\(...\)` and render a 12×12
    color block before the text in the list.

13. **Search mode prefixes.** `/q `, `/f `, `/r ` to switch search scope
    or mode without leaving the keyboard.

14. **Per-resolution window state.** Key window position by `(WxH)_` so
    laptop ↔ external monitor ↔ docked behaves naturally.

---

## 9. Anti-patterns to avoid

Things Ditto does that we should **not** copy.

1. **CRC-32 deduplication.** Collision-prone, demands per-format
   canonicalisation kludges. SHA-256 (current ditox) is correct.

2. **No schema versioning.** "Try the SELECT, ALTER on exception" works
   but the current schema is only legible by reading 200 lines of
   migration code. Keep `schema_meta.version`.

3. **Inline image BLOBs.** A 5 MB screenshot stored twice (DIB + PNG)
   inflates SQLite. Content-addressed external store (current ditox) is
   strictly better for this workload.

4. **No transactions on the hot path.** Every clip insert is its own
   implicit transaction with an fsync. On a busy DB this is the dominant
   cost. We should explicitly batch when possible.

5. **CRC-only dedup with no second-stage byte compare.** Distinct clips
   silently merge. Either second-stage compare or use a cryptographic
   hash.

6. **String-concatenated SQL** (`Format("WHERE … = '%s'", value)` in
   `Clip.cpp::LoadFormat`). Use prepared statements with binds — every
   single `rusqlite` query in ditox already does this; keep doing it.

7. **287 source files in a single MFC project.** Module boundaries matter
   for both compile time and reasoning. Ditox's three-crate split is
   already healthier; keep core thin.

8. **Pre-shared-password symmetric encryption** for sync. Vulnerable to
   key compromise. If we add sync, use modern asymmetric crypto (e.g.
   noise protocol, libp2p, age).

9. **Native plug-in DLLs with full process privilege.** Add-in DLLs
   running unsandboxed in the host process is a security and stability
   nightmare. Pick a sandboxed scripting engine (Rhai, Lua via mlua) or
   an out-of-process WASM model.

10. **Embedding `IWebBrowser2` to render HTML previews.** MSHTML is huge,
    has decades of CVEs, and ties us to Windows. If we want HTML preview,
    use `pulldown-cmark` for markdown or sanitize-and-render HTML
    ourselves.

11. **No Linux/macOS port path.** MFC, the registry, GDI/GDI+, GDI handles
    everywhere — Ditto cannot port. Ditox already pays the abstraction
    cost; don't regress.

12. **Mixing portable / installed / Store / Chocolatey detection via
    marker files next to the exe.** Use a single config-path resolver and
    expose `--config <path>` at the CLI. Done.

---

## 10. Concrete suggestions for the ditox roadmap

In rough priority order. Each item is independently tractable; estimates
assume "one focused weekend".

### 10.1 Event-based Windows clipboard listener (M)

Move the Windows watcher off polling. Use `windows-rs` to call
`AddClipboardFormatListener` against a hidden message-only window; pump
`WM_CLIPBOARDUPDATE`. Keep polling as a fallback. Saves idle CPU + cuts
capture latency to ~zero. Linux stays on polling until we wrap
`wlr-data-control` ourselves.

### 10.2 Multi-format capture (L)

Open `EntryType` to `Text | Html | Rtf | Image | Files | Custom(String)`
and store one row per format in a new `entry_formats` table linked by
`entry_id`. Update `Watcher::poll_internal` to enumerate every available
format on the clipboard, not just one. Implications cascade through
schema, search, paste, and UI — this is the biggest architectural move
on the list, but it unlocks everything else (HTML preview, paste-as-rich,
file-paste).

### 10.3 Synthesize Ctrl-V on copy (M)

Rename `Message::CopyEntry` to `Message::PasteEntry`. After writing the
clipboard, restore the previously-foreground window (already tracked) and
synthesize Ctrl-V via `SendInput` (Windows) or `ydotool` / virtual
keyboard (Linux). Adds the "click → paste" UX that makes Ditto feel
magical. Optional per-app keystroke override.

### 10.4 Per-app capture exclusion (S)

Config key `[capture] exclude = ["*KeePass*", "*1Password*", "*Bitwarden*"]`.
On Windows, resolve foreground process basename via `GetForegroundWindow`
+ `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`; on Linux,
read `/proc/<pid>/comm` after locating the focused window via
`wlr-foreign-toplevel-management`.

### 10.5 Special paste menu (M)

Right-click / Tab on a text entry → submenu of transforms: paste-as-plain,
upper, lower, capitalize, snake-case, kebab-case, slugify, trim, GUID,
remove-line-feeds, datetime. Ports the Ditto "Special Paste" menu to
ditox. Pure transformations on the text, no clipboard format change.

### 10.6 LAN peer-to-peer sync (XL)

Optional feature. mDNS discovery of other ditox instances on the LAN,
TOFU-pinned ed25519 keys, noise-protocol-encrypted TCP, sync the
`entries` rows + image blobs. Out of scope for v0.4 but the design should
be sketched now so the core schema accommodates it (UUID PKs and SHA-256
hashes already do).

### 10.7 GUI feature parity with TUI (M)

The GUI is missing: regex search, multi-select, collection management,
per-row favorite toggle, customizable keybindings, image zoom in side
panel. All in the existing `gui-improvements.md` task; just unblock and
ship.

### 10.8 Notes/Color/Tag wins (S each)

- Color swatch in list rows when the content matches `#RRGGBB` /
  `rgb(...)` / `hsl(...)`.
- Tag system (many-to-many between entries and tags) — orthogonal to
  collections.
- "Today" tab is already there; add "Yesterday", "This Week", "Last
  Month" filter chips.

### 10.9 Settings UX (M)

`storage.data_dir` is a documented bug (parsed but ignored). Fix the
resolver to honour it. While we're there, add a settings GUI screen so
users don't need to edit TOML by hand for the things that matter
(`max_entries`, `poll_interval_ms`, theme).

### 10.10 Suspend/resume + power awareness (S)

Listen for `WM_POWERBROADCAST` (Windows) and logind `PrepareForSleep` /
`PrepareForShutdown` signals (Linux/dbus). On resume, drop and reopen
the rusqlite handle to dodge any host-VM staleness. Cheap insurance.

### 10.11 Save-on-clear-clipboard option (S)

Ditto has "Save Clipboard" hotkey that captures whatever is on the
clipboard right now even if we already have it (forces dedup-bypass). Add
`ditox save` CLI command + optional global hotkey.

### 10.12 i18n bootstrap (M)

Wrap user-facing strings in `t!("key", "english fallback")`, vendored
translations under `locales/<lang>/<crate>.ftl` (Fluent format). Even with
zero translations to start, the wrapping is what makes future
contributions easy.

---

## Appendix A — File reference index

For future work, key Ditto file paths discovered during the analysis:

- Clipboard listener: `src/ClipboardViewer.{h,cpp}`,
  `src/CopyThread.{h,cpp}`
- Clip model & dedup: `src/Clip.{h,cpp}`, `src/Crc32Dynamic.h`,
  `src/IClipAggregator.h`, `src/CF_*Aggregator.{h,cpp}`,
  `src/HTMLFormatAggregator.{h,cpp}`, `src/RichTextAggregator.{h,cpp}`,
  `src/ImageFormatAggregator.{h,cpp}`
- Paste path: `src/ProcessPaste.{h,cpp}`, `src/OleClipSource.{h,cpp}`,
  `src/SendKeys.{h,cpp}`, `src/ExternalWindowTracker.{h,cpp}`
- Storage: `src/DatabaseUtilities.{h,cpp}`, `src/Clip_ImportExport.{h,cpp}`,
  `src/MainTableFunctions.{h,cpp}`, `src/sqlite/*`
- Encryption (sync only): `EncryptDecrypt/Encryption.{h,cpp}`
- Hotkeys / accelerators: `src/HotKeys.{h,cpp}`, `src/Accels.{h,cpp}`,
  `src/ActionEnums.{h,cpp}`
- Sync / Friends: `src/Server.{h,cpp}`, `src/Client.{h,cpp}`,
  `src/SendSocket.{h,cpp}`, `src/RecieveSocket.{h,cpp}`,
  `src/AutoSendToClientThread.{h,cpp}`, `src/CustomFriendsHelper.{h,cpp}`,
  `src/ServerDefines.h`
- UI / popup: `src/QPasteWnd.{h,cpp}`, `src/QListCtrl.{h,cpp}`,
  `src/MainFrm.{h,cpp}`, `src/ExternalWindowTracker.{h,cpp}`
- Theming / DPI / dark mode: `src/Theme.{h,cpp}`, `src/DPI.{h,cpp}`,
  `src/AlphaBlend.{h,cpp}`, `src/ModernScrollBar.{h,cpp}`
- Add-in ABI: `Shared/IClip.h`, `Shared/DittoDefines.h`,
  `src/DittoAddin*.{h,cpp}`, `Addins/DittoUtil/`
- Scripting: `src/DittoChaiScript.{h,cpp}`, `src/ChaiScriptOnCopy.{h,cpp}`,
  `src/ChaiScriptXml.{h,cpp}`, `src/chaiscript/`
- Special paste / transforms: `src/SpecialPasteOptions.{h,cpp}`,
  `src/Slugify.h`, `src/ConvertRTFToText.{h,cpp}`,
  `src/FormatSQL.{h,cpp}`
- Installer: `DittoSetup/DittoSetup_10.iss`, `DittoSetup/*.isl`,
  `DittoSetup/Chocolatey/`

## Appendix B — Methodology

- Clone: `git clone --depth 1 https://github.com/sabrogden/Ditto.git
  /tmp/Ditto-reference`.
- Six parallel exploration agents covered: clipboard capture, storage,
  UI/window management, hotkeys/IPC/sync, config/extras, and a ditox
  baseline survey.
- All file:line references are valid for Ditto release 3.25.113.0
  (master at `2026-04-02 04:06:06 UTC`) and ditox `v0.3.1`.
- No GPL code was copied. This document describes patterns and behaviours
  observed at the architectural level only.
