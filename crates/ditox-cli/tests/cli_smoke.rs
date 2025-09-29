use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
// use std::process;
use std::fs;
use tempfile::tempdir;

fn bin() -> Command {
    Command::cargo_bin("ditox-cli").unwrap()
}

#[test]
fn text_flow() {
    let dir = tempdir().unwrap();
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
    assert!(v.as_array().unwrap().len() >= 1);

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
    assert!(v.as_array().unwrap().len() >= 1);
}

#[test]
fn image_from_file() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");
    bin().arg("--db").arg(&db).arg("init-db").assert().success();

    // decode tiny PNG fixture
    let img_path = dir.path().join("tiny.png");
    let b64 = include_str!("fixtures/tiny.png.b64");
    let bytes = base64::decode(b64.trim()).unwrap();
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

#[test]
fn favorites_and_prune() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");
    bin().arg("--db").arg(&db).arg("init-db").assert().success();
    // add 5 entries
    for i in 0..5 {
        bin()
            .arg("--db")
            .arg(&db)
            .args(["add"])
            .write_stdin(format!("item{}", i))
            .assert()
            .success();
    }
    // list json schema check
    let output = bin()
        .arg("--db")
        .arg(&db)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let items: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let a0 = items.as_array().unwrap();
    let first = &a0[0];
    assert!(first.get("id").and_then(|v| v.as_str()).is_some());
    assert!(first.get("text").and_then(|v| v.as_str()).is_some());
    assert!(first.get("kind").and_then(|v| v.as_str()).is_some());
    // favorite first id
    let first_id = first.get("id").unwrap().as_str().unwrap();
    bin()
        .arg("--db")
        .arg(&db)
        .args(["favorite", first_id])
        .assert()
        .success();
    // list favorites only
    let favs = bin()
        .arg("--db")
        .arg(&db)
        .args(["list", "--json", "--favorites"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let fav_items: serde_json::Value = serde_json::from_slice(&favs).unwrap();
    assert!(fav_items
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.get("id").unwrap().as_str().unwrap() == first_id));
    // prune to 2 non-favorite items (favorites are preserved by default)
    bin()
        .arg("--db")
        .arg(&db)
        .args(["prune", "--max-items", "2"])
        .assert()
        .success();
    let after = bin()
        .arg("--db")
        .arg(&db)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let after_v: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert!(after_v.as_array().unwrap().len() >= 2);

    // Ensure favorites survive; prune non-favorites to 0 so only favorite remains
    bin()
        .arg("--db")
        .arg(&db)
        .args(["prune", "--max-items", "0", "--keep-favorites"])
        .assert()
        .success();
    let after2 = bin()
        .arg("--db")
        .arg(&db)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let after2_v: serde_json::Value = serde_json::from_slice(&after2).unwrap();
    assert_eq!(after2_v.as_array().unwrap().len(), 1);
    assert_eq!(
        after2_v.as_array().unwrap()[0]
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        first_id
    );
}

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
    let b64 = include_str!("fixtures/tiny.png.b64");
    let bytes = base64::decode(b64.trim()).unwrap();
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
