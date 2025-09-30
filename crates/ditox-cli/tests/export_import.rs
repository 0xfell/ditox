mod common;
use common::TestEnv;

#[test]
fn export_then_import_roundtrip() {
    let t1 = TestEnv::new();
    t1.bin()
        .arg("--db")
        .arg(&t1.db)
        .arg("init-db")
        .assert()
        .success();
    // add text and image
    t1.bin()
        .arg("--db")
        .arg(&t1.db)
        .arg("add")
        .write_stdin("roundtrip")
        .assert()
        .success();
    let png = t1.write_tiny_png();
    t1.bin()
        .arg("--db")
        .arg(&t1.db)
        .args(["add", "--image-path"])
        .arg(&png)
        .assert()
        .success();

    // export to dir
    let exp = t1.cfg.join("export");
    t1.bin()
        .arg("--db")
        .arg(&t1.db)
        .args(["export"])
        .arg(&exp)
        .assert()
        .success();
    assert!(exp.join("clips.jsonl").exists());

    // import into a fresh DB
    let t2 = TestEnv::new();
    t2.bin()
        .arg("--db")
        .arg(&t2.db)
        .arg("init-db")
        .assert()
        .success();
    t2.bin()
        .arg("--db")
        .arg(&t2.db)
        .args(["import"])
        .arg(&exp)
        .assert()
        .success();

    // list shows at least 1 text and 1 image item
    let txt = String::from_utf8(
        t2.bin()
            .arg("--db")
            .arg(&t2.db)
            .args(["list", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    assert!(!v.as_array().unwrap().is_empty());
    let imgs = String::from_utf8(
        t2.bin()
            .arg("--db")
            .arg(&t2.db)
            .args(["list", "--images", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    let v2: serde_json::Value = serde_json::from_str(&imgs).unwrap();
    assert!(!v2.as_array().unwrap().is_empty());
}
