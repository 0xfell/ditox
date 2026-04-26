//! String transforms that don't fall cleanly into case or whitespace
//! categories: PlainTextOnly (multi-format placeholder), Slugify,
//! AsciiOnly, PosixifyPaths, Typoglycemia.

use super::Transform;
use crate::error::Result;

/// Drop every clipboard format except `text/plain`. In the current
/// single-format Entry model this is a **no-op identity** for text
/// entries — there's nothing else to drop. Phase 4's multi-format
/// Entry rework (see `docs/notes/master-plan-v1.md` H1) will give
/// this transform real teeth: it'll prune HTML / RTF / file-list /
/// image formats from the active clip envelope and emit only the
/// canonical plain text.
///
/// Kept in the registry today as a forward-compat placeholder so
/// CLI / config files / TUI menus that reference `plain-text-only`
/// continue to resolve when the multi-format path lands.
pub struct PlainTextOnly;
impl Transform for PlainTextOnly {
    fn id(&self) -> &'static str {
        "plain-text-only"
    }
    fn name(&self) -> &'static str {
        "Plain text only"
    }
    fn description(&self) -> &'static str {
        "Drop non-text formats (HTML/RTF/etc.). Currently a no-op for single-format text entries; activates with Phase 4's multi-format Entry rework."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        // Phase 4 hook: drop non-text envelopes. Today we already
        // hold only text — pass through.
        Ok(text.to_string())
    }
}

/// `Hello, World!` → `hello-world`. URL-safe ASCII slug:
///
/// 1. Apply Unicode NFKD decomposition (`"é"` → `"e\u{0301}"`).
/// 2. Drop combining marks (`U+0300..=U+036F`).
/// 3. Map a small custom punctuation/symbol table (`©`→`(c)`,
///    `™`→`tm`, `→`→`->`, etc.) BEFORE step 4 in case some symbols
///    don't decompose to ASCII.
/// 4. Replace any non-`[A-Za-z0-9-]` run with a single `-`.
/// 5. Lowercase.
/// 6. Trim leading/trailing `-`.
///
/// Result is always `[a-z0-9-]+` (or empty for input that decomposes
/// to nothing).
pub struct Slugify;
impl Transform for Slugify {
    fn id(&self) -> &'static str {
        "slugify"
    }
    fn name(&self) -> &'static str {
        "Slugify"
    }
    fn description(&self) -> &'static str {
        "URL-safe ASCII slug (lowercase, hyphens, no diacritics)."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        // Phase 1 — symbol map.
        let mapped = apply_symbol_map(text);

        // Phase 2 — NFKD decomposition.
        use unicode_normalization::UnicodeNormalization;
        let decomposed: String = mapped.nfkd().collect();

        // Phase 3 — drop combining marks + non-alphanumeric runs.
        let mut out = String::with_capacity(decomposed.len());
        let mut last_was_sep = true; // suppresses leading `-`
        for ch in decomposed.chars() {
            if is_combining_mark(ch) {
                continue;
            }
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
                last_was_sep = false;
            } else if !last_was_sep {
                out.push('-');
                last_was_sep = true;
            }
        }

        // Phase 4 — strip trailing `-`.
        while out.ends_with('-') {
            out.pop();
        }

        Ok(out)
    }
}

/// Custom symbol → ASCII map, applied before NFKD decomposition for
/// glyphs that don't have an ASCII-friendly normalisation.
///
/// **Do not** read or copy Ditto's `Slugify.h` table; this list is
/// authored from scratch (RFC 3986 reserved-chars + a handful of
/// commonly-encountered marks).
fn apply_symbol_map(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '©' => out.push_str("(c)"),
            '®' => out.push_str("(r)"),
            '™' => out.push_str("tm"),
            '°' => out.push_str("deg"),
            '×' => out.push('x'),
            '÷' => out.push('/'),
            '–' | '—' => out.push('-'),                // en-dash, em-dash
            '\u{2018}' | '\u{2019}' => out.push('\''), // smart single quotes
            '\u{201C}' | '\u{201D}' => out.push('"'),  // smart double quotes
            '…' => out.push_str("..."),
            '→' => out.push_str("->"),
            '←' => out.push_str("<-"),
            other => out.push(other),
        }
    }
    out
}

/// Returns true iff `ch` is in a Unicode combining-mark block.
/// Covers most common diacritics — Mn (Mark, nonspacing) and Mc
/// (Mark, spacing combining) ranges from BMP. We don't pull in
/// `unicode-properties` for this; the ranges are stable.
fn is_combining_mark(ch: char) -> bool {
    matches!(ch as u32,
        0x0300..=0x036F | // Combining Diacritical Marks
        0x1AB0..=0x1AFF | // Combining Diacritical Marks Extended
        0x1DC0..=0x1DFF | // Combining Diacritical Marks Supplement
        0x20D0..=0x20FF | // Combining Diacritical Marks for Symbols
        0xFE20..=0xFE2F   // Combining Half Marks
    )
}

/// Strip every non-ASCII byte. Useful for normalising input destined
/// for ASCII-only sinks. Drops bytes silently — no replacement
/// character.
pub struct AsciiOnly;
impl Transform for AsciiOnly {
    fn id(&self) -> &'static str {
        "ascii-only"
    }
    fn name(&self) -> &'static str {
        "ASCII only"
    }
    fn description(&self) -> &'static str {
        "Strip non-ASCII characters."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(text.chars().filter(|c| c.is_ascii()).collect())
    }
}

/// `\` → `/`. Useful when copying Windows file paths into
/// POSIX-style consumers (`scp`, Linux apps via WSL).
pub struct PosixifyPaths;
impl Transform for PosixifyPaths {
    fn id(&self) -> &'static str {
        "posixify-paths"
    }
    fn name(&self) -> &'static str {
        "Posixify paths"
    }
    fn description(&self) -> &'static str {
        "Replace backslashes with forward slashes."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        Ok(text.replace('\\', "/"))
    }
}

/// Shuffle the inner letters of every word ≥ 4 characters,
/// preserving first and last. Famously still readable. Uses a
/// time-based seed so consecutive runs produce different shuffles
/// (intended).
///
/// Word boundary: ASCII whitespace. Punctuation attached to the
/// word stays with it (`hello,` is treated as a single 6-char
/// word with `,` as the last char — the last "letter" position
/// is `,`, so the actual letters that get shuffled are positions
/// 1..=4).
pub struct Typoglycemia;
impl Transform for Typoglycemia {
    fn id(&self) -> &'static str {
        "typoglycemia"
    }
    fn name(&self) -> &'static str {
        "Typoglycemia"
    }
    fn description(&self) -> &'static str {
        "Shuffle inner letters of long words; first/last preserved."
    }
    fn apply_text(&self, text: &str) -> Result<String> {
        let mut out = String::with_capacity(text.len());
        // Tokenise on whitespace, preserving the whitespace itself
        // so the round-trip is exact except for the inner letters.
        let mut current = String::new();
        for ch in text.chars() {
            if ch.is_whitespace() {
                if !current.is_empty() {
                    out.push_str(&shuffle_inner(&current));
                    current.clear();
                }
                out.push(ch);
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            out.push_str(&shuffle_inner(&current));
        }
        Ok(out)
    }
}

/// Shuffle the inner chars of `word`. For `word.chars().count() < 4`
/// returns the input unchanged.
fn shuffle_inner(word: &str) -> String {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 4 {
        return word.to_string();
    }
    let first = chars[0];
    let last = *chars.last().expect("len >= 4");
    let mut inner: Vec<char> = chars[1..chars.len() - 1].to_vec();

    // Deterministic-per-call shuffle via a small linear congruential
    // generator seeded from the system clock. This avoids pulling
    // in `rand` for one transform.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(42);
    let mut state = seed.wrapping_add(word.len() as u64);
    let len = inner.len();
    if len > 1 {
        // Fisher-Yates with the LCG.
        for i in (1..len).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (state >> 33) as usize % (i + 1);
            inner.swap(i, j);
        }
    }

    let mut out = String::with_capacity(word.len());
    out.push(first);
    for c in inner {
        out.push(c);
    }
    out.push(last);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(t: &dyn Transform, input: &str) -> String {
        t.apply_text(input).expect("infallible transform")
    }

    // -- PlainTextOnly --

    #[test]
    fn plain_text_only_is_identity_for_text() {
        assert_eq!(ok(&PlainTextOnly, "hello world"), "hello world");
        assert_eq!(ok(&PlainTextOnly, ""), "");
    }

    // -- Slugify --

    #[test]
    fn slugify_basic_lowercases_and_hyphenates() {
        assert_eq!(ok(&Slugify, "Hello World"), "hello-world");
    }

    #[test]
    fn slugify_drops_diacritics() {
        assert_eq!(ok(&Slugify, "café"), "cafe");
        assert_eq!(ok(&Slugify, "Crème Brûlée"), "creme-brulee");
        assert_eq!(ok(&Slugify, "über alles"), "uber-alles");
    }

    #[test]
    fn slugify_collapses_punctuation_runs() {
        assert_eq!(ok(&Slugify, "foo!!! bar??? baz..."), "foo-bar-baz");
        assert_eq!(ok(&Slugify, "a----b"), "a-b");
    }

    #[test]
    fn slugify_strips_leading_and_trailing_hyphens() {
        assert_eq!(ok(&Slugify, "---hello---"), "hello");
        assert_eq!(ok(&Slugify, " spaces "), "spaces");
    }

    #[test]
    fn slugify_handles_symbol_map() {
        assert_eq!(ok(&Slugify, "© 2026"), "c-2026");
        assert_eq!(ok(&Slugify, "Foo™"), "footm");
        assert_eq!(ok(&Slugify, "5° hot"), "5deg-hot");
    }

    #[test]
    fn slugify_handles_smart_quotes() {
        assert_eq!(ok(&Slugify, "\u{201C}hello\u{201D}"), "hello");
        assert_eq!(ok(&Slugify, "don\u{2019}t stop"), "don-t-stop");
    }

    #[test]
    fn slugify_empty_input() {
        assert_eq!(ok(&Slugify, ""), "");
        assert_eq!(ok(&Slugify, "   "), "");
        assert_eq!(ok(&Slugify, "..."), "");
    }

    #[test]
    fn slugify_preserves_digits() {
        assert_eq!(ok(&Slugify, "version 1.2.3"), "version-1-2-3");
    }

    #[test]
    fn slugify_idempotent_on_already_slug() {
        // A pre-slugified string round-trips.
        assert_eq!(ok(&Slugify, "hello-world-123"), "hello-world-123");
    }

    // -- AsciiOnly --

    #[test]
    fn ascii_only_strips_unicode() {
        assert_eq!(ok(&AsciiOnly, "café"), "caf");
        assert_eq!(ok(&AsciiOnly, "🦀 rust"), " rust");
    }

    #[test]
    fn ascii_only_preserves_ascii() {
        assert_eq!(ok(&AsciiOnly, "hello world!"), "hello world!");
    }

    // -- PosixifyPaths --

    #[test]
    fn posixify_paths_basic() {
        assert_eq!(ok(&PosixifyPaths, r"C:\Users\foo\bar"), "C:/Users/foo/bar");
    }

    #[test]
    fn posixify_paths_no_backslashes_is_identity() {
        assert_eq!(ok(&PosixifyPaths, "/already/posix"), "/already/posix");
    }

    // -- Typoglycemia --

    #[test]
    fn typoglycemia_short_words_unchanged() {
        // Words < 4 chars have nothing to shuffle.
        assert_eq!(ok(&Typoglycemia, "a"), "a");
        assert_eq!(ok(&Typoglycemia, "to"), "to");
        assert_eq!(ok(&Typoglycemia, "the"), "the");
    }

    #[test]
    fn typoglycemia_preserves_first_and_last() {
        let out = ok(&Typoglycemia, "hello");
        let chars: Vec<char> = out.chars().collect();
        assert_eq!(chars[0], 'h');
        assert_eq!(chars.last(), Some(&'o'));
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn typoglycemia_preserves_whitespace_layout() {
        let input = "the quick brown fox";
        let out = ok(&Typoglycemia, input);
        // Same number of words, same separator characters.
        let in_spaces: usize = input.chars().filter(|c| *c == ' ').count();
        let out_spaces: usize = out.chars().filter(|c| *c == ' ').count();
        assert_eq!(in_spaces, out_spaces);
    }

    #[test]
    fn typoglycemia_preserves_total_length() {
        let input = "aaaaa bbbbb ccccc";
        let out = ok(&Typoglycemia, input);
        assert_eq!(input.chars().count(), out.chars().count());
    }

    #[test]
    fn typoglycemia_empty_input() {
        assert_eq!(ok(&Typoglycemia, ""), "");
    }

    // -- shuffle_inner direct --

    #[test]
    fn shuffle_inner_short_word_unchanged() {
        assert_eq!(shuffle_inner("the"), "the");
        assert_eq!(shuffle_inner("of"), "of");
    }

    #[test]
    fn shuffle_inner_keeps_outer_letters() {
        let s = shuffle_inner("hello");
        assert!(s.starts_with('h'));
        assert!(s.ends_with('o'));
    }
}
