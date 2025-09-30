mod common;
use common::TestEnv;

#[test]
fn config_json_prints_paths() {
    let t = TestEnv::new();
    let out = t
        .bin()
        .args(["config", "--json"]) // uses default settings
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    // Storage backend present
    assert!(v.get("storage").is_some());
    // Config dir under our isolated XDG path
    let cfgdir = v.get("config_dir").and_then(|s| s.as_str()).unwrap_or("");
    assert!(cfgdir.contains("ditox"));
}
