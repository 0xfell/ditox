use std::path::Path;

fn main() {
    // Generate .ico from .png if needed
    generate_ico_if_needed();

    // Windows resource embedding
    #[cfg(windows)]
    {
        embed_windows_resources();
    }
}

fn generate_ico_if_needed() {
    let ico_path = Path::new("ditox.ico");
    let png_path = Path::new("../ditox.png");

    // Only regenerate if ICO doesn't exist or PNG is newer
    if ico_path.exists() {
        if let (Ok(ico_meta), Ok(png_meta)) = (ico_path.metadata(), png_path.metadata()) {
            if let (Ok(ico_time), Ok(png_time)) = (ico_meta.modified(), png_meta.modified()) {
                if ico_time >= png_time {
                    return; // ICO is up to date
                }
            }
        }
    }

    println!("cargo:rerun-if-changed=../ditox.png");

    // Load PNG and create ICO with multiple resolutions
    let img = match image::open(png_path) {
        Ok(img) => img,
        Err(e) => {
            println!("cargo:warning=Could not load ditox.png: {}", e);
            return;
        }
    };

    let mut ico_data = Vec::new();

    // ICO file format:
    // - Header (6 bytes)
    // - Directory entries (16 bytes each)
    // - Image data (PNG format embedded)

    let sizes: &[u32] = &[256, 48, 32, 16];
    let num_images = sizes.len() as u16;

    // Write ICO header
    ico_data.extend_from_slice(&[0, 0]); // Reserved
    ico_data.extend_from_slice(&[1, 0]); // Type: 1 = ICO
    ico_data.extend_from_slice(&num_images.to_le_bytes()); // Number of images

    // Calculate offsets - header is 6 bytes, each directory entry is 16 bytes
    let header_size = 6 + (num_images as usize * 16);
    let mut image_data: Vec<Vec<u8>> = Vec::new();

    // Resize images and encode as PNG
    for &size in sizes {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let mut png_data = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_data);
        resized
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("Failed to encode PNG");
        image_data.push(png_data);
    }

    // Write directory entries
    let mut offset = header_size;
    for (i, &size) in sizes.iter().enumerate() {
        let width = if size >= 256 { 0u8 } else { size as u8 };
        let height = if size >= 256 { 0u8 } else { size as u8 };

        ico_data.push(width); // Width (0 = 256)
        ico_data.push(height); // Height (0 = 256)
        ico_data.push(0); // Color palette (0 = no palette)
        ico_data.push(0); // Reserved
        ico_data.extend_from_slice(&[1, 0]); // Color planes
        ico_data.extend_from_slice(&[32, 0]); // Bits per pixel
        ico_data.extend_from_slice(&(image_data[i].len() as u32).to_le_bytes()); // Image size
        ico_data.extend_from_slice(&(offset as u32).to_le_bytes()); // Offset

        offset += image_data[i].len();
    }

    // Write image data
    for data in &image_data {
        ico_data.extend_from_slice(data);
    }

    // Write ICO file to both locations (root for winres, installer for Inno Setup)
    if let Err(e) = std::fs::write(ico_path, &ico_data) {
        println!("cargo:warning=Could not write ditox.ico: {}", e);
    } else {
        println!("cargo:warning=Generated ditox.ico with {} sizes", num_images);
    }

    // Also copy to installer directory
    let installer_ico = Path::new("installer/ditox.ico");
    if let Err(e) = std::fs::write(installer_ico, &ico_data) {
        println!("cargo:warning=Could not write installer/ditox.ico: {}", e);
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("ditox.ico");
    res.set("ProductName", "Ditox Clipboard Manager");
    res.set("FileDescription", "Ditox Clipboard Manager");
    res.set("CompanyName", "Ditox");
    res.set("LegalCopyright", "Copyright (c) 2024 Ditox");

    if let Err(e) = res.compile() {
        println!("cargo:warning=Failed to compile Windows resources: {}", e);
    }
}

#[cfg(not(windows))]
fn embed_windows_resources() {
    // No-op on non-Windows platforms
}
