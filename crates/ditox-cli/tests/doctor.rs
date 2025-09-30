mod common;
use common::TestEnv;

#[test]
fn doctor_reports_clipboard_and_search() {
    let t = TestEnv::new();
    let out = String::from_utf8(
        t.bin()
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .arg("--db")
            .arg(&t.db)
            .arg("doctor")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(out.contains("clipboard:"));
    assert!(out.contains("search (fts or like):"));
}
