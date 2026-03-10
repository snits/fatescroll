// ABOUTME: Fixes table collection YAML files by correcting `id` fields and detecting stale references.
// ABOUTME: Two-pass: fixes ids to match filenames, then scans for chain/compound references needing update.

use std::fs;
use std::path::Path;

use crate::error::{Error, LoadError};

/// How to handle stale references found during fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefHandling {
    WarnOnly,
    Update,
}

/// Describes the action taken on a single table file.
#[derive(Debug)]
pub enum FixAction {
    /// The `id` field was missing and has been added.
    Added { path: std::path::PathBuf, id: String },
    /// The `id` field had a wrong value and has been corrected.
    Corrected {
        path: std::path::PathBuf,
        old_id: String,
        id: String,
    },
    /// The `id` field was already correct.
    Ok { path: std::path::PathBuf },
    /// A reference was updated from old to new value.
    UpdatedReference {
        path: std::path::PathBuf,
        old_ref: String,
        new_ref: String,
    },
}

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

/// Accumulated results from fixing a collection.
#[derive(Debug)]
pub struct FixResult {
    pub actions: Vec<FixAction>,
    pub warnings: Vec<FixWarning>,
    pub errors: Vec<Error>,
}

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
                    } else if let Some(mapping) = chain.as_mapping() {
                        let table_key = serde_yaml::Value::String("table".into());
                        if let Some(serde_yaml::Value::String(s)) = mapping.get(&table_key) {
                            refs.push(s.clone());
                        }
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
            if let Some(entry_map) = entry.as_mapping_mut()
                && let Some(serde_yaml::Value::Sequence(chains)) = entry_map.get_mut(&chain_key)
            {
                for chain in chains {
                    if let Some(old) = chain.as_str()
                        && let Some(new_id) = corrections.get(old)
                    {
                        let old_str = old.to_string();
                        *chain = serde_yaml::Value::String(new_id.clone());
                        updated.push((old_str, new_id.clone()));
                    } else if let Some(mapping) = chain.as_mapping_mut() {
                        let table_key = serde_yaml::Value::String("table".into());
                        if let Some(serde_yaml::Value::String(old)) = mapping.get(&table_key)
                            && let Some(new_id) = corrections.get(old.as_str()).cloned()
                        {
                            let old_str = old.clone();
                            mapping.insert(table_key, serde_yaml::Value::String(new_id.clone()));
                            updated.push((old_str, new_id));
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
            if let Some(old) = table_ref.as_str()
                && let Some(new_id) = corrections.get(old)
            {
                let old_str = old.to_string();
                *table_ref = serde_yaml::Value::String(new_id.clone());
                updated.push((old_str, new_id.clone()));
            }
        }
    }

    updated
}

/// Fix id fields across all table files in a collection.
///
/// For each YAML file:
/// - If the `id` field is missing, insert it matching the filename stem.
/// - If the `id` field is wrong, replace it with the filename stem.
/// - If it's correct, record as Ok.
/// - If the file can't be parsed, record the error and continue.
pub fn fix_collection(manifest_path: &Path, ref_handling: RefHandling) -> Result<FixResult, Error> {
    let (_manifest, files, discovery_errors) =
        crate::collection::discover_collection_files(manifest_path)?;

    let mut result = FixResult {
        actions: Vec::new(),
        warnings: Vec::new(),
        errors: discovery_errors,
    };

    for file in files {
        let mut value: serde_yaml::Value = match serde_yaml::from_str(&file.contents) {
            Ok(v) => v,
            Err(e) => {
                result.errors.push(Error::Yaml(e));
                continue;
            }
        };

        let mapping = match value.as_mapping_mut() {
            Some(m) => m,
            None => {
                result.errors.push(
                    LoadError::InvalidFormat {
                        path: file.path.clone(),
                        reason: "YAML root is not a mapping".into(),
                    }
                    .into(),
                );
                continue;
            }
        };

        let id_key = serde_yaml::Value::String("id".to_string());
        let expected_id = serde_yaml::Value::String(file.stem.clone());

        if let Some(existing_id) = mapping.get(&id_key) {
            if existing_id == &expected_id {
                result.actions.push(FixAction::Ok { path: file.path });
            } else {
                let old_id = existing_id
                    .as_str()
                    .unwrap_or("<non-string>")
                    .to_string();
                mapping.insert(id_key, expected_id);
                let yaml_out = serde_yaml::to_string(&value)?;
                fs::write(&file.path, &yaml_out)?;
                result.actions.push(FixAction::Corrected {
                    path: file.path,
                    old_id,
                    id: file.stem,
                });
            }
        } else {
            let mut new_mapping = serde_yaml::Mapping::new();
            new_mapping.insert(id_key, expected_id);
            for (k, v) in mapping.iter() {
                new_mapping.insert(k.clone(), v.clone());
            }
            *mapping = new_mapping;
            let yaml_out = serde_yaml::to_string(&value)?;
            fs::write(&file.path, &yaml_out)?;
            result.actions.push(FixAction::Added {
                path: file.path,
                id: file.stem,
            });
        }
    }

    // Pass 2: Detect stale references if any ids were corrected.
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
        let (_manifest2, files2, discovery_errors2) =
            crate::collection::discover_collection_files(manifest_path)?;
        result.errors.extend(discovery_errors2);

        // Collect valid file stems for false-positive guard
        let valid_stems: std::collections::HashSet<String> =
            files2.iter().map(|f| f.stem.clone()).collect();

        // Filter corrections to only those with valid target stems
        let filtered_corrections: std::collections::HashMap<String, String> = corrections
            .into_iter()
            .filter(|(_, new_id)| valid_stems.contains(new_id))
            .collect();

        if ref_handling == RefHandling::Update {
            // Update mode: fix stale references and report as actions
            for file in files2 {
                let mut value: serde_yaml::Value = match serde_yaml::from_str(&file.contents) {
                    Ok(v) => v,
                    Err(e) => {
                        result.errors.push(Error::Yaml(e));
                        continue;
                    }
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
        } else {
            // WarnOnly mode: detect stale references and report as warnings
            for file in &files2 {
                let value: serde_yaml::Value = match serde_yaml::from_str(&file.contents) {
                    Ok(v) => v,
                    Err(e) => {
                        result.errors.push(Error::Yaml(e));
                        continue;
                    }
                };

                let refs = extract_references(&value);
                for ref_str in &refs {
                    if let Some(new_id) = filtered_corrections.get(ref_str.as_str()) {
                        result.warnings.push(FixWarning::StaleReference {
                            path: file.path.clone(),
                            reference: ref_str.clone(),
                            suggested: new_id.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_collection(tables: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        let tables_dir = dir.path().join("tables");
        fs::create_dir_all(&tables_dir).unwrap();

        let manifest = r#"name: Test
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: tables
    namespace: test.tables
"#;
        fs::write(dir.path().join("manifest.yaml"), manifest).unwrap();

        for (filename, content) in tables {
            fs::write(tables_dir.join(filename), content).unwrap();
        }

        dir
    }

    #[test]
    fn fix_adds_missing_id() {
        let dir = setup_collection(&[(
            "my-table.yaml",
            "name: My Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
        )]);
        let manifest = dir.path().join("manifest.yaml");
        let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(&result.actions[0], FixAction::Added { id, .. } if id == "my-table"));
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());

        // Verify the file was actually updated
        let content = fs::read_to_string(dir.path().join("tables/my-table.yaml")).unwrap();
        assert!(content.contains("id: my-table"));
    }

    #[test]
    fn fix_corrects_wrong_id() {
        let dir = setup_collection(&[(
            "correct-name.yaml",
            "id: wrong-name\nname: Some Table\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: Something\n",
        )]);
        let manifest = dir.path().join("manifest.yaml");
        let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

        assert_eq!(result.actions.len(), 1);
        assert!(result.warnings.is_empty());
        assert!(
            matches!(&result.actions[0], FixAction::Corrected { old_id, id, .. } if old_id == "wrong-name" && id == "correct-name")
        );

        let content = fs::read_to_string(dir.path().join("tables/correct-name.yaml")).unwrap();
        assert!(content.contains("id: correct-name"));
        assert!(!content.contains("wrong-name"));
    }

    #[test]
    fn fix_reports_unparseable_yaml() {
        let dir = setup_collection(&[
            ("good.yaml", "id: good\nname: Good\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: X\n"),
            ("bad.yaml", "{{{{not valid yaml at all"),
        ]);
        let manifest = dir.path().join("manifest.yaml");
        let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

        // One good file processed, one error collected
        assert_eq!(result.actions.len(), 1);
        assert!(result.warnings.is_empty());
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn fix_reports_non_mapping_yaml_as_format_error() {
        let dir = setup_collection(&[("scalar.yaml", "just a string, not a mapping")]);
        let manifest = dir.path().join("manifest.yaml");
        let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

        assert!(result.actions.is_empty());
        assert!(result.warnings.is_empty());
        assert_eq!(result.errors.len(), 1);

        // Verify the error is a LoadError::InvalidFormat, not FileRead
        match &result.errors[0] {
            Error::Load(LoadError::InvalidFormat { path, .. }) => {
                assert!(path.to_string_lossy().contains("scalar.yaml"));
            }
            other => panic!("Expected LoadError::InvalidFormat, got: {other:?}"),
        }
    }

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

    #[test]
    fn fix_skips_correct_files() {
        let dir = setup_collection(&[(
            "already-correct.yaml",
            "id: already-correct\nname: Correct\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: X\n",
        )]);
        let manifest = dir.path().join("manifest.yaml");
        let result = fix_collection(&manifest, RefHandling::WarnOnly).unwrap();

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(&result.actions[0], FixAction::Ok { .. }));
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }

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

        // No warnings in Update mode
        assert!(result.warnings.is_empty());

        // Verify the file was actually updated
        let content = fs::read_to_string(dir.path().join("tables/wilderness.yaml")).unwrap();
        assert!(content.contains("wolf-count"));
        assert!(!content.contains("wolf-counter"));
    }

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

    #[test]
    fn extract_references_from_structured_chain() {
        let yaml = r#"
id: mishap
name: Wizard Mishap
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 1
    text: Roll twice
    chain:
      - table: mishap
        reroll: [1]
      - plain-ref
  - min: 2
    max: 4
    text: Normal
"#;
        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let refs = extract_references(&value);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"mishap".to_string()));
        assert!(refs.contains(&"plain-ref".to_string()));
    }

    #[test]
    fn update_references_in_structured_chain() {
        let yaml = r#"
id: mishap
name: Wizard Mishap
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 1
    text: Roll twice
    chain:
      - table: old-name
        reroll: [1]
      - plain-old-ref
  - min: 2
    max: 4
    text: Normal
"#;
        let mut value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let mut corrections = std::collections::HashMap::new();
        corrections.insert("old-name".to_string(), "new-name".to_string());
        corrections.insert("plain-old-ref".to_string(), "plain-new-ref".to_string());

        let updated = update_references(&mut value, &corrections);
        assert_eq!(updated.len(), 2);
        assert!(updated.contains(&("old-name".to_string(), "new-name".to_string())));
        assert!(updated.contains(&("plain-old-ref".to_string(), "plain-new-ref".to_string())));

        let refs = extract_references(&value);
        assert!(refs.contains(&"new-name".to_string()));
        assert!(refs.contains(&"plain-new-ref".to_string()));
    }

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
}
