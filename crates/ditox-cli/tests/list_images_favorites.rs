mod common;
use common::TestEnv;

#[test]
fn list_images_respects_favorites_filter() {
    let t = TestEnv::new();
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    let png = t.write_tiny_png();
    // add two images
    for _ in 0..2 {
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["add", "--image-path"])
            .arg(&png)
            .assert()
            .success();
    }
    // get first id
    let text = String::from_utf8(
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .args(["list", "--images"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let first_id = text.split_whitespace().next().unwrap().to_string();
    // favorite it
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .args(["favorite", &first_id])
        .assert()
        .success();
    // list favorites only
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .args(["list", "--images", "--favorites", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(
        v.as_array().unwrap()[0]
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        first_id
    );
}
