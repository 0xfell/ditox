mod common;
use common::TestEnv;

#[test]
fn search_json_returns_expected() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    // add
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("add")
        .write_stdin("the quick brown fox")
        .assert()
        .success();
    // search
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .args(["search", "quick", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(v.as_array().unwrap().iter().any(|e| {
        e.get("text")
            .and_then(|s| s.as_str())
            .map(|s| s.contains("quick"))
            .unwrap_or(false)
    }));
}
