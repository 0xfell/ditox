//! Special-paste transforms (Phase 3 sub-task 3.1).
//!
//! A library of text-level transforms applied between "select a clip"
//! and "write to clipboard". The user picks a transform from a TUI/GUI
//! menu (or invokes `ditox transform` from the CLI); the transform
//! consumes the entry's text content and produces the bytes that
//! actually land on the OS clipboard. Original entries are
//! **never** mutated — transforms produce new clipboard payloads
//! for the current copy operation only.
//!
//! ## Trait
//!
//! ```ignore
//! pub trait Transform: Send + Sync {
//!     fn id(&self) -> &'static str;        // CLI / config name
//!     fn name(&self) -> &'static str;      // user-visible label
//!     fn description(&self) -> &'static str;
//!     fn apply_text(&self, text: &str) -> Result<String>;
//! }
//! ```
//!
//! Image-format transforms (ImagesHorizontal / ImagesVertical from
//! the spec) are intentionally **not** in this trait — they take
//! multiple entries as input rather than a single text string and
//! warrant their own trait when multi-entry-selection UX lands in
//! Phase 4. The `Transform` trait covers all 21 text transforms.
//!
//! ## Registry
//!
//! [`registry`] returns a `&'static [&'static dyn Transform]` of every
//! built-in transform. The CLI's `ditox transform list` enumerates
//! it; lookup by id is via [`get`].

use crate::error::Result;

pub mod case;
pub mod meta;
pub mod string;
pub mod whitespace;

/// A reversible-or-not text transformation.
///
/// Implementations are all `&self` and zero-state — every transform
/// is a free function pretending to be a struct so callers can iterate
/// the registry generically. Stateful or parameterised transforms
/// keep their config in their `Self`.
pub trait Transform: Send + Sync {
    /// Stable kebab-case identifier. Used by CLI args, config files,
    /// and menu IDs. Must be unique within the registry.
    fn id(&self) -> &'static str;

    /// Short user-visible label shown in TUI/GUI menus.
    fn name(&self) -> &'static str;

    /// One-line description for help text and tooltips.
    fn description(&self) -> &'static str;

    /// Run the transform. Returns the transformed text or an error.
    /// Most transforms are infallible; the `Result` exists for the
    /// few (Slugify on certain unicode inputs, datetime formatting
    /// with custom formats) that can fail.
    fn apply_text(&self, text: &str) -> Result<String>;
}

/// All built-in transforms in one slice. Order is the order they
/// appear in TUI/GUI menus.
pub fn registry() -> &'static [&'static dyn Transform] {
    &[
        &string::PlainTextOnly,
        &case::UpperCase,
        &case::LowerCase,
        &case::TitleCase,
        &case::SentenceCase,
        &case::InvertCase,
        &case::CamelCase,
        &case::PascalCase,
        &case::SnakeCase,
        &case::KebabCase,
        &string::Slugify,
        &whitespace::RemoveLineFeeds,
        &whitespace::AddLineFeed,
        &whitespace::TrimWhitespace,
        &whitespace::CollapseWhitespace,
        &meta::PrependDateTime,
        &meta::AppendDateTime,
        &meta::InsertGuid,
        &string::PosixifyPaths,
        &string::AsciiOnly,
        &string::Typoglycemia,
    ]
}

/// Look up a transform by id. ASCII-case-insensitive.
pub fn get(id: &str) -> Option<&'static dyn Transform> {
    let lower = id.to_ascii_lowercase();
    registry().iter().find(|t| t.id() == lower).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique() {
        let ids: Vec<&str> = registry().iter().map(|t| t.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            ids.len(),
            sorted.len(),
            "duplicate transform id in registry: {:?}",
            ids
        );
    }

    #[test]
    fn registry_ids_are_kebab_case() {
        for t in registry() {
            for ch in t.id().chars() {
                assert!(
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-',
                    "transform id {:?} has non-kebab char {:?}",
                    t.id(),
                    ch
                );
            }
        }
    }

    #[test]
    fn registry_has_at_least_twenty_transforms() {
        // The spec calls for 22; we ship 21 (image transforms
        // deferred). If the count drops, something was accidentally
        // removed.
        assert!(
            registry().len() >= 20,
            "registry shrank: {} transforms",
            registry().len()
        );
    }

    #[test]
    fn get_resolves_canonical_id() {
        let t = get("upper-case").expect("upper-case must resolve");
        assert_eq!(t.id(), "upper-case");
    }

    #[test]
    fn get_is_case_insensitive() {
        assert!(get("UPPER-CASE").is_some());
        assert!(get("Upper-Case").is_some());
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        assert!(get("not-a-real-transform").is_none());
        assert!(get("").is_none());
    }
}
