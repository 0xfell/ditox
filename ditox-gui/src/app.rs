//! Ditox iced GUI application - Modern redesign

use ditox_core::app::TabFilter;
use ditox_core::{Clipboard, Config, Database, Entry, EntryType, Result, Watcher};
#[cfg(windows)]
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use iced::widget::{
    button, column, container, image as iced_image, mouse_area, operation, row, scrollable,
    text, text_input, Column, Row, Space,
};
use iced::{
    event, keyboard, window, ContentFit, Element, Font, Length, Point, Size, Subscription, Task, Theme,
};
use iced::window::Direction;
use iced::widget::Id as WidgetId;
use iced::widget::scrollable::RelativeOffset;

// Bootstrap Icons font
const ICONS: Font = Font::with_name("bootstrap-icons");
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
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

/// Scroll to make the selected item visible
/// Returns a Task that snaps the scrollable to the appropriate position
fn scroll_to_selected(selected_index: usize, total_entries: usize) -> Task<Message> {
    if total_entries == 0 {
        return Task::none();
    }
    // Calculate the relative position (0.0 = top, 1.0 = bottom)
    let relative_pos = selected_index as f32 / (total_entries.saturating_sub(1).max(1)) as f32;
    operation::snap_to(ENTRY_LIST_ID, RelativeOffset { x: 0.0, y: relative_pos })
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
        return operation::snap_to(ENTRY_LIST_ID, RelativeOffset { x: 0.0, y: relative_y });
    } else if direction > 0 && item_bottom > visible_bottom {
        // Scrolling down and item is below visible area - scroll to show it at bottom
        let target_offset = (item_bottom - viewport_height).max(0.0);
        let relative_y = if content_height > viewport_height {
            target_offset / (content_height - viewport_height)
        } else {
            0.0
        };
        return operation::snap_to(ENTRY_LIST_ID, RelativeOffset { x: 0.0, y: relative_y });
    }

    Task::none()
}

/// Delay before focusing search input (to avoid capturing the hotkey's "v")
const FOCUS_DELAY_MS: u64 = 250;

/// Delay before forcing window focus with Win32 APIs
const FORCE_FOCUS_DELAY_MS: u64 = 100;

/// Create a delayed focus task
fn delayed_focus_search() -> Task<Message> {
    Task::perform(
        async {
            tokio::time::sleep(Duration::from_millis(FOCUS_DELAY_MS)).await;
        },
        |_| Message::FocusSearch,
    )
}

/// Create a delayed Win32 force focus task (after Iced makes window visible)
fn delayed_force_focus() -> Task<Message> {
    Task::perform(
        async {
            tokio::time::sleep(Duration::from_millis(FORCE_FOCUS_DELAY_MS)).await;
        },
        |_| Message::ForceWindowFocus,
    )
}

/// Default window size
const DEFAULT_WINDOW_SIZE: Size = Size::new(420.0, 520.0);
/// Minimum window size
const MIN_WINDOW_SIZE: Size = Size::new(320.0, 300.0);

#[cfg(windows)]
fn force_restore_window(width: u32, height: u32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, ShowWindow, SetForegroundWindow,
        SetWindowPos, IsWindowVisible, GetWindowLongW, IsIconic, GWL_STYLE, GWL_EXSTYLE,
        WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE, WS_SIZEBOX, GetForegroundWindow,
        BringWindowToTop, SW_RESTORE, SW_SHOWNORMAL,
        HWND_TOPMOST, HWND_NOTOPMOST, SWP_SHOWWINDOW, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };
    use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
    use windows::Win32::Foundation::{HWND, BOOL, LPARAM};

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
                hwnd, is_iconic, is_visible
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
                let _ = windows::Win32::System::Threading::AttachThreadInput(current_thread_id, fg_thread_id, BOOL::from(true));

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
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)
            );
            // SPI_SETFOREGROUNDLOCKTIMEOUT to 0
            let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                windows::Win32::UI::WindowsAndMessaging::SPI_SETFOREGROUNDLOCKTIMEOUT,
                0,
                Some(0 as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)
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
                HWND_TOPMOST,
                x, y, width as i32, height as i32,
                flags
            );

            // 4. Force focus
            let _ = BringWindowToTop(hwnd);
            let fg_result = SetForegroundWindow(hwnd);
            tracing::info!("force_restore_window: SetForegroundWindow = {:?}", fg_result);

            // 5. Restore settings
            // Restore foreground lock timeout
            let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                windows::Win32::UI::WindowsAndMessaging::SPI_SETFOREGROUNDLOCKTIMEOUT,
                0,
                Some(original_timeout as usize as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)
            );

            // Detach input
            if attached {
                 let _ = windows::Win32::System::Threading::AttachThreadInput(current_thread_id, fg_thread_id, BOOL::from(false));
            }

            // Remove TOPMOST after a brief moment
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0, 0, 0, 0,
                SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            );

            // Final state check
            let is_iconic = IsIconic(hwnd).as_bool();
            let is_visible = IsWindowVisible(hwnd).as_bool();
            let fg = GetForegroundWindow();
            let is_fg = fg == hwnd;
            tracing::info!(
                "force_restore_window: Done. iconic={}, visible={}, foreground={}",
                is_iconic, is_visible, is_fg
            );
        } else {
            tracing::warn!("force_restore_window: No main window found!");
        }
    }
}

#[cfg(not(windows))]
fn force_restore_window(_width: u32, _height: u32) {
    // No-op on non-Windows platforms
}

/// Remove TOPMOST flag from our window (called when hiding)
#[cfg(windows)]
fn remove_topmost() {
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, SetWindowPos,
        GetWindowLongW, GWL_STYLE, GWL_EXSTYLE,
        WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE, WS_SIZEBOX,
        HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::Foundation::{HWND, BOOL, LPARAM};

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
                HWND_NOTOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            );
            tracing::info!("remove_topmost: Removed TOPMOST flag");
        }
    }
}

#[cfg(not(windows))]
fn remove_topmost() {
    // No-op on non-Windows platforms
}

/// Check if our main window is actually visible at Win32 level
/// This helps detect when Win+D has hidden us but our visible flag is still true
#[cfg(windows)]
fn is_window_actually_visible() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, GetWindowLongW,
        GWL_STYLE, GWL_EXSTYLE, WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE, WS_SIZEBOX,
        GetForegroundWindow,
    };
    use windows::Win32::Foundation::{HWND, BOOL, LPARAM};
    use std::process;

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
            data.found_visible, is_foreground
        );

        // Consider visible only if both visible AND we have foreground
        data.found_visible && is_foreground
    }
}

#[cfg(not(windows))]
fn is_window_actually_visible() -> bool {
    true // Assume visible on non-Windows
}

/// Window state that gets persisted
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

impl WindowState {
    fn state_file_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("com", "ditox", "ditox")
            .map(|dirs| dirs.data_dir().join("window_state.json"))
    }

    pub fn load() -> Self {
        let state: Self = Self::state_file_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();

        if state.x < -1000.0
            || state.y < -1000.0
            || state.width < MIN_WINDOW_SIZE.width
            || state.height < MIN_WINDOW_SIZE.height
        {
            tracing::warn!(
                "Invalid window state detected ({}, {}) {}x{}, using defaults",
                state.x, state.y, state.width, state.height
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

        if let Some(path) = Self::state_file_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&path, content);
            }
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
    pub const X: char = '\u{F62A}';          // x mark
    pub const GRIP_VERTICAL: char = '\u{F3FF}'; // grip-vertical (resize handle)

    // Actions
    pub const GEAR: char = '\u{F3E5}';       // settings gear
    pub const QUESTION: char = '\u{F505}';   // question circle
    pub const TRASH: char = '\u{F5DE}';      // trash can
    pub const STAR: char = '\u{F588}';       // star outline
    pub const STAR_FILL: char = '\u{F586}';  // star filled

    // Status
    pub const CIRCLE_FILL: char = '\u{F287}'; // filled circle (status indicator)

    // Types
    pub const FILE_TEXT: char = '\u{F3C1}';   // text file
    pub const IMAGE: char = '\u{F40D}';       // image
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
            scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. } => colors::ACCENT_DIM,
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
                colors::INFO.r, colors::INFO.g, colors::INFO.b, 0.15,
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

// ============================================================================
// Application state
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Main,
    Settings,
    Help,
    ImagePreview(String), // entry_id
    ConfirmDelete(String), // entry_id - confirmation for deleting favorites
}

#[derive(Debug, Clone)]
pub enum Message {
    // Entry interactions
    CopyEntry(usize),
    CopySelected,
    DeleteEntry(String),
    ToggleFavorite(String),

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
    /// Unconditionally show the window (sent by IPC `--show`).
    IpcShow,
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

    // View modes
    ShowSettings,
    HideSettings,
    ToggleHelp,
    CloseOverlay,

    // Image preview
    ShowImagePreview(String), // entry_id
    CloseImagePreview,
    CopyFromPreview,

    // Delete confirmation
    RequestDelete(String), // entry_id - triggers confirmation for favorites
    ConfirmDeleteEntry(String), // entry_id - actually delete after confirmation
    CancelDelete,

    // Settings
    ToggleStartup,

    // Focus
    FocusSearch,

    // Win32 force focus (delayed after Iced makes window visible)
    ForceWindowFocus,

    // Scroll tracking
    Scrolled(scrollable::Viewport),
}

pub struct DitoxApp {
    db: Arc<Mutex<Database>>,
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
}

impl DitoxApp {
    fn new(db: Database, config: Config, start_hidden: bool) -> (Self, Task<Message>) {
        let total_count = db.count().unwrap_or(0);
        let entries = db.get_page(0, PAGE_SIZE).unwrap_or_default();
        let window_state = WindowState::load();

        tracing::info!(
            "Loaded window state: {}x{} at ({}, {})",
            window_state.width, window_state.height, window_state.x, window_state.y
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

        let tabs = vec![
            TabFilter::All,
            TabFilter::Text,
            TabFilter::Images,
            TabFilter::Favorites,
            TabFilter::Today,
        ];

        let app = DitoxApp {
            db: Arc::new(Mutex::new(db)),
            config,
            search_query: String::new(),
            selected_index: 0,
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
        };

        let initial_task = window::oldest().and_then(move |id| {
            window::move_to(id, Point::new(window_state.x, window_state.y))
                .chain(window::resize(id, Size::new(window_state.width, window_state.height)))
        })
        .chain(delayed_focus_search());

        (app, initial_task)
    }

    fn title(&self) -> String {
        String::from("Ditox")
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CopyEntry(index) => {
                if let Some(entry) = self.entries.get(index) {
                    let result = match entry.entry_type {
                        EntryType::Text => Clipboard::set_text(&entry.content),
                        EntryType::Image => match entry.image_path() {
                            Some(p) => Clipboard::set_image(&p.to_string_lossy()),
                            None => Err(ditox_core::DitoxError::Other(
                                "image entry missing extension".into(),
                            )),
                        },
                    };
                    if result.is_ok() {
                        let _ = self.db.lock().unwrap().touch(&entry.id);
                        tracing::info!("Copied: {}", entry.preview(30));
                    }
                }
                self.save_window_state();
                self.visible = false;
                return window::oldest()
                    .and_then(|id| window::set_mode(id, window::Mode::Hidden));
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
                let was_in_preview = matches!(self.view_mode, ViewMode::ImagePreview(_));
                let was_in_confirm = matches!(self.view_mode, ViewMode::ConfirmDelete(_));
                let _ = self.db.lock().unwrap().delete(&id);
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
                    let _ = self.db.lock().unwrap().toggle_favorite(&id);
                    self.refresh_entries();
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
                     
                     // Offload to background thread
                     return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let db = db.lock().unwrap();
                                db.search_entries_filtered(&query, 50, &filter_str, collection_id.as_deref())
                                    .map_err(|e| e.to_string())
                            }).await.unwrap()
                        },
                        Message::SearchCompleted
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
                if self.view_mode == ViewMode::Main && self.selected_index + 1 < self.entries.len() {
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
                // Check actual window visibility at Win32 level
                // This detects Win+D hiding us even though our visible flag is true
                let actually_visible = is_window_actually_visible();
                tracing::info!(
                    "ToggleWindow: self.visible={}, actually_visible={}",
                    self.visible, actually_visible
                );

                // If we're visible AND actually visible (have foreground), hide
                if self.visible && actually_visible {
                    tracing::info!("Window is visible, hiding it");
                    self.save_window_state();
                    self.visible = false;
                    // Remove TOPMOST flag before hiding
                    remove_topmost();
                    return window::oldest()
                        .and_then(|id| window::set_mode(id, window::Mode::Hidden));
                }

                // Otherwise, show the window
                tracing::info!("Window is hidden or not foreground, showing it");

                self.visible = true;
                self.search_query.clear();
                self.refresh_entries();
                self.last_show_time = Instant::now();
                self.input_blocked_until = Some(Instant::now() + Duration::from_millis(300));

                let ws = self.window_state.clone();
                // Then let Iced configure the window
                return window::oldest().and_then(move |id| {
                    window::set_mode(id, window::Mode::Windowed)
                        .chain(window::minimize(id, false))
                        .chain(window::resize(id, Size::new(ws.width, ws.height)))
                        .chain(window::move_to(id, Point::new(ws.x, ws.y)))
                        .chain(window::gain_focus(id))
                        .chain(operation::snap_to(
                            ENTRY_LIST_ID,
                            scrollable::RelativeOffset { x: 0.0, y: 0.0 },
                        ))
                })
                .chain(delayed_force_focus())  // Call again after Iced finishes
                .chain(delayed_focus_search());
            }

            Message::ForceWindowFocus => {
                // Windows-only: works around Win+D hiding us. On Linux/macOS
                // this is a no-op (see `force_restore_window` stub).
                #[cfg(windows)]
                {
                    tracing::info!("ForceWindowFocus: Applying Win32 force restore");
                }
                force_restore_window(
                    self.window_state.width as u32,
                    self.window_state.height as u32,
                );
            }

            Message::IpcShow => {
                // Forced show from `ditox-gui --show`; always route through the
                // show branch of ToggleWindow by making ourselves look hidden.
                self.visible = false;
                return self.update(Message::ToggleWindow);
            }

            Message::HideWindow | Message::CloseOverlay => {
                // Check if we're closing an image preview - preserve scroll position
                let was_image_preview = matches!(self.view_mode, ViewMode::ImagePreview(_));

                if self.view_mode != ViewMode::Main {
                    self.view_mode = ViewMode::Main;
                    // If closing image preview, scroll back to selected entry
                    if was_image_preview {
                        return scroll_to_selected(self.selected_index, self.entries.len());
                    }
                    return Task::none();
                }
                self.save_window_state();
                self.visible = false;
                // Remove TOPMOST flag before hiding
                remove_topmost();
                return window::oldest()
                    .and_then(|id| window::set_mode(id, window::Mode::Hidden));
            }

            Message::StartDrag => {
                return window::oldest().and_then(|id| window::drag(id));
            }

            Message::StartResize(direction) => {
                return window::oldest().and_then(move |id| window::drag_resize(id, direction));
            }

            Message::Tick => {
                if self.visible && self.last_refresh.elapsed() > Duration::from_secs(2) {
                    self.refresh_entries();
                }
            }

            #[cfg(windows)]
            Message::GlobalHotkeyPressed => {
                let elapsed = self.last_hotkey_time.elapsed();
                if elapsed < Duration::from_millis(300) {
                    return Task::none();
                }
                self.last_hotkey_time = Instant::now();
                return self.update(Message::ToggleWindow);
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
                self.visible = true;
                self.refresh_entries();
                // Block input for 300ms to prevent capturing stray keystrokes
                self.input_blocked_until = Some(Instant::now() + Duration::from_millis(300));
                return delayed_focus_search();
            }

            Message::WindowUnfocused => {
                let elapsed = self.last_show_time.elapsed();
                if elapsed < Duration::from_millis(500) {
                    return Task::none();
                }
                self.save_window_state();
                self.visible = false;
                return window::oldest()
                    .and_then(|id| window::set_mode(id, window::Mode::Hidden));
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
                tracing::info!("Toggling startup: currently={}, setting to {}", currently_enabled, !currently_enabled);
                match crate::startup::set_startup_enabled(!currently_enabled) {
                    Ok(()) => tracing::info!("Startup setting changed successfully"),
                    Err(e) => tracing::error!("Failed to change startup setting: {}", e),
                }
            }

            Message::FocusSearch => {
                return iced::widget::operation::focus(search_input_id());
            }

            Message::ShowImagePreview(entry_id) => {
                // Update selected_index to match the clicked entry
                if let Some(index) = self.entries.iter().position(|e| e.id == entry_id) {
                    self.selected_index = index;
                }
                self.view_mode = ViewMode::ImagePreview(entry_id);
                // Preserve scroll position by snapping back to the selected item
                return scroll_to_selected(self.selected_index, self.entries.len());
            }

            Message::CloseImagePreview => {
                self.view_mode = ViewMode::Main;
                // Scroll back to the selected entry
                return scroll_to_selected(self.selected_index, self.entries.len());
            }

            Message::CopyFromPreview => {
                if let ViewMode::ImagePreview(ref entry_id) = self.view_mode {
                    if let Some(entry) = self.entries.iter().find(|e| e.id == *entry_id) {
                        let result = match entry.image_path() {
                            Some(p) => Clipboard::set_image(&p.to_string_lossy()),
                            None => Err(ditox_core::DitoxError::Other(
                                "image entry missing extension".into(),
                            )),
                        };
                        if result.is_ok() {
                            let _ = self.db.lock().unwrap().touch(&entry.id);
                            tracing::info!("Copied image: {}", entry.preview(30));
                        }
                    }
                }
                self.view_mode = ViewMode::Main;
                self.save_window_state();
                self.visible = false;
                return window::oldest()
                    .and_then(|id| window::set_mode(id, window::Mode::Hidden));
            }

            Message::Scrolled(viewport) => {
                self.scroll_viewport = Some(viewport);
            }
        }

        Task::none()
    }

    fn active_tab_filter(&self) -> &TabFilter {
        self.tabs.get(self.active_tab).unwrap_or(&TabFilter::All)
    }

    fn total_pages(&self) -> usize {
        if self.total_count == 0 { 1 } else { (self.total_count + PAGE_SIZE - 1) / PAGE_SIZE }
    }

    fn load_current_page(&mut self) {
        let filter = self.active_tab_filter().clone();
        let (filter_str, collection_id) = filter.db_filter();
        let offset = self.current_page * PAGE_SIZE;
        self.entries = self.db.lock().unwrap().get_page_filtered(offset, PAGE_SIZE, filter_str, collection_id).unwrap_or_default();
        self.selected_index = 0;
    }

    // ========================================================================
    // Views
    // ========================================================================

    fn view(&self) -> Element<'_, Message> {
        let main_view = self.view_main();

        match &self.view_mode {
            ViewMode::Main => main_view,
            ViewMode::Settings => self.view_with_overlay(main_view, self.view_settings()),
            ViewMode::Help => self.view_with_overlay(main_view, self.view_help()),
            ViewMode::ImagePreview(entry_id) => {
                self.view_with_overlay(main_view, self.view_image_preview(entry_id))
            }
            ViewMode::ConfirmDelete(entry_id) => {
                self.view_with_overlay(main_view, self.view_confirm_delete(entry_id))
            }
        }
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
        let entry_list = self.view_entries();
        let status_bar = self.view_status();

        let content = column![
            title_bar,
            search_section,
            tab_bar,
            entry_list,
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
        // Draggable title area (fills most of the bar)
        let title = mouse_area(
            row![
                text("Ditox").size(13).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
            ]
            .padding([0, 12])
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::StartDrag);

        // Small resize grip in top-right corner (diagonal lines icon)
        let resize_grip = mouse_area(
            container(
                icon(icons::GRIP_VERTICAL).size(10).color(colors::TEXT_MUTED)
            )
            .padding([4, 8])
        )
        .on_press(Message::StartResize(Direction::NorthEast));

        container(
            row![
                title,
                resize_grip,
            ]
            .align_y(iced::Alignment::Center),
        )
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
                    .style(if is_active { styles::tab_active } else { styles::tab_inactive })
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
        let interactive = self.view_mode == ViewMode::Main;

        // Favorite indicator
        let fav_star = if entry.favorite {
            icon(icons::STAR_FILL).size(12).color(colors::WARNING)
        } else {
            text(" ").size(12)
        };

        // Time
        let time = text(entry.relative_time()).size(10).color(colors::TEXT_MUTED);

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
                let filename = text(entry.preview(30))
                    .size(12)
                    .color(if is_selected {
                        colors::TEXT_PRIMARY
                    } else {
                        colors::TEXT_SECONDARY
                    });

                row![
                    thumbnail,
                    container(fav_star).width(Length::Fixed(18.0)),
                    filename,
                    Space::new().width(Length::Fill),
                    time,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
            }
            EntryType::Text => {
                // Text entry: type badge + preview text
                let type_badge = container(icon(icons::FILE_TEXT).size(10))
                    .padding([2, 5])
                    .style(styles::badge_text);

                let preview = text(entry.preview(45))
                    .size(12)
                    .color(if is_selected {
                        colors::TEXT_PRIMARY
                    } else {
                        colors::TEXT_SECONDARY
                    });

                row![
                    type_badge,
                    container(fav_star).width(Length::Fixed(18.0)),
                    preview,
                    Space::new().width(Length::Fill),
                    time,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
            }
        };

        // Determine action on click: images open preview, text copies directly
        let on_press = if interactive {
            match entry.entry_type {
                EntryType::Image => Some(Message::ShowImagePreview(entry_id_preview)),
                EntryType::Text => Some(Message::CopyEntry(index)),
            }
        } else {
            None
        };

        let entry_btn = button(entry_content)
            .style(if is_selected {
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

        row![entry_btn, fav_btn, del_btn]
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
            let handle = self.image_cache
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
        let count = if self.search_query.is_empty() {
            format!("{}", self.total_count)
        } else {
            format!("{}/{}", self.entries.len(), self.total_count)
        };

        let page_info = if self.search_query.is_empty() && self.total_pages() > 1 {
            format!("  |  {}/{}", self.current_page + 1, self.total_pages())
        } else {
            String::new()
        };

        let search_indicator: Element<'_, Message> = if self.is_searching {
            text("Searching...").size(11).color(colors::TEXT_MUTED).into()
        } else {
            Space::new().width(0).height(0).into()
        };

        // Status badge - using icon
        let status_badge = container(icon(icons::CIRCLE_FILL).size(8).color(colors::SUCCESS))
            .padding([0, 4]);

        container(
            row![
                text(count).size(11).color(colors::ACCENT),
                text(" entries").size(11).color(colors::TEXT_MUTED),
                text(page_info).size(11).color(colors::TEXT_MUTED),
                Space::new().width(10),
                search_indicator,
                Space::new().width(Length::Fill),
                status_badge,
                text("Ctrl+Shift+V").size(10).color(colors::TEXT_MUTED),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(styles::status_bar)
        .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let startup_enabled = crate::startup::is_startup_enabled();

        let content = column![
            // Header
            text("Settings").size(16).color(colors::TEXT_PRIMARY),
            Space::new().height(16),

            // Startup toggle
            row![
                text("Run on startup").size(12).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                button(text(if startup_enabled { "ON" } else { "OFF" }).size(11))
                    .style(if startup_enabled { styles::tab_active } else { styles::tab_inactive })
                    .on_press(Message::ToggleStartup)
                    .padding([4, 12]),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(10),

            // Poll interval
            row![
                text("Poll interval").size(12).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                text(format!("{}ms", self.poll_interval_ms))
                    .size(12)
                    .color(colors::TEXT_PRIMARY),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(10),

            // Max entries
            row![
                text("Max entries").size(12).color(colors::TEXT_SECONDARY),
                Space::new().width(Length::Fill),
                text(format!("{}", self.config.general.max_entries))
                    .size(12)
                    .color(colors::TEXT_PRIMARY),
            ]
            .align_y(iced::Alignment::Center),
            Space::new().height(20),

            // Close button
            button(text("Close").size(11))
                .style(styles::primary_btn)
                .on_press(Message::HideSettings)
                .padding([8, 24]),
        ]
        .padding(20)
        .width(Length::Fixed(280.0));

        container(content)
            .style(styles::modal)
            .into()
    }

    fn view_help(&self) -> Element<'_, Message> {
        let content = column![
            // Header
            row![
                text("Keyboard Shortcuts").size(16).color(colors::TEXT_PRIMARY),
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
                text("Tab / Shift+Tab").size(11).color(colors::ACCENT),
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

        container(content)
            .style(styles::modal)
            .into()
    }

    fn view_image_preview(&self, entry_id: &str) -> Element<'_, Message> {
        // Find the entry in our current entries list
        let entry = self.entries.iter().find(|e| e.id == entry_id);

        let content = if let Some(entry) = entry {
            let path_buf = entry.image_path().unwrap_or_default();
            let path = path_buf.to_string_lossy().into_owned();

            // Image display
            let image_display: Element<'_, Message> = if path_buf.exists() {
                container(
                    iced_image(iced_image::Handle::from_path(&path))
                        .content_fit(ContentFit::Contain)
                        .width(Length::Fill)
                        .height(Length::Fixed(280.0)),
                )
                .width(Length::Fill)
                .style(styles::preview_image_container)
                .padding(8)
                .into()
            } else {
                container(
                    column![
                        icon(icons::IMAGE).size(48).color(colors::TEXT_MUTED),
                        text("Image not found").size(12).color(colors::TEXT_MUTED),
                    ]
                    .spacing(8)
                    .align_x(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fixed(280.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(styles::preview_image_container)
                .into()
            };

            // Metadata — show the synthesized filename (image-<hash8>.<ext>).
            // Owned String to avoid a borrow that outlives the function body.
            let filename: String = entry.preview(40);

            let size_str = if entry.byte_size < 1024 {
                format!("{} B", entry.byte_size)
            } else if entry.byte_size < 1024 * 1024 {
                format!("{:.1} KB", entry.byte_size as f64 / 1024.0)
            } else {
                format!("{:.1} MB", entry.byte_size as f64 / (1024.0 * 1024.0))
            };

            let entry_id_delete = entry.id.clone();

            column![
                // Header
                row![
                    text("Image Preview").size(16).color(colors::TEXT_PRIMARY),
                    Space::new().width(Length::Fill),
                    button(icon(icons::X).size(12))
                        .style(styles::action_btn)
                        .on_press(Message::CloseImagePreview)
                        .padding([4, 8]),
                ]
                .align_y(iced::Alignment::Center),
                Space::new().height(12),
                // Image
                image_display,
                Space::new().height(12),
                // Metadata
                row![
                    text(filename).size(11).color(colors::TEXT_SECONDARY),
                    Space::new().width(Length::Fill),
                    text(size_str).size(11).color(colors::TEXT_MUTED),
                ],
                row![
                    text(entry.relative_time()).size(10).color(colors::TEXT_MUTED),
                    Space::new().width(Length::Fill),
                    if entry.favorite {
                        icon(icons::STAR_FILL).size(10).color(colors::WARNING)
                    } else {
                        text("").size(10)
                    },
                ],
                Space::new().height(16),
                // Actions
                row![
                    button(text("Copy to Clipboard").size(11))
                        .style(styles::primary_btn)
                        .on_press(Message::CopyFromPreview)
                        .padding([8, 16]),
                    Space::new().width(Length::Fill),
                    button(row![icon(icons::TRASH).size(12), text(" Delete").size(11),].spacing(4))
                        .style(styles::delete_btn)
                        .on_press(Message::DeleteEntry(entry_id_delete))
                        .padding([8, 16]),
                ]
                .spacing(8),
            ]
            .padding(20)
            .width(Length::Fixed(400.0))
        } else {
            // Entry not found (maybe deleted while preview was open)
            column![
                text("Entry not found").size(14).color(colors::TEXT_MUTED),
                Space::new().height(16),
                button(text("Close").size(11))
                    .style(styles::primary_btn)
                    .on_press(Message::CloseImagePreview)
                    .padding([8, 16]),
            ]
            .padding(20)
            .width(Length::Fixed(300.0))
        };

        container(content).style(styles::modal).into()
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
                    text("Delete Favorite?").size(16).color(colors::TEXT_PRIMARY),
                ]
                .align_y(iced::Alignment::Center),
                Space::new().height(16),

                // Warning message
                text("This entry is marked as a favorite.")
                    .size(12)
                    .color(colors::TEXT_SECONDARY),
                Space::new().height(8),

                // Entry preview
                container(
                    text(preview_text)
                        .size(11)
                        .color(colors::TEXT_MUTED)
                )
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
        let keyboard_sub = event::listen_with(|event, _status, _window| {
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                match key.as_ref() {
                    keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::HideWindow),
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Some(Message::MoveUp),
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Some(Message::MoveDown),
                    keyboard::Key::Named(keyboard::key::Named::Enter) => Some(Message::CopySelected),
                    keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => Some(Message::PrevPage),
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => Some(Message::NextPage),
                    keyboard::Key::Named(keyboard::key::Named::Tab) => {
                        if modifiers.shift() {
                            Some(Message::PrevTab)
                        } else {
                            Some(Message::NextTab)
                        }
                    }
                    keyboard::Key::Character(c) => {
                        let s: &str = c;
                        match s {
                            "?" => Some(Message::ToggleHelp),
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
            iced::stream::channel(10, |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                let receiver = GlobalHotKeyEvent::receiver();
                loop {
                    if let Ok(event) = receiver.try_recv() {
                        if event.state == HotKeyState::Pressed {
                            let _ = sender.try_send(Message::GlobalHotkeyPressed);
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
        });

        // IPC subscription: drain commands from any `ditox-gui --toggle` etc.
        // launched after us. Cross-platform, but a no-op on platforms where
        // the IPC server isn't running.
        let ipc_sub = Subscription::run(|| {
            iced::stream::channel(
                16,
                |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                    loop {
                        // Drain any pending commands from the global receiver.
                        while let Some(cmd) = crate::ipc_bridge::try_recv() {
                            let msg = match cmd {
                                crate::ipc::IpcCommand::Toggle => Message::ToggleWindow,
                                crate::ipc::IpcCommand::Show => Message::IpcShow,
                                crate::ipc::IpcCommand::Hide => Message::HideWindow,
                                crate::ipc::IpcCommand::Quit => Message::QuitApp,
                            };
                            let _ = sender.try_send(msg);
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                },
            )
        });

        let clipboard_sub = Subscription::run(|| {
            iced::stream::channel(10, |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
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
                    let poll_interval = POLL_INTERVAL_MS.load(std::sync::atomic::Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(poll_interval)).await;
                }
            })
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
            iced::stream::channel(10, |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                let receiver = MenuEvent::receiver();
                loop {
                    if let Ok(event) = receiver.try_recv() {
                        let _ = sender.try_send(Message::TrayMenuEvent(event.id.0.clone()));
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
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
        ]);
        #[cfg(not(windows))]
        let subs = Subscription::batch([
            keyboard_sub,
            tick_sub,
            clipboard_sub,
            focus_sub,
            tray_sub,
            ipc_sub,
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
             // Normal pagination
             if let Ok(entries) = self.db.lock().unwrap().get_page_filtered(
                self.current_page * self.config.general.max_entries,
                self.config.general.max_entries,
                filter,
                collection_id
            ) {
                self.entries = entries;
            }
            // Update counts 
             if let Ok(count) = self.db.lock().unwrap().count_filtered(filter, collection_id) {
                self.total_count = count;
            }
        } else {
            // Search mode - use new search_entries_filtered
             match self.db.lock().unwrap().search_entries_filtered(
                &self.search_query, 
                self.config.general.max_entries,
                filter,
                collection_id
            ) {
                Ok(entries) => {
                    self.total_count = entries.len();
                    self.entries = entries;
                    // Reset to first page for search results
                    self.current_page = 0; 
                }
                Err(e) => {
                    eprintln!("Search error: {}", e);
                    self.entries.clear();
                    self.total_count = 0;
                }
            }
        }
        
        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
        
        self.update_image_cache();
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
#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_linux_tray_thread() {
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
static APP_START_HIDDEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn boot_app() -> (DitoxApp, Task<Message>) {
    let config = APP_CONFIG
        .get()
        .expect("APP_CONFIG must be set before running the app")
        .clone();
    let start_hidden = APP_START_HIDDEN.load(std::sync::atomic::Ordering::Relaxed);
    let db = Database::open().expect("Failed to open database for app");
    DitoxApp::new(db, config, start_hidden)
}

pub fn run_with(_db: Database, config: Config, start_hidden: bool) -> Result<()> {
    let window_state = WindowState::load();

    // Store config for the boot function (db will be opened fresh since it's not Sync)
    let _ = APP_CONFIG.set(config);
    APP_START_HIDDEN.store(start_hidden, std::sync::atomic::Ordering::Relaxed);

    let mut settings = iced::window::Settings::default();
    settings.size = Size::new(window_state.width, window_state.height);
    settings.position = window::Position::Specific(Point::new(window_state.x, window_state.y));
    settings.icon = load_window_icon();
    // On Windows we draw our own title bar + resize zones because the custom
    // dark styling can't be applied to the native chrome. On Linux/macOS the
    // native compositor chrome integrates much better with each DE's theme,
    // so we enable it there.
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
    // `--hide` / autostart: the window should come up already hidden; the user
    // will summon it later via `ditox-gui --toggle`.
    settings.visible = !start_hidden;

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
