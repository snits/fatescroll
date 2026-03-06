// ABOUTME: Integration tests for the fatescroll CLI.
// ABOUTME: Tests subcommands against fixture collections.

use std::path::PathBuf;
use std::process::Command;

fn fatescroll_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fatescroll"))
}

fn fixtures_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn validate_valid_collection() {
    let output = fatescroll_bin()
        .args(["validate", &fixtures_path("valid-collection").to_string_lossy()])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_invalid_collection_fails() {
    let output = fatescroll_bin()
        .args([
            "validate",
            &fixtures_path("invalid-collection").to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}

#[test]
fn roll_on_table() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.terrain.wilderness",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wilderness Terrain"));
}

#[test]
fn search_by_tag() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--tag",
            "npc",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("NPC Occupation"));
}

#[test]
fn roll_nonexistent_table() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "nonexistent.table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}
