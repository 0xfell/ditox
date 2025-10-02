use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    let mut c = Command::cargo_bin("ditox-cli").unwrap();
    c.arg("--store").arg("mem");
    c
}

#[test]
fn list_available_themes() {
    bin()
        .args(["pick", "--themes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dark"))
        .stdout(predicate::str::contains("high-contrast"));
}

#[test]
fn ascii_preview_no_unicode() {
    let out = bin()
        .args(["pick", "--preview", "dark", "--ascii", "--color=never"]) // ensure no ANSI
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("Ditox Picker — Preview"));
    // No box-drawing characters
    assert!(!s.contains("┌") && !s.contains("─") && !s.contains("│") && !s.contains("┘"));
    // Contains ASCII label for Enter
    assert!(s.contains("Enter copy"));
}

#[test]
fn list_glyphs_and_layouts() {
    // glyphs
    bin()
        .args(["pick", "--glyphsets"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ascii"));
    // layouts
    bin()
        .args(["pick", "--layouts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"));
}
