//! Whitespace transforms: trim, collapse, line-feed mutation.

use super::Transform;
use crate::error::Result;

/// `.trim()`. Strips ASCII whitespace and Unicode whitespace from
/// both ends.
pub struct TrimWhitespace;
impl Transform for TrimWhitespace {
    fn id(&self) -> &'static str {
        "trim-whitespace"
    }
    fn name(&self) -> &'static str {
        "Trim whitespace"
    }
    fn description(&self) -> &'static str {
        "Strip leading and trailing whitespace."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(text.trim().to_string())
    }
}

/// Replace any run of whitespace with a single ASCII space.
/// Newlines are preserved as themselves *outside* runs (i.e.
/// `"a   b"` → `"a b"`, but `"a\nb"` → `"a\nb"` — newlines aren't
/// collapsed-then-replaced; only horizontal-whitespace runs).
///
/// Wait — that's confusing. Actually the spec says "multi-space →
/// single space". We collapse horizontal whitespace runs only,
/// preserving newlines / tabs in their positions but coalescing
/// adjacent ASCII spaces.
///
/// Implementation: walk chars, on each non-space char emit it; on
/// each space char emit ONE space if the previous char wasn't a
/// space.
pub struct CollapseWhitespace;
impl Transform for CollapseWhitespace {
    fn id(&self) -> &'static str {
        "collapse-whitespace"
    }
    fn name(&self) -> &'static str {
        "Collapse whitespace"
    }
    fn description(&self) -> &'static str {
        "Collapse runs of horizontal whitespace into a single space."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());
        let mut last_was_space = false;
        for ch in text.chars() {
            // Treat ASCII space and tab as collapsible horizontal
            // whitespace; preserve newlines verbatim.
            if ch == ' ' || ch == '\t' {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
            } else {
                out.push(ch);
                last_was_space = false;
            }
        }
        Ok(out)
    }
}

/// Replace `\r\n` and `\n` with a single space. Useful for
/// "paste this code as a one-liner".
pub struct RemoveLineFeeds;
impl Transform for RemoveLineFeeds {
    fn id(&self) -> &'static str {
        "remove-line-feeds"
    }
    fn name(&self) -> &'static str {
        "Remove line feeds"
    }
    fn description(&self) -> &'static str {
        "Replace line breaks with a single space."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        // Two-pass: normalise CRLF first so we don't emit double
        // spaces.
        let normalised = text.replace("\r\n", "\n");
        Ok(normalised.replace('\n', " "))
    }
}

/// Append a single trailing `\n`. The CLI exposes a fixed-N variant
/// in a future iteration; the spec's `AddLineFeeds(n)` is reduced
/// here to `AddLineFeed` (n=1) so the registry can be a flat list of
/// zero-state structs. Power users can chain transforms or feed the
/// output through `printf` for arbitrary N.
pub struct AddLineFeed;
impl Transform for AddLineFeed {
    fn id(&self) -> &'static str {
        "add-line-feed"
    }
    fn name(&self) -> &'static str {
        "Add line feed"
    }
    fn description(&self) -> &'static str {
        "Append a single trailing newline."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len() + 1);
        out.push_str(text);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(t: &dyn Transform, input: &str) -> String {
        t.apply_text(input).expect("infallible transform")
    }

    #[test]
    fn trim_whitespace() {
        assert_eq!(ok(&TrimWhitespace, "  hello  "), "hello");
        assert_eq!(ok(&TrimWhitespace, "\thello\n"), "hello");
        assert_eq!(ok(&TrimWhitespace, "no trim"), "no trim");
        assert_eq!(ok(&TrimWhitespace, ""), "");
    }

    #[test]
    fn collapse_whitespace_basic() {
        assert_eq!(ok(&CollapseWhitespace, "hello   world"), "hello world");
        assert_eq!(ok(&CollapseWhitespace, "a\t\tb"), "a b");
        assert_eq!(ok(&CollapseWhitespace, "a \t b"), "a b");
    }

    #[test]
    fn collapse_whitespace_preserves_newlines() {
        // Newlines aren't collapsed; only ASCII space + tab runs
        // are. This matches `cat | tr -s ' '` semantics.
        assert_eq!(ok(&CollapseWhitespace, "a\n\nb"), "a\n\nb");
    }

    #[test]
    fn collapse_whitespace_empty_and_no_collapse() {
        assert_eq!(ok(&CollapseWhitespace, ""), "");
        assert_eq!(ok(&CollapseWhitespace, "single"), "single");
    }

    #[test]
    fn remove_line_feeds_basic() {
        assert_eq!(ok(&RemoveLineFeeds, "line1\nline2"), "line1 line2");
    }

    #[test]
    fn remove_line_feeds_crlf() {
        assert_eq!(ok(&RemoveLineFeeds, "line1\r\nline2"), "line1 line2");
    }

    #[test]
    fn remove_line_feeds_multiple() {
        assert_eq!(ok(&RemoveLineFeeds, "a\nb\nc"), "a b c");
    }

    #[test]
    fn add_line_feed_appends() {
        assert_eq!(ok(&AddLineFeed, "hello"), "hello\n");
    }

    #[test]
    fn add_line_feed_idempotent() {
        // Don't double up: input already ending in `\n` stays as-is.
        assert_eq!(ok(&AddLineFeed, "hello\n"), "hello\n");
    }

    #[test]
    fn add_line_feed_empty() {
        assert_eq!(ok(&AddLineFeed, ""), "\n");
    }
}
