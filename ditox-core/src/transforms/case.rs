//! Case-style transforms.
//!
//! UpperCase, LowerCase, TitleCase, SentenceCase, InvertCase
//! operate on whole strings; CamelCase, PascalCase, SnakeCase,
//! KebabCase tokenise on word boundaries first.
//!
//! Word-boundary detection (Tier 2): a "boundary" is any of:
//! - whitespace
//! - an ASCII punctuation character that's not a hyphen embedded
//!   within a word (so `well-known` is one word, `foo, bar` is two)
//! - a transition from lowercase to uppercase inside a token
//!   (`HTTPRequest` → `HTTP`, `Request`; `httpRequest` → `http`,
//!   `Request`).
//!
//! These rules are documented in each impl's doc comment; tests
//! enumerate the edge cases.

use super::Transform;
use crate::error::Result;

/// `HELLO WORLD`. Locale-agnostic ASCII upper.
pub struct UpperCase;
impl Transform for UpperCase {
    fn id(&self) -> &'static str {
        "upper-case"
    }
    fn name(&self) -> &'static str {
        "UPPER CASE"
    }
    fn description(&self) -> &'static str {
        "Convert all letters to UPPERCASE."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(text.to_uppercase())
    }
}

/// `hello world`. Locale-agnostic ASCII lower; non-ASCII letters are
/// passed through `char::to_lowercase` so European diacritics do the
/// right thing (`Ñ` → `ñ`).
pub struct LowerCase;
impl Transform for LowerCase {
    fn id(&self) -> &'static str {
        "lower-case"
    }
    fn name(&self) -> &'static str {
        "lower case"
    }
    fn description(&self) -> &'static str {
        "Convert all letters to lowercase."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(text.to_lowercase())
    }
}

/// `Hello World`. Capitalises the first letter of every
/// whitespace-separated token; lowercases the rest.
pub struct TitleCase;
impl Transform for TitleCase {
    fn id(&self) -> &'static str {
        "title-case"
    }
    fn name(&self) -> &'static str {
        "Title Case"
    }
    fn description(&self) -> &'static str {
        "Capitalise the first letter of every word."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());
        let mut at_word_start = true;
        for ch in text.chars() {
            if ch.is_whitespace() {
                out.push(ch);
                at_word_start = true;
            } else if at_word_start {
                for u in ch.to_uppercase() {
                    out.push(u);
                }
                at_word_start = false;
            } else {
                for l in ch.to_lowercase() {
                    out.push(l);
                }
            }
        }
        Ok(out)
    }
}

/// `Hello world`. Capitalises the first letter; lowercases everything
/// else.
pub struct SentenceCase;
impl Transform for SentenceCase {
    fn id(&self) -> &'static str {
        "sentence-case"
    }
    fn name(&self) -> &'static str {
        "Sentence case"
    }
    fn description(&self) -> &'static str {
        "Capitalise the first letter only."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());
        let mut first = true;
        for ch in text.chars() {
            if first && !ch.is_whitespace() {
                for u in ch.to_uppercase() {
                    out.push(u);
                }
                first = false;
            } else {
                for l in ch.to_lowercase() {
                    out.push(l);
                }
            }
        }
        Ok(out)
    }
}

/// `hELLO wORLD`. Per-character case swap.
pub struct InvertCase;
impl Transform for InvertCase {
    fn id(&self) -> &'static str {
        "invert-case"
    }
    fn name(&self) -> &'static str {
        "iNVERT cASE"
    }
    fn description(&self) -> &'static str {
        "Swap the case of every letter."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());
        for ch in text.chars() {
            if ch.is_uppercase() {
                for l in ch.to_lowercase() {
                    out.push(l);
                }
            } else if ch.is_lowercase() {
                for u in ch.to_uppercase() {
                    out.push(u);
                }
            } else {
                out.push(ch);
            }
        }
        Ok(out)
    }
}

// ============================================================================
// Tier 2: case-style tokenising transforms
// ============================================================================

/// Tokenise `text` into words. A word boundary is any of:
/// - whitespace,
/// - an ASCII punctuation character (`_`, `-`, `.`, `/`, etc.),
/// - a lowercase-to-uppercase transition (`fooBar` → `foo`, `Bar`),
/// - an uppercase-followed-by-uppercase-then-lowercase boundary so
///   `HTTPRequest` splits as `HTTP`, `Request` (the boundary is
///   between `HTTP` and `Request`).
///
/// Tokens are emitted lowercased; callers re-case them. The
/// boundary-detection logic operates on the **original** case
/// (we keep a `Vec<char>` accumulator so `is_uppercase` /
/// `is_lowercase` queries against the previous char return the
/// right thing — flushing to a lowercase `String` mid-walk would
/// erase the `prev_upper` signal needed for the acronym boundary).
fn tokenise_words(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current: Vec<char> = Vec::new();

    let flush = |buf: &mut Vec<char>, out: &mut Vec<String>| {
        if !buf.is_empty() {
            let lower: String = buf.iter().flat_map(|c| c.to_lowercase()).collect();
            out.push(lower);
            buf.clear();
        }
    };

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        let ch = chars[i];

        // Whitespace + ASCII punctuation are word boundaries.
        if ch.is_whitespace() || (ch.is_ascii_punctuation() && ch != '\'') {
            flush(&mut current, &mut tokens);
            i += 1;
            continue;
        }

        // Inside a word: detect lowercase→uppercase or
        // uppercase-acronym→camel-followed-by-lowercase boundaries.
        if ch.is_uppercase() {
            // Lookahead: is the next char lowercase?
            let next_lower = i + 1 < n && chars[i + 1].is_lowercase();

            // Inspect the original case of the most-recent char in
            // `current` (which we kept un-lowered for exactly this
            // reason).
            let prev = current.last().copied();
            let prev_lower = prev.is_some_and(|c| c.is_lowercase());
            let prev_upper = prev.is_some_and(|c| c.is_uppercase());

            // `fooBar` → close `foo`, start `B`.
            // `HTTPRequest` (we're at `R`, prev=`P` upper, next=`e`)
            // → close `HTTP`, start `R`.
            if !current.is_empty() && (prev_lower || (prev_upper && next_lower)) {
                flush(&mut current, &mut tokens);
            }
        }

        current.push(ch);
        i += 1;
    }

    flush(&mut current, &mut tokens);
    tokens
}

/// `helloWorld`. First word lowercase; subsequent words
/// `Capitalised`; concatenated.
pub struct CamelCase;
impl Transform for CamelCase {
    fn id(&self) -> &'static str {
        "camel-case"
    }
    fn name(&self) -> &'static str {
        "camelCase"
    }
    fn description(&self) -> &'static str {
        "Tokens joined; first lowercase, subsequent Capitalised."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let tokens = tokenise_words(text);
        let mut out = String::new();
        for (i, tok) in tokens.iter().enumerate() {
            if i == 0 {
                out.push_str(tok);
            } else {
                let mut chars = tok.chars();
                if let Some(first) = chars.next() {
                    for u in first.to_uppercase() {
                        out.push(u);
                    }
                    out.push_str(chars.as_str());
                }
            }
        }
        Ok(out)
    }
}

/// `HelloWorld`. Every word `Capitalised`; concatenated.
pub struct PascalCase;
impl Transform for PascalCase {
    fn id(&self) -> &'static str {
        "pascal-case"
    }
    fn name(&self) -> &'static str {
        "PascalCase"
    }
    fn description(&self) -> &'static str {
        "Tokens joined, every word Capitalised."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let tokens = tokenise_words(text);
        let mut out = String::new();
        for tok in tokens {
            let mut chars = tok.chars();
            if let Some(first) = chars.next() {
                for u in first.to_uppercase() {
                    out.push(u);
                }
                out.push_str(chars.as_str());
            }
        }
        Ok(out)
    }
}

/// `hello_world`. Lowercase tokens joined with `_`.
pub struct SnakeCase;
impl Transform for SnakeCase {
    fn id(&self) -> &'static str {
        "snake-case"
    }
    fn name(&self) -> &'static str {
        "snake_case"
    }
    fn description(&self) -> &'static str {
        "Lowercase tokens joined with underscores."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(tokenise_words(text).join("_"))
    }
}

/// `hello-world`. Lowercase tokens joined with `-`.
pub struct KebabCase;
impl Transform for KebabCase {
    fn id(&self) -> &'static str {
        "kebab-case"
    }
    fn name(&self) -> &'static str {
        "kebab-case"
    }
    fn description(&self) -> &'static str {
        "Lowercase tokens joined with hyphens."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(tokenise_words(text).join("-"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(t: &dyn Transform, input: &str) -> String {
        t.apply_text(input).expect("infallible transform")
    }

    // -- UpperCase / LowerCase / SentenceCase / InvertCase --

    #[test]
    fn upper_case_ascii() {
        assert_eq!(ok(&UpperCase, "hello world"), "HELLO WORLD");
    }

    #[test]
    fn upper_case_unicode() {
        assert_eq!(ok(&UpperCase, "café"), "CAFÉ");
        assert_eq!(ok(&UpperCase, "ñoño"), "ÑOÑO");
    }

    #[test]
    fn lower_case_ascii() {
        assert_eq!(ok(&LowerCase, "Hello World"), "hello world");
    }

    #[test]
    fn lower_case_unicode() {
        assert_eq!(ok(&LowerCase, "ÑOÑO"), "ñoño");
    }

    #[test]
    fn sentence_case() {
        assert_eq!(ok(&SentenceCase, "hello world"), "Hello world");
        assert_eq!(ok(&SentenceCase, "HELLO WORLD"), "Hello world");
        // Leading whitespace is preserved; first non-whitespace char
        // is the one capitalised.
        assert_eq!(ok(&SentenceCase, "  HELLO WORLD"), "  Hello world");
    }

    #[test]
    fn invert_case() {
        assert_eq!(ok(&InvertCase, "Hello World"), "hELLO wORLD");
        assert_eq!(ok(&InvertCase, "abc123XYZ"), "ABC123xyz");
    }

    // -- TitleCase --

    #[test]
    fn title_case_basic() {
        assert_eq!(ok(&TitleCase, "hello world"), "Hello World");
        assert_eq!(ok(&TitleCase, "the quick brown fox"), "The Quick Brown Fox");
    }

    #[test]
    fn title_case_already_titled() {
        assert_eq!(ok(&TitleCase, "Hello World"), "Hello World");
    }

    #[test]
    fn title_case_lowercases_rest_of_word() {
        assert_eq!(ok(&TitleCase, "HELLO WORLD"), "Hello World");
    }

    #[test]
    fn title_case_preserves_internal_whitespace() {
        assert_eq!(ok(&TitleCase, "hello   world"), "Hello   World");
        assert_eq!(ok(&TitleCase, "\tfoo\nbar\n"), "\tFoo\nBar\n");
    }

    // -- tokenise_words --

    #[test]
    fn tokenise_simple_space() {
        assert_eq!(tokenise_words("hello world"), vec!["hello", "world"]);
    }

    #[test]
    fn tokenise_underscore_separated() {
        assert_eq!(tokenise_words("hello_world"), vec!["hello", "world"]);
    }

    #[test]
    fn tokenise_camel_boundary() {
        assert_eq!(tokenise_words("helloWorld"), vec!["hello", "world"]);
    }

    #[test]
    fn tokenise_pascal_boundary() {
        assert_eq!(tokenise_words("HelloWorld"), vec!["hello", "world"]);
    }

    #[test]
    fn tokenise_acronym_boundary() {
        // HTTPRequest → http, request. The boundary is between the
        // last uppercase of the acronym and the first lowercase of
        // the next word.
        assert_eq!(tokenise_words("HTTPRequest"), vec!["http", "request"]);
        assert_eq!(
            tokenise_words("parseHTTPRequest"),
            vec!["parse", "http", "request"]
        );
    }

    #[test]
    fn tokenise_punctuation() {
        assert_eq!(tokenise_words("foo, bar; baz"), vec!["foo", "bar", "baz"]);
        assert_eq!(tokenise_words("foo.bar/baz"), vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn tokenise_apostrophe_kept_inside_word() {
        // Apostrophes are not split — `don't` → one token.
        assert_eq!(tokenise_words("don't stop"), vec!["don't", "stop"]);
    }

    #[test]
    fn tokenise_empty_input() {
        assert_eq!(tokenise_words(""), Vec::<String>::new());
    }

    #[test]
    fn tokenise_only_separators() {
        assert_eq!(tokenise_words("   ___---"), Vec::<String>::new());
    }

    // -- CamelCase / PascalCase / SnakeCase / KebabCase --

    #[test]
    fn camel_case() {
        assert_eq!(ok(&CamelCase, "hello world"), "helloWorld");
        assert_eq!(ok(&CamelCase, "Hello World"), "helloWorld");
        assert_eq!(ok(&CamelCase, "hello_world_foo"), "helloWorldFoo");
        assert_eq!(ok(&CamelCase, "HTTPRequest"), "httpRequest");
    }

    #[test]
    fn pascal_case() {
        assert_eq!(ok(&PascalCase, "hello world"), "HelloWorld");
        assert_eq!(ok(&PascalCase, "hello_world_foo"), "HelloWorldFoo");
        assert_eq!(ok(&PascalCase, "HTTPRequest"), "HttpRequest");
    }

    #[test]
    fn snake_case() {
        assert_eq!(ok(&SnakeCase, "hello world"), "hello_world");
        assert_eq!(ok(&SnakeCase, "HelloWorld"), "hello_world");
        assert_eq!(ok(&SnakeCase, "HTTPRequest"), "http_request");
        assert_eq!(ok(&SnakeCase, "hello-world"), "hello_world");
    }

    #[test]
    fn kebab_case() {
        assert_eq!(ok(&KebabCase, "hello world"), "hello-world");
        assert_eq!(ok(&KebabCase, "HelloWorld"), "hello-world");
        assert_eq!(ok(&KebabCase, "HTTPRequest"), "http-request");
        assert_eq!(ok(&KebabCase, "hello_world"), "hello-world");
    }

    #[test]
    fn case_styles_on_empty_input() {
        for t in [
            &CamelCase as &dyn Transform,
            &PascalCase,
            &SnakeCase,
            &KebabCase,
        ] {
            assert_eq!(ok(t, ""), "", "{} on empty", t.id());
        }
    }
}
