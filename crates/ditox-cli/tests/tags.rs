mod common;
use common::TestEnv;

#[test]
fn tag_add_ls_rm_flow() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    // add a clip
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .arg("add")
        .write_stdin("tag me")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = String::from_utf8(out).unwrap();
    let id = id.trim().trim_start_matches("added ").to_string();

    // add tags
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["tag", "add", &id, "a", "b"]) // two tags
        .assert()
        .success();
    // list tags and verify
    let ls = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["tag", "ls", &id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(ls.contains("a"));
    assert!(ls.contains("b"));

    // remove one tag
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["tag", "rm", &id, "a"])
        .assert()
        .success();
    let ls2 = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["tag", "ls", &id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(!ls2.split_whitespace().any(|t| t == "a"));
    assert!(ls2.split_whitespace().any(|t| t == "b"));
}
