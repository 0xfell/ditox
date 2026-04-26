//! End-to-end test that the `Watcher` consumes `CaptureSource`
//! correctly: clip injected via a mock source ends up as a row in the
//! database, dedup short-circuits repeats, and image priority wins
//! over text in the same clip.

#![cfg(unix)]

use ditox_core::capture::{CaptureSource, RawClip, RawFormat};
use ditox_core::config::Config;
use ditox_core::db::{data_dir_override, set_data_dir_override, Database};
use ditox_core::watcher::Watcher;
use ditox_core::EntryType;
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
