**Ditox TUI Full Visual Customization — PRD**

- Repo context summary
  - TUI is implemented in `ratatui 0.27` + `crossterm 0.27` (portable across Linux/macOS/Windows).
  - Current “picker” lives in `crates/ditox-cli/src/picker.rs:239` (terminal init/alt screen) and loads a minimal theme via `crate::theme::load_tui_theme()` (`crates/ditox-cli/src/picker.rs:270`).
  - Existing theme support is experimental and limited to four color knobs in `~/.config/ditox/tui_theme.toml`, parsed in `crates/ditox-cli/src/theme.rs:1`.
  - TUI settings in config cover paging/time formatting only (`crates/ditox-cli/src/config.rs:48`).
  - README documents the picker and the ad‑hoc theme file (`README.md:213`, `README.md:225`).

Goal, in one line: deliver a robust, cross‑platform system that lets users fully customize the TUI’s look and feel (colors, styles, glyphs, borders, layout, visibility, and status/help content), with safe defaults, graceful fallbacks, and no regressions to CLI or headless flows.

Status (2025-10-02)
- Implemented in repo (initial slice):
  - CLI flags on `pick`: `--theme <name|path>`, `--ascii`, and `--color auto|always|never`.
- Reads `~/.config/ditox/settings.toml` `[tui]` when flags are not provided: `theme`, `color`, `box_chars`, `alt_screen`, `glyphs`, `layout`.
  - Date/time customization: `[tui] date_format = "dd-mm-yyyy"`, `[tui] auto_recent_days = 3` (used by `{*_auto}` and `{recent}` tokens).
  - Discovery and preview: `ditox pick --themes` lists built-ins + user themes; `ditox pick --preview <theme>` prints an ASCII snapshot without entering the alt screen.
  - Glyph packs: `--glyphs <name|path>`; list with `--glyphsets`. Built-ins: `ascii`, `unicode`.
  - Layout packs: `--layout <name|path>`; list with `--layouts`. Minimal `help = "visible|hidden"` supported.
  - Capability dump: `ditox pick --dump-caps` prints `color_depth`, `unicode`, and `no_color` detection.
  - Built-in themes: `dark` (default) and `high-contrast`.
  - Palette mapped today: highlight fg/bg, border fg, help fg (compatible superset of legacy `tui_theme.toml`).
  - ASCII mode removes Unicode borders and replaces ⏎ with `Enter` in help/footer; `NO_COLOR` respected.
  - Tests: unit tests for color parsing; integration tests for `--themes` and `--preview` (asserts ASCII-only output).
- Removed: legacy `tui_theme.toml` fallback; themes must come from built-ins or `~/.config/ditox/themes/*.toml`.
- Deferred to next phases: templates, full palette surface, richer glyph sets, live reload, layout presets beyond footer toggle.

---

**Objectives**

- Visual customization breadth
  - Color palette (truecolor + 256/16‑color fallback) and semantic style tokens used consistently across all widgets.
  - Typography styles (bold, italic, underline, dim) where supported by terminals.
  - Glyph/icon set, border styles (plain/rounded/double), and line‑drawing charset toggles (ASCII vs Unicode).
  - Layout knobs (panel visibility, preview/metadata composition, help/status bars on/off, positions, padding, wrap).
  - List item template customization (text, image, and metadata badges).
  - Search highlight style and match markers.
  - Status/help/footer content strings with variable interpolation (non‑i18n; English only in v1).
- Runtime ergonomics
  - Theming files discoverable under the config dir (Linux XDG, macOS Application Support, Windows Roaming).
  - Hot reload on theme file change (optional: live preview toggle).
  - CLI flags to select/override theme/layout/glyph packs for one‑off sessions.
  - Defaults that “just work” on minimal terminals; obey `NO_COLOR`.
- Cross‑platform fidelity
  - Works on Ubuntu/Debian/Fedora/Arch, NixOS; macOS 12+; Windows 10 1903+ and Windows Terminal/ConPTY.
  - Degrades gracefully on terminals without truecolor/Unicode box‑drawing; verified behavior on tmux/screen.
- Safety/performance
  - Zero panics on bad theme files; fall back to sane defaults with warning.
  - Startup stays snappy (<=50 ms overhead vs today). No redraw thrash. No excessive file watchers on Windows.
  - No user content leakage in logs; never execute user theme as code.

Non‑goals (v1)
- Keybinding remapping (out of scope for “visual” customization; can be v2).
- Full i18n/l10n of strings (follow‑up; we allow string templates for a few surfaces in English).
- Image previews inside terminal (sixel/kitty graphics) — out of scope.
- Font selection (terminals own fonts; we support glyph fallbacks).

---

**Personas & Use Cases**

- Backend dev with light terminal: wants high‑contrast, no Unicode, no colors (NO_COLOR).
- Mac user with modern terminal: prefers a Solarized‑Dark look with rounded borders and Nerd Font icons.
- Ops/SRE in tmux: disable alt screen, maximize list density, show time as relative, compact help.
- Windows developer: ensure borders render, avoid misaligned wide glyphs, keep colors consistent.
- Accessibility needs: provide a colorblind‑friendly scheme and non‑color affordances (markers, bold/underline).

---

**Current State (as‑is) Constraints**

- Theme struct has 4 colors only and no border/glyph/layout control (`crates/ditox-cli/src/theme.rs`).
- Styles are hard‑coded ad‑hoc in `picker.rs` (borders/title/help lines etc.).
- Config has limited TUI knobs: page size, tag auto‑apply, absolute time toggle (`crates/ditox-cli/src/config.rs:48`).
- README mentions `tui_theme.toml` but not discoverable themes or CLI overrides.

---

**Functional Requirements**

- Theming model
  - Define a semantic palette with tokens used across the UI:
    - Core: `fg`, `bg`, `muted`, `surface`, `surface_alt`, `accent`, `accent_muted`, `success`, `warning`, `error`, `info`, `hint`, `selection_fg`, `selection_bg`, `search_match_fg`, `search_match_bg`, `badge_fg`, `badge_bg`, `border_fg`, `title_fg`, `help_fg`, `status_fg`, `status_bg`, `scrollbar_fg`, `scrollbar_bg`, `favorite_fg`, `image_fg`, `tag_fg`, `tag_bg`.
  - Each token is a color + optional `modifiers` (bold/italic/underline/dim).
  - Colors accept named (`red`), hex (`#RRGGBB`), or `rgb(r,g,b)` formats; degrade to 256/16 color maps automatically.
  - Global theme options: `border_style` (plain|rounded|double), `box_chars` (unicode|ascii), `intensity` scale (0.8–1.2 to lighten/darken), `contrast_mode` (normal|high), `truecolor` (auto|force|off), `no_color` obeys `NO_COLOR` env unless `force=true`.
- Glyphs/Icons
  - All glyphs are user‑configurable strings with width constraints and fallbacks:
    - `icons.favorite`, `icons.image`, `icons.text`, `icons.tag`, `icons.search`, `icons.selected`, `icons.unselected`, `icons.separator`, `icons.scroll_thumb`, `icons.scroll_track`, `icons.toast_ok`, `icons.toast_warn`, `icons.toast_err`.
  - Provide two built‑in sets: `ascii` and `nerdfont`; allow custom per‑icon overrides.
- Layout and visibility
  - Options to show/hide: header title, help line, status bar, scrollbar, metadata line, badges.
  - Positions: `search_bar_position` (top|bottom), `help_position` (bottom|inline?), `status_position` (bottom).
  - Spacing: `padding_x`, `padding_y`, `gutter` between list and preview (if preview present later).
  - Density: `list_line_height` (1 or 2), `wrap_preview` (true|false), `truncate_marker` string.
- Templates
  - `list.item.template`: text template with tokens: `{preview}`, `{created_abs}`, `{created_rel}`, `{last_used_abs}`, `{last_used_rel}`, `{favorite}`, `{tags}`, `{kind}`.
  - `list.meta.template`: metadata line template; allow dim style mapping and per‑token style overrides, e.g., `{favorite:favorite_fg}`.
- Search highlight
  - Style tokens for primary/secondary highlights; toggle underline instead of color when `NO_COLOR`.
- Status/help/footer
  - Customize static text and separators; expose variables like `{total}`, `{page}`, `{page_count}`, `{filters}`, `{query}`.
- Behavior toggles (visual)
  - `alt_screen` (on|off), `cursor_visible` (on|off), `mouse` (on|off; scroll to move selection), `scrollbar` (on|off).

---

**Configuration Model**

- Files and discovery
  - Config root uses `directories` crate:
    - Linux: `~/.config/ditox`
    - macOS: `~/Library/Application Support/ditox`
    - Windows: `%APPDATA%\ditox`
  - New tree:
    - `settings.toml` (existing; can contain `[tui]` overrides)
    - `themes/<name>.toml` (theme files)
    - `glyphs/<name>.toml` (optional glyph packs)
    - `layouts/<name>.toml` (optional layout packs)
  - Defaults:
    - Built‑ins: `dark`, `light`, `high-contrast`, `solarized-dark`, `ascii-minimal`.
- Effective config resolution
  - Load `settings.tui.theme = "<name>"` and locate `themes/<name>.toml`; merge any inline `[tui.colors]`, `[tui.layout]`, `[tui.glyphs]` from `settings.toml`.
  - CLI overrides:
    - `ditox pick --theme <name|path>` (path can be absolute or relative to config dir)
    - `--glyphs <name|path>`, `--layout <name|path>`, `--no-color`, `--color=always|auto|never`
    - `--no-alt-screen`, `--unicode`/`--ascii`.
- Schema (v1)
  - Top‑level: `version = 1`
  - `[palette]` semantic tokens with optional `[palette.<token>.modifiers] = ["bold","underline","dim"]`
  - `[options]` border_style, box_chars, truecolor, intensity, contrast_mode, no_color.force
  - `[icons]` keys listed above
  - `[layout]`: visibility/positions/spacing/density and templates
  - Backward compat: support legacy `tui_theme.toml` with `highlight_fg`, `highlight_bg`, `border_fg`, `help_fg` mapped onto new tokens.

---

**Terminal Capability Detection & Fallback**

- Capability probe (on start; cache)
  - Color depth (truecolor/24‑bit vs 256 vs 16) using `COLORTERM=truecolor`, `TERM`, `crossterm::supports_color`.
  - Unicode line drawing support: detect via env (`LC_CTYPE` UTF‑8) and optionally a one‑frame probe guarded by `--assume-unicode`.
  - Windows: rely on `crossterm` to enable VT processing; assume Windows 10 ConPTY or Windows Terminal; fallback to ASCII if enabling VT fails.
  - tmux/screen: respect `$TERM` and map truecolor to 24‑bit sequences if supported (`tmux` passthrough).
- Fallback strategy
  - If `NO_COLOR` or `--color=never`: strip all colors; preserve bold/underline unless `NO_COLOR` is treated as “monochrome only”.
  - If 256/16 colors: map Rgb to nearest palette; keep tokens distinct enough to remain legible.
  - If `box_chars=ascii` or Unicode not supported: replace all borders and icons with ASCII equivalents.
  - If terminal width < 50 cols: auto switch to 1‑line list items and hide help; show minimal status.

---

**UX Details (Applied to Current Picker)**

- Widgets influenced
  - Outer frame (Block/Borders/Title): tokenized `border_fg`, `title_fg`, `surface` background; `border_style` respected.
  - List items: per‑state styles (normal, selected, favorited, image), badges for `{kind}` (text/image) and `{favorite}` glyph.
  - Search bar: prompt icon, query text, highlight style applied to matches in list.
  - Metadata line: dim or configured style; template‑driven composition.
  - Help/footer: configurable text and separators; allow compact/expanded variants.
  - Status bar: page/total display, filter badges, remote badge (existing `remote_badge` bool), error/warn toast area.
- Live reload
  - Watch active theme file(s) (and optional glyph/layout files) with `notify` crate; on change, parse safely and re‑render next frame.
  - Toggle: `settings.tui.live_reload = true|false` and `--no-reload`.

---

**Accessibility**

- Built‑in `high-contrast` palette and `colorblind-safe` palette (avoid red/green ambiguity, emphasize luminance and accents).
- Non‑color affordances: selection uses inverse + underline; search highlights use underline in `NO_COLOR`.
- User‑settable `ui_scale` for padding and spacing (does not change font size, but adds whitespace; optional).
- Ensure legibility under 256‑color mapping: contrast ratio checks at parse time; warn if too low.

---

**Security & Privacy**

- Theme/glyph/layout files are parsed as TOML only; no code execution or path traversal outside config dir unless explicit `--theme /abs/path`.
- On parse failure or unknown keys: log a single warning to stderr (not saved), continue with defaults.
- Never log clipboard contents or DB data due to rendering errors.

---

**Performance & Resource Requirements**

- Cold start overhead for capability probe + theme load: target <= 50 ms.
- Live reload debounce: 150 ms; maximum of 4 redraws per second due to file change.
- No background threads beyond file watcher; watcher off by default on Windows to avoid extraneous wakeups; enable only when `live_reload=true` or `--reload`.
- Memory footprint comparable to current TUI; theme structs small and static.

---

**API & CLI Surface**

- CLI
  - Delivered: `ditox pick --theme <name|path> [--ascii] [--color auto|always|never]`
  - Delivered: `--glyphs <name|path>`, `--layout <name|path>`
  - Delivered: `--themes`, `--glyphsets`, `--layouts`, `--preview <name>`, `--dump-caps`
  - Planned: `--no-alt-screen`, `--mouse on|off`
- Settings (`settings.toml`)
  - `[tui] theme`, `glyphs`, `layout`, `live_reload`, `color` (auto|always|never), `alt_screen`, `mouse`, `box_chars`, `truecolor`.
  - Keep existing fields (`page_size`, `auto_apply_tag_ms`, `absolute_times`).

---

**Backwards Compatibility & Migration**

- Legacy `~/.config/ditox/tui_theme.toml` support removed. Users should create `~/.config/ditox/themes/<name>.toml` and set `[tui].theme = "<name>"`.
- Default TUI behavior without any settings remains visually close to today’s style.
- No changes to non‑interactive CLI commands or JSON outputs.

---

**Cross‑Platform Considerations**

- Linux (Ubuntu, Debian, Fedora, Arch, NixOS)
  - Works in common terminals (GNOME Terminal, Alacritty, Kitty, WezTerm, Konsole) and tmux/screen; truecolor auto where available.
  - NixOS: theme files live in the same XDG config dir; no system services required.
- macOS
  - Terminal.app: 256 colors by default; map truecolor tokens down; ensure borders/glyphs degrade cleanly.
  - iTerm2/Kitty/WezTerm: support truecolor and Unicode; rounded borders/icons OK.
- Windows
  - Require Windows 10 1903+; `crossterm` enables VT processing on ConPTY. Fallback to ASCII borders/glyphs if VT fails.
  - Windows Terminal OK; legacy cmd.exe supported with `--ascii` and 16‑color palette mapping.
  - Config dir via `directories`: `%APPDATA%\ditox\themes`, etc.

---

**Error Handling & Observability**

- Parsing
  - Validate colors; on invalid hex or bad rgb tuple → ignore that key and warn “invalid color ‘xyz’, using default for token ‘badge_bg’”.
- Runtime
  - If a theme references unknown tokens, warn once; ignore extras silently.
  - Add `RUST_LOG=ditox=debug` tracing around theme load, capability detection, and fallback selection.
- Diagnostics
  - `ditox pick --dump-caps` prints detected capabilities (color depth, unicode, alt screen availability).
  - `ditox pick --preview <theme>` prints the first frame to stdout for easy visual testing in CI.

---

**Acceptance Criteria**

- Delivered
  - The picker renders near‑equivalent defaults with no theme present.
  - Users can select a built‑in theme: `--theme high-contrast` works cross‑platform; borders and colors reflect the theme where Unicode/color are available.
  - ASCII mode (`--ascii`) shows no Unicode borders/icons; layout remains aligned. `NO_COLOR` suppresses color tokens; selection continues to use reverse/bold.
  - `--themes`, `--preview`, and `--dump-caps` behave as specified (preview is ASCII only, no alt screen).
  - Unit tests: color parsing and preview smoke assert ASCII‑only output.
- Pending
  - Live reload while editing theme files.
  - Snapshot tests that assert styled buffer regions for one theme under truecolor and ascii+16‑color.
  - README updates and migration notes.

---

**Implementation Plan**

- Phase 0: Refactor scaffolding
  - Introduce `ui::caps` (detect capabilities), `ui::style` (palette tokens, style resolver), `ui::glyphs`, `ui::layout`, `ui::theme_loader` (disk + built‑ins).
  - Replace direct `Style::default().fg(...)` calls with `StyleToken::X.resolve(caps)`.
- Phase 1: Palette and glyphs
  - Implement palette schema, parsing, default themes (dark/light/high‑contrast/solarized/asc‑minimal).
  - Implement glyph packs and fallbacks (ascii/nerdfont).
- Phase 2: Layout and templates
  - Add visibility/position/spacing toggles; integrate into `picker.rs` draw routines.
  - Add list/meta templates and token rendering; add search highlight token styles.
- Phase 3: CLI surface and discovery
  - Delivered: `--theme`, `--color`, `--ascii`, `--themes`, `--preview`, `--dump-caps`.
  - Pending: `--layout`, `--glyphs`, `--no-alt-screen`, `--mouse`.
- Phase 4: Live reload
  - Add file watcher with debounce; guard by setting and CLI; ensure Windows idleness.
- Phase 5: Tests, docs, polish
  - Parser unit tests; buffer snapshot tests; cross‑platform CI matrix (Linux, macOS, Windows) for `--preview` textual snapshots.
  - README + `PRD.md` updates; migration note for `tui_theme.toml`.

---

**Testing Strategy**

- Unit tests
  - Color parsing round‑trips (named, hex, rgb); modifier parsing; intensity adjustments and 256/16 mapping.
  - Glyph width safety: reject 0‑width and overly wide default icons; ensure fallback.
- Snapshot tests
  - Use `ratatui::buffer::Buffer` to render a deterministic frame with fixed caps: one with truecolor+unicode, one with ascii+16‑color.
  - Assert tokenized styles are applied in expected regions (selection, matches, badges).
- Integration tests
  - Headless picker flow (already present in `picker.rs` tests) + new flags.
  - Theme discovery and `--preview` golden output (text mode) in CI on Linux/macOS/Windows.
- Manual tests
  - tmux/screen on Linux; macOS Terminal/iTerm2; Windows Terminal + cmd.exe; WSL.

---

**Risks & Mitigations**

- Terminal diversity causes inconsistencies
  - Mitigate via robust capability probe, conservative defaults, and explicit `--ascii` escape hatch.
- Performance regressions due to style indirection
  - Resolve tokens once per frame; keep resolver zero‑alloc; cache mapped styles.
- Windows VT quirks
  - Use `crossterm` VT enable; if it fails, force ascii/no‑color; document.
- Theme complexity overwhelms users
  - Provide opinionated presets and small, documented subset of commonly needed knobs; keep advanced tokens optional.

---

**Rollout**

- v1.1.0 (behind flags) — ship palette/glyphs, `--theme`, presets, and docs; maintain legacy theme mapping.
- v1.2.0 — add layout/templates and live reload; add `--preview`/`--dump-caps`.
- v1.3.0 — refine accessibility presets; stabilize schema; deprecate legacy `tui_theme.toml` with migration note.

---

**Success Metrics**

- >50% of users with custom themes use built‑in presets without edits.
- <1% issue rate related to rendering across supported OSes.
- Startup overhead stays <50 ms vs baseline on Linux and macOS; <70 ms on Windows Terminal.
- Zero crashes due to theme parsing in crash reports.

---

**Open Questions**

- Should we allow per‑widget overrides (e.g., different border style for dialog toasts)? Proposal: yes, but only for a few surfaces (`border_style_toast`, `status_style`).
- Do we want to expose a compact “density” preset switch (`density=compact|comfortable`) in addition to granular spacing? Proposal: yes; map to spacing defaults.
- Should we add an environment variable to pick theme (`DITOX_TUI_THEME`)? Proposal: yes; lower precedence than CLI flags.

---

**Example Snippets (schema highlights)**

- settings.toml (user)
  - `[tui] theme = "dark"; color = "auto"; alt_screen = true; mouse = false; live_reload = false`
- themes/dark.toml (partial)
  - `version = 1`
  - `[palette] fg = "#e6e1cf"; bg = "#0b0e14"; surface = "#11151c"; border_fg = "gray"; selection_bg = "#1f6feb"; selection_fg = "black"; help_fg = "yellow"`
  - `[options] border_style = "rounded"; box_chars = "unicode"; truecolor = "auto"`
- glyphs/nerdfont.toml (partial)
  - `favorite = "★"`; `image = ""`; `text = ""`; `tag = ""`; `selected = "▸"`; `unselected = " "`
- layout/compact.toml (partial)
  - `help = "hidden"`; `status = "visible"`; `search_bar_position = "top"`; `list_line_height = 1`; `wrap_preview = false`
  - `list.item.template = "{selected}{favorite} {preview} {tags} {kind}"`
  - `list.meta.template = "{created_rel} • last {last_used_rel}"`

---

**Repo Touchpoints (code refs)**

- Theme loader/scaffold: `crates/ditox-cli/src/theme.rs:1` (to be replaced by new `ui::style` + loader).
- Picker TUI init and main loop: `crates/ditox-cli/src/picker.rs:239` (alt screen, terminal init), `crates/ditox-cli/src/picker.rs:270` (theme load site), draw logic across file for tokenization.
- TUI settings struct: `crates/ditox-cli/src/config.rs:48` (extend `[tui]`).
- README TUI docs: `README.md:213` (picker), `README.md:225` (legacy theme) — to be updated.

If you want, I can proceed with glyph packs, layout templates, and live reload next, and extend the palette to the full token set above.

Notes on current implementation (2025-10-02)
- Minimal palette (4 tokens) powers borders, highlights, and help/footer.
- Environment variables honored (lowest precedence): `DITOX_TUI_THEME`, `DITOX_TUI_COLOR`, `DITOX_TUI_ASCII` — useful for wrappers; CLI flags take precedence when provided.
- Legacy `~/.config/ditox/tui_theme.toml` is still read; fields map to the new palette subset.
