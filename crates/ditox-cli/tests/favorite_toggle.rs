mod common;
use common::TestEnv;

#[test]
fn favorite_unfavorite_flow() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    // add
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .arg("add")
        .write_stdin("star me")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = String::from_utf8(out).unwrap();
    let id = id.trim().trim_start_matches("added ").to_string();
    // favorite
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["favorite", &id])
        .assert()
        .success();
    let favs = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["list", "--favorites", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let j: serde_json::Value = serde_json::from_str(&favs).unwrap();
    assert!(j
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e.get("id").unwrap().as_str().unwrap() == id));
    // unfavorite
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["unfavorite", &id])
        .assert()
        .success();
    let favs2 = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["list", "--favorites", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let j2: serde_json::Value = serde_json::from_str(&favs2).unwrap();
    assert!(!j2
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e.get("id").unwrap().as_str().unwrap() == id));
}
