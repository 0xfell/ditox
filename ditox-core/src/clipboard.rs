use crate::error::{DitoxError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Clipboard;

impl Clipboard {
    /// Compute SHA256 hash of content
    pub fn hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(result)
    }

    #[cfg(unix)]
    fn mime_to_extension(mime: &str) -> &'static str {
        match mime {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/bmp" => "bmp",
            _ => "png",
        }
    }

    #[cfg(unix)]
    fn extension_to_mime(ext: &str) -> &'static str {
        match ext {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            _ => "image/png",
        }
    }
}

// ============================================================================
// Linux/Wayland Implementation
// ============================================================================

#[cfg(unix)]
mod platform {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use wl_clipboard_rs::paste::{
        get_contents, ClipboardType, Error as PasteError, MimeType as PasteMimeType, Seat,
    };

    impl Clipboard {
        /// Get current clipboard text content
        pub fn get_text() -> Result<Option<String>> {
            let result = get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                PasteMimeType::Text,
            );

            match result {
                Ok((mut reader, _)) => {
                    let mut content = String::new();
                    std::io::Read::read_to_string(&mut reader, &mut content)?;

                    if content.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(content))
                    }
                }
                Err(PasteError::NoSeats)
                | Err(PasteError::ClipboardEmpty)
                | Err(PasteError::NoMimeType) => Ok(None),
                Err(e) => Err(DitoxError::Clipboard(format!(
                    "Failed to get clipboard: {}",
                    e
                ))),
            }
        }

        /// Get current clipboard image and save to path
        /// Returns the path where the image was saved, size, and hash
        pub fn get_image(save_dir: &Path) -> Result<Option<(String, usize, String)>> {
            // Try common image MIME types in order of preference
            let image_mimes = [
                "image/png",
                "image/jpeg",
                "image/gif",
                "image/webp",
                "image/bmp",
            ];

            for mime in &image_mimes {
                let result = get_contents(
                    ClipboardType::Regular,
                    Seat::Unspecified,
                    PasteMimeType::Specific(mime),
                );

                match result {
                    Ok((mut reader, mime_type)) => {
                        let mut data = Vec::new();
                        std::io::Read::read_to_end(&mut reader, &mut data)?;

                        if data.is_empty() {
                            continue;
                        }

                        let hash = Self::hash(&data);
                        let ext = Self::mime_to_extension(&mime_type);
                        let timestamp = chrono::Utc::now().timestamp();
                        let filename = format!("{}_{}.{}", timestamp, &hash[..8], ext);
                        let path = save_dir.join(&filename);

                        // Create directory if needed
                        std::fs::create_dir_all(save_dir)?;
                        std::fs::write(&path, &data)?;

                        let size = data.len();
                        let path_str = path.to_string_lossy().to_string();

                        return Ok(Some((path_str, size, hash)));
                    }
                    Err(PasteError::NoSeats)
                    | Err(PasteError::ClipboardEmpty)
                    | Err(PasteError::NoMimeType) => {
                        continue;
                    }
                    Err(e) => {
                        return Err(DitoxError::Clipboard(format!(
                            "Failed to get clipboard: {}",
                            e
                        )))
                    }
                }
            }

            Ok(None)
        }

        /// Set clipboard text content
        /// Uses wl-copy CLI which properly forks and daemonizes
        pub fn set_text(content: &str) -> Result<()> {
            let mut child = Command::new("wl-copy")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to spawn wl-copy: {}", e)))?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(content.as_bytes()).map_err(|e| {
                    DitoxError::Clipboard(format!("Failed to write to wl-copy: {}", e))
                })?;
            }

            let status = child
                .wait()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to wait for wl-copy: {}", e)))?;

            if !status.success() {
                return Err(DitoxError::Clipboard(format!(
                    "wl-copy exited with status: {}",
                    status
                )));
            }

            Ok(())
        }

        /// Set clipboard image content from file path
        /// Uses wl-copy CLI which properly forks and daemonizes
        pub fn set_image(path: &str) -> Result<()> {
            // Read the image file
            let data = std::fs::read(path)
                .map_err(|e| DitoxError::Clipboard(format!("Failed to read image file: {}", e)))?;

            // Determine MIME type from extension
            let mime_type = Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .map(Self::extension_to_mime)
                .unwrap_or("image/png");

            // Pipe the image data to wl-copy
            let mut child = Command::new("wl-copy")
                .arg("--type")
                .arg(mime_type)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to spawn wl-copy: {}", e)))?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(&data).map_err(|e| {
                    DitoxError::Clipboard(format!("Failed to write to wl-copy: {}", e))
                })?;
            }

            let status = child
                .wait()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to wait for wl-copy: {}", e)))?;

            if !status.success() {
                return Err(DitoxError::Clipboard(format!(
                    "wl-copy exited with status: {}",
                    status
                )));
            }

            Ok(())
        }
    }
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(windows)]
mod platform {
    use super::*;
    use arboard::Clipboard as ArboardClipboard;

    impl Clipboard {
        /// Get current clipboard text content
        pub fn get_text() -> Result<Option<String>> {
            let mut clipboard = ArboardClipboard::new()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to access clipboard: {}", e)))?;

            match clipboard.get_text() {
                Ok(text) if text.is_empty() => Ok(None),
                Ok(text) => Ok(Some(text)),
                Err(arboard::Error::ContentNotAvailable) => Ok(None),
                Err(e) => Err(DitoxError::Clipboard(format!(
                    "Failed to get clipboard text: {}",
                    e
                ))),
            }
        }

        /// Get current clipboard image and save to path
        /// Returns the path where the image was saved, size, and hash
        pub fn get_image(save_dir: &Path) -> Result<Option<(String, usize, String)>> {
            let mut clipboard = ArboardClipboard::new()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to access clipboard: {}", e)))?;

            match clipboard.get_image() {
                Ok(img_data) => {
                    // Convert to PNG bytes
                    let width = img_data.width;
                    let height = img_data.height;
                    let bytes = img_data.bytes;

                    // Create image buffer and encode to PNG
                    let img_buffer: image::RgbaImage =
                        image::ImageBuffer::from_raw(width as u32, height as u32, bytes.into_owned())
                            .ok_or_else(|| {
                                DitoxError::Clipboard("Failed to create image buffer".to_string())
                            })?;

                    let mut png_data = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut png_data);
                    img_buffer
                        .write_to(&mut cursor, image::ImageFormat::Png)
                        .map_err(|e| {
                            DitoxError::Clipboard(format!("Failed to encode PNG: {}", e))
                        })?;

                    let hash = Self::hash(&png_data);
                    let timestamp = chrono::Utc::now().timestamp();
                    let filename = format!("{}_{}.png", timestamp, &hash[..8]);
                    let path = save_dir.join(&filename);

                    // Create directory if needed
                    std::fs::create_dir_all(save_dir)?;
                    std::fs::write(&path, &png_data)?;

                    let size = png_data.len();
                    let path_str = path.to_string_lossy().to_string();

                    Ok(Some((path_str, size, hash)))
                }
                Err(arboard::Error::ContentNotAvailable) => Ok(None),
                Err(e) => Err(DitoxError::Clipboard(format!(
                    "Failed to get clipboard image: {}",
                    e
                ))),
            }
        }

        /// Set clipboard text content
        pub fn set_text(content: &str) -> Result<()> {
            let mut clipboard = ArboardClipboard::new()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to access clipboard: {}", e)))?;

            clipboard
                .set_text(content)
                .map_err(|e| DitoxError::Clipboard(format!("Failed to set clipboard text: {}", e)))
        }

        /// Set clipboard image content from file path
        pub fn set_image(path: &str) -> Result<()> {
            let mut clipboard = ArboardClipboard::new()
                .map_err(|e| DitoxError::Clipboard(format!("Failed to access clipboard: {}", e)))?;

            // Read and decode the image file
            let img = image::open(path)
                .map_err(|e| DitoxError::Clipboard(format!("Failed to open image: {}", e)))?;

            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            let img_data = arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: rgba.into_raw().into(),
            };

            clipboard
                .set_image(img_data)
                .map_err(|e| DitoxError::Clipboard(format!("Failed to set clipboard image: {}", e)))
        }
    }
}
