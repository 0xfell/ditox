//! End-to-end test that the `Watcher` consumes `CaptureSource`
//! correctly: clip injected via a mock source ends up as a row in the
//! database, dedup short-circuits repeats, and image priority wins
//! over text in the same clip.

#![cfg(unix)]

use ditox_core::capture::{CaptureSource, RawClip, RawFormat};
use ditox_core::config::{CaptureExcludeConfig, Config};
use ditox_core::db::{data_dir_override, set_data_dir_override, Database};
use ditox_core::foreground::{
    ForegroundId, ForegroundSnapshot, ForegroundTracker, MockForegroundTracker,
};
use ditox_core::watcher::Watcher;
use ditox_core::EntryType;
use rusqlite::Connection;
use std::error::Error as StdError;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tempfile::TempDir;

// `set_data_dir_override` is process-wide; serialize tests that touch it.
static OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

fn reset_override() {
    let _ = set_data_dir_override(None);
}

/// Test capture source that pops one clip per `current_snapshot()`.
/// `MockCaptureSource` returns the same first-queued clip forever,
/// which doesn't match the watcher's "advance on change" semantics.
struct QueueSource {
    name: String,
    queue: Mutex<std::collections::VecDeque<RawClip>>,
}

impl QueueSource {
    fn new(name: &str, clips: Vec<RawClip>) -> Self {
        Self {
            name: name.to_string(),
            queue: Mutex::new(clips.into_iter().collect()),
        }
    }
}

impl CaptureSource for QueueSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn current_snapshot(&self) -> ditox_core::error::Result<Option<RawClip>> {
        Ok(self.queue.lock().unwrap().pop_front())
    }

    fn subscribe(&mut self) -> ditox_core::error::Result<mpsc::Receiver<RawClip>> {
        // Watcher uses snapshot, not subscribe; never called here.
        let (_tx, rx) = mpsc::channel();
        Ok(rx)
    }

    fn shutdown(&mut self) -> ditox_core::error::Result<()> {
        Ok(())
    }
}

fn setup_db() -> (TempDir, Database) {
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();
    // Sanity: the override took effect.
    assert_eq!(data_dir_override().as_deref(), Some(tmp.path()));
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    (tmp, db)
}

fn open_with_schema() -> Database {
    let db = Database::open().unwrap();
    db.init_schema().unwrap();
    db
}

#[test]
fn watcher_captures_text_from_mock_source() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    let source = Box::new(QueueSource::new(
        "test-source",
        vec![RawClip::text("hello world".to_string())],
    ));
    let config = Config::default();
    let mut watcher = Watcher::with_sources(db, config, vec![source]);

    let captured = watcher.poll_once()?;
    assert!(captured, "first clip should land");

    // Re-open the DB to read entries (the watcher owns the original
    // handle; we don't need it back).
    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_type, EntryType::Text);
    assert_eq!(entries[0].content, "hello world");

    reset_override();
    Ok(())
}

#[test]
fn watcher_dedups_repeated_clip_via_last_hash() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Same clip queued twice — second poll should short-circuit on
    // `last_hash`.
    let source = Box::new(QueueSource::new(
        "test-source",
        vec![
            RawClip::text("repeat me".to_string()),
            RawClip::text("repeat me".to_string()),
        ],
    ));
    let mut watcher = Watcher::with_sources(db, Config::default(), vec![source]);

    assert!(watcher.poll_once()?, "first clip captured");
    assert!(!watcher.poll_once()?, "duplicate short-circuited");

    let db2 = open_with_schema();
    assert_eq!(db2.get_all(1000)?.len(), 1);

    reset_override();
    Ok(())
}

#[test]
fn watcher_prefers_image_over_text_in_same_clip() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Mimic browser "Copy image": clipboard exposes both an image
    // payload and a text URL. Watcher should record the image.
    // 8-byte PNG header is enough for the path; storage doesn't
    // validate the bytes are a real PNG.
    let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let mixed = RawClip {
        captured_at: SystemTime::now(),
        source_app: None,
        formats: vec![
            RawFormat {
                mime: "text/plain;charset=utf-8".to_string(),
                bytes: b"https://example.com/cat.png".to_vec(),
            },
            RawFormat {
                mime: "image/png".to_string(),
                bytes: png_header,
            },
        ],
    };

    let source = Box::new(QueueSource::new("test-source", vec![mixed]));
    let mut watcher = Watcher::with_sources(db, Config::default(), vec![source]);
    assert!(watcher.poll_once()?, "image clip captured");

    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].entry_type,
        EntryType::Image,
        "image must win over text in mixed-format clip"
    );

    let conn = Connection::open(Database::get_db_path()?)?;
    let format_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entry_formats WHERE entry_id = ?1",
        [&entries[0].id],
        |row| row.get(0),
    )?;
    assert_eq!(
        format_count, 2,
        "image canonical row and text/plain extra must both be stored"
    );

    let text_search = db2.search_entries_in_format(
        "https://example.com/cat.png",
        "text/plain;charset=utf-8",
        10,
    )?;
    assert_eq!(
        text_search.len(),
        1,
        "stored text/plain extra should be searchable"
    );

    reset_override();
    Ok(())
}

#[test]
fn watcher_persists_all_allowed_formats_from_same_clip() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    let rich = RawClip {
        captured_at: SystemTime::now(),
        source_app: None,
        formats: vec![
            RawFormat {
                mime: "text/plain".to_string(),
                bytes: b"hello formatted".to_vec(),
            },
            RawFormat {
                mime: "text/html".to_string(),
                bytes: b"<p><strong>hello formatted</strong></p>".to_vec(),
            },
            RawFormat {
                mime: "text/rtf".to_string(),
                bytes: br"{\rtf1\rsid12345 hello formatted}".to_vec(),
            },
        ],
    };

    let source = Box::new(QueueSource::new("rich", vec![rich]));
    let mut watcher = Watcher::with_sources(db, Config::default(), vec![source]);
    assert!(watcher.poll_once()?, "rich text clip captured");

    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_type, EntryType::Text);
    assert_eq!(entries[0].content, "hello formatted");

    let conn = Connection::open(Database::get_db_path()?)?;
    let rows: Vec<(String, i64)> = conn
        .prepare(
            "SELECT format_name, canonical FROM entry_formats
             WHERE entry_id = ?1
             ORDER BY canonical DESC, format_name ASC",
        )?
        .query_map([&entries[0].id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    assert_eq!(
        rows,
        vec![
            ("text/plain;charset=utf-8".to_string(), 1),
            ("text/html".to_string(), 0),
            ("text/rtf".to_string(), 0),
        ]
    );

    let html = db2.search_entries_in_format("strong", "text/html", 10)?;
    assert_eq!(html.len(), 1, "HTML extra should feed format FTS");

    reset_override();
    Ok(())
}

#[test]
fn watcher_falls_through_to_second_source_when_first_empty() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Source 1 returns nothing; source 2 has a clip. Watcher should
    // pick source 2's clip on the same poll.
    let empty = Box::new(QueueSource::new("empty", vec![]));
    let backup = Box::new(QueueSource::new(
        "backup",
        vec![RawClip::text("fallback".to_string())],
    ));
    let mut watcher = Watcher::with_sources(db, Config::default(), vec![empty, backup]);

    assert!(watcher.poll_once()?, "fallback source used");
    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "fallback");

    reset_override();
    Ok(())
}

#[test]
fn watcher_first_source_wins_when_both_have_content() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Both sources have content; the first one in the list wins per
    // poll. This is the priority semantic Phase 1 will need for X11
    // (CLIPBOARD selection beats PRIMARY selection).
    let primary = Box::new(QueueSource::new(
        "primary",
        vec![RawClip::text("from-primary".to_string())],
    ));
    let secondary = Box::new(QueueSource::new(
        "secondary",
        vec![RawClip::text("from-secondary".to_string())],
    ));
    let mut watcher = Watcher::with_sources(db, Config::default(), vec![primary, secondary]);

    assert!(watcher.poll_once()?);
    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "from-primary");

    reset_override();
    Ok(())
}

#[test]
fn watcher_initialize_hash_primes_from_first_nonempty_source() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // The first source has content at startup. After
    // `initialize_hash` we should NOT capture that same content on
    // the next poll — that's bug #4 from the original watcher hunt.
    let queue = Arc::new(Mutex::new(std::collections::VecDeque::from(vec![
        // First read — initialize_hash takes this.
        RawClip::text("startup-content".to_string()),
        // Second read — poll_once sees it again and must dedup.
        RawClip::text("startup-content".to_string()),
    ])));

    struct SharedQueueSource(Arc<Mutex<std::collections::VecDeque<RawClip>>>);
    impl CaptureSource for SharedQueueSource {
        fn name(&self) -> &str {
            "shared"
        }
        fn current_snapshot(&self) -> ditox_core::error::Result<Option<RawClip>> {
            Ok(self.0.lock().unwrap().pop_front())
        }
        fn subscribe(&mut self) -> ditox_core::error::Result<mpsc::Receiver<RawClip>> {
            let (_tx, rx) = mpsc::channel();
            Ok(rx)
        }
        fn shutdown(&mut self) -> ditox_core::error::Result<()> {
            Ok(())
        }
    }

    let source: Box<dyn CaptureSource> = Box::new(SharedQueueSource(queue));
    let mut watcher = Watcher::with_sources(db, Config::default(), vec![source]);

    watcher.initialize_hash();
    let captured = watcher.poll_once()?;
    assert!(
        !captured,
        "must not capture content already on clipboard at startup"
    );

    let db2 = open_with_schema();
    assert_eq!(db2.get_all(1000)?.len(), 0);

    reset_override();
    Ok(())
}

// ============================================================================
// Per-app capture exclusion (Phase 3 sub-task 3.2)
// ============================================================================

fn make_snap(basename: &str) -> ForegroundSnapshot {
    ForegroundSnapshot {
        identifier: ForegroundId::Hypr {
            address: "0x1234".to_string(),
        },
        process_basename: basename.to_string(),
        title: format!("{basename} window"),
        captured_at: SystemTime::now(),
    }
}

fn config_with_exclude(patterns: Vec<&str>) -> Config {
    let mut c = Config::default();
    c.capture.exclude = CaptureExcludeConfig {
        processes: patterns.into_iter().map(String::from).collect(),
    };
    c
}

#[test]
fn watcher_skips_clip_when_foreground_matches_exclude() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // KeePassXC just briefly wrote credentials to the clipboard. The
    // watcher must drop the clip and never insert a row.
    let source = Box::new(QueueSource::new(
        "keepass-source",
        vec![RawClip::text("hunter2".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(Some(
        make_snap("org.keepassxc.KeePassXC"),
    )));
    let config = config_with_exclude(vec!["*KeePass*"]);

    let mut watcher = Watcher::with_sources_and_tracker(db, config, vec![source], tracker);

    let captured = watcher.poll_once()?;
    assert!(!captured, "clip must be dropped when foreground excluded");

    let db2 = open_with_schema();
    assert_eq!(
        db2.get_all(1000)?.len(),
        0,
        "no entry must reach the database for excluded apps"
    );

    reset_override();
    Ok(())
}

#[test]
fn watcher_captures_clip_when_foreground_not_in_exclude() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    let source = Box::new(QueueSource::new(
        "browser",
        vec![RawClip::text("hello from firefox".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> =
        Box::new(MockForegroundTracker::new(Some(make_snap("firefox"))));
    let config = config_with_exclude(vec!["*KeePass*", "*1Password*"]);

    let mut watcher = Watcher::with_sources_and_tracker(db, config, vec![source], tracker);

    let captured = watcher.poll_once()?;
    assert!(captured, "non-excluded foreground must allow capture");

    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "hello from firefox");

    reset_override();
    Ok(())
}

#[test]
fn watcher_captures_when_no_foreground_available() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Tracker returns None — typical of GNOME Wayland or platforms
    // where the tracker is a Noop. The watcher must capture rather
    // than silently drop everything just because foreground info is
    // unavailable.
    let source = Box::new(QueueSource::new(
        "noop-fg",
        vec![RawClip::text("captured anyway".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(None));
    let config = config_with_exclude(vec!["*KeePass*"]);

    let mut watcher = Watcher::with_sources_and_tracker(db, config, vec![source], tracker);

    let captured = watcher.poll_once()?;
    assert!(
        captured,
        "missing foreground info must NOT block capture (fail-open)"
    );

    let db2 = open_with_schema();
    assert_eq!(db2.get_all(1000)?.len(), 1);

    reset_override();
    Ok(())
}

#[test]
fn excluded_clip_does_not_advance_last_hash() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Scenario: KeePass clipboard for "shared-content" is excluded.
    // Then the same content "shared-content" appears via Firefox —
    // we must capture it. If we'd advanced `last_hash` on the
    // excluded poll, dedup would short-circuit the second poll.
    let source = Box::new(QueueSource::new(
        "alternating-fg",
        vec![
            RawClip::text("shared-content".to_string()),
            RawClip::text("shared-content".to_string()),
        ],
    ));
    let tracker = Arc::new(MockForegroundTracker::new(Some(make_snap("KeePassXC"))));
    let tracker_box: Box<dyn ForegroundTracker> = Box::new(MockForegroundTrackerHandle {
        inner: Arc::clone(&tracker),
    });
    let config = config_with_exclude(vec!["*KeePass*"]);

    let mut watcher = Watcher::with_sources_and_tracker(db, config, vec![source], tracker_box);

    // First poll: foreground = KeePassXC, clip dropped.
    assert!(!watcher.poll_once()?);

    // Switch foreground to Firefox.
    tracker.set_snapshot(Some(make_snap("firefox")));

    // Second poll: same content but allowed foreground. Must
    // capture (we'd fail this if `last_hash` had been bumped).
    assert!(watcher.poll_once()?);

    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "shared-content");

    reset_override();
    Ok(())
}

/// Adapter so tests can share a single `Arc<MockForegroundTracker>`
/// across the test body and the boxed-into-watcher tracker. The
/// watcher takes `Box<dyn ForegroundTracker>`; we wrap an Arc so
/// `set_snapshot` calls from the test body affect the watcher's view.
struct MockForegroundTrackerHandle {
    inner: Arc<MockForegroundTracker>,
}

impl ForegroundTracker for MockForegroundTrackerHandle {
    fn name(&self) -> &str {
        "mock-handle"
    }

    fn snapshot(&self) -> ditox_core::error::Result<Option<ForegroundSnapshot>> {
        self.inner.snapshot()
    }

    fn restore(&self, snap: &ForegroundSnapshot) -> ditox_core::error::Result<()> {
        self.inner.restore(snap)
    }

    fn subscribe(&mut self) -> ditox_core::error::Result<mpsc::Receiver<ForegroundSnapshot>> {
        Err(ditox_core::error::DitoxError::Other(
            "MockForegroundTrackerHandle does not support subscribe in this test".to_string(),
        ))
    }

    fn shutdown(&mut self) -> ditox_core::error::Result<()> {
        Ok(())
    }
}

// ============================================================================
// Filter rules (Phase 3 sub-task 3.4)
// ============================================================================

#[test]
fn watcher_drops_clip_matching_filter_rule() -> Result<(), Box<dyn StdError>> {
    use ditox_core::filter::{FilterAction, FilterRule, PatternKind};

    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Persist a filter rule that drops anything containing "secret".
    let rule = FilterRule::new_now(
        "drop-secrets",
        "secret",
        PatternKind::Contains,
        None,
        FilterAction::Drop,
        0,
    );
    db.add_filter_rule(&rule)?;

    let source = Box::new(QueueSource::new(
        "rule-source",
        vec![RawClip::text("a secret value".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(None));

    let mut watcher =
        Watcher::with_sources_and_tracker(db, Config::default(), vec![source], tracker);

    let captured = watcher.poll_once()?;
    assert!(!captured, "rule must drop matching clip");

    let db2 = open_with_schema();
    assert_eq!(
        db2.get_all(1000)?.len(),
        0,
        "no entry must reach the database when a drop rule matches"
    );

    reset_override();
    Ok(())
}

#[test]
fn watcher_captures_clip_not_matching_any_filter_rule() -> Result<(), Box<dyn StdError>> {
    use ditox_core::filter::{FilterAction, FilterRule, PatternKind};

    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    let rule = FilterRule::new_now(
        "drop-secrets",
        "secret",
        PatternKind::Contains,
        None,
        FilterAction::Drop,
        0,
    );
    db.add_filter_rule(&rule)?;

    let source = Box::new(QueueSource::new(
        "rule-pass",
        vec![RawClip::text("public information".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(None));

    let mut watcher =
        Watcher::with_sources_and_tracker(db, Config::default(), vec![source], tracker);

    let captured = watcher.poll_once()?;
    assert!(captured, "non-matching clip must pass through filter rules");

    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "public information");

    reset_override();
    Ok(())
}

#[test]
fn watcher_filter_rules_first_match_wins() -> Result<(), Box<dyn StdError>> {
    use ditox_core::filter::{FilterAction, FilterRule, PatternKind};

    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // First rule (position 0) drops; second rule (position 1) would
    // also match but is irrelevant — the engine stops after the
    // first match.
    let r1 = FilterRule::new_now(
        "drop-first",
        "value",
        PatternKind::Contains,
        None,
        FilterAction::Drop,
        0,
    );
    let r2 = FilterRule::new_now(
        "tag-second",
        "value",
        PatternKind::Contains,
        None,
        FilterAction::Tag("would-tag".to_string()),
        1,
    );
    db.add_filter_rule(&r1)?;
    db.add_filter_rule(&r2)?;

    let source = Box::new(QueueSource::new(
        "first-wins",
        vec![RawClip::text("a value to drop".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(None));

    let mut watcher =
        Watcher::with_sources_and_tracker(db, Config::default(), vec![source], tracker);

    assert!(!watcher.poll_once()?);
    let db2 = open_with_schema();
    assert_eq!(db2.get_all(1000)?.len(), 0);

    reset_override();
    Ok(())
}

#[test]
fn watcher_filter_rule_tag_action_tags_inserted_entry() -> Result<(), Box<dyn StdError>> {
    use ditox_core::filter::{FilterAction, FilterRule, PatternKind};

    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    let rule = FilterRule::new_now(
        "tag-rust",
        "cargo",
        PatternKind::Contains,
        None,
        FilterAction::Tag("rust".to_string()),
        0,
    );
    db.add_filter_rule(&rule)?;

    let source = Box::new(QueueSource::new(
        "tag-rule",
        vec![RawClip::text("cargo test --workspace".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(None));

    let mut watcher =
        Watcher::with_sources_and_tracker(db, Config::default(), vec![source], tracker);

    assert!(watcher.poll_once()?);
    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    let tags = db2.get_tags_for_entry(&entries[0].id)?;
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "rust");

    reset_override();
    Ok(())
}

#[test]
fn watcher_filter_rule_transform_action_mutates_text_before_insert() -> Result<(), Box<dyn StdError>>
{
    use ditox_core::filter::{FilterAction, FilterRule, PatternKind};

    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    let rule = FilterRule::new_now(
        "lowercase-captures",
        "SHOUT",
        PatternKind::Contains,
        None,
        FilterAction::Transform("lower-case".to_string()),
        0,
    );
    db.add_filter_rule(&rule)?;

    let source = Box::new(QueueSource::new(
        "transform-rule",
        vec![RawClip::text("SHOUT THIS".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> = Box::new(MockForegroundTracker::new(None));

    let mut watcher =
        Watcher::with_sources_and_tracker(db, Config::default(), vec![source], tracker);

    assert!(watcher.poll_once()?);
    let db2 = open_with_schema();
    let entries = db2.get_all(1000)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].content, "shout this",
        "transform:<id> filter action must alter captured text before insertion"
    );

    reset_override();
    Ok(())
}

#[test]
fn watcher_filter_rule_process_glob_restricts_match() -> Result<(), Box<dyn StdError>> {
    use ditox_core::filter::{FilterAction, FilterRule, PatternKind};

    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // Rule only fires when foreground basename matches *Term*.
    let rule = FilterRule::new_now(
        "scoped",
        "secret",
        PatternKind::Contains,
        Some("*Term*".to_string()),
        FilterAction::Drop,
        0,
    );
    db.add_filter_rule(&rule)?;

    // Tracker reports "Firefox" — does NOT match scope → rule should
    // not fire → clip captured.
    let source = Box::new(QueueSource::new(
        "scope-miss",
        vec![RawClip::text("a secret payload".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> =
        Box::new(MockForegroundTracker::new(Some(make_snap("Firefox"))));

    let mut watcher =
        Watcher::with_sources_and_tracker(db, Config::default(), vec![source], tracker);

    assert!(
        watcher.poll_once()?,
        "scoped rule must skip non-matching foreground"
    );
    let db2 = open_with_schema();
    assert_eq!(db2.get_all(1000)?.len(), 1);

    reset_override();
    Ok(())
}

#[test]
fn empty_exclude_list_skips_foreground_check() -> Result<(), Box<dyn StdError>> {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    reset_override();
    let (_tmp, db) = setup_db();

    // With an empty processes list, the watcher must not even call
    // `snapshot()` on the tracker — capture proceeds unconditionally.
    // We assert the behaviour indirectly: a tracker that errors on
    // snapshot would surface the error if we called it; here we use
    // a Mock that returns a basename that WOULD have matched if the
    // exclude list had wildcard-matched it. Since the list is empty,
    // we capture.
    let source = Box::new(QueueSource::new(
        "any",
        vec![RawClip::text("uncensored".to_string())],
    ));
    let tracker: Box<dyn ForegroundTracker> =
        Box::new(MockForegroundTracker::new(Some(make_snap("KeePassXC"))));
    let mut config = Config::default();
    config.capture.exclude.processes.clear();

    let mut watcher = Watcher::with_sources_and_tracker(db, config, vec![source], tracker);

    assert!(watcher.poll_once()?);
    let db2 = open_with_schema();
    assert_eq!(db2.get_all(1000)?.len(), 1);

    reset_override();
    Ok(())
}
