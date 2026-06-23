// ABOUTME: Integration tests for the fatescroll CLI.
// ABOUTME: Tests subcommands against fixture collections.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

const TEST_MANIFEST_YAML: &str = "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n";

fn fatescroll_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fatescroll"))
}

fn fixtures_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name)
}

#[test]
fn validate_valid_collection() {
    let output = fatescroll_bin()
        .args([
            "validate",
            "--collection",
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
            "--collection",
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
fn roll_json_output() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.terrain.wilderness",
            "--json",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(value["table_name"], "Wilderness Terrain");
    assert!(value["roll"].is_number());
    assert!(value["children"].is_array());
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
            "--collection",
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
    assert!(
        content.contains("id: my-table"),
        "File should contain id: my-table, got: {content}"
    );
}

#[test]
fn cwd_fallback_succeeds_when_manifest_present() {
    let collection = fixtures_path("valid-collection");
    let output = fatescroll_bin()
        .args(["validate"])
        .current_dir(&collection)
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cwd_fallback_fails_without_manifest() {
    let dir = TempDir::new().unwrap();
    let output = fatescroll_bin()
        .args(["validate"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No collection found"),
        "Expected 'No collection found' in stderr, got: {stderr}"
    );
}

#[test]
fn search_tags_lists_unique_tags() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--tags",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected_tag in [
        "animal",
        "bandit",
        "encounter",
        "generator",
        "merchant",
        "npc",
        "terrain",
        "wilderness",
    ] {
        assert!(
            stdout.contains(expected_tag),
            "Expected tag '{expected_tag}' in output, got: {stdout}"
        );
    }
}

#[test]
fn search_tags_conflicts_with_name() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--tags",
            "--name",
            "foo",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        !output.status.success(),
        "Expected failure when --tags and --name are combined"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--tags") || stderr.contains("cannot be used with"),
        "Expected conflict error in stderr, got: {stderr}"
    );
}

#[test]
fn search_json_output() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--tag",
            "npc",
            "--json",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let hits = value.as_array().expect("expected a JSON array");
    let hit = hits
        .iter()
        .find(|h| h["name"] == "NPC Occupation")
        .expect("expected a hit named 'NPC Occupation'");
    assert!(hit["id"].is_string(), "expected string id, got: {hit}");
    let tags = hit["tags"].as_array().expect("expected tags array");
    assert!(
        tags.iter().any(|t| t == "npc"),
        "expected tags to contain 'npc', got: {hit}"
    );
}

#[test]
fn search_tags_json_output() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--tags",
            "--json",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let tags = value.as_array().expect("expected a JSON array");
    assert!(
        tags.iter().any(|t| t == "npc"),
        "expected tags array to contain 'npc', got: {value}"
    );
}

#[test]
fn search_json_no_results() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--name",
            "__no_such_table__",
            "--json",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert!(
        value.as_array().unwrap().is_empty(),
        "expected empty JSON array, got: {value}"
    );
}

#[test]
fn validate_fix_warns_about_stale_references() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(
        dir.path().join("manifest.yaml"),
        "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("wolf-count.yaml"),
        "id: wolf-counter\nname: Wolf Count\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Wolves\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("wilderness.yaml"),
        "id: wilderness\nname: Wilderness\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 2\n    text: Animals\n    chain:\n      - wolf-counter\n  - min: 3\n    max: 4\n    text: Nothing\n",
    ).unwrap();

    let output = fatescroll_bin()
        .args([
            "validate",
            "--fix",
            "--collection",
            &dir.path().to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stale reference"),
        "Expected 'stale reference' in stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("--update-refs"),
        "Expected '--update-refs' in stderr, got: {stderr}"
    );
}

#[test]
fn validate_fix_update_refs_fixes_stale_references() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(
        dir.path().join("manifest.yaml"),
        "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("wolf-count.yaml"),
        "id: wolf-counter\nname: Wolf Count\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Wolves\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("wilderness.yaml"),
        "id: wilderness\nname: Wilderness\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 2\n    text: Animals\n    chain:\n      - wolf-counter\n  - min: 3\n    max: 4\n    text: Nothing\n",
    ).unwrap();

    let output = fatescroll_bin()
        .args([
            "validate",
            "--fix",
            "--update-refs",
            "--collection",
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
        stdout.contains("Updated reference"),
        "Expected 'Updated reference' in stdout, got: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("stale reference"),
        "Expected no 'stale reference' in stderr, got: {stderr}"
    );
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

#[test]
fn show_displays_simple_table() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.wilderness-encounter",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wilderness Encounter"));
    assert!(stdout.contains("Roll: 1d8"));
    assert!(stdout.contains("→ animal-type"));
}

#[test]
fn show_displays_compound_table() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.npc.quick-npc",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Quick NPC Generator"));
    assert!(stdout.contains("Tables:"));
    assert!(stdout.contains("npc-occupation"));
}

#[test]
fn show_nonexistent_table_fails() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "nonexistent.table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}

#[test]
fn roll_mishap_table_with_reroll_chain() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.mishap",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "roll failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wizard Mishap"));
}

#[test]
fn show_mishap_table_displays_reroll() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.mishap",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wizard Mishap"));
    assert!(stdout.contains("reroll"));
}

#[test]
fn validate_named_manifest_in_directory() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(
        dir.path().join("campaign.manifest.yaml"),
        "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("my-table.yaml"),
        "id: my-table\nname: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let output = fatescroll_bin()
        .args(["validate", "--collection", &dir.path().to_string_lossy()])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_multiple_manifests_error() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    let manifest_content = "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n";

    std::fs::write(dir.path().join("manifest.yaml"), manifest_content).unwrap();
    std::fs::write(dir.path().join("other.manifest.yaml"), manifest_content).unwrap();

    std::fs::write(
        tables_dir.join("my-table.yaml"),
        "id: my-table\nname: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let output = fatescroll_bin()
        .args(["validate", "--collection", &dir.path().to_string_lossy()])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Multiple manifests found"),
        "Expected 'Multiple manifests found' in stderr, got: {stderr}"
    );
}

#[test]
fn validate_named_manifest_direct_file_path() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(
        dir.path().join("campaign.manifest.yaml"),
        "name: Test\nversion: \"1.0\"\nnamespace: test\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: tables\n    namespace: test.tables\n",
    ).unwrap();

    std::fs::write(
        tables_dir.join("my-table.yaml"),
        "id: my-table\nname: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let manifest_path = dir.path().join("campaign.manifest.yaml");
    let output = fatescroll_bin()
        .args(["validate", "--collection", &manifest_path.to_string_lossy()])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_collection_with_file_entries() {
    let output = fatescroll_bin()
        .args([
            "validate",
            "--collection",
            &fixtures_path("file-entries-collection").to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Collection is valid."));
}

#[test]
fn roll_on_file_entry_table() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("file-entries-collection").to_string_lossy(),
            "filetest.terrain.wilderness",
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
fn validate_files_only_manifest() {
    let output = fatescroll_bin()
        .args([
            "validate",
            "--collection",
            &fixtures_path("files-only-collection").to_string_lossy(),
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
fn init_explicit_1d6() {
    let output = fatescroll_bin()
        .args(["init", "--roll", "1d6", "--name", "Test Table"])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name: Test Table"));
    assert!(stdout.contains("roll: 1d6"));
    assert!(stdout.contains("type: simple"));
    assert_eq!(stdout.matches("  - min:").count(), 6);
}

#[test]
fn init_flat_distribution() {
    let output = fatescroll_bin()
        .args([
            "init",
            "--entries",
            "8",
            "--distribution",
            "flat",
            "--name",
            "Flat Table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roll: 1d8"));
    assert_eq!(stdout.matches("  - min:").count(), 8);
}

#[test]
fn init_bell_exact_match() {
    let output = fatescroll_bin()
        .args([
            "init",
            "--entries",
            "11",
            "--distribution",
            "bell",
            "--name",
            "Bell Table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roll: 2d6"));
    assert_eq!(stdout.matches("  - min:").count(), 11);
}

#[test]
fn init_bell_no_exact_match_shows_suggestions() {
    let output = fatescroll_bin()
        .args(["init", "--entries", "12", "--distribution", "bell"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("2d6"));
    assert!(stderr.contains("2d7"));
    assert!(stderr.contains("With 2 dice:"));
    assert!(stderr.contains("With 3 dice:"));
}

#[test]
fn init_bell_too_few_entries() {
    let output = fatescroll_bin()
        .args(["init", "--entries", "2", "--distribution", "bell"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("at least 3 entries"));
}

#[test]
fn init_output_to_file() {
    let dir = TempDir::new().unwrap();
    let output_file = dir.path().join("test-table.yaml");
    let output = fatescroll_bin()
        .args([
            "init",
            "--roll",
            "1d4",
            "--name",
            "File Table",
            "--output",
            &output_file.to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let contents = std::fs::read_to_string(&output_file).unwrap();
    assert!(contents.contains("roll: 1d4"));
    assert_eq!(contents.matches("  - min:").count(), 4);
}

#[test]
fn init_output_refuses_overwrite() {
    let dir = TempDir::new().unwrap();
    let output_file = dir.path().join("existing.yaml");
    std::fs::write(&output_file, "existing content").unwrap();
    let output = fatescroll_bin()
        .args([
            "init",
            "--roll",
            "1d4",
            "--output",
            &output_file.to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn init_invalid_dice_expression() {
    let output = fatescroll_bin()
        .args(["init", "--roll", "1z6"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}

#[test]
fn init_zero_entries() {
    let output = fatescroll_bin()
        .args(["init", "--entries", "0", "--distribution", "flat"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("at least 1"));
}

#[test]
fn init_requires_roll_or_entries() {
    let output = fatescroll_bin()
        .args(["init", "--name", "No Dice"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}

#[test]
fn roll_on_d66_table() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.traveller.d66-sample",
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
        stdout.contains("D66 Sample Table"),
        "Expected table name in output, got: {stdout}"
    );
    // The output should contain a valid D66 result (e.g., "Entry 11" through "Entry 66")
    // where both digits are 1-6
    let has_valid_entry = (1u32..=6)
        .flat_map(|d1| (1u32..=6).map(move |d2| d1 * 10 + d2))
        .any(|v| stdout.contains(&format!("Entry {v}")));
    assert!(
        has_valid_entry,
        "Expected a valid D66 entry in output, got: {stdout}"
    );
}

#[test]
fn import_happy_path() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(dir.path().join("manifest.yaml"), TEST_MANIFEST_YAML).unwrap();

    let src_dir = TempDir::new().unwrap();
    std::fs::write(
        src_dir.path().join("my-table.yaml"),
        "id: my-table\nname: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let src_file = src_dir.path().join("my-table.yaml");
    let manifest_path = dir.path().join("manifest.yaml");
    let output = fatescroll_bin()
        .args([
            "import",
            "--collection",
            &manifest_path.to_string_lossy(),
            "--target-dir",
            "tables",
            &src_file.to_string_lossy(),
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
        stdout.contains("Imported:"),
        "Expected 'Imported:' in stdout, got: {stdout}"
    );
    assert!(
        stdout.contains("Collection is valid after import."),
        "Expected validation message in stdout, got: {stdout}"
    );
    assert!(
        tables_dir.join("my-table.yaml").exists(),
        "Expected file to exist in target dir"
    );
}

#[test]
fn import_creates_target_directory() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("manifest.yaml"), TEST_MANIFEST_YAML).unwrap();

    let src_dir = TempDir::new().unwrap();
    std::fs::write(
        src_dir.path().join("my-table.yaml"),
        "id: my-table\nname: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let src_file = src_dir.path().join("my-table.yaml");
    let manifest_path = dir.path().join("manifest.yaml");
    let expected_dest = dir.path().join("tables").join("my-table.yaml");

    assert!(
        !dir.path().join("tables").exists(),
        "tables dir should not exist yet"
    );

    let output = fatescroll_bin()
        .args([
            "import",
            "--collection",
            &manifest_path.to_string_lossy(),
            "--target-dir",
            "tables",
            &src_file.to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        expected_dest.exists(),
        "Expected file to be created in new target dir"
    );
}

#[test]
fn import_nonexistent_file_fails() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(dir.path().join("manifest.yaml"), TEST_MANIFEST_YAML).unwrap();

    let manifest_path = dir.path().join("manifest.yaml");
    let output = fatescroll_bin()
        .args([
            "import",
            "--collection",
            &manifest_path.to_string_lossy(),
            "--target-dir",
            "tables",
            "/nonexistent/path/ghost-table.yaml",
        ])
        .output()
        .expect("failed to run fatescroll");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ghost-table.yaml") || stderr.contains("No such file"),
        "Expected filename or 'No such file' in stderr, got: {stderr}"
    );
}

#[test]
fn import_invalid_table_fails_validation() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(dir.path().join("manifest.yaml"), TEST_MANIFEST_YAML).unwrap();

    let src_dir = TempDir::new().unwrap();
    std::fs::write(
        src_dir.path().join("bad-table.yaml"),
        "id: bad-table\nname: Bad Table\ntype: simple\ntags: []\nroll: NOTADICE\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
    ).unwrap();

    let src_file = src_dir.path().join("bad-table.yaml");
    let manifest_path = dir.path().join("manifest.yaml");
    let output = fatescroll_bin()
        .args([
            "import",
            "--collection",
            &manifest_path.to_string_lossy(),
            "--target-dir",
            "tables",
            &src_file.to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");

    assert!(
        !output.status.success(),
        "Expected failure due to invalid table, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        tables_dir.join("bad-table.yaml").exists(),
        "File should have been copied before validation failed"
    );
}

#[test]
fn import_multiple_files() {
    let dir = TempDir::new().unwrap();
    let tables_dir = dir.path().join("tables");
    std::fs::create_dir_all(&tables_dir).unwrap();

    std::fs::write(dir.path().join("manifest.yaml"), TEST_MANIFEST_YAML).unwrap();

    let src_dir = TempDir::new().unwrap();
    std::fs::write(
        src_dir.path().join("table-one.yaml"),
        "id: table-one\nname: Table One\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Alpha\n",
    ).unwrap();
    std::fs::write(
        src_dir.path().join("table-two.yaml"),
        "id: table-two\nname: Table Two\ntype: simple\ntags: []\nroll: 1d6\nresults:\n  - min: 1\n    max: 6\n    text: Beta\n",
    ).unwrap();

    let src_file_one = src_dir.path().join("table-one.yaml");
    let src_file_two = src_dir.path().join("table-two.yaml");
    let manifest_path = dir.path().join("manifest.yaml");
    let output = fatescroll_bin()
        .args([
            "import",
            "--collection",
            &manifest_path.to_string_lossy(),
            "--target-dir",
            "tables",
            &src_file_one.to_string_lossy(),
            &src_file_two.to_string_lossy(),
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
        stdout.contains("Collection is valid after import."),
        "Expected validation message in stdout, got: {stdout}"
    );
    assert!(
        tables_dir.join("table-one.yaml").exists(),
        "Expected table-one.yaml in target dir"
    );
    assert!(
        tables_dir.join("table-two.yaml").exists(),
        "Expected table-two.yaml in target dir"
    );
}

#[test]
fn init_d66_template() {
    let output = fatescroll_bin()
        .args(["init", "--roll", "D66", "--name", "Test D66"])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("roll: D66"),
        "Expected 'roll: D66' in output, got: {stdout}"
    );
    assert_eq!(
        stdout.matches("  - min:").count(),
        36,
        "Expected exactly 36 entries, got output: {stdout}"
    );
    assert!(
        !stdout.contains("min: 17"),
        "Output should not contain impossible D66 value 17, got: {stdout}"
    );
}

#[test]
fn search_by_name() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--name",
            "Wilderness",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Wilderness Terrain"),
        "Expected 'Wilderness Terrain' in output, got: {stdout}"
    );
    assert!(
        stdout.contains("Wilderness Encounter"),
        "Expected 'Wilderness Encounter' in output, got: {stdout}"
    );
}

#[test]
fn search_by_name_no_results() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--name",
            "ZZZZNONEXISTENT",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No tables found."),
        "Expected 'No tables found.' in output, got: {stdout}"
    );
}

#[test]
fn search_by_namespace() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--namespace",
            "test.npc",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("NPC Occupation"),
        "Expected 'NPC Occupation' in output, got: {stdout}"
    );
    assert!(
        stdout.contains("NPC Disposition"),
        "Expected 'NPC Disposition' in output, got: {stdout}"
    );
    assert!(
        stdout.contains("Quick NPC Generator"),
        "Expected 'Quick NPC Generator' in output, got: {stdout}"
    );
}

#[test]
fn search_by_namespace_no_results() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "--namespace",
            "nonexistent",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No tables found."),
        "Expected 'No tables found.' in output, got: {stdout}"
    );
}

#[test]
fn roll_with_modifier_clamps_high() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("modifier-collection").to_string_lossy(),
            "mod.shadowdark.carousing",
            "--modifier",
            "100",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rolled 14"), "got: {stdout}");
}

#[test]
fn roll_with_modifier_on_plain_table_errors() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.animal-type",
            "--modifier",
            "2",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not support a roll modifier"),
        "got: {stderr}"
    );
}

#[test]
fn roll_without_modifier_still_works() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("modifier-collection").to_string_lossy(),
            "mod.traveller.aging",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Aging"), "got: {stdout}");
}

#[test]
fn roll_with_value_direct_lookup() {
    // wilderness-encounter entry 6-7 is "Abandoned campsite" (no chain): a clean
    // deterministic lookup proving --value skips the dice roll.
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.wilderness-encounter",
            "--value",
            "6",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Abandoned campsite"), "got: {stdout}");
    assert!(stdout.contains("rolled 6"), "got: {stdout}");
}

#[test]
fn roll_with_value_resolves_chain() {
    // wilderness-encounter entry 1-3 chains to animal-type; --value must run the
    // full pipeline, so the child table appears in the output.
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.wilderness-encounter",
            "--value",
            "1",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Animal encounter"), "got: {stdout}");
    assert!(stdout.contains("Animal Type"), "got: {stdout}");
}

#[test]
fn roll_with_value_accepts_negative() {
    // aging covers -5..6; --value -5 looks up the most-negative entry directly.
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("modifier-collection").to_string_lossy(),
            "mod.traveller.aging",
            "--value",
            "-5",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Severe decline"), "got: {stdout}");
}

#[test]
fn roll_with_value_out_of_range_errors() {
    // animal-type covers 1..=4; 9 is outside every entry.
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.animal-type",
            "--value",
            "9",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("out of range"), "got: {stderr}");
}

#[test]
fn roll_with_value_does_not_clamp() {
    // Unlike --modifier, --value past the envelope errors rather than clamping:
    // aging covers -5..6, so 100 must NOT resolve to the top entry.
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("modifier-collection").to_string_lossy(),
            "mod.traveller.aging",
            "--value",
            "100",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("No effect"),
        "value must not clamp: {stdout}"
    );
    assert!(stderr.contains("out of range"), "got: {stderr}");
}

#[test]
fn roll_with_value_conflicts_with_modifier() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("modifier-collection").to_string_lossy(),
            "mod.shadowdark.carousing",
            "--value",
            "5",
            "--modifier",
            "2",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        !output.status.success(),
        "--value and --modifier must be mutually exclusive"
    );
}

#[test]
fn roll_with_value_on_compound_errors() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.npc.quick-npc",
            "--value",
            "1",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not support direct value lookup"),
        "got: {stderr}"
    );
}
