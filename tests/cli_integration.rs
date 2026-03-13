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
