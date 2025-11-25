//! Clipboard and watcher tests with mocking.
//!
//! Tests clipboard priority logic (image vs text) and watcher behavior.

use std::collections::HashMap;

// ============================================================================
// Mock Clipboard Implementation
// ============================================================================

/// Represents clipboard content with multiple MIME types (like a real clipboard)
#[derive(Default, Clone)]
struct MockClipboard {
    /// Text content (text/plain MIME type)
    text: Option<String>,
    /// Image data keyed by MIME type (image/png, image/jpeg, etc.)
    images: HashMap<String, Vec<u8>>,
}

impl MockClipboard {
    fn new() -> Self {
        Self::default()
    }

    fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    fn with_image(mut self, mime: &str, data: Vec<u8>) -> Self {
        self.images.insert(mime.to_string(), data);
        self
    }

    /// Simulates Clipboard::get_text() behavior
    fn get_text(&self) -> Option<String> {
        self.text.clone()
    }

    /// Simulates Clipboard::get_image() behavior with specific MIME type requests
    fn get_image(&self, requested_mimes: &[&str]) -> Option<(String, Vec<u8>)> {
        for mime in requested_mimes {
            if let Some(data) = self.images.get(*mime) {
                return Some((mime.to_string(), data.clone()));
            }
        }
        None
    }
}

// ============================================================================
// Watcher Priority Logic (extracted for testing)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum CapturedContent {
    Text(String),
    Image { mime: String, size: usize },
    None,
}

/// Simulates the watcher's poll() logic for determining what to capture
fn poll_clipboard(clipboard: &MockClipboard) -> CapturedContent {
    // Image MIME types to check, in order of preference
    let image_mimes = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/bmp"];

    // Try image first (this is the fix we implemented)
    if let Some((mime, data)) = clipboard.get_image(&image_mimes) {
        return CapturedContent::Image {
            mime,
            size: data.len(),
        };
    }

    // Try text if no image
    if let Some(text) = clipboard.get_text() {
        return CapturedContent::Text(text);
    }

    CapturedContent::None
}

// ============================================================================
// Priority Tests
// ============================================================================

#[test]
fn test_image_prioritized_over_text() {
    // Simulates browser "Copy image" which provides both URL and image data
    let clipboard = MockClipboard::new()
        .with_text("https://example.com/image.png")
        .with_image("image/png", vec![0x89, 0x50, 0x4E, 0x47]); // PNG magic bytes

    let captured = poll_clipboard(&clipboard);

    match captured {
        CapturedContent::Image { mime, .. } => {
            assert_eq!(mime, "image/png");
        }
        _ => panic!("Expected image to be captured, got {:?}", captured),
    }
}

#[test]
fn test_text_captured_when_no_image() {
    let clipboard = MockClipboard::new().with_text("Hello, World!");

    let captured = poll_clipboard(&clipboard);

    assert_eq!(captured, CapturedContent::Text("Hello, World!".to_string()));
}

#[test]
fn test_nothing_captured_from_empty_clipboard() {
    let clipboard = MockClipboard::new();

    let captured = poll_clipboard(&clipboard);

    assert_eq!(captured, CapturedContent::None);
}

#[test]
fn test_jpeg_image_captured() {
    let clipboard = MockClipboard::new()
        .with_text("https://example.com/photo.jpg")
        .with_image("image/jpeg", vec![0xFF, 0xD8, 0xFF]); // JPEG magic bytes

    let captured = poll_clipboard(&clipboard);

    match captured {
        CapturedContent::Image { mime, .. } => {
            assert_eq!(mime, "image/jpeg");
        }
        _ => panic!("Expected JPEG image to be captured"),
    }
}

#[test]
fn test_png_preferred_over_jpeg() {
    // When both PNG and JPEG are available, PNG should be preferred (it's first in the list)
    let clipboard = MockClipboard::new()
        .with_image("image/jpeg", vec![0xFF, 0xD8, 0xFF])
        .with_image("image/png", vec![0x89, 0x50, 0x4E, 0x47]);

    let captured = poll_clipboard(&clipboard);

    match captured {
        CapturedContent::Image { mime, .. } => {
            assert_eq!(mime, "image/png");
        }
        _ => panic!("Expected PNG image to be captured"),
    }
}

#[test]
fn test_gif_image_captured() {
    let clipboard = MockClipboard::new()
        .with_image("image/gif", vec![0x47, 0x49, 0x46]); // GIF magic bytes

    let captured = poll_clipboard(&clipboard);

    match captured {
        CapturedContent::Image { mime, .. } => {
            assert_eq!(mime, "image/gif");
        }
        _ => panic!("Expected GIF image to be captured"),
    }
}

#[test]
fn test_webp_image_captured() {
    let clipboard = MockClipboard::new()
        .with_image("image/webp", vec![0x52, 0x49, 0x46, 0x46]); // WEBP magic bytes

    let captured = poll_clipboard(&clipboard);

    match captured {
        CapturedContent::Image { mime, .. } => {
            assert_eq!(mime, "image/webp");
        }
        _ => panic!("Expected WebP image to be captured"),
    }
}

#[test]
fn test_unsupported_image_mime_falls_back_to_text() {
    // If clipboard has an unsupported image MIME type, fall back to text
    let clipboard = MockClipboard::new()
        .with_text("Some text")
        .with_image("image/tiff", vec![0x49, 0x49]); // TIFF (not in our supported list)

    let captured = poll_clipboard(&clipboard);

    assert_eq!(captured, CapturedContent::Text("Some text".to_string()));
}

#[test]
fn test_browser_copy_image_scenario() {
    // Real-world scenario: Firefox "Copy Image" on Amazon product image
    let clipboard = MockClipboard::new()
        .with_text("https://m.media-amazon.com/images/I/419TLPVGf9L._AC_.jpg")
        .with_image("image/jpeg", vec![0xFF, 0xD8, 0xFF, 0xE0]); // JPEG data

    let captured = poll_clipboard(&clipboard);

    match captured {
        CapturedContent::Image { mime, size } => {
            assert_eq!(mime, "image/jpeg");
            assert_eq!(size, 4);
        }
        CapturedContent::Text(t) => {
            panic!("Should have captured image, not text URL: {}", t);
        }
        _ => panic!("Expected image to be captured"),
    }
}

#[test]
fn test_copy_text_only_scenario() {
    // Regular text copy (no image data)
    let clipboard = MockClipboard::new().with_text("sudo apt install something");

    let captured = poll_clipboard(&clipboard);

    assert_eq!(
        captured,
        CapturedContent::Text("sudo apt install something".to_string())
    );
}

#[test]
fn test_copy_image_url_as_text_scenario() {
    // User explicitly copies the URL (right-click -> Copy Link), not the image
    // In this case, only text is available
    let clipboard =
        MockClipboard::new().with_text("https://example.com/images/photo.png");

    let captured = poll_clipboard(&clipboard);

    assert_eq!(
        captured,
        CapturedContent::Text("https://example.com/images/photo.png".to_string())
    );
}

// ============================================================================
// MIME Type Priority Order Tests
// ============================================================================

#[test]
fn test_mime_priority_order() {
    // Verify the exact priority order: png > jpeg > gif > webp > bmp
    let test_cases = vec![
        ("image/png", 0),
        ("image/jpeg", 1),
        ("image/gif", 2),
        ("image/webp", 3),
        ("image/bmp", 4),
    ];

    let image_mimes = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/bmp"];

    for (mime, expected_idx) in test_cases {
        let idx = image_mimes.iter().position(|&m| m == mime);
        assert_eq!(idx, Some(expected_idx), "MIME type {} should be at index {}", mime, expected_idx);
    }
}

// ============================================================================
// Hash Deduplication Tests (simulated)
// ============================================================================

#[derive(Default)]
struct MockDatabase {
    hashes: std::collections::HashSet<String>,
}

impl MockDatabase {
    fn exists_by_hash(&self, hash: &str) -> bool {
        self.hashes.contains(hash)
    }

    fn insert_hash(&mut self, hash: String) {
        self.hashes.insert(hash);
    }
}

fn compute_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[test]
fn test_duplicate_image_not_captured_twice() {
    let image_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
    let hash = compute_hash(&image_data);

    let mut db = MockDatabase::default();

    // First capture
    assert!(!db.exists_by_hash(&hash));
    db.insert_hash(hash.clone());

    // Second capture of same image
    assert!(db.exists_by_hash(&hash));
}

#[test]
fn test_duplicate_text_not_captured_twice() {
    let text = "Hello, World!";
    let hash = compute_hash(text.as_bytes());

    let mut db = MockDatabase::default();

    // First capture
    assert!(!db.exists_by_hash(&hash));
    db.insert_hash(hash.clone());

    // Second capture of same text
    assert!(db.exists_by_hash(&hash));
}

#[test]
fn test_different_content_has_different_hash() {
    let hash1 = compute_hash(b"content1");
    let hash2 = compute_hash(b"content2");

    assert_ne!(hash1, hash2);
}
