use anyhow::Result;
use std::process::{Command, Stdio};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::SystemClipboard;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use ditox_core::clipboard::Clipboard as _;

#[cfg(target_os = "linux")]
fn try_prog_bytes(prog: &str, args: &[&str], input: &[u8]) -> Result<bool> {
    let mut child = match Command::new(prog)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        let _ = stdin.write_all(input);
    }
    let status = child.wait()?;
    Ok(status.success())
}

#[cfg(target_os = "linux")]
fn try_prog_str(prog: &str, args: &[&str], input: &str) -> Result<bool> {
    try_prog_bytes(prog, args, input.as_bytes())
}

pub fn copy_text(text: &str, force_wl_copy: bool) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        if (force_wl_copy || std::env::var_os("WAYLAND_DISPLAY").is_some())
            && try_prog_str("wl-copy", &[], text)?
        {
            return Ok(());
        }
    }
    // system clipboard fallback
    let cb = SystemClipboard::new();
    if let Err(e1) = cb.set_text(text) {
        // platform-specific utility fallbacks
        #[cfg(target_os = "linux")]
        {
            if try_prog_str("xclip", &["-selection", "clipboard"], text)?
                || try_prog_str("xsel", &["-b"], text)?
            {
                return Ok(());
            }
        }
        #[cfg(target_os = "macos")]
        {
            if Command::new("pbcopy")
                .stdin(Stdio::piped())
                .spawn()
                .and_then(|mut c| {
                    if let Some(mut i) = c.stdin.take() {
                        use std::io::Write as _;
                        let _ = i.write_all(text.as_bytes());
                    }
                    c.wait()
                })
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Ok(());
            }
        }
        #[cfg(target_os = "windows")]
        {
            if Command::new("clip")
                .stdin(Stdio::piped())
                .spawn()
                .and_then(|mut c| {
                    if let Some(mut i) = c.stdin.take() {
                        use std::io::Write as _;
                        let _ = i.write_all(text.as_bytes());
                    }
                    c.wait()
                })
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Ok(());
            }
        }
        return Err(e1);
    }
    Ok(())
}

pub fn copy_image(img: &ditox_core::ImageRgba, force_wl_copy: bool) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use image::ImageEncoder;
        if force_wl_copy || std::env::var_os("WAYLAND_DISPLAY").is_some() {
            let mut buf = Vec::new();
            let enc = image::codecs::png::PngEncoder::new(&mut buf);
            enc.write_image(
                &img.bytes,
                img.width,
                img.height,
                image::ExtendedColorType::Rgba8,
            )?;
            if try_prog_bytes("wl-copy", &["-t", "image/png"], &buf)? {
                return Ok(());
            }
        }
    }
    // system clipboard fallback
    let cb = SystemClipboard::new();
    cb.set_image(img)?;
    Ok(())
}
