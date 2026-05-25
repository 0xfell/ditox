//! Platform / compositor detection.
//!
//! Detects the OS and (on Linux) the active compositor / desktop
//! environment. Cached process-wide via `OnceLock`.
//!
//! Used to gate compositor-specific code paths in Phases 2, 5, 6 of
//! the v1.0 master plan:
//! - Hyprland uses `hyprctl` for foreground/cursor/sendshortcut.
//! - Sway uses `swaymsg` for foreground.
//! - Wlroots compositors get `wlr-foreign-toplevel`.
//! - GNOME Wayland is degraded (no wlr-* protocols, no in-process hotkey).
//! - Windows / macOS get their native APIs.
//!
//! Detection is **cheap and never panics**. Each probe is wrapped in
//! `Result`; on any error we fall back to `Unknown` and log once.

use std::sync::OnceLock;

/// Top-level platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    /// Linux desktop. The inner value names the compositor / DE.
    Linux(LinuxCompositor),
    /// Windows. Inner value is `(major, minor, build)`.
    Windows(WindowsVersion),
    /// macOS. Inner value is `(major, minor, patch)`.
    Macos(MacosVersion),
    /// Anything else (BSD, Solaris, headless, …).
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxCompositor {
    Hyprland {
        signature: Option<String>,
    },
    Sway {
        sock: Option<String>,
    },
    Kde {
        wayland: bool,
    },
    Gnome {
        wayland: bool,
    },
    /// Generic wlroots-based compositor we don't specifically recognise.
    Wlroots {
        name: String,
    },
    /// X11-only desktop (no `WAYLAND_DISPLAY`).
    X11Only {
        name: Option<String>,
    },
    /// Couldn't tell — env vars missing or contradictory.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

static DETECTED: OnceLock<Platform> = OnceLock::new();

/// Detect (or return cached) platform. First call performs the
/// detection; subsequent calls are O(1).
pub fn detect() -> &'static Platform {
    DETECTED.get_or_init(do_detect)
}

/// Force re-detection. **Test-only**; never call from production code.
/// Note: `OnceLock::set` will fail silently if already initialised, so
/// this is best-effort.
#[doc(hidden)]
pub fn force_detect_for_test() -> Platform {
    do_detect()
}

fn do_detect() -> Platform {
    #[cfg(target_os = "windows")]
    {
        return Platform::Windows(detect_windows_version().unwrap_or(WindowsVersion {
            major: 0,
            minor: 0,
            build: 0,
        }));
    }

    #[cfg(target_os = "macos")]
    {
        return Platform::Macos(detect_macos_version().unwrap_or(MacosVersion {
            major: 0,
            minor: 0,
            patch: 0,
        }));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return Platform::Linux(detect_linux_compositor());
    }

    #[allow(unreachable_code)]
    Platform::Other
}

#[cfg(all(unix, not(target_os = "macos")))]
fn detect_linux_compositor() -> LinuxCompositor {
    // Hyprland: most reliable signal is HYPRLAND_INSTANCE_SIGNATURE.
    // XDG_CURRENT_DESKTOP can also be "Hyprland".
    if let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        return LinuxCompositor::Hyprland {
            signature: Some(sig),
        };
    }
    if env_eq_case_insensitive("XDG_CURRENT_DESKTOP", "Hyprland") {
        return LinuxCompositor::Hyprland { signature: None };
    }

    // Sway: SWAYSOCK is the canonical signal.
    if let Ok(sock) = std::env::var("SWAYSOCK") {
        return LinuxCompositor::Sway { sock: Some(sock) };
    }
    if env_eq_case_insensitive("XDG_CURRENT_DESKTOP", "sway") {
        return LinuxCompositor::Sway { sock: None };
    }

    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();

    // KDE / Plasma.
    if env_eq_case_insensitive("XDG_CURRENT_DESKTOP", "KDE")
        || env_eq_case_insensitive("XDG_SESSION_DESKTOP", "KDE")
        || env_eq_case_insensitive("XDG_SESSION_DESKTOP", "plasma")
    {
        return LinuxCompositor::Kde { wayland };
    }

    // GNOME.
    if env_eq_case_insensitive("XDG_CURRENT_DESKTOP", "GNOME")
        || env_eq_case_insensitive("XDG_SESSION_DESKTOP", "gnome")
    {
        return LinuxCompositor::Gnome { wayland };
    }

    // Generic wlroots fallback for an unfamiliar Wayland compositor.
    if wayland {
        let name = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "wlroots".to_string());
        return LinuxCompositor::Wlroots { name };
    }

    // No wayland → X11 desktop.
    let display = std::env::var("XDG_CURRENT_DESKTOP").ok();
    if std::env::var_os("DISPLAY").is_some() {
        return LinuxCompositor::X11Only { name: display };
    }

    LinuxCompositor::Unknown
}

#[cfg(all(unix, not(target_os = "macos")))]
fn env_eq_case_insensitive(key: &str, value: &str) -> bool {
    std::env::var(key)
        .map(|v| v.eq_ignore_ascii_case(value))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn detect_windows_version() -> Option<WindowsVersion> {
    // Best-effort: read OSVERSIONINFOEXW via `windows` crate would
    // require pulling in the dep just for this. For now expose
    // (0, 0, 0) and let later phases swap in a richer detector when
    // they need version-specific behaviour.
    Some(WindowsVersion {
        major: 0,
        minor: 0,
        build: 0,
    })
}

#[cfg(target_os = "macos")]
fn detect_macos_version() -> Option<MacosVersion> {
    Some(MacosVersion {
        major: 0,
        minor: 0,
        patch: 0,
    })
}

impl Platform {
    /// Does this platform expose `wlr-foreign-toplevel-management-v1`?
    /// Used for the Wayland foreground tracker (Phase 2).
    pub fn supports_wlr_foreign_toplevel(&self) -> bool {
        matches!(
            self,
            Platform::Linux(
                LinuxCompositor::Hyprland { .. }
                    | LinuxCompositor::Sway { .. }
                    | LinuxCompositor::Kde { wayland: true }
                    | LinuxCompositor::Wlroots { .. }
            )
        )
    }

    /// Can the application register a global hotkey via its own
    /// process? On Wayland, hotkey ownership is the compositor's; the
    /// user must bind from compositor config.
    pub fn supports_global_hotkey_in_app(&self) -> bool {
        matches!(self, Platform::Windows(_) | Platform::Macos(_))
    }

    /// Is `hyprctl` likely available on this system?
    pub fn supports_hyprctl(&self) -> bool {
        matches!(self, Platform::Linux(LinuxCompositor::Hyprland { .. }))
    }

    /// Ordered list of paste-synthesis strategies to try, by name.
    /// The strings match config-file enum values.
    pub fn paste_synthesizer_chain(&self) -> Vec<&'static str> {
        match self {
            Platform::Windows(_) => vec!["sendinput"],
            Platform::Macos(_) => vec!["cgevent"],
            Platform::Linux(LinuxCompositor::Hyprland { .. }) => {
                vec!["hyprctl", "wtype", "ydotool"]
            }
            Platform::Linux(LinuxCompositor::Sway { .. }) => vec!["wtype", "ydotool"],
            Platform::Linux(LinuxCompositor::Wlroots { .. }) => vec!["wtype", "ydotool"],
            Platform::Linux(LinuxCompositor::Kde { wayland: true }) => vec!["wtype", "ydotool"],
            Platform::Linux(LinuxCompositor::Gnome { wayland: true }) => vec!["ydotool"],
            Platform::Linux(LinuxCompositor::X11Only { .. }) => vec!["xdotool", "ydotool"],
            // Unknown / non-wayland desktop: nothing reliable.
            _ => vec![],
        }
    }

    /// Returns a short slug suitable for status output.
    pub fn slug(&self) -> &'static str {
        match self {
            Platform::Windows(_) => "windows",
            Platform::Macos(_) => "macos",
            Platform::Linux(LinuxCompositor::Hyprland { .. }) => "hyprland",
            Platform::Linux(LinuxCompositor::Sway { .. }) => "sway",
            Platform::Linux(LinuxCompositor::Kde { wayland: true }) => "kde-wayland",
            Platform::Linux(LinuxCompositor::Kde { wayland: false }) => "kde-x11",
            Platform::Linux(LinuxCompositor::Gnome { wayland: true }) => "gnome-wayland",
            Platform::Linux(LinuxCompositor::Gnome { wayland: false }) => "gnome-x11",
            Platform::Linux(LinuxCompositor::Wlroots { .. }) => "wlroots",
            Platform::Linux(LinuxCompositor::X11Only { .. }) => "x11",
            Platform::Linux(LinuxCompositor::Unknown) => "linux-unknown",
            Platform::Other => "other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to mutate env, run a test, and restore. Sequenced via a
    /// single mutex because every test case here mucks with the
    /// process environment.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        // Save originals.
        let saved: Vec<_> = vars
            .iter()
            .map(|(k, _)| (k.to_string(), std::env::var_os(k)))
            .collect();
        // Apply.
        for (k, v) in vars {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        f();
        // Restore.
        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(&k, v),
                None => std::env::remove_var(&k),
            }
        }
    }

    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn detects_hyprland_via_signature() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", Some("abc123_999")),
                ("SWAYSOCK", None),
                ("XDG_CURRENT_DESKTOP", None),
                ("WAYLAND_DISPLAY", Some("wayland-0")),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "hyprland");
                assert!(p.supports_hyprctl());
                assert!(!p.supports_global_hotkey_in_app());
                assert!(p.supports_wlr_foreign_toplevel());
                assert_eq!(
                    p.paste_synthesizer_chain(),
                    vec!["hyprctl", "wtype", "ydotool"]
                );
            },
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn detects_hyprland_via_xdg() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", None),
                ("SWAYSOCK", None),
                ("XDG_CURRENT_DESKTOP", Some("Hyprland")),
                ("WAYLAND_DISPLAY", Some("wayland-0")),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "hyprland");
            },
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn detects_sway_via_swaysock() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", None),
                ("SWAYSOCK", Some("/tmp/sway.sock")),
                ("XDG_CURRENT_DESKTOP", None),
                ("WAYLAND_DISPLAY", Some("wayland-0")),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "sway");
                assert!(!p.supports_hyprctl());
                assert_eq!(p.paste_synthesizer_chain(), vec!["wtype", "ydotool"]);
            },
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn detects_kde_wayland() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", None),
                ("SWAYSOCK", None),
                ("XDG_CURRENT_DESKTOP", Some("KDE")),
                ("WAYLAND_DISPLAY", Some("wayland-0")),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "kde-wayland");
                assert!(p.supports_wlr_foreign_toplevel());
            },
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn detects_gnome_wayland_as_degraded() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", None),
                ("SWAYSOCK", None),
                ("XDG_CURRENT_DESKTOP", Some("GNOME")),
                ("WAYLAND_DISPLAY", Some("wayland-0")),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "gnome-wayland");
                assert!(!p.supports_wlr_foreign_toplevel());
                assert_eq!(p.paste_synthesizer_chain(), vec!["ydotool"]);
            },
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn detects_x11_only() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", None),
                ("SWAYSOCK", None),
                ("XDG_CURRENT_DESKTOP", Some("XFCE")),
                ("WAYLAND_DISPLAY", None),
                ("DISPLAY", Some(":0")),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "x11");
                assert_eq!(p.paste_synthesizer_chain(), vec!["xdotool", "ydotool"]);
            },
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn unknown_when_no_signals() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("HYPRLAND_INSTANCE_SIGNATURE", None),
                ("SWAYSOCK", None),
                ("XDG_CURRENT_DESKTOP", None),
                ("WAYLAND_DISPLAY", None),
                ("DISPLAY", None),
            ],
            || {
                let p = force_detect_for_test();
                assert_eq!(p.slug(), "linux-unknown");
                assert!(p.paste_synthesizer_chain().is_empty());
            },
        );
    }

    #[test]
    fn detect_returns_cached_value() {
        // Just exercise the public path — value depends on host.
        let a = detect();
        let b = detect();
        assert!(std::ptr::eq(a, b));
    }
}
