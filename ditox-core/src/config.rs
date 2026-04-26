use crate::error::{DitoxError, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Expand `~` (home directory) and `$VAR` / `${VAR}` (environment variables)
/// in a path string. Returns the expanded path.
///
/// - `~` and `~/foo` → `$HOME/foo`.
/// - `~user/foo` is **not** expanded (single-user expansion only).
/// - `$VAR` and `${VAR}` are expanded from the process environment.
/// - Unknown variables are left literal (no error).
///
/// Used when resolving `Config.storage.data_dir` so users can write
/// `data_dir = "~/synced/ditox"` or
/// `data_dir = "$XDG_DATA_HOME/ditox-alt"` in their TOML.
pub fn expand_path(input: impl AsRef<str>) -> PathBuf {
    let raw = input.as_ref();
    // Handle leading ~ or ~/
    let with_tilde = if let Some(stripped) = raw.strip_prefix('~') {
        if stripped.is_empty() || stripped.starts_with('/') {
            if let Some(home) = std::env::var_os("HOME") {
                let mut s = String::from(home.to_string_lossy());
                s.push_str(stripped);
                s
            } else {
                raw.to_string()
            }
        } else {
            // ~user/... — leave alone, no per-user expansion supported.
            raw.to_string()
        }
    } else {
        raw.to_string()
    };

    // Expand $VAR and ${VAR}.
    let mut out = String::with_capacity(with_tilde.len());
    let bytes = with_tilde.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            // Try ${NAME}
            if bytes[i + 1] == b'{' {
                if let Some(close) = with_tilde[i + 2..].find('}') {
                    let name = &with_tilde[i + 2..i + 2 + close];
                    if !name.is_empty() {
                        if let Ok(val) = std::env::var(name) {
                            out.push_str(&val);
                            i = i + 2 + close + 1;
                            continue;
                        }
                    }
                }
                // Fall through: literal ${
            } else {
                // $NAME — read identifier characters
                let start = i + 1;
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > start {
                    let name = &with_tilde[start..end];
                    if let Ok(val) = std::env::var(name) {
                        out.push_str(&val);
                        i = end;
                        continue;
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    PathBuf::from(out)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    pub keybindings: KeybindingsConfig,
    pub capture: CaptureConfig,
    pub paste: PasteConfig,
    /// Phase 3 sub-task 3.8: per-clip URL templates for the
    /// "Translate" / "Search the web" actions.
    pub actions: crate::url_template::ActionsConfig,
    /// Phase 4 sub-task 4.4: GUI launcher behaviour.
    pub gui: GuiConfig,
}

/// Phase 4 sub-task 4.4 — GUI launcher behaviour.
///
/// Today's surface area:
/// - `position`: where the launcher panel appears.
/// - `hide_on_blur` + `hide_on_blur_grace_ms`: 4.8.
/// - `pinned_layer`: 4.6 (always-on-top toggle).
///
/// Phase 5 will add per-clip hotkey config; that lives in its
/// own block.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct GuiConfig {
    /// Where to anchor the launcher when shown. Defaults to
    /// bottom-left of the active monitor with a 24 px margin
    /// (matches the post-013 floating-launcher visual reference).
    pub position: GuiPosition,
    /// When `true` (default) the launcher hides on focus loss
    /// after the grace window. Set to `false` to disable
    /// auto-hide entirely (pin-by-config rather than per-summon).
    pub hide_on_blur: bool,
    /// Grace period in ms before a focus-loss event triggers
    /// hide. Some compositors emit a brief unfocus during the
    /// show animation; the grace prevents that from killing the
    /// summon. Defaults to 250 ms.
    pub hide_on_blur_grace_ms: u64,
    /// When `true`, the launcher uses the layer-shell `Overlay`
    /// layer instead of `Top` so it draws above the rest of the
    /// session including fullscreen windows. Off by default; the
    /// in-launcher pin button (sub-task 4.6) toggles this at
    /// runtime.
    pub pinned: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            position: GuiPosition::default(),
            hide_on_blur: true,
            hide_on_blur_grace_ms: 250,
            pinned: false,
        }
    }
}

impl GuiConfig {
    /// Resolved blur-grace as a `Duration`. Falls back to 250 ms
    /// when `hide_on_blur_grace_ms == 0`.
    pub fn hide_on_blur_grace(&self) -> std::time::Duration {
        if self.hide_on_blur_grace_ms == 0 {
            std::time::Duration::from_millis(250)
        } else {
            std::time::Duration::from_millis(self.hide_on_blur_grace_ms)
        }
    }
}

/// Position modes for the launcher panel. Matches
/// `docs/notes/ui-replication.md::A4`.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum GuiPosition {
    /// Bottom-left of the active monitor, 24 px margin. The
    /// post-013 default.
    #[default]
    Default,
    /// At the last-known-good geometry for the active resolution
    /// (sub-task 3.7's `window_state.json` map). Falls back to
    /// `Default` on first launch / unknown resolution.
    AtPrevious,
    /// At the current text-cursor / mouse-cursor position. On
    /// Windows the cursor is the text caret (`GetCaretPos`); on
    /// Hyprland it's the mouse cursor (`hyprctl cursorpos`). On
    /// other compositors falls back to
    /// [`AtActiveWindowCentre`](Self::AtActiveWindowCentre).
    AtCursor,
    /// Centred over the last-tracked foreground window. On
    /// platforms without a foreground tracker
    /// (`Platform::supports_layer_shell` is `false`) falls back
    /// to `Default`.
    AtActiveWindowCentre,
    /// Fixed anchor + offset. The verbose form exists so users
    /// can pin to e.g. the top-right of their primary monitor.
    Fixed {
        #[serde(default)]
        horizontal: HorizontalAnchor,
        #[serde(default)]
        vertical: VerticalAnchor,
        /// Offset from the anchor in DIPs. `[x, y]`. Positive `x`
        /// is rightward, positive `y` is downward.
        #[serde(default)]
        offset: [i32; 2],
    },
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAnchor {
    #[default]
    Left,
    Centre,
    Right,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAnchor {
    Top,
    Middle,
    #[default]
    Bottom,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralConfig {
    pub max_entries: usize,
    pub poll_interval_ms: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_entries: 500,
            poll_interval_ms: 250,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[derive(Default)]
pub struct StorageConfig {
    pub data_dir: Option<PathBuf>,
}

impl StorageConfig {
    /// Resolve `data_dir` to an absolute, expanded path if set.
    /// Returns `None` if the user didn't override.
    pub fn resolved_data_dir(&self) -> Option<PathBuf> {
        self.data_dir
            .as_ref()
            .map(|p| expand_path(p.to_string_lossy()))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct UiConfig {
    pub show_preview: bool,
    pub date_format: DateFormat,
    pub theme: ThemeConfig,
    pub graphics_protocol: Option<GraphicsProtocol>,
    /// Font size in pixels (width, height) for image rendering
    /// Example: [9, 18] for 9x18 pixel font
    pub font_size: Option<(u16, u16)>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum GraphicsProtocol {
    Kitty,
    Sixel,
    Iterm2,
    Halfblocks,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_preview: true,
            date_format: DateFormat::Relative,
            theme: ThemeConfig::default(),
            graphics_protocol: None, // Auto-detect
            font_size: None,         // Auto-detect
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum DateFormat {
    #[default]
    Relative,
    Iso,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ThemeConfig {
    pub selected: String,
    pub border: String,
    pub text: String,
    pub muted: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            selected: "#7aa2f7".to_string(),
            border: "#565f89".to_string(),
            text: "#c0caf5".to_string(),
            muted: "#565f89".to_string(),
        }
    }
}

/// Custom keybindings configuration
///
/// Format: "key" = "action"
/// Keys: "q", "ctrl+d", "alt+x", "shift+g", "enter", "esc", "tab", "space", "f1"-"f12"
/// Actions: see `Action::config_name()` for all available actions
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct KeybindingsConfig {
    /// Custom key bindings that override defaults
    /// Example: { "p" = "toggle_preview", "ctrl+x" = "delete" }
    #[serde(flatten)]
    pub bindings: HashMap<String, String>,
}

// Note: KeybindingsConfig::create_resolver() is implemented in ditox-tui
// since it depends on crossterm for key parsing

/// Multi-format clipboard capture limits and policy.
///
/// Phase 1 (sub-task 1.8). Defaults match Ditto's documented behaviour:
/// capture every format the OS publishes, drop a clip if any single
/// format or the total clip exceeds the cap (Ditto silently truncates;
/// we drop with a warning so the user notices).
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CaptureConfig {
    /// Capture mode controlling which formats are persisted.
    pub mode: CaptureMode,
    /// Per-format size cap in bytes. Formats larger than this are
    /// dropped from the clip (other formats may still land).
    pub max_format_size_bytes: u64,
    /// Per-clip total size cap in bytes (sum of all formats' bytes
    /// after canonicalisation). When exceeded, the entire clip is
    /// dropped — partial captures would silently lose information.
    pub max_clip_size_bytes: u64,
    /// Format inclusion / exclusion lists. Only consulted when
    /// `mode = "custom"`. `include` is the allowlist (empty list =
    /// allow all). `exclude` is the denylist and is applied AFTER
    /// `include` for both `mode = "all"` and `mode = "custom"`.
    pub formats: CaptureFormatsConfig,
    /// Per-app capture exclusion (Phase 3 sub-task 3.2).
    /// See [`CaptureExcludeConfig`].
    pub exclude: CaptureExcludeConfig,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::All,
            // 10 MiB. Big enough for screenshots and rich paste from
            // Word, small enough that a runaway producer can't flood
            // the DB unattended.
            max_format_size_bytes: 10 * 1024 * 1024,
            // 25 MiB. Sum of all formats. A typical multi-format clip
            // is 1-3 MiB; 25 MiB tolerates a 4K screenshot plus its
            // text-URL caption plus html/rtf without dropping.
            max_clip_size_bytes: 25 * 1024 * 1024,
            formats: CaptureFormatsConfig::default(),
            exclude: CaptureExcludeConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    /// Capture every format the OS exposes (subject to per-format and
    /// per-clip size caps and the `formats.exclude` denylist).
    #[default]
    All,
    /// v0.3.1-compatible behaviour: only `text/plain` and the
    /// canonical `image/*` are captured; everything else is ignored.
    Minimal,
    /// Capture only formats listed in `formats.include` (minus any
    /// in `formats.exclude`).
    Custom,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CaptureFormatsConfig {
    /// Allowlist for `mode = "custom"`. Empty list under
    /// `mode = "custom"` means "allow nothing" — explicit so users
    /// don't accidentally enable custom mode and capture nothing.
    pub include: Vec<String>,
    /// Denylist applied to all modes. Use this to silence a noisy
    /// internal format from a specific app.
    pub exclude: Vec<String>,
}

/// Per-app capture exclusion (Phase 3 sub-task 3.2).
///
/// When the foreground app's `process_basename` matches any of the
/// listed glob patterns at capture time, the entire clip is dropped
/// before it reaches the database. Useful for password managers that
/// briefly stage credentials on the system clipboard — KeePassXC,
/// 1Password, Bitwarden, the `pass` CLI — and for `ydotoold`, whose
/// own paste-back synthesis would otherwise feed the watcher its own
/// output.
///
/// Patterns support `*` (zero or more characters) and `?` (one
/// character); matching is **ASCII case-insensitive** (Windows
/// reports basenames as `Firefox.exe`; Linux as `firefox`).
///
/// Matching uses the foreground tracker provided by
/// [`crate::foreground::build_default_tracker`]; when no tracker is
/// available (GNOME Wayland, etc.) the exclusion list is silently
/// inactive and all clips are captured.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CaptureExcludeConfig {
    /// Glob patterns matched against the foreground app's
    /// `process_basename`. Defaults to a small built-in list
    /// covering common password managers and `ydotoold`. Override
    /// in TOML to extend or replace.
    pub processes: Vec<String>,
}

impl Default for CaptureExcludeConfig {
    fn default() -> Self {
        Self {
            // Kept short and conservative. Users in security-sensitive
            // workflows should extend this list; we err on the side of
            // capturing-by-default rather than silently dropping clips
            // from apps the user actually wants in their history.
            processes: vec![
                "*KeePass*".to_string(),
                "*1Password*".to_string(),
                "*Bitwarden*".to_string(),
                "ydotoold".to_string(),
            ],
        }
    }
}

impl CaptureExcludeConfig {
    /// Returns `true` iff `basename` matches any configured glob.
    /// ASCII-case-insensitive.
    pub fn excludes(&self, basename: &str) -> bool {
        self.processes.iter().any(|pat| glob_match(pat, basename))
    }
}

/// Match `input` against `pattern` where `*` matches zero or more
/// characters and `?` matches exactly one. ASCII case-insensitive.
///
/// Implementation is the standard two-pointer + last-star backtrack
/// algorithm (no recursion, O(p × i) worst case). Used by
/// [`CaptureExcludeConfig::excludes`] and kept module-local; if a
/// caller needs richer glob semantics they should pull in `globset`.
pub(crate) fn glob_match(pattern: &str, input: &str) -> bool {
    let p = pattern.to_ascii_lowercase();
    let i = input.to_ascii_lowercase();
    glob_match_bytes(p.as_bytes(), i.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], input: &[u8]) -> bool {
    let (mut pi, mut ii) = (0usize, 0usize);
    // Last position where we matched a `*`, plus the position in
    // `input` we resumed at. On mismatch we back up to here.
    let (mut star_p, mut star_i) = (None::<usize>, 0usize);

    while ii < input.len() {
        if pi < pattern.len() && (pattern[pi] == input[ii] || pattern[pi] == b'?') {
            pi += 1;
            ii += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star_p = Some(pi);
            star_i = ii;
            pi += 1;
        } else if let Some(sp) = star_p {
            // Backtrack: consume one more char into the last `*`.
            pi = sp + 1;
            star_i += 1;
            ii = star_i;
        } else {
            return false;
        }
    }

    // Trailing `*`s in pattern still match.
    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }
    pi == pattern.len()
}

impl CaptureConfig {
    /// Decide whether a single `format_name` should be captured given
    /// the current mode and allow/deny lists.
    ///
    /// Order of evaluation:
    /// 1. `formats.exclude` — wins over everything (explicit user
    ///    veto on a noisy format).
    /// 2. `mode = Minimal` — only canonical text/image formats.
    /// 3. `mode = Custom` — must be in `formats.include`.
    /// 4. `mode = All` — yes.
    pub fn should_capture_format(&self, format_name: &str) -> bool {
        if self.formats.exclude.iter().any(|x| x == format_name) {
            return false;
        }
        match self.mode {
            CaptureMode::All => true,
            CaptureMode::Minimal => is_minimal_format(format_name),
            CaptureMode::Custom => self.formats.include.iter().any(|x| x == format_name),
        }
    }

    /// True when an individual format's byte length is acceptable.
    pub fn format_size_ok(&self, byte_len: u64) -> bool {
        byte_len <= self.max_format_size_bytes
    }

    /// True when the cumulative byte length across all formats in a
    /// clip is acceptable.
    pub fn clip_size_ok(&self, total_byte_len: u64) -> bool {
        total_byte_len <= self.max_clip_size_bytes
    }
}

/// Phase 2 paste-back configuration.
///
/// Controls (a) per-app keystroke override, (b) explicit synthesizer
/// chain override (otherwise the platform default is used), and
/// (c) a kill-switch to disable paste-back entirely (the launcher
/// then writes the clip to the clipboard but doesn't try to
/// restore focus / synthesise keystrokes — user pastes manually).
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct PasteConfig {
    /// Disable paste-back entirely. Clipboard is still written; the
    /// user pastes manually with their own Ctrl+V. Useful when the
    /// synthesizer interacts badly with a particular workflow.
    pub disabled: bool,

    /// Override the platform-default synthesizer chain. Each string
    /// must be one of `"hyprctl"`, `"wtype"`, `"ydotool"`, `"off"`.
    /// `None` (the default) defers to
    /// [`crate::platform::Platform::paste_synthesizer_chain`].
    pub synthesizer_chain: Option<Vec<String>>,

    /// Per-app keystroke overrides keyed by `process_basename`.
    /// Lookup is **ASCII-case-insensitive** — keys are normalised to
    /// lowercase at config load. Values are parsed by
    /// [`crate::paste::keystroke::parse`] (`"ctrl+v"`, `"ctrl+shift+v"`,
    /// `"\"+gp"` for vim's register paste, etc.).
    ///
    /// Entries not in the map fall back to
    /// [`crate::paste::keystroke::DEFAULT_KEYSTROKE`].
    pub keystrokes: std::collections::HashMap<String, String>,

    /// TTL for the [`crate::paste::sentinel::PasteSentinel`]: how
    /// long after a paste-back the watcher should ignore a captured
    /// clip whose hash matches the just-pasted hash. Defaults to
    /// 2000 ms — long enough to absorb the round-trip through the
    /// compositor + synthesizer + the watcher's poll interval.
    pub sentinel_ttl_ms: u64,

    /// Re-fire window for the [`crate::paste::cursor::SelectionCursor`]:
    /// if the launcher is summoned again within this many milliseconds
    /// of the previous summon, the cursor index is `+1`'d (cycling
    /// through history); otherwise it resets to `0`. Defaults to
    /// 800 ms — comfortable double-tap of `Ctrl+Shift+V`.
    pub cursor_refire_window_ms: u64,
}

impl PasteConfig {
    /// Resolve the keystroke override for `process_basename`,
    /// falling back to `DEFAULT_KEYSTROKE` when no entry matches.
    /// ASCII-case-insensitive — `"FIREFOX"` matches a config key
    /// of `"firefox"`.
    pub fn keystroke_for(&self, process_basename: &str) -> String {
        let lower = process_basename.to_ascii_lowercase();
        self.keystrokes
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == lower)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| crate::paste::keystroke::DEFAULT_KEYSTROKE.to_string())
    }

    /// Resolved sentinel TTL as a [`std::time::Duration`].
    /// Falls back to 2 s when `sentinel_ttl_ms` is `0` (the unset
    /// default in TOML).
    pub fn sentinel_ttl(&self) -> std::time::Duration {
        if self.sentinel_ttl_ms == 0 {
            std::time::Duration::from_millis(2000)
        } else {
            std::time::Duration::from_millis(self.sentinel_ttl_ms)
        }
    }

    /// Resolved cursor re-fire window as a
    /// [`std::time::Duration`]. Falls back to
    /// [`crate::paste::cursor::DEFAULT_REFIRE_WINDOW`] (800 ms)
    /// when `cursor_refire_window_ms` is `0` (the unset default in
    /// TOML).
    pub fn cursor_refire_window(&self) -> std::time::Duration {
        if self.cursor_refire_window_ms == 0 {
            crate::paste::cursor::DEFAULT_REFIRE_WINDOW
        } else {
            std::time::Duration::from_millis(self.cursor_refire_window_ms)
        }
    }
}

/// `Minimal` mode keeps only the formats that ditox v0.3.1 captured:
/// canonical UTF-8 text and the canonical image MIME types.
fn is_minimal_format(format_name: &str) -> bool {
    matches!(
        format_name,
        "text/plain;charset=utf-8"
            | "text/plain"
            | "image/png"
            | "image/jpeg"
            | "image/gif"
            | "image/webp"
            | "image/bmp"
            | "image/tiff"
    )
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)
                .map_err(|e| DitoxError::Config(format!("Failed to parse config: {}", e)))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn get_config_path() -> Result<PathBuf> {
        ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .ok_or_else(|| DitoxError::Config("Could not determine config directory".into()))
    }

    /// Apply this config's `storage.data_dir` (if set) as the
    /// process-wide override. Call early in startup, before any
    /// `Database::open()` / `Database::get_*` resolution.
    ///
    /// Returns the resolved override if any.
    pub fn apply_storage_override(&self) -> Result<Option<PathBuf>> {
        let resolved = self.storage.resolved_data_dir();
        if let Some(ref dir) = resolved {
            crate::db::set_data_dir_override(Some(dir.clone()))?;
        }
        Ok(resolved)
    }
}

/// Path-expansion helper kept on `Config` namespace for ergonomics
/// when callers don't want to import the free function.
impl Config {
    pub fn expand_path(input: &str) -> PathBuf {
        expand_path(input)
    }

    /// Returns `true` if `candidate` is non-existent **and** the
    /// legacy default data dir contains a `ditox.db`. Used to warn
    /// about lost data when switching `storage.data_dir` after
    /// previously running on defaults.
    pub fn legacy_db_exists_outside(candidate: &Path) -> bool {
        let candidate_db = candidate.join("ditox.db");
        if candidate_db.exists() {
            return false;
        }
        ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.data_dir().join("ditox.db").exists())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod capture_config_tests {
    use super::*;

    #[test]
    fn default_capture_mode_is_all_with_generous_caps() {
        let c = CaptureConfig::default();
        assert_eq!(c.mode, CaptureMode::All);
        assert_eq!(c.max_format_size_bytes, 10 * 1024 * 1024);
        assert_eq!(c.max_clip_size_bytes, 25 * 1024 * 1024);
    }

    #[test]
    fn exclude_list_vetoes_in_all_modes() {
        let c = CaptureConfig {
            mode: CaptureMode::All,
            formats: CaptureFormatsConfig {
                include: vec![],
                exclude: vec!["application/x-vnd.foo".to_string()],
            },
            ..Default::default()
        };
        assert!(!c.should_capture_format("application/x-vnd.foo"));
        assert!(c.should_capture_format("text/plain;charset=utf-8"));
    }

    #[test]
    fn minimal_mode_drops_html_and_rtf() {
        let c = CaptureConfig {
            mode: CaptureMode::Minimal,
            ..Default::default()
        };
        assert!(c.should_capture_format("text/plain;charset=utf-8"));
        assert!(c.should_capture_format("image/png"));
        assert!(!c.should_capture_format("text/html"));
        assert!(!c.should_capture_format("text/rtf"));
        assert!(!c.should_capture_format("application/json"));
    }

    #[test]
    fn custom_mode_only_captures_listed_formats() {
        let c = CaptureConfig {
            mode: CaptureMode::Custom,
            formats: CaptureFormatsConfig {
                include: vec!["text/html".to_string(), "image/png".to_string()],
                exclude: vec![],
            },
            ..Default::default()
        };
        assert!(c.should_capture_format("text/html"));
        assert!(c.should_capture_format("image/png"));
        assert!(!c.should_capture_format("text/plain;charset=utf-8"));
        assert!(!c.should_capture_format("text/rtf"));
    }

    #[test]
    fn custom_mode_with_empty_include_captures_nothing() {
        let c = CaptureConfig {
            mode: CaptureMode::Custom,
            ..Default::default()
        };
        assert!(!c.should_capture_format("text/plain;charset=utf-8"));
        assert!(!c.should_capture_format("image/png"));
    }

    #[test]
    fn exclude_overrides_include_in_custom_mode() {
        let c = CaptureConfig {
            mode: CaptureMode::Custom,
            formats: CaptureFormatsConfig {
                include: vec!["text/html".to_string()],
                exclude: vec!["text/html".to_string()],
            },
            ..Default::default()
        };
        assert!(!c.should_capture_format("text/html"));
    }

    #[test]
    fn size_caps_are_inclusive() {
        let c = CaptureConfig::default();
        // Exactly at limit must pass.
        assert!(c.format_size_ok(c.max_format_size_bytes));
        // One byte over must fail.
        assert!(!c.format_size_ok(c.max_format_size_bytes + 1));

        assert!(c.clip_size_ok(c.max_clip_size_bytes));
        assert!(!c.clip_size_ok(c.max_clip_size_bytes + 1));
    }

    #[test]
    fn parses_from_toml() {
        let toml = r#"
            [capture]
            mode = "minimal"
            max_format_size_bytes = 5242880
            max_clip_size_bytes = 10485760

            [capture.formats]
            include = ["text/html"]
            exclude = ["application/x-vnd.bad"]
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert_eq!(parsed.capture.mode, CaptureMode::Minimal);
        assert_eq!(parsed.capture.max_format_size_bytes, 5242880);
        assert_eq!(parsed.capture.max_clip_size_bytes, 10485760);
        assert_eq!(parsed.capture.formats.include, vec!["text/html"]);
        assert_eq!(
            parsed.capture.formats.exclude,
            vec!["application/x-vnd.bad"]
        );
    }
}

#[cfg(test)]
mod glob_tests {
    use super::*;

    #[test]
    fn empty_pattern_matches_only_empty_input() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "x"));
        assert!(!glob_match("x", ""));
    }

    #[test]
    fn literal_match_is_case_insensitive() {
        assert!(glob_match("firefox", "firefox"));
        assert!(glob_match("Firefox", "firefox"));
        assert!(glob_match("FIREFOX", "firefox"));
        assert!(glob_match("firefox", "FIREFOX"));
        assert!(!glob_match("firefox", "chromium"));
    }

    #[test]
    fn star_matches_zero_chars() {
        assert!(glob_match("foo*", "foo"));
        assert!(glob_match("*foo", "foo"));
        assert!(glob_match("*foo*", "foo"));
        assert!(glob_match("*", ""));
    }

    #[test]
    fn star_matches_arbitrary_chars() {
        assert!(glob_match("*KeePass*", "org.keepassxc.KeePassXC"));
        assert!(glob_match("*KeePass*", "KeePass"));
        assert!(glob_match("*KeePass*", "MyKeePassExtra"));
        assert!(!glob_match("*KeePass*", "Firefox"));
    }

    #[test]
    fn question_matches_exactly_one_char() {
        assert!(glob_match("f?refox", "firefox"));
        assert!(glob_match("f?refox", "fArefox"));
        assert!(!glob_match("f?refox", "frefox")); // missing the char
        assert!(!glob_match("f?refox", "firrrefox")); // too many chars
    }

    #[test]
    fn double_star_does_not_explode() {
        // Naive backtrackers can blow up on `**foo`. Make sure ours
        // doesn't, both for matches and non-matches.
        assert!(glob_match("**KeePass**", "MyKeePassDB"));
        assert!(glob_match("a**b", "axxb"));
        assert!(!glob_match("a**b", "axxc"));
    }

    #[test]
    fn pattern_with_only_stars_matches_anything() {
        assert!(glob_match("***", ""));
        assert!(glob_match("***", "anything"));
        assert!(glob_match("***", "🦀")); // even unicode (lowercased per char)
    }

    #[test]
    fn complex_glob_with_mixed_wildcards() {
        // Matches "1Password 8.exe" and "1Password CLI.exe".
        assert!(glob_match("1Password*.exe", "1Password 8.exe"));
        assert!(glob_match("1Password*.exe", "1Password CLI.exe"));
        assert!(!glob_match("1Password*.exe", "Bitwarden.exe"));
    }

    #[test]
    fn special_glob_chars_match_literally_when_no_pattern() {
        // An exact-match pattern with no wildcards.
        assert!(glob_match("ydotoold", "ydotoold"));
        assert!(!glob_match("ydotoold", "ydotool"));
    }
}

#[cfg(test)]
mod capture_exclude_tests {
    use super::*;

    #[test]
    fn default_excludes_password_managers_and_ydotoold() {
        let c = CaptureExcludeConfig::default();
        assert!(c.excludes("KeePassXC"));
        assert!(c.excludes("org.keepassxc.KeePassXC"));
        assert!(c.excludes("1Password"));
        assert!(c.excludes("1Password 8.exe"));
        assert!(c.excludes("Bitwarden.exe"));
        assert!(c.excludes("ydotoold"));
    }

    #[test]
    fn default_does_not_exclude_browsers_or_editors() {
        let c = CaptureExcludeConfig::default();
        assert!(!c.excludes("firefox"));
        assert!(!c.excludes("chromium"));
        assert!(!c.excludes("brave-browser"));
        assert!(!c.excludes("nvim"));
        assert!(!c.excludes("code"));
    }

    #[test]
    fn match_is_case_insensitive() {
        let c = CaptureExcludeConfig::default();
        assert!(c.excludes("KEEPASSXC"));
        assert!(c.excludes("keepassxc"));
        assert!(c.excludes("1PASSWORD.EXE"));
    }

    #[test]
    fn empty_processes_means_nothing_excluded() {
        let c = CaptureExcludeConfig { processes: vec![] };
        assert!(!c.excludes("KeePassXC"));
        assert!(!c.excludes("1Password"));
        assert!(!c.excludes(""));
    }

    #[test]
    fn user_override_replaces_defaults() {
        let c = CaptureExcludeConfig {
            processes: vec!["my-secret-app".to_string()],
        };
        assert!(c.excludes("my-secret-app"));
        // KeePass is no longer excluded — user took explicit ownership.
        assert!(!c.excludes("KeePassXC"));
    }

    #[test]
    fn parses_from_toml() {
        let toml = r#"
            [capture.exclude]
            processes = ["*Vault*", "1pass-cli"]
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            parsed.capture.exclude.processes,
            vec!["*Vault*".to_string(), "1pass-cli".to_string()]
        );
        assert!(parsed.capture.exclude.excludes("HashiVault"));
        assert!(parsed.capture.exclude.excludes("1pass-cli"));
        assert!(!parsed.capture.exclude.excludes("KeePassXC")); // overridden
    }

    #[test]
    fn missing_capture_exclude_uses_defaults() {
        // Existing TOML without a [capture.exclude] block still picks
        // up the password-manager defaults — security-relevant default.
        let toml = r#"
            [capture]
            mode = "all"
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(parsed.capture.exclude.excludes("KeePassXC"));
        assert!(parsed.capture.exclude.excludes("1Password"));
    }
}

#[cfg(test)]
mod paste_config_tests {
    use super::*;

    #[test]
    fn default_paste_config_is_enabled_with_default_keystroke() {
        let p = PasteConfig::default();
        assert!(!p.disabled);
        assert!(p.synthesizer_chain.is_none());
        assert!(p.keystrokes.is_empty());
        // The 0-default sentinel_ttl_ms maps to 2 seconds at use-site.
        assert_eq!(p.sentinel_ttl(), std::time::Duration::from_secs(2));
        // The 0-default cursor_refire_window_ms maps to 800 ms.
        assert_eq!(
            p.cursor_refire_window(),
            crate::paste::cursor::DEFAULT_REFIRE_WINDOW
        );
    }

    #[test]
    fn keystroke_for_unknown_basename_is_default() {
        let p = PasteConfig::default();
        assert_eq!(
            p.keystroke_for("notepad.exe"),
            crate::paste::keystroke::DEFAULT_KEYSTROKE
        );
    }

    #[test]
    fn keystroke_for_known_basename_returns_override() {
        let mut p = PasteConfig::default();
        p.keystrokes.insert("gvim".to_string(), "\"+gp".to_string());
        assert_eq!(p.keystroke_for("gvim"), "\"+gp");
    }

    #[test]
    fn keystroke_for_match_is_case_insensitive() {
        // Windows reports basenames with mixed case
        // (`Firefox.exe`); match the user's lowercase config key.
        let mut p = PasteConfig::default();
        p.keystrokes
            .insert("firefox.exe".to_string(), "ctrl+v".to_string());
        assert_eq!(p.keystroke_for("FIREFOX.EXE"), "ctrl+v");
        assert_eq!(p.keystroke_for("Firefox.Exe"), "ctrl+v");
    }

    #[test]
    fn sentinel_ttl_uses_explicit_value() {
        let p = PasteConfig {
            sentinel_ttl_ms: 5000,
            ..PasteConfig::default()
        };
        assert_eq!(p.sentinel_ttl(), std::time::Duration::from_millis(5000));
    }

    #[test]
    fn cursor_refire_window_uses_explicit_value() {
        let p = PasteConfig {
            cursor_refire_window_ms: 1500,
            ..PasteConfig::default()
        };
        assert_eq!(
            p.cursor_refire_window(),
            std::time::Duration::from_millis(1500)
        );
    }

    #[test]
    fn paste_config_toml_round_trip() {
        let toml = r#"
            [paste]
            disabled = false
            synthesizer_chain = ["wtype", "off"]
            sentinel_ttl_ms = 1500
            cursor_refire_window_ms = 1200

            [paste.keystrokes]
            "gvim" = "\"+gp"
            "firefox.exe" = "ctrl+v"
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(!parsed.paste.disabled);
        assert_eq!(
            parsed.paste.synthesizer_chain.as_deref(),
            Some(&vec!["wtype".to_string(), "off".to_string()][..])
        );
        assert_eq!(parsed.paste.sentinel_ttl_ms, 1500);
        assert_eq!(parsed.paste.cursor_refire_window_ms, 1200);
        assert_eq!(
            parsed.paste.cursor_refire_window(),
            std::time::Duration::from_millis(1200)
        );
        assert_eq!(parsed.paste.keystroke_for("gvim"), "\"+gp");
        assert_eq!(parsed.paste.keystroke_for("firefox.exe"), "ctrl+v");
    }

    #[test]
    fn paste_section_is_optional_in_toml() {
        // Existing config files without a [paste] section must still
        // parse; everything defaults.
        let toml = r#"
            [general]
            max_entries = 1000
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(!parsed.paste.disabled);
        assert!(parsed.paste.keystrokes.is_empty());
    }
}

#[cfg(test)]
mod gui_config_tests {
    use super::*;

    #[test]
    fn default_gui_config_has_sensible_values() {
        let g = GuiConfig::default();
        assert!(matches!(g.position, GuiPosition::Default));
        assert!(g.hide_on_blur);
        assert_eq!(g.hide_on_blur_grace_ms, 250);
        assert!(!g.pinned);
    }

    #[test]
    fn hide_on_blur_grace_falls_back_to_250_on_zero() {
        let g = GuiConfig {
            hide_on_blur_grace_ms: 0,
            ..GuiConfig::default()
        };
        assert_eq!(g.hide_on_blur_grace().as_millis(), 250);
    }

    #[test]
    fn hide_on_blur_grace_uses_explicit_value() {
        let g = GuiConfig {
            hide_on_blur_grace_ms: 1000,
            ..GuiConfig::default()
        };
        assert_eq!(g.hide_on_blur_grace().as_millis(), 1000);
    }

    #[test]
    fn parses_default_position_from_toml() {
        let toml = r#"
            [gui]
            [gui.position]
            mode = "default"
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(matches!(parsed.gui.position, GuiPosition::Default));
    }

    #[test]
    fn parses_at_previous_position() {
        let toml = r#"
            [gui.position]
            mode = "at_previous"
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(matches!(parsed.gui.position, GuiPosition::AtPrevious));
    }

    #[test]
    fn parses_at_cursor_position() {
        let toml = r#"
            [gui.position]
            mode = "at_cursor"
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(matches!(parsed.gui.position, GuiPosition::AtCursor));
    }

    #[test]
    fn parses_fixed_position() {
        let toml = r#"
            [gui.position]
            mode = "fixed"
            horizontal = "right"
            vertical = "top"
            offset = [-20, 40]
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        match parsed.gui.position {
            GuiPosition::Fixed {
                horizontal,
                vertical,
                offset,
            } => {
                assert_eq!(horizontal, HorizontalAnchor::Right);
                assert_eq!(vertical, VerticalAnchor::Top);
                assert_eq!(offset, [-20, 40]);
            }
            other => panic!("expected Fixed, got {:?}", other),
        }
    }

    #[test]
    fn parses_fixed_with_default_anchors() {
        let toml = r#"
            [gui.position]
            mode = "fixed"
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        match parsed.gui.position {
            GuiPosition::Fixed {
                horizontal,
                vertical,
                offset,
            } => {
                assert_eq!(horizontal, HorizontalAnchor::Left);
                assert_eq!(vertical, VerticalAnchor::Bottom);
                assert_eq!(offset, [0, 0]);
            }
            other => panic!("expected Fixed, got {:?}", other),
        }
    }

    #[test]
    fn parses_pinned_and_hide_on_blur_overrides() {
        let toml = r#"
            [gui]
            hide_on_blur = false
            hide_on_blur_grace_ms = 500
            pinned = true
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(!parsed.gui.hide_on_blur);
        assert_eq!(parsed.gui.hide_on_blur_grace().as_millis(), 500);
        assert!(parsed.gui.pinned);
    }

    #[test]
    fn missing_gui_section_uses_defaults() {
        let toml = r#"
            [general]
            max_entries = 1000
        "#;
        let parsed: Config = toml::from_str(toml).unwrap();
        assert!(matches!(parsed.gui.position, GuiPosition::Default));
        assert!(parsed.gui.hide_on_blur);
    }
}
