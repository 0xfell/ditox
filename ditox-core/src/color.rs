//! CSS-style color detection (Phase 3 sub-task 3.3).
//!
//! Detects the first color literal in a text snippet so the TUI and
//! TUI list views can render a filled swatch alongside the entry
//! preview. Five formats are recognised in priority order:
//!
//! 1. `#RRGGBBAA` — 8-digit RGBA hex (alpha last, per CSS Color 4).
//! 2. `#RRGGBB` — 6-digit RGB hex.
//! 3. `#RGB` — 3-digit short RGB hex (`#f0a` → `#ff00aa`).
//! 4. `rgb(r, g, b)` / `rgba(r, g, b, a)` — comma-separated.
//! 5. `hsl(h, s%, l%)` / `hsla(h, s%, l%, a)` — converted to RGB
//!    via the standard CSS algorithm.
//!
//! ## Parser strategy
//!
//! A single combined regex finds candidates in one pass; the
//! matched substring is then dispatched to a per-format parser
//! that does the actual byte-to-color conversion. We cap the
//! matched range so adversarial input (a 1 MiB clip with no
//! colors) finishes in microseconds rather than scanning the
//! whole content.
//!
//! Recognised extras:
//! - `#RGBA` (4-digit short — alpha last) is **not** in the spec
//!   list but we accept it for symmetry with `#RGB` since CSS
//!   Color 4 standardises both. Documented in the doc comment so
//!   surprise is minimal.

use regex::Regex;
use std::sync::OnceLock;

/// A color found in some text. Always 8-bit per channel; alpha
/// defaults to `0xFF` (fully opaque) for formats that don't
/// specify it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DetectedColor {
    /// Red channel, 0..=255.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel; `0xFF` = fully opaque.
    pub a: u8,
    /// Byte offset (inclusive) where the color literal starts in
    /// the input string.
    pub start: usize,
    /// Byte offset (exclusive) where the color literal ends.
    pub end: usize,
}

impl DetectedColor {
    /// Build a `DetectedColor` from RGB channels. Alpha defaults
    /// to `0xFF`. `start` and `end` are caller-provided byte
    /// offsets into the source text.
    pub fn rgb(r: u8, g: u8, b: u8, start: usize, end: usize) -> Self {
        Self {
            r,
            g,
            b,
            a: 0xFF,
            start,
            end,
        }
    }

    /// Build a `DetectedColor` with explicit alpha.
    pub fn rgba(r: u8, g: u8, b: u8, a: u8, start: usize, end: usize) -> Self {
        Self {
            r,
            g,
            b,
            a,
            start,
            end,
        }
    }
}

/// Maximum input length we'll scan. Inputs longer than this are
/// truncated before regex matching so a runaway clip can't tie up
/// the renderer's CPU. Phase 3 callers pass entry previews
/// (typically 200-500 bytes); 4 KiB is a generous ceiling.
const MAX_SCAN_BYTES: usize = 4096;

/// Combined regex for all 5 formats. Built once via `OnceLock`.
///
/// Rust's `regex` crate is a strict DFA and doesn't support
/// lookahead — so we post-filter hex matches in
/// [`detect_first_color`] to reject e.g. `#abcde` (the regex
/// matches `#abc`, the post-filter rejects because the next char
/// is also a hex digit).
fn color_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // The alternation arms are mutually exclusive (`#` vs
        // `rgb(` vs `hsl(`), so order doesn't affect correctness.
        // Inside the hex arm we sort longest-first because the
        // regex engine is greedy left-to-right.
        Regex::new(
            r"(?xi)
            (?:
                \#(?P<hex>[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{4}|[0-9a-f]{3})
              |
                rgba?\(
                    \s*(?P<rr>\d{1,3})\s*,
                    \s*(?P<rg>\d{1,3})\s*,
                    \s*(?P<rb>\d{1,3})\s*
                    (?:,\s*(?P<ra>[\d.]+)\s*)?
                \)
              |
                hsla?\(
                    \s*(?P<hh>\d{1,3})(?:deg)?\s*,
                    \s*(?P<hs>\d{1,3})%\s*,
                    \s*(?P<hl>\d{1,3})%\s*
                    (?:,\s*(?P<ha>[\d.]+)\s*)?
                \)
            )
            ",
        )
        .expect("color regex must compile")
    })
}

/// Find the first color literal in `text`. Returns `None` when no
/// color is recognised within the first [`MAX_SCAN_BYTES`] bytes.
///
/// The returned `start` / `end` are byte offsets into the **whole**
/// input (not the truncated scan window) so callers can splice
/// based on them safely.
pub fn detect_first_color(text: &str) -> Option<DetectedColor> {
    let scan = if text.len() > MAX_SCAN_BYTES {
        // Truncate at a UTF-8 boundary to avoid splitting a
        // multi-byte char.
        let mut cut = MAX_SCAN_BYTES;
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }
        &text[..cut]
    } else {
        text
    };

    // Iterate matches so we can skip false-positive hex matches
    // (lookahead-emulation: a hex match where the next char is
    // also a hex digit is actually `#XXXXXX...` with too many
    // digits, which CSS considers invalid).
    for m in color_regex().captures_iter(scan) {
        let Some(whole) = m.get(0) else { continue };
        let start = whole.start();
        let end = whole.end();

        if let Some(c) = parse_match(scan, &m, start, end) {
            return Some(c);
        }
    }
    None
}

fn parse_match(scan: &str, m: &regex::Captures, start: usize, end: usize) -> Option<DetectedColor> {
    if let Some(hex) = m.name("hex") {
        // Reject 3- or 6-digit matches followed by another hex
        // digit (would mean the literal had 4-5 or 7+ chars after
        // `#` — invalid CSS).
        let next_char = scan[end..].chars().next();
        if let Some(c) = next_char {
            if c.is_ascii_hexdigit() {
                return None;
            }
        }
        let s = hex.as_str();
        let (r, g, b, a) = match s.len() {
            8 => (
                hex_byte(&s[0..2])?,
                hex_byte(&s[2..4])?,
                hex_byte(&s[4..6])?,
                hex_byte(&s[6..8])?,
            ),
            6 => (
                hex_byte(&s[0..2])?,
                hex_byte(&s[2..4])?,
                hex_byte(&s[4..6])?,
                0xFF,
            ),
            4 => (
                hex_nibble_to_byte(&s[0..1])?,
                hex_nibble_to_byte(&s[1..2])?,
                hex_nibble_to_byte(&s[2..3])?,
                hex_nibble_to_byte(&s[3..4])?,
            ),
            3 => (
                hex_nibble_to_byte(&s[0..1])?,
                hex_nibble_to_byte(&s[1..2])?,
                hex_nibble_to_byte(&s[2..3])?,
                0xFF,
            ),
            _ => return None,
        };
        return Some(DetectedColor::rgba(r, g, b, a, start, end));
    }

    if let (Some(rr), Some(rg), Some(rb)) = (m.name("rr"), m.name("rg"), m.name("rb")) {
        let r: u8 = rr.as_str().parse().ok().filter(|x: &u32| *x <= 255)? as u8;
        let g: u8 = rg.as_str().parse().ok().filter(|x: &u32| *x <= 255)? as u8;
        let b: u8 = rb.as_str().parse().ok().filter(|x: &u32| *x <= 255)? as u8;
        let a = m
            .name("ra")
            .and_then(|s| s.as_str().parse::<f32>().ok())
            .map(|f| (f.clamp(0.0, 1.0) * 255.0).round() as u8)
            .unwrap_or(0xFF);
        return Some(DetectedColor::rgba(r, g, b, a, start, end));
    }

    if let (Some(hh), Some(hs), Some(hl)) = (m.name("hh"), m.name("hs"), m.name("hl")) {
        let h: f32 = hh.as_str().parse::<u32>().ok()? as f32 % 360.0;
        let s: f32 = (hs.as_str().parse::<u32>().ok()?.min(100) as f32) / 100.0;
        let l: f32 = (hl.as_str().parse::<u32>().ok()?.min(100) as f32) / 100.0;
        let (r, g, b) = hsl_to_rgb(h, s, l);
        let a = m
            .name("ha")
            .and_then(|s| s.as_str().parse::<f32>().ok())
            .map(|f| (f.clamp(0.0, 1.0) * 255.0).round() as u8)
            .unwrap_or(0xFF);
        return Some(DetectedColor::rgba(r, g, b, a, start, end));
    }

    None
}

/// Parse two hex digits to a byte.
fn hex_byte(s: &str) -> Option<u8> {
    u8::from_str_radix(s, 16).ok()
}

/// Parse one hex digit as a byte where the digit is duplicated:
/// `'a'` → `0xAA`. CSS short hex semantics.
fn hex_nibble_to_byte(s: &str) -> Option<u8> {
    let nib = u8::from_str_radix(s, 16).ok()?;
    Some(nib * 0x11)
}

/// CSS-spec HSL → RGB. Hue in degrees \[0, 360); saturation +
/// lightness in [0, 1].
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());

    let (r1, g1, b1) = match h_prime as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x), // 5 (and any wraparound)
    };

    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> DetectedColor {
        detect_first_color(text).unwrap_or_else(|| panic!("no color in {:?}", text))
    }

    // -- 8-digit hex --

    #[test]
    fn hex_8_digit_rgba() {
        let c = d("color: #ff5500aa background");
        assert_eq!((c.r, c.g, c.b, c.a), (0xFF, 0x55, 0x00, 0xAA));
    }

    // -- 6-digit hex --

    #[test]
    fn hex_6_digit_rgb() {
        let c = d("background: #ff5500;");
        assert_eq!((c.r, c.g, c.b, c.a), (0xFF, 0x55, 0x00, 0xFF));
    }

    #[test]
    fn hex_6_digit_uppercase() {
        // Bare hex without `#` is intentionally NOT detected — too
        // many false positives in plain English text.
        assert!(detect_first_color("FF5500").is_none());
        // With the `#` prefix, case is irrelevant.
        let c = d("#FF5500");
        assert_eq!((c.r, c.g, c.b), (0xFF, 0x55, 0x00));
    }

    // -- 4-digit hex (extension) --

    #[test]
    fn hex_4_digit_short_rgba() {
        // `#f0a8` → `#ff00aa88` per CSS Color 4 short syntax.
        let c = d("#f0a8");
        assert_eq!((c.r, c.g, c.b, c.a), (0xFF, 0x00, 0xAA, 0x88));
    }

    // -- 3-digit hex --

    #[test]
    fn hex_3_digit_short_rgb() {
        let c = d("#f0a");
        assert_eq!((c.r, c.g, c.b, c.a), (0xFF, 0x00, 0xAA, 0xFF));
    }

    // -- rgb()/rgba() --

    #[test]
    fn rgb_function() {
        let c = d("color: rgb(255, 85, 0);");
        assert_eq!((c.r, c.g, c.b, c.a), (255, 85, 0, 0xFF));
    }

    #[test]
    fn rgba_function_with_alpha() {
        let c = d("rgba(255, 85, 0, 0.5)");
        assert_eq!((c.r, c.g, c.b), (255, 85, 0));
        // 0.5 * 255 = 127.5 → 128 (round-to-even, but rounded as 128).
        assert_eq!(c.a, 128);
    }

    #[test]
    fn rgba_handles_extra_whitespace() {
        let c = d("rgba(  255 ,  85  ,  0  ,  1  )");
        assert_eq!((c.r, c.g, c.b, c.a), (255, 85, 0, 0xFF));
    }

    // -- hsl()/hsla() --

    #[test]
    fn hsl_red() {
        let c = d("color: hsl(0, 100%, 50%);");
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
    }

    #[test]
    fn hsl_green() {
        let c = d("hsl(120, 100%, 50%)");
        assert_eq!((c.r, c.g, c.b), (0, 255, 0));
    }

    #[test]
    fn hsl_blue() {
        let c = d("hsl(240, 100%, 50%)");
        assert_eq!((c.r, c.g, c.b), (0, 0, 255));
    }

    #[test]
    fn hsl_with_deg_suffix() {
        let c = d("hsl(120deg, 100%, 50%)");
        assert_eq!((c.r, c.g, c.b), (0, 255, 0));
    }

    #[test]
    fn hsla_with_alpha() {
        let c = d("hsla(0, 100%, 50%, 0.5)");
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
        assert_eq!(c.a, 128);
    }

    // -- offsets --

    #[test]
    fn detected_offsets_point_at_match() {
        let s = "  prefix #ff5500 suffix";
        let c = d(s);
        assert_eq!(&s[c.start..c.end], "#ff5500");
    }

    // -- not-a-color cases --

    #[test]
    fn no_color_returns_none() {
        assert!(detect_first_color("just plain text").is_none());
        assert!(detect_first_color("").is_none());
        assert!(detect_first_color("#xyz123").is_none());
    }

    #[test]
    fn five_digit_hex_is_not_a_color() {
        // `#abcde` is not a valid CSS color.
        assert!(detect_first_color("#abcde").is_none());
    }

    #[test]
    fn rgb_out_of_range_rejected_or_clamped() {
        // 999 > 255: regex still matches the digits. The parse
        // would fail, then `parse().ok().filter(|x| *x <= 255)`
        // returns None and detect_first_color returns None.
        assert!(detect_first_color("rgb(999, 0, 0)").is_none());
    }

    // -- priority order --

    #[test]
    fn first_match_wins_in_text() {
        // Two colors in the input; the one earlier in the text wins.
        let s = "early #ff0000 then later rgb(0, 0, 255)";
        let c = d(s);
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
    }

    // -- truncation --

    #[test]
    fn input_longer_than_max_scan_still_works() {
        // Construct a string longer than MAX_SCAN_BYTES with the
        // color near the start.
        let mut s = String::from("#ff5500 ");
        s.push_str(&"x".repeat(8 * 1024));
        let c = d(&s);
        assert_eq!((c.r, c.g, c.b), (255, 85, 0));
    }

    #[test]
    fn input_color_past_truncation_returns_none() {
        let mut s = "x".repeat(MAX_SCAN_BYTES + 100);
        s.push_str(" #ff5500"); // beyond the scan window
        assert!(detect_first_color(&s).is_none());
    }
}
