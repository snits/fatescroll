// ABOUTME: Fixes table collection YAML files by adding or correcting `id` fields.
// ABOUTME: Walks manifest directories, parses YAML as generic values, and patches ids to match filenames.

use std::fs;
use std::path::Path;

use crate::error::{Error, LoadError};

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
}

/// Accumulated results from fixing a collection.
#[derive(Debug)]
pub struct FixResult {
    pub actions: Vec<FixAction>,
    pub errors: Vec<Error>,
}

/// Fix id fields across all table files in a collection.
///
/// For each YAML file:
/// - If the `id` field is missing, insert it matching the filename stem.
/// - If the `id` field is wrong, replace it with the filename stem.
/// - If it's correct, record as Ok.
/// - If the file can't be parsed, record the error and continue.
pub fn fix_collection(manifest_path: &Path) -> Result<FixResult, Error> {
    let (_manifest, files, discovery_errors) =
        crate::collection::discover_collection_files(manifest_path)?;

    let mut result = FixResult {
        actions: Vec::new(),
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
                    LoadError::FileRead {
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
        let result = fix_collection(&manifest).unwrap();

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(&result.actions[0], FixAction::Added { id, .. } if id == "my-table"));
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
        let result = fix_collection(&manifest).unwrap();

        assert_eq!(result.actions.len(), 1);
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
        let result = fix_collection(&manifest).unwrap();

        // One good file processed, one error collected
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn fix_skips_correct_files() {
        let dir = setup_collection(&[(
            "already-correct.yaml",
            "id: already-correct\nname: Correct\ntype: simple\ntags: []\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: X\n",
        )]);
        let manifest = dir.path().join("manifest.yaml");
        let result = fix_collection(&manifest).unwrap();

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(&result.actions[0], FixAction::Ok { .. }));
        assert!(result.errors.is_empty());
    }
}
