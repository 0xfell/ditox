# Win+D (Show Desktop) Window Restore Problem

## Problem Description

When a user presses Win+D (Show Desktop) in Windows, our clipboard manager window cannot be restored using the global hotkey (Ctrl+Shift+V). The window appears in the taskbar but doesn't come to the foreground.

### Root Cause

Win+D doesn't just change z-order - it actually **minimizes** all windows:
- Sets `IsIconic()` to `true`
- Moves window position to `-32000,-32000` (offscreen)
- The Desktop window covers everything

This is different from just losing focus. Standard `SetForegroundWindow()` calls fail because the window is minimized, not just behind other windows.

### Observed Behavior (from logs)

```
ToggleWindow: self.visible=false, actually_visible=false
Window is hidden or not foreground, showing it
force_restore_window: Found window HWND(0x7709d6), iconic=false, visible=true
```

The window reports `iconic=false` but is still not visible because:
1. The test automated correctly caught the minimized state
2. Manual testing shows the window sometimes doesn't minimize but still fails to foreground

## Solutions Explored

### Solution 1: SW_RESTORE Before SetForegroundWindow (Current Implementation)

```rust
// Check if minimized
if IsIconic(hwnd).as_bool() {
    ShowWindow(hwnd, SW_RESTORE);
}

// Show window and bring to foreground
ShowWindow(hwnd, SW_SHOWNORMAL);
SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_SHOWWINDOW);
BringWindowToTop(hwnd);
SetForegroundWindow(hwnd);

// Remove TOPMOST after brief moment
std::thread::sleep(Duration::from_millis(100));
SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_SHOWWINDOW);
```

**Status**: Partially working. The automated test passes but manual testing shows inconsistent behavior.

### Solution 2: WS_EX_TOPMOST Extended Style (Permanent)

Add `WS_EX_TOPMOST` to keep window always on top:

```cpp
SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
```

**Trade-off**: Window stays in front of ALL windows, which is annoying for a clipboard manager.

### Solution 3: Hook Show Desktop Message (Most Robust)

Install a `WH_GETMESSAGE` hook in Explorer process to detect Show Desktop:

- Windows posts undocumented message `WM_USER+83` to Program Manager window
- This triggers on Win+D, Show Desktop button, and Peek Desktop
- Hook can adjust window z-order when detected

**Trade-off**:
- Requires DLL injection into Explorer
- Version-dependent (tested on Win10 22H2)
- Complex to implement

### Solution 4: Low-Level Keyboard Hook

Detect Win+D key combination and temporarily make window topmost:

```rust
let hook = SetWindowsHookEx(WH_KEYBOARD_LL, keyboard_proc, NULL, 0);
// Detect Win+D, then use SetWindowPos with HWND_TOPMOST
```

**Trade-off**: Only works for Win+D shortcut, not Show Desktop button.

### Solution 5: WM_SYSCOMMAND with SC_RESTORE

```cpp
SendMessage(hwnd, WM_SYSCOMMAND, SC_RESTORE, NULL);
SetForegroundWindow(hwnd);
```

Alternative to `ShowWindow(SW_RESTORE)`.

## How Other Apps Handle This

### Ditto Clipboard Manager

[Ditto](https://github.com/sabrogden/Ditto) is a popular Windows clipboard manager. Based on research:

- Uses hotkey `Ctrl+`` (backtick) by default
- Creates a popup window that appears at cursor position
- Source code is C++/MFC based
- May use similar `SetForegroundWindow` + `TOPMOST` tricks

No specific documentation found on how Ditto handles Win+D scenario.

### Windows Built-in Clipboard (Win+V)

Windows 11's native clipboard history (Win+V) seems to handle this correctly, likely because:
- It's a system component with special privileges
- May use undocumented shell APIs
- Can bypass foreground restrictions

## Recommended Approach for Ditox

1. **Current Implementation** (in `force_restore_window()`):
   - Use `EnumWindows` to find our main window by PID
   - Check `IsIconic()` and call `SW_RESTORE` if minimized
   - Temporarily set `HWND_TOPMOST` to appear above desktop
   - Call `BringWindowToTop` + `SetForegroundWindow`
   - Remove `TOPMOST` flag after 100ms

2. **Future Improvements**:
   - Consider using `AttachThreadInput` to attach to foreground thread
   - Try `SystemParametersInfo(SPI_SETFOREGROUNDLOCKTIMEOUT, 0, ...)` to disable foreground lock
   - Consider implementing the Explorer hook for more robust detection

## Test Script

See `test_win_d.ps1` for automated testing:
- Uses `SendInput` API (more reliable than `keybd_event`)
- Detects window using `EnumWindows` (more reliable than `MainWindowHandle`)
- Checks `IsIconic`, `WS_VISIBLE` style, and window position

## References

- [Microsoft Q&A: How to avoid minimize on show desktop](https://learn.microsoft.com/en-us/answers/questions/2127546/how-to-avoid-minimize-on-show-desktop-or-peek-desk)
- [AutoHotkey Community: Show Desktop Minimize and Restore](https://www.autohotkey.com/board/topic/69180-show-desktop-minimize-and-restore-all-windows/)
- [SetWindowPos function documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)
- [Ditto Clipboard Manager](https://github.com/sabrogden/Ditto)
