# Test script for Win+D window restore issue
# This script:
# 1. Builds and starts ditox-gui
# 2. Waits for it to initialize
# 3. Shows the window with Ctrl+Shift+V
# 4. Presses Win+D to show desktop
# 5. Tries to restore with Ctrl+Shift+V
# 6. Checks if window is visible AND in foreground

Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
    using System;
    using System.Runtime.InteropServices;
    using System.Diagnostics;
    using System.Drawing;
    using System.Collections.Generic;

    public class Win32Helper {
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct INPUT {
            public uint type;
            public INPUTUNION u;
        }

        [StructLayout(LayoutKind.Explicit)]
        public struct INPUTUNION {
            [FieldOffset(0)] public KEYBDINPUT ki;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct KEYBDINPUT {
            public ushort wVk;
            public ushort wScan;
            public uint dwFlags;
            public uint time;
            public IntPtr dwExtraInfo;
        }

        public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

        [DllImport("user32.dll")]
        public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

        [DllImport("user32.dll")]
        public static extern bool IsWindowVisible(IntPtr hWnd);

        [DllImport("user32.dll")]
        public static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

        [DllImport("user32.dll", SetLastError = true)]
        public static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);

        [DllImport("user32.dll")]
        public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

        [DllImport("user32.dll")]
        public static extern bool IsIconic(IntPtr hWnd);

        [DllImport("user32.dll")]
        public static extern int GetWindowLong(IntPtr hWnd, int nIndex);

        public const int GWL_STYLE = -16;
        public const int GWL_EXSTYLE = -20;
        public const int WS_VISIBLE = 0x10000000;
        public const int WS_MINIMIZE = 0x20000000;
        public const int WS_SIZEBOX = 0x00040000;
        public const int WS_EX_TOOLWINDOW = 0x00000080;
        public const int WS_EX_NOACTIVATE = 0x08000000;

        public const uint INPUT_KEYBOARD = 1;
        public const uint KEYEVENTF_KEYUP = 0x0002;
        public const ushort VK_LWIN = 0x5B;
        public const ushort VK_D = 0x44;
        public const ushort VK_CONTROL = 0x11;
        public const ushort VK_SHIFT = 0x10;
        public const ushort VK_V = 0x56;

        private static uint _targetPid;
        private static IntPtr _foundHwnd;

        private static INPUT CreateKeyInput(ushort vk, bool keyUp) {
            INPUT input = new INPUT();
            input.type = INPUT_KEYBOARD;
            input.u.ki.wVk = vk;
            input.u.ki.wScan = 0;
            input.u.ki.dwFlags = keyUp ? KEYEVENTF_KEYUP : 0;
            input.u.ki.time = 0;
            input.u.ki.dwExtraInfo = IntPtr.Zero;
            return input;
        }

        [DllImport("user32.dll", SetLastError = true)]
        public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        public static extern IntPtr SendMessage(IntPtr hWnd, UInt32 Msg, IntPtr wParam, IntPtr lParam);

        public const uint WM_COMMAND = 0x0111;
        public const int TOGGLE_DESKTOP = 407;

        public static void SendToggleDesktopMessage() {
            IntPtr hwnd = FindWindow("Shell_TrayWnd", null);
            SendMessage(hwnd, WM_COMMAND, (IntPtr)TOGGLE_DESKTOP, IntPtr.Zero);
        }

        public static void SendWinD() {
            SendToggleDesktopMessage();
        }

        [DllImport("user32.dll")]
        public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

        public static void SendCtrlShiftV() {
            // Ctrl Down
            keybd_event((byte)VK_CONTROL, 0, 0, UIntPtr.Zero);
            System.Threading.Thread.Sleep(50);

            // Shift Down
            keybd_event((byte)VK_SHIFT, 0, 0, UIntPtr.Zero);
            System.Threading.Thread.Sleep(50);

            // V Down
            keybd_event((byte)VK_V, 0, 0, UIntPtr.Zero);
            System.Threading.Thread.Sleep(50);

            // V Up
            keybd_event((byte)VK_V, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
            System.Threading.Thread.Sleep(50);

            // Shift Up
            keybd_event((byte)VK_SHIFT, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
            System.Threading.Thread.Sleep(50);

            // Ctrl Up
            keybd_event((byte)VK_CONTROL, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
            System.Threading.Thread.Sleep(50);
        }

        private static bool EnumWindowsCallback(IntPtr hWnd, IntPtr lParam) {
            uint pid;
            GetWindowThreadProcessId(hWnd, out pid);
            if (pid == _targetPid) {
                int style = GetWindowLong(hWnd, GWL_STYLE);
                int exStyle = GetWindowLong(hWnd, GWL_EXSTYLE);

                bool isToolWindow = (exStyle & WS_EX_TOOLWINDOW) != 0;
                bool isNoActivate = (exStyle & WS_EX_NOACTIVATE) != 0;
                bool hasSizebox = (style & WS_SIZEBOX) != 0;

                // Find main window (has sizebox, not a tool window)
                if (!isToolWindow && !isNoActivate && hasSizebox) {
                    _foundHwnd = hWnd;
                    return false; // Stop enumeration
                }
            }
            return true; // Continue enumeration
        }

        // Find the main window for a process using EnumWindows
        public static IntPtr FindMainWindow(uint processId) {
            _targetPid = processId;
            _foundHwnd = IntPtr.Zero;
            EnumWindows(EnumWindowsCallback, IntPtr.Zero);
            return _foundHwnd;
        }

        public static bool IsWindowInForeground(uint targetPid) {
            IntPtr fg = GetForegroundWindow();
            if (fg == IntPtr.Zero) return false;
            uint fgPid;
            GetWindowThreadProcessId(fg, out fgPid);
            return fgPid == targetPid;
        }

        // Check if window is truly visible (not minimized, has valid rect, style says visible)
        public static bool IsWindowTrulyVisible(IntPtr hWnd) {
            if (hWnd == IntPtr.Zero) return false;

            // Check WS_VISIBLE style
            int style = GetWindowLong(hWnd, GWL_STYLE);
            if ((style & WS_VISIBLE) == 0) return false;

            // Check not minimized
            if (IsIconic(hWnd)) return false;
            if ((style & WS_MINIMIZE) != 0) return false;

            // Check has valid window rect with non-zero size
            RECT rect;
            if (!GetWindowRect(hWnd, out rect)) return false;
            int width = rect.Right - rect.Left;
            int height = rect.Bottom - rect.Top;

            // Allow (0,0,0,0) during window setup - just check not minimized offscreen
            if (rect.Left == -32000 || rect.Top == -32000) return false;

            // For now, accept if style says visible and not iconic
            return true;
        }

        // Get window position info for debugging
        public static string GetWindowInfo(IntPtr hWnd) {
            if (hWnd == IntPtr.Zero) return "NULL handle";

            RECT rect;
            GetWindowRect(hWnd, out rect);
            int style = GetWindowLong(hWnd, GWL_STYLE);
            bool iconic = IsIconic(hWnd);

            return string.Format("Rect=({0},{1},{2},{3}) Style=0x{4:X} Iconic={5}",
                rect.Left, rect.Top, rect.Right, rect.Bottom, style, iconic);
        }
    }
"@

function Get-WindowState {
    param([uint32]$processId)
    $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
    if (-not $proc) { return @{ Visible = $false; TrulyVisible = $false; Foreground = $false; Handle = [IntPtr]::Zero; Info = "Process not found" } }

    # Use EnumWindows to find the main window (more reliable than MainWindowHandle)
    $hwnd = [Win32Helper]::FindMainWindow($processId)
    if ($hwnd -eq [IntPtr]::Zero) {
        return @{ Visible = $false; TrulyVisible = $false; Foreground = $false; Handle = [IntPtr]::Zero; Info = "Window not found via EnumWindows" }
    }

    $visible = [Win32Helper]::IsWindowVisible($hwnd)
    $trulyVisible = [Win32Helper]::IsWindowTrulyVisible($hwnd)
    $foreground = [Win32Helper]::IsWindowInForeground($processId)
    $info = [Win32Helper]::GetWindowInfo($hwnd)

    return @{
        Visible = $visible
        TrulyVisible = $trulyVisible
        Foreground = $foreground
        Handle = $hwnd
        Info = $info
    }
}

Write-Host "=== Ditox Win+D Test ===" -ForegroundColor Cyan
Write-Host ""

# Kill any existing ditox-gui
Write-Host "Stopping any existing ditox-gui processes..." -ForegroundColor Yellow
Stop-Process -Name "ditox-gui" -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# Build first
Write-Host "Building ditox-gui..." -ForegroundColor Yellow
$buildResult = & cargo build -p ditox-gui 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Build failed" -ForegroundColor Red
    Write-Host $buildResult
    exit 1
}
Write-Host "Build successful" -ForegroundColor Green

# Start ditox-gui directly (not through cargo run to avoid cargo's window)
# Cleanup previous log
if (Test-Path "$PSScriptRoot\app.log") { Remove-Item "$PSScriptRoot\app.log" -Force }

Write-Host "Starting ditox-gui..." -ForegroundColor Yellow
$ditoxProcess = Start-Process -FilePath "cmd.exe" -ArgumentList "/c `".\target\debug\ditox-gui.exe > app.log 2>&1`"" -PassThru -WindowStyle Normal
Start-Sleep -Seconds 2

if (-not $ditoxProcess -or $ditoxProcess.HasExited) {
    Write-Host "ERROR: ditox-gui failed to start" -ForegroundColor Red
    if (Test-Path "$PSScriptRoot\app.log") { Get-Content "$PSScriptRoot\app.log" }
    exit 1
}

if (-not $ditoxProcess -or $ditoxProcess.HasExited) {
    Write-Host "ERROR: ditox-gui failed to start" -ForegroundColor Red
    if (Test-Path "$PSScriptRoot\app.log") { Get-Content "$PSScriptRoot\app.log" }
    exit 1
}

# Find actual ditox-gui process ID (since we used cmd wrapper)
$targetPid = 0
for ($i = 0; $i -lt 10; $i++) {
    $proc = Get-Process -Name "ditox-gui" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($proc) {
        $targetPid = $proc.Id
        break
    }
    Start-Sleep -Milliseconds 500
}

if ($targetPid -eq 0) {
    Write-Host "ERROR: Could not find ditox-gui process" -ForegroundColor Red
    exit 1
}

Write-Host "ditox-gui started (PID: $targetPid)" -ForegroundColor Green
Start-Sleep -Seconds 1

# Test 1: Check if window is visible. If not, show it.
Write-Host ""
Write-Host "Test 1: checking window visibility..." -ForegroundColor Yellow

$state1 = Get-WindowState -processId $targetPid
if (-not $state1.TrulyVisible) {
    Write-Host "Window hidden, showing with Ctrl+Shift+V..." -ForegroundColor Yellow
    [Win32Helper]::SendCtrlShiftV()
    
    # Wait for window to appear
    for ($i = 0; $i -lt 10; $i++) {
        Start-Sleep -Milliseconds 200
        $state1 = Get-WindowState -processId $targetPid
        if ($state1.TrulyVisible) { break }
    }
} else {
    Write-Host "Window already visible." -ForegroundColor Green
}

$test1Pass = $state1.TrulyVisible -and $state1.Foreground
Write-Host "  TrulyVisible: $($state1.TrulyVisible), Foreground: $($state1.Foreground)" -ForegroundColor $(if ($test1Pass) { "Green" } else { "Red" })
Write-Host "  Info: $($state1.Info)" -ForegroundColor Gray

if (-not $state1.TrulyVisible) {
    Write-Host "  ERROR: Window didn't show." -ForegroundColor Red
    Stop-Process -Id $targetPid -ErrorAction SilentlyContinue
    exit 1
}

# Update process ID for subsequent checks
$ditoxProcess = [PSCustomObject]@{ Id = $targetPid }

# Test 2: Press Win+D to show desktop
Write-Host ""
Write-Host "Test 2: Pressing Win+D to show desktop..." -ForegroundColor Yellow
[Win32Helper]::SendWinD()
Start-Sleep -Seconds 1

$state2 = Get-WindowState -processId $ditoxProcess.Id
Write-Host "  TrulyVisible: $($state2.TrulyVisible), Foreground: $($state2.Foreground)" -ForegroundColor $(if (-not $state2.Foreground) { "Green" } else { "Red" })
Write-Host "  Info: $($state2.Info)" -ForegroundColor Gray

if ($state2.TrulyVisible -or $state2.Foreground) {
    Write-Host "  ERROR: Window was not hidden by Win+D" -ForegroundColor Red
    Stop-Process -Id $ditoxProcess.Id -ErrorAction SilentlyContinue
    exit 1
}

# Test 3: Try to restore with Ctrl+Shift+V
Write-Host ""
Write-Host "Test 3: Restoring window with Ctrl+Shift+V..." -ForegroundColor Yellow
[Win32Helper]::SendCtrlShiftV()
Start-Sleep -Seconds 1

$state3 = Get-WindowState -processId $ditoxProcess.Id
$test3Pass = $state3.TrulyVisible -and $state3.Foreground
Write-Host "  TrulyVisible: $($state3.TrulyVisible), Foreground: $($state3.Foreground)" -ForegroundColor $(if ($test3Pass) { "Green" } else { "Red" })
Write-Host "  Info: $($state3.Info)" -ForegroundColor Gray

# Summary
Write-Host ""
Write-Host "=== Test Summary ===" -ForegroundColor Cyan
if ($test3Pass) {
    Write-Host "PASSED: Window correctly restored after Win+D" -ForegroundColor Green
    $exitCode = 0
} else {
    Write-Host "FAILED: Window did not restore after Win+D" -ForegroundColor Red
    Write-Host "  Final state - TrulyVisible: $($state3.TrulyVisible), Foreground: $($state3.Foreground)" -ForegroundColor Red
    Write-Host "  Window Info: $($state3.Info)" -ForegroundColor Red
    $exitCode = 1
}

# Cleanup
Write-Host ""
Write-Host "Cleaning up..." -ForegroundColor Yellow
Stop-Process -Id $ditoxProcess.Id -ErrorAction SilentlyContinue

exit $exitCode
