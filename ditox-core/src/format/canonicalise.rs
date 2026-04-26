//! Per-format byte canonicalisation.
//!
//! Goal: make `SHA-256(canonical_bytes)` stable across cosmetic
//! variations that the source application produces but that don't
//! reflect a semantic difference for the user. Two consecutive
//! `Ctrl+C` from Microsoft Word on the same selection emit RTF
//! payloads with different `\rsidN` revision-save IDs but identical
//! visible content; we want to dedup them.
//!
//! Each canonicaliser is intentionally conservative — if we can't
//! parse the input cleanly, we hash the original bytes and let the
//! per-clip dedup catch any near-misses. False positives (treating
//! distinct content as identical) are worse than false negatives
//! (missing a dedup opportunity).

use std::sync::OnceLock;

use regex::bytes::Regex;

/// Result of parsing a Windows "HTML Format" clipboard envelope.
///
/// Spec: <https://learn.microsoft.com/en-us/windows/win32/dataxchg/html-clipboard-format>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalHtml {
    /// Bytes between `StartFragment` and `EndFragment` markers — the
    /// actual user-visible HTML content. Used as the canonical
    /// payload for hashing and storage.
    pub fragment: Vec<u8>,
    /// `SourceURL` header value if present. Stored for paste-back
    /// envelope reconstruction (sub-task 1.6).
    pub source_url: Option<String>,
    /// True when the input parsed as a valid envelope. False means
    /// the bytes were treated as plain HTML and `fragment` is the
    /// whole input.
    pub was_envelope: bool,
}

/// Canonicalise an HTML clipboard payload.
///
/// On Windows, `text/html` clipboard data is wrapped in a header
/// envelope:
///
/// ```text
/// Version:0.9
/// StartHTML:00000123
/// EndHTML:00000456
/// StartFragment:00000234
/// EndFragment:00000345
/// SourceURL:https://example.com/page
/// <html>
///   <body>
///     <!--StartFragment-->actual content<!--EndFragment-->
///   </body>
/// </html>
/// ```
///
/// Linux/Wayland clients typically post the raw HTML without the
/// envelope. We detect both and return the fragment in either case.
pub fn html_envelope(bytes: &[u8]) -> CanonicalHtml {
    // Quick reject: must start with "Version:" within first 32 bytes
    // to be considered an envelope. Cheap pre-filter.
    let head = &bytes[..bytes.len().min(64)];
    if !head.windows(8).any(|w| w == b"Version:") {
        return CanonicalHtml {
            fragment: bytes.to_vec(),
            source_url: None,
            was_envelope: false,
        };
    }

    let header_text = std::str::from_utf8(head).unwrap_or("");
    let mut start_frag: Option<usize> = None;
    let mut end_frag: Option<usize> = None;
    let mut source_url: Option<String> = None;

    // Parse headers up to the first blank line or the first byte
    // beyond `EndHTML`. Each header is `Key:Value` on its own line.
    for line in header_text.lines() {
        if let Some(rest) = line.strip_prefix("StartFragment:") {
            start_frag = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("EndFragment:") {
            end_frag = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("SourceURL:") {
            let url = rest.trim().to_string();
            if !url.is_empty() {
                source_url = Some(url);
            }
        } else if line.is_empty() {
            // Headers terminated.
            break;
        }
    }

    // Headers may also appear past the first 64 bytes — re-scan the
    // whole prefix once we know an envelope is plausible. The Windows
    // spec allows up to ~120 bytes of headers.
    if start_frag.is_none() || end_frag.is_none() {
        let scan_to = bytes.len().min(512);
        let header_str = std::str::from_utf8(&bytes[..scan_to]).unwrap_or("");
        for line in header_str.lines() {
            if let Some(rest) = line.strip_prefix("StartFragment:") {
                start_frag = start_frag.or_else(|| rest.trim().parse().ok());
            } else if let Some(rest) = line.strip_prefix("EndFragment:") {
                end_frag = end_frag.or_else(|| rest.trim().parse().ok());
            } else if let Some(rest) = line.strip_prefix("SourceURL:") {
                if source_url.is_none() {
                    let url = rest.trim().to_string();
                    if !url.is_empty() {
                        source_url = Some(url);
                    }
                }
            }
        }
    }

    match (start_frag, end_frag) {
        (Some(s), Some(e)) if e <= bytes.len() && s <= e => CanonicalHtml {
            fragment: bytes[s..e].to_vec(),
            source_url,
            was_envelope: true,
        },
        _ => CanonicalHtml {
            // Malformed envelope: keep the original bytes so dedup
            // still works, but flag it as not an envelope so callers
            // know not to trust `source_url`.
            fragment: bytes.to_vec(),
            source_url: None,
            was_envelope: false,
        },
    }
}

/// Strip volatile metadata from RTF bytes so that two consecutive
/// copies from Word produce identical canonical bytes.
///
/// Removes:
/// - `\rsid<N>`, `\insrsid<N>`, `\delrsid<N>`, `\charrsid<N>`,
///   `\pararsid<N>`, `\sectrsid<N>`, `\tblrsid<N>` — Revision Save
///   IDs that change every save.
/// - `{\*\rsidtbl ...}` — RSID lookup table.
/// - `{\*\datastore ...}` — embedded binary data store.
/// - `{\*\mmathPr ...}` — math properties (Word-internal).
///
/// Conservative on brace matching: we only strip a `{\*\...}` group
/// if its closing brace appears within 4096 bytes of its opening,
/// which covers all real-world RSID tables and avoids gobbling the
/// whole document on a malformed input.
pub fn rtf(bytes: &[u8]) -> Vec<u8> {
    // Quick reject: not RTF if it doesn't start with `{\rtf`.
    if bytes.len() < 5 || !bytes.starts_with(b"{\\rtf") {
        return bytes.to_vec();
    }

    let mut work = bytes.to_vec();

    // Strip `\rsid*` family of control words. Each match is `\` +
    // word + optional negative sign + digits, terminated by a
    // non-alphanumeric character (space, `\`, `{`, `}`, etc.).
    let rsid_re = rsid_regex();
    work = rsid_re.replace_all(&work, &b""[..]).to_vec();

    // Strip `{\*\rsidtbl ...}`, `{\*\datastore ...}`,
    // `{\*\mmathPr ...}` etc. Use a brace-balanced replacer rather
    // than regex (regex can't match balanced braces).
    work = strip_destination_groups(
        &work,
        &[
            b"{\\*\\rsidtbl",
            b"{\\*\\datastore",
            b"{\\*\\mmathPr",
            b"{\\*\\fldinst HYPERLINK", // hyperlink groups carry session-specific IDs
            b"{\\*\\themedata",
            b"{\\*\\colorschememapping",
            b"{\\*\\latentstyles",
        ],
        4096,
    );

    work
}

fn rsid_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // `\<word>-?<digits>`. The `regex` crate doesn't support
        // look-ahead, but we don't actually need a word boundary:
        // RTF tokenises `\rsid12345abc` as the control word
        // `\rsid12345` followed by literal text `abc`, so stripping
        // the `\rsid12345` prefix and leaving `abc` is correct.
        // `\d+` is greedy, so the digit run stops at the first
        // non-digit character.
        Regex::new(r"(?-u)\\(?:rsid|insrsid|delrsid|charrsid|pararsid|sectrsid|tblrsid)-?\d+")
            .expect("rsid regex")
    })
}

/// Walk `bytes` and remove every occurrence of any prefix in
/// `prefixes` together with its matching closing `}`. Brace counter
/// limits at `max_span` bytes from the opening to avoid runaway
/// stripping on malformed input.
fn strip_destination_groups(bytes: &[u8], prefixes: &[&[u8]], max_span: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    'outer: while i < bytes.len() {
        for prefix in prefixes {
            if bytes[i..].starts_with(prefix) {
                // Find matching `}` with brace counting.
                let mut depth = 1usize;
                let mut j = i + prefix.len();
                let limit = (i + max_span).min(bytes.len());
                while j < limit {
                    match bytes[j] {
                        b'{' if j + 1 < bytes.len() && bytes[j + 1] != b'\\' => {
                            // Plain `{` opens a brace group.
                            depth += 1;
                            j += 1;
                        }
                        b'{' => {
                            depth += 1;
                            j += 1;
                        }
                        b'}' => {
                            depth -= 1;
                            j += 1;
                            if depth == 0 {
                                // Successfully ate the group; skip
                                // past the closing brace.
                                i = j;
                                continue 'outer;
                            }
                        }
                        b'\\' => {
                            // Skip escape sequences (`\\`, `\{`, `\}`).
                            j += if j + 1 < bytes.len() { 2 } else { 1 };
                        }
                        _ => {
                            j += 1;
                        }
                    }
                }
                // Group never closed within the limit — give up,
                // keep the original bytes (may be malformed RTF, may
                // be a legitimately huge group). Falls through to
                // the unchanged-byte path below.
                break;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Canonical bytes for hashing a single format. Dispatches on the
/// MIME type:
///
/// - `text/html` → fragment (envelope-aware)
/// - `text/rtf` → \rsid-stripped
/// - everything else → bytes as-is
///
/// The output is what gets fed into the SHA-256 used as
/// `entry_formats.format_hash`.
pub fn canonical_bytes_for(mime: &str, bytes: &[u8]) -> Vec<u8> {
    match mime {
        crate::format::well_known::TEXT_HTML | "win32:HTML Format" => html_envelope(bytes).fragment,
        crate::format::well_known::TEXT_RTF | "win32:Rich Text Format" => rtf(bytes),
        _ => bytes.to_vec(),
    }
}

/// SHA-256 of `canonical_bytes_for(mime, bytes)`. The hex value is
/// what's stored in `entry_formats.format_hash`.
pub fn format_hash(mime: &str, bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let canonical = canonical_bytes_for(mime, bytes);
    let mut h = Sha256::new();
    h.update(&canonical);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_html_without_envelope_passes_through() {
        let html = b"<p>Hello</p>";
        let r = html_envelope(html);
        assert!(!r.was_envelope);
        assert_eq!(r.fragment, html);
        assert!(r.source_url.is_none());
    }

    #[test]
    fn windows_envelope_extracts_fragment_and_source_url() {
        // Synthesize a real-shaped envelope. EndFragment is exclusive
        // (matches Microsoft spec); start=200 + 12 bytes of "bold
        // content" → end=212.
        let header = "Version:0.9\r\nStartHTML:00000150\r\nEndHTML:00000300\r\nStartFragment:00000200\r\nEndFragment:00000212\r\nSourceURL:https://example.com/page\r\n";
        let mut envelope = header.as_bytes().to_vec();
        while envelope.len() < 200 {
            envelope.push(b' ');
        }
        envelope.extend_from_slice(b"bold content");
        envelope.extend_from_slice(b"<!--EndFragment--></body></html>");

        let r = html_envelope(&envelope);
        assert!(r.was_envelope, "should detect envelope: {:?}", r);
        assert_eq!(r.fragment, b"bold content");
        assert_eq!(r.source_url.as_deref(), Some("https://example.com/page"));
    }

    #[test]
    fn malformed_envelope_falls_back_to_raw_bytes() {
        let bad = b"Version:0.9\r\nStartFragment:99999\r\nEndFragment:88888\r\n<p>oops</p>";
        let r = html_envelope(bad);
        assert!(!r.was_envelope);
        assert_eq!(r.fragment, bad);
    }

    #[test]
    fn rtf_strips_rsid_control_words() {
        let raw = b"{\\rtf1\\ansi\\rsid12345\\insrsid67890 hello \\charrsid111 world}";
        // After stripping rsid words: `\ansi\rsid…\insrsid…` becomes
        // `\ansi`, leaving the original single space before `hello`.
        // The pair of spaces around `\charrsid111` collapses to two
        // spaces around the empty replacement.
        let stripped = rtf(raw);
        assert_eq!(stripped, b"{\\rtf1\\ansi hello  world}".to_vec());
    }

    #[test]
    fn rtf_strips_rsidtbl_destination_group() {
        let raw = b"{\\rtf1\\ansi{\\*\\rsidtbl\\rsid111\\rsid222\\rsid333} hello}";
        let stripped = rtf(raw);
        assert_eq!(stripped, b"{\\rtf1\\ansi hello}".to_vec());
    }

    #[test]
    fn rtf_strips_datastore_group() {
        let raw = b"{\\rtf1{\\*\\datastore deadbeefdeadbeef}content}";
        let stripped = rtf(raw);
        assert_eq!(stripped, b"{\\rtf1content}".to_vec());
    }

    #[test]
    fn rtf_two_copies_with_different_rsid_dedup() {
        let copy1 = b"{\\rtf1\\rsid111 same content}";
        let copy2 = b"{\\rtf1\\rsid222 same content}";
        assert_eq!(rtf(copy1), rtf(copy2));
        assert_eq!(
            format_hash("text/rtf", copy1),
            format_hash("text/rtf", copy2)
        );
    }

    #[test]
    fn rtf_non_rtf_input_passes_through() {
        let raw = b"this is not RTF at all";
        assert_eq!(rtf(raw), raw.to_vec());
    }

    #[test]
    fn rtf_unmatched_destination_group_is_preserved() {
        // No closing brace within max_span — should NOT strip.
        let mut raw = b"{\\rtf1{\\*\\rsidtbl".to_vec();
        raw.extend(std::iter::repeat_n(b'x', 5000));
        let stripped = rtf(&raw);
        // The destination group's opening should still be present
        // because we didn't find the matching close.
        assert!(
            stripped
                .windows(b"{\\*\\rsidtbl".len())
                .any(|w| w == b"{\\*\\rsidtbl"),
            "unmatched group should be preserved verbatim"
        );
    }

    #[test]
    fn format_hash_dispatches_per_mime() {
        // text/plain — hashes raw bytes.
        let plain_hash = format_hash("text/plain;charset=utf-8", b"hello");
        assert_eq!(plain_hash.len(), 64);

        // text/html — hashes fragment, not envelope. The envelope and
        // raw fragment of the same content must hash identically.
        let raw = b"<p>same content</p>";
        let raw_hash = format_hash("text/html", raw);

        // Build an envelope that wraps the same fragment.
        let header = "Version:0.9\r\nStartFragment:00000100\r\nEndFragment:00000119\r\n";
        let mut envelope = header.as_bytes().to_vec();
        while envelope.len() < 100 {
            envelope.push(b' ');
        }
        envelope.extend_from_slice(raw);
        let env_hash = format_hash("text/html", &envelope);
        assert_eq!(
            raw_hash, env_hash,
            "envelope-wrapped HTML must hash to the same value as the raw fragment"
        );

        // text/rtf — hashes \rsid-stripped bytes.
        let rtf1 = b"{\\rtf1\\rsid111 hello}";
        let rtf2 = b"{\\rtf1\\rsid999 hello}";
        assert_eq!(format_hash("text/rtf", rtf1), format_hash("text/rtf", rtf2));
    }

    #[test]
    fn canonical_bytes_for_unknown_mime_is_identity() {
        assert_eq!(
            canonical_bytes_for("application/octet-stream", &[1, 2, 3]),
            vec![1, 2, 3]
        );
        assert_eq!(
            canonical_bytes_for("image/png", &[0x89, 0x50, 0x4E, 0x47]),
            vec![0x89, 0x50, 0x4E, 0x47]
        );
    }
}
