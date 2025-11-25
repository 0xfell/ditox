//! Windows startup registration using auto-launch

#[cfg(windows)]
use auto_launch::AutoLaunchBuilder;

/// Enable or disable running Ditox on Windows startup
#[cfg(windows)]
pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_path_str = exe_path.to_string_lossy().to_string();

    tracing::debug!("Setting startup enabled={} for path: {}", enabled, exe_path_str);

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
        Ok(()) => tracing::debug!("Auto-launch {} succeeded", if enabled { "enable" } else { "disable" }),
        Err(e) => tracing::error!("Auto-launch {} failed: {}", if enabled { "enable" } else { "disable" }, e),
    }

    result.map_err(|e| format!("Failed to {} startup: {}", if enabled { "enable" } else { "disable" }, e))
}

/// Check if Ditox is configured to run on Windows startup
#[cfg(windows)]
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

    tracing::debug!("Checking startup enabled for {}: {}", exe_path_str, result);
    result
}

// Stub implementations for non-Windows platforms
#[cfg(not(windows))]
pub fn set_startup_enabled(_enabled: bool) -> Result<(), String> {
    Err("Startup registration is only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn is_startup_enabled() -> bool {
    false
}
