//! Cross-entry aggregation for the multi-select "merge" workflow.
//!
//! When a user selects N entries in the TUI and chooses "paste as
//! one", we synthesise a single new clip by combining each entry's
//! bytes for one chosen format. Different formats need different join
//! strategies — HTML wants a single Windows clipboard envelope around
//! the concatenated fragments, RTF wants one fresh `\rtf1` wrapper,
//! images want a 2D stack. Each strategy is a [`FormatAggregator`].
//!
//! Aggregators consume canonical bytes (the same bytes
//! [`crate::format::canonicalise`] produces and the same bytes the DB
//! holds in `entry_formats.content` for inline rows) and produce
//! canonical bytes for the same `format_name`. The output is
//! re-canonicalised before storage by the existing
//! [`crate::db::ExtraFormat`] pipeline, so aggregator round-trips
//! through the DB are idempotent.
//!
//! ## Stability contract
//!
//! - [`FormatAggregator::format_name`] is the canonical
//!   `entry_formats.format_name` the merged result lands in.
//! - [`FormatAggregator::aggregate`] is byte-deterministic for the
//!   same inputs (no timestamps, no random IDs).
//! - Empty input is always rejected with [`AggregateError::Empty`];
//!   callers must never invoke with zero parts.
//! - A single-input invocation is allowed and produces a wrapped form
//!   of that input (envelope/wrapper still applied) so the output is
//!   indistinguishable from a multi-input merge of size 1.
//!
//! ## v0.4 limitations (recorded in task 023 work log)
//!
//! - [`RtfAggregator`] does naive prologue stripping + body concat
//!   inside a fresh `{\rtf1\ansi …}` wrapper. Per-input font tables
//!   and color tables are kept verbatim in their own group, so
//!   `\f0` / `\cf0` collisions between inputs are possible. A real
//!   merger that rewrites font/color indices is Phase 2+ work.
//! - [`ImageStackAggregator`] always emits PNG (the workspace `image`
//!   crate features are `png`, `jpeg`, `webp`; PNG is the only one
//!   that preserves alpha for the transparent padding produced when
//!   inputs have unequal cross-axis dimensions).
//! - File-list aggregation is `text/uri-list` only; Windows
//!   `CF_HDROP` inputs are converted to URI lists at the capture
//!   layer (Phase 1.4) so this aggregator never sees raw HDROP bytes.

use std::io::Cursor;

use image::{ImageFormat, RgbaImage};

use crate::format::well_known;

/// Errors produced by [`FormatAggregator::aggregate`].
#[derive(Debug, thiserror::Error)]
pub enum AggregateError {
    /// Caller passed zero input parts. Aggregators always reject
    /// empty input rather than silently producing an empty payload —
    /// the empty case is a caller bug (no entries selected).
    #[error("no input parts to aggregate")]
    Empty,

    /// An image-bearing input failed to decode. Carries the index in
    /// the original input slice so callers can surface "image #3 was
    /// corrupt" to the user.
    #[error("failed to decode image at index {index}: {source}")]
    ImageDecode {
        index: usize,
        source: image::ImageError,
    },

    /// PNG re-encoding of the stacked image failed. Almost always an
    /// out-of-memory condition (the stacked canvas is too large).
    #[error("failed to encode aggregated image: {0}")]
    ImageEncode(image::ImageError),

    /// A text-bearing input was not valid UTF-8. Plain text and URI
    /// list aggregators require UTF-8; HTML and RTF byte-concat and
    /// don't enforce UTF-8.
    #[error("invalid UTF-8 in input at index {index}")]
    InvalidUtf8 { index: usize },
}

/// Strategy for combining N canonical-byte payloads of one format
/// into a single canonical-byte payload of the same format.
pub trait FormatAggregator {
    /// Canonical format name this aggregator emits — matches
    /// `entry_formats.format_name` and the `RawFormat::mime` value
    /// used during capture. Always a stable static string per impl.
    fn format_name(&self) -> &str;

    /// Combine `parts` into a single canonical payload for
    /// [`Self::format_name`]. Returns [`AggregateError::Empty`] when
    /// `parts` is empty.
    fn aggregate(&self, parts: &[&[u8]]) -> Result<Vec<u8>, AggregateError>;
}

// ---------------------------------------------------------------------------
// PlainTextAggregator
// ---------------------------------------------------------------------------

/// Joins UTF-8 text payloads with a configurable separator.
///
/// Default separator is `"\n"`. The separator is inserted only
/// *between* parts; the first and last parts are emitted verbatim
/// (no leading or trailing separator), matching `String::join`
/// semantics.
#[derive(Debug, Clone)]
pub struct PlainTextAggregator {
    pub separator: String,
}

impl Default for PlainTextAggregator {
    fn default() -> Self {
        Self {
            separator: "\n".to_string(),
        }
    }
}

impl PlainTextAggregator {
    pub fn new(separator: impl Into<String>) -> Self {
        Self {
            separator: separator.into(),
        }
    }
}

impl FormatAggregator for PlainTextAggregator {
    fn format_name(&self) -> &str {
        well_known::TEXT_PLAIN_UTF8
    }

    fn aggregate(&self, parts: &[&[u8]]) -> Result<Vec<u8>, AggregateError> {
        if parts.is_empty() {
            return Err(AggregateError::Empty);
        }
        // Validate UTF-8 up front so a mid-write failure can't leave
        // a partially-built buffer in callers' hands.
        for (i, p) in parts.iter().enumerate() {
            if std::str::from_utf8(p).is_err() {
                return Err(AggregateError::InvalidUtf8 { index: i });
            }
        }
        let sep = self.separator.as_bytes();
        let total: usize = parts.iter().map(|p| p.len()).sum::<usize>()
            + sep.len() * parts.len().saturating_sub(1);
        let mut out = Vec::with_capacity(total);
        for (i, p) in parts.iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(sep);
            }
            out.extend_from_slice(p);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// HtmlEnvelopeAggregator
// ---------------------------------------------------------------------------

/// Wraps the byte concatenation of N HTML fragments in a single
/// Windows "HTML Format" clipboard envelope.
///
/// Header layout (all values 8-digit zero-padded so the header is a
/// fixed 97 bytes regardless of the fragment size):
///
/// ```text
/// Version:0.9\r\n
/// StartHTML:<offset>\r\n
/// EndHTML:<offset>\r\n
/// StartFragment:<offset>\r\n
/// EndFragment:<offset>\r\n
/// ```
///
/// Body:
///
/// ```text
/// <html><body><!--StartFragment-->{joined}<!--EndFragment--></body></html>
/// ```
///
/// Round-trip: feeding the output through
/// [`crate::format::canonicalise::html_envelope`] returns the joined
/// fragments exactly. This is asserted by [`tests`].
///
/// `source_url` (when set) is *not* part of the fixed-length header —
/// it's appended after `EndFragment`, before the body. The fragment
/// offsets account for the appended bytes.
#[derive(Debug, Clone, Default)]
pub struct HtmlEnvelopeAggregator {
    /// Optional `SourceURL:` header to record the original page the
    /// merged HTML came from. Currently aggregator callers leave this
    /// `None` (a merge of N entries has no single source URL); kept
    /// for parity with paste-back of a single-entry envelope.
    pub source_url: Option<String>,
}

impl HtmlEnvelopeAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source_url(url: impl Into<String>) -> Self {
        Self {
            source_url: Some(url.into()),
        }
    }
}

impl FormatAggregator for HtmlEnvelopeAggregator {
    fn format_name(&self) -> &str {
        well_known::TEXT_HTML
    }

    fn aggregate(&self, parts: &[&[u8]]) -> Result<Vec<u8>, AggregateError> {
        if parts.is_empty() {
            return Err(AggregateError::Empty);
        }

        // Concat the input fragments verbatim — no separator. Each
        // fragment is already a self-contained chunk of HTML; adding
        // `<br>` or `<p>` would impose layout the user didn't ask for.
        let fragment_len: usize = parts.iter().map(|p| p.len()).sum();

        // Optional `SourceURL:` header. Appended right after
        // `EndFragment:` so that the four core offsets occupy fixed
        // positions and stay computable up-front.
        let source_url_header = self
            .source_url
            .as_ref()
            .map(|u| format!("SourceURL:{}\r\n", u))
            .unwrap_or_default();

        // Fixed header layout — exactly 97 bytes when the source URL
        // is absent. Lengths:
        //   "Version:0.9\r\n"             = 13
        //   "StartHTML:00000000\r\n"      = 20
        //   "EndHTML:00000000\r\n"        = 18
        //   "StartFragment:00000000\r\n"  = 24
        //   "EndFragment:00000000\r\n"    = 22
        //                            sum  = 97
        const FIXED_HEADER_LEN: usize = 97;

        // Body layout:
        //   "<html><body>"               12 bytes
        //   "<!--StartFragment-->"       20 bytes
        //   <fragment bytes>             fragment_len
        //   "<!--EndFragment-->"         18 bytes
        //   "</body></html>"             14 bytes
        const PRE_FRAG: &[u8] = b"<html><body><!--StartFragment-->";
        const POST_FRAG: &[u8] = b"<!--EndFragment--></body></html>";

        let header_len = FIXED_HEADER_LEN + source_url_header.len();
        let start_html = header_len;
        let start_fragment = header_len + PRE_FRAG.len();
        let end_fragment = start_fragment + fragment_len;
        let end_html = end_fragment + POST_FRAG.len();

        let mut out = Vec::with_capacity(end_html);
        out.extend_from_slice(b"Version:0.9\r\n");
        out.extend_from_slice(format!("StartHTML:{:08}\r\n", start_html).as_bytes());
        out.extend_from_slice(format!("EndHTML:{:08}\r\n", end_html).as_bytes());
        out.extend_from_slice(format!("StartFragment:{:08}\r\n", start_fragment).as_bytes());
        out.extend_from_slice(format!("EndFragment:{:08}\r\n", end_fragment).as_bytes());
        out.extend_from_slice(source_url_header.as_bytes());
        out.extend_from_slice(PRE_FRAG);
        for p in parts {
            out.extend_from_slice(p);
        }
        out.extend_from_slice(POST_FRAG);

        debug_assert_eq!(
            out.len(),
            end_html,
            "header offsets must match output length"
        );
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// RtfAggregator
// ---------------------------------------------------------------------------

/// Wraps N RTF documents in a single fresh `{\rtf1\ansi …}` envelope,
/// separating each input's body with `\par`.
///
/// Each input is passed through [`strip_rtf_prologue`] to remove the
/// outer `{` / `}` and the leading `\rtf1\ansi…\deff…` prologue
/// control words. The remaining body (including any `\fonttbl`,
/// `\colortbl`, `\stylesheet` destination groups) is wrapped in a
/// `{}` group to scope local control-word state, then joined with
/// `\par\n` separators.
///
/// Non-RTF inputs (anything not starting with `{\rtf`) are escaped and
/// emitted as literal text — `{`, `}`, `\` are RTF metacharacters and
/// must be escaped with a leading backslash.
#[derive(Debug, Clone, Default)]
pub struct RtfAggregator;

impl RtfAggregator {
    pub fn new() -> Self {
        Self
    }
}

impl FormatAggregator for RtfAggregator {
    fn format_name(&self) -> &str {
        well_known::TEXT_RTF
    }

    fn aggregate(&self, parts: &[&[u8]]) -> Result<Vec<u8>, AggregateError> {
        if parts.is_empty() {
            return Err(AggregateError::Empty);
        }

        // ~64 bytes wrapper + ~6 bytes per separator + sum of input
        // lengths is a safe lower bound.
        let approx: usize = 64 + parts.iter().map(|p| p.len() + 8).sum::<usize>();
        let mut out = Vec::with_capacity(approx);

        out.extend_from_slice(b"{\\rtf1\\ansi\n");
        for (i, &p) in parts.iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(b"\\par\n");
            }
            out.push(b'{');
            if p.starts_with(b"{\\rtf") {
                let body = strip_rtf_prologue(p);
                out.extend_from_slice(body);
            } else {
                // Plain text — escape the three RTF metacharacters
                // (`\`, `{`, `}`). Anything else is passed through;
                // non-ASCII bytes are written verbatim and rendered
                // by the consumer per the RTF default codepage.
                escape_plain_into_rtf(p, &mut out);
            }
            out.push(b'}');
        }
        out.extend_from_slice(b"}\n");
        Ok(out)
    }
}

/// Strip the outer `{` and `}` braces and the leading `\rtf…\ansi…`
/// prologue control words from a complete RTF document. Returns a
/// slice into the input pointing at the first byte of real content
/// (typically `{\fonttbl` or literal text).
///
/// Conservative on malformed input: if the input doesn't start with
/// `{\rtf`, returns the whole slice unchanged. If the prologue
/// control-word skip walks past the end, returns an empty slice.
fn strip_rtf_prologue(bytes: &[u8]) -> &[u8] {
    if !bytes.starts_with(b"{\\rtf") || !bytes.ends_with(b"}") {
        return bytes;
    }
    // Strip outer braces.
    let inner = &bytes[1..bytes.len() - 1];

    // Skip prologue control words: `\rtf1`, `\ansi`, `\ansicpg1252`,
    // `\deff0`, `\deflang1033`, `\deflangfe2052`, etc. Stop at the
    // first `{` (a destination group like `\fonttbl`) or the first
    // non-control-word byte (literal content).
    let mut i = 0;
    // Optional leading whitespace (rare but possible).
    while i < inner.len() && inner[i].is_ascii_whitespace() {
        i += 1;
    }
    while i < inner.len() && inner[i] == b'\\' {
        let cw_start = i;
        i += 1; // skip `\`
                // Control word name: ASCII letters.
        while i < inner.len() && inner[i].is_ascii_alphabetic() {
            i += 1;
        }
        // No letters → this is a control symbol (`\\`, `\{`, `\}`,
        // `\*`), not a control word — bail out without consuming it.
        if i == cw_start + 1 {
            return &inner[cw_start..];
        }
        // Optional negative sign + digits (numeric parameter).
        if i < inner.len() && inner[i] == b'-' {
            i += 1;
        }
        while i < inner.len() && inner[i].is_ascii_digit() {
            i += 1;
        }
        // Single optional delimiter space — consumed and not part of
        // content.
        if i < inner.len() && inner[i] == b' ' {
            i += 1;
        }
        // Stop conditions: hit a destination group or content text.
        if i >= inner.len() || inner[i] != b'\\' {
            break;
        }
    }
    &inner[i..]
}

/// Escape RTF metacharacters into `out`. `\`, `{`, `}` get a leading
/// backslash; everything else is copied verbatim.
fn escape_plain_into_rtf(bytes: &[u8], out: &mut Vec<u8>) {
    out.reserve(bytes.len());
    for &b in bytes {
        match b {
            b'\\' | b'{' | b'}' => {
                out.push(b'\\');
                out.push(b);
            }
            _ => out.push(b),
        }
    }
}

// ---------------------------------------------------------------------------
// UriListAggregator
// ---------------------------------------------------------------------------

/// Concatenates RFC-2483 URI lists (`text/uri-list`) into a single
/// CRLF-terminated URI list.
///
/// Each input is split on `\r\n` or `\n`, blank lines are dropped,
/// `#`-comment lines are dropped, and the surviving lines are emitted
/// in input order with `\r\n` terminators (per RFC 2483 §5).
///
/// Duplicate URIs are *not* deduplicated — the user explicitly chose
/// these entries to merge, so preserving order and multiplicity is
/// the correct behaviour.
#[derive(Debug, Clone, Default)]
pub struct UriListAggregator;

impl UriListAggregator {
    pub fn new() -> Self {
        Self
    }
}

impl FormatAggregator for UriListAggregator {
    fn format_name(&self) -> &str {
        well_known::URI_LIST
    }

    fn aggregate(&self, parts: &[&[u8]]) -> Result<Vec<u8>, AggregateError> {
        if parts.is_empty() {
            return Err(AggregateError::Empty);
        }

        let mut out = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            let s = std::str::from_utf8(p).map_err(|_| AggregateError::InvalidUtf8 { index: i })?;
            for line in s.lines() {
                let trimmed = line.trim_end_matches('\r');
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                out.extend_from_slice(trimmed.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// ImageStackAggregator
// ---------------------------------------------------------------------------

/// Stacking direction for [`ImageStackAggregator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackAxis {
    /// Stack inputs left-to-right; output width = sum, height = max.
    Horizontal,
    /// Stack inputs top-to-bottom; output width = max, height = sum.
    Vertical,
}

/// Decodes N image inputs (PNG/JPEG/WebP), pastes them into a single
/// transparent RGBA8 canvas along the chosen axis, and re-encodes as
/// PNG.
///
/// Layout: each input is placed at (0, cumulative_offset) for
/// vertical, (cumulative_offset, 0) for horizontal. Cross-axis
/// padding is fully transparent (RGBA `0,0,0,0`). Inputs are *not*
/// resized; mismatched cross-axis dimensions just leave empty space.
///
/// Always emits PNG (the workspace `image` crate features cover
/// `png`/`jpeg`/`webp` and PNG is the only one that preserves the
/// alpha channel needed for the transparent padding).
#[derive(Debug, Clone)]
pub struct ImageStackAggregator {
    pub axis: StackAxis,
}

impl ImageStackAggregator {
    pub fn new(axis: StackAxis) -> Self {
        Self { axis }
    }
}

impl FormatAggregator for ImageStackAggregator {
    fn format_name(&self) -> &str {
        well_known::IMAGE_PNG
    }

    fn aggregate(&self, parts: &[&[u8]]) -> Result<Vec<u8>, AggregateError> {
        if parts.is_empty() {
            return Err(AggregateError::Empty);
        }

        // Decode all inputs up front — convert to RGBA8 so the paste
        // step is a uniform per-pixel copy.
        let mut decoded: Vec<RgbaImage> = Vec::with_capacity(parts.len());
        for (i, p) in parts.iter().enumerate() {
            let img = image::load_from_memory(p).map_err(|e| AggregateError::ImageDecode {
                index: i,
                source: e,
            })?;
            decoded.push(img.into_rgba8());
        }

        // Compute output canvas dimensions.
        let (out_w, out_h) = match self.axis {
            StackAxis::Horizontal => {
                let w: u32 = decoded.iter().map(|i| i.width()).sum();
                let h: u32 = decoded.iter().map(|i| i.height()).max().unwrap_or(0);
                (w, h)
            }
            StackAxis::Vertical => {
                let w: u32 = decoded.iter().map(|i| i.width()).max().unwrap_or(0);
                let h: u32 = decoded.iter().map(|i| i.height()).sum();
                (w, h)
            }
        };

        let mut canvas = RgbaImage::new(out_w, out_h);
        let mut cursor: u32 = 0;
        for img in &decoded {
            let (x, y) = match self.axis {
                StackAxis::Horizontal => (cursor, 0u32),
                StackAxis::Vertical => (0u32, cursor),
            };
            // Manual paste — `image::imageops::overlay` would also
            // work but is gated on the `image` crate's `default-features`
            // path; we keep `default-features = false` to slim build
            // time, so we hand-roll the loop.
            for (px, py, pixel) in img.enumerate_pixels() {
                canvas.put_pixel(x + px, y + py, *pixel);
            }
            cursor += match self.axis {
                StackAxis::Horizontal => img.width(),
                StackAxis::Vertical => img.height(),
            };
        }

        let mut out = Cursor::new(Vec::with_capacity((out_w * out_h * 4) as usize));
        canvas
            .write_to(&mut out, ImageFormat::Png)
            .map_err(AggregateError::ImageEncode)?;
        Ok(out.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::canonicalise::html_envelope;

    // -----------------------------------------------------------------
    // PlainTextAggregator
    // -----------------------------------------------------------------

    #[test]
    fn plain_text_empty_is_error() {
        let agg = PlainTextAggregator::default();
        assert!(matches!(agg.aggregate(&[]), Err(AggregateError::Empty)));
    }

    #[test]
    fn plain_text_single_part_is_identity() {
        let agg = PlainTextAggregator::default();
        let out = agg.aggregate(&[b"hello"]).unwrap();
        assert_eq!(out, b"hello");
    }

    #[test]
    fn plain_text_default_separator_is_newline() {
        let agg = PlainTextAggregator::default();
        let out = agg.aggregate(&[b"one", b"two", b"three"]).unwrap();
        assert_eq!(out, b"one\ntwo\nthree");
    }

    #[test]
    fn plain_text_custom_separator_only_between_parts() {
        let agg = PlainTextAggregator::new(" | ");
        let out = agg.aggregate(&[b"a", b"b", b"c"]).unwrap();
        assert_eq!(out, b"a | b | c");
    }

    #[test]
    fn plain_text_rejects_invalid_utf8() {
        let agg = PlainTextAggregator::default();
        let bad: &[u8] = &[0xFF, 0xFE, 0xFD];
        let err = agg.aggregate(&[b"ok", bad]).unwrap_err();
        assert!(matches!(err, AggregateError::InvalidUtf8 { index: 1 }));
    }

    #[test]
    fn plain_text_format_name_is_canonical() {
        assert_eq!(
            PlainTextAggregator::default().format_name(),
            "text/plain;charset=utf-8"
        );
    }

    // -----------------------------------------------------------------
    // HtmlEnvelopeAggregator
    // -----------------------------------------------------------------

    #[test]
    fn html_envelope_empty_is_error() {
        let agg = HtmlEnvelopeAggregator::new();
        assert!(matches!(agg.aggregate(&[]), Err(AggregateError::Empty)));
    }

    #[test]
    fn html_envelope_header_has_fixed_97_byte_length() {
        // The header layout is exactly 97 bytes when no SourceURL is
        // set — relied on by the offset math in `aggregate`.
        let agg = HtmlEnvelopeAggregator::new();
        let out = agg.aggregate(&[b"x"]).unwrap();
        // Body starts at offset 97 with `<html><body>...`.
        assert_eq!(&out[97..109], b"<html><body>");
    }

    #[test]
    fn html_envelope_round_trip_via_canonicalise() {
        let agg = HtmlEnvelopeAggregator::new();
        let parts: &[&[u8]] = &[b"<p>one</p>", b"<p>two</p>", b"<p>three</p>"];
        let out = agg.aggregate(parts).unwrap();

        let parsed = html_envelope(&out);
        assert!(parsed.was_envelope, "output must be a valid envelope");
        // Concat of inputs must equal parsed fragment.
        let mut expected = Vec::new();
        for p in parts {
            expected.extend_from_slice(p);
        }
        assert_eq!(parsed.fragment, expected);
        assert!(parsed.source_url.is_none());
    }

    #[test]
    fn html_envelope_with_source_url_round_trips() {
        let agg = HtmlEnvelopeAggregator::with_source_url("https://example.com/page");
        let out = agg.aggregate(&[b"<b>hi</b>"]).unwrap();
        let parsed = html_envelope(&out);
        assert!(parsed.was_envelope);
        assert_eq!(parsed.fragment, b"<b>hi</b>");
        assert_eq!(
            parsed.source_url.as_deref(),
            Some("https://example.com/page")
        );
    }

    #[test]
    fn html_envelope_offsets_match_actual_positions() {
        let agg = HtmlEnvelopeAggregator::new();
        let out = agg.aggregate(&[b"abc"]).unwrap();
        let s = std::str::from_utf8(&out).unwrap();

        // Extract the four declared offsets.
        let pick = |key: &str| -> usize {
            let line = s.lines().find(|l| l.starts_with(key)).unwrap();
            line[key.len()..].trim().parse().unwrap()
        };
        let start_html = pick("StartHTML:");
        let end_html = pick("EndHTML:");
        let start_fragment = pick("StartFragment:");
        let end_fragment = pick("EndFragment:");

        // StartHTML points at the first byte of `<html>`.
        assert_eq!(&out[start_html..start_html + 6], b"<html>");
        // EndHTML is the byte AFTER the closing `</html>`.
        assert_eq!(end_html, out.len());
        // Fragment slice equals the input.
        assert_eq!(&out[start_fragment..end_fragment], b"abc");
    }

    #[test]
    fn html_envelope_format_name_is_text_html() {
        assert_eq!(HtmlEnvelopeAggregator::new().format_name(), "text/html");
    }

    // -----------------------------------------------------------------
    // RtfAggregator
    // -----------------------------------------------------------------

    #[test]
    fn rtf_empty_is_error() {
        let agg = RtfAggregator::new();
        assert!(matches!(agg.aggregate(&[]), Err(AggregateError::Empty)));
    }

    #[test]
    fn rtf_wraps_single_input_in_fresh_envelope() {
        let agg = RtfAggregator::new();
        let input = b"{\\rtf1\\ansi hello}";
        let out = agg.aggregate(&[input]).unwrap();
        // Output starts and ends with our wrapper.
        assert!(out.starts_with(b"{\\rtf1\\ansi\n"));
        assert!(out.ends_with(b"}\n"));
        // The input's body (`hello`) appears inside the inner group.
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("{hello}"), "body should be wrapped: {}", s);
    }

    #[test]
    fn rtf_joins_multiple_inputs_with_par_separator() {
        let agg = RtfAggregator::new();
        let a = b"{\\rtf1\\ansi first}";
        let b = b"{\\rtf1\\ansi second}";
        let out = agg.aggregate(&[a, b]).unwrap();
        let s = String::from_utf8(out).unwrap();
        // Both bodies present.
        assert!(s.contains("{first}"));
        assert!(s.contains("{second}"));
        // `\par` separator between them.
        assert!(s.contains("}\\par\n{"), "missing \\par separator: {}", s);
    }

    #[test]
    fn rtf_strips_prologue_with_charset_and_default_font() {
        let agg = RtfAggregator::new();
        let input = b"{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033 body text}";
        let out = agg.aggregate(&[input]).unwrap();
        let s = String::from_utf8(out).unwrap();
        // The inner group should contain `body text` only — none of
        // the prologue control words should leak into the body.
        assert!(s.contains("{body text}"), "prologue not stripped: {}", s);
        assert!(!s.contains("\\ansicpg1252"), "prologue word leaked: {}", s);
        assert!(!s.contains("\\deff0"), "prologue word leaked: {}", s);
    }

    #[test]
    fn rtf_preserves_destination_groups_after_prologue() {
        let agg = RtfAggregator::new();
        // `\fonttbl` is a destination group that should survive into
        // the merged body (it carries font definitions referenced by
        // the content text).
        let input = b"{\\rtf1\\ansi{\\fonttbl{\\f0 Arial;}}content}";
        let out = agg.aggregate(&[input]).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(
            s.contains("{\\fonttbl{\\f0 Arial;}}"),
            "fonttbl lost: {}",
            s
        );
        assert!(s.contains("content"));
    }

    #[test]
    fn rtf_escapes_metacharacters_in_plain_text_input() {
        let agg = RtfAggregator::new();
        // Not RTF — should be escaped and emitted as literal text.
        let input = b"a{b}c\\d";
        let out = agg.aggregate(&[input]).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("{a\\{b\\}c\\\\d}"), "bad escape: {}", s);
    }

    #[test]
    fn rtf_format_name_is_text_rtf() {
        assert_eq!(RtfAggregator::new().format_name(), "text/rtf");
    }

    // -----------------------------------------------------------------
    // UriListAggregator
    // -----------------------------------------------------------------

    #[test]
    fn uri_list_empty_is_error() {
        let agg = UriListAggregator::new();
        assert!(matches!(agg.aggregate(&[]), Err(AggregateError::Empty)));
    }

    #[test]
    fn uri_list_single_part_normalises_to_crlf() {
        let agg = UriListAggregator::new();
        let out = agg
            .aggregate(&[b"file:///tmp/a.txt\nfile:///tmp/b.txt\n"])
            .unwrap();
        assert_eq!(out, b"file:///tmp/a.txt\r\nfile:///tmp/b.txt\r\n");
    }

    #[test]
    fn uri_list_joins_inputs_in_order_preserving_duplicates() {
        let agg = UriListAggregator::new();
        let out = agg
            .aggregate(&[b"file:///a\r\nfile:///b\r\n", b"file:///b\r\nfile:///c\r\n"])
            .unwrap();
        // Dup of file:///b is preserved (user-explicit selection).
        assert_eq!(out, b"file:///a\r\nfile:///b\r\nfile:///b\r\nfile:///c\r\n");
    }

    #[test]
    fn uri_list_drops_blank_and_comment_lines() {
        let agg = UriListAggregator::new();
        let out = agg
            .aggregate(&[b"# header comment\nfile:///a\n\n#trailing\nfile:///b\n"])
            .unwrap();
        assert_eq!(out, b"file:///a\r\nfile:///b\r\n");
    }

    #[test]
    fn uri_list_rejects_invalid_utf8() {
        let agg = UriListAggregator::new();
        let bad: &[u8] = &[0xFF];
        let err = agg.aggregate(&[bad]).unwrap_err();
        assert!(matches!(err, AggregateError::InvalidUtf8 { index: 0 }));
    }

    #[test]
    fn uri_list_format_name_is_text_uri_list() {
        assert_eq!(UriListAggregator::new().format_name(), "text/uri-list");
    }

    // -----------------------------------------------------------------
    // ImageStackAggregator
    // -----------------------------------------------------------------

    /// Encode a solid-colour PNG of the given dimensions for use as a
    /// test fixture.
    fn make_png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut img = RgbaImage::new(w, h);
        for px in img.pixels_mut() {
            *px = image::Rgba(rgba);
        }
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn image_stack_empty_is_error() {
        let agg = ImageStackAggregator::new(StackAxis::Vertical);
        assert!(matches!(agg.aggregate(&[]), Err(AggregateError::Empty)));
    }

    #[test]
    fn image_stack_vertical_sums_heights_and_takes_max_width() {
        let red = make_png(4, 3, [255, 0, 0, 255]);
        let blue = make_png(6, 5, [0, 0, 255, 255]);
        let agg = ImageStackAggregator::new(StackAxis::Vertical);
        let out = agg.aggregate(&[&red, &blue]).unwrap();
        let decoded = image::load_from_memory(&out).unwrap().into_rgba8();
        assert_eq!(decoded.dimensions(), (6, 8));
        // Top-left pixel comes from red input.
        assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
        // Pixel just below red (row 3) comes from blue input.
        assert_eq!(decoded.get_pixel(0, 3).0, [0, 0, 255, 255]);
        // Padding to the right of red (col 5, row 0) is transparent.
        assert_eq!(decoded.get_pixel(5, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn image_stack_horizontal_sums_widths_and_takes_max_height() {
        let red = make_png(3, 4, [255, 0, 0, 255]);
        let blue = make_png(5, 6, [0, 0, 255, 255]);
        let agg = ImageStackAggregator::new(StackAxis::Horizontal);
        let out = agg.aggregate(&[&red, &blue]).unwrap();
        let decoded = image::load_from_memory(&out).unwrap().into_rgba8();
        assert_eq!(decoded.dimensions(), (8, 6));
        // Top-left red.
        assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
        // Just past red horizontally → blue.
        assert_eq!(decoded.get_pixel(3, 0).0, [0, 0, 255, 255]);
        // Padding below red (row 5, col 0) is transparent.
        assert_eq!(decoded.get_pixel(0, 5).0, [0, 0, 0, 0]);
    }

    #[test]
    fn image_stack_single_input_is_re_encoded_as_png() {
        let red = make_png(2, 2, [255, 0, 0, 255]);
        let agg = ImageStackAggregator::new(StackAxis::Vertical);
        let out = agg.aggregate(&[&red]).unwrap();
        let decoded = image::load_from_memory(&out).unwrap().into_rgba8();
        assert_eq!(decoded.dimensions(), (2, 2));
        assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn image_stack_returns_decode_error_with_index() {
        let red = make_png(2, 2, [255, 0, 0, 255]);
        let bad: &[u8] = b"not an image";
        let agg = ImageStackAggregator::new(StackAxis::Vertical);
        let err = agg.aggregate(&[&red, bad]).unwrap_err();
        match err {
            AggregateError::ImageDecode { index, .. } => assert_eq!(index, 1),
            other => panic!("expected ImageDecode {{ index: 1 }}, got {:?}", other),
        }
    }

    #[test]
    fn image_stack_format_name_is_image_png() {
        assert_eq!(
            ImageStackAggregator::new(StackAxis::Vertical).format_name(),
            "image/png"
        );
    }

    // -----------------------------------------------------------------
    // Trait-object usage
    // -----------------------------------------------------------------

    #[test]
    fn aggregators_are_object_safe() {
        // Compile-time check that every impl can live behind a
        // trait object — required by the planned UI which holds a
        // `Box<dyn FormatAggregator>` chosen by the user's selection.
        let aggs: Vec<Box<dyn FormatAggregator>> = vec![
            Box::new(PlainTextAggregator::default()),
            Box::new(HtmlEnvelopeAggregator::new()),
            Box::new(RtfAggregator::new()),
            Box::new(UriListAggregator::new()),
            Box::new(ImageStackAggregator::new(StackAxis::Vertical)),
        ];
        let names: Vec<&str> = aggs.iter().map(|a| a.format_name()).collect();
        assert_eq!(
            names,
            vec![
                "text/plain;charset=utf-8",
                "text/html",
                "text/rtf",
                "text/uri-list",
                "image/png",
            ]
        );
    }
}
