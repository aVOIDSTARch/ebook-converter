//! Basic CLI integration tests.

#![allow(deprecated)] // Command::cargo_bin deprecated for custom build-dir; still works for default

use assert_cmd::Command;

#[test]
fn help_prints_and_exits_success() {
    Command::cargo_bin("ebook-converter")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn config_show_runs() {
    Command::cargo_bin("ebook-converter")
        .unwrap()
        .args(["config", "show"])
        .assert()
        .success();
}

#[test]
fn config_show_json_valid() {
    let out = Command::cargo_bin("ebook-converter")
        .unwrap()
        .args(["config", "show", "--json"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&out.get_output().stdout).unwrap();
    let _: serde_json::Value = serde_json::from_str(stdout).expect("config show --json should output valid JSON");
}

#[test]
fn validate_nonexistent_file_fails() {
    Command::cargo_bin("ebook-converter")
        .unwrap()
        .args(["validate", "/nonexistent/file.epub"])
        .assert()
        .failure();
}

#[test]
fn convert_nonexistent_file_reports_missing() {
    // CLI currently continues and exits 0 when one input is missing; it prints to stderr
    let out = Command::cargo_bin("ebook-converter")
        .unwrap()
        .args(["convert", "/nonexistent/file.epub", "-o", "out.epub"])
        .assert();
    let stderr = std::str::from_utf8(&out.get_output().stderr).unwrap();
    assert!(stderr.contains("not found") || stderr.contains("Input file not found"));
}
