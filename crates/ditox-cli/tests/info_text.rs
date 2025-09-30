mod common;
use common::TestEnv;

#[test]
fn info_text_outputs_expected_fields() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .arg("add")
        .write_stdin("inspect me")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = String::from_utf8(out).unwrap();
    let id = id.trim().trim_start_matches("added ").to_string();
    let info = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["info", &id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(info.contains("kind:\ttext"));
    assert!(info.contains("preview:"));
}
