mod common;
use common::TestEnv;

#[test]
fn prune_by_zero_age_keeps_only_favorites() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    // add three
    let mut ids = Vec::new();
    for s in ["a", "b", "c"] {
        let id = String::from_utf8(
            t.bin()
                .arg("--db")
                .arg(&t.db)
                .arg("add")
                .write_stdin(s)
                .assert()
                .success()
                .get_output()
                .stdout
                .clone(),
        )
        .unwrap();
        ids.push(id.trim().trim_start_matches("added ").to_string());
    }
    // favorite the middle one
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["favorite", &ids[1]])
        .assert()
        .success();
    // prune with age 0s
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["prune", "--max-age", "0s", "--keep-favorites"])
        .assert()
        .success();
    // only favorite should remain
    let out = String::from_utf8(
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
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(
        v.as_array().unwrap()[0]
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        ids[1]
    );
}
