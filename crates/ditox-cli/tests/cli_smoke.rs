use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use std::process;
use tempfile::tempdir;
use std::fs;

fn bin() -> Command { Command::cargo_bin("ditox-cli").unwrap() }

#[test]
fn text_flow() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");

    // init
    bin().arg("--db").arg(&db).arg("init-db").assert().success();

    // add via stdin
    let mut cmd = bin();
    let assert = cmd.arg("--db").arg(&db).arg("add").write_stdin("hello test").assert();
    assert.success().stdout(predicate::str::contains("added "));

    // list json
    let output = bin().arg("--db").arg(&db).args(["list","--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(v.as_array().unwrap().len() >= 1);

    // search json
    let output = bin().arg("--db").arg(&db).args(["search","hello","--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(v.as_array().unwrap().len() >= 1);
}

#[test]
fn image_from_file() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("ditox.db");
    bin().arg("--db").arg(&db).arg("init-db").assert().success();

    // create tiny 2x2 PNG
    let img_path = dir.path().join("tiny.png");
    {
        use image::{RgbaImage, Rgba, ImageBuffer};
        let mut img: RgbaImage = ImageBuffer::new(2,2);
        for p in img.pixels_mut() { *p = Rgba([0,255,0,255]); }
        img.save(&img_path).unwrap();
    }

    bin().arg("--db").arg(&db).args(["add","--image-path"]).arg(&img_path).assert().success();

    let out = bin().arg("--db").arg(&db).args(["list","--images","--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let arr = v.as_array().unwrap();
    assert!(!arr.is_empty());
}
