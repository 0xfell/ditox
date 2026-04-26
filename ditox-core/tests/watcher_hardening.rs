//! Tests for watcher daemon hardening (task 016): flock-based
//! single-instance, heartbeat freshness, status probes, stop signal.

#![cfg(unix)]

use ditox_core::db::set_data_dir_override;
use ditox_core::watcher::{
    get_heartbeat_file_path, get_lock_file_path, get_pid_file_path, stop_watcher, watcher_status,
    HEARTBEAT_STALE_AFTER_SECS,
};
use fs2::FileExt;
use std::fs::OpenOptions;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

static OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn status_when_no_watcher_running() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let st = watcher_status();
    assert_eq!(st.pid, None);
    assert!(!st.pid_alive);
    assert!(!st.locked);
    assert_eq!(st.last_heartbeat, None);
    assert!(!st.healthy);

    set_data_dir_override(None).unwrap();
}

#[test]
fn stop_watcher_no_op_when_no_pid() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let result = stop_watcher().unwrap();
    assert!(!result, "stop_watcher should return false when no daemon");

    set_data_dir_override(None).unwrap();
}

#[test]
fn stale_pid_file_is_cleaned_up_by_stop() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    // Write a PID that almost certainly doesn't correspond to a real process.
    let pid_path = get_pid_file_path().unwrap();
    std::fs::write(&pid_path, "999999").unwrap();

    let result = stop_watcher().unwrap();
    assert!(!result, "stale PID should not count as stopped");
    assert!(
        !pid_path.exists(),
        "stop_watcher should clean up stale PID file"
    );

    set_data_dir_override(None).unwrap();
}

#[test]
fn lock_held_detection() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let lock_path = get_lock_file_path().unwrap();

    // Take the lock from this test process.
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .unwrap();
    f.try_lock_exclusive().expect("acquire lock");

    let st = watcher_status();
    assert!(st.locked, "watcher_status should detect held lock");

    FileExt::unlock(&f).expect("unlock");
    let st = watcher_status();
    assert!(!st.locked, "after unlock, lock should not be detected");

    set_data_dir_override(None).unwrap();
}

#[test]
fn heartbeat_freshness_check() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let hb_path = get_heartbeat_file_path().unwrap();

    // Write a fresh heartbeat (now). Without a live PID, status is
    // still not healthy.
    std::fs::write(&hb_path, now_secs().to_string()).unwrap();
    let st = watcher_status();
    assert!(st.last_heartbeat.is_some());
    assert!(st.heartbeat_age_secs.unwrap() < 5);
    assert!(!st.healthy, "no PID alive => not healthy");

    // Write a stale heartbeat.
    let stale = now_secs() - HEARTBEAT_STALE_AFTER_SECS - 10;
    std::fs::write(&hb_path, stale.to_string()).unwrap();
    let st = watcher_status();
    assert!(st.heartbeat_age_secs.unwrap() > HEARTBEAT_STALE_AFTER_SECS);
    assert!(!st.healthy);

    set_data_dir_override(None).unwrap();
}

#[test]
fn invalid_pid_file_yields_none() {
    let _g = OVERRIDE_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    set_data_dir_override(Some(tmp.path().to_path_buf())).unwrap();

    let pid_path = get_pid_file_path().unwrap();
    std::fs::write(&pid_path, "not-a-number").unwrap();

    let st = watcher_status();
    assert_eq!(st.pid, None);

    set_data_dir_override(None).unwrap();
}
