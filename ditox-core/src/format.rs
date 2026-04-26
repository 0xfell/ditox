//! Clipboard format identity.
//!
//! `RawFormat::mime` (from `capture.rs`) carries a free-form string
//! that may be a real MIME type, a Wayland mime variant
//! (`text/plain;charset=utf-8`), or a Windows-only format prefixed
//! `win32:` (e.g. `win32:CF_DIB`, `win32:CF_HDROP`).
//!
//! This module provides `FormatId`, a typed view over those strings
//! with cross-platform conversion helpers, plus a registry of well-known
//! MIME and `CF_*` constants used by the capture/persistence/aggregator
//! layers.
//!
//! Stability contract: the canonical strings emitted by
//! `FormatId::canonical()` are the values stored in
//! `entry_formats.format_name` and travel through the schema
//! migration. Changing them is a breaking schema change requiring a
//! fresh migration step.

use std::fmt;

pub mod canonicalise;

/// Typed identifier for a clipboard format.
///
/// `Mime` is the cross-platform path; `Win32` is for formats with no
/// MIME equivalent (e.g. `CF_LOCALE`, `CF_HDROP`'s exact wire format,
/// custom Windows-registered formats discovered via
/// `RegisterClipboardFormatW`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FormatId {
    /// A canonical MIME type (lowercase, optional `;charset=...`).
    Mime(String),
    /// A Windows clipboard format. Standard `CF_*` constants are
    /// stored as e.g. `"CF_DIB"`; custom formats registered via
    /// `RegisterClipboardFormatW` are stored as their registered
    /// string (`"HTML Format"`, `"x-special/...gnome-copied-files"`,
    /// etc.).
    Win32(String),
}

impl FormatId {
    /// Construct from a canonical-format string as stored in the DB.
    /// Returns `None` for the empty string.
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }
        if let Some(rest) = s.strip_prefix("win32:") {
            Some(FormatId::Win32(rest.to_string()))
        } else {
            Some(FormatId::Mime(s.to_string()))
        }
    }

    /// Canonical string used as the persistence key in
    /// `entry_formats.format_name` and as the `RawFormat::mime` value.
    pub fn canonical(&self) -> String {
        match self {
            FormatId::Mime(s) => s.clone(),
            FormatId::Win32(s) => format!("win32:{}", s),
        }
    }

    /// True if this is a text-bearing format that should round-trip
    /// through `String` (UTF-8). Used to decide `entry_formats.storage`
    /// = `'inline'` vs `'blob_file'`.
    pub fn is_text_like(&self) -> bool {
        match self {
            FormatId::Mime(s) => {
                let lower = s.to_ascii_lowercase();
                lower.starts_with("text/")
                    || lower == "application/json"
                    || lower == "application/xml"
                    || lower == "application/xhtml+xml"
                    || lower == well_known::URI_LIST
            }
            FormatId::Win32(s) => matches!(
                s.as_str(),
                "CF_TEXT" | "CF_UNICODETEXT" | "CF_OEMTEXT" | "HTML Format" | "Rich Text Format"
            ),
        }
    }

    /// True if this format carries image bytes that should land in
    /// the content-addressed blob store.
    pub fn is_image_like(&self) -> bool {
        match self {
            FormatId::Mime(s) => s.to_ascii_lowercase().starts_with("image/"),
            FormatId::Win32(s) => matches!(
                s.as_str(),
                "CF_BITMAP" | "CF_DIB" | "CF_DIBV5" | "CF_TIFF" | "PNG"
            ),
        }
    }

    /// Map a Windows standard `CF_*` predefined-format integer to the
    /// canonical `Win32` variant. Returns `None` for codes that don't
    /// correspond to a standard format (custom registered formats are
    /// looked up via `GetClipboardFormatNameW`, not this mapping).
    ///
    /// Numeric values are the constants from `winuser.h`.
    pub fn from_win32_cf(cf: u32) -> Option<Self> {
        let name = match cf {
            1 => "CF_TEXT",
            2 => "CF_BITMAP",
            3 => "CF_METAFILEPICT",
            4 => "CF_SYLK",
            5 => "CF_DIF",
            6 => "CF_TIFF",
            7 => "CF_OEMTEXT",
            8 => "CF_DIB",
            9 => "CF_PALETTE",
            10 => "CF_PENDATA",
            11 => "CF_RIFF",
            12 => "CF_WAVE",
            13 => "CF_UNICODETEXT",
            14 => "CF_ENHMETAFILE",
            15 => "CF_HDROP",
            16 => "CF_LOCALE",
            17 => "CF_DIBV5",
            _ => return None,
        };
        Some(FormatId::Win32(name.to_string()))
    }

    /// Map a Wayland MIME-type advertisement to the canonical
    /// `FormatId`. The Wayland protocol is MIME-based so this is
    /// usually identity, but we lower-case and trim defensive
    /// whitespace and translate a few well-known synonyms.
    pub fn from_wayland_mime(mime: &str) -> Self {
        let trimmed = mime.trim();
        let lower = trimmed.to_ascii_lowercase();
        let canonical = match lower.as_str() {
            // Wayland sometimes advertises plain text without charset.
            // Persisting both as the same key dedupes the format set.
            "text/plain" => "text/plain;charset=utf-8".to_string(),
            "utf8_string" | "string" => "text/plain;charset=utf-8".to_string(),
            // GNOME / X11 file-list quirks.
            "x-special/gnome-copied-files" => well_known::GNOME_COPIED_FILES.to_string(),
            // Pass through; capture.rs uses this MIME for image dispatch.
            other => other.to_string(),
        };
        FormatId::Mime(canonical)
    }

    /// Mirror the `entry_formats.storage` decision.
    pub fn storage(&self) -> Storage {
        if self.is_image_like() {
            Storage::BlobFile
        } else {
            Storage::Inline
        }
    }
}

impl fmt::Display for FormatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical())
    }
}

/// Storage class for a captured format. Mirrors the
/// `entry_formats.storage` CHECK constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storage {
    Inline,
    BlobFile,
}

impl Storage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Storage::Inline => "inline",
            Storage::BlobFile => "blob_file",
        }
    }
}

/// Well-known canonical MIME / format strings used across the
/// codebase. Centralised so a string typo is a compile error rather
/// than a silent dedup failure.
pub mod well_known {
    /// UTF-8 plain text. Always emitted with the explicit charset so
    /// dedup is byte-stable across Wayland clients that omit it.
    pub const TEXT_PLAIN_UTF8: &str = "text/plain;charset=utf-8";

    /// HTML clipboard content. On Windows this is the parsed
    /// fragment from the `HTML Format` envelope; on Linux it's the
    /// raw `text/html` advertisement.
    pub const TEXT_HTML: &str = "text/html";

    /// RTF clipboard content. Windows raw is `Rich Text Format`;
    /// canonicalised to MIME.
    pub const TEXT_RTF: &str = "text/rtf";

    /// File-list selection (URI list per RFC 2483). Wayland and most
    /// Linux file managers use this MIME directly; Windows
    /// `CF_HDROP` is converted to this canonical form.
    pub const URI_LIST: &str = "text/uri-list";

    /// GNOME / Nautilus's pre-RFC-2483 file-copy format. Stored as
    /// its own MIME so we can round-trip back to the same clients.
    pub const GNOME_COPIED_FILES: &str = "x-special/gnome-copied-files";

    /// PNG image. Default canonical image format on both platforms.
    pub const IMAGE_PNG: &str = "image/png";

    /// JPEG image. Stored as `image/jpeg` (not `image/jpg`); matches
    /// IANA registry.
    pub const IMAGE_JPEG: &str = "image/jpeg";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_recognises_win32_prefix() {
        let f = FormatId::parse("win32:CF_DIB").unwrap();
        assert!(matches!(f, FormatId::Win32(ref s) if s == "CF_DIB"));
        let f = FormatId::parse("text/html").unwrap();
        assert!(matches!(f, FormatId::Mime(ref s) if s == "text/html"));
        assert!(FormatId::parse("").is_none());
    }

    #[test]
    fn canonical_round_trips_through_parse() {
        for s in [
            "text/plain;charset=utf-8",
            "image/png",
            "win32:CF_DIB",
            "win32:HTML Format",
            "x-special/gnome-copied-files",
        ] {
            let f = FormatId::parse(s).unwrap();
            assert_eq!(f.canonical(), s, "round-trip failed for {}", s);
        }
    }

    #[test]
    fn is_text_like_classification() {
        assert!(FormatId::parse("text/html").unwrap().is_text_like());
        assert!(FormatId::parse("text/plain;charset=utf-8")
            .unwrap()
            .is_text_like());
        assert!(FormatId::parse("application/json").unwrap().is_text_like());
        assert!(FormatId::parse("text/uri-list").unwrap().is_text_like());
        assert!(!FormatId::parse("image/png").unwrap().is_text_like());
        assert!(!FormatId::parse("application/octet-stream")
            .unwrap()
            .is_text_like());

        assert!(FormatId::parse("win32:CF_TEXT").unwrap().is_text_like());
        assert!(FormatId::parse("win32:HTML Format").unwrap().is_text_like());
        assert!(!FormatId::parse("win32:CF_DIB").unwrap().is_text_like());
    }

    #[test]
    fn is_image_like_classification() {
        assert!(FormatId::parse("image/png").unwrap().is_image_like());
        assert!(FormatId::parse("image/jpeg").unwrap().is_image_like());
        assert!(!FormatId::parse("text/plain;charset=utf-8")
            .unwrap()
            .is_image_like());

        assert!(FormatId::parse("win32:CF_DIB").unwrap().is_image_like());
        assert!(FormatId::parse("win32:CF_BITMAP").unwrap().is_image_like());
        assert!(!FormatId::parse("win32:CF_TEXT").unwrap().is_image_like());
    }

    #[test]
    fn storage_class_follows_image_text_split() {
        assert_eq!(
            FormatId::parse("text/plain;charset=utf-8")
                .unwrap()
                .storage(),
            Storage::Inline
        );
        assert_eq!(
            FormatId::parse("image/png").unwrap().storage(),
            Storage::BlobFile
        );
        assert_eq!(
            FormatId::parse("win32:CF_DIB").unwrap().storage(),
            Storage::BlobFile
        );
    }

    #[test]
    fn from_win32_cf_known_constants() {
        assert_eq!(
            FormatId::from_win32_cf(1),
            Some(FormatId::Win32("CF_TEXT".to_string()))
        );
        assert_eq!(
            FormatId::from_win32_cf(8),
            Some(FormatId::Win32("CF_DIB".to_string()))
        );
        assert_eq!(
            FormatId::from_win32_cf(13),
            Some(FormatId::Win32("CF_UNICODETEXT".to_string()))
        );
        assert_eq!(
            FormatId::from_win32_cf(15),
            Some(FormatId::Win32("CF_HDROP".to_string()))
        );
        // Unknown standard CF code → None (custom formats handled
        // separately via `GetClipboardFormatNameW`).
        assert_eq!(FormatId::from_win32_cf(0xC000), None);
    }

    #[test]
    fn from_wayland_mime_normalises_synonyms() {
        // Plain-text variants collapse to the explicit-charset form.
        assert_eq!(
            FormatId::from_wayland_mime("text/plain").canonical(),
            "text/plain;charset=utf-8"
        );
        assert_eq!(
            FormatId::from_wayland_mime("UTF8_STRING").canonical(),
            "text/plain;charset=utf-8"
        );
        assert_eq!(
            FormatId::from_wayland_mime("STRING").canonical(),
            "text/plain;charset=utf-8"
        );

        // Already-canonical strings round-trip.
        assert_eq!(
            FormatId::from_wayland_mime("image/png").canonical(),
            "image/png"
        );
        assert_eq!(
            FormatId::from_wayland_mime("text/html").canonical(),
            "text/html"
        );

        // Whitespace is trimmed.
        assert_eq!(
            FormatId::from_wayland_mime("  text/plain;charset=utf-8  ").canonical(),
            "text/plain;charset=utf-8"
        );
    }

    #[test]
    fn well_known_constants_are_what_callers_expect() {
        assert_eq!(well_known::TEXT_PLAIN_UTF8, "text/plain;charset=utf-8");
        assert_eq!(well_known::TEXT_HTML, "text/html");
        assert_eq!(well_known::TEXT_RTF, "text/rtf");
        assert_eq!(well_known::IMAGE_PNG, "image/png");
        assert_eq!(well_known::IMAGE_JPEG, "image/jpeg");
        assert_eq!(well_known::URI_LIST, "text/uri-list");
        assert_eq!(
            well_known::GNOME_COPIED_FILES,
            "x-special/gnome-copied-files"
        );
    }
}
