//! Ditox iced GUI application - Modern redesign

use ditox_core::app::TabFilter;
use ditox_core::foreground::{ForegroundSnapshot, ForegroundTracker};
use ditox_core::paste::keystroke::KeystrokeSequence;
use ditox_core::paste::sentinel::PasteSentinel;
use ditox_core::paste::synthesize::{paste_with_chain, Synthesizer};
// Phase 4 sub-task 4.3: do NOT import `ditox_core::Result` here. The
// `iced_layershell::to_layer_message` macro expands a `TryInto` impl
// that uses bare `Result<T, E>` which must resolve to `std::result::Result`
// (the 2-generic version). Importing `ditox_core::Result` (which is
// `Result<T, DitoxError>` with a single generic) shadows std's and
// breaks macro expansion. The one site that wanted `ditox_core::Result`
// (`run_with`) qualifies it explicitly below.
use ditox_core::{
    Clipboard, Collection, Config, Database, DbHandle, Entry, EntryType, Tag, Watcher,
};
#[cfg(windows)]
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use iced::widget::scrollable::RelativeOffset;
use iced::widget::Id as WidgetId;
use iced::widget::{
    button, column, container, image as iced_image, mouse_area, operation, row, scrollable, text,
    text_input, tooltip, Column, Row, Space,
};
use iced::window::Direction;
use iced::{
    event, keyboard, window, ContentFit, Element, Font, Length, Point, Size, Subscription, Task,
    Theme,
};

// Bootstrap Icons font
const ICONS: Font = Font::with_name("bootstrap-icons");
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

/// Page size for entry list
const PAGE_SIZE: usize = 20;

/// Search input ID for focus management
fn search_input_id() -> WidgetId {
    WidgetId::new("search_input")
}

/// Scrollable ID for entry list (for programmatic scrolling)
const ENTRY_LIST_ID: &str = "entry_list";

/// Estimated height of each entry row in pixels
const ENTRY_ROW_HEIGHT: f32 = 56.0;

fn non_empty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn open_path_external(path: &str) {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(path).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();

    if let Err(e) = result {
        tracing::warn!("failed to open image externally: {e}");
    }
}

fn config_mtime(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn gui_key_combo(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<String> {
    let mut parts = Vec::new();
    if modifiers.control() {
        parts.push("ctrl".to_string());
    }
    if modifiers.alt() {
        parts.push("alt".to_string());
    }
    if modifiers.shift() {
        parts.push("shift".to_string());
    }
    let key = match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::Escape) => "esc".to_string(),
        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => "up".to_string(),
        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => "down".to_string(),
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => "left".to_string(),
        keyboard::Key::Named(keyboard::key::Named::ArrowRight) => "right".to_string(),
        keyboard::Key::Named(keyboard::key::Named::Enter) => "enter".to_string(),
        keyboard::Key::Named(keyboard::key::Named::Tab) => "tab".to_string(),
        keyboard::Key::Character(c) => c.to_lowercase(),
        _ => return None,
    };
    parts.push(key);
    Some(parts.join("+"))
}

fn gui_action_message(action: &str) -> Option<Message> {
    match action {
        "hide" | "close" | "escape" => Some(Message::HideWindow),
        "move_up" | "up" => Some(Message::MoveUp),
        "move_down" | "down" => Some(Message::MoveDown),
        "copy" | "copy_selected" => Some(Message::CopySelected),
        "prev_page" => Some(Message::PrevPage),
        "next_page" => Some(Message::NextPage),
        "prev_tab" => Some(Message::PrevTab),
        "next_tab" => Some(Message::NextTab),
        "preview" | "toggle_preview" | "entry_panel" => Some(Message::ToggleSelectedEntryPanel),
        "help" | "toggle_help" => Some(Message::ToggleHelp),
        "multi_select" | "toggle_multi_select" => Some(Message::ToggleMultiSelect),
        _ => None,
    }
}

/// Scroll to make the selected item visible
/// Returns a Task that snaps the scrollable to the appropriate position
fn scroll_to_selected(selected_index: usize, total_entries: usize) -> Task<Message> {
    if total_entries == 0 {
        return Task::none();
    }
    // Calculate the relative position (0.0 = top, 1.0 = bottom)
    let relative_pos = selected_index as f32 / (total_entries.saturating_sub(1).max(1)) as f32;
    operation::snap_to(
        ENTRY_LIST_ID,
        RelativeOffset {
            x: 0.0,
            y: relative_pos,
        },
    )
}

/// Check if the selected item is visible and scroll only if needed
/// Returns a Task to scroll if the item is outside the visible area
fn scroll_if_needed(
    selected_index: usize,
    total_entries: usize,
    viewport: Option<&scrollable::Viewport>,
    direction: i32, // -1 for up, 1 for down
) -> Task<Message> {
    if total_entries == 0 {
        return Task::none();
    }

    // If we don't have viewport info yet, use simple scroll
    let Some(vp) = viewport else {
        return scroll_to_selected(selected_index, total_entries);
    };

    let viewport_height = vp.bounds().height;
    let content_height = vp.content_bounds().height;
    let scroll_offset = vp.absolute_offset().y;

    // Calculate the position of the selected item
    let item_top = selected_index as f32 * ENTRY_ROW_HEIGHT;
    let item_bottom = item_top + ENTRY_ROW_HEIGHT;

    // Check if item is visible
    let visible_top = scroll_offset;
    let visible_bottom = scroll_offset + viewport_height;

    if direction < 0 && item_top < visible_top {
        // Scrolling up and item is above visible area - scroll to show it at top
        let target_offset = item_top.max(0.0);
        let relative_y = if content_height > viewport_height {
            target_offset / (content_height - viewport_height)
        } else {
            0.0
        };
        return operation::snap_to(
            ENTRY_LIST_ID,
            RelativeOffset {
                x: 0.0,
                y: relative_y,
            },
        );
    } else if direction > 0 && item_bottom > visible_bottom {
        // Scrolling down and item is below visible area - scroll to show it at bottom
        let target_offset = (item_bottom - viewport_height).max(0.0);
        let relative_y = if content_height > viewport_height {
            target_offset / (content_height - viewport_height)
        } else {
            0.0
        };
        return operation::snap_to(
            ENTRY_LIST_ID,
            RelativeOffset {
                x: 0.0,
                y: relative_y,
            },
        );
    }

    Task::none()
}

/// Delay before focusing search input (to avoid capturing the hotkey's "v")
const FOCUS_DELAY_MS: u64 = 250;

/// Create a delayed focus task
fn delayed_focus_search() -> Task<Message> {
    Task::perform(
        async {
            tokio::time::sleep(Duration::from_millis(FOCUS_DELAY_MS)).await;
        },
        |_| Message::FocusSearch,
    )
}

/// Default window size
const DEFAULT_WINDOW_SIZE: Size = Size::new(420.0, 520.0);

/// Margin (px) between the floating window and the screen edges.
const FLOATING_MARGIN: f32 = 20.0;
/// Minimum window size
const MIN_WINDOW_SIZE: Size = Size::new(320.0, 300.0);

#[cfg(windows)]
#[allow(dead_code)]
fn force_restore_window(width: u32, height: u32) {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowLongW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, SetWindowPos,
        ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, SW_SHOWNORMAL,
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_SIZEBOX,
    };

    struct CallbackData {
        target_pid: u32,
        main_hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut CallbackData);
        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

        if window_pid == data.target_pid {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

            let is_tool_window = (ex_style & WS_EX_TOOLWINDOW.0) != 0;
            let is_no_activate = (ex_style & WS_EX_NOACTIVATE.0) != 0;
            let has_sizebox = (style & WS_SIZEBOX.0) != 0;

            if is_tool_window || is_no_activate || !has_sizebox {
                return BOOL(1);
            }

            data.main_hwnd = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    unsafe {
        let pid = GetCurrentProcessId();
        let mut data = CallbackData {
            target_pid: pid,
            main_hwnd: None,
        };
        let _ = EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

        if let Some(hwnd) = data.main_hwnd {
            // Check current state
            let is_iconic = IsIconic(hwnd).as_bool();
            let is_visible = IsWindowVisible(hwnd).as_bool();
            tracing::info!(
                "force_restore_window: Found window {:?}, iconic={}, visible={}",
                hwnd,
                is_iconic,
                is_visible
            );

            // 1. Prepare for focus stealing
            let current_thread_id = GetCurrentThreadId();
            let fg_hwnd = GetForegroundWindow();
            let mut fg_thread_id: u32 = 0;
            if !fg_hwnd.0.is_null() {
                fg_thread_id = GetWindowThreadProcessId(fg_hwnd, None);
            }

            let mut attached = false;
            // Only attach if foreground thread is different and valid
            if fg_thread_id != 0 && fg_thread_id != current_thread_id {
                // Try System::Threading
                let _ = windows::Win32::System::Threading::AttachThreadInput(
                    current_thread_id,
                    fg_thread_id,
                    true,
                );

                attached = true;
                tracing::info!("force_restore_window: Attempted AttachThreadInput");
            }

            // 2. Disable foreground lock timeout
            let mut original_timeout: u32 = 0;
            // SPI_GETFOREGROUNDLOCKTIMEOUT
            let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                windows::Win32::UI::WindowsAndMessaging::SPI_GETFOREGROUNDLOCKTIMEOUT,
                0,
                Some(&mut original_timeout as *mut _ as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
            // SPI_SETFOREGROUNDLOCKTIMEOUT to 0
            let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                windows::Win32::UI::WindowsAndMessaging::SPI_SETFOREGROUNDLOCKTIMEOUT,
                0,
                Some(0 as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );

            // 3. Restore window state
            // Win+D minimizes windows - we need to restore them
            // Check position too - sometimes IsIconic is false but window is at -32000
            let mut rect = windows::Win32::Foundation::RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
            let is_offscreen = rect.left <= -30000; // -32000 is typical minimized coord

            if is_iconic || is_offscreen {
                tracing::info!("force_restore_window: Window is minimized/offscreen (iconic={}, left={}), restoring...", is_iconic, rect.left);
                let _ = ShowWindow(hwnd, SW_RESTORE);
            } else {
                // Even if not iconic, ensure we are visible
                let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            }

            // Make TOPMOST to ensure it's above everything including desktop
            // Check if still offscreen after restore attempts
            let mut rect = windows::Win32::Foundation::RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
            let still_offscreen = rect.left <= -30000;

            let mut flags = SWP_SHOWWINDOW | SWP_NOSIZE;
            let mut x = 0;
            let mut y = 0;

            if still_offscreen {
                tracing::warn!("force_restore_window: Window still offscreen at {}, forcing move to 100,100 with size {}x{}", rect.left, width, height);
                // NOT adding SWP_NOMOVE
                x = 100;
                y = 100;
                // Force size restore
                flags = SWP_SHOWWINDOW; // Reset flags to remove NOSIZE if it was there
            } else {
                flags |= SWP_NOMOVE; // Only preserve position if ON SCREEN
            }

            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width as i32,
                height as i32,
                flags,
            );

            // 4. Force focus
            let _ = BringWindowToTop(hwnd);
            let fg_result = SetForegroundWindow(hwnd);
            tracing::info!(
                "force_restore_window: SetForegroundWindow = {:?}",
                fg_result
            );

            // 5. Restore settings
            // Restore foreground lock timeout
            let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                windows::Win32::UI::WindowsAndMessaging::SPI_SETFOREGROUNDLOCKTIMEOUT,
                0,
                Some(original_timeout as usize as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );

            // Detach input
            if attached {
                let _ = windows::Win32::System::Threading::AttachThreadInput(
                    current_thread_id,
                    fg_thread_id,
                    false,
                );
            }

            // Remove TOPMOST after a brief moment
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );

            // Final state check
            let is_iconic = IsIconic(hwnd).as_bool();
            let is_visible = IsWindowVisible(hwnd).as_bool();
            let fg = GetForegroundWindow();
            let is_fg = fg == hwnd;
            tracing::info!(
                "force_restore_window: Done. iconic={}, visible={}, foreground={}",
                is_iconic,
                is_visible,
                is_fg
            );
        } else {
            tracing::warn!("force_restore_window: No main window found!");
        }
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn force_restore_window(_width: u32, _height: u32) {
    // No-op on non-Windows platforms
}

/// Remove TOPMOST flag from our window (called when hiding)
#[cfg(windows)]
#[allow(dead_code)]
fn remove_topmost() {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongW, GetWindowThreadProcessId, SetWindowPos, GWL_EXSTYLE,
        GWL_STYLE, HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_SIZEBOX,
    };

    struct CallbackData {
        target_pid: u32,
        main_hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut CallbackData);
        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

        if window_pid == data.target_pid {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let is_tool_window = (ex_style & WS_EX_TOOLWINDOW.0) != 0;
            let is_no_activate = (ex_style & WS_EX_NOACTIVATE.0) != 0;
            let has_sizebox = (style & WS_SIZEBOX.0) != 0;

            if is_tool_window || is_no_activate || !has_sizebox {
                return BOOL(1);
            }
            data.main_hwnd = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    unsafe {
        let pid = GetCurrentProcessId();
        let mut data = CallbackData {
            target_pid: pid,
            main_hwnd: None,
        };
        let _ = EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

        if let Some(hwnd) = data.main_hwnd {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            tracing::info!("remove_topmost: Removed TOPMOST flag");
        }
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn remove_topmost() {
    // No-op on non-Windows platforms
}

/// Check if our main window is actually visible at Win32 level
/// This helps detect when Win+D has hidden us but our visible flag is still true
#[cfg(windows)]
#[allow(dead_code)]
fn is_window_actually_visible() -> bool {
    use std::process;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowLongW, GetWindowThreadProcessId,
        IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_SIZEBOX,
    };

    struct CallbackData {
        target_pid: u32,
        found_visible: bool,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut CallbackData);
        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

        if window_pid == data.target_pid {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let is_tool_window = (ex_style & WS_EX_TOOLWINDOW.0) != 0;
            let is_no_activate = (ex_style & WS_EX_NOACTIVATE.0) != 0;
            let has_sizebox = (style & WS_SIZEBOX.0) != 0;

            // Only check main window (with sizebox)
            if is_tool_window || is_no_activate || !has_sizebox {
                return BOOL(1);
            }

            // Check if main window is visible
            if IsWindowVisible(hwnd).as_bool() {
                data.found_visible = true;
                return BOOL(0); // Stop enumeration
            }
        }
        BOOL(1)
    }

    unsafe {
        let pid = process::id();
        let mut data = CallbackData {
            target_pid: pid,
            found_visible: false,
        };
        let _ = EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

        // Also check if we're the foreground window
        let fg = GetForegroundWindow();
        let mut fg_pid: u32 = 0;
        if !fg.0.is_null() {
            GetWindowThreadProcessId(fg, Some(&mut fg_pid));
        }
        let is_foreground = fg_pid == pid;

        tracing::info!(
            "is_window_actually_visible: visible={}, is_foreground={}",
            data.found_visible,
            is_foreground
        );

        // Consider visible only if both visible AND we have foreground
        data.found_visible && is_foreground
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn is_window_actually_visible() -> bool {
    true // Assume visible on non-Windows
}

/// In-memory window state. The shape is identical to the pre-Phase-3.7
/// flat struct so the 11 read sites elsewhere in `DitoxApp` (which
/// access `self.window_state.x` / `.y` / `.width` / `.height`
/// directly) keep compiling without churn.
///
/// Phase 3 sub-task 3.7 changed only the **on-disk** representation
/// to a multi-key map keyed by monitor resolution
/// (`<width>x<height>`); see [`WindowStateFile`]. `WindowState::load`
/// picks the best matching geometry for the current monitor, and
/// `WindowState::save` upserts under the current monitor's key.
///
/// Old-format `{x, y, width, height}` JSON files are detected and
/// migrated transparently into the new format on first load.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            width: DEFAULT_WINDOW_SIZE.width,
            height: DEFAULT_WINDOW_SIZE.height,
        }
    }
}

/// On-disk persistence record. New format (Phase 3.7):
///
/// ```jsonc
/// {
///   "version": 2,
///   "geometries": {
///     "1920x1080": { "x": ..., "y": ..., "width": ..., "height": ...,
///                    "last_used": "2026-04-26T19:00:00Z" }
///   },
///   "last_resolution_key": "1920x1080"
/// }
/// ```
///
/// On load:
/// 1. If the current monitor key is known and present → use that.
/// 2. Else if `last_resolution_key` is present → use that (most
///    recent monitor).
/// 3. Else fall through to the first available geometry.
/// 4. Else default.
///
/// Phase 4 will extend the resolution key with the monitor model +
/// serial (e.g. `"1920x1080@DP-1:LG_DISPLAY_ABC"`) once a proper
/// per-event monitor tracker lands; the file format is stable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct WindowStateFile {
    /// Schema version. Bumped to `2` for the multi-key format.
    /// Old-format files (no `version` field) are detected via the
    /// `WindowStateLegacy` shape and migrated.
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    geometries: std::collections::HashMap<String, PersistedGeometry>,
    #[serde(default)]
    last_resolution_key: Option<String>,
}

fn default_version() -> u32 {
    2
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedGeometry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    /// ISO-8601 timestamp of the last save. Useful for debugging
    /// and for the LRU fallback path.
    #[serde(default)]
    last_used: String,
}

impl PersistedGeometry {
    fn from_state(state: &WindowState) -> Self {
        Self {
            x: state.x,
            y: state.y,
            width: state.width,
            height: state.height,
            last_used: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn to_state(&self) -> WindowState {
        WindowState {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

/// Old-format file shape: a single flat `{x, y, width, height}`.
/// Detected via JSON probe — if the parsed file lacks a `version`
/// field but does have `x`/`y`/`width`/`height`, it's the legacy
/// format and gets migrated under the
/// [`LEGACY_RESOLUTION_KEY`] key.
#[derive(Debug, Clone, serde::Deserialize)]
struct WindowStateLegacy {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Synthetic key used when migrating a legacy single-geometry file
/// or when [`current_monitor_key`] returns `None` (the
/// `Position::SpecificWith` callback hasn't run yet, or this is the
/// first save before any iced event).
const LEGACY_RESOLUTION_KEY: &str = "legacy";

/// Captured at window creation by the `Position::SpecificWith`
/// callback. Format `"<width>x<height>"`. Set once per process;
/// reads are atomic-string clones.
static MONITOR_KEY: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Tracks which key the most-recent `WindowState::load` resolved.
/// `WindowState::save` re-uses it when [`MONITOR_KEY`] hasn't been
/// set yet — avoids forcing every save to fall back to
/// `LEGACY_RESOLUTION_KEY` just because iced hasn't rendered yet.
static LAST_LOADED_KEY: std::sync::OnceLock<std::sync::Mutex<Option<String>>> =
    std::sync::OnceLock::new();

fn last_loaded_key_mutex() -> &'static std::sync::Mutex<Option<String>> {
    LAST_LOADED_KEY.get_or_init(|| std::sync::Mutex::new(None))
}

/// Detect the shape of a persisted `window_state.json` and return
/// a normalised `WindowStateFile`. Order:
///
/// 1. If the JSON has a top-level `"geometries"` key → parse as new
///    format directly.
/// 2. Else if it has top-level `"x"`, `"y"`, `"width"`, `"height"`
///    (all four numeric) → parse as legacy and migrate under
///    [`LEGACY_RESOLUTION_KEY`].
/// 3. Else → `None` (corrupt / unknown shape).
///
/// We can't rely on parser-order ("try new, fall back to legacy")
/// because `WindowStateFile` accepts every legacy field as
/// `#[serde(default)]`-absent, silently losing the user's geometry.
fn parse_persisted_shape(content: &str) -> Option<WindowStateFile> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = value.as_object()?;

    if obj.contains_key("geometries") {
        // New format. Defer to the typed parser.
        return serde_json::from_str(content).ok();
    }

    let has_legacy_fields = ["x", "y", "width", "height"]
        .iter()
        .all(|k| obj.get(*k).is_some_and(|v| v.is_number()));

    if has_legacy_fields {
        let legacy: WindowStateLegacy = serde_json::from_value(value).ok()?;
        tracing::info!("migrating legacy single-geometry window_state.json to multi-key format");
        let mut geometries = std::collections::HashMap::new();
        geometries.insert(
            LEGACY_RESOLUTION_KEY.to_string(),
            PersistedGeometry {
                x: legacy.x,
                y: legacy.y,
                width: legacy.width,
                height: legacy.height,
                last_used: chrono::Utc::now().to_rfc3339(),
            },
        );
        return Some(WindowStateFile {
            version: 2,
            geometries,
            last_resolution_key: Some(LEGACY_RESOLUTION_KEY.to_string()),
        });
    }

    None
}

/// Build a resolution key from a monitor size pair. Public so
/// `Position::SpecificWith` can call it.
pub(crate) fn make_resolution_key(width: f32, height: f32) -> String {
    format!("{}x{}", width as i32, height as i32)
}

/// Set the current monitor resolution key. Called from the iced
/// `Position::SpecificWith` callback once the monitor size is
/// known. Idempotent (subsequent calls do nothing).
pub(crate) fn set_current_monitor_key(width: f32, height: f32) {
    let _ = MONITOR_KEY.set(make_resolution_key(width, height));
}

/// Best guess at the current monitor's resolution key. Resolution
/// order:
/// 1. Captured monitor size from `Position::SpecificWith`.
/// 2. Last successfully-loaded key (per-process state).
/// 3. `None` — caller falls back to [`LEGACY_RESOLUTION_KEY`].
fn current_monitor_key() -> Option<String> {
    if let Some(k) = MONITOR_KEY.get() {
        return Some(k.clone());
    }
    last_loaded_key_mutex().lock().ok().and_then(|g| g.clone())
}

impl WindowState {
    fn state_file_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.data_dir().join("window_state.json"))
    }

    /// Read the persisted file, normalise legacy formats, pick the
    /// geometry best matching the current monitor, and return as a
    /// flat `WindowState`. Returns the default geometry when no
    /// file exists or every geometry fails validation.
    pub fn load() -> Self {
        let Some(path) = Self::state_file_path() else {
            return Self::default();
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return Self::default();
        };

        // Detect the shape via serde_json::Value rather than relying
        // on parser order: every field of `WindowStateFile` is
        // `#[serde(default)]`, so the new-format parser would happily
        // accept a legacy `{x,y,w,h}` file as an empty new-format
        // file (geometries map empty) — losing the user's saved
        // position silently.
        let file: WindowStateFile = match parse_persisted_shape(&content) {
            Some(f) => f,
            None => {
                tracing::warn!("window_state.json unrecognised shape; using defaults");
                return Self::default();
            }
        };

        // Pick the geometry: current key → last_resolution_key → first.
        let chosen_key = current_monitor_key()
            .filter(|k| file.geometries.contains_key(k))
            .or_else(|| {
                file.last_resolution_key
                    .as_ref()
                    .filter(|k| file.geometries.contains_key(*k))
                    .cloned()
            })
            .or_else(|| file.geometries.keys().next().cloned());

        let Some(key) = chosen_key else {
            return Self::default();
        };

        // Remember which key we loaded so save() can reuse it when
        // the monitor key hasn't been captured yet.
        if let Ok(mut guard) = last_loaded_key_mutex().lock() {
            *guard = Some(key.clone());
        }

        let geom = file
            .geometries
            .get(&key)
            .expect("key was selected from this map");
        let state = geom.to_state();

        if state.x < -1000.0
            || state.y < -1000.0
            || state.width < MIN_WINDOW_SIZE.width
            || state.height < MIN_WINDOW_SIZE.height
        {
            tracing::warn!(
                "invalid window state in key '{}' ({}, {}) {}x{}, using defaults",
                key,
                state.x,
                state.y,
                state.width,
                state.height
            );
            return Self::default();
        }
        state
    }

    pub fn save(&self) {
        if self.x < -1000.0
            || self.y < -1000.0
            || self.width < MIN_WINDOW_SIZE.width
            || self.height < MIN_WINDOW_SIZE.height
        {
            return;
        }

        let Some(path) = Self::state_file_path() else {
            return;
        };

        // Read-modify-write via the shared shape-detect helper so a
        // save under one resolution doesn't drop entries for the
        // others.
        let mut file: WindowStateFile = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| parse_persisted_shape(&content))
            .unwrap_or_default();
        if file.version == 0 {
            file.version = 2;
        }

        let key = current_monitor_key().unwrap_or_else(|| LEGACY_RESOLUTION_KEY.to_string());
        file.geometries
            .insert(key.clone(), PersistedGeometry::from_state(self));
        file.last_resolution_key = Some(key);

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&file) {
            let _ = std::fs::write(&path, content);
        }
    }
}

#[cfg(test)]
mod window_state_tests {
    use super::*;

    fn parse_legacy(raw: &str) -> Option<WindowStateLegacy> {
        serde_json::from_str(raw).ok()
    }

    fn parse_new(raw: &str) -> Option<WindowStateFile> {
        serde_json::from_str(raw).ok()
    }

    #[test]
    fn legacy_format_round_trips_through_legacy_parser() {
        let raw = r#"{"x":100.0,"y":200.0,"width":420.0,"height":520.0}"#;
        let legacy = parse_legacy(raw).expect("legacy parse");
        assert_eq!(legacy.x, 100.0);
        assert_eq!(legacy.y, 200.0);
        assert_eq!(legacy.width, 420.0);
        assert_eq!(legacy.height, 520.0);
    }

    #[test]
    fn new_format_parses_with_geometries_map() {
        let raw = r#"{
            "version": 2,
            "geometries": {
                "1920x1080": {
                    "x": 100.0, "y": 200.0,
                    "width": 420.0, "height": 520.0,
                    "last_used": "2026-04-26T19:00:00Z"
                }
            },
            "last_resolution_key": "1920x1080"
        }"#;
        let f = parse_new(raw).expect("new parse");
        assert_eq!(f.version, 2);
        assert!(f.geometries.contains_key("1920x1080"));
        assert_eq!(f.last_resolution_key.as_deref(), Some("1920x1080"));
    }

    #[test]
    fn parse_persisted_shape_detects_new_format() {
        let raw = r#"{
            "version": 2,
            "geometries": {
                "1920x1080": { "x": 1.0, "y": 2.0, "width": 420.0, "height": 520.0, "last_used": "" }
            },
            "last_resolution_key": "1920x1080"
        }"#;
        let f = parse_persisted_shape(raw).expect("new shape");
        assert!(f.geometries.contains_key("1920x1080"));
    }

    #[test]
    fn parse_persisted_shape_migrates_legacy_format() {
        // The bug: legacy-format JSON would parse as an empty
        // new-format file because every new-format field is
        // serde-default-absent. Probe-based detection fixes this.
        let raw = r#"{"x":150.0,"y":250.0,"width":420.0,"height":520.0}"#;
        let f = parse_persisted_shape(raw).expect("legacy migration");
        assert_eq!(f.version, 2);
        assert_eq!(
            f.last_resolution_key.as_deref(),
            Some(LEGACY_RESOLUTION_KEY)
        );
        let g = f
            .geometries
            .get(LEGACY_RESOLUTION_KEY)
            .expect("legacy entry");
        assert_eq!(g.x, 150.0);
        assert_eq!(g.y, 250.0);
        assert_eq!(g.width, 420.0);
        assert_eq!(g.height, 520.0);
    }

    #[test]
    fn parse_persisted_shape_rejects_unknown_object() {
        let raw = r#"{"foo": "bar"}"#;
        assert!(parse_persisted_shape(raw).is_none());
    }

    #[test]
    fn parse_persisted_shape_rejects_partial_legacy() {
        // Missing one of x/y/width/height → unknown shape.
        let raw = r#"{"x":1.0,"y":2.0,"width":420.0}"#;
        assert!(parse_persisted_shape(raw).is_none());
    }

    #[test]
    fn parse_persisted_shape_rejects_corrupt_json() {
        assert!(parse_persisted_shape("not json").is_none());
        assert!(parse_persisted_shape("[]").is_none());
        assert!(parse_persisted_shape("42").is_none());
    }

    #[test]
    fn make_resolution_key_format() {
        assert_eq!(make_resolution_key(1920.0, 1080.0), "1920x1080");
        assert_eq!(make_resolution_key(3840.0, 2160.0), "3840x2160");
        // Float truncation: 1919.5 → 1919 (truncates toward zero).
        assert_eq!(make_resolution_key(1919.5, 1080.0), "1919x1080");
    }

    #[test]
    fn persisted_geometry_round_trips_to_state() {
        let pg = PersistedGeometry {
            x: 100.0,
            y: 200.0,
            width: 420.0,
            height: 520.0,
            last_used: "2026-04-26T19:00:00Z".to_string(),
        };
        let s = pg.to_state();
        assert_eq!(s.x, 100.0);
        assert_eq!(s.width, 420.0);
        let pg2 = PersistedGeometry::from_state(&s);
        assert_eq!(pg2.x, 100.0);
        assert_eq!(pg2.width, 420.0);
        // last_used regenerated to "now" — just check format.
        assert!(
            pg2.last_used.contains('T'),
            "last_used must be ISO-ish: {}",
            pg2.last_used
        );
    }

    #[test]
    fn missing_geometries_returns_empty_default() {
        let f = WindowStateFile::default();
        assert_eq!(f.version, 0); // default is 0, load() bumps to 2 on save
        assert!(f.geometries.is_empty());
        assert!(f.last_resolution_key.is_none());
    }

    #[test]
    fn legacy_resolution_key_constant_is_stable() {
        // If this changes, every old-format JSON file silently
        // creates a new "default" entry instead of being recognised
        // as the migrated legacy one. Hold the line.
        assert_eq!(LEGACY_RESOLUTION_KEY, "legacy");
    }

    #[test]
    fn current_monitor_key_falls_through_when_unset() {
        // If neither MONITOR_KEY nor LAST_LOADED_KEY is set,
        // current_monitor_key returns None. We can't reliably test
        // the OnceLock-set path here because the static persists
        // across tests; just sanity-check the function returns
        // something sensible (Some or None).
        let k = current_monitor_key();
        // Allow either: tests run in arbitrary order, the OnceLock
        // may or may not be set by an earlier test.
        if let Some(k) = k {
            assert!(!k.is_empty(), "key must not be empty if present");
        }
    }
}

// ============================================================================
// Modern color palette - Sleek dark theme with teal accents
// ============================================================================
#[allow(clippy::approx_constant)] // values are colour channels, not math constants
mod colors {
    use iced::Color;

    // Backgrounds - deeper, richer darks
    pub const BG_BASE: Color = Color::from_rgb(0.067, 0.071, 0.082); // #11121520
    pub const BG_SURFACE: Color = Color::from_rgb(0.094, 0.098, 0.114); // #181a1d
    pub const BG_ELEVATED: Color = Color::from_rgb(0.125, 0.133, 0.153); // #202227
    pub const BG_HOVER: Color = Color::from_rgb(0.157, 0.165, 0.192); // #282a31

    // Accent - modern teal/cyan
    pub const ACCENT: Color = Color::from_rgb(0.318, 0.816, 0.816); // #51d0d0
    pub const ACCENT_DIM: Color = Color::from_rgb(0.200, 0.545, 0.545); // #338b8b
    pub const ACCENT_GLOW: Color = Color::from_rgba(0.318, 0.816, 0.816, 0.15);

    // Semantic colors
    #[allow(dead_code)]
    pub const SUCCESS: Color = Color::from_rgb(0.298, 0.733, 0.486); // #4cbb7c
    pub const WARNING: Color = Color::from_rgb(0.988, 0.725, 0.298); // #fcb94c
    pub const DANGER: Color = Color::from_rgb(0.914, 0.349, 0.388); // #e95963
    pub const INFO: Color = Color::from_rgb(0.388, 0.569, 0.969); // #6391f7

    // Text hierarchy
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.949, 0.957, 0.973); // #f2f4f8
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.600, 0.620, 0.670); // #999eab
    pub const TEXT_MUTED: Color = Color::from_rgb(0.420, 0.440, 0.490); // #6b707d

    // Borders
    pub const BORDER: Color = Color::from_rgb(0.180, 0.192, 0.220); // #2e3138
    pub const BORDER_FOCUS: Color = Color::from_rgba(0.318, 0.816, 0.816, 0.5);
}

// ============================================================================
// Bootstrap Icons - Unicode codepoints
// See: https://icons.getbootstrap.com/
// ============================================================================
mod icons {
    // Window controls
    pub const X: char = '\u{F62A}'; // x mark
    pub const GRIP_VERTICAL: char = '\u{F3FF}'; // grip-vertical (resize handle)

    // Actions
    pub const GEAR: char = '\u{F3E5}'; // settings gear
    pub const QUESTION: char = '\u{F505}'; // question circle
    pub const TRASH: char = '\u{F5DE}'; // trash can
    pub const STAR: char = '\u{F588}'; // star outline
    pub const STAR_FILL: char = '\u{F586}'; // star filled

    // Phase 4 sub-task 4.6: pin button (always-on-top toggle).
    pub const PIN: char = '\u{F4D7}'; // pin angle
    pub const PIN_FILL: char = '\u{F4D5}'; // pin angle fill

    // Phase 4 sub-task 4.10: inline list extras (collection /
    // notes glyphs). Bootstrap Icons codepoints.
    pub const FOLDER: char = '\u{F3D6}'; // folder outline (entry in collection)
    pub const JOURNAL_TEXT: char = '\u{F4A4}'; // journal-text (entry has notes)
    pub const TAG: char = '\u{F5B0}'; // tag outline

    // Status
    pub const CIRCLE_FILL: char = '\u{F287}'; // filled circle (status indicator)

    // Types
    pub const FILE_TEXT: char = '\u{F3C1}'; // text file
    pub const IMAGE: char = '\u{F40D}'; // image
}

/// Create an icon text widget
fn icon(codepoint: char) -> iced::widget::Text<'static> {
    text(codepoint.to_string()).font(ICONS)
}

// ============================================================================
// Modern widget styles
// ============================================================================
mod styles {
    use super::colors;
    use iced::border::Radius;
    use iced::widget::{button, container, scrollable, text_input};
    use iced::{Background, Border, Color, Shadow, Vector};

    // Main container - the app window background (borderless)
    pub fn app_container(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_BASE)),
            border: Border::default(),
            shadow: Shadow::default(),
            text_color: Some(colors::TEXT_PRIMARY),
            snap: false,
        }
    }

    // Custom title bar
    pub fn title_bar(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_SURFACE)),
            border: Border {
                color: colors::BORDER,
                width: 0.0,
                radius: Radius::new(0.0),
            },
            shadow: Shadow::default(),
            text_color: Some(colors::TEXT_SECONDARY),
            snap: false,
        }
    }

    // Search input - sleek with subtle border
    pub fn search_input(_theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
        let border_color = match status {
            text_input::Status::Focused { .. } => colors::ACCENT,
            text_input::Status::Hovered => colors::BORDER_FOCUS,
            _ => colors::BORDER,
        };
        text_input::Style {
            background: Background::Color(colors::BG_ELEVATED),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: Radius::new(8.0),
            },
            icon: colors::TEXT_MUTED,
            placeholder: colors::TEXT_MUTED,
            value: colors::TEXT_PRIMARY,
            selection: colors::ACCENT,
        }
    }

    // Tab button - pill style
    pub fn tab_inactive(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let bg = match status {
            button::Status::Hovered => colors::BG_HOVER,
            button::Status::Pressed => colors::BG_ELEVATED,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: colors::TEXT_SECONDARY,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::new(6.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    pub fn tab_active(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let bg = match status {
            button::Status::Hovered => colors::ACCENT,
            button::Status::Pressed => colors::ACCENT_DIM,
            _ => colors::ACCENT_GLOW,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: colors::ACCENT,
            border: Border {
                color: colors::ACCENT,
                width: 1.0,
                radius: Radius::new(6.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    // Entry row - clean with hover effect
    pub fn entry_row(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let bg = match status {
            button::Status::Hovered => colors::BG_HOVER,
            button::Status::Pressed => colors::BG_ELEVATED,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: colors::TEXT_PRIMARY,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::new(8.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    pub fn entry_row_selected(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let bg = match status {
            button::Status::Hovered => colors::ACCENT_DIM,
            button::Status::Pressed => colors::ACCENT_DIM,
            _ => colors::ACCENT_GLOW,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: colors::TEXT_PRIMARY,
            border: Border {
                color: Color::from_rgba(colors::ACCENT.r, colors::ACCENT.g, colors::ACCENT.b, 0.4),
                width: 1.0,
                radius: Radius::new(8.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    // Phase 4 sub-task 4.9: tooltip-as-preview panel. Elevated
    // background + subtle border + 4 px radius. The tooltip
    // widget handles positioning + hover-delay + auto-hide; we
    // only style the surrounding container.
    pub fn tooltip_panel(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_ELEVATED)),
            text_color: Some(colors::TEXT_PRIMARY),
            border: Border {
                color: Color::from_rgba(colors::ACCENT.r, colors::ACCENT.g, colors::ACCENT.b, 0.3),
                width: 1.0,
                radius: Radius::new(4.0),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            snap: false,
        }
    }

    // Phase 4 sub-task 4.6: pin button in the title bar.
    // Transparent background, accent on hover, no border. Visual
    // state distinction (filled vs outlined icon) lives at the
    // call site via the pin/pin-fill icon swap.
    pub fn header_icon_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let bg = match status {
            button::Status::Hovered => colors::BG_HOVER,
            button::Status::Pressed => colors::BG_ELEVATED,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: colors::TEXT_PRIMARY,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::new(4.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    // Action buttons (fav, delete) - minimal until hover
    pub fn action_btn(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, text) = match status {
            button::Status::Hovered => (colors::BG_HOVER, colors::TEXT_PRIMARY),
            button::Status::Pressed => (colors::BG_ELEVATED, colors::TEXT_PRIMARY),
            _ => (Color::TRANSPARENT, colors::TEXT_MUTED),
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::new(4.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    pub fn delete_btn(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, text) = match status {
            button::Status::Hovered => (colors::DANGER, colors::TEXT_PRIMARY),
            button::Status::Pressed => (Color::from_rgb(0.7, 0.2, 0.25), colors::TEXT_PRIMARY),
            _ => (Color::TRANSPARENT, colors::TEXT_MUTED),
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::new(4.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    // Scrollable - minimal scrollbar
    pub fn scrollable_style(_theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
        let scroller_color = match status {
            scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. } => {
                colors::ACCENT_DIM
            }
            _ => colors::BG_HOVER,
        };
        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: scrollable::Rail {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border::default(),
                scroller: scrollable::Scroller {
                    background: Background::Color(scroller_color),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: Radius::new(3.0),
                    },
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border::default(),
                scroller: scrollable::Scroller {
                    background: Background::Color(scroller_color),
                    border: Border::default(),
                },
            },
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Background::Color(Color::TRANSPARENT),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: colors::TEXT_MUTED,
            },
        }
    }

    // Modal overlay - darker backdrop
    pub fn overlay(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.75))),
            border: Border::default(),
            shadow: Shadow::default(),
            text_color: Some(colors::TEXT_PRIMARY),
            snap: false,
        }
    }

    // Modal card - floating panel
    pub fn modal(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_SURFACE)),
            border: Border {
                color: colors::ACCENT,
                width: 1.0,
                radius: Radius::new(12.0),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
            text_color: Some(colors::TEXT_PRIMARY),
            snap: false,
        }
    }

    /// Right-hand inspector panel rendered next to the main entry list.
    /// Distinguished from the main pane by an elevated background + a left
    /// divider drawn via the border's left edge.
    pub fn side_panel(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_SURFACE)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: Radius::new(0.0),
            },
            shadow: Shadow::default(),
            text_color: Some(colors::TEXT_PRIMARY),
            snap: false,
        }
    }

    // Status bar
    pub fn status_bar(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_SURFACE)),
            border: Border {
                color: colors::BORDER,
                width: 0.0,
                radius: Radius::new(0.0),
            },
            shadow: Shadow::default(),
            text_color: Some(colors::TEXT_MUTED),
            snap: false,
        }
    }

    // Primary button
    pub fn primary_btn(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let bg = match status {
            button::Status::Hovered => colors::ACCENT,
            button::Status::Pressed => colors::ACCENT_DIM,
            _ => colors::ACCENT_DIM,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: colors::BG_BASE,
            border: Border {
                color: colors::ACCENT,
                width: 0.0,
                radius: Radius::new(6.0),
            },
            shadow: Shadow::default(),
            snap: false,
        }
    }

    // Type badge styles
    pub fn badge_text(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::from_rgba(
                colors::INFO.r,
                colors::INFO.g,
                colors::INFO.b,
                0.15,
            ))),
            border: Border {
                color: colors::INFO,
                width: 1.0,
                radius: Radius::new(4.0),
            },
            text_color: Some(colors::INFO),
            shadow: Shadow::default(),
            snap: false,
        }
    }

    // Thumbnail container - dark background for letterboxing
    pub fn thumbnail_container(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_ELEVATED)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: Radius::new(4.0),
            },
            shadow: Shadow::default(),
            text_color: None,
            snap: false,
        }
    }

    // Thumbnail placeholder (for missing images)
    pub fn thumbnail_placeholder(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_HOVER)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: Radius::new(4.0),
            },
            shadow: Shadow::default(),
            text_color: Some(colors::TEXT_MUTED),
            snap: false,
        }
    }

    // Preview modal image container
    pub fn preview_image_container(_theme: &iced::Theme) -> container::Style {
        container::Style {
            background: Some(Background::Color(colors::BG_BASE)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: Radius::new(8.0),
            },
            shadow: Shadow::default(),
            text_color: None,
            snap: false,
        }
    }
}

// ============================================================================
// Global state for subscriptions (iced 0.14 requires fn() -> Stream)
// ============================================================================
static CLIPBOARD_WATCHER: std::sync::OnceLock<Arc<Mutex<Watcher>>> = std::sync::OnceLock::new();
static POLL_INTERVAL_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(100);
static GUI_KEYBINDINGS: std::sync::OnceLock<Arc<Mutex<HashMap<String, String>>>> =
    std::sync::OnceLock::new();

// ============================================================================
// Application state
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Main,
    Settings,
    Help,
    /// Side panel inspecting a single entry (text or image). The user opens
    /// it with Tab on the focused entry; the main list stays visible to its
    /// left and the side panel takes the right portion of the window.
    EntryPanel(String), // entry_id
    ConfirmDelete(String), // entry_id - confirmation for deleting favorites
}

/// Phase 4 sub-task 4.3: `#[to_layer_message]` augments the enum
/// with the layer-shell control variants (AnchorChange,
/// SizeChange, MarginChange, etc.) plus the `TryInto<LayershellCustomActionWithId>`
/// impl that `iced_layershell::build_pattern::application` requires.
///
/// On non-Linux builds the macro still emits the variants but
/// they're never produced by iced — harmless dead code that
/// keeps the same Message enum buildable cross-platform.
#[cfg_attr(unix, iced_layershell::to_layer_message)]
#[derive(Debug, Clone)]
pub enum Message {
    // Entry interactions
    CopyEntry(usize),
    CopySelected,
    DeleteEntry(String),
    ToggleFavorite(String),
    SelectTagFilter(Option<String>),
    TagInputChanged(String),
    AddTagToEntry(String),
    RemoveTagFromEntry {
        entry_id: String,
        tag_id: String,
    },
    MoveEntryToCollection {
        entry_id: String,
        collection_id: Option<String>,
    },
    SetEntryGlobalHotkey(String),
    ClearEntryGlobalHotkey(String),
    ImageZoomIn,
    ImageZoomOut,
    ImageZoomFit,
    ImageZoomActualSize,
    OpenImageExternal(String),
    SettingsMaxEntriesChanged(String),
    SettingsPollIntervalChanged(String),
    SettingsThemeSelectedChanged(String),
    SettingsThemeTextChanged(String),
    SettingsThemeBorderChanged(String),
    SettingsThemeMutedChanged(String),
    ToggleSettingsHideOnBlur,
    SaveSettings,
    ToggleMultiSelect,
    ToggleEntrySelected(String),
    BulkDeleteSelected,
    BulkCopyJoined,
    BulkTagInputChanged(String),
    BulkTagSelected,
    BulkMoveSelectedToCollection(String),
    BulkTransformInputChanged(String),
    BulkTransformSelected,
    CollectionNameInputChanged(String),
    CollectionColorInputChanged(String),
    SelectCollectionForEdit(String),
    CreateCollection,
    SaveCollection,
    DeleteSelectedCollection,

    // Search
    SearchChanged(String),
    PerformSearch(String),
    SearchCompleted(std::result::Result<Vec<Entry>, String>),

    // Navigation
    MoveUp,
    MoveDown,
    NextPage,
    PrevPage,

    // Tabs
    SelectTab(usize),
    NextTab,
    PrevTab,

    // Window
    ToggleWindow,
    HideWindow,
    WindowFocused,
    WindowUnfocused,
    WindowMoved(Point),
    WindowResized(Size),
    StartDrag,
    StartResize(Direction),

    // Refresh
    Tick,

    // Global hotkey (Windows only)
    #[cfg(windows)]
    GlobalHotkeyPressed,

    // Clipboard watcher
    ClipboardChanged,

    // Tray menu
    TrayMenuEvent(String),
    QuitApp,

    // Phase 4 sub-tasks 4.1 + 4.2 — IPC plumbing.
    /// Tick emitted every 50 ms so we drain pending IPC commands
    /// from the daemon socket via `try_recv`.
    PollIpc,
    /// Captured the main window's `Id` on first open. Stored in a
    /// static so IPC handlers can target the window with
    /// `iced::window::set_mode`.
    WindowOpened(iced::window::Id),
    /// Phase 4 sub-task 4.6: toggle the always-on-top pin. Flips
    /// the launcher between `Layer::Top` and `Layer::Overlay`
    /// (layer-shell only) and gates hide-on-blur — pinned
    /// launchers don't auto-close.
    TogglePin,

    // View modes
    ShowSettings,
    HideSettings,
    ToggleHelp,
    CloseOverlay,

    // Side panel inspector
    ToggleEntryPanel(String), // entry_id - open or close depending on state
    /// Toggle the side panel for the currently selected entry. Sent by the
    /// keyboard subscription (Tab key) since it can't see app state.
    ToggleSelectedEntryPanel,
    CloseEntryPanel,
    CopyFromPreview,

    // Delete confirmation
    RequestDelete(String), // entry_id - triggers confirmation for favorites
    ConfirmDeleteEntry(String), // entry_id - actually delete after confirmation
    CancelDelete,

    // Settings
    ToggleStartup,

    // Focus
    FocusSearch,

    // Scroll tracking
    Scrolled(scrollable::Viewport),
}

pub struct DitoxApp {
    /// Handle to the dedicated DB actor thread. Cheap clone, `Send +
    /// Sync`. Replaces the Phase-0-pre `Arc<Mutex<Database>>` pattern
    /// — see `docs/notes/db-actor.md`.
    db: DbHandle,
    config: Config,
    search_query: String,
    selected_index: usize,
    entries: Vec<Entry>,
    visible: bool,
    view_mode: ViewMode,
    tabs: Vec<TabFilter>,
    active_tab: usize,
    current_page: usize,
    total_count: usize,
    window_state: WindowState,
    #[cfg(windows)]
    _hotkey_manager: Option<GlobalHotKeyManager>,
    _tray_icon: Option<TrayIcon>,
    last_refresh: Instant,
    poll_interval_ms: u64,
    last_show_time: Instant,
    #[cfg_attr(not(windows), allow(dead_code))]
    last_hotkey_time: Instant,
    /// Block search input until this time (to prevent capturing hotkey "v")
    input_blocked_until: Option<Instant>,
    /// Cache image handles to avoid reloading from disk on every render
    image_cache: HashMap<String, iced_image::Handle>,
    /// Current scroll viewport for smart scrolling
    scroll_viewport: Option<scrollable::Viewport>,
    is_searching: bool,
    /// Phase 2 paste-back: snapshot of the window that was focused
    /// BEFORE the launcher appeared. Captured in `main.rs::run` and
    /// threaded through `boot_app`. `None` when no foreground tracker
    /// is available (GNOME Wayland, unsupported platforms) or the
    /// snapshot returned `None` (no foreground / launcher already
    /// running). Consumed (cloned) by `paste_and_exit`.
    previous_foreground: Option<ForegroundSnapshot>,
    /// Phase 2 paste-back: foreground tracker used by
    /// `paste_and_exit` to call `restore()` before synthesising the
    /// paste keystroke. Wrapped in `Box<dyn>` so the platform-specific
    /// impl is opaque to the GUI.
    foreground_tracker: Box<dyn ForegroundTracker>,
    /// Phase 2 paste-back: ordered chain of synthesizer backends.
    /// Built by `pick_chain(platform)` in `main.rs::run`. Always ends
    /// with `OffSynthesizer` so `paste_with_chain` never returns Err
    /// from this chain.
    synthesizer_chain: Vec<Box<dyn Synthesizer>>,
    /// Phase 4 sub-task 4.6: always-on-top pin state. Initialised
    /// from `Config.gui.pinned`; toggled at runtime via the header
    /// pin button (`Message::TogglePin`) and persisted to config
    /// only via explicit user edit.
    pinned: bool,
    /// Phase 4b: all known tags for chips/type-ahead surfaces.
    all_tags: Vec<Tag>,
    /// Phase 4b: collections shown as tabs after the built-in filters.
    all_collections: Vec<Collection>,
    /// Phase 4b: visible-entry tag cache keyed by entry id.
    entry_tags: HashMap<String, Vec<Tag>>,
    /// Phase 4b: active tag filters. Multiple selected tags are ANDed.
    active_tag_filters: HashSet<String>,
    /// Phase 4b: side-panel tag editor buffer.
    tag_input: String,
    image_zoom: f32,
    settings_max_entries: String,
    settings_poll_interval_ms: String,
    settings_theme_selected: String,
    settings_theme_text: String,
    settings_theme_border: String,
    settings_theme_muted: String,
    settings_status: Option<String>,
    multi_select: bool,
    selected_entry_ids: HashSet<String>,
    bulk_tag_input: String,
    bulk_transform_input: String,
    collection_name_input: String,
    collection_color_input: String,
    editing_collection_id: Option<String>,
    collection_status: Option<String>,
    config_path: Option<PathBuf>,
    config_mtime: Option<SystemTime>,
}

impl DitoxApp {
    #[allow(clippy::too_many_arguments)]
    fn new(
        db: Database,
        config: Config,
        start_hidden: bool,
        previous_foreground: Option<ForegroundSnapshot>,
        foreground_tracker: Box<dyn ForegroundTracker>,
        synthesizer_chain: Vec<Box<dyn Synthesizer>>,
        initial_selection: usize,
    ) -> (Self, Task<Message>) {
        // Move the Database onto its own thread; iced runs against
        // the cheap `DbHandle`. The `DbActorJoin` is dropped on the
        // floor — the actor exits naturally when the last `DbHandle`
        // clone (this struct's `db` field) drops on app exit.
        let (db, _join) = DbHandle::spawn(db);
        let total_count = db.call(|d| d.count().unwrap_or(0)).unwrap_or(0);
        let entries = db
            .call(|d| d.get_page(0, PAGE_SIZE).unwrap_or_default())
            .unwrap_or_default();
        let window_state = WindowState::load();

        // Phase 2 paste-back sub-task 2.9: clamp the persisted cursor
        // index to the visible list. Modulo wraps past-end values back
        // to the top of the list (useful when a stale cursor was
        // persisted before entries were deleted) — matches
        // `SelectionCursor::index_for_list`.
        let selected_index = if entries.is_empty() {
            0
        } else {
            initial_selection % entries.len()
        };

        tracing::info!(
            "Loaded window state: {}x{} at ({}, {})",
            window_state.width,
            window_state.height,
            window_state.x,
            window_state.y
        );

        #[cfg(windows)]
        let hotkey_manager = {
            let hotkey_manager = GlobalHotKeyManager::new().ok();
            if let Some(ref manager) = hotkey_manager {
                let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
                if manager.register(hotkey).is_ok() {
                    tracing::info!("Registered global hotkey: Ctrl+Shift+V");
                }
            }
            hotkey_manager
        };

        #[cfg(windows)]
        let tray_icon = setup_tray_icon();
        #[cfg(all(unix, not(target_os = "macos")))]
        let tray_icon: Option<TrayIcon> = {
            spawn_linux_tray_thread();
            None // owned by the tray thread; nothing to hold here
        };
        #[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
        let tray_icon: Option<TrayIcon> = None;

        let watcher_db = Database::open().unwrap_or_else(|e| {
            tracing::error!("Failed to open watcher database: {}", e);
            panic!("Cannot run without database");
        });
        let poll_interval_ms = config.general.poll_interval_ms;
        let mut watcher = Watcher::new(watcher_db, config.clone());
        watcher.initialize_hash();

        // Initialize static globals for subscriptions (iced 0.14 requirement)
        let _ = CLIPBOARD_WATCHER.set(Arc::new(Mutex::new(watcher)));
        POLL_INTERVAL_MS.store(poll_interval_ms, std::sync::atomic::Ordering::Relaxed);
        let _ = GUI_KEYBINDINGS.set(Arc::new(Mutex::new(config.keybindings.gui.clone())));

        let tabs = vec![
            TabFilter::All,
            TabFilter::Text,
            TabFilter::Images,
            TabFilter::Favorites,
            TabFilter::Today,
            TabFilter::Yesterday,
            TabFilter::ThisWeek,
            TabFilter::ThisMonth,
            TabFilter::Older,
        ];

        // Capture before `config` is moved into the struct.
        let initial_pinned = config.gui.pinned;
        let settings_max_entries = config.general.max_entries.to_string();
        let settings_poll_interval_ms = config.general.poll_interval_ms.to_string();
        let settings_theme_selected = config.ui.theme.selected.clone();
        let settings_theme_text = config.ui.theme.text.clone();
        let settings_theme_border = config.ui.theme.border.clone();
        let settings_theme_muted = config.ui.theme.muted.clone();
        let config_path = Config::get_config_path().ok();
        let config_mtime = config_path.as_ref().and_then(|p| config_mtime(p));

        let mut app = DitoxApp {
            db,
            config,
            search_query: String::new(),
            selected_index,
            entries,
            visible: !start_hidden,
            view_mode: ViewMode::Main,
            tabs,
            active_tab: 0,
            current_page: 0,
            total_count,
            window_state: window_state.clone(),
            #[cfg(windows)]
            _hotkey_manager: hotkey_manager,
            _tray_icon: tray_icon,
            last_refresh: Instant::now(),
            poll_interval_ms,
            last_show_time: Instant::now(),
            last_hotkey_time: Instant::now() - Duration::from_secs(10),
            input_blocked_until: None,
            image_cache: HashMap::new(),
            scroll_viewport: None,
            is_searching: false,
            previous_foreground,
            foreground_tracker,
            synthesizer_chain,
            pinned: initial_pinned,
            all_tags: Vec::new(),
            all_collections: Vec::new(),
            entry_tags: HashMap::new(),
            active_tag_filters: HashSet::new(),
            tag_input: String::new(),
            image_zoom: 1.0,
            settings_max_entries,
            settings_poll_interval_ms,
            settings_theme_selected,
            settings_theme_text,
            settings_theme_border,
            settings_theme_muted,
            settings_status: None,
            multi_select: false,
            selected_entry_ids: HashSet::new(),
            bulk_tag_input: String::new(),
            bulk_transform_input: String::new(),
            collection_name_input: String::new(),
            collection_color_input: String::new(),
            editing_collection_id: None,
            collection_status: None,
            config_path,
            config_mtime,
        };

        // One-shot mode: don't override the bottom-left position picked by
        // `Position::SpecificWith`. Just focus the search input.
        app.refresh_tag_cache();
        let initial_task = delayed_focus_search();

        (app, initial_task)
    }

    fn title(&self) -> String {
        String::from("Ditox")
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    /// Phase 4 sub-tasks 4.1 + 4.2: drain pending IPC commands from
    /// the daemon receiver and execute them. Each `DaemonCommand`
    /// carries a one-shot reply channel so the IPC client (a fresh
    /// `ditox-gui --foo` invocation) sees a synchronous OK / ERR.
    ///
    /// Today's scope is plumbing only: `Show` / `Hide` / `Toggle`
    /// update the in-memory `visible` flag and (best-effort) issue
    /// `iced::window::set_mode` if we know the main window's `Id`,
    /// but the existing one-shot UX (paste → exit) stays unchanged.
    /// Phase 4 sub-task 4.3 will integrate `iced_layershell` and
    /// rework `paste_and_exit` into `paste_and_hide` so the daemon
    /// genuinely lives across multiple show/hide cycles.
    fn drain_ipc(&mut self) -> Task<Message> {
        use crate::ipc::Command;

        let Some(mutex) = APP_IPC_RX.get() else {
            return Task::none();
        };
        let Ok(guard) = mutex.lock() else {
            return Task::none();
        };
        let Some(rx) = guard.as_ref() else {
            return Task::none();
        };

        let mut tasks: Vec<Task<Message>> = Vec::new();
        loop {
            let mut cmd = match rx.try_recv() {
                Ok(c) => c,
                Err(_) => break,
            };

            match cmd.command.clone() {
                Command::Show => {
                    // Snapshot the previously-focused app BEFORE
                    // showing — once we're visible, the foreground
                    // is the daemon itself.
                    self.refresh_foreground_snapshot();
                    // Phase 4 sub-task 4.7: fire the cycling cursor
                    // so rapid re-summons advance the selection.
                    self.fire_cursor_for_summon();
                    self.visible = true;
                    self.last_show_time = Instant::now();
                    cmd.reply_ok();
                    if let Some(id) = APP_MAIN_WINDOW_ID.get() {
                        tasks.push(iced::window::set_mode(*id, iced::window::Mode::Windowed));
                    }
                }
                Command::Hide => {
                    self.visible = false;
                    cmd.reply_ok();
                    if let Some(id) = APP_MAIN_WINDOW_ID.get() {
                        tasks.push(iced::window::set_mode(*id, iced::window::Mode::Hidden));
                    }
                }
                Command::Toggle => {
                    let was_visible = self.visible;
                    if !was_visible {
                        // About to show: refresh foreground first,
                        // then advance the cycling cursor.
                        self.refresh_foreground_snapshot();
                        self.fire_cursor_for_summon();
                        self.last_show_time = Instant::now();
                    }
                    self.visible = !self.visible;
                    cmd.reply_ok();
                    if let Some(id) = APP_MAIN_WINDOW_ID.get() {
                        let mode = if self.visible {
                            iced::window::Mode::Windowed
                        } else {
                            iced::window::Mode::Hidden
                        };
                        tasks.push(iced::window::set_mode(*id, mode));
                    }
                }
                Command::Quit => {
                    cmd.reply_ok();
                    // Save window state before exiting so geometry
                    // persists for the next launch.
                    self.save_window_state();
                    tracing::info!("quit requested via IPC; exiting");
                    // Defer the actual exit by one tick so the reply
                    // makes it back to the IPC client before the
                    // process dies.
                    std::thread::Builder::new()
                        .name("ditox-gui-quit".into())
                        .spawn(|| {
                            std::thread::sleep(Duration::from_millis(50));
                            std::process::exit(0);
                        })
                        .ok();
                }
                Command::Status => {
                    let payload = format!(
                        "visible={} entries={} version={}",
                        self.visible,
                        self.entries.len(),
                        env!("CARGO_PKG_VERSION")
                    );
                    cmd.reply_ok_with(&payload);
                }
                Command::PasteClip(id) | Command::Emit(id) => match self.entry_by_id(&id) {
                    Ok(entry) => {
                        cmd.reply_ok();
                        tasks.push(self.paste_and_hide(entry));
                    }
                    Err(e) => cmd.reply_err(&e),
                },
                Command::ReloadConfig => {
                    self.config_mtime = None;
                    self.reload_config_if_changed();
                    cmd.reply_ok();
                }
                Command::GetEntry(id) => match self.db.call(move |d| d.get_by_id(&id)) {
                    Ok(Ok(Some(entry))) => match serde_json::to_string(&entry) {
                        Ok(json) => cmd.reply_ok_with(&json),
                        Err(e) => cmd.reply_err(&e.to_string()),
                    },
                    Ok(Ok(None)) => cmd.reply_err("not-found"),
                    Ok(Err(e)) => cmd.reply_err(&e.to_string()),
                    Err(e) => cmd.reply_err(&e.to_string()),
                },
                Command::ListEntries { limit, json } => {
                    match self.db.call(move |d| d.get_page(0, limit)) {
                        Ok(Ok(entries)) if json => match serde_json::to_string(&entries) {
                            Ok(body) => cmd.reply_ok_with(&body),
                            Err(e) => cmd.reply_err(&e.to_string()),
                        },
                        Ok(Ok(entries)) => {
                            let body = entries
                                .iter()
                                .map(|e| format!("{} {}", e.id, e.preview(60)))
                                .collect::<Vec<_>>()
                                .join(" | ");
                            cmd.reply_ok_with(&body);
                        }
                        Ok(Err(e)) => cmd.reply_err(&e.to_string()),
                        Err(e) => cmd.reply_err(&e.to_string()),
                    }
                }
            }
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn entry_by_id(&self, id: &str) -> std::result::Result<Entry, String> {
        self.db
            .call({
                let id = id.to_string();
                move |d| d.get_by_id(&id)
            })
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "not-found".to_string())
    }

    /// Phase 4 sub-task 4.3: hide the main window via
    /// `iced::window::set_mode(id, Hidden)`. The daemon process
    /// stays alive and accepts subsequent `--show` / `--toggle` IPC
    /// commands. If the window's `Id` hasn't been captured yet
    /// (rare race during very early startup) returns `Task::none()`
    /// and the window stays visible — Phase 4 polish will retry on
    /// the next iced tick.
    fn hide_window(&mut self) -> Task<Message> {
        self.visible = false;
        if let Some(id) = APP_MAIN_WINDOW_ID.get() {
            iced::window::set_mode(*id, iced::window::Mode::Hidden)
        } else {
            tracing::debug!("hide_window: window id not captured yet; skipping set_mode");
            Task::none()
        }
    }

    /// Phase 4 sub-task 4.7: fire the persistent selection cursor
    /// (sub-task 2.9 groundwork) on each daemon summon and pre-select
    /// the resulting entry. Modifier-held cycling: rapid re-summons
    /// (within `paste.cursor_refire_window_ms`, default 800 ms)
    /// advance the index by one; idle past the window resets to 0.
    ///
    /// Called from the IPC `Show` / `Toggle` (going-visible) handlers
    /// after `refresh_foreground_snapshot`.
    fn fire_cursor_for_summon(&mut self) {
        use ditox_core::paste::cursor::PersistentSelectionCursor;

        let persistent = match PersistentSelectionCursor::at_default_path() {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(error = %e, "selection cursor unavailable on summon");
                return;
            }
        };

        let cursor = persistent.fire_and_persist(
            std::time::SystemTime::now(),
            self.config.paste.cursor_refire_window(),
        );
        let entries_len = self.entries.len();
        let new_index = cursor.index_for_list(entries_len);
        tracing::debug!(
            cursor_index = cursor.index(),
            window_ms = self.config.paste.cursor_refire_window().as_millis(),
            entries = entries_len,
            selected = new_index,
            "fired selection cursor on summon"
        );
        self.selected_index = new_index;
    }

    /// Phase 4 sub-task 4.3: re-snapshot the foreground app right
    /// before showing the window. The original `previous_foreground`
    /// captured in `main.rs::run` was stale after the first
    /// hide/show cycle — paste-back would target whichever app was
    /// focused when the daemon first launched, not the one the user
    /// is summoning from now. Called from the IPC `Show` / `Toggle`
    /// handlers immediately before flipping `set_mode(Windowed)`.
    ///
    /// `ForegroundFilter` (wrapping the tracker) drops self-snapshots
    /// (the `ditox-gui` process), so we always get the previously-active
    /// app — never the daemon itself.
    fn refresh_foreground_snapshot(&mut self) {
        match self.foreground_tracker.snapshot() {
            Ok(Some(snap)) => {
                tracing::info!(
                    process = %snap.process_basename,
                    title = %snap.title,
                    kind = %snap.identifier.kind(),
                    "refreshed foreground snapshot for paste-back"
                );
                self.previous_foreground = Some(snap);
            }
            Ok(None) => {
                tracing::debug!("foreground tracker returned None; clearing previous snapshot");
                self.previous_foreground = None;
            }
            Err(e) => {
                tracing::warn!(error = %e, "foreground refresh failed; keeping previous snapshot");
            }
        }
    }

    /// Phase 2 paste-back (post-Phase-4): write `entry` to the
    /// clipboard, record a sentinel hash so the watcher skips the
    /// inevitable re-capture, optionally restore the previously-focused
    /// window and synthesise the paste keystroke, then **hide** the
    /// daemon's window (post-4.3) — formerly `paste_and_exit` which
    /// `process::exit(0)`-ed.
    ///
    /// Returns a `Task<Message>` so call sites compose with iced's
    /// command stream; non-divergent so the daemon can serve
    /// follow-up summons.
    ///
    /// Per-step error handling:
    ///
    /// - Clipboard write fails → log + still try paste-back (so the
    ///   user at least gets focus restored).
    /// - Sentinel record fails → already swallowed inside
    ///   `PasteSentinel::record` (best-effort log).
    /// - Paste-back fails → log; user pastes manually.
    fn paste_and_hide(&mut self, entry: Entry) -> Task<Message> {
        // 1. Write clipboard.
        let write_result = match entry.entry_type {
            EntryType::Text => Clipboard::set_text(&entry.content),
            EntryType::Image => match entry.image_path() {
                Some(p) => Clipboard::set_image(&p.to_string_lossy()),
                None => Err(ditox_core::DitoxError::Other(
                    "image entry missing extension".into(),
                )),
            },
        };
        if let Err(e) = &write_result {
            tracing::warn!(error = %e, "clipboard write failed");
        }

        // 2. Mark in DB (LRU bump) — fire-and-forget; we exit before
        //    the actor processes it but SQLite WAL keeps the write
        //    durable past process exit.
        let id = entry.id.clone();
        let _ = self.db.dispatch(move |d| {
            let _ = d.touch(&id);
        });
        tracing::info!("Pasting: {}", entry.preview(30));

        // 3. Record paste sentinel so the watcher (in this process or
        //    the daemon) skips the re-capture. The hash we record
        //    must match what `Watcher::process_clip` would compute:
        //    - Text: `Clipboard::hash(content.as_bytes())`.
        //    - Image: the entry's `content` field IS the SHA-256 of
        //      the image bytes (content-addressed storage; see
        //      AGENTS.md).
        if write_result.is_ok() {
            let inner_hash = match entry.entry_type {
                EntryType::Text => Clipboard::hash(entry.content.as_bytes()),
                EntryType::Image => entry.content.clone(),
            };
            if let Ok(sentinel) = PasteSentinel::at_default_path() {
                sentinel.record(&inner_hash);
                tracing::debug!(
                    hash = %&inner_hash[..8.min(inner_hash.len())],
                    "recorded paste sentinel"
                );
            }
        }

        // 4. Foreground restore + keystroke synthesis. Skipped when:
        //    - Paste-back is disabled in config.
        //    - No previous-foreground snapshot exists.
        //    - The platform doesn't support client-driven re-focus
        //      (Wlr / Unknown — see ForegroundId::supports_restore).
        if !self.config.paste.disabled && write_result.is_ok() {
            if let Some(snap) = self.previous_foreground.clone() {
                if snap.identifier.supports_restore() {
                    if let Err(e) = self.foreground_tracker.restore(&snap) {
                        tracing::warn!(
                            error = %e,
                            target = %snap.process_basename,
                            "foreground restore failed; user will need to focus manually"
                        );
                    } else {
                        // Brief sleep gives the compositor time to
                        // switch focus before we synthesise the
                        // keystroke. 50 ms is the spec value (task
                        // 024 line 167); on Hyprland we typically
                        // see focus change well under 10 ms but the
                        // padding handles slow compositors.
                        std::thread::sleep(Duration::from_millis(50));

                        let keys_str = self.config.paste.keystroke_for(&snap.process_basename);
                        match KeystrokeSequence::parse(&keys_str) {
                            Ok(keys) => {
                                match paste_with_chain(&self.synthesizer_chain, &snap, &keys) {
                                    Ok(name) => tracing::info!(
                                        synth = name,
                                        target = %snap.process_basename,
                                        "paste-back succeeded"
                                    ),
                                    Err(e) => tracing::warn!(
                                        error = %e,
                                        "paste-back chain failed; user will paste manually"
                                    ),
                                }
                            }
                            Err(e) => tracing::warn!(
                                error = %e,
                                keystroke = %keys_str,
                                "invalid keystroke override; user will paste manually"
                            ),
                        }
                    }
                } else {
                    tracing::debug!(
                        kind = %snap.identifier.kind(),
                        "previous foreground doesn't support restore; skipping"
                    );
                }
            } else {
                tracing::debug!("no previous-foreground snapshot; skipping paste-back");
            }
        }

        self.save_window_state();
        // Phase 4: hide the daemon's window instead of exiting so
        // the next `--show` / `--toggle` IPC summon (or compositor
        // keybind) re-uses this process.
        self.hide_window()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CopyEntry(index) => {
                // Phase 2 paste-back + Phase 4 daemon: clipboard
                // write + sentinel + restore + keystroke synthesis,
                // then HIDE the window (was `process::exit` in the
                // one-shot model).
                if let Some(entry) = self.entries.get(index).cloned() {
                    return self.paste_and_hide(entry);
                }
                // Index out of bounds (extremely unlikely): just
                // hide. Previous one-shot model exited even on
                // no-op; keeping the daemon alive is the better
                // default now that it owns the IPC socket.
                self.save_window_state();
                return self.hide_window();
            }

            Message::CopySelected => {
                return self.update(Message::CopyEntry(self.selected_index));
            }

            Message::RequestDelete(id) => {
                // Check if entry is a favorite - if so, show confirmation dialog
                if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
                    if entry.favorite {
                        self.view_mode = ViewMode::ConfirmDelete(id);
                    } else {
                        // Non-favorite: delete directly
                        return self.update(Message::ConfirmDeleteEntry(id));
                    }
                }
            }

            Message::ConfirmDeleteEntry(id) => {
                // Actually delete the entry (after confirmation for favorites)
                let was_in_preview = matches!(self.view_mode, ViewMode::EntryPanel(_));
                let was_in_confirm = matches!(self.view_mode, ViewMode::ConfirmDelete(_));
                // Block on delete — `refresh_entries` below queries
                // the new state, and on a busy actor we'd otherwise
                // race and re-show the deleted row briefly.
                let _ = self.db.call(move |d| d.delete(&id));
                self.refresh_entries();
                // Close modal after deletion
                if was_in_preview || was_in_confirm {
                    self.view_mode = ViewMode::Main;
                }
            }

            Message::CancelDelete => {
                self.view_mode = ViewMode::Main;
            }

            Message::DeleteEntry(id) => {
                // Legacy handler - redirect to new flow
                return self.update(Message::RequestDelete(id));
            }

            Message::ToggleFavorite(id) => {
                if self.view_mode == ViewMode::Main {
                    // Same reasoning as `ConfirmDeleteEntry`: we
                    // refresh immediately after, so block on the
                    // round-trip to avoid stale display.
                    let _ = self.db.call(move |d| d.toggle_favorite(&id));
                    self.refresh_entries();
                }
            }

            Message::SelectTagFilter(tag_id) => {
                if let Some(tag_id) = tag_id {
                    if self.active_tag_filters.contains(&tag_id) {
                        self.active_tag_filters.remove(&tag_id);
                    } else {
                        self.active_tag_filters.insert(tag_id);
                    }
                } else {
                    self.active_tag_filters.clear();
                }
                self.current_page = 0;
                self.refresh_entries();
            }

            Message::TagInputChanged(value) => {
                self.tag_input = value;
            }

            Message::AddTagToEntry(entry_id) => {
                let tag_name = self.tag_input.trim().to_string();
                if !tag_name.is_empty() {
                    let _ = self.db.call(move |d| {
                        d.add_tag_to_entry_by_name(&entry_id, &tag_name, None)
                            .map(|_| ())
                    });
                    self.tag_input.clear();
                    self.refresh_entries();
                }
            }

            Message::RemoveTagFromEntry { entry_id, tag_id } => {
                let _ = self
                    .db
                    .call(move |d| d.remove_tag_from_entry(&entry_id, &tag_id));
                self.refresh_entries();
            }

            Message::MoveEntryToCollection {
                entry_id,
                collection_id,
            } => {
                let _ = self
                    .db
                    .call(move |d| d.set_entry_collection(&entry_id, collection_id.as_deref()));
                self.refresh_entries();
            }

            Message::SetEntryGlobalHotkey(entry_id) => {
                let idx = self
                    .entries
                    .iter()
                    .position(|e| e.id == entry_id)
                    .unwrap_or(0)
                    + 1;
                let hotkey = format!("ctrl+alt+{}", idx.min(9));
                let _ = self.db.call({
                    let entry_id = entry_id.clone();
                    let hotkey = hotkey.clone();
                    move |d| d.set_entry_hotkeys(&entry_id, Some(&hotkey), None)
                });
                self.regenerate_hyprland_binds();
                self.refresh_entries();
            }

            Message::ClearEntryGlobalHotkey(entry_id) => {
                let _ = self
                    .db
                    .call(move |d| d.set_entry_hotkeys(&entry_id, None, None));
                self.regenerate_hyprland_binds();
                self.refresh_entries();
            }

            Message::ImageZoomIn => {
                self.image_zoom = (self.image_zoom + 0.1).min(4.0);
            }

            Message::ImageZoomOut => {
                self.image_zoom = (self.image_zoom - 0.1).max(0.25);
            }

            Message::ImageZoomFit => {
                self.image_zoom = 1.0;
            }

            Message::ImageZoomActualSize => {
                self.image_zoom = 1.0;
            }

            Message::OpenImageExternal(path) => {
                open_path_external(&path);
            }

            Message::SettingsMaxEntriesChanged(value) => {
                self.settings_max_entries = value;
            }

            Message::SettingsPollIntervalChanged(value) => {
                self.settings_poll_interval_ms = value;
            }

            Message::SettingsThemeSelectedChanged(value) => {
                self.settings_theme_selected = value;
            }

            Message::SettingsThemeTextChanged(value) => {
                self.settings_theme_text = value;
            }

            Message::SettingsThemeBorderChanged(value) => {
                self.settings_theme_border = value;
            }

            Message::SettingsThemeMutedChanged(value) => {
                self.settings_theme_muted = value;
            }

            Message::ToggleSettingsHideOnBlur => {
                self.config.gui.hide_on_blur = !self.config.gui.hide_on_blur;
            }

            Message::SaveSettings => {
                let max_entries = match self.settings_max_entries.trim().parse::<usize>() {
                    Ok(v) if v > 0 => v,
                    _ => {
                        self.settings_status =
                            Some("max_entries must be a positive integer".into());
                        return Task::none();
                    }
                };
                let poll_interval = match self.settings_poll_interval_ms.trim().parse::<u64>() {
                    Ok(v) if v > 0 => v,
                    _ => {
                        self.settings_status = Some("poll_interval_ms must be positive".into());
                        return Task::none();
                    }
                };

                self.config.general.max_entries = max_entries;
                self.config.general.poll_interval_ms = poll_interval;
                self.config.ui.theme.selected = self.settings_theme_selected.clone();
                self.config.ui.theme.text = self.settings_theme_text.clone();
                self.config.ui.theme.border = self.settings_theme_border.clone();
                self.config.ui.theme.muted = self.settings_theme_muted.clone();
                self.poll_interval_ms = poll_interval;
                POLL_INTERVAL_MS.store(poll_interval, std::sync::atomic::Ordering::Relaxed);

                match self.config.save() {
                    Ok(()) => {
                        self.config_mtime = self.config_path.as_ref().and_then(|p| config_mtime(p));
                        self.settings_status = Some("Settings saved".into());
                        self.refresh_entries();
                    }
                    Err(e) => {
                        self.settings_status = Some(format!("Save failed: {e}"));
                    }
                }
            }

            Message::ToggleMultiSelect => {
                self.multi_select = !self.multi_select;
                if !self.multi_select {
                    self.selected_entry_ids.clear();
                }
            }

            Message::ToggleEntrySelected(entry_id) => {
                if self.selected_entry_ids.contains(&entry_id) {
                    self.selected_entry_ids.remove(&entry_id);
                } else {
                    self.selected_entry_ids.insert(entry_id);
                }
            }

            Message::BulkDeleteSelected => {
                let ids: Vec<String> = self.selected_entry_ids.iter().cloned().collect();
                let _ = self.db.call(move |d| {
                    for id in ids {
                        let _ = d.delete(&id);
                    }
                });
                self.selected_entry_ids.clear();
                self.refresh_entries();
            }

            Message::BulkCopyJoined => {
                let selected = self.selected_entry_ids.clone();
                let text = self
                    .entries
                    .iter()
                    .filter(|e| selected.contains(&e.id) && e.entry_type == EntryType::Text)
                    .map(|e| e.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                if !text.is_empty() {
                    let _ = Clipboard::set_text(&text);
                }
            }

            Message::BulkTagInputChanged(value) => {
                self.bulk_tag_input = value;
            }

            Message::BulkTagSelected => {
                let tag = self.bulk_tag_input.trim().to_string();
                if !tag.is_empty() {
                    let ids: Vec<String> = self.selected_entry_ids.iter().cloned().collect();
                    let _ = self.db.call(move |d| {
                        for id in ids {
                            let _ = d.add_tag_to_entry_by_name(&id, &tag, None);
                        }
                    });
                    self.bulk_tag_input.clear();
                    self.refresh_entries();
                }
            }

            Message::BulkMoveSelectedToCollection(collection_id) => {
                let ids: Vec<String> = self.selected_entry_ids.iter().cloned().collect();
                let _ = self.db.call(move |d| {
                    for id in ids {
                        let _ = d.set_entry_collection(&id, Some(&collection_id));
                    }
                });
                self.refresh_entries();
            }

            Message::BulkTransformInputChanged(value) => {
                self.bulk_transform_input = value;
            }

            Message::BulkTransformSelected => {
                if let Some(transform) =
                    ditox_core::transforms::get(self.bulk_transform_input.trim())
                {
                    let selected = self.selected_entry_ids.clone();
                    let text = self
                        .entries
                        .iter()
                        .filter(|e| selected.contains(&e.id) && e.entry_type == EntryType::Text)
                        .filter_map(|e| transform.apply_text(&e.content).ok())
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !text.is_empty() {
                        let _ = Clipboard::set_text(&text);
                    }
                }
            }

            Message::CollectionNameInputChanged(value) => {
                self.collection_name_input = value;
            }

            Message::CollectionColorInputChanged(value) => {
                self.collection_color_input = value;
            }

            Message::SelectCollectionForEdit(collection_id) => {
                if let Some(collection) =
                    self.all_collections.iter().find(|c| c.id == collection_id)
                {
                    self.collection_name_input = collection.name.clone();
                    self.collection_color_input = collection.color.clone().unwrap_or_default();
                    self.editing_collection_id = Some(collection.id.clone());
                    self.collection_status = None;
                }
            }

            Message::CreateCollection => {
                let name = self.collection_name_input.trim().to_string();
                if name.is_empty() {
                    self.collection_status = Some("Collection name is required".into());
                } else {
                    let color = non_empty_trimmed(&self.collection_color_input);
                    let position = self.all_collections.len() as i32;
                    let collection = Collection::with_options(name, color, None, position);
                    match self.db.call(move |d| d.create_collection(&collection)) {
                        Ok(Ok(())) => {
                            self.collection_name_input.clear();
                            self.collection_color_input.clear();
                            self.editing_collection_id = None;
                            self.collection_status = Some("Collection created".into());
                            self.refresh_tag_cache();
                        }
                        Ok(Err(e)) => self.collection_status = Some(format!("Create failed: {e}")),
                        Err(e) => self.collection_status = Some(format!("Create failed: {e}")),
                    }
                }
            }

            Message::SaveCollection => {
                let Some(collection_id) = self.editing_collection_id.clone() else {
                    self.collection_status = Some("Select a collection first".into());
                    return Task::none();
                };
                let name = self.collection_name_input.trim().to_string();
                if name.is_empty() {
                    self.collection_status = Some("Collection name is required".into());
                } else {
                    let color = non_empty_trimmed(&self.collection_color_input);
                    let result = self.db.call(move |d| {
                        let Some(mut collection) = d.get_collection_by_id(&collection_id)? else {
                            return Ok(false);
                        };
                        collection.name = name;
                        collection.color = color;
                        d.update_collection(&collection)
                    });
                    match result {
                        Ok(Ok(true)) => {
                            self.collection_status = Some("Collection saved".into());
                            self.refresh_tag_cache();
                        }
                        Ok(Ok(false)) => {
                            self.collection_status = Some("Collection was not found".into());
                        }
                        Ok(Err(e)) => self.collection_status = Some(format!("Save failed: {e}")),
                        Err(e) => self.collection_status = Some(format!("Save failed: {e}")),
                    }
                }
            }

            Message::DeleteSelectedCollection => {
                let Some(collection_id) = self.editing_collection_id.clone() else {
                    self.collection_status = Some("Select a collection first".into());
                    return Task::none();
                };
                match self.db.call(move |d| d.delete_collection(&collection_id)) {
                    Ok(Ok(true)) => {
                        self.collection_name_input.clear();
                        self.collection_color_input.clear();
                        self.editing_collection_id = None;
                        self.collection_status = Some("Collection deleted".into());
                        self.refresh_entries();
                        self.refresh_tag_cache();
                    }
                    Ok(Ok(false)) => {
                        self.collection_status = Some("Collection was not found".into())
                    }
                    Ok(Err(e)) => self.collection_status = Some(format!("Delete failed: {e}")),
                    Err(e) => self.collection_status = Some(format!("Delete failed: {e}")),
                }
            }

            Message::SearchChanged(query) => {
                // Ignore input while blocked (prevents capturing "v" from Ctrl+Shift+V)
                if let Some(blocked_until) = self.input_blocked_until {
                    if Instant::now() < blocked_until {
                        return Task::none();
                    }
                    // Block period has passed, clear it
                    self.input_blocked_until = None;
                }

                self.search_query = query.clone();
                if query.is_empty() {
                    self.refresh_entries();
                    return Task::none();
                }

                // Debounce: wait 20ms (virtually instant but handles key mash)
                // We pass the query content so we can verify if it's still current when the task completes
                let query_to_search = query.clone();
                return Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    },
                    move |_| Message::PerformSearch(query_to_search),
                );
            }

            Message::PerformSearch(query) => {
                // Only search if the query is still the current one (handles debouncing)
                if query == self.search_query {
                    if query.is_empty() {
                        self.refresh_entries();
                        return Task::none();
                    }

                    self.is_searching = true;

                    let db = self.db.clone();
                    let filter = self.active_tab_filter().clone();
                    let (filter_str_ref, collection_id_ref) = filter.db_filter();
                    let filter_str = filter_str_ref.to_string();
                    let collection_id = collection_id_ref.map(|s| s.to_string());

                    // Phase 3 sub-task 3.6: parse out any /p /h /r
                    // /q /f search-mode prefix BEFORE handing off to
                    // the actor. The dispatch helper routes to the
                    // right DB method based on the parsed scope.
                    let parsed = ditox_core::search::parse(&query);

                    // Offload to a tokio blocking thread so the iced
                    // render loop doesn't stall on the actor
                    // round-trip. The actor itself runs the SQL on
                    // its own thread.
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                db.call(move |d| {
                                    ditox_core::search::dispatch(
                                        d,
                                        &parsed,
                                        50,
                                        &filter_str,
                                        collection_id.as_deref(),
                                    )
                                    .map_err(|e| e.to_string())
                                })
                                .unwrap_or_else(|e| Err(e.to_string()))
                            })
                            .await
                            .unwrap()
                        },
                        Message::SearchCompleted,
                    );
                }
            }

            Message::SearchCompleted(result) => {
                self.is_searching = false;
                match result {
                    Ok(results) => {
                        self.entries = results;
                        self.selected_index = 0;
                    }
                    Err(e) => {
                        tracing::error!("Search failed: {}", e);
                    }
                }
            }

            Message::MoveUp => {
                if self.view_mode == ViewMode::Main && self.selected_index > 0 {
                    self.selected_index -= 1;
                    return scroll_if_needed(
                        self.selected_index,
                        self.entries.len(),
                        self.scroll_viewport.as_ref(),
                        -1,
                    );
                }
            }

            Message::MoveDown => {
                if self.view_mode == ViewMode::Main && self.selected_index + 1 < self.entries.len()
                {
                    self.selected_index += 1;
                    return scroll_if_needed(
                        self.selected_index,
                        self.entries.len(),
                        self.scroll_viewport.as_ref(),
                        1,
                    );
                }
            }

            Message::NextPage => {
                if self.view_mode == ViewMode::Main
                    && self.search_query.is_empty()
                    && self.current_page + 1 < self.total_pages()
                {
                    self.current_page += 1;
                    self.load_current_page();
                }
            }

            Message::PrevPage => {
                if self.view_mode == ViewMode::Main
                    && self.search_query.is_empty()
                    && self.current_page > 0
                {
                    self.current_page -= 1;
                    self.load_current_page();
                }
            }

            Message::SelectTab(index) => {
                if self.view_mode == ViewMode::Main && index < self.tabs.len() {
                    self.active_tab = index;
                    self.current_page = 0;
                    self.refresh_entries();
                }
            }

            Message::NextTab => {
                if self.view_mode == ViewMode::Main {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    self.current_page = 0;
                    self.refresh_entries();
                }
            }

            Message::PrevTab => {
                if self.view_mode == ViewMode::Main {
                    self.active_tab = if self.active_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                    self.current_page = 0;
                    self.refresh_entries();
                }
            }

            Message::ToggleWindow => {
                // One-shot mode: only the tray "Show" item routes here, and
                // we're always already shown — so just gain focus.
                return window::oldest().and_then(window::gain_focus);
            }

            Message::HideWindow | Message::CloseOverlay => {
                // One-shot mode: Esc / overlay-click closes any overlay first;
                // pressing it again from Main exits the process.
                if self.view_mode != ViewMode::Main {
                    self.view_mode = ViewMode::Main;
                    return Task::none();
                }
                self.save_window_state();
                return self.hide_window();
            }

            Message::StartDrag => {
                return window::oldest().and_then(window::drag);
            }

            Message::StartResize(direction) => {
                return window::oldest().and_then(move |id| window::drag_resize(id, direction));
            }

            Message::Tick => {
                self.reload_config_if_changed();
                if self.visible && self.last_refresh.elapsed() > Duration::from_secs(2) {
                    self.refresh_entries();
                }
            }

            Message::PollIpc => {
                return self.drain_ipc();
            }

            Message::WindowOpened(id) => {
                let _ = APP_MAIN_WINDOW_ID.set(id);
                tracing::debug!(?id, "main window opened");
            }

            Message::TogglePin => {
                self.pinned = !self.pinned;
                tracing::info!(pinned = self.pinned, "pin toggled");
                // Phase 4 sub-task 4.6: flip the layer-shell layer
                // by emitting the auto-generated LayerChange
                // variant. iced_layershell intercepts it via the
                // TryInto<LayershellCustomActionWithId> impl and
                // pushes the change to the compositor.
                #[cfg(target_os = "linux")]
                {
                    use iced_layershell::reexport::Layer;
                    let new_layer = if self.pinned {
                        Layer::Overlay
                    } else {
                        Layer::Top
                    };
                    return Task::done(Message::LayerChange(new_layer));
                }
            }

            #[cfg(windows)]
            Message::GlobalHotkeyPressed => {
                // Windows: the GUI is one-shot like Linux now, but the hotkey
                // is still registered while we're alive. Gain focus on press.
                let elapsed = self.last_hotkey_time.elapsed();
                if elapsed < Duration::from_millis(300) {
                    return Task::none();
                }
                self.last_hotkey_time = Instant::now();
                return window::oldest().and_then(window::gain_focus);
            }

            Message::ClipboardChanged => {
                self.refresh_entries();
            }

            Message::WindowMoved(position) => {
                // Ignore offscreen coordinates (common during minimization/Win+D)
                if position.x > -10000.0 && position.y > -10000.0 {
                    self.window_state.x = position.x;
                    self.window_state.y = position.y;
                }
            }

            Message::WindowResized(size) => {
                // Ignore "minimized" sizes (e.g. 160x28 observed in logs)
                if size.width > 200.0 && size.height > 100.0 {
                    self.window_state.width = size.width;
                    self.window_state.height = size.height;
                }
            }

            Message::WindowFocused => {
                // Block input for 300ms to prevent capturing stray keystrokes
                // (e.g. the "v" from Ctrl+Shift+V on Windows).
                self.input_blocked_until = Some(Instant::now() + Duration::from_millis(300));
                return delayed_focus_search();
            }

            Message::WindowUnfocused => {
                // Phase 4 sub-task 4.8: hide-on-blur with grace
                // period. Replaces the one-shot model's
                // `process::exit(0)` with `hide_window()` so the
                // daemon stays alive across blur events. The grace
                // window prevents the brief unfocus that some
                // compositors emit while the window is animating
                // in from killing the launcher before the user
                // can interact.
                //
                // Phase 4 sub-task 4.6: pinned launchers don't
                // auto-hide regardless of blur events. The user
                // explicitly opted in to "stay visible".
                if self.pinned || !self.config.gui.hide_on_blur {
                    return Task::none();
                }
                let elapsed = self.last_show_time.elapsed();
                if elapsed < self.config.gui.hide_on_blur_grace() {
                    return Task::none();
                }
                self.save_window_state();
                return self.hide_window();
            }

            Message::TrayMenuEvent(menu_id) => {
                if let Some(ids) = TRAY_MENU_IDS.get() {
                    if menu_id == ids.show.0 {
                        return self.update(Message::ToggleWindow);
                    } else if menu_id == ids.startup.0 {
                        let currently_enabled = crate::startup::is_startup_enabled();
                        let _ = crate::startup::set_startup_enabled(!currently_enabled);
                    } else if menu_id == ids.quit.0 {
                        return self.update(Message::QuitApp);
                    }
                }
            }

            Message::QuitApp => {
                self.save_window_state();
                std::process::exit(0);
            }

            Message::ShowSettings => {
                self.view_mode = ViewMode::Settings;
            }

            Message::HideSettings => {
                self.view_mode = ViewMode::Main;
            }

            Message::ToggleHelp => {
                self.view_mode = if self.view_mode == ViewMode::Help {
                    ViewMode::Main
                } else {
                    ViewMode::Help
                };
            }

            Message::ToggleStartup => {
                let currently_enabled = crate::startup::is_startup_enabled();
                tracing::info!(
                    "Toggling startup: currently={}, setting to {}",
                    currently_enabled,
                    !currently_enabled
                );
                match crate::startup::set_startup_enabled(!currently_enabled) {
                    Ok(()) => tracing::info!("Startup setting changed successfully"),
                    Err(e) => tracing::error!("Failed to change startup setting: {}", e),
                }
            }

            Message::FocusSearch => {
                return iced::widget::operation::focus(search_input_id());
            }

            Message::ToggleEntryPanel(entry_id) => {
                // Toggle: if already inspecting this entry, close. Otherwise
                // open the side panel for the given entry.
                if let ViewMode::EntryPanel(ref current) = self.view_mode {
                    if current == &entry_id {
                        self.view_mode = ViewMode::Main;
                        return Task::none();
                    }
                }
                if let Some(index) = self.entries.iter().position(|e| e.id == entry_id) {
                    self.selected_index = index;
                }
                self.view_mode = ViewMode::EntryPanel(entry_id);
                return scroll_to_selected(self.selected_index, self.entries.len());
            }

            Message::ToggleSelectedEntryPanel => {
                if let Some(entry) = self.entries.get(self.selected_index) {
                    let id = entry.id.clone();
                    return self.update(Message::ToggleEntryPanel(id));
                }
            }

            Message::CloseEntryPanel => {
                self.view_mode = ViewMode::Main;
                return scroll_to_selected(self.selected_index, self.entries.len());
            }

            Message::CopyFromPreview => {
                // Phase 2 paste-back from the side-inspector "Copy"
                // button. Same path as `CopyEntry` once we resolve
                // the entry from the panel's entry_id.
                let entry_clone = if let ViewMode::EntryPanel(ref entry_id) = self.view_mode {
                    self.entries.iter().find(|e| e.id == *entry_id).cloned()
                } else {
                    None
                };
                if let Some(entry) = entry_clone {
                    return self.paste_and_hide(entry);
                }
                self.save_window_state();
                return self.hide_window();
            }

            Message::Scrolled(viewport) => {
                self.scroll_viewport = Some(viewport);
            }

            // Phase 4 sub-task 4.3: catch-all for the variants
            // synthesised by `iced_layershell::to_layer_message`
            // (AnchorChange, SizeChange, MarginChange,
            // SetInputRegion, etc.). iced never emits them on the
            // xdg_toplevel path; iced_layershell uses them
            // internally for compositor round-trips. We don't act
            // on them ourselves.
            _ => {}
        }

        Task::none()
    }

    fn active_tab_filter(&self) -> &TabFilter {
        self.tabs.get(self.active_tab).unwrap_or(&TabFilter::All)
    }

    fn total_pages(&self) -> usize {
        if self.total_count == 0 {
            1
        } else {
            self.total_count.div_ceil(PAGE_SIZE)
        }
    }

    fn load_current_page(&mut self) {
        let filter = self.active_tab_filter().clone();
        let (filter_str, collection_id) = filter.db_filter();
        let offset = self.current_page * PAGE_SIZE;
        let filter_owned = filter_str.to_string();
        let collection_owned = collection_id.map(|s| s.to_string());
        let tag_filters = self.active_tag_filters.clone();
        self.entries = self
            .db
            .call(move |d| {
                if tag_filters.len() == 1 {
                    let tag_id = tag_filters.iter().next().expect("tag exists");
                    d.get_page_filtered_with_tag(
                        offset,
                        PAGE_SIZE,
                        &filter_owned,
                        collection_owned.as_deref(),
                        tag_id,
                    )
                } else {
                    d.get_page_filtered(
                        offset,
                        PAGE_SIZE,
                        &filter_owned,
                        collection_owned.as_deref(),
                    )
                }
                .unwrap_or_default()
            })
            .unwrap_or_default();
        let entries = std::mem::take(&mut self.entries);
        self.entries = self.apply_tag_filter_to_entries(entries);
        self.selected_index = 0;
        self.refresh_tag_cache();
    }

    // ========================================================================
    // Views
    // ========================================================================

    fn view(&self) -> Element<'_, Message> {
        match &self.view_mode {
            ViewMode::Main => self.view_main(),
            ViewMode::Settings => self.view_with_overlay(self.view_main(), self.view_settings()),
            ViewMode::Help => self.view_with_overlay(self.view_main(), self.view_help()),
            ViewMode::EntryPanel(entry_id) => self.view_main_with_panel(entry_id),
            ViewMode::ConfirmDelete(entry_id) => {
                self.view_with_overlay(self.view_main(), self.view_confirm_delete(entry_id))
            }
        }
    }

    /// Render the main list with a side panel showing the focused entry's
    /// metadata + actions. Replaces the modal-style image preview.
    fn view_main_with_panel(&self, entry_id: &str) -> Element<'_, Message> {
        let main = self.view_main();
        let panel = self.view_entry_panel(entry_id);
        // Two-column layout: main list (3 parts) + side panel (2 parts).
        // The window stays at 420px so the split is roughly 252 / 168 px.
        container(
            row![
                container(main).width(Length::FillPortion(3)),
                container(panel)
                    .width(Length::FillPortion(2))
                    .style(styles::side_panel),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_with_overlay<'a>(
        &self,
        background: Element<'a, Message>,
        modal: Element<'a, Message>,
    ) -> Element<'a, Message> {
        // Wrap modal in mouse_area to block clicks on background
        let overlay = mouse_area(
            container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(styles::overlay),
        )
        .on_press(Message::CloseOverlay);

        iced::widget::stack![background, overlay].into()
    }

    fn view_main(&self) -> Element<'_, Message> {
        let title_bar = self.view_title_bar();
        let search_section = self.view_search();
        let tab_bar = self.view_tabs();
        let tag_bar = self.view_tag_filter_bar();
        let entry_list = self.view_entries();
        let bulk_bar = self.view_bulk_bar();
        let status_bar = self.view_status();

        let content = column![
            title_bar,
            search_section,
            tab_bar,
            tag_bar,
            entry_list,
            bulk_bar,
            status_bar,
        ]
        .spacing(0);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(styles::app_container)
            .into()
    }

    fn view_title_bar(&self) -> Element<'_, Message> {
        // Draggable title area (fills most of the bar). On
        // layer-shell windows the drag is internal — see
        // sub-task 4.5 below — but on xdg_toplevel the
        // compositor moves the window when we issue
        // `window::drag`.
        let title = mouse_area(
            row![
                text("Ditox").size(13).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
            ]
            .padding([0, 12])
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::StartDrag);

        // Phase 4 sub-task 4.6: pin button. Toggles between
        // `Layer::Top` (default; below system overlays) and
        // `Layer::Overlay` (above everything including
        // fullscreen). Visual feedback: filled vs outlined
        // pushpin icon.
        let pin_icon = if self.pinned {
            icons::PIN_FILL
        } else {
            icons::PIN
        };
        let pin_button = button(icon(pin_icon).size(11).color(if self.pinned {
            colors::ACCENT
        } else {
            colors::TEXT_MUTED
        }))
        .on_press(Message::TogglePin)
        .padding([4, 6])
        .style(styles::header_icon_button);

        // Small resize grip in top-right corner (diagonal lines icon)
        let resize_grip = mouse_area(
            container(
                icon(icons::GRIP_VERTICAL)
                    .size(10)
                    .color(colors::TEXT_MUTED),
            )
            .padding([4, 8]),
        )
        .on_press(Message::StartResize(Direction::NorthEast));

        container(row![title, pin_button, resize_grip,].align_y(iced::Alignment::Center))
            .width(Length::Fill)
            .padding([4, 0])
            .style(styles::title_bar)
            .into()
    }

    fn view_search(&self) -> Element<'_, Message> {
        let interactive = self.view_mode == ViewMode::Main;

        let search_input = text_input("Search clipboard history...", &self.search_query)
            .id(search_input_id())
            .on_input_maybe(interactive.then_some(Message::SearchChanged as fn(String) -> Message))
            .padding(10)
            .width(Length::Fill)
            .style(styles::search_input);

        let help_btn = button(icon(icons::QUESTION).size(14))
            .style(styles::action_btn)
            .on_press(Message::ToggleHelp)
            .padding([6, 10]);

        let settings_btn = button(icon(icons::GEAR).size(14))
            .style(styles::action_btn)
            .on_press_maybe(interactive.then_some(Message::ShowSettings))
            .padding([6, 10]);

        container(
            row![search_input, help_btn, settings_btn]
                .spacing(6)
                .align_y(iced::Alignment::Center),
        )
        .padding([10, 12])
        .into()
    }

    fn view_tabs(&self) -> Element<'_, Message> {
        let interactive = self.view_mode == ViewMode::Main;
        let tabs: Vec<Element<_>> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let is_active = i == self.active_tab;
                button(text(tab.label()).size(11))
                    .style(if is_active {
                        styles::tab_active
                    } else {
                        styles::tab_inactive
                    })
                    .on_press_maybe(interactive.then_some(Message::SelectTab(i)))
                    .padding([5, 12])
                    .into()
            })
            .collect();

        container(
            Row::with_children(tabs)
                .spacing(4)
                .align_y(iced::Alignment::Center),
        )
        .padding([6, 12])
        .width(Length::Fill)
        .into()
    }

    fn view_tag_filter_bar(&self) -> Element<'_, Message> {
        let interactive = self.view_mode == ViewMode::Main;
        let mut chips: Vec<Element<'_, Message>> = Vec::new();

        let all_active = self.active_tag_filters.is_empty();
        chips.push(
            button(text("All tags").size(10))
                .style(if all_active {
                    styles::tab_active
                } else {
                    styles::tab_inactive
                })
                .on_press_maybe(interactive.then_some(Message::SelectTagFilter(None)))
                .padding([4, 8])
                .into(),
        );

        for tag in self.all_tags.iter().take(8) {
            let active = self.active_tag_filters.contains(&tag.id);
            let label = tag
                .color
                .as_ref()
                .map(|color| format!("{} #{}", color, tag.name))
                .unwrap_or_else(|| format!("#{}", tag.name));
            chips.push(
                button(text(label).size(10))
                    .style(if active {
                        styles::tab_active
                    } else {
                        styles::tab_inactive
                    })
                    .on_press_maybe(
                        interactive.then_some(Message::SelectTagFilter(Some(tag.id.clone()))),
                    )
                    .padding([4, 8])
                    .into(),
            );
        }

        container(Row::with_children(chips).spacing(4))
            .padding([4, 12])
            .width(Length::Fill)
            .into()
    }

    fn view_bulk_bar(&self) -> Element<'_, Message> {
        if !self.multi_select {
            return container(
                row![
                    button(text("Multi-select").size(10))
                        .style(styles::tab_inactive)
                        .on_press(Message::ToggleMultiSelect)
                        .padding([4, 8]),
                    Space::new().width(Length::Fill),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([4, 12])
            .into();
        }

        let count = self.selected_entry_ids.len();
        let mut row_items: Row<'_, Message> = row![
            button(text("Done").size(10))
                .style(styles::tab_active)
                .on_press(Message::ToggleMultiSelect)
                .padding([4, 8]),
            text(format!("{} selected", count))
                .size(10)
                .color(colors::TEXT_MUTED),
            button(text("Copy joined").size(10))
                .style(styles::primary_btn)
                .on_press(Message::BulkCopyJoined)
                .padding([4, 8]),
            button(text("Delete").size(10))
                .style(styles::delete_btn)
                .on_press(Message::BulkDeleteSelected)
                .padding([4, 8]),
            text_input("tag", &self.bulk_tag_input)
                .on_input(Message::BulkTagInputChanged)
                .padding(5)
                .size(10)
                .width(Length::Fixed(64.0))
                .style(styles::search_input),
            button(text("Tag").size(10))
                .style(styles::tab_inactive)
                .on_press(Message::BulkTagSelected)
                .padding([4, 8]),
            text_input("transform", &self.bulk_transform_input)
                .on_input(Message::BulkTransformInputChanged)
                .padding(5)
                .size(10)
                .width(Length::Fixed(82.0))
                .style(styles::search_input),
            button(text("Apply").size(10))
                .style(styles::tab_inactive)
                .on_press(Message::BulkTransformSelected)
                .padding([4, 8]),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        for collection in &self.all_collections {
            row_items = row_items.push(
                button(text(format!("Move: {}", collection.name)).size(10))
                    .style(styles::tab_inactive)
                    .on_press(Message::BulkMoveSelectedToCollection(collection.id.clone()))
                    .padding([4, 8]),
            );
        }

        container(row_items).padding([4, 12]).into()
    }

    fn view_entries(&self) -> Element<'_, Message> {
        if self.entries.is_empty() {
            container(
                column![
                    text("No entries").size(14).color(colors::TEXT_SECONDARY),
                    text("Copy something to get started")
                        .size(12)
                        .color(colors::TEXT_MUTED),
                ]
                .spacing(6)
                .align_x(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            let items: Vec<Element<_>> = self
                .entries
                .iter()
                .enumerate()
                .map(|(i, entry)| self.view_entry_row(i, entry))
                .collect();

            scrollable(Column::with_children(items).spacing(2).padding([0, 8]))
                .id(ENTRY_LIST_ID)
                .height(Length::Fill)
                .style(styles::scrollable_style)
                .on_scroll(Message::Scrolled)
                .into()
        }
    }

    fn view_entry_row(&self, index: usize, entry: &Entry) -> Element<'_, Message> {
        let is_selected = index == self.selected_index;
        let entry_id = entry.id.clone();
        let entry_id_fav = entry.id.clone();
        let entry_id_preview = entry.id.clone();
        let entry_id_select = entry.id.clone();
        let interactive = self.view_mode == ViewMode::Main;
        let is_bulk_selected = self.selected_entry_ids.contains(&entry.id);

        // Phase 4 sub-task 4.10: hotkey number prefix for the first
        // 10 entries. `1`..`9` map to indices 0..8; `0` maps to
        // index 9 (the 10th entry). Beyond index 9 the prefix is
        // blank so the column stays aligned. Phase 5 will let the
        // user actually invoke these hotkeys via Alt+<digit>;
        // today the prefix is informational so the user learns
        // the upcoming binding.
        let hotkey_prefix: Element<'_, Message> = match index {
            0..=8 => text(format!("{}", index + 1))
                .size(10)
                .color(colors::TEXT_MUTED)
                .into(),
            9 => text("0").size(10).color(colors::TEXT_MUTED).into(),
            _ => text("  ").size(10).into(),
        };

        // Favorite indicator
        let fav_star = if entry.favorite {
            icon(icons::STAR_FILL).size(12).color(colors::WARNING)
        } else {
            text(" ").size(12)
        };

        // Phase 4 sub-task 4.10: glyph cluster between preview and
        // time. Each glyph is rendered only when its underlying
        // state is set on the entry; the column compresses
        // gracefully when nothing applies.
        let collection_glyph: Option<Element<'_, Message>> =
            entry.collection_id.as_ref().map(|_| {
                icon(icons::FOLDER)
                    .size(11)
                    .color(colors::TEXT_MUTED)
                    .into()
            });
        let notes_glyph: Option<Element<'_, Message>> =
            if entry.notes.as_ref().is_some_and(|n| !n.is_empty()) {
                Some(
                    icon(icons::JOURNAL_TEXT)
                        .size(11)
                        .color(colors::TEXT_MUTED)
                        .into(),
                )
            } else {
                None
            };
        let tag_glyph: Option<Element<'_, Message>> = self
            .entry_tags
            .get(&entry.id)
            .filter(|tags| !tags.is_empty())
            .map(|_| icon(icons::TAG).size(11).color(colors::TEXT_MUTED).into());

        // Time
        let time = text(entry.relative_time())
            .size(10)
            .color(colors::TEXT_MUTED);

        // Build entry content based on type
        let entry_content: Row<'_, Message> = match entry.entry_type {
            EntryType::Image => {
                // Image entry: thumbnail + filename. `entry.content` is the
                // content-addressable hash now; derive the real path for
                // iced's image loader.
                let path_string = entry
                    .image_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let thumbnail = self.view_thumbnail(&path_string, 40, 40);
                let filename = text(entry.preview(30)).size(12).color(if is_selected {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                });

                let mut img_row: Row<'_, Message> = row![
                    container(hotkey_prefix).width(Length::Fixed(14.0)),
                    thumbnail,
                    container(fav_star).width(Length::Fixed(18.0)),
                    filename,
                    Space::new().width(Length::Fill),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center);
                if let Some(g) = collection_glyph {
                    img_row = img_row.push(g);
                }
                if let Some(g) = notes_glyph {
                    img_row = img_row.push(g);
                }
                if let Some(g) = tag_glyph {
                    img_row = img_row.push(g);
                }
                img_row.push(time)
            }
            EntryType::Text => {
                // Text entry: type badge + (optional color swatch) + preview text
                let type_badge = container(icon(icons::FILE_TEXT).size(10))
                    .padding([2, 5])
                    .style(styles::badge_text);

                let preview = text(entry.preview(45)).size(12).color(if is_selected {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                });

                // Phase 3 sub-task 3.3: scan the entry's text for a
                // CSS-style color literal and render a 12×12 px
                // filled square before the preview when one's
                // found. The Container's `bg` is set via a per-row
                // style closure that captures the parsed RGBA.
                let swatch: Option<Element<'_, Message>> =
                    ditox_core::color::detect_first_color(&entry.content).map(|c| {
                        let bg = iced::Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0);
                        container(Space::new())
                            .width(Length::Fixed(12.0))
                            .height(Length::Fixed(12.0))
                            .style(move |_theme: &iced::Theme| container::Style {
                                background: Some(iced::Background::Color(bg)),
                                border: iced::Border {
                                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                                    width: 1.0,
                                    radius: 2.0.into(),
                                },
                                ..container::Style::default()
                            })
                            .into()
                    });

                let mut row_items: Row<'_, Message> = row![
                    container(hotkey_prefix).width(Length::Fixed(14.0)),
                    type_badge,
                    container(fav_star).width(Length::Fixed(18.0)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center);

                if let Some(s) = swatch {
                    row_items = row_items.push(s);
                }

                row_items = row_items
                    .push(preview)
                    .push(Space::new().width(Length::Fill));

                if let Some(g) = collection_glyph {
                    row_items = row_items.push(g);
                }
                if let Some(g) = notes_glyph {
                    row_items = row_items.push(g);
                }
                if let Some(g) = tag_glyph {
                    row_items = row_items.push(g);
                }

                row_items.push(time)
            }
        };

        // One-shot UX: a click on any entry — text or image — copies and
        // exits. The side panel (Tab key) is the way to inspect an entry
        // without copying. `entry_id_preview` is intentionally unused now
        // but kept to minimise churn; suppressed below.
        let _ = &entry_id_preview;
        let on_press = if interactive && self.multi_select {
            Some(Message::ToggleEntrySelected(entry_id_select))
        } else if interactive {
            Some(Message::CopyEntry(index))
        } else {
            None
        };

        let entry_btn = button(entry_content)
            .style(if is_selected || is_bulk_selected {
                styles::entry_row_selected
            } else {
                styles::entry_row
            })
            .on_press_maybe(on_press)
            .padding([8, 10])
            .width(Length::Fill);

        // Action buttons - fixed width for alignment, disabled when modal open
        let fav_btn = button(
            icon(if entry.favorite {
                icons::STAR_FILL
            } else {
                icons::STAR
            })
            .size(12)
            .color(if entry.favorite {
                colors::WARNING
            } else {
                colors::TEXT_MUTED
            }),
        )
        .style(styles::action_btn)
        .on_press_maybe(interactive.then_some(Message::ToggleFavorite(entry_id_fav)))
        .padding([6, 8])
        .width(Length::Fixed(32.0));

        let del_btn = button(icon(icons::TRASH).size(12))
            .style(styles::delete_btn)
            .on_press_maybe(interactive.then_some(Message::DeleteEntry(entry_id)))
            .padding([6, 8])
            .width(Length::Fixed(32.0));

        // Phase 4 sub-task 4.9: tooltip-as-preview. Wrap the
        // entry button in iced's built-in `tooltip` widget. Hover
        // → preview appears to the right with a longer slice of
        // the entry text (text entries) or a larger thumbnail
        // (image entries). iced's tooltip already handles the
        // hover-delay + auto-hide-on-leave so no extra state
        // tracking is needed at the DitoxApp level.
        let tooltip_content: Element<'_, Message> = match entry.entry_type {
            EntryType::Text => container(
                text(entry.preview(500))
                    .size(11)
                    .color(colors::TEXT_PRIMARY),
            )
            .max_width(360.0)
            .padding(8)
            .style(styles::tooltip_panel)
            .into(),
            EntryType::Image => {
                let path_string = entry
                    .image_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                container(self.view_thumbnail(&path_string, 256, 256))
                    .padding(8)
                    .style(styles::tooltip_panel)
                    .into()
            }
        };

        let entry_with_tooltip = tooltip(entry_btn, tooltip_content, tooltip::Position::Right);

        let selected_mark: Element<'_, Message> = if self.multi_select {
            button(text(if is_bulk_selected { "x" } else { " " }).size(10))
                .style(if is_bulk_selected {
                    styles::tab_active
                } else {
                    styles::tab_inactive
                })
                .on_press_maybe(
                    interactive.then_some(Message::ToggleEntrySelected(entry.id.clone())),
                )
                .padding([4, 7])
                .width(Length::Fixed(28.0))
                .into()
        } else {
            Space::new().width(0).into()
        };

        row![selected_mark, entry_with_tooltip, fav_btn, del_btn]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .padding([0, 4])
            .into()
    }

    /// Render a thumbnail for an image entry
    /// Returns a container with the image or a placeholder if loading fails
    fn view_thumbnail(&self, path: &str, width: u16, height: u16) -> Element<'_, Message> {
        let path_buf = std::path::Path::new(path);

        if path_buf.exists() {
            // Use cached handle if available, otherwise create from path
            // (iced internally caches the actual image data, but Handle::from_path
            // still involves string allocation on each call)
            let handle = self
                .image_cache
                .get(path)
                .cloned()
                .unwrap_or_else(|| iced_image::Handle::from_path(path));

            container(
                iced_image(handle)
                    .content_fit(ContentFit::Contain)
                    .width(Length::Fixed(width as f32))
                    .height(Length::Fixed(height as f32)),
            )
            .width(Length::Fixed(width as f32 + 4.0)) // +4 for border/padding
            .height(Length::Fixed(height as f32 + 4.0))
            .style(styles::thumbnail_container)
            .into()
        } else {
            // Placeholder for missing images
            container(icon(icons::IMAGE).size(16).color(colors::TEXT_MUTED))
                .width(Length::Fixed(width as f32 + 4.0))
                .height(Length::Fixed(height as f32 + 4.0))
                .style(styles::thumbnail_placeholder)
                .into()
        }
    }

    fn view_status(&self) -> Element<'_, Message> {
        // Floating-launcher status bar:  "{n} items · Press Tab to preview · vX.Y.Z"
        // Search/page indicators replace the items count when relevant.
        let primary: String = if !self.search_query.is_empty() {
            format!("{}/{} matches", self.entries.len(), self.total_count)
        } else if self.total_pages() > 1 {
            format!(
                "{} items · {}/{}",
                self.total_count,
                self.current_page + 1,
                self.total_pages()
            )
        } else {
            format!("{} items", self.total_count)
        };

        let hint: Element<'_, Message> = if self.is_searching {
            text("Searching…").size(10).color(colors::TEXT_MUTED).into()
        } else {
            text("· Press Tab to preview")
                .size(10)
                .color(colors::TEXT_MUTED)
                .into()
        };

        let version = format!("· v{}", env!("CARGO_PKG_VERSION"));

        container(
            row![
                text(primary).size(10).color(colors::ACCENT),
                Space::new().width(6),
                hint,
                Space::new().width(Length::Fill),
                text(version).size(10).color(colors::TEXT_MUTED),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center),
        )
        .padding([6, 12])
        .width(Length::Fill)
        .style(styles::status_bar)
        .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let startup_enabled = crate::startup::is_startup_enabled();
        let status: Element<'_, Message> = self
            .settings_status
            .as_ref()
            .map(|s| text(s.clone()).size(11).color(colors::TEXT_MUTED).into())
            .unwrap_or_else(|| Space::new().height(0).into());

        let content = column![
            text("Settings").size(16).color(colors::TEXT_PRIMARY),
            Space::new().height(10),
            row![
                text("Run on startup")
                    .size(12)
                    .color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                button(text(if startup_enabled { "ON" } else { "OFF" }).size(11))
                    .style(if startup_enabled {
                        styles::tab_active
                    } else {
                        styles::tab_inactive
                    })
                    .on_press(Message::ToggleStartup)
                    .padding([4, 12]),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(10),
            row![
                text("Hide on blur").size(12).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                button(
                    text(if self.config.gui.hide_on_blur {
                        "ON"
                    } else {
                        "OFF"
                    })
                    .size(11)
                )
                .style(if self.config.gui.hide_on_blur {
                    styles::tab_active
                } else {
                    styles::tab_inactive
                })
                .on_press(Message::ToggleSettingsHideOnBlur)
                .padding([4, 12]),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(10),
            text("General").size(12).color(colors::TEXT_SECONDARY),
            row![
                text("Max entries").size(12).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                text_input("500", &self.settings_max_entries)
                    .on_input(Message::SettingsMaxEntriesChanged)
                    .padding(6)
                    .size(11)
                    .width(Length::Fixed(96.0))
                    .style(styles::search_input),
            ]
            .align_y(iced::Alignment::Center),
            row![
                text("Poll interval ms")
                    .size(12)
                    .color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                text_input("250", &self.settings_poll_interval_ms)
                    .on_input(Message::SettingsPollIntervalChanged)
                    .padding(6)
                    .size(11)
                    .width(Length::Fixed(96.0))
                    .style(styles::search_input),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(10),
            text("Theme").size(12).color(colors::TEXT_SECONDARY),
            self.view_setting_text(
                "Selected",
                &self.settings_theme_selected,
                Message::SettingsThemeSelectedChanged
            ),
            self.view_setting_text(
                "Text",
                &self.settings_theme_text,
                Message::SettingsThemeTextChanged
            ),
            self.view_setting_text(
                "Border",
                &self.settings_theme_border,
                Message::SettingsThemeBorderChanged
            ),
            self.view_setting_text(
                "Muted",
                &self.settings_theme_muted,
                Message::SettingsThemeMutedChanged
            ),
            Space::new().height(10),
            self.view_collections_settings(),
            Space::new().height(10),
            status,
            row![
                button(text("Save").size(11))
                    .style(styles::primary_btn)
                    .on_press(Message::SaveSettings)
                    .padding([8, 20]),
                Space::new().width(Length::Fill),
                button(text("Close").size(11))
                    .style(styles::tab_inactive)
                    .on_press(Message::HideSettings)
                    .padding([8, 20]),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .spacing(6)
        .padding(20)
        .width(Length::Fixed(380.0));

        container(scrollable(content).height(Length::Fixed(500.0)))
            .style(styles::modal)
            .into()
    }

    fn view_collections_settings(&self) -> Element<'_, Message> {
        let status: Element<'_, Message> = self
            .collection_status
            .as_ref()
            .map(|s| text(s.clone()).size(11).color(colors::TEXT_MUTED).into())
            .unwrap_or_else(|| Space::new().height(0).into());

        let mut collection_list = Column::new().spacing(4);
        for collection in &self.all_collections {
            let selected = self.editing_collection_id.as_deref() == Some(collection.id.as_str());
            let label = collection
                .color
                .as_ref()
                .map(|color| format!("{} {}", color, collection.name))
                .unwrap_or_else(|| collection.name.clone());
            collection_list = collection_list.push(
                button(text(label).size(11))
                    .style(if selected {
                        styles::tab_active
                    } else {
                        styles::tab_inactive
                    })
                    .on_press(Message::SelectCollectionForEdit(collection.id.clone()))
                    .padding([4, 8])
                    .width(Length::Fill),
            );
        }

        column![
            text("Collections").size(12).color(colors::TEXT_SECONDARY),
            collection_list,
            row![
                text_input("name", &self.collection_name_input)
                    .on_input(Message::CollectionNameInputChanged)
                    .padding(6)
                    .size(11)
                    .width(Length::Fill)
                    .style(styles::search_input),
                text_input("#rrggbb", &self.collection_color_input)
                    .on_input(Message::CollectionColorInputChanged)
                    .padding(6)
                    .size(11)
                    .width(Length::Fixed(92.0))
                    .style(styles::search_input),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            row![
                button(text("Create").size(11))
                    .style(styles::primary_btn)
                    .on_press(Message::CreateCollection)
                    .padding([5, 10]),
                button(text("Save").size(11))
                    .style(styles::tab_inactive)
                    .on_press(Message::SaveCollection)
                    .padding([5, 10]),
                button(text("Delete").size(11))
                    .style(styles::delete_btn)
                    .on_press(Message::DeleteSelectedCollection)
                    .padding([5, 10]),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            status,
        ]
        .spacing(6)
        .into()
    }

    fn view_setting_text(
        &self,
        label: &'static str,
        value: &str,
        on_input: fn(String) -> Message,
    ) -> Element<'_, Message> {
        row![
            text(label).size(12).color(colors::TEXT_SECONDARY),
            Space::new().width(Length::Fill),
            text_input("#rrggbb", value)
                .on_input(on_input)
                .padding(6)
                .size(11)
                .width(Length::Fixed(120.0))
                .style(styles::search_input),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_help(&self) -> Element<'_, Message> {
        let content = column![
            // Header
            row![
                text("Keyboard Shortcuts")
                    .size(16)
                    .color(colors::TEXT_PRIMARY),
                Space::new().width(Length::Fill),
                button(icon(icons::X).size(12))
                    .style(styles::action_btn)
                    .on_press(Message::ToggleHelp)
                    .padding([4, 8]),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(16),
            text("Navigation").size(12).color(colors::TEXT_SECONDARY),
            row![
                text("Up / Down").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Move selection").size(11).color(colors::TEXT_MUTED),
            ],
            row![
                text("Enter").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Copy and close").size(11).color(colors::TEXT_MUTED),
            ],
            row![
                text("Escape").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Hide window").size(11).color(colors::TEXT_MUTED),
            ],
            row![
                text("Tab").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Toggle preview panel")
                    .size(11)
                    .color(colors::TEXT_MUTED),
            ],
            row![
                text("Shift+Left/Right").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Switch tabs").size(11).color(colors::TEXT_MUTED),
            ],
            row![
                text("Left / Right").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Navigate pages").size(11).color(colors::TEXT_MUTED),
            ],
            Space::new().height(12),
            text("Actions").size(12).color(colors::TEXT_SECONDARY),
            row![
                text("?").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Toggle help").size(11).color(colors::TEXT_MUTED),
            ],
            Space::new().height(12),
            text("Global").size(12).color(colors::TEXT_SECONDARY),
            row![
                text("Ctrl+Shift+V").size(11).color(colors::ACCENT),
                Space::new().width(Length::Fill),
                text("Show/Hide Ditox").size(11).color(colors::TEXT_MUTED),
            ],
            Space::new().height(20),
            button(text("Close").size(11))
                .style(styles::primary_btn)
                .on_press(Message::ToggleHelp)
                .padding([8, 24]),
        ]
        .spacing(4)
        .padding(20)
        .width(Length::Fixed(300.0));

        container(content).style(styles::modal).into()
    }

    /// Side panel rendered next to the entry list when `view_mode ==
    /// EntryPanel(_)`. Mirrors the floating-launcher reference design:
    /// type label + relative timestamp + content area + Copy / Delete
    /// buttons + char/line counter (text only).
    fn view_entry_panel(&self, entry_id: &str) -> Element<'_, Message> {
        let entry = self.entries.iter().find(|e| e.id == entry_id);

        let content = if let Some(entry) = entry {
            let entry_id_owned = entry.id.clone();

            // Header: type label + close button
            let type_label = match entry.entry_type {
                EntryType::Text => "Text",
                EntryType::Image => "Image",
            };
            let header = row![
                text(type_label).size(13).color(colors::TEXT_PRIMARY),
                Space::new().width(Length::Fill),
                button(icon(icons::X).size(11))
                    .style(styles::action_btn)
                    .on_press(Message::CloseEntryPanel)
                    .padding([3, 6]),
            ]
            .align_y(iced::Alignment::Center);

            // Subhead: "Copied <relative_time>"
            let subhead = row![
                text(format!("Copied {}", entry.relative_time()))
                    .size(10)
                    .color(colors::TEXT_MUTED),
                Space::new().width(Length::Fill),
                if entry.favorite {
                    icon(icons::STAR_FILL).size(11).color(colors::WARNING)
                } else {
                    icon(icons::STAR).size(11).color(colors::TEXT_MUTED)
                },
            ]
            .align_y(iced::Alignment::Center);

            // Content area: image thumbnail or text excerpt
            let body: Element<'_, Message> = match entry.entry_type {
                EntryType::Image => {
                    let path_buf = entry.image_path().unwrap_or_default();
                    let path = path_buf.to_string_lossy().into_owned();
                    let image_height = (180.0 * self.image_zoom).clamp(45.0, 720.0);
                    if path_buf.exists() {
                        let image = container(
                            iced_image(iced_image::Handle::from_path(&path))
                                .content_fit(ContentFit::Contain)
                                .width(Length::Fill)
                                .height(Length::Fixed(image_height)),
                        )
                        .width(Length::Fill)
                        .style(styles::preview_image_container)
                        .padding(6);
                        column![
                            row![
                                button(text("-").size(11))
                                    .style(styles::tab_inactive)
                                    .on_press(Message::ImageZoomOut)
                                    .padding([3, 8]),
                                text(format!("{}%", (self.image_zoom * 100.0).round() as u32))
                                    .size(10)
                                    .color(colors::TEXT_MUTED),
                                button(text("+").size(11))
                                    .style(styles::tab_inactive)
                                    .on_press(Message::ImageZoomIn)
                                    .padding([3, 8]),
                                button(text("Fit").size(10))
                                    .style(styles::tab_inactive)
                                    .on_press(Message::ImageZoomFit)
                                    .padding([3, 8]),
                                button(text("Actual").size(10))
                                    .style(styles::tab_inactive)
                                    .on_press(Message::ImageZoomActualSize)
                                    .padding([3, 8]),
                                button(text("Open").size(10))
                                    .style(styles::tab_inactive)
                                    .on_press(Message::OpenImageExternal(path.clone()))
                                    .padding([3, 8]),
                            ]
                            .spacing(4)
                            .align_y(iced::Alignment::Center),
                            image,
                        ]
                        .spacing(6)
                        .into()
                    } else {
                        container(
                            column![
                                icon(icons::IMAGE).size(32).color(colors::TEXT_MUTED),
                                text("Image not found").size(11).color(colors::TEXT_MUTED),
                            ]
                            .spacing(6)
                            .align_x(iced::Alignment::Center),
                        )
                        .width(Length::Fill)
                        .height(Length::Fixed(180.0))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .style(styles::preview_image_container)
                        .into()
                    }
                }
                EntryType::Text => {
                    let excerpt = entry.preview(400);
                    container(text(excerpt).size(11).color(colors::TEXT_SECONDARY))
                        .padding(8)
                        .width(Length::Fill)
                        .style(styles::preview_image_container)
                        .into()
                }
            };

            // Footer: char/line count for text, byte size for image
            let footer_text = match entry.entry_type {
                EntryType::Text => {
                    let chars = entry.content.chars().count();
                    let lines = entry.content.lines().count().max(1);
                    format!("{} chars · {} lines", chars, lines)
                }
                EntryType::Image => {
                    if entry.byte_size < 1024 {
                        format!("{} B", entry.byte_size)
                    } else if entry.byte_size < 1024 * 1024 {
                        format!("{:.1} KB", entry.byte_size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", entry.byte_size as f64 / (1024.0 * 1024.0))
                    }
                }
            };

            let current_tags = self.entry_tags.get(&entry.id).cloned().unwrap_or_default();
            let tag_chips: Vec<Element<'_, Message>> = if current_tags.is_empty() {
                vec![text("No tags").size(10).color(colors::TEXT_MUTED).into()]
            } else {
                current_tags
                    .into_iter()
                    .map(|tag| {
                        button(text(format!("#{} x", tag.name)).size(10))
                            .style(styles::tab_inactive)
                            .on_press(Message::RemoveTagFromEntry {
                                entry_id: entry.id.clone(),
                                tag_id: tag.id,
                            })
                            .padding([3, 6])
                            .into()
                    })
                    .collect()
            };

            let tag_editor = column![
                text("Tags").size(11).color(colors::TEXT_SECONDARY),
                Row::with_children(tag_chips).spacing(4),
                row![
                    text_input("add tag", &self.tag_input)
                        .on_input(Message::TagInputChanged)
                        .padding(6)
                        .size(11)
                        .style(styles::search_input),
                    button(text("Add").size(10))
                        .style(styles::primary_btn)
                        .on_press(Message::AddTagToEntry(entry.id.clone()))
                        .padding([5, 8]),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(6);

            let current_collection = entry.collection_id.as_deref();
            let mut collection_buttons: Vec<Element<'_, Message>> = Vec::new();
            collection_buttons.push(
                button(text("Uncollected").size(10))
                    .style(if current_collection.is_none() {
                        styles::tab_active
                    } else {
                        styles::tab_inactive
                    })
                    .on_press(Message::MoveEntryToCollection {
                        entry_id: entry.id.clone(),
                        collection_id: None,
                    })
                    .padding([3, 6])
                    .into(),
            );
            for collection in self.all_collections.iter().take(5) {
                let active = current_collection == Some(collection.id.as_str());
                collection_buttons.push(
                    button(text(collection.name.clone()).size(10))
                        .style(if active {
                            styles::tab_active
                        } else {
                            styles::tab_inactive
                        })
                        .on_press(Message::MoveEntryToCollection {
                            entry_id: entry.id.clone(),
                            collection_id: Some(collection.id.clone()),
                        })
                        .padding([3, 6])
                        .into(),
                );
            }
            let collection_editor = column![
                text("Collection").size(11).color(colors::TEXT_SECONDARY),
                Row::with_children(collection_buttons).spacing(4),
            ]
            .spacing(6);

            let hotkey_editor = column![
                text("Hotkey").size(11).color(colors::TEXT_SECONDARY),
                row![
                    text(
                        entry
                            .global_hotkey
                            .clone()
                            .unwrap_or_else(|| "Not bound".to_string())
                    )
                    .size(10)
                    .color(colors::TEXT_MUTED),
                    Space::new().width(Length::Fill),
                    button(text("Bind").size(10))
                        .style(styles::tab_inactive)
                        .on_press(Message::SetEntryGlobalHotkey(entry.id.clone()))
                        .padding([3, 6]),
                    button(text("Clear").size(10))
                        .style(styles::delete_btn)
                        .on_press(Message::ClearEntryGlobalHotkey(entry.id.clone()))
                        .padding([3, 6]),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(6);

            // Action buttons: Copy + Delete (compact for the narrow panel).
            let actions = column![
                button(
                    row![
                        icon(icons::CIRCLE_FILL).size(8),
                        text("Copy Again").size(11)
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                )
                .style(styles::primary_btn)
                .on_press(Message::CopyFromPreview)
                .padding([7, 12])
                .width(Length::Fill),
                button(
                    row![icon(icons::TRASH).size(11), text("Delete").size(11)]
                        .spacing(6)
                        .align_y(iced::Alignment::Center)
                )
                .style(styles::delete_btn)
                .on_press(Message::DeleteEntry(entry_id_owned))
                .padding([7, 12])
                .width(Length::Fill),
            ]
            .spacing(6);

            column![
                header,
                Space::new().height(4),
                subhead,
                Space::new().height(10),
                body,
                Space::new().height(10),
                tag_editor,
                Space::new().height(10),
                collection_editor,
                Space::new().height(10),
                hotkey_editor,
                Space::new().height(10),
                actions,
                Space::new().height(Length::Fill),
                text(footer_text).size(10).color(colors::TEXT_MUTED),
            ]
            .spacing(0)
            .padding(12)
        } else {
            column![
                text("Entry not found").size(12).color(colors::TEXT_MUTED),
                Space::new().height(8),
                button(text("Close").size(11))
                    .style(styles::primary_btn)
                    .on_press(Message::CloseEntryPanel)
                    .padding([6, 12]),
            ]
            .padding(12)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_confirm_delete(&self, entry_id: &str) -> Element<'_, Message> {
        let entry = self.entries.iter().find(|e| e.id == entry_id);

        let content = if let Some(entry) = entry {
            let preview_text = entry.preview(40);
            let entry_id_confirm = entry.id.clone();

            column![
                // Header with warning icon
                row![
                    icon(icons::STAR_FILL).size(16).color(colors::WARNING),
                    Space::new().width(8),
                    text("Delete Favorite?")
                        .size(16)
                        .color(colors::TEXT_PRIMARY),
                ]
                .align_y(iced::Alignment::Center),
                Space::new().height(16),
                // Warning message
                text("This entry is marked as a favorite.")
                    .size(12)
                    .color(colors::TEXT_SECONDARY),
                Space::new().height(8),
                // Entry preview
                container(text(preview_text).size(11).color(colors::TEXT_MUTED))
                    .padding([8, 12])
                    .width(Length::Fill)
                    .style(styles::thumbnail_container),
                Space::new().height(16),
                // Action buttons
                row![
                    button(text("Cancel").size(11))
                        .style(styles::tab_inactive)
                        .on_press(Message::CancelDelete)
                        .padding([8, 20]),
                    Space::new().width(Length::Fill),
                    button(row![icon(icons::TRASH).size(12), text(" Delete").size(11)].spacing(4))
                        .style(styles::delete_btn)
                        .on_press(Message::ConfirmDeleteEntry(entry_id_confirm))
                        .padding([8, 16]),
                ]
                .align_y(iced::Alignment::Center),
            ]
            .padding(20)
            .width(Length::Fixed(320.0))
        } else {
            // Entry not found
            column![
                text("Entry not found").size(14).color(colors::TEXT_MUTED),
                Space::new().height(16),
                button(text("Close").size(11))
                    .style(styles::primary_btn)
                    .on_press(Message::CancelDelete)
                    .padding([8, 16]),
            ]
            .padding(20)
            .width(Length::Fixed(280.0))
        };

        container(content).style(styles::modal).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = event::listen_with(|event, status, _window| {
            if status == event::Status::Captured {
                return None;
            }
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            {
                if let Some(combo) = gui_key_combo(&key, modifiers) {
                    if let Some(bindings) = GUI_KEYBINDINGS.get() {
                        if let Ok(bindings) = bindings.lock() {
                            if let Some(action) = bindings.get(&combo) {
                                if let Some(message) = gui_action_message(action) {
                                    return Some(message);
                                }
                            }
                        }
                    }
                }
                match key.as_ref() {
                    keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::HideWindow),
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Some(Message::MoveUp),
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                        Some(Message::MoveDown)
                    }
                    keyboard::Key::Named(keyboard::key::Named::Enter) => {
                        Some(Message::CopySelected)
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                        // Shift+Left cycles to previous tab; bare Left navigates pages.
                        if modifiers.shift() {
                            Some(Message::PrevTab)
                        } else {
                            Some(Message::PrevPage)
                        }
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                        // Shift+Right cycles to next tab; bare Right navigates pages.
                        if modifiers.shift() {
                            Some(Message::NextTab)
                        } else {
                            Some(Message::NextPage)
                        }
                    }
                    keyboard::Key::Named(keyboard::key::Named::Tab) => {
                        // Tab toggles the side inspector panel for the
                        // currently selected entry. The handler in `update`
                        // resolves selection -> entry id (the subscription
                        // closure can't see app state, so we route through
                        // a dedicated message).
                        Some(Message::ToggleSelectedEntryPanel)
                    }
                    keyboard::Key::Character(c) => {
                        let s: &str = c;
                        match s {
                            "?" => Some(Message::ToggleHelp),
                            "m" | "M" => Some(Message::ToggleMultiSelect),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        });

        let tick_sub = iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);

        // Global hotkey is Windows-only; on Linux the user binds a compositor
        // shortcut to `ditox-gui --toggle` which goes through the IPC socket.
        #[cfg(windows)]
        let hotkey_sub = Subscription::run(|| {
            iced::stream::channel(
                10,
                |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                    let receiver = GlobalHotKeyEvent::receiver();
                    loop {
                        if let Ok(event) = receiver.try_recv() {
                            if event.state == HotKeyState::Pressed {
                                let _ = sender.try_send(Message::GlobalHotkeyPressed);
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                },
            )
        });

        // Phase 4 sub-tasks 4.1 + 4.2: poll the IPC receiver every
        // 50 ms. We can't bridge the std::mpsc::Receiver directly
        // into iced's async runtime (it's not Send-compatible with
        // iced's stream::channel future), so we emit a tick that
        // triggers an `update()` call where we drain the rx.
        let ipc_sub = iced::time::every(Duration::from_millis(50)).map(|_| Message::PollIpc);

        // Window-open subscription: capture the main window's Id on
        // creation so subsequent IPC commands can target it via
        // `iced::window::set_mode`.
        let window_open_sub = iced::window::open_events().map(Message::WindowOpened);

        let clipboard_sub = Subscription::run(|| {
            iced::stream::channel(
                10,
                |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                    loop {
                        let changed = {
                            if let Some(watcher) = CLIPBOARD_WATCHER.get() {
                                if let Ok(mut w) = watcher.lock() {
                                    w.poll_once().unwrap_or(false)
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };
                        if changed {
                            let _ = sender.try_send(Message::ClipboardChanged);
                        }
                        let poll_interval =
                            POLL_INTERVAL_MS.load(std::sync::atomic::Ordering::Relaxed);
                        tokio::time::sleep(Duration::from_millis(poll_interval)).await;
                    }
                },
            )
        });

        let focus_sub = event::listen_with(|event, _status, _window| {
            if let iced::Event::Window(window_event) = event {
                match window_event {
                    window::Event::Focused => Some(Message::WindowFocused),
                    window::Event::Unfocused => Some(Message::WindowUnfocused),
                    window::Event::Moved(position) => Some(Message::WindowMoved(position)),
                    window::Event::Resized(size) => Some(Message::WindowResized(size)),
                    _ => None,
                }
            } else {
                None
            }
        });

        let tray_sub = Subscription::run(|| {
            iced::stream::channel(
                10,
                |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                    let receiver = MenuEvent::receiver();
                    loop {
                        if let Ok(event) = receiver.try_recv() {
                            let _ = sender.try_send(Message::TrayMenuEvent(event.id.0.clone()));
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                },
            )
        });

        #[cfg(windows)]
        let subs = Subscription::batch([
            keyboard_sub,
            tick_sub,
            hotkey_sub,
            clipboard_sub,
            focus_sub,
            tray_sub,
            ipc_sub,
            window_open_sub,
        ]);
        #[cfg(not(windows))]
        let subs = Subscription::batch([
            keyboard_sub,
            tick_sub,
            clipboard_sub,
            focus_sub,
            tray_sub,
            ipc_sub,
            window_open_sub,
        ]);
        subs
    }

    fn refresh_entries(&mut self) {
        // Determine filter from active tab
        let (filter, collection_id) = if self.active_tab < self.tabs.len() {
            self.tabs[self.active_tab].db_filter()
        } else {
            ("all", None)
        };

        if self.search_query.is_empty() {
            // Normal pagination. Owned strings so the closure is `'static`.
            let filter_owned = filter.to_string();
            let collection_owned = collection_id.map(|s| s.to_string());
            let tag_filters = self.active_tag_filters.clone();
            let offset = self.current_page * self.config.general.max_entries;
            let limit = self.config.general.max_entries;

            let f = filter_owned.clone();
            let c = collection_owned.clone();
            let tag_for_entries = tag_filters.clone();
            if let Ok(Ok(entries)) = self.db.call(move |d| {
                if tag_for_entries.len() == 1 {
                    let tag_id = tag_for_entries.iter().next().expect("tag exists");
                    d.get_page_filtered_with_tag(offset, limit, &f, c.as_deref(), tag_id)
                } else {
                    d.get_page_filtered(offset, limit, &f, c.as_deref())
                }
            }) {
                self.entries = self.apply_tag_filter_to_entries(entries);
            }
            // Update counts
            if let Ok(Ok(count)) = self.db.call(move |d| {
                if tag_filters.len() == 1 {
                    let tag_id = tag_filters.iter().next().expect("tag exists");
                    d.count_filtered_with_tag(&filter_owned, collection_owned.as_deref(), tag_id)
                } else {
                    d.count_filtered(&filter_owned, collection_owned.as_deref())
                }
            }) {
                self.total_count = if self.active_tag_filters.len() > 1 {
                    self.entries.len()
                } else {
                    count
                };
            }
        } else {
            // Search mode - parse search-prefix scope (Phase 3 sub-task 3.6)
            // and route via the shared dispatch helper. /p /h /r /q /f
            // resolve to format-restricted / notes-only / full-text DB
            // methods; everything else lands on the tab-aware
            // `search_entries_filtered`.
            let parsed = ditox_core::search::parse(&self.search_query);
            let limit = self.config.general.max_entries;
            let filter_owned = filter.to_string();
            let collection_owned = collection_id.map(|s| s.to_string());
            let result = self.db.call(move |d| {
                ditox_core::search::dispatch(
                    d,
                    &parsed,
                    limit,
                    &filter_owned,
                    collection_owned.as_deref(),
                )
            });
            match result {
                Ok(Ok(entries)) => {
                    let entries = self.apply_tag_filter_to_entries(entries);
                    self.total_count = entries.len();
                    self.entries = entries;
                    // Reset to first page for search results
                    self.current_page = 0;
                }
                Ok(Err(e)) => {
                    tracing::error!("search error: {}", e);
                    self.entries.clear();
                    self.total_count = 0;
                }
                Err(e) => {
                    tracing::error!("db actor error: {}", e);
                    self.entries.clear();
                    self.total_count = 0;
                }
            }
        }

        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }

        self.update_image_cache();
        self.refresh_tag_cache();
    }

    fn apply_tag_filter_to_entries(&self, entries: Vec<Entry>) -> Vec<Entry> {
        if self.active_tag_filters.is_empty() {
            return entries;
        }

        entries
            .into_iter()
            .filter(|entry| {
                self.db
                    .call({
                        let entry_id = entry.id.clone();
                        let tag_ids = self.active_tag_filters.clone();
                        move |d| {
                            d.get_tags_for_entry(&entry_id).map(|tags| {
                                let entry_tag_ids: HashSet<String> =
                                    tags.into_iter().map(|tag| tag.id).collect();
                                tag_ids.iter().all(|tag_id| entry_tag_ids.contains(tag_id))
                            })
                        }
                    })
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false)
            })
            .collect()
    }

    fn refresh_tag_cache(&mut self) {
        self.all_tags = self
            .db
            .call(|d| d.get_all_tags().unwrap_or_default())
            .unwrap_or_default();
        self.all_collections = self
            .db
            .call(|d| d.get_all_collections().unwrap_or_default())
            .unwrap_or_default();

        let mut tabs = vec![
            TabFilter::All,
            TabFilter::Text,
            TabFilter::Images,
            TabFilter::Favorites,
            TabFilter::Today,
            TabFilter::Yesterday,
            TabFilter::ThisWeek,
            TabFilter::ThisMonth,
            TabFilter::Older,
        ];
        tabs.extend(self.all_collections.iter().map(|c| TabFilter::Collection {
            id: c.id.clone(),
            name: c.name.clone(),
        }));
        tabs.push(TabFilter::Uncollected);
        self.tabs = tabs;
        if self.active_tab >= self.tabs.len() {
            self.active_tab = 0;
        }

        let entry_ids: Vec<String> = self.entries.iter().map(|e| e.id.clone()).collect();
        let tags = self
            .db
            .call(move |d| {
                let mut out = HashMap::new();
                for id in entry_ids {
                    out.insert(id.clone(), d.get_tags_for_entry(&id).unwrap_or_default());
                }
                out
            })
            .unwrap_or_default();
        self.entry_tags = tags;
    }

    fn reload_config_if_changed(&mut self) {
        let Some(path) = self.config_path.clone() else {
            return;
        };
        let Some(mtime) = config_mtime(&path) else {
            return;
        };
        if self.config_mtime.is_some_and(|old| old >= mtime) {
            return;
        }
        match Config::load() {
            Ok(config) => {
                self.config = config;
                self.config_mtime = Some(mtime);
                self.poll_interval_ms = self.config.general.poll_interval_ms;
                POLL_INTERVAL_MS.store(
                    self.config.general.poll_interval_ms,
                    std::sync::atomic::Ordering::Relaxed,
                );
                if let Some(bindings) = GUI_KEYBINDINGS.get() {
                    if let Ok(mut bindings) = bindings.lock() {
                        *bindings = self.config.keybindings.gui.clone();
                    }
                }
                self.settings_max_entries = self.config.general.max_entries.to_string();
                self.settings_poll_interval_ms = self.config.general.poll_interval_ms.to_string();
                self.settings_theme_selected = self.config.ui.theme.selected.clone();
                self.settings_theme_text = self.config.ui.theme.text.clone();
                self.settings_theme_border = self.config.ui.theme.border.clone();
                self.settings_theme_muted = self.config.ui.theme.muted.clone();
                self.refresh_entries();
            }
            Err(e) => tracing::warn!("failed to hot-reload config: {e}"),
        }
    }

    fn regenerate_hyprland_binds(&self) {
        #[cfg(all(unix, not(target_os = "macos")))]
        if let Ok(Ok(entries)) = self.db.call(|d| d.entries_with_global_hotkeys()) {
            if let Err(e) = crate::hyprland_config::write_clip_binds(&entries) {
                tracing::warn!(error = %e, "failed to regenerate Hyprland clip binds");
            }
        }
    }

    /// Update image cache for currently visible entries. The cache is keyed
    /// by the resolved blob path (not the hash), because iced's `Handle` is
    /// path-based — we need to match how we render thumbnails.
    fn update_image_cache(&mut self) {
        let current_paths: std::collections::HashSet<String> = self
            .entries
            .iter()
            .filter(|e| e.entry_type == EntryType::Image)
            .filter_map(|e| e.image_path().map(|p| p.to_string_lossy().into_owned()))
            .collect();

        self.image_cache
            .retain(|path, _| current_paths.contains(path));

        for path in current_paths {
            if !self.image_cache.contains_key(&path) {
                let path_buf = std::path::Path::new(&path);
                if path_buf.exists() {
                    self.image_cache
                        .insert(path.clone(), iced_image::Handle::from_path(&path));
                }
            }
        }
    }

    fn save_window_state(&self) {
        self.window_state.save();
    }
}

// ============================================================================
// Tray icon setup
// ============================================================================

struct TrayMenuIds {
    show: tray_icon::menu::MenuId,
    startup: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

static TRAY_MENU_IDS: std::sync::OnceLock<TrayMenuIds> = std::sync::OnceLock::new();

/// Build the shared tray menu. The returned items must stay alive for the
/// lifetime of the tray icon or the menu disappears.
fn build_tray_menu() -> Option<(Menu, MenuItem, CheckMenuItem, MenuItem)> {
    let menu = Menu::new();
    #[cfg(windows)]
    let show_label = "Show (Ctrl+Shift+V)";
    #[cfg(not(windows))]
    let show_label = "Show";
    let show_item = MenuItem::new(show_label, true, None);
    let startup_enabled = crate::startup::is_startup_enabled();
    let startup_item = CheckMenuItem::new("Run at login", true, startup_enabled, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let _ = TRAY_MENU_IDS.set(TrayMenuIds {
        show: show_item.id().clone(),
        startup: startup_item.id().clone(),
        quit: quit_item.id().clone(),
    });

    menu.append(&show_item).ok()?;
    menu.append(&startup_item).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit_item).ok()?;

    Some((menu, show_item, startup_item, quit_item))
}

/// Windows: build the tray on the iced thread. The win32 event loop is already
/// running here courtesy of winit.
#[cfg(windows)]
fn setup_tray_icon() -> Option<TrayIcon> {
    let (menu, _show, _startup, _quit) = build_tray_menu()?;
    let icon = create_default_icon()?;

    TrayIconBuilder::new()
        .with_tooltip("Ditox Clipboard Manager")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .ok()
}

/// Linux: tray-icon's Linux backend requires a GTK event loop on the same
/// thread where the `TrayIcon` is created. iced/winit does not run GTK, so
/// we spawn a dedicated GTK thread that owns the tray and drives
/// `gtk::main()`. Menu events travel back to the iced app via the global
/// `MenuEvent::receiver()` (which the existing subscription already polls).
///
/// `tray-icon`'s Linux backend pulls in `libappindicator-sys`, which `dlopen`s
/// `libayatana-appindicator3.so.1` (or fallbacks) on first use and **panics**
/// from a `lazy_static` initialiser if the library isn't on the loader path.
/// We can't recover from that panic on a dedicated thread, so we probe the
/// library *before* spawning the tray thread and skip cleanly when it's
/// missing — typically when running a `cargo build`'d binary outside
/// `nix develop` on NixOS, or on minimal distros without the appindicator
/// system package installed.
#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_linux_tray_thread() {
    if !appindicator_loadable() {
        tracing::warn!(
            "tray-icon requires libayatana-appindicator3.so.1 or \
             libappindicator3.so.1 at runtime; none of the candidate names \
             were loadable. Continuing without a tray icon. \
             Hint: NixOS users should run via `nix run` / `nix build` (which \
             wraps the binary with the right LD_LIBRARY_PATH) or launch from \
             a `nix develop` shell; on other distros install the \
             libayatana-appindicator (or libappindicator-gtk3) system package."
        );
        return;
    }

    std::thread::Builder::new()
        .name("ditox-tray".into())
        .spawn(|| {
            if let Err(e) = gtk::init() {
                tracing::warn!("Could not initialise GTK for tray icon: {e}");
                return;
            }

            // Menu + icon live for the rest of the thread.
            let tray_bits = build_tray_menu();
            let icon = create_default_icon();

            let tray = match (tray_bits, icon) {
                (Some((menu, _show, _startup, _quit)), Some(icon)) => TrayIconBuilder::new()
                    .with_tooltip("Ditox Clipboard Manager")
                    .with_icon(icon)
                    .with_menu(Box::new(menu))
                    .build()
                    .ok(),
                _ => None,
            };

            if tray.is_none() {
                tracing::warn!(
                    "Could not create tray icon; continuing without tray. \
                     (Desktop may not provide a StatusNotifierItem host.)"
                );
            }

            // Keep `_tray` and the menu items alive and pump GTK events.
            let _tray = tray;
            gtk::main();
        })
        .expect("failed to spawn tray thread");
}

/// Probe for the libappindicator dynamic library that `tray-icon` needs at
/// runtime, returning `true` iff one of the four candidate sonames can be
/// `dlopen`ed. `libappindicator-sys` itself panics on failure from a
/// `lazy_static` initialiser; we'd rather skip the tray than crash the
/// dedicated GTK thread.
///
/// The candidate list mirrors `libappindicator-sys-0.9.0/src/lib.rs:41`:
///
/// ```text
/// libayatana-appindicator3.so.1
/// libappindicator3.so.1
/// libayatana-appindicator3.so
/// libappindicator3.so
/// ```
///
/// We open with `RTLD_LAZY` (don't resolve symbols yet) and immediately
/// `dlclose` so the probe doesn't pin the library into the address space —
/// `tray-icon`'s real load path will reopen it through `libappindicator-sys`'s
/// own caching layer.
#[cfg(all(unix, not(target_os = "macos")))]
fn appindicator_loadable() -> bool {
    use std::ffi::CString;

    const CANDIDATES: &[&str] = &[
        "libayatana-appindicator3.so.1",
        "libappindicator3.so.1",
        "libayatana-appindicator3.so",
        "libappindicator3.so",
    ];

    for name in CANDIDATES {
        let Ok(c) = CString::new(*name) else { continue };
        // SAFETY: `dlopen` accepts any valid C string; failure returns null,
        // which we surface as a fall-through. We close successful handles
        // immediately and never expose them outside this function.
        unsafe {
            let handle = libc::dlopen(c.as_ptr(), libc::RTLD_LAZY);
            if !handle.is_null() {
                libc::dlclose(handle);
                return true;
            }
        }
    }

    false
}

const ICON_PNG: &[u8] = include_bytes!("../../ditox.png");

fn create_default_icon() -> Option<Icon> {
    let img = image::load_from_memory(ICON_PNG).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

fn load_window_icon() -> Option<iced::window::Icon> {
    let img = image::load_from_memory(ICON_PNG).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    iced::window::icon::from_rgba(rgba.into_raw(), width, height).ok()
}

// ============================================================================
// Application entry point
// ============================================================================

/// Global storage for app config (iced 0.14 requires Fn boot closure, Database is not Sync)
static APP_CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
static APP_START_HIDDEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Phase 2 paste-back sub-task 2.9: raw selection-cursor index passed
/// in by `run_with`. Atomic because it's a plain `usize` and we don't
/// need a `Mutex` round-trip — the value is only ever read once by
/// `boot_app`. Default `0` (top of the list) when unset.
static APP_INITIAL_SELECTION: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Phase 4 sub-tasks 4.1 + 4.2: receiver end of the IPC server's
/// command channel. The iced subscription drains it on every poll
/// tick. `boot_app` `take()`s it on first call.
#[allow(clippy::type_complexity)]
static APP_IPC_RX: std::sync::OnceLock<
    Mutex<Option<std::sync::mpsc::Receiver<crate::ipc::DaemonCommand>>>,
> = std::sync::OnceLock::new();

/// Once iced reports the main window's `Id` (via `open_events`), we
/// store it here so subsequent IPC `Show`/`Hide`/`Toggle` commands
/// can target it via `iced::window::set_mode`.
static APP_MAIN_WINDOW_ID: std::sync::OnceLock<iced::window::Id> = std::sync::OnceLock::new();

/// Phase 2 paste-back state. Wrapped in `Mutex<Option<T>>` so
/// `boot_app` can `take()` ownership into the `DitoxApp` instance —
/// these aren't `Sync` (the trackers are `Send`-only) and even
/// `ForegroundSnapshot` shouldn't be cloned out of a static (it
/// represents a one-time capture).
#[allow(clippy::type_complexity)]
static APP_PREVIOUS_FOREGROUND: std::sync::OnceLock<Mutex<Option<ForegroundSnapshot>>> =
    std::sync::OnceLock::new();
#[allow(clippy::type_complexity)]
static APP_FOREGROUND_TRACKER: std::sync::OnceLock<Mutex<Option<Box<dyn ForegroundTracker>>>> =
    std::sync::OnceLock::new();
#[allow(clippy::type_complexity)]
static APP_SYNTHESIZER_CHAIN: std::sync::OnceLock<Mutex<Option<Vec<Box<dyn Synthesizer>>>>> =
    std::sync::OnceLock::new();

fn boot_app() -> (DitoxApp, Task<Message>) {
    let config = APP_CONFIG
        .get()
        .expect("APP_CONFIG must be set before running the app")
        .clone();
    let start_hidden = APP_START_HIDDEN.load(std::sync::atomic::Ordering::Relaxed);
    let db = Database::open().expect("Failed to open database for app");

    // Take ownership of the paste-back state from the statics. After
    // this `take()`, subsequent calls to `boot_app` (which iced
    // shouldn't do — boot is one-shot) would receive `None` and the
    // launcher would degrade gracefully (clipboard-only, no restore).
    let previous_foreground = APP_PREVIOUS_FOREGROUND
        .get()
        .and_then(|m| m.lock().ok().and_then(|mut g| g.take()));
    let foreground_tracker = APP_FOREGROUND_TRACKER
        .get()
        .and_then(|m| m.lock().ok().and_then(|mut g| g.take()))
        .unwrap_or_else(|| {
            tracing::warn!("APP_FOREGROUND_TRACKER not set; using NoopForegroundTracker fallback");
            Box::new(ditox_core::foreground::NoopForegroundTracker::new())
        });
    let synthesizer_chain = APP_SYNTHESIZER_CHAIN
        .get()
        .and_then(|m| m.lock().ok().and_then(|mut g| g.take()))
        .unwrap_or_else(|| {
            tracing::warn!("APP_SYNTHESIZER_CHAIN not set; using OffSynthesizer fallback");
            vec![
                Box::new(ditox_core::paste::synthesize::OffSynthesizer::new())
                    as Box<dyn Synthesizer>,
            ]
        });
    let initial_selection = APP_INITIAL_SELECTION.load(std::sync::atomic::Ordering::Relaxed);

    DitoxApp::new(
        db,
        config,
        start_hidden,
        previous_foreground,
        foreground_tracker,
        synthesizer_chain,
        initial_selection,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_with(
    _db: Database,
    config: Config,
    start_hidden: bool,
    previous_foreground: Option<ForegroundSnapshot>,
    foreground_tracker: Box<dyn ForegroundTracker>,
    synthesizer_chain: Vec<Box<dyn Synthesizer>>,
    initial_selection: usize,
    ipc_rx: Option<std::sync::mpsc::Receiver<crate::ipc::DaemonCommand>>,
) -> ditox_core::Result<()> {
    // Store config for the boot function (db will be opened fresh since it's not Sync)
    let _ = APP_CONFIG.set(config);
    APP_START_HIDDEN.store(start_hidden, std::sync::atomic::Ordering::Relaxed);
    APP_INITIAL_SELECTION.store(initial_selection, std::sync::atomic::Ordering::Relaxed);

    // Phase 2 paste-back state. Wrapped in Mutex<Option<...>> so
    // boot_app can take ownership; subsequent invocations (shouldn't
    // happen but defensively handled) get None and degrade.
    let _ = APP_PREVIOUS_FOREGROUND.set(Mutex::new(previous_foreground));
    let _ = APP_FOREGROUND_TRACKER.set(Mutex::new(Some(foreground_tracker)));
    let _ = APP_SYNTHESIZER_CHAIN.set(Mutex::new(Some(synthesizer_chain)));

    // Phase 4 sub-tasks 4.1 + 4.2: stash the IPC receiver for the
    // GUI's poll subscription to drain.
    let _ = APP_IPC_RX.set(Mutex::new(ipc_rx));

    // One-shot floating-launcher: ignore any persisted size and force a
    // compact 420x520 window anchored to the bottom-left of the active
    // monitor.
    #[allow(clippy::field_reassign_with_default)]
    let settings = {
        let mut settings = iced::window::Settings::default();
        settings.size = DEFAULT_WINDOW_SIZE;
        // SpecificWith receives (window_size, monitor_size) and returns the
        // top-left corner. Anchor to the bottom-left with a 20px margin on
        // both axes (matches the floating-launcher reference design).
        //
        // Phase 3 sub-task 3.7: also publish the monitor's resolution
        // (`<width>x<height>`) into [`MONITOR_KEY`] so subsequent
        // `WindowState::save` calls persist under the per-monitor key.
        // Idempotent — only the first invocation wins (one monitor
        // per process for now; Phase 4 will replace this with a
        // per-event monitor tracker).
        settings.position = window::Position::SpecificWith(|window_size, monitor_size| {
            set_current_monitor_key(monitor_size.width, monitor_size.height);
            Point::new(
                FLOATING_MARGIN,
                (monitor_size.height - window_size.height - FLOATING_MARGIN).max(FLOATING_MARGIN),
            )
        });
        settings.icon = load_window_icon();
        #[cfg(windows)]
        {
            settings.decorations = false;
        }
        #[cfg(not(windows))]
        {
            settings.decorations = true;
        }
        settings.transparent = false;
        settings.resizable = true;
        settings.min_size = Some(MIN_WINDOW_SIZE);
        settings.visible = !start_hidden;
        settings
    };

    // Phase 4 sub-task 4.3: dispatch between the layer-shell path
    // (Linux compositors that support wlr-layer-shell) and the
    // xdg_toplevel path (everything else: Windows, macOS, GNOME
    // Wayland, X11). Settings translation lives inside each arm.
    #[cfg(target_os = "linux")]
    {
        let platform = ditox_core::platform::detect();
        if platform.supports_layer_shell() {
            tracing::info!(
                platform = ?platform,
                "starting iced_layershell window (wlr-layer-shell)"
            );
            return run_layer_shell(start_hidden);
        }
        tracing::info!(
            platform = ?platform,
            "starting iced xdg_toplevel window (compositor lacks wlr-layer-shell)"
        );
    }

    iced::application(boot_app, DitoxApp::update, DitoxApp::view)
        .subscription(DitoxApp::subscription)
        .theme(DitoxApp::theme)
        .title(DitoxApp::title)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES) // Load Bootstrap Icons
        .window(settings)
        .run()
        .map_err(|e| ditox_core::DitoxError::Other(e.to_string()))?;

    Ok(())
}

/// Phase 4 sub-task 4.3: layer-shell window for wlr-layer-shell
/// compositors (Hyprland, Sway, Wlroots-generic, KDE Plasma 5.27+).
///
/// Uses `iced_layershell::build_pattern::application` instead of
/// `iced::application`. The Message enum has the
/// `#[iced_layershell::to_layer_message]` attribute applied so the
/// layer-shell control variants and `TryInto<LayershellCustomActionWithId>`
/// impl are auto-generated. The catch-all `_ => {}` in `update`
/// soaks up the unhandled control variants.
///
/// Settings translation (Phase 4 sub-task 4.4):
/// - `[gui.position]` mode → Anchor + margin + initial size.
/// - `[gui.pinned]` → Layer::Top vs Layer::Overlay.
/// - Window size is fixed at 420x520 (the launcher's intent;
///   user-resizable would conflict with a layer-shell anchor).
/// - Exclusive keyboard interactivity so the launcher captures
///   key events without depending on compositor focus.
#[cfg(target_os = "linux")]
fn run_layer_shell(_start_hidden: bool) -> ditox_core::Result<()> {
    use iced_layershell::reexport::{KeyboardInteractivity, Layer};
    use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};

    let config = APP_CONFIG.get().cloned().unwrap_or_default();
    let (anchor, margin) = layer_anchor_and_margin_for(&config.gui.position);

    let layer_settings = LayerShellSettings {
        anchor,
        layer: if config.gui.pinned {
            Layer::Overlay
        } else {
            Layer::Top
        },
        size: Some((
            DEFAULT_WINDOW_SIZE.width as u32,
            DEFAULT_WINDOW_SIZE.height as u32,
        )),
        // (top, right, bottom, left) per layershellev.
        margin,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        // exclusive_zone = -1 means "no zone reserved" (we float over
        // other windows rather than push them).
        exclusive_zone: -1,
        start_mode: StartMode::Active,
        events_transparent: false,
    };

    iced_layershell::build_pattern::application(
        boot_app,
        || String::from("ditox-gui"),
        DitoxApp::update,
        DitoxApp::view,
    )
    .subscription(DitoxApp::subscription)
    .theme(iced::Theme::Dark)
    .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
    .settings(Settings {
        id: Some("ditox-gui".to_string()),
        layer_settings,
        ..Settings::default()
    })
    .run()
    .map_err(|e| ditox_core::DitoxError::Other(e.to_string()))?;

    Ok(())
}

/// Phase 4 sub-task 4.4: translate a [`ditox_core::config::GuiPosition`]
/// to a `(layer-shell Anchor, margin)` pair.
///
/// Margin tuple is `(top, right, bottom, left)` per `layershellev`.
///
/// Modes that need runtime queries (`AtCursor` /
/// `AtActiveWindowCentre`) are reduced here to their nearest
/// static equivalent — the daemon repositions per-summon via
/// IPC `AnchorChange`/`MarginChange` messages once Phase 4
/// polish lands those queries (next iteration).
#[cfg(target_os = "linux")]
fn layer_anchor_and_margin_for(
    pos: &ditox_core::config::GuiPosition,
) -> (iced_layershell::reexport::Anchor, (i32, i32, i32, i32)) {
    use ditox_core::config::{GuiPosition, HorizontalAnchor, VerticalAnchor};
    use iced_layershell::reexport::Anchor;

    match pos {
        GuiPosition::Default => (Anchor::Bottom | Anchor::Left, (0, 0, 24, 24)),
        // AtPrevious + AtCursor + AtActiveWindowCentre fall back
        // to the bottom-left default for the initial layer-shell
        // creation. The daemon repositions on each summon via
        // `AnchorChange`/`MarginChange` messages once the runtime
        // query path lands (Phase 4 follow-up).
        GuiPosition::AtPrevious | GuiPosition::AtCursor | GuiPosition::AtActiveWindowCentre => {
            (Anchor::Bottom | Anchor::Left, (0, 0, 24, 24))
        }
        GuiPosition::Fixed {
            horizontal,
            vertical,
            offset,
        } => {
            // Build the anchor from the chosen edges. Layer-shell
            // semantics: anchoring to two perpendicular edges
            // pins the corresponding corner. The bitflags-style
            // `Anchor::empty()` plus `|=` lets us add edges
            // selectively (Centre / Middle = no edge bits, which
            // means "centred along that axis").
            let mut anchor = Anchor::empty();
            let (off_x_left, off_x_right) = match horizontal {
                HorizontalAnchor::Left => {
                    anchor |= Anchor::Left;
                    (offset[0], 0)
                }
                HorizontalAnchor::Right => {
                    anchor |= Anchor::Right;
                    (0, -offset[0])
                }
                HorizontalAnchor::Centre => (0, 0),
            };
            let (off_y_top, off_y_bottom) = match vertical {
                VerticalAnchor::Top => {
                    anchor |= Anchor::Top;
                    (offset[1], 0)
                }
                VerticalAnchor::Bottom => {
                    anchor |= Anchor::Bottom;
                    (0, -offset[1])
                }
                VerticalAnchor::Middle => (0, 0),
            };
            // (top, right, bottom, left)
            (anchor, (off_y_top, off_x_right, off_y_bottom, off_x_left))
        }
    }
}
