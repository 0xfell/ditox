//! Search-mode prefix parser (Phase 3 sub-task 3.6).
//!
//! Power-user search syntax: a leading slash + single letter selects
//! which corpus the query runs against.
//!
//! ```text
//!   "hello"     → Default — broad search across all formats + notes
//!   "/p hello"  → Plain   — text/plain;charset=utf-8 only
//!   "/h hello"  → Html    — text/html only
//!   "/r hello"  → Rtf     — text/rtf only
//!   "/q hello"  → Notes   — entry notes only
//!   "/f hello"  → FullText — explicit unrestricted (alias of Default
//!                            today; reserved as a future-extension
//!                            point if Default is ever narrowed)
//! ```
//!
//! Prefix matching is **strict**: a leading `/` followed by a
//! recognised single-letter scope code, then a single ASCII space,
//! then the rest of the query. Any deviation (`"/p"` alone, no space;
//! `"/x foo"`, unknown scope) parses back as `Default` with the
//! literal input as the query — fail-soft so unfortunate clip
//! contents that happen to start with `/` aren't silently mangled
//! into format-restricted searches.
//!
//! ## Routing
//!
//! Callers route the parsed query to the appropriate
//! [`crate::db::Database`] method:
//!
//! - `Default`/`FullText` → `search_entries` /
//!   `search_entries_filtered` (tab- and collection-aware).
//! - `Plain` → `search_entries_in_format(query,
//!   "text/plain;charset=utf-8", limit)`.
//! - `Html`  → `search_entries_in_format(query, "text/html", limit)`.
//! - `Rtf`   → `search_entries_in_format(query, "text/rtf", limit)`.
//! - `Notes` → `search_notes_only(query, limit)`.
//!
//! The format-restricted scopes intentionally **do not honour** the
//! tab/collection filters today — they're power-user modes whose
//! intent is "give me every clip whose HTML/RTF/notes contain this".
//! Phase 3 polish or Phase 4 may revisit if the combination proves
//! useful.

use std::fmt;

/// Search corpus selector. The default corresponds to ditox's
/// pre-3.6 search behaviour — a UNION of `format_content_fts` (any
/// format) and `entries_fts` (notes + legacy single-format
/// content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SearchScope {
    /// Broad search across every captured format plus notes.
    /// Matches the legacy single-corpus FTS5 query.
    #[default]
    Default,
    /// Restrict to `text/plain;charset=utf-8` only.
    Plain,
    /// Restrict to `text/html`.
    Html,
    /// Restrict to `text/rtf`.
    Rtf,
    /// Restrict to entry notes (the `notes` column on `entries`).
    Notes,
    /// Explicit "search everything" — alias of [`Default`](Self::Default)
    /// today. Reserved as a future-extension point if `Default`
    /// is ever narrowed to text+notes only.
    FullText,
}

impl SearchScope {
    /// Single-letter prefix code (`'p'`, `'h'`, `'r'`, `'q'`, `'f'`).
    /// `Default` returns `None` because it has no prefix.
    pub fn prefix_char(self) -> Option<char> {
        match self {
            Self::Default => None,
            Self::Plain => Some('p'),
            Self::Html => Some('h'),
            Self::Rtf => Some('r'),
            Self::Notes => Some('q'),
            Self::FullText => Some('f'),
        }
    }

    /// Resolve a single-letter prefix code (case-insensitive) to a
    /// scope. Returns `None` for an unknown letter.
    pub fn from_prefix_char(c: char) -> Option<Self> {
        Some(match c.to_ascii_lowercase() {
            'p' => Self::Plain,
            'h' => Self::Html,
            'r' => Self::Rtf,
            'q' => Self::Notes,
            'f' => Self::FullText,
            _ => return None,
        })
    }

    /// Canonical format name for the format-restricted scopes.
    /// `Default`/`FullText`/`Notes` return `None` because they
    /// don't map to a single format.
    pub fn format_name(self) -> Option<&'static str> {
        match self {
            Self::Plain => Some("text/plain;charset=utf-8"),
            Self::Html => Some("text/html"),
            Self::Rtf => Some("text/rtf"),
            Self::Default | Self::FullText | Self::Notes => None,
        }
    }
}

impl fmt::Display for SearchScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Default => "default",
            Self::Plain => "plain",
            Self::Html => "html",
            Self::Rtf => "rtf",
            Self::Notes => "notes",
            Self::FullText => "full-text",
        };
        f.write_str(s)
    }
}

/// Result of running [`parse`] on a search-bar input. The `query`
/// has had any recognised prefix stripped; callers should not
/// re-strip it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    pub scope: SearchScope,
    pub query: String,
}

impl ParsedQuery {
    /// Construct a `Default`-scoped query directly. Useful when a
    /// caller already knows there's no prefix to parse.
    pub fn default_for(query: impl Into<String>) -> Self {
        Self {
            scope: SearchScope::Default,
            query: query.into(),
        }
    }
}

/// Run the parsed query against the database, routing to the
/// appropriate FTS5 method based on the scope.
///
/// `filter` and `collection_id` are honoured **only** for the
/// `Default` / `FullText` scopes (which use
/// [`crate::db::Database::search_entries_filtered`]). The
/// format-restricted scopes (`Plain` / `Html` / `Rtf`) and `Notes`
/// search the entire history without tab/collection scoping —
/// they're power-user modes whose intent is "give me every clip
/// whose <format> contains this regardless of where I was".
///
/// Pass `filter = "all"` and `collection_id = None` from contexts
/// that don't have tab semantics (e.g. ditox-tui's `App`, which
/// currently has no tab filter on search).
pub fn dispatch(
    db: &crate::db::Database,
    parsed: &ParsedQuery,
    limit: usize,
    filter: &str,
    collection_id: Option<&str>,
) -> crate::error::Result<Vec<crate::entry::Entry>> {
    match parsed.scope {
        SearchScope::Default | SearchScope::FullText => {
            db.search_entries_filtered(&parsed.query, limit, filter, collection_id)
        }
        SearchScope::Plain => {
            db.search_entries_in_format(&parsed.query, "text/plain;charset=utf-8", limit)
        }
        SearchScope::Html => db.search_entries_in_format(&parsed.query, "text/html", limit),
        SearchScope::Rtf => db.search_entries_in_format(&parsed.query, "text/rtf", limit),
        SearchScope::Notes => db.search_notes_only(&parsed.query, limit),
    }
}

/// Parse a search-bar input. The grammar is:
///
/// ```text
/// input := PREFIXED | LITERAL
/// PREFIXED := '/' SCOPE_LETTER ' ' BODY
/// LITERAL  := <any string>
/// SCOPE_LETTER := 'p' | 'h' | 'r' | 'q' | 'f'  (case-insensitive)
/// BODY := <any string, leading whitespace already consumed>
/// ```
///
/// Anything not matching `PREFIXED` (including `"/p"` alone, `"/x foo"`,
/// `"hello"`) falls through to `Default { query: input }`.
pub fn parse(input: &str) -> ParsedQuery {
    let bytes = input.as_bytes();

    // Need at least 3 bytes: '/', letter, space.
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b' ' {
        if let Some(scope) = SearchScope::from_prefix_char(bytes[1] as char) {
            // Consume the trailing space and any further leading
            // whitespace from the body — `"/p   hello"` → query
            // `"hello"`. Trailing whitespace is preserved (FTS5
            // tokenisation drops it anyway).
            let body = input[3..].trim_start();
            return ParsedQuery {
                scope,
                query: body.to_string(),
            };
        }
    }

    ParsedQuery::default_for(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- prefix-char round trip --

    #[test]
    fn scope_round_trips_through_prefix_char() {
        let scopes = [
            SearchScope::Plain,
            SearchScope::Html,
            SearchScope::Rtf,
            SearchScope::Notes,
            SearchScope::FullText,
        ];
        for s in scopes {
            let c = s.prefix_char().expect("non-default scopes have a prefix");
            assert_eq!(SearchScope::from_prefix_char(c), Some(s));
        }
    }

    #[test]
    fn default_has_no_prefix_char() {
        assert_eq!(SearchScope::Default.prefix_char(), None);
    }

    #[test]
    fn unknown_prefix_char_returns_none() {
        assert_eq!(SearchScope::from_prefix_char('x'), None);
        assert_eq!(SearchScope::from_prefix_char('a'), None);
        assert_eq!(SearchScope::from_prefix_char('/'), None);
    }

    #[test]
    fn from_prefix_char_is_case_insensitive() {
        assert_eq!(SearchScope::from_prefix_char('P'), Some(SearchScope::Plain));
        assert_eq!(SearchScope::from_prefix_char('H'), Some(SearchScope::Html));
        assert_eq!(SearchScope::from_prefix_char('Q'), Some(SearchScope::Notes));
    }

    #[test]
    fn format_name_round_trip_for_format_scopes() {
        assert_eq!(
            SearchScope::Plain.format_name(),
            Some("text/plain;charset=utf-8")
        );
        assert_eq!(SearchScope::Html.format_name(), Some("text/html"));
        assert_eq!(SearchScope::Rtf.format_name(), Some("text/rtf"));
    }

    #[test]
    fn format_name_is_none_for_corpus_scopes() {
        assert_eq!(SearchScope::Default.format_name(), None);
        assert_eq!(SearchScope::FullText.format_name(), None);
        assert_eq!(SearchScope::Notes.format_name(), None);
    }

    // -- parse happy paths --

    #[test]
    fn empty_input_parses_as_default_empty_query() {
        let p = parse("");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "");
    }

    #[test]
    fn no_prefix_parses_as_default() {
        let p = parse("hello world");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "hello world");
    }

    #[test]
    fn slash_p_space_parses_as_plain() {
        let p = parse("/p hello");
        assert_eq!(p.scope, SearchScope::Plain);
        assert_eq!(p.query, "hello");
    }

    #[test]
    fn slash_h_space_parses_as_html() {
        let p = parse("/h hello");
        assert_eq!(p.scope, SearchScope::Html);
        assert_eq!(p.query, "hello");
    }

    #[test]
    fn slash_r_space_parses_as_rtf() {
        let p = parse("/r hello");
        assert_eq!(p.scope, SearchScope::Rtf);
        assert_eq!(p.query, "hello");
    }

    #[test]
    fn slash_q_space_parses_as_notes() {
        let p = parse("/q hello");
        assert_eq!(p.scope, SearchScope::Notes);
        assert_eq!(p.query, "hello");
    }

    #[test]
    fn slash_f_space_parses_as_fulltext() {
        let p = parse("/f hello");
        assert_eq!(p.scope, SearchScope::FullText);
        assert_eq!(p.query, "hello");
    }

    #[test]
    fn prefix_is_case_insensitive() {
        assert_eq!(parse("/P hello").scope, SearchScope::Plain);
        assert_eq!(parse("/H hello").scope, SearchScope::Html);
        assert_eq!(parse("/Q hello").scope, SearchScope::Notes);
    }

    #[test]
    fn extra_whitespace_after_prefix_is_collapsed() {
        let p = parse("/p    hello");
        assert_eq!(p.scope, SearchScope::Plain);
        assert_eq!(p.query, "hello");
    }

    #[test]
    fn body_can_contain_spaces_and_punctuation() {
        let p = parse("/h <div>foo</div>");
        assert_eq!(p.scope, SearchScope::Html);
        assert_eq!(p.query, "<div>foo</div>");
    }

    // -- parse fail-soft paths (treat as literal Default) --

    #[test]
    fn slash_p_without_space_falls_through_to_default() {
        // `/pfoo` is NOT a prefixed query — no separator.
        let p = parse("/pfoo");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "/pfoo");
    }

    #[test]
    fn slash_p_alone_falls_through_to_default() {
        // `/p` is too short for the prefix grammar.
        let p = parse("/p");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "/p");
    }

    #[test]
    fn slash_p_space_only_parses_with_empty_body() {
        // `/p ` is a valid prefix with an empty query body. FTS5 will
        // return zero matches, which is the right outcome.
        let p = parse("/p ");
        assert_eq!(p.scope, SearchScope::Plain);
        assert_eq!(p.query, "");
    }

    #[test]
    fn unknown_prefix_letter_falls_through() {
        let p = parse("/x hello");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "/x hello");
    }

    #[test]
    fn slash_followed_by_digit_falls_through() {
        let p = parse("/3 hello");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "/3 hello");
    }

    #[test]
    fn double_slash_falls_through() {
        // `//foo` could conceivably be an escape for a literal slash;
        // we treat it as Default. Documented behaviour.
        let p = parse("//foo");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "//foo");
    }

    #[test]
    fn leading_whitespace_disables_prefix_recognition() {
        // " /p hello" — leading space means it's a literal.
        let p = parse(" /p hello");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, " /p hello");
    }

    // -- ParsedQuery helpers --

    #[test]
    fn parsed_query_default_for_constructs_default_scope() {
        let p = ParsedQuery::default_for("hi");
        assert_eq!(p.scope, SearchScope::Default);
        assert_eq!(p.query, "hi");
    }

    #[test]
    fn display_returns_human_readable_scope() {
        assert_eq!(SearchScope::Default.to_string(), "default");
        assert_eq!(SearchScope::Plain.to_string(), "plain");
        assert_eq!(SearchScope::Html.to_string(), "html");
        assert_eq!(SearchScope::Rtf.to_string(), "rtf");
        assert_eq!(SearchScope::Notes.to_string(), "notes");
        assert_eq!(SearchScope::FullText.to_string(), "full-text");
    }
}
