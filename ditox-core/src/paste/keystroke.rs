//! Per-app keystroke override parser (Phase 2 sub-task 2.6).
//!
//! Reads strings from the `[paste.keystrokes]` config table:
//!
//! ```toml
//! [paste.keystrokes]
//! "gvim.exe"   = "\"+gp"          # vim register paste
//! "firefox.exe" = "ctrl+v"
//! "konsole"    = "ctrl+shift+v"
//! "alacritty"  = "ctrl+shift+v"
//! "foot"       = "ctrl+shift+v"
//! ```
//!
//! Default keystroke is `ctrl+v`. The launcher resolves a per-app
//! override by basename lookup (lowercased) and falls back to the
//! default otherwise.
//!
//! ## Format
//!
//! A keystroke sequence is one or more **chords** separated by
//! whitespace; each chord is "press all these keys at the same
//! time". Within a chord, modifier names are joined with `+`.
//!
//! Examples:
//!
//! | String              | Parsed                                     |
//! |---------------------|--------------------------------------------|
//! | `ctrl+v`            | one chord: Ctrl+V                          |
//! | `ctrl+shift+v`      | one chord: Ctrl+Shift+V                    |
//! | `shift+insert`      | one chord: Shift+Insert                    |
//! | `ctrl+v ctrl+s`     | two chords: Ctrl+V, then Ctrl+S            |
//! | `enter`             | one chord: Enter                           |
//! | `ctrl+enter`        | one chord: Ctrl+Enter                      |
//! | `"+gp`              | four chords: `"`, `+`, `g`, `p`            |
//! | `a`                 | one chord: literal `a`                     |
//!
//! ## Disambiguation rules
//!
//! `+` is overloaded — it's both the chord-joiner (`ctrl+v`) and a
//! valid literal character (vim's register paste `"+gp` includes a
//! literal `+`). Resolution per whitespace-separated fragment:
//!
//! 1. If the fragment is **exactly** a special-key name
//!    (`enter`, `tab`, `escape`, `space`, `backspace`, `delete`,
//!    `insert`, `home`, `end`, `pageup`, `pagedown`, `up`, `down`,
//!    `left`, `right`, `f1`–`f24`), it parses as one chord with that
//!    key.
//! 2. Else, if the fragment starts with `<modifier>+`, it parses as
//!    a chord — split on `+`, all parts before the last are
//!    modifiers (must each be one of `ctrl`/`shift`/`alt`/`super`),
//!    and the last is the key (special name or single character).
//! 3. Else, each character of the fragment is its own one-key chord.
//!    This is what makes `"+gp` resolve to four sequential
//!    keystrokes — none of `"`, `+`, `g`, `p` is a modifier name, so
//!    rule 2 doesn't fire.
//!
//! Comparisons are ASCII-case-insensitive (`Ctrl+V` parses identically
//! to `ctrl+v`).

use std::fmt;
use std::str::FromStr;

/// Default keystroke when no per-app override is configured.
pub const DEFAULT_KEYSTROKE: &str = "ctrl+v";

/// Modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    /// Logo / Super / Cmd / Win key — same physical key, four names.
    Super,
}

impl Modifier {
    /// Parse a token. Case-insensitive. Accepts `super`/`logo`/`win`/`meta`/`cmd`.
    pub fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(Modifier::Ctrl),
            "shift" => Some(Modifier::Shift),
            "alt" => Some(Modifier::Alt),
            "super" | "logo" | "win" | "meta" | "cmd" => Some(Modifier::Super),
            _ => None,
        }
    }

    /// Stable lowercase name used in config and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Modifier::Ctrl => "ctrl",
            Modifier::Shift => "shift",
            Modifier::Alt => "alt",
            Modifier::Super => "super",
        }
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Non-character keys that have a stable name. Excludes the modifier
/// keys (those live in [`Modifier`]).
///
/// Members chosen for parity with the spec's example list plus the
/// keys ditox actually needs for paste-back (Insert, arrows, function
/// keys for terminal pastes, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialKey {
    Enter,
    Tab,
    Escape,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
}

impl SpecialKey {
    /// Parse a token. Case-insensitive. Returns `None` for unknown
    /// names; callers fall back to the literal-character path.
    pub fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "enter" | "return" => Some(SpecialKey::Enter),
            "tab" => Some(SpecialKey::Tab),
            "escape" | "esc" => Some(SpecialKey::Escape),
            "space" => Some(SpecialKey::Space),
            "backspace" | "bs" => Some(SpecialKey::Backspace),
            "delete" | "del" => Some(SpecialKey::Delete),
            "insert" | "ins" => Some(SpecialKey::Insert),
            "home" => Some(SpecialKey::Home),
            "end" => Some(SpecialKey::End),
            "pageup" | "pgup" => Some(SpecialKey::PageUp),
            "pagedown" | "pgdn" => Some(SpecialKey::PageDown),
            "up" | "uparrow" => Some(SpecialKey::Up),
            "down" | "downarrow" => Some(SpecialKey::Down),
            "left" | "leftarrow" => Some(SpecialKey::Left),
            "right" | "rightarrow" => Some(SpecialKey::Right),
            "f1" => Some(SpecialKey::F1),
            "f2" => Some(SpecialKey::F2),
            "f3" => Some(SpecialKey::F3),
            "f4" => Some(SpecialKey::F4),
            "f5" => Some(SpecialKey::F5),
            "f6" => Some(SpecialKey::F6),
            "f7" => Some(SpecialKey::F7),
            "f8" => Some(SpecialKey::F8),
            "f9" => Some(SpecialKey::F9),
            "f10" => Some(SpecialKey::F10),
            "f11" => Some(SpecialKey::F11),
            "f12" => Some(SpecialKey::F12),
            "f13" => Some(SpecialKey::F13),
            "f14" => Some(SpecialKey::F14),
            "f15" => Some(SpecialKey::F15),
            "f16" => Some(SpecialKey::F16),
            "f17" => Some(SpecialKey::F17),
            "f18" => Some(SpecialKey::F18),
            "f19" => Some(SpecialKey::F19),
            "f20" => Some(SpecialKey::F20),
            "f21" => Some(SpecialKey::F21),
            "f22" => Some(SpecialKey::F22),
            "f23" => Some(SpecialKey::F23),
            "f24" => Some(SpecialKey::F24),
            _ => None,
        }
    }

    /// Stable lowercase name used in config and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecialKey::Enter => "enter",
            SpecialKey::Tab => "tab",
            SpecialKey::Escape => "escape",
            SpecialKey::Space => "space",
            SpecialKey::Backspace => "backspace",
            SpecialKey::Delete => "delete",
            SpecialKey::Insert => "insert",
            SpecialKey::Home => "home",
            SpecialKey::End => "end",
            SpecialKey::PageUp => "pageup",
            SpecialKey::PageDown => "pagedown",
            SpecialKey::Up => "up",
            SpecialKey::Down => "down",
            SpecialKey::Left => "left",
            SpecialKey::Right => "right",
            SpecialKey::F1 => "f1",
            SpecialKey::F2 => "f2",
            SpecialKey::F3 => "f3",
            SpecialKey::F4 => "f4",
            SpecialKey::F5 => "f5",
            SpecialKey::F6 => "f6",
            SpecialKey::F7 => "f7",
            SpecialKey::F8 => "f8",
            SpecialKey::F9 => "f9",
            SpecialKey::F10 => "f10",
            SpecialKey::F11 => "f11",
            SpecialKey::F12 => "f12",
            SpecialKey::F13 => "f13",
            SpecialKey::F14 => "f14",
            SpecialKey::F15 => "f15",
            SpecialKey::F16 => "f16",
            SpecialKey::F17 => "f17",
            SpecialKey::F18 => "f18",
            SpecialKey::F19 => "f19",
            SpecialKey::F20 => "f20",
            SpecialKey::F21 => "f21",
            SpecialKey::F22 => "f22",
            SpecialKey::F23 => "f23",
            SpecialKey::F24 => "f24",
        }
    }
}

impl fmt::Display for SpecialKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One key in a chord — either a special-named key or a literal
/// character.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Special(SpecialKey),
    /// Literal printable character. May be any Unicode scalar; the
    /// synthesizer back-end is responsible for translating to the
    /// platform's key/scancode representation.
    Char(char),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Special(s) => s.fmt(f),
            Key::Char(c) => write!(f, "{}", c),
        }
    }
}

/// One "press all these keys at the same time" group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

impl Chord {
    pub fn new(key: Key) -> Self {
        Self {
            modifiers: Vec::new(),
            key,
        }
    }

    pub fn with_modifiers(mut self, modifiers: Vec<Modifier>) -> Self {
        self.modifiers = modifiers;
        self
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for m in &self.modifiers {
            write!(f, "{}+", m)?;
        }
        write!(f, "{}", self.key)
    }
}

/// An ordered series of chords — what the user typed top-down.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeystrokeSequence {
    pub chords: Vec<Chord>,
}

impl KeystrokeSequence {
    /// Construct from a vec of chords; mostly for tests.
    pub fn new(chords: Vec<Chord>) -> Self {
        Self { chords }
    }

    /// Convenience: parse from string. Equivalent to [`parse`].
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        parse(input)
    }

    /// True when the sequence has no chords at all.
    pub fn is_empty(&self) -> bool {
        self.chords.is_empty()
    }

    /// Number of chords.
    pub fn len(&self) -> usize {
        self.chords.len()
    }
}

impl fmt::Display for KeystrokeSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, c) in self.chords.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            c.fmt(f)?;
        }
        Ok(())
    }
}

impl FromStr for KeystrokeSequence {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s)
    }
}

/// Parse failures from [`parse`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    /// Input was empty or whitespace-only.
    #[error("keystroke string is empty")]
    Empty,

    /// A chord fragment ends with a trailing `+` (e.g. `ctrl+`) — no
    /// key follows the last modifier.
    #[error("chord '{fragment}' ends with '+' but no key follows")]
    DanglingPlus { fragment: String },

    /// A modifier appears in the key position (e.g. `ctrl+ctrl`) —
    /// the last `+`-segment must be a non-modifier key.
    #[error("chord '{fragment}' uses modifier '{token}' as the key")]
    ModifierAsKey { fragment: String, token: String },

    /// A non-final `+`-segment isn't a known modifier (e.g.
    /// `foo+v` — `foo` is neither a modifier nor a known key, and
    /// rule 2 of the parser only fires when the fragment STARTS with
    /// a modifier).
    ///
    /// In practice this error only fires when the fragment passed
    /// rule-1 (not a special key name) and rule-2 was attempted but
    /// a non-final segment failed to parse as a modifier — this is a
    /// genuine user typo. The fragment falls through to rule-3
    /// (literal sequence) only when the FIRST segment isn't a
    /// modifier.
    #[error("chord '{fragment}' segment '{token}' is not a known modifier")]
    UnknownModifier { fragment: String, token: String },

    /// The key segment of a chord is multi-character but not a
    /// recognised special key name (e.g. `ctrl+abc`).
    #[error("chord '{fragment}' has unrecognised key '{token}'")]
    UnknownKey { fragment: String, token: String },
}

/// Parse a keystroke string into a [`KeystrokeSequence`].
///
/// See the module docs for the format. Empty / whitespace-only input
/// returns [`ParseError::Empty`].
pub fn parse(input: &str) -> Result<KeystrokeSequence, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    let mut chords = Vec::new();
    for fragment in trimmed.split_whitespace() {
        parse_fragment(fragment, &mut chords)?;
    }

    if chords.is_empty() {
        // Defensive — split_whitespace on a non-empty trimmed input
        // always yields ≥1 fragment, but guard anyway.
        return Err(ParseError::Empty);
    }
    Ok(KeystrokeSequence { chords })
}

fn parse_fragment(fragment: &str, out: &mut Vec<Chord>) -> Result<(), ParseError> {
    // Rule 1 — exact special-key match (e.g. `enter`, `f5`).
    if let Some(sk) = SpecialKey::parse(fragment) {
        out.push(Chord::new(Key::Special(sk)));
        return Ok(());
    }

    // Rule 2 — starts with a modifier name followed by `+`.
    // `Modifier::parse` is case-insensitive; we check the first
    // `+`-delimited segment.
    if let Some((first, _rest)) = fragment.split_once('+') {
        if Modifier::parse(first).is_some() {
            return parse_chord_with_modifiers(fragment, out);
        }
    }

    // Rule 3 — literal sequence of single-character chords.
    for c in fragment.chars() {
        out.push(Chord::new(Key::Char(c)));
    }
    Ok(())
}

fn parse_chord_with_modifiers(fragment: &str, out: &mut Vec<Chord>) -> Result<(), ParseError> {
    // Splitting on `+` gives N segments; the last is the key, all
    // earlier ones are modifiers. Rule-2 entry guarantees ≥1 `+`,
    // so segments.len() ≥ 2.
    let segments: Vec<&str> = fragment.split('+').collect();

    // Trailing `+`: e.g. `ctrl+v+` has segments `["ctrl","v",""]`.
    // The empty last segment means the user wrote `+` with no key.
    let key_segment = segments.last().expect("≥2 segments by construction");
    if key_segment.is_empty() {
        return Err(ParseError::DanglingPlus {
            fragment: fragment.to_string(),
        });
    }

    // All segments before the last must be modifiers.
    let mut modifiers = Vec::with_capacity(segments.len() - 1);
    for &seg in &segments[..segments.len() - 1] {
        match Modifier::parse(seg) {
            Some(m) => modifiers.push(m),
            None => {
                return Err(ParseError::UnknownModifier {
                    fragment: fragment.to_string(),
                    token: seg.to_string(),
                });
            }
        }
    }

    // Key segment: first try special-key, then single character. A
    // multi-char non-special is a user typo (e.g. `ctrl+abc`).
    let key = if let Some(sk) = SpecialKey::parse(key_segment) {
        Key::Special(sk)
    } else if Modifier::parse(key_segment).is_some() {
        return Err(ParseError::ModifierAsKey {
            fragment: fragment.to_string(),
            token: key_segment.to_string(),
        });
    } else {
        let mut chars = key_segment.chars();
        let first = chars.next().expect("key segment non-empty");
        if chars.next().is_some() {
            return Err(ParseError::UnknownKey {
                fragment: fragment.to_string(),
                token: key_segment.to_string(),
            });
        }
        Key::Char(first)
    };

    out.push(Chord { modifiers, key });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ks(input: &str) -> KeystrokeSequence {
        parse(input).expect(input)
    }

    // -----------------------------------------------------------------
    // Modifier
    // -----------------------------------------------------------------

    #[test]
    fn modifier_parse_canonical_names() {
        assert_eq!(Modifier::parse("ctrl"), Some(Modifier::Ctrl));
        assert_eq!(Modifier::parse("shift"), Some(Modifier::Shift));
        assert_eq!(Modifier::parse("alt"), Some(Modifier::Alt));
        assert_eq!(Modifier::parse("super"), Some(Modifier::Super));
    }

    #[test]
    fn modifier_parse_synonyms_and_case() {
        assert_eq!(Modifier::parse("Control"), Some(Modifier::Ctrl));
        assert_eq!(Modifier::parse("LOGO"), Some(Modifier::Super));
        assert_eq!(Modifier::parse("Win"), Some(Modifier::Super));
        assert_eq!(Modifier::parse("meta"), Some(Modifier::Super));
        assert_eq!(Modifier::parse("cmd"), Some(Modifier::Super));
    }

    #[test]
    fn modifier_parse_rejects_unknown() {
        assert_eq!(Modifier::parse("hyper"), None);
        assert_eq!(Modifier::parse("v"), None);
        assert_eq!(Modifier::parse(""), None);
    }

    // -----------------------------------------------------------------
    // SpecialKey
    // -----------------------------------------------------------------

    #[test]
    fn specialkey_parse_canonical_names() {
        for (input, expected) in [
            ("enter", SpecialKey::Enter),
            ("tab", SpecialKey::Tab),
            ("escape", SpecialKey::Escape),
            ("space", SpecialKey::Space),
            ("backspace", SpecialKey::Backspace),
            ("delete", SpecialKey::Delete),
            ("insert", SpecialKey::Insert),
            ("f5", SpecialKey::F5),
            ("f24", SpecialKey::F24),
        ] {
            assert_eq!(SpecialKey::parse(input), Some(expected), "{}", input);
        }
    }

    #[test]
    fn specialkey_parse_synonyms_and_case() {
        assert_eq!(SpecialKey::parse("Return"), Some(SpecialKey::Enter));
        assert_eq!(SpecialKey::parse("ESC"), Some(SpecialKey::Escape));
        assert_eq!(SpecialKey::parse("Bs"), Some(SpecialKey::Backspace));
        assert_eq!(SpecialKey::parse("PgUp"), Some(SpecialKey::PageUp));
        assert_eq!(SpecialKey::parse("Ins"), Some(SpecialKey::Insert));
    }

    #[test]
    fn specialkey_parse_rejects_unknown() {
        assert_eq!(SpecialKey::parse("v"), None);
        assert_eq!(SpecialKey::parse("foo"), None);
        assert_eq!(SpecialKey::parse(""), None);
        // F0 isn't real.
        assert_eq!(SpecialKey::parse("f0"), None);
        // F25 isn't in our table.
        assert_eq!(SpecialKey::parse("f25"), None);
    }

    // -----------------------------------------------------------------
    // parse() — happy path table
    // -----------------------------------------------------------------

    #[test]
    fn parse_simple_chord() {
        let s = ks("ctrl+v");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Ctrl]);
        assert_eq!(s.chords[0].key, Key::Char('v'));
    }

    #[test]
    fn parse_multi_modifier_chord() {
        let s = ks("ctrl+shift+v");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Ctrl, Modifier::Shift]);
        assert_eq!(s.chords[0].key, Key::Char('v'));
    }

    #[test]
    fn parse_chord_with_special_key() {
        let s = ks("shift+insert");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Shift]);
        assert_eq!(s.chords[0].key, Key::Special(SpecialKey::Insert));
    }

    #[test]
    fn parse_two_chords_separated_by_whitespace() {
        let s = ks("ctrl+v ctrl+s");
        assert_eq!(s.chords.len(), 2);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Ctrl]);
        assert_eq!(s.chords[0].key, Key::Char('v'));
        assert_eq!(s.chords[1].modifiers, vec![Modifier::Ctrl]);
        assert_eq!(s.chords[1].key, Key::Char('s'));
    }

    #[test]
    fn parse_special_key_alone() {
        // Rule 1: exact special-key name → one Step(Special).
        let s = ks("enter");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![]);
        assert_eq!(s.chords[0].key, Key::Special(SpecialKey::Enter));
    }

    #[test]
    fn parse_vim_register_paste_is_four_literal_chords() {
        // Spec: `"+gp` = `"`, `+`, `g`, `p` (vim's register-paste
        // sequence). Rule 3 (literal characters) fires because none
        // of the segments before the first `+` is a known modifier.
        let s = ks("\"+gp");
        assert_eq!(s.chords.len(), 4);
        assert_eq!(s.chords[0].key, Key::Char('"'));
        assert_eq!(s.chords[1].key, Key::Char('+'));
        assert_eq!(s.chords[2].key, Key::Char('g'));
        assert_eq!(s.chords[3].key, Key::Char('p'));
        for c in &s.chords {
            assert!(c.modifiers.is_empty());
        }
    }

    #[test]
    fn parse_single_char_no_modifier() {
        let s = ks("a");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![]);
        assert_eq!(s.chords[0].key, Key::Char('a'));
    }

    #[test]
    fn parse_multi_char_literal_yields_one_chord_per_char() {
        let s = ks("abc");
        assert_eq!(s.chords.len(), 3);
        assert_eq!(s.chords[0].key, Key::Char('a'));
        assert_eq!(s.chords[1].key, Key::Char('b'));
        assert_eq!(s.chords[2].key, Key::Char('c'));
    }

    #[test]
    fn parse_chord_with_special_key_after_ctrl() {
        let s = ks("ctrl+enter");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Ctrl]);
        assert_eq!(s.chords[0].key, Key::Special(SpecialKey::Enter));
    }

    #[test]
    fn parse_is_case_insensitive() {
        let s = ks("CTRL+Shift+V");
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Ctrl, Modifier::Shift]);
        // Note: literal char preserves case.
        assert_eq!(s.chords[0].key, Key::Char('V'));
    }

    #[test]
    fn parse_super_synonyms_resolve_identically() {
        for s in ["super+l", "logo+l", "win+l", "meta+l", "cmd+l"] {
            let parsed = ks(s);
            assert_eq!(parsed.chords.len(), 1);
            assert_eq!(parsed.chords[0].modifiers, vec![Modifier::Super]);
            assert_eq!(parsed.chords[0].key, Key::Char('l'));
        }
    }

    #[test]
    fn parse_handles_extra_whitespace() {
        let s = ks("  ctrl+v   ctrl+s  ");
        assert_eq!(s.chords.len(), 2);
    }

    #[test]
    fn parse_default_keystroke_constant() {
        let s = ks(DEFAULT_KEYSTROKE);
        assert_eq!(s.chords.len(), 1);
        assert_eq!(s.chords[0].modifiers, vec![Modifier::Ctrl]);
        assert_eq!(s.chords[0].key, Key::Char('v'));
    }

    // -----------------------------------------------------------------
    // parse() — error path
    // -----------------------------------------------------------------

    #[test]
    fn parse_empty_input_errors() {
        assert_eq!(parse(""), Err(ParseError::Empty));
        assert_eq!(parse("   "), Err(ParseError::Empty));
        assert_eq!(parse("\t\n"), Err(ParseError::Empty));
    }

    #[test]
    fn parse_dangling_plus_errors() {
        assert_eq!(
            parse("ctrl+"),
            Err(ParseError::DanglingPlus {
                fragment: "ctrl+".to_string()
            })
        );
        assert_eq!(
            parse("ctrl+shift+"),
            Err(ParseError::DanglingPlus {
                fragment: "ctrl+shift+".to_string()
            })
        );
    }

    #[test]
    fn parse_unknown_modifier_errors() {
        // `hyper` is not a known modifier. Rule-2 entry succeeded
        // (first segment IS a modifier — `ctrl`), but the middle
        // segment isn't.
        assert_eq!(
            parse("ctrl+hyper+v"),
            Err(ParseError::UnknownModifier {
                fragment: "ctrl+hyper+v".to_string(),
                token: "hyper".to_string(),
            })
        );
    }

    #[test]
    fn parse_modifier_as_key_errors() {
        // `ctrl+ctrl` would otherwise look like Ctrl-pressed-while-
        // pressing-ctrl which is meaningless.
        assert_eq!(
            parse("ctrl+ctrl"),
            Err(ParseError::ModifierAsKey {
                fragment: "ctrl+ctrl".to_string(),
                token: "ctrl".to_string(),
            })
        );
    }

    #[test]
    fn parse_unknown_multichar_key_errors() {
        // `ctrl+abc` — `abc` is multi-char and not a known special.
        assert_eq!(
            parse("ctrl+abc"),
            Err(ParseError::UnknownKey {
                fragment: "ctrl+abc".to_string(),
                token: "abc".to_string(),
            })
        );
    }

    // -----------------------------------------------------------------
    // From/Display round-trip
    // -----------------------------------------------------------------

    #[test]
    fn display_round_trips_for_canonical_input() {
        for input in [
            "ctrl+v",
            "ctrl+shift+v",
            "shift+insert",
            "enter",
            "ctrl+enter",
        ] {
            let s: KeystrokeSequence = input.parse().unwrap();
            assert_eq!(s.to_string(), input, "round-trip for {}", input);
        }
    }

    #[test]
    fn display_round_trips_for_two_chord_sequence() {
        let s: KeystrokeSequence = "ctrl+v ctrl+s".parse().unwrap();
        assert_eq!(s.to_string(), "ctrl+v ctrl+s");
    }

    #[test]
    fn display_for_literal_sequence_preserves_chars() {
        // The vim case: parse `"+gp` → display "\" + g p" (4
        // space-separated chords). Round-trip is asymmetric for this
        // input (the parsed form is canonical, the original isn't).
        let s: KeystrokeSequence = "\"+gp".parse().unwrap();
        assert_eq!(s.to_string(), "\" + g p");
    }

    // -----------------------------------------------------------------
    // KeystrokeSequence helpers
    // -----------------------------------------------------------------

    #[test]
    fn from_str_trait_works() {
        let s: KeystrokeSequence = "ctrl+v".parse().unwrap();
        assert_eq!(s.len(), 1);
        assert!(!s.is_empty());
    }

    #[test]
    fn default_is_empty() {
        let s = KeystrokeSequence::default();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }
}
