// ABOUTME: Integration tests for the fatescroll CLI.
// ABOUTME: Tests subcommands against fixture collections.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

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
        .args([
            "validate",
            &fixtures_path("valid-collection").to_string_lossy(),
        ])
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
fn validate_fix_adds_missing_ids() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(
        dir.path().join("manifest.yaml"),
        "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("my-table.yaml"),
        "name: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let output = fatescroll_bin()
        .args([
            "validate",
            "--fix",
            &dir.path().to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Added id 'my-table'"),
        "Expected 'Added id' in output, got: {stdout}"
    );

    // Verify the file was actually fixed
    let content = std::fs::read_to_string(tables_dir.join("my-table.yaml")).unwrap();
    assert!(content.contains("id: my-table"), "File should contain id: my-table, got: {content}");
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
