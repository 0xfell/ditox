use std::process::{Command, Stdio};

pub fn clipboard_tools_roundtrip() {
    #[cfg(target_os = "linux")]
    linux_roundtrip();
    #[cfg(target_os = "macos")]
    macos_roundtrip();
    #[cfg(target_os = "windows")]
    windows_roundtrip();
}

#[cfg(target_os = "linux")]
fn linux_roundtrip() {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    println!("session: {}", if wayland { "wayland" } else { "unknown/x11" });
    // wl-copy/paste
    let has_wl_copy = Command::new("wl-copy").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).spawn().and_then(|mut c| c.wait()).map(|s| s.success()).unwrap_or(false);
    let has_wl_paste = Command::new("wl-paste").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).spawn().and_then(|mut c| c.wait()).map(|s| s.success()).unwrap_or(false);
    println!("wl-clipboard: {}", if has_wl_copy && has_wl_paste { "present" } else { "missing" });
    if has_wl_copy && has_wl_paste {
        let ok = Command::new("sh")
            .arg("-lc")
            .arg("printf test | wl-copy && sleep 0.05 && wl-paste")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "test")
            .unwrap_or(false);
        println!("wl roundtrip: {}", if ok { "ok" } else { "failed" });
        if !ok {
            println!("hint: ensure your compositor exposes wl-data-control; try installing wl-clipboard and running inside your Wayland session.");
        }
    } else if wayland {
        println!("hint: install wl-clipboard (package: wl-clipboard)");
    }
    // X11 fallbacks
    let has_xclip = Command::new("xclip").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).spawn().and_then(|mut c| c.wait()).map(|s| s.success()).unwrap_or(false);
    let has_xsel = Command::new("xsel").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).spawn().and_then(|mut c| c.wait()).map(|s| s.success()).unwrap_or(false);
    println!("xclip: {} | xsel: {}", if has_xclip { "present" } else { "missing" }, if has_xsel { "present" } else { "missing" });
}

#[cfg(target_os = "macos")]
fn macos_roundtrip() {
    let ok = Command::new("sh")
        .arg("-lc")
        .arg("printf test | pbcopy && pbpaste")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "test")
        .unwrap_or(false);
    println!("pbcopy/pbpaste: {}", if ok { "ok" } else { "failed" });
    if !ok { println!("hint: try closing clipboard managers that may lock NSPasteboard."); }
}

#[cfg(target_os = "windows")]
fn windows_roundtrip() {
    let ok = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("Set-Clipboard 'test'; (Get-Clipboard)")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "test")
        .unwrap_or(false);
    println!("Get-Clipboard: {}", if ok { "ok" } else { "failed" });
    if !ok { println!("hint: PowerShell Get-Clipboard must be available; try running as your desktop user session."); }
}
