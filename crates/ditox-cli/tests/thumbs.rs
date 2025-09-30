mod common;
use common::TestEnv;

#[test]
fn thumbs_generate_for_stored_image() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    let png = t.write_tiny_png();
    // add image from path (keep file so thumbs can read it)
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["add", "--image-path"])
        .arg(&png)
        .assert()
        .success();

    // generate thumbs
    let out = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .arg("thumbs")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(out.contains("thumbnails generated:"));
}
