//! User-managed capture-time filter rules (Phase 3 sub-task 3.4).
//!
//! The watcher evaluates filter rules immediately after the per-app
//! exclusion check (sub-task 3.2) and before the dedup / sentinel
//! checks. Rules are stored in the `filter_rules` SQL table (schema
//! v4) and managed via the [`crate::db::Database`] CRUD helpers
//! and the `ditox rules` CLI subcommand.
//!
//! ## Rule structure
//!
//! ```ignore
//! FilterRule {
//!     id: String,                // uuid v4
//!     name: String,              // user-facing label
//!     pattern: String,           // regex / glob / substring
//!     pattern_kind: PatternKind,
//!     process_glob: Option<String>, // optional process scope
//!     action: FilterAction,      // Drop | Transform(id) | Tag(name)
//!     enabled: bool,
//!     position: i64,             // evaluation order; lower first
//!     created_at: String,        // ISO 8601
//! }
//! ```
//!
//! ## Evaluation
//!
//! [`FilterEngine::evaluate`] walks enabled rules in `position`
//! order and returns the first match's [`FilterAction`]. Match
//! requires:
//!
//! 1. The text matches the rule's pattern.
//! 2. If `process_glob` is `Some`, the foreground app's
//!    `process_basename` matches it (uses the same
//!    [`crate::config::glob_match`] helper as `[capture.exclude]`).
//!
//! ## Action semantics
//!
//! - `Drop` — the watcher returns `Ok(false)` without inserting
//!   and **without advancing `last_hash`** so a later identical
//!   clip from a non-matching context still captures.
//! - `Transform(id)` — applies the named text transform from
//!   [`crate::transforms::registry`] to the clip's text before
//!   insertion. Only meaningful for text entries; image entries
//!   skip the transform with a warning.
//! - `Tag(name)` — applies a tag to the inserted entry. Tags
//!   themselves are a Phase 4b feature; this action is parsed and
//!   stored today but the watcher logs and skips it until tags
//!   land. Documented in the doc comment.

use crate::error::{DitoxError, Result};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// How [`FilterRule::pattern`] is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternKind {
    /// `regex::Regex` syntax. Compiled with case-insensitive +
    /// size-limit caps so a malicious pattern can't DoS the
    /// watcher.
    Regex,
    /// Glob with `*` (zero-or-more chars) and `?` (one char).
    /// ASCII case-insensitive. Reuses
    /// [`crate::config::glob_match`].
    Glob,
    /// Plain substring (`text.contains(pattern)`). ASCII
    /// case-insensitive.
    Contains,
}

impl PatternKind {
    /// Parse from the canonical lowercase name used in the DB
    /// `pattern_kind` TEXT column and the CLI flag.
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "regex" => Self::Regex,
            "glob" => Self::Glob,
            "contains" => Self::Contains,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Regex => "regex",
            Self::Glob => "glob",
            Self::Contains => "contains",
        }
    }
}

/// What to do when a rule matches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum FilterAction {
    /// Discard the clip. Most common action.
    Drop,
    /// Apply a transform from
    /// [`crate::transforms::registry`] before insertion.
    /// `String` is the transform's id (e.g. `"upper-case"`,
    /// `"slugify"`).
    Transform(String),
    /// Apply a tag with the given name. Tags are Phase 4b; today
    /// this action is parsed but unimplemented (watcher logs and
    /// captures normally).
    Tag(String),
}

impl FilterAction {
    /// Serialise to the canonical TEXT representation stored in
    /// the DB and accepted by the CLI:
    ///
    /// ```text
    /// "drop"
    /// "transform:<transform-id>"
    /// "tag:<tag-name>"
    /// ```
    pub fn to_storage(&self) -> String {
        match self {
            Self::Drop => "drop".to_string(),
            Self::Transform(id) => format!("transform:{id}"),
            Self::Tag(name) => format!("tag:{name}"),
        }
    }

    /// Parse from the canonical TEXT representation. Errors on
    /// unknown action prefixes.
    pub fn from_storage(s: &str) -> Result<Self> {
        match s {
            "drop" => Ok(Self::Drop),
            other if other.starts_with("transform:") => Ok(Self::Transform(
                other.trim_start_matches("transform:").to_string(),
            )),
            other if other.starts_with("tag:") => {
                Ok(Self::Tag(other.trim_start_matches("tag:").to_string()))
            }
            other => Err(DitoxError::Other(format!(
                "filter action '{other}' not recognised; expected drop / transform:<id> / tag:<name>"
            ))),
        }
    }
}

/// One row of `filter_rules`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub pattern_kind: PatternKind,
    pub process_glob: Option<String>,
    pub action: FilterAction,
    pub enabled: bool,
    pub position: i64,
    pub created_at: String,
}

impl FilterRule {
    /// Build a new rule with a fresh UUID and `created_at = now`.
    /// Position defaults to `0`; callers that want the rule at
    /// the end of the chain pass `position = current_max + 1`.
    pub fn new_now(
        name: impl Into<String>,
        pattern: impl Into<String>,
        pattern_kind: PatternKind,
        process_glob: Option<String>,
        action: FilterAction,
        position: i64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            pattern: pattern.into(),
            pattern_kind,
            process_glob,
            action,
            enabled: true,
            position,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Wall-clock at creation. Convenience for tests / display.
    pub fn created_at_systime(&self) -> Option<SystemTime> {
        chrono::DateTime::parse_from_rfc3339(&self.created_at)
            .ok()
            .map(SystemTime::from)
    }
}

/// Compiled-once view of a [`FilterRule`]. Holds a precompiled
/// regex when the kind is `Regex`; lookup is cheap thereafter.
struct CompiledRule {
    rule: FilterRule,
    regex: Option<regex::Regex>,
}

impl CompiledRule {
    fn compile(rule: FilterRule) -> Result<Self> {
        let regex = if rule.pattern_kind == PatternKind::Regex {
            // Defence-in-depth: cap the compiled regex's size and
            // DFA size so a malicious pattern can't OOM us. The
            // `regex` crate's defaults are 10 MiB / 2 MiB; we keep
            // the size_limit at the default and explicitly cap
            // dfa_size_limit at 4 MiB so worst-case match-time
            // memory stays bounded even for adversarial patterns.
            let re = regex::RegexBuilder::new(&rule.pattern)
                .case_insensitive(true)
                .dfa_size_limit(4 * 1024 * 1024)
                .build()
                .map_err(|e| {
                    DitoxError::Other(format!("filter rule '{}' regex invalid: {e}", rule.name))
                })?;
            Some(re)
        } else {
            None
        };
        Ok(Self { rule, regex })
    }

    /// Returns true iff this rule matches the given text + process
    /// basename context.
    fn matches(&self, text: &str, process_basename: Option<&str>) -> bool {
        // Process scope: if `process_glob` is set, the foreground
        // basename must match. If we have no foreground info
        // (None) and the rule has a process_glob, we conservatively
        // skip the rule (can't confirm the scope).
        if let Some(scope) = self.rule.process_glob.as_ref() {
            let Some(basename) = process_basename else {
                return false;
            };
            if !crate::config::glob_match(scope, basename) {
                return false;
            }
        }

        // Pattern match.
        match self.rule.pattern_kind {
            PatternKind::Regex => self
                .regex
                .as_ref()
                .map(|r| r.is_match(text))
                .unwrap_or(false),
            PatternKind::Glob => crate::config::glob_match(&self.rule.pattern, text),
            PatternKind::Contains => {
                let needle = self.rule.pattern.to_ascii_lowercase();
                let hay = text.to_ascii_lowercase();
                hay.contains(&needle)
            }
        }
    }
}

/// Compiled rule set evaluated at capture time. Built from a
/// `Vec<FilterRule>` (typically loaded from the DB) and used
/// repeatedly across many clips. `evaluate` returns the first
/// matching rule's action; if no rule matches, returns `None`
/// (capture proceeds normally).
pub struct FilterEngine {
    rules: Vec<CompiledRule>,
}

impl FilterEngine {
    /// Build an engine from `rules`. Disabled rules are filtered
    /// out at construction time. Rules with invalid regex are
    /// skipped with a `warn` log so a single bad rule doesn't
    /// take down the whole engine.
    pub fn from_rules(rules: Vec<FilterRule>) -> Self {
        let mut compiled = Vec::with_capacity(rules.len());
        for rule in rules {
            if !rule.enabled {
                continue;
            }
            let name = rule.name.clone();
            match CompiledRule::compile(rule) {
                Ok(c) => compiled.push(c),
                Err(e) => {
                    tracing::warn!(
                        rule = %name,
                        error = %e,
                        "skipping filter rule (compile error)"
                    );
                }
            }
        }
        // Sort by position ascending so the engine evaluates in
        // user-specified order regardless of DB row order.
        compiled.sort_by_key(|c| c.rule.position);
        Self { rules: compiled }
    }

    /// Evaluate against `text` and an optional foreground
    /// `process_basename`. Returns `Some(MatchedRule { rule, action })`
    /// for the first matching rule, or `None` if no rule matches.
    pub fn evaluate(&self, text: &str, process_basename: Option<&str>) -> Option<MatchedRule<'_>> {
        for compiled in &self.rules {
            if compiled.matches(text, process_basename) {
                return Some(MatchedRule {
                    rule: &compiled.rule,
                });
            }
        }
        None
    }

    /// True iff there are no enabled rules. Lets the watcher
    /// short-circuit the evaluation without touching `self`.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Borrowed view of a matched rule. Carries a reference into the
/// engine so callers can read the action without cloning. If a
/// caller needs ownership (e.g. to log + stash for later) they
/// `.rule.clone()`.
pub struct MatchedRule<'a> {
    pub rule: &'a FilterRule,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop_rule(name: &str, pattern: &str, kind: PatternKind, position: i64) -> FilterRule {
        FilterRule::new_now(name, pattern, kind, None, FilterAction::Drop, position)
    }

    fn drop_rule_for(
        name: &str,
        pattern: &str,
        kind: PatternKind,
        process: &str,
        position: i64,
    ) -> FilterRule {
        FilterRule::new_now(
            name,
            pattern,
            kind,
            Some(process.to_string()),
            FilterAction::Drop,
            position,
        )
    }

    // -- PatternKind --

    #[test]
    fn pattern_kind_round_trip() {
        for k in [PatternKind::Regex, PatternKind::Glob, PatternKind::Contains] {
            assert_eq!(PatternKind::from_str_lossy(k.as_str()), Some(k));
        }
    }

    #[test]
    fn pattern_kind_case_insensitive() {
        assert_eq!(
            PatternKind::from_str_lossy("REGEX"),
            Some(PatternKind::Regex)
        );
        assert_eq!(PatternKind::from_str_lossy("Glob"), Some(PatternKind::Glob));
    }

    #[test]
    fn pattern_kind_unknown_returns_none() {
        assert_eq!(PatternKind::from_str_lossy("substring"), None);
        assert_eq!(PatternKind::from_str_lossy(""), None);
    }

    // -- FilterAction --

    #[test]
    fn filter_action_drop_round_trip() {
        let a = FilterAction::Drop;
        assert_eq!(a.to_storage(), "drop");
        assert_eq!(FilterAction::from_storage("drop").unwrap(), a);
    }

    #[test]
    fn filter_action_transform_round_trip() {
        let a = FilterAction::Transform("upper-case".to_string());
        assert_eq!(a.to_storage(), "transform:upper-case");
        assert_eq!(
            FilterAction::from_storage("transform:upper-case").unwrap(),
            a
        );
    }

    #[test]
    fn filter_action_tag_round_trip() {
        let a = FilterAction::Tag("secret".to_string());
        assert_eq!(a.to_storage(), "tag:secret");
        assert_eq!(FilterAction::from_storage("tag:secret").unwrap(), a);
    }

    #[test]
    fn filter_action_from_storage_rejects_unknown() {
        assert!(FilterAction::from_storage("garbage").is_err());
        assert!(FilterAction::from_storage("xfm:foo").is_err());
        assert!(FilterAction::from_storage("").is_err());
    }

    // -- FilterRule --

    #[test]
    fn filter_rule_new_now_assigns_uuid_and_timestamp() {
        let r = FilterRule::new_now(
            "test",
            "pwd",
            PatternKind::Contains,
            None,
            FilterAction::Drop,
            0,
        );
        assert!(uuid::Uuid::parse_str(&r.id).is_ok());
        assert!(r.enabled);
        assert!(r.created_at_systime().is_some());
    }

    // -- FilterEngine --

    #[test]
    fn engine_drops_disabled_rules() {
        let mut r = drop_rule("d", "x", PatternKind::Contains, 0);
        r.enabled = false;
        let e = FilterEngine::from_rules(vec![r]);
        assert!(e.is_empty());
    }

    #[test]
    fn engine_first_match_wins_by_position() {
        let r1 = drop_rule("first", "foo", PatternKind::Contains, 1);
        let r2 = drop_rule("second", "foo", PatternKind::Contains, 0);
        // r2 has lower position → should win.
        let e = FilterEngine::from_rules(vec![r1, r2]);
        let m = e.evaluate("hello foo", None).expect("must match");
        assert_eq!(m.rule.name, "second");
    }

    #[test]
    fn engine_contains_match() {
        let r = drop_rule("p", "password", PatternKind::Contains, 0);
        let e = FilterEngine::from_rules(vec![r]);
        assert!(e.evaluate("my password is hunter2", None).is_some());
        assert!(e.evaluate("My PASSWORD here", None).is_some()); // case-insensitive
        assert!(e.evaluate("nothing here", None).is_none());
    }

    #[test]
    fn engine_glob_match() {
        let r = drop_rule("g", "*secret*", PatternKind::Glob, 0);
        let e = FilterEngine::from_rules(vec![r]);
        assert!(e.evaluate("a secret value", None).is_some());
        assert!(e.evaluate("MY SECRETS", None).is_some()); // case-insensitive
        assert!(e.evaluate("public info", None).is_none());
    }

    #[test]
    fn engine_regex_match() {
        let r = drop_rule("r", r"^[A-Z]{2,}-\d{4,}$", PatternKind::Regex, 0);
        let e = FilterEngine::from_rules(vec![r]);
        // Case-insensitive flag is on, so lowercase "abc-1234" matches too.
        assert!(e.evaluate("ABC-1234", None).is_some());
        assert!(e.evaluate("abc-1234", None).is_some());
        assert!(e.evaluate("not a ticket id", None).is_none());
    }

    #[test]
    fn engine_invalid_regex_is_skipped_not_panicking() {
        let r = drop_rule("bad", "[unclosed", PatternKind::Regex, 0);
        // No panic; rule is silently dropped by the engine.
        let e = FilterEngine::from_rules(vec![r]);
        assert!(e.is_empty());
    }

    #[test]
    fn engine_process_glob_restricts_match() {
        let r = drop_rule_for("p", "secret", PatternKind::Contains, "*KeePass*", 0);
        let e = FilterEngine::from_rules(vec![r]);
        // Process matches → rule fires.
        assert!(e.evaluate("a secret value", Some("KeePassXC")).is_some());
        // Process doesn't match → rule skipped.
        assert!(e.evaluate("a secret value", Some("firefox")).is_none());
        // No process info → rule conservatively skipped.
        assert!(e.evaluate("a secret value", None).is_none());
    }

    #[test]
    fn engine_no_rules_is_empty() {
        let e = FilterEngine::from_rules(vec![]);
        assert!(e.is_empty());
        assert!(e.evaluate("anything", None).is_none());
    }

    #[test]
    fn engine_returns_none_when_no_match() {
        let r = drop_rule("p", "xyz", PatternKind::Contains, 0);
        let e = FilterEngine::from_rules(vec![r]);
        assert!(e.evaluate("nothing matching", None).is_none());
    }
}
