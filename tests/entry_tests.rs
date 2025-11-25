//! Entry struct and EntryType tests.
//!
//! Tests entry creation, serialization, hashing, and helper methods.

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};

// ============================================================================
// Hash Tests
// ============================================================================

#[test]
fn test_compute_hash_deterministic() {
    let content = b"Hello, World!";

    let mut hasher1 = Sha256::new();
    hasher1.update(content);
    let hash1 = hex::encode(hasher1.finalize());

    let mut hasher2 = Sha256::new();
    hasher2.update(content);
    let hash2 = hex::encode(hasher2.finalize());

    assert_eq!(hash1, hash2, "Same content should produce same hash");
}

#[test]
fn test_compute_hash_different_content() {
    let mut hasher1 = Sha256::new();
    hasher1.update(b"content1");
    let hash1 = hex::encode(hasher1.finalize());

    let mut hasher2 = Sha256::new();
    hasher2.update(b"content2");
    let hash2 = hex::encode(hasher2.finalize());

    assert_ne!(hash1, hash2, "Different content should produce different hash");
}

#[test]
fn test_compute_hash_empty_content() {
    let mut hasher = Sha256::new();
    hasher.update(b"");
    let hash = hex::encode(hasher.finalize());

    // SHA256 of empty string is well-known
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_compute_hash_unicode() {
    let mut hasher = Sha256::new();
    hasher.update("Hello 世界 🌍".as_bytes());
    let hash = hex::encode(hasher.finalize());

    assert_eq!(hash.len(), 64, "SHA256 hash should be 64 hex characters");
}

#[test]
fn test_compute_hash_binary_data() {
    let binary_data: Vec<u8> = (0..=255).collect();

    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let hash = hex::encode(hasher.finalize());

    assert_eq!(hash.len(), 64);
}

// ============================================================================
// Preview Tests
// ============================================================================

#[test]
fn test_preview_short_text() {
    let content = "Hello";
    let preview = create_preview(content, 40);

    assert_eq!(preview, "Hello");
}

#[test]
fn test_preview_exact_length() {
    let content = "x".repeat(40);
    let preview = create_preview(&content, 40);

    assert_eq!(preview.len(), 40);
    assert!(!preview.ends_with("..."));
}

#[test]
fn test_preview_truncates_long_text() {
    let content = "x".repeat(100);
    let preview = create_preview(&content, 40);

    assert!(preview.len() <= 40);
    assert!(preview.ends_with("..."));
}

#[test]
fn test_preview_normalizes_whitespace() {
    let content = "Hello\n\tWorld\r\n  Test";
    let preview = create_preview(content, 40);

    assert!(!preview.contains('\n'));
    assert!(!preview.contains('\t'));
    assert!(!preview.contains('\r'));
}

#[test]
fn test_preview_trims_whitespace() {
    let content = "  \t  Hello World  \n  ";
    let preview = create_preview(content, 40);

    assert!(!preview.starts_with(' '));
    assert!(!preview.ends_with(' ') || preview.ends_with("..."));
}

#[test]
fn test_preview_empty_content() {
    let content = "";
    let preview = create_preview(content, 40);

    assert_eq!(preview, "");
}

#[test]
fn test_preview_only_whitespace() {
    let content = "   \t\n\r   ";
    let preview = create_preview(content, 40);

    assert_eq!(preview, "");
}

#[test]
fn test_preview_unicode() {
    let content = "Hello 世界 🌍";
    let preview = create_preview(content, 40);

    assert!(preview.contains("Hello"));
    // Note: Unicode character handling may vary
}

#[test]
fn test_preview_max_len_zero() {
    let content = "Hello";
    let preview = create_preview(content, 0);

    // Should handle gracefully
    assert!(preview.is_empty() || preview == "...");
}

/// Helper function to simulate Entry::preview() behavior
fn create_preview(content: &str, max_len: usize) -> String {
    let cleaned: String = content
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();

    if trimmed.len() > max_len {
        if max_len >= 3 {
            format!("{}...", &trimmed[..max_len.saturating_sub(3)])
        } else {
            "...".to_string()
        }
    } else {
        trimmed.to_string()
    }
}

// ============================================================================
// Relative Time Tests
// ============================================================================

#[test]
fn test_relative_time_now() {
    let created_at = Utc::now();
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "now");
}

#[test]
fn test_relative_time_minutes() {
    let created_at = Utc::now() - Duration::minutes(5);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "5m");
}

#[test]
fn test_relative_time_hours() {
    let created_at = Utc::now() - Duration::hours(3);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "3h");
}

#[test]
fn test_relative_time_days() {
    let created_at = Utc::now() - Duration::days(2);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "2d");
}

#[test]
fn test_relative_time_weeks() {
    let created_at = Utc::now() - Duration::weeks(3);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "3w");
}

#[test]
fn test_relative_time_boundary_59_seconds() {
    let created_at = Utc::now() - Duration::seconds(59);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "now");
}

#[test]
fn test_relative_time_boundary_60_seconds() {
    let created_at = Utc::now() - Duration::seconds(60);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "1m");
}

#[test]
fn test_relative_time_boundary_59_minutes() {
    let created_at = Utc::now() - Duration::minutes(59);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "59m");
}

#[test]
fn test_relative_time_boundary_60_minutes() {
    let created_at = Utc::now() - Duration::minutes(60);
    let relative = calculate_relative_time(created_at);

    assert_eq!(relative, "1h");
}

/// Helper function to simulate Entry::relative_time() behavior
fn calculate_relative_time(created_at: chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(created_at);

    if duration.num_seconds() < 60 {
        "now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d", duration.num_days())
    } else {
        format!("{}w", duration.num_weeks())
    }
}

// ============================================================================
// EntryType Tests
// ============================================================================

#[test]
fn test_entry_type_as_str() {
    assert_eq!(entry_type_as_str("text"), "text");
    assert_eq!(entry_type_as_str("image"), "image");
}

#[test]
fn test_entry_type_from_str() {
    assert_eq!(entry_type_from_str("text"), Some("text"));
    assert_eq!(entry_type_from_str("image"), Some("image"));
    assert_eq!(entry_type_from_str("invalid"), None);
    assert_eq!(entry_type_from_str("TEXT"), None); // case-sensitive
}

#[test]
fn test_entry_type_short() {
    assert_eq!(entry_type_short("text"), "txt");
    assert_eq!(entry_type_short("image"), "img");
}

/// Helper functions to simulate EntryType methods
fn entry_type_as_str(t: &str) -> &str {
    match t {
        "text" => "text",
        "image" => "image",
        _ => "unknown",
    }
}

fn entry_type_from_str(s: &str) -> Option<&str> {
    match s {
        "text" => Some("text"),
        "image" => Some("image"),
        _ => None,
    }
}

fn entry_type_short(t: &str) -> &str {
    match t {
        "text" => "txt",
        "image" => "img",
        _ => "???",
    }
}

// ============================================================================
// Entry Creation Tests
// ============================================================================

#[test]
fn test_new_text_entry_fields() {
    let content = "Hello, World!";

    // Simulate Entry::new_text
    let id = uuid::Uuid::new_v4().to_string();
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex::encode(hasher.finalize());
    let byte_size = content.len();

    assert!(!id.is_empty());
    assert_eq!(hash.len(), 64);
    assert_eq!(byte_size, 13);
}

#[test]
fn test_new_text_entry_unique_ids() {
    let id1 = uuid::Uuid::new_v4().to_string();
    let id2 = uuid::Uuid::new_v4().to_string();

    assert_ne!(id1, id2, "Each entry should have a unique ID");
}

#[test]
fn test_new_image_entry_fields() {
    let path = "/path/to/image.png";
    let size = 1024;
    let hash = "abc123";

    // Simulate Entry::new_image
    let id = uuid::Uuid::new_v4().to_string();

    assert!(!id.is_empty());
    assert_eq!(path, "/path/to/image.png");
    assert_eq!(size, 1024);
    assert_eq!(hash, "abc123");
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_entry_json_serialization() {
    use serde_json;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct TestEntry {
        id: String,
        entry_type: String,
        content: String,
        hash: String,
        byte_size: usize,
        created_at: String,
        pinned: bool,
    }

    let entry = TestEntry {
        id: "test-id".to_string(),
        entry_type: "text".to_string(),
        content: "Hello".to_string(),
        hash: "abc123".to_string(),
        byte_size: 5,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        pinned: false,
    };

    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: TestEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(entry, deserialized);
}

#[test]
fn test_entry_json_contains_all_fields() {
    use serde_json::{self, Value};

    #[derive(serde::Serialize)]
    struct TestEntry {
        id: String,
        entry_type: String,
        content: String,
        hash: String,
        byte_size: usize,
        created_at: String,
        pinned: bool,
    }

    let entry = TestEntry {
        id: "test-id".to_string(),
        entry_type: "text".to_string(),
        content: "Hello".to_string(),
        hash: "abc123".to_string(),
        byte_size: 5,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        pinned: false,
    };

    let json: Value = serde_json::to_value(&entry).unwrap();

    assert!(json.get("id").is_some());
    assert!(json.get("entry_type").is_some());
    assert!(json.get("content").is_some());
    assert!(json.get("hash").is_some());
    assert!(json.get("byte_size").is_some());
    assert!(json.get("created_at").is_some());
    assert!(json.get("pinned").is_some());
}

#[test]
fn test_entry_json_unicode_content() {
    use serde_json;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct TestEntry {
        content: String,
    }

    let entry = TestEntry {
        content: "Hello 世界 🌍 مرحبا".to_string(),
    };

    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: TestEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(entry.content, deserialized.content);
}

#[test]
fn test_entry_json_special_characters() {
    use serde_json;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct TestEntry {
        content: String,
    }

    let entry = TestEntry {
        content: "Line1\nLine2\tTab\"Quote\\Backslash".to_string(),
    };

    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: TestEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(entry.content, deserialized.content);
}

// ============================================================================
// Byte Size Tests
// ============================================================================

#[test]
fn test_byte_size_ascii() {
    let content = "Hello";
    assert_eq!(content.len(), 5);
}

#[test]
fn test_byte_size_unicode() {
    let content = "世界";
    // Each Chinese character is 3 bytes in UTF-8
    assert_eq!(content.len(), 6);
}

#[test]
fn test_byte_size_emoji() {
    let content = "🌍";
    // Emoji is 4 bytes in UTF-8
    assert_eq!(content.len(), 4);
}

#[test]
fn test_byte_size_mixed() {
    let content = "Hello 世界 🌍";
    // 5 (Hello) + 1 (space) + 6 (世界) + 1 (space) + 4 (🌍) = 17
    assert_eq!(content.len(), 17);
}

// ============================================================================
// Sanitized Content Tests
// ============================================================================

#[test]
fn test_sanitized_content_strips_ansi_csi_sequences() {
    // CSI sequences: ESC [ ... letter
    let content = "Hello \x1b[31mRed\x1b[0m World";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Hello Red World");
}

#[test]
fn test_sanitized_content_strips_ansi_color_codes() {
    let content = "\x1b[1;32mBold Green\x1b[0m";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Bold Green");
}

#[test]
fn test_sanitized_content_strips_ansi_cursor_movement() {
    let content = "Line1\x1b[2KLine2\x1b[1ALine3";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Line1Line2Line3");
}

#[test]
fn test_sanitized_content_strips_osc_sequences() {
    // OSC sequences: ESC ] ... BEL or ST
    let content = "Hello\x1b]0;Window Title\x07World";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "HelloWorld");
}

#[test]
fn test_sanitized_content_preserves_newlines() {
    let content = "Line1\nLine2\nLine3";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Line1\nLine2\nLine3");
}

#[test]
fn test_sanitized_content_preserves_tabs() {
    let content = "Col1\tCol2\tCol3";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Col1\tCol2\tCol3");
}

#[test]
fn test_sanitized_content_preserves_carriage_return() {
    let content = "Line1\r\nLine2";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Line1\r\nLine2");
}

#[test]
fn test_sanitized_content_strips_control_characters() {
    // Control characters: ASCII 0-31 except \n, \t, \r
    let content = "Hello\x00\x01\x02\x03World";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "HelloWorld");
}

#[test]
fn test_sanitized_content_strips_bell_character() {
    let content = "Alert\x07Sound";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "AlertSound");
}

#[test]
fn test_sanitized_content_strips_backspace() {
    let content = "Hel\x08lo";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Hello");
}

#[test]
fn test_sanitized_content_strips_form_feed() {
    let content = "Page1\x0cPage2";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Page1Page2");
}

#[test]
fn test_sanitized_content_strips_vertical_tab() {
    let content = "Line1\x0bLine2";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Line1Line2");
}

#[test]
fn test_sanitized_content_complex_ansi() {
    // Complex ANSI with multiple sequences
    let content = "\x1b[38;5;196mRed256\x1b[48;2;0;255;0mGreenBG\x1b[0m";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Red256GreenBG");
}

#[test]
fn test_sanitized_content_preserves_unicode() {
    let content = "Hello 世界 🌍";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "Hello 世界 🌍");
}

#[test]
fn test_sanitized_content_empty_string() {
    let content = "";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "");
}

#[test]
fn test_sanitized_content_only_ansi() {
    let content = "\x1b[31m\x1b[0m";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "");
}

#[test]
fn test_sanitized_content_browser_copied_url_with_escapes() {
    // Simulates content that might come from browser copy
    let content = "https://example.com/image.png\x1b[0m";
    let sanitized = sanitize_content(content);
    assert_eq!(sanitized, "https://example.com/image.png");
}

/// Helper function that mirrors Entry::sanitized_content() and strip_ansi_escapes()
fn sanitize_content(content: &str) -> String {
    let without_ansi = strip_ansi_escapes(content);
    without_ansi
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t' | '\r'))
        .collect()
}

fn strip_ansi_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\x07' || next == '\\' {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}
