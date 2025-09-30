mod common;
use common::TestEnv;

#[test]
fn list_limit_works() {
    let t = TestEnv::new();
    // init
    t.bin()
        .arg("--db")
        .arg(&t.db)
        .arg("init-db")
        .assert()
        .success();
    // add multiple entries
    for i in 0..10 {
        t.bin()
            .arg("--db")
            .arg(&t.db)
            .arg("add")
            .write_stdin(format!("msg{}", i))
            .assert()
            .success();
    }
    // limit
    let out = t
        .bin()
        .arg("--db")
        .arg(&t.db)
        .args(["list", "--json", "--limit", "3"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 3);
}
