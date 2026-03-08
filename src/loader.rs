// ABOUTME: Loads table collections from the filesystem into a registry.
// ABOUTME: Reads manifests, discovers YAML files, parses, validates, and registers.

use std::path::Path;

use crate::error::{Error, LoadError, ValidationError};
use crate::models::Table;
use crate::registry::Registry;
use crate::validator::{validate_namespace, validate_table};

/// Load a collection from a manifest file path.
/// Returns a populated Registry or collected errors.
pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    let (manifest, files, mut errors) = crate::collection::discover_collection_files(manifest_path)?;

    let mut registry = Registry::new();

    // Validate manifest namespace
    if let Err(e) = validate_namespace(&manifest.namespace) {
        errors.push(e.into());
    }

    // Cache namespace validation results: true = valid, false = rejected
    let mut namespace_valid: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();

    for file in &files {
        let is_valid = match namespace_valid.get(&file.namespace) {
            Some(&valid) => valid,
            None => {
                let valid = match validate_namespace(&file.namespace) {
                    Ok(()) => true,
                    Err(e) => {
                        errors.push(e.into());
                        false
                    }
                };
                namespace_valid.insert(file.namespace.clone(), valid);
                valid
            }
        };
        if !is_valid {
            continue;
        }

        let fqid = format!("{}.{}", file.namespace, file.stem);

        let table: Table = match serde_yaml::from_str(&file.contents) {
            Ok(t) => t,
            Err(e) => {
                errors.push(Error::Yaml(e));
                continue;
            }
        };

        if table.id() != file.stem {
            errors.push(
                ValidationError::IdFilenameMismatch {
                    id: table.id().to_string(),
                    filename: file.stem.clone(),
                    path: file.path.clone(),
                }
                .into(),
            );
            continue;
        }

        if let Err(e) = validate_table(&table) {
            errors.push(e.into());
            continue;
        }

        if let Err(e) = registry.register(fqid, table) {
            errors.push(e.into());
        }
    }

    if errors.is_empty() {
        Ok(registry)
    } else {
        Err(LoadError::Multiple { errors }.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn load_valid_collection() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let registry = load_collection(&manifest_path).unwrap();

        // Should have loaded all tables
        assert!(registry.get("test.terrain.wilderness").is_some());
        assert!(registry.get("test.npc.npc-occupation").is_some());
        assert!(registry.get("test.npc.npc-disposition").is_some());
        assert!(registry.get("test.npc.npc-quirk").is_some());
        assert!(registry.get("test.npc.quick-npc").is_some());
        assert!(
            registry
                .get("test.encounters.wilderness-encounter")
                .is_some()
        );
        assert!(registry.get("test.encounters.animal-type").is_some());
    }

    #[test]
    fn load_invalid_collection_accumulates_errors() {
        let manifest_path = fixtures_path("invalid-collection/manifest.yaml");
        let err = load_collection(&manifest_path).unwrap_err();
        // The invalid collection has tables with bad dice, gaps, overlaps,
        // and reversed ranges. The loader should collect multiple errors.
        match err {
            Error::Load(LoadError::Multiple { errors }) => {
                assert!(
                    errors.len() >= 2,
                    "Expected multiple errors, got {}",
                    errors.len()
                );
            }
            other => panic!("Expected LoadError::Multiple, got: {other}"),
        }
    }

    #[test]
    fn load_manifest_not_found() {
        let result = load_collection(&PathBuf::from("/nonexistent/manifest.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_rejects_id_filename_mismatch() {
        let manifest_path = fixtures_path("id-mismatch-collection/manifest.yaml");
        let err = load_collection(&manifest_path).unwrap_err();
        let err_msg = format!("{err}");
        assert!(
            err_msg.contains("wrong-id") && err_msg.contains("actual-filename"),
            "Expected id mismatch error, got: {err_msg}"
        );
    }

    #[test]
    fn loaded_table_has_correct_data() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let registry = load_collection(&manifest_path).unwrap();
        let table = registry.get("test.terrain.wilderness").unwrap();
        assert_eq!(table.name(), "Wilderness Terrain");
    }
}
