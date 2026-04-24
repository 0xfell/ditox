//! Startup (run-at-login) registration.
//!
//! On Windows this uses `auto-launch` which writes to the Run registry key.
//! On Linux this writes an XDG autostart `.desktop` file to `~/.config/autostart`.

#[cfg(windows)]
mod imp {
    use auto_launch::AutoLaunchBuilder;

    pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
        let exe_path_str = exe_path.to_string_lossy().to_string();

        tracing::debug!(
            "Setting startup enabled={} for path: {}",
            enabled,
            exe_path_str
        );

        let auto_launch = AutoLaunchBuilder::new()
            .set_app_name("Ditox")
            .set_app_path(&exe_path_str)
            .set_use_launch_agent(false) // Use registry on Windows
            .build()
            .map_err(|e| format!("Failed to create auto-launch: {}", e))?;

        let result = if enabled {
            auto_launch.enable()
        } else {
            auto_launch.disable()
        };

        match &result {
            Ok(()) => tracing::debug!(
                "Auto-launch {} succeeded",
                if enabled { "enable" } else { "disable" }
            ),
            Err(e) => tracing::error!(
                "Auto-launch {} failed: {}",
                if enabled { "enable" } else { "disable" },
                e
            ),
        }

        result.map_err(|e| {
            format!(
                "Failed to {} startup: {}",
                if enabled { "enable" } else { "disable" },
                e
            )
        })
    }

    pub fn is_startup_enabled() -> bool {
        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return false,
        };

        let exe_path_str = exe_path.to_string_lossy().to_string();

        let result = AutoLaunchBuilder::new()
            .set_app_name("Ditox")
            .set_app_path(&exe_path_str)
            .build()
            .map(|al| al.is_enabled().unwrap_or(false))
            .unwrap_or(false);

        tracing::debug!(
            "Checking startup enabled for {}: {}",
            exe_path_str,
            result
        );
        result
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use std::fs;
    use std::path::PathBuf;

    fn autostart_path() -> Option<PathBuf> {
        // Respect XDG_CONFIG_HOME, fall back to ~/.config
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
            })?;
        Some(base.join("autostart").join("ditox-gui.desktop"))
    }

    fn desktop_contents(exe: &str) -> String {
        // `--hide` so we start in the tray / hidden (summoned later via --toggle).
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Ditox\n\
             Comment=Clipboard manager\n\
             Exec={} --hide\n\
             Icon=ditox\n\
             Terminal=false\n\
             Categories=Utility;\n\
             X-GNOME-Autostart-enabled=true\n",
            exe
        )
    }

    pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
        let path = autostart_path().ok_or("Could not resolve XDG config dir")?;

        if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| format!("Failed to get exe path: {}", e))?
                .to_string_lossy()
                .to_string();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create autostart dir: {}", e))?;
            }
            fs::write(&path, desktop_contents(&exe))
                .map_err(|e| format!("Failed to write autostart file: {}", e))?;
            tracing::debug!("Wrote autostart file: {}", path.display());
        } else {
            match fs::remove_file(&path) {
                Ok(_) => tracing::debug!("Removed autostart file: {}", path.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Failed to remove autostart file: {}", e)),
            }
        }
        Ok(())
    }

    pub fn is_startup_enabled() -> bool {
        autostart_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }
}

#[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
mod imp {
    pub fn set_startup_enabled(_enabled: bool) -> Result<(), String> {
        Err("Startup registration is not supported on this platform".to_string())
    }
    pub fn is_startup_enabled() -> bool {
        false
    }
}

pub use imp::{is_startup_enabled, set_startup_enabled};
