use assert_cmd::Command;
use predicates::prelude::*;
// use std::process;
use std::env;
use std::fs;
use tempfile::tempdir;

fn bin() -> Command {
    let mut c = Command::cargo_bin("ditox-cli").unwrap();
    c.arg("--store").arg("sqlite");
    c
}

#[test]
fn text_flow() {
    let dir = tempdir().unwrap();
    let cfg = dir.path().join("cfg");
    std::fs::create_dir_all(&cfg).unwrap();
    env::set_var("XDG_CONFIG_HOME", &cfg);
    let db = dir.path().join("ditox.db");

    // init
    bin().arg("--db").arg(&db).arg("init-db").assert().success();

    // add via stdin
    let mut cmd = bin();
    let assert = cmd
        .arg("--db")
        .arg(&db)
        .arg("add")
        .write_stdin("hello test")
        .assert();
    assert.success().stdout(predicate::str::contains("added "));

    // list json
    let output = bin()
        .arg("--db")
        .arg(&db)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(!v.as_array().unwrap().is_empty());

    // search json
    let output = bin()
        .arg("--db")
        .arg(&db)
        .args(["search", "hello", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(!v.as_array().unwrap().is_empty());
}

#[test]
fn image_from_file() {
    let dir = tempdir().unwrap();
    let cfg = dir.path().join("cfg");
    std::fs::create_dir_all(&cfg).unwrap();
    env::set_var("XDG_CONFIG_HOME", &cfg);
    let db = dir.path().join("ditox.db");
    bin().arg("--db").arg(&db).arg("init-db").assert().success();

    // decode tiny PNG fixture
    let img_path = dir.path().join("tiny.png");
    use base64::{engine::general_purpose, Engine as _};
    let b64 = include_str!("fixtures/tiny.png.b64");
    let bytes = general_purpose::STANDARD.decode(b64.trim()).unwrap();
    fs::write(&img_path, &bytes).unwrap();

    bin()
        .arg("--db")
        .arg(&db)
        .args(["add", "--image-path"])
        .arg(&img_path)
        .assert()
        .success();

    let out = bin()
        .arg("--db")
        .arg(&db)
        .args(["list", "--images", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let arr = v.as_array().unwrap();
    assert!(!arr.is_empty());

    // info for first id
    // Extract id field from first list output by re-listing without json
    let text = String::from_utf8(
        bin()
            .arg("--db")
            .arg(&db)
            .args(["list", "--images"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let first_id = text.split_whitespace().next().unwrap().to_string();
    let info = String::from_utf8(
        bin()
            .arg("--db")
            .arg(&db)
            .args(["info", &first_id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(info.contains("kind:\timage"));
}

// Removed: replaced by more robust tests in favorite_toggle.rs and prune_age.rs

// Removed: legacy doctor behavior test that assumed image-path checks.
// Current `doctor` focuses on clipboard tools and FTS capability; see doctor.rs.

#[test]
#[cfg(target_os = "linux")]
fn clipboard_image_copy_guarded() {
    if std::env::var("DITOX_E2E_CLIPBOARD").ok().as_deref() != Some("1") {
        return;
    }
    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        return;
    }

    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");
    bin().arg("--db").arg(&db).arg("init-db").assert().success();

    // decode tiny PNG fixture and add
    let img_path = dir.path().join("tiny.png");
    use base64::{engine::general_purpose, Engine as _};
    let b64 = include_str!("fixtures/tiny.png.b64");
    let bytes = general_purpose::STANDARD.decode(b64.trim()).unwrap();
    fs::write(&img_path, &bytes).unwrap();
    bin()
        .arg("--db")
        .arg(&db)
        .args(["add", "--image-path"])
        .arg(&img_path)
        .assert()
        .success();

    // get id
    let out = String::from_utf8(
        bin()
            .arg("--db")
            .arg(&db)
            .args(["list", "--images"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let id = out.split_whitespace().next().unwrap().to_string();
    // copy and read back via arboard
    bin()
        .arg("--db")
        .arg(&db)
        .args(["copy"])
        .arg(&id)
        .assert()
        .success();
    let mut cb = arboard::Clipboard::new().unwrap();
    let img = cb.get_image().unwrap();
    assert!(img.width > 0 && img.height > 0);
}
