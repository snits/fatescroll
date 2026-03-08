# Stale Reference Detection Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Detect stale chain/compound references after id corrections, warn by default, auto-fix with `--update-refs`.

**Architecture:** Two-pass approach in `fix_collection`: pass 1 fixes ids (existing), pass 2 detects stale references by scanning for old_ids in chain/tables arrays. Uses `RefHandling` enum to control warn-only vs auto-update behavior. Pass 2 re-reads files from disk to avoid overwriting pass 1 changes.

**Tech Stack:** Rust, serde_yaml (Value manipulation), clap 4, thiserror 2, tempfile (tests)

**Review findings incorporated:**
- `corrections` uses owned `HashMap<String, String>` to avoid borrow checker conflict with `result.actions.push()`
- `UpdatedReference` match arm added to `cmd_fix` in Task 1 (not deferred to Task 3)
- `update_references` receives a filtered corrections map (only entries with valid stems)
- Task 1 split into 1a (types + mechanical) and 1b (pass 2 logic + test)

---

### Task 1a: Add types, update signature, update callers

**Files:**
- Modify: `src/fixer.rs:1-29`
- Modify: `src/main.rs`

This task is purely mechanical — adding types, changing the function signature, and updating all call sites so the code compiles. No new behavior.

**Step 1: Add new types to fixer.rs**

Add to `src/fixer.rs` after the imports (line 7), before `FixAction`:

```rust
/// How to handle stale references found during fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefHandling {
    WarnOnly,
    Update,
}
```

Add variant to `FixAction` (after `Ok`):

```rust
/// A reference was updated from old to new value.
UpdatedReference {
    path: std::path::PathBuf,
    old_ref: String,
    new_ref: String,
},
```

Add `FixWarning` enum (after `FixAction`):

```rust
/// A warning about a potential issue that was not auto-fixed.
#[derive(Debug)]
pub enum FixWarning {
    /// A reference matches a corrected old_id and may be stale.
    StaleReference {
        path: std::path::PathBuf,
        reference: String,
        suggested: String,
    },
}
```

Update `FixResult` to include warnings:

```rust
pub struct FixResult {
    pub actions: Vec<FixAction>,
    pub warnings: Vec<FixWarning>,
    pub errors: Vec<Error>,
}
```

**Step 2: Update fix_collection signature**

Change to:

```rust
pub fn fix_collection(manifest_path: &Path, ref_handling: RefHandling) -> Result<FixResult, Error> {
```

The `ref_handling` parameter is unused for now (pass 2 comes in Task 1b). Suppress the warning by prefixing: `_ref_handling` or adding `let _ = ref_handling;` at the top of the function.

Initialize `warnings` in FixResult:

```rust
let mut result = FixResult {
    actions: Vec::new(),
    warnings: Vec::new(),
    errors: discovery_errors,
};
```

**Step 3: Update caller in main.rs**

In `cmd_fix`, change the signature and fix_collection call:

```rust
fn cmd_fix(collection: &Path, update_refs: bool) -> Result<(), fatescroll::Error> {
    let manifest_path = collection.join("manifest.yaml");
    let ref_handling = if update_refs {
        fatescroll::fixer::RefHandling::Update
    } else {
        fatescroll::fixer::RefHandling::WarnOnly
    };
    let result = fatescroll::fixer::fix_collection(&manifest_path, ref_handling)?;
```

Add `UpdatedReference` match arm to the action printing loop:

```rust
fatescroll::fixer::FixAction::UpdatedReference {
    path,
    old_ref,
    new_ref,
} => {
    println!(
        "Updated reference '{old_ref}' -> '{new_ref}' in {}",
        path.display()
    );
}
```

Add warnings printing after the actions loop, before the errors block:

```rust
if !result.warnings.is_empty() {
    eprintln!("\nWarnings:");
    for warning in &result.warnings {
        match warning {
            fatescroll::fixer::FixWarning::StaleReference {
                path,
                reference,
                suggested,
            } => {
                eprintln!(
                    "  Warning: stale reference '{}' in {} (should be '{}')",
                    reference,
                    path.display(),
                    suggested,
                );
            }
        }
    }
    eprintln!("\nUse --update-refs to automatically fix stale references.");
}
```

Update the Validate match arm to pass `false` (update_refs not yet wired as CLI flag):

```rust
Commands::Validate { collection, fix } => {
    resolve_collection(collection).and_then(|collection| {
        if fix {
            cmd_fix(&collection, false)
        } else {
            cmd_validate(&collection)
        }
    })
}
```

**Step 4: Update existing fixer tests**

In every existing test in `fixer::tests`, change:
```rust
fix_collection(&manifest)
```
to:
```rust
fix_collection(&manifest, RefHandling::WarnOnly)
```

This affects tests: `fix_adds_missing_id`, `fix_corrects_wrong_id`, `fix_reports_unparseable_yaml`, `fix_reports_non_mapping_yaml_as_format_error`, `fix_skips_correct_files`.

Also add `assert!(result.warnings.is_empty());` to each existing test to verify no false warnings.

**Step 5: Run all tests and clippy**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: All pass, no warnings (except possibly an unused variable warning for `ref_handling` — suppress it)

**Step 6: Commit**

```bash
git add src/fixer.rs src/main.rs
git commit -s -m "refactor: add RefHandling, FixWarning types and update fix_collection signature

Adds RefHandling enum, FixWarning::StaleReference, and
FixAction::UpdatedReference types. Updates fix_collection to accept
RefHandling parameter (not yet used). Updates all callers and tests.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 1b: Implement pass 2 stale reference detection

**Files:**
- Modify: `src/fixer.rs`

**Step 1: Write the failing test**

Add to `fixer::tests`:

```rust
#[test]
fn fix_warns_about_stale_chain_reference() {
    let dir = setup_collection(&[
        (
            "wolf-count.yaml",
            "id: wolf-counter\nname: Wolf Count\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Wolves\n",
        ),
        (
            "wilderness.yaml",
            "id: wilderness\nname: Wilderness\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 2\n    text: Animals\n    chain:\n      - wolf-counter\n  - min: 3\n    max: 4\n    text: Nothing\n",
        ),
    ]);
    let manifest = dir.path().join("manifest.yaml");
    let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

    // wolf-count.yaml should have its id corrected
    assert!(result.actions.iter().any(|a| matches!(a, FixAction::Corrected { old_id, id, .. } if old_id == "wolf-counter" && id == "wolf-count")));

    // Should warn about stale reference in wilderness.yaml
    assert_eq!(result.warnings.len(), 1);
    match &result.warnings[0] {
        FixWarning::StaleReference { reference, suggested, .. } => {
            assert_eq!(reference, "wolf-counter");
            assert_eq!(suggested, "wolf-count");
        }
    }

    // The stale reference should NOT be updated in the file (warn only)
    let content = fs::read_to_string(dir.path().join("tables/wilderness.yaml")).unwrap();
    assert!(content.contains("wolf-counter"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test fixer::tests::fix_warns_about_stale_chain_reference`
Expected: FAIL — no warnings generated (pass 2 doesn't exist yet)

**Step 3: Add helper functions**

Add before `fix_collection`:

```rust
/// Extract all chain and compound table references from a parsed YAML value.
fn extract_references(value: &serde_yaml::Value) -> Vec<String> {
    let mut refs = Vec::new();
    let mapping = match value.as_mapping() {
        Some(m) => m,
        None => return refs,
    };

    // Check results[].chain[] (Simple tables)
    let results_key = serde_yaml::Value::String("results".into());
    if let Some(serde_yaml::Value::Sequence(results)) = mapping.get(&results_key) {
        let chain_key = serde_yaml::Value::String("chain".into());
        for entry in results {
            if let Some(serde_yaml::Value::Sequence(chains)) = entry.get(&chain_key) {
                for chain in chains {
                    if let Some(s) = chain.as_str() {
                        refs.push(s.to_string());
                    }
                }
            }
        }
    }

    // Check tables[] (Compound tables)
    let tables_key = serde_yaml::Value::String("tables".into());
    if let Some(serde_yaml::Value::Sequence(tables)) = mapping.get(&tables_key) {
        for table_ref in tables {
            if let Some(s) = table_ref.as_str() {
                refs.push(s.to_string());
            }
        }
    }

    refs
}

/// Update stale references in a parsed YAML value. Returns list of (old, new) pairs updated.
fn update_references(
    value: &mut serde_yaml::Value,
    corrections: &std::collections::HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut updated = Vec::new();
    let mapping = match value.as_mapping_mut() {
        Some(m) => m,
        None => return updated,
    };

    // Update results[].chain[]
    let results_key = serde_yaml::Value::String("results".into());
    if let Some(serde_yaml::Value::Sequence(results)) = mapping.get_mut(&results_key) {
        let chain_key = serde_yaml::Value::String("chain".into());
        for entry in results {
            if let Some(entry_map) = entry.as_mapping_mut() {
                if let Some(serde_yaml::Value::Sequence(chains)) = entry_map.get_mut(&chain_key) {
                    for chain in chains {
                        if let Some(old) = chain.as_str() {
                            if let Some(new_id) = corrections.get(old) {
                                let old_str = old.to_string();
                                *chain = serde_yaml::Value::String(new_id.clone());
                                updated.push((old_str, new_id.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Update tables[]
    let tables_key = serde_yaml::Value::String("tables".into());
    if let Some(serde_yaml::Value::Sequence(tables)) = mapping.get_mut(&tables_key) {
        for table_ref in tables {
            if let Some(old) = table_ref.as_str() {
                if let Some(new_id) = corrections.get(old) {
                    let old_str = old.to_string();
                    *table_ref = serde_yaml::Value::String(new_id.clone());
                    updated.push((old_str, new_id.clone()));
                }
            }
        }
    }

    updated
}
```

**Step 4: Implement pass 2 in fix_collection**

After the existing pass 1 loop (the `for file in files` block), before `Ok(result)`, add:

```rust
// Pass 2: Detect stale references if any ids were corrected.
// Uses owned Strings to avoid borrowing from result.actions during mutation.
let corrections: std::collections::HashMap<String, String> = result
    .actions
    .iter()
    .filter_map(|a| match a {
        FixAction::Corrected { old_id, id, .. } => Some((old_id.clone(), id.clone())),
        _ => None,
    })
    .collect();

if !corrections.is_empty() {
    // Re-read files from disk to pick up pass 1 changes
    let (_manifest2, files2, _) =
        crate::collection::discover_collection_files(manifest_path)?;

    // Collect valid file stems for false-positive guard
    let valid_stems: std::collections::HashSet<String> = files2
        .iter()
        .map(|f| f.stem.clone())
        .collect();

    // Scan for stale references
    for file in &files2 {
        let value: serde_yaml::Value = match serde_yaml::from_str(&file.contents) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let refs = extract_references(&value);
        for ref_str in &refs {
            if let Some(new_id) = corrections.get(ref_str.as_str()) {
                if valid_stems.contains(new_id) {
                    result.warnings.push(FixWarning::StaleReference {
                        path: file.path.clone(),
                        reference: ref_str.clone(),
                        suggested: new_id.clone(),
                    });
                }
            }
        }
    }

    // If update mode, apply the fixes
    if ref_handling == RefHandling::Update && !result.warnings.is_empty() {
        // Filter corrections to only include those with valid stems
        let filtered_corrections: std::collections::HashMap<String, String> = corrections
            .into_iter()
            .filter(|(_, new_id)| valid_stems.contains(new_id))
            .collect();

        for file in files2 {
            let mut value: serde_yaml::Value = match serde_yaml::from_str(&file.contents) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let updated = update_references(&mut value, &filtered_corrections);
            if !updated.is_empty() {
                let yaml_out = serde_yaml::to_string(&value)?;
                fs::write(&file.path, &yaml_out)?;
                for (old_ref, new_ref) in updated {
                    result.actions.push(FixAction::UpdatedReference {
                        path: file.path.clone(),
                        old_ref,
                        new_ref,
                    });
                }
            }
        }
        // Clear warnings since they were acted on
        result.warnings.clear();
    }
}
```

Remove the `let _ = ref_handling;` suppression from Task 1a (if added).

**Step 5: Run test to verify it passes**

Run: `cargo test fixer::tests::fix_warns_about_stale_chain_reference`
Expected: PASS

**Step 6: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 7: Commit**

```bash
git add src/fixer.rs
git commit -s -m "feat: detect stale references after id corrections

Add pass 2 to fix_collection that re-reads files from disk and scans
for chain/compound references matching corrected old_ids. Reports
stale references as warnings (WarnOnly) or auto-updates them (Update).

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add tests for update mode and edge cases

**Files:**
- Modify: `src/fixer.rs` (tests section)

**Step 1: Write test for Update mode**

```rust
#[test]
fn fix_updates_stale_chain_reference_when_forced() {
    let dir = setup_collection(&[
        (
            "wolf-count.yaml",
            "id: wolf-counter\nname: Wolf Count\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Wolves\n",
        ),
        (
            "wilderness.yaml",
            "id: wilderness\nname: Wilderness\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 2\n    text: Animals\n    chain:\n      - wolf-counter\n  - min: 3\n    max: 4\n    text: Nothing\n",
        ),
    ]);
    let manifest = dir.path().join("manifest.yaml");
    let result = fix_collection(&manifest, RefHandling::Update).unwrap();

    // Should have UpdatedReference action
    assert!(result.actions.iter().any(|a| matches!(a,
        FixAction::UpdatedReference { old_ref, new_ref, .. }
        if old_ref == "wolf-counter" && new_ref == "wolf-count"
    )));

    // Warnings should be cleared (they were acted on)
    assert!(result.warnings.is_empty());

    // Verify the file was actually updated
    let content = fs::read_to_string(dir.path().join("tables/wilderness.yaml")).unwrap();
    assert!(content.contains("wolf-count"));
    assert!(!content.contains("wolf-counter"));
}
```

**Step 2: Write test for compound table stale references**

```rust
#[test]
fn fix_warns_about_stale_compound_reference() {
    let dir = setup_collection(&[
        (
            "npc-occupation.yaml",
            "id: npc-job\nname: NPC Job\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Smith\n",
        ),
        (
            "quick-npc.yaml",
            "id: quick-npc\nname: Quick NPC\ntype: compound\ntags: []\ntables:\n  - npc-job\n",
        ),
    ]);
    let manifest = dir.path().join("manifest.yaml");
    let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

    // npc-occupation.yaml should have its id corrected
    assert!(result.actions.iter().any(|a| matches!(a, FixAction::Corrected { old_id, .. } if old_id == "npc-job")));

    // Should warn about stale reference in quick-npc.yaml
    assert_eq!(result.warnings.len(), 1);
    match &result.warnings[0] {
        FixWarning::StaleReference { reference, suggested, .. } => {
            assert_eq!(reference, "npc-job");
            assert_eq!(suggested, "npc-occupation");
        }
    }
}
```

**Step 3: Write test for file needing both id correction AND reference update**

```rust
#[test]
fn fix_handles_file_with_both_id_and_reference_correction() {
    let dir = setup_collection(&[
        (
            "wolf-count.yaml",
            "id: wolf-counter\nname: Wolf Count\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Wolves\n",
        ),
        (
            "encounter.yaml",
            "id: encountr\nname: Encounter\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 2\n    text: Wolves\n    chain:\n      - wolf-counter\n  - min: 3\n    max: 4\n    text: Nothing\n",
        ),
    ]);
    let manifest = dir.path().join("manifest.yaml");
    let result = fix_collection(&manifest, RefHandling::Update).unwrap();

    // Both ids should be corrected
    assert!(result.actions.iter().any(|a| matches!(a, FixAction::Corrected { id, .. } if id == "wolf-count")));
    assert!(result.actions.iter().any(|a| matches!(a, FixAction::Corrected { id, .. } if id == "encounter")));

    // Reference should be updated
    assert!(result.actions.iter().any(|a| matches!(a,
        FixAction::UpdatedReference { old_ref, new_ref, .. }
        if old_ref == "wolf-counter" && new_ref == "wolf-count"
    )));

    // Verify encounter.yaml has BOTH corrections: correct id AND updated reference
    let content = fs::read_to_string(dir.path().join("tables/encounter.yaml")).unwrap();
    assert!(content.contains("id: encounter"), "id should be corrected");
    assert!(content.contains("wolf-count"), "reference should be updated to wolf-count");
    assert!(!content.contains("wolf-counter"), "old reference should be gone");
    assert!(!content.contains("encountr"), "old id should be gone");
}
```

**Step 4: Write test for no corrections = no warnings**

```rust
#[test]
fn fix_no_warnings_when_no_corrections() {
    let dir = setup_collection(&[
        (
            "already-correct.yaml",
            "id: already-correct\nname: Correct\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: X\n",
        ),
    ]);
    let manifest = dir.path().join("manifest.yaml");
    let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

    assert!(result.warnings.is_empty());
}
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

**Step 6: Commit**

```bash
git add src/fixer.rs
git commit -s -m "test: add comprehensive tests for stale reference detection

Tests cover: compound table references, update mode, file needing
both id and reference correction, and no-corrections-no-warnings.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 3: Add --update-refs CLI flag

**Files:**
- Modify: `src/main.rs`

**Step 1: Add flag to Validate command**

In the `Validate` variant of `Commands` enum, add:

```rust
/// Update stale references after id corrections (requires --fix)
#[arg(long, requires = "fix")]
update_refs: bool,
```

**Step 2: Update the Validate match arm**

Change the match arm to pass `update_refs`:

```rust
Commands::Validate { collection, fix, update_refs } => {
    resolve_collection(collection).and_then(|collection| {
        if fix {
            cmd_fix(&collection, update_refs)
        } else {
            cmd_validate(&collection)
        }
    })
}
```

Note: `cmd_fix` already accepts `update_refs: bool` from Task 1a, and already has the full output logic including warnings. This task just wires the CLI flag.

**Step 3: Run all tests and clippy**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: All pass, no warnings

**Step 4: Commit**

```bash
git add src/main.rs
git commit -s -m "feat: add --update-refs flag to validate --fix

Wire CLI flag to control stale reference handling. When --fix detects
stale references, it warns and suggests --update-refs. With
--update-refs, it auto-updates the references.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 4: Add integration tests and update ABOUTME

**Files:**
- Modify: `tests/cli_integration.rs`
- Modify: `src/fixer.rs:1-2` (ABOUTME comment)

**Step 1: Add integration test for warnings**

Add to `tests/cli_integration.rs`. Read the existing tests first to follow the same pattern (they use `std::process::Command::new` with `env!("CARGO_BIN_EXE_fatescroll")` or `assert_cmd`).

The test should:
1. Create a temp collection with a wrong id and a stale chain reference
2. Run `fatescroll validate --fix --collection <path>`
3. Assert stderr contains "Warning: stale reference"
4. Assert stderr contains "Use --update-refs"

**Step 2: Add integration test for --update-refs**

The test should:
1. Create a temp collection with a wrong id and a stale chain reference
2. Run `fatescroll validate --fix --update-refs --collection <path>`
3. Assert stdout contains "Updated reference"
4. Assert stderr does NOT contain "Warning: stale reference"

**Step 3: Update ABOUTME comment**

Change `src/fixer.rs` lines 1-2 to:

```rust
// ABOUTME: Fixes table collection YAML files by correcting `id` fields and detecting stale references.
// ABOUTME: Two-pass: fixes ids to match filenames, then scans for chain/compound references needing update.
```

**Step 4: Run all tests and clippy**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: All pass

**Step 5: Commit**

```bash
git add tests/cli_integration.rs src/fixer.rs
git commit -s -m "test: add integration tests for stale reference warnings and --update-refs

Also updates ABOUTME comment to reflect the two-pass behavior.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 5: Final verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: Clean

**Step 3: Review git log**

Run: `git log --oneline main..HEAD`
Verify commit series is clean and tells a story.
