//! CLI integration tests for the codeplag binary.
//!
//! These tests exercise the CLI commands using assert_cmd to verify
//! that the binary runs correctly from the command line.

use assert_cmd::Command;
use predicates::prelude::*;

// ── Basic CLI Behavior ────────────────────────────────────────────

#[test]
fn binary_has_help() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("code plagiarism"))
        .stdout(predicate::str::contains("analyze"))
        .stdout(predicate::str::contains("compare"))
        .stdout(predicate::str::contains("project"));
}

#[test]
fn binary_has_version() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.arg("--version").assert().success();
}

// ── Analyze Command ───────────────────────────────────────────────

#[test]
fn analyze_local_directory() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "analyze",
        "--path",
        "test_fixtures/original",
        "--output",
        "text",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Found"));
}

#[test]
fn analyze_with_json_output() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "analyze",
        "--path",
        "test_fixtures/original",
        "--output",
        "json",
    ])
    .assert()
    .success();
}

#[test]
fn analyze_with_functions_flag() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "--functions",
        "analyze",
        "--path",
        "test_fixtures/original",
        "--output",
        "text",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Found"));
}

#[test]
fn analyze_with_custom_threshold() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    // Global options must come BEFORE the subcommand
    cmd.args([
        "--threshold", "0.9",
        "analyze",
        "--path",
        "test_fixtures/original",
        "--output",
        "text",
    ])
    .assert()
    .success();
}

#[test]
fn analyze_with_custom_kgram_window() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "--k-gram", "6",
        "--window", "5",
        "analyze",
        "--path",
        "test_fixtures/original",
        "--output",
        "text",
    ])
    .assert()
    .success();
}

// ── Compare Command ───────────────────────────────────────────────

#[test]
fn compare_two_files() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "compare",
        "--file",
        "test_fixtures/original/sort_rust.rs",
        "--against",
        "test_fixtures/renamed/sort_rust.rs",
        "--output",
        "text",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Comparing"));
}

#[test]
fn compare_file_against_directory() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "compare",
        "--file",
        "test_fixtures/original/sort_rust.rs",
        "--against",
        "test_fixtures/renamed",
        "--output",
        "text",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("against"));
}

#[test]
fn compare_with_json_output() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "compare",
        "--file",
        "test_fixtures/original/sort_rust.rs",
        "--against",
        "test_fixtures/renamed/sort_rust.rs",
        "--output",
        "json",
    ])
    .assert()
    .success();
}

#[test]
fn compare_with_functions() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "--functions",
        "compare",
        "--file",
        "test_fixtures/original/sort_rust.rs",
        "--against",
        "test_fixtures/renamed/sort_rust.rs",
        "--output",
        "text",
    ])
    .assert()
    .success();
}

// ── Project Command ───────────────────────────────────────────────

#[test]
fn project_compare_two_directories() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "project",
        "-a",
        "test_fixtures/original",
        "-b",
        "test_fixtures/renamed",
        "--output",
        "text",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Project Similarity"));
}

#[test]
fn project_with_json_output() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "project",
        "-a",
        "test_fixtures/original",
        "-b",
        "test_fixtures/renamed",
        "--output",
        "json",
    ])
    .assert()
    .success();
}

#[test]
fn project_with_threshold() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "--threshold", "0.9",
        "project",
        "-a",
        "test_fixtures/original",
        "-b",
        "test_fixtures/unrelated",
        "--output",
        "text",
    ])
    .assert()
    .success();
}

// ── Error Handling ────────────────────────────────────────────────

#[test]
fn analyze_nonexistent_path_fails() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args(["analyze", "--path", "nonexistent_directory_xyz"])
        .assert()
        .failure();
}

#[test]
fn compare_nonexistent_file_fails() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "compare",
        "--file",
        "nonexistent.rs",
        "--against",
        "nonexistent_dir",
    ])
    .assert()
    .failure();
}

#[test]
fn invalid_threshold_rejected() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "analyze",
        "--path",
        "test_fixtures/original",
        "--threshold",
        "2.0", // threshold must be in [0, 1]
    ])
    .assert()
    .failure();
}

// ── Verbose Mode ──────────────────────────────────────────────────

#[test]
fn verbose_flag_accepted() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args([
        "--verbose",
        "analyze",
        "--path",
        "test_fixtures/original",
        "--output",
        "text",
    ])
    .assert()
    .success();
}

// ── Batch Command (basic smoke test) ──────────────────────────────

#[test]
fn batch_command_help() {
    let mut cmd = Command::cargo_bin("codeplag").unwrap();
    cmd.args(["batch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repos"));
}
