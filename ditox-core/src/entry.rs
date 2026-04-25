use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    Text,
    Image,
}

impl EntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntryType::Text => "text",
            EntryType::Image => "image",
        }
    }

    // Named `from_str` for symmetry with `as_str`. Not implementing the
    // `FromStr` trait because our error type is just "unknown variant" —
    // a custom `Result` would add ceremony without value.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(EntryType::Text),
            "image" => Some(EntryType::Image),
            _ => None,
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            EntryType::Text => "txt",
            EntryType::Image => "img",
        }
    }

    /// Returns a single-character icon for the entry type
    pub fn icon(&self) -> &'static str {
        match self {
            EntryType::Text => "T",
            EntryType::Image => "I",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub entry_type: EntryType,
    pub content: String,
    pub hash: String,
    pub byte_size: usize,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub favorite: bool,
    /// Optional user annotation/note for this entry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Optional collection ID for organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    /// For image entries: the file extension ("png", "jpg", …). For text
    /// entries this is `None`. The on-disk path is derived from
    /// `(hash, image_extension)` — see `Database::image_path`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_extension: Option<String>,
}

impl Entry {
    pub fn new_text(content: String) -> Self {
        let hash = Self::compute_hash(content.as_bytes());
        let byte_size = content.len();
        let now = Utc::now();

        Self {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Text,
            content,
            hash,
            byte_size,
            created_at: now,
            last_used: now,
            favorite: false,
            notes: None,
            collection_id: None,
            image_extension: None,
        }
    }

    /// Construct an image entry. `hash` is the SHA-256 of the image bytes and
    /// uniquely identifies the on-disk blob; `extension` is a bare lowercase
    /// extension like "png" or "jpg" (no leading dot). The DB `content`
    /// column stores the bare hash and the path is derived — never hand the
    /// caller a filesystem path here.
    pub fn new_image(hash: String, size: usize, extension: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Image,
            content: hash.clone(),
            hash,
            byte_size: size,
            created_at: now,
            last_used: now,
            favorite: false,
            notes: None,
            collection_id: None,
            image_extension: Some(extension),
        }
    }

    /// For image entries, resolve the absolute on-disk path of the backing
    /// blob. Returns `None` for non-image entries or if the data directory
    /// can't be resolved.
    pub fn image_path(&self) -> Option<std::path::PathBuf> {
        if self.entry_type != EntryType::Image {
            return None;
        }
        let ext = self.image_extension.as_deref().unwrap_or("png");
        crate::db::Database::image_path(&self.hash, ext).ok()
    }

    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn preview(&self, max_len: usize) -> String {
        match self.entry_type {
            EntryType::Text => {
                let cleaned: String = self
                    .content
                    .chars()
                    .filter(|c| !c.is_control() || c.is_whitespace())
                    .map(|c| if c.is_whitespace() { ' ' } else { c })
                    .collect();
                let trimmed = cleaned.trim();
                // Use char count, not byte length, to handle UTF-8 properly
                let char_count = trimmed.chars().count();
                if char_count > max_len {
                    let truncated: String =
                        trimmed.chars().take(max_len.saturating_sub(3)).collect();
                    format!("{}...", truncated)
                } else {
                    trimmed.to_string()
                }
            }
            EntryType::Image => {
                // `content` is the content-addressable hash; show a short,
                // stable label that doesn't leak implementation details.
                let ext = self.image_extension.as_deref().unwrap_or("png");
                let short = self.content.chars().take(8).collect::<String>();
                format!("image-{}.{}", short, ext)
            }
        }
    }

    /// Returns content sanitized for safe TUI display.
    /// Strips ANSI escape sequences and non-printable control characters.
    pub fn sanitized_content(&self) -> String {
        // Strip ANSI escape sequences: ESC [ ... (ending with letter)
        let without_ansi = strip_ansi_escapes(&self.content);

        // Filter out control characters except newline, tab, carriage return
        without_ansi
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t' | '\r'))
            .collect()
    }

    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);

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

    /// Detect and return the content type of this entry
    #[allow(dead_code)]
    pub fn detected_content_type(&self) -> crate::content_type::ContentType {
        match self.entry_type {
            EntryType::Text => crate::content_type::detect(&self.content),
            EntryType::Image => crate::content_type::ContentType::Text, // Images are handled separately
        }
    }

    /// Get a short label for the detected content type
    #[allow(dead_code)]
    pub fn content_type_label(&self) -> &'static str {
        match self.entry_type {
            EntryType::Text => self.detected_content_type().label(),
            EntryType::Image => "img",
        }
    }
}

/// Strips ANSI escape sequences from a string.
fn strip_ansi_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC character - start of escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we hit a letter (end of CSI sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                // OSC sequence (ESC ] ... ST or BEL)
                chars.next(); // consume ']'
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\x07' || next == '\\' {
                        break;
                    }
                }
            }
            // Skip other escape sequences
        } else {
            result.push(c);
        }
    }

    result
}
