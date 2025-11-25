# Task: Windows 11 Support

> **Status:** in-progress
> **Priority:** high
> **Created:** 2025-12-05
> **Completed:**

## Description

Add Windows 11 support to ditox clipboard manager. Currently the app only works on Linux with Wayland. This task abstracts the clipboard layer to support multiple platforms using compile-time feature flags.

## Requirements

- [x] Create platform abstraction for clipboard operations
- [x] Implement Windows clipboard provider using `arboard` crate
- [x] Keep existing Wayland support via conditional compilation
- [x] Fix process checking in watcher.rs for Windows
- [x] Update Cargo.toml with platform-specific dependencies
- [x] Test build on Windows 11

## Implementation Notes

### Platform-Specific Code Identified
1. `clipboard.rs` - Uses `wl-clipboard-rs` and `wl-copy` CLI (100% Wayland-specific)
2. `watcher.rs` - Uses `libc::kill()` for Unix process checking

### Strategy
- Use conditional compilation with `#[cfg(unix)]` and `#[cfg(windows)]` attributes
- Keep `wl-clipboard-rs` for Linux/Wayland via target-specific dependencies
- Add `arboard` crate for Windows (also works on macOS for future support)
- Add `sysinfo` crate for Windows process checking

### Dependencies Added (Platform-Specific)

**Unix (Linux):**
- `wl-clipboard-rs` - Wayland clipboard library
- `libc` - For process signal checking

**Windows:**
- `arboard` - Cross-platform clipboard library
- `sysinfo` - Cross-platform process/system info

### Files Modified
1. `Cargo.toml` - Added platform-specific dependencies
2. `src/clipboard.rs` - Refactored with `#[cfg(unix)]` and `#[cfg(windows)]` modules
3. `src/watcher.rs` - Added `is_process_running_by_pid()` with platform implementations

## Testing

- `cargo build` on Windows succeeds
- 19/19 unit tests pass
- CLI integration tests have pre-existing issues with XDG environment variables (not related to this change)

## Known Issues

The CLI integration tests use `XDG_DATA_HOME` environment variable which is Linux-specific. The `directories` crate on Windows uses `APPDATA` instead. This is a pre-existing test infrastructure issue.

## Work Log

### 2025-12-05
- Analyzed codebase for platform-specific code
- Refactored `clipboard.rs` with separate Unix (Wayland) and Windows implementations
- Updated `watcher.rs` with cross-platform process checking using `sysinfo`
- Updated `Cargo.toml` with target-specific dependencies
- Build succeeds on Windows 11
- All 19 unit tests pass
