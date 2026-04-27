//! Translate / web-search URL templates (Phase 3 sub-task 3.8).
//!
//! Lets the user open a clip's contents in an external service via
//! a configurable URL template. Two built-in templates ship by
//! default:
//!
//! ```toml
//! [actions]
//! translate_url  = "https://translate.google.com/?text={q}"
//! web_search_url = "https://duckduckgo.com/?q={q}"
//! ```
//!
//! `{q}` is the substitution placeholder; the clip's text is
//! URL-encoded (RFC 3986 unreserved chars passed through, everything
//! else `%XX`-escaped) and substituted in place. Other tokens in
//! the template are passed through verbatim.
//!
//! ## Cross-platform launch
//!
//! [`open_in_browser`] shells out to:
//!
//! - Linux:   `xdg-open <url>`
//! - macOS:   `open <url>`
//! - Windows: `cmd /C start "" <url>` (the empty `""` is the
//!   window-title argument that `start` requires)
//!
//! We don't pull in the `open` crate because the platform branches
//! are tiny and we'd rather keep workspace deps lean.

use crate::error::{DitoxError, Result};
use serde::{Deserialize, Serialize};

/// Configurable URL templates. Surfaces in the
/// `[actions]` section of `config.toml`.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct ActionsConfig {
    /// Template for the "Translate" action. Default opens Google
    /// Translate with the clip text pre-filled. `{q}` is replaced
    /// with the URL-encoded clip content.
    pub translate_url: String,
    /// Template for the "Search the web" action. Default opens a
    /// DuckDuckGo query.
    pub web_search_url: String,
}

impl Default for ActionsConfig {
    fn default() -> Self {
        Self {
            translate_url: "https://translate.google.com/?text={q}".to_string(),
            web_search_url: "https://duckduckgo.com/?q={q}".to_string(),
        }
    }
}

impl ActionsConfig {
    /// Render the translate URL with `query` substituted (URL-encoded).
    pub fn translate_url_for(&self, query: &str) -> String {
        substitute(&self.translate_url, query)
    }

    /// Render the web-search URL with `query` substituted (URL-encoded).
    pub fn web_search_url_for(&self, query: &str) -> String {
        substitute(&self.web_search_url, query)
    }
}

/// Built-in actions that can be applied to a clip via the
/// [`ActionsConfig`] templates. Used by the CLI / future GUI
/// context-menu integration to pick a template by name without
/// hardcoding the template strings at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UrlAction {
    Translate,
    WebSearch,
}

impl UrlAction {
    /// Resolve the action against a config to produce a
    /// substituted URL ready for [`open_in_browser`].
    pub fn url_for(self, config: &ActionsConfig, query: &str) -> String {
        match self {
            Self::Translate => config.translate_url_for(query),
            Self::WebSearch => config.web_search_url_for(query),
        }
    }

    /// Parse from a CLI / config string. Accepts the canonical
    /// names (`"translate"`, `"search"`) plus a few common
    /// synonyms.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "translate" | "tr" | "trans" => Some(Self::Translate),
            "search" | "web" | "websearch" | "web-search" => Some(Self::WebSearch),
            _ => None,
        }
    }
}

/// Substitute `{q}` in `template` with the URL-encoded `query`.
/// Other text in the template is passed through verbatim.
///
/// Multiple `{q}` occurrences all receive the same substitution.
/// `{` not followed by `q}` is passed through literally.
pub fn substitute(template: &str, query: &str) -> String {
    let encoded = url_encode(query);
    // Cheap multi-occurrence replace; templates are short (< 200 B).
    template.replace("{q}", &encoded)
}

/// RFC 3986 unreserved-set URL encoder. The unreserved set is
/// `[A-Za-z0-9-_.~]`; every other byte is `%XX`-escaped using
/// uppercase hex.
///
/// Preferred over `urlencoding`/`percent-encoding` crates for
/// dependency hygiene — the function is ~25 lines and trivially
/// correct.
pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        if is_unreserved(byte) {
            out.push(byte as char);
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            out.push('%');
            out.push(HEX[((byte >> 4) & 0xf) as usize] as char);
            out.push(HEX[(byte & 0xf) as usize] as char);
        }
    }
    out
}

#[inline]
fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~')
}

/// Open `url` in the user's default browser via the platform's
/// open-by-protocol launcher. Returns when the launcher process
/// has been spawned (it does not wait for the browser to
/// actually open the URL).
///
/// Platforms:
/// - Linux:   `xdg-open <url>`
/// - macOS:   `open <url>`
/// - Windows: `cmd /C start "" <url>` (the empty `""` is the
///   mandatory window-title argument for `start`).
///
/// Errors surface the launcher's stderr — typical failure modes
/// are "no default browser registered" or "xdg-open not on PATH".
pub fn open_in_browser(url: &str) -> Result<()> {
    use std::process::Command;

    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("start").arg("").arg(url);
        c
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        return Err(DitoxError::Other(format!(
            "no browser-launcher implementation for this platform; would open {}",
            url
        )));
    }

    let output = cmd.output().map_err(|e| {
        DitoxError::Other(format!(
            "failed to spawn browser launcher: {} (url={})",
            e, url
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DitoxError::Other(format!(
            "browser launcher exited with status {}: {} (url={})",
            output.status,
            stderr.trim(),
            url
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- url_encode --

    #[test]
    fn unreserved_chars_pass_through() {
        assert_eq!(url_encode("HelloWorld"), "HelloWorld");
        assert_eq!(url_encode("abc123"), "abc123");
        assert_eq!(url_encode("a-_.~b"), "a-_.~b");
    }

    #[test]
    fn space_becomes_percent_20() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }

    #[test]
    fn special_chars_encoded() {
        assert_eq!(url_encode("a&b"), "a%26b");
        assert_eq!(url_encode("a=b"), "a%3Db");
        assert_eq!(url_encode("a/b"), "a%2Fb");
        assert_eq!(url_encode("a?b"), "a%3Fb");
        assert_eq!(url_encode("a#b"), "a%23b");
        assert_eq!(url_encode("a+b"), "a%2Bb");
    }

    #[test]
    fn unicode_encoded_per_byte_utf8() {
        // "ñ" = 0xC3 0xB1 in UTF-8
        assert_eq!(url_encode("ñ"), "%C3%B1");
        // "🦀" = 0xF0 0x9F 0xA6 0x80
        assert_eq!(url_encode("🦀"), "%F0%9F%A6%80");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(url_encode(""), "");
    }

    #[test]
    fn hex_uppercase() {
        // Per RFC 3986 §2.1, percent-encoded bytes SHOULD use
        // uppercase hex.
        assert_eq!(url_encode(":"), "%3A");
        assert_ne!(url_encode(":"), "%3a");
    }

    // -- substitute --

    #[test]
    fn substitute_replaces_placeholder() {
        let t = "https://example.com/?q={q}";
        assert_eq!(substitute(t, "hello"), "https://example.com/?q=hello");
    }

    #[test]
    fn substitute_url_encodes_the_query() {
        let t = "https://x.com/?q={q}";
        assert_eq!(
            substitute(t, "hello world"),
            "https://x.com/?q=hello%20world"
        );
    }

    #[test]
    fn substitute_replaces_every_occurrence() {
        let t = "https://x.com/{q}/{q}";
        assert_eq!(substitute(t, "hi"), "https://x.com/hi/hi");
    }

    #[test]
    fn substitute_template_without_placeholder_is_identity() {
        let t = "https://example.com/static";
        assert_eq!(substitute(t, "ignored"), "https://example.com/static");
    }

    #[test]
    fn substitute_handles_lone_braces() {
        // `{x}` with x != q is left literal.
        assert_eq!(substitute("a{x}b", "anything"), "a{x}b");
        // Lone `{` stays.
        assert_eq!(substitute("a{b{q}c", "yes"), "a{byesc");
    }

    // -- ActionsConfig --

    #[test]
    fn default_config_has_known_templates() {
        let c = ActionsConfig::default();
        assert!(c.translate_url.contains("{q}"));
        assert!(c.translate_url.contains("translate"));
        assert!(c.web_search_url.contains("{q}"));
        assert!(c.web_search_url.contains("duckduckgo"));
    }

    #[test]
    fn translate_url_for_substitutes_query() {
        let c = ActionsConfig::default();
        let url = c.translate_url_for("hello world");
        assert!(url.contains("hello%20world"));
        assert!(!url.contains("{q}"), "placeholder must be substituted");
    }

    #[test]
    fn web_search_url_for_substitutes_query() {
        let c = ActionsConfig::default();
        let url = c.web_search_url_for("rust async");
        assert!(url.contains("rust%20async"));
        assert!(!url.contains("{q}"));
    }

    // -- UrlAction --

    #[test]
    fn url_action_resolves_to_correct_template() {
        let c = ActionsConfig::default();
        let t = UrlAction::Translate.url_for(&c, "x");
        let s = UrlAction::WebSearch.url_for(&c, "x");
        assert!(t.contains("translate"));
        assert!(s.contains("duckduckgo"));
        assert!(t != s);
    }

    #[test]
    fn url_action_from_name_canonical_and_synonyms() {
        assert_eq!(
            UrlAction::from_name("translate"),
            Some(UrlAction::Translate)
        );
        assert_eq!(
            UrlAction::from_name("TRANSLATE"),
            Some(UrlAction::Translate)
        );
        assert_eq!(UrlAction::from_name("tr"), Some(UrlAction::Translate));
        assert_eq!(UrlAction::from_name("trans"), Some(UrlAction::Translate));

        assert_eq!(UrlAction::from_name("search"), Some(UrlAction::WebSearch));
        assert_eq!(UrlAction::from_name("web"), Some(UrlAction::WebSearch));
        assert_eq!(
            UrlAction::from_name("websearch"),
            Some(UrlAction::WebSearch)
        );
        assert_eq!(
            UrlAction::from_name("web-search"),
            Some(UrlAction::WebSearch)
        );

        assert_eq!(UrlAction::from_name("garbage"), None);
        assert_eq!(UrlAction::from_name(""), None);
    }
}
