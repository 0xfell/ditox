mod common;
use common::TestEnv;

#[test]
fn delete_and_clear() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    // add two
    let mut ids = Vec::new();
    for s in ["one", "two"] {
        let out = t
            .bin()
            .arg("--db")
            .arg(&t.db)
            .arg("add")
            .write_stdin(s)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let id = String::from_utf8(out).unwrap();
        ids.push(id.trim().trim_start_matches("added ").to_string());
    }
    // delete first
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["delete", &ids[0]])
        .assert()
        .success();
    let after = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["list"]) // plain list
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(!after.contains(&ids[0]));
    assert!(after.contains(&ids[1]));

    // clear
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("delete")
        .assert()
        .success();
    let after2 = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["list", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&after2).unwrap();
    assert!(v.as_array().unwrap().is_empty());
}
