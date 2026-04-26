//! "Meta" transforms that emit timestamps or identifiers as part of
//! the output: PrependDateTime, AppendDateTime, InsertGuid.

use super::Transform;
use crate::error::Result;

/// `PrependDateTime` and `AppendDateTime` use this format.
/// `chrono`'s strftime-style; the ISO-ish layout matches Ditto's
/// default and most users' "I want a timestamp tag" expectation.
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// `2026-04-26 19:00:00 hello`. Prepends the current local
/// date-time + space to the input.
pub struct PrependDateTime;
impl Transform for PrependDateTime {
    fn id(&self) -> &'static str {
        "prepend-date-time"
    }
    fn name(&self) -> &'static str {
        "Prepend date/time"
    }
    fn description(&self) -> &'static str {
        "Insert YYYY-MM-DD HH:MM:SS prefix."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let now = chrono::Local::now().format(DATETIME_FORMAT).to_string();
        Ok(format!("{} {}", now, text))
    }
}

/// `hello 2026-04-26 19:00:00`. Appends the current local date-time
/// + space.
pub struct AppendDateTime;
impl Transform for AppendDateTime {
    fn id(&self) -> &'static str {
        "append-date-time"
    }
    fn name(&self) -> &'static str {
        "Append date/time"
    }
    fn description(&self) -> &'static str {
        "Append YYYY-MM-DD HH:MM:SS suffix."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let now = chrono::Local::now().format(DATETIME_FORMAT).to_string();
        Ok(format!("{} {}", text, now))
    }
}

/// Append a fresh UUIDv4 (with a leading space). Useful for tagging
/// generated artefacts.
pub struct InsertGuid;
impl Transform for InsertGuid {
    fn id(&self) -> &'static str {
        "insert-guid"
    }
    fn name(&self) -> &'static str {
        "Insert GUID"
    }
    fn description(&self) -> &'static str {
        "Append a fresh UUIDv4."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4();
        if text.is_empty() {
            Ok(id.to_string())
        } else {
            Ok(format!("{} {}", text, id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(t: &dyn Transform, input: &str) -> String {
        t.apply_text(input).expect("infallible transform")
    }

    #[test]
    fn prepend_date_time_inserts_prefix() {
        let out = ok(&PrependDateTime, "hello");
        // Must end with " hello" and start with a 4-digit year.
        assert!(out.ends_with(" hello"), "output: {:?}", out);
        let prefix = &out[..out.len() - 6]; // drop " hello"
        assert_eq!(
            prefix.len(),
            19,
            "datetime prefix wrong length: {:?}",
            prefix
        );
        assert!(prefix.contains('-'));
        assert!(prefix.contains(':'));
    }

    #[test]
    fn append_date_time_inserts_suffix() {
        let out = ok(&AppendDateTime, "hello");
        assert!(out.starts_with("hello "), "output: {:?}", out);
        let suffix = &out[6..];
        assert_eq!(suffix.len(), 19);
        assert!(suffix.contains('-'));
        assert!(suffix.contains(':'));
    }

    #[test]
    fn insert_guid_appends_uuid() {
        let out = ok(&InsertGuid, "hello");
        assert!(out.starts_with("hello "));
        // UUIDv4 length is 36 chars including hyphens.
        let uuid_part = &out[6..];
        assert_eq!(uuid_part.len(), 36);
        assert_eq!(uuid_part.matches('-').count(), 4);
        // Must be parseable.
        let parsed = uuid::Uuid::parse_str(uuid_part);
        assert!(parsed.is_ok(), "appended chunk must parse as UUID");
    }

    #[test]
    fn insert_guid_on_empty_returns_just_uuid() {
        let out = ok(&InsertGuid, "");
        assert_eq!(out.len(), 36);
        assert!(uuid::Uuid::parse_str(&out).is_ok());
    }

    #[test]
    fn insert_guid_two_calls_produce_different_uuids() {
        let a = ok(&InsertGuid, "x");
        let b = ok(&InsertGuid, "x");
        assert_ne!(a, b, "UUIDv4 should be unique per call");
    }
}
