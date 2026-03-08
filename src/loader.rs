// ABOUTME: Loads table collections from the filesystem into a registry.
// ABOUTME: Reads manifests, discovers YAML files, parses, validates, and registers.

use std::fs;
use std::path::Path;

use crate::error::{Error, LoadError, ValidationError};
use crate::models::{Manifest, Table};
use crate::registry::Registry;
use crate::validator::{validate_namespace, validate_table};

/// Load a collection from a manifest file path.
/// Returns a populated Registry or collected errors.
pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    if !manifest_path.exists() {
        return Err(LoadError::ManifestNotFound {
            path: manifest_path.to_path_buf(),
        }
        .into());
    }

    let manifest_contents = fs::read_to_string(manifest_path).map_err(|e| LoadError::FileRead {
        path: manifest_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let mut manifest: Manifest = serde_yaml::from_str(&manifest_contents)?;
    manifest.base_path = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let mut registry = Registry::new();
    let mut errors: Vec<Error> = Vec::new();

    // Validate manifest namespace
    if let Err(e) = validate_namespace(&manifest.namespace) {
        errors.push(e.into());
    }

    for dir_entry in &manifest.directories {
        // Validate directory namespace
        if let Err(e) = validate_namespace(&dir_entry.namespace) {
            errors.push(e.into());
            continue;
        }

        let dir_path = manifest.base_path.join(&dir_entry.path);
        if !dir_path.is_dir() {
            errors.push(ValidationError::DirectoryNotFound { path: dir_path }.into());
            continue;
        }

        // Discover and load YAML files
        let entries = match fs::read_dir(&dir_path) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(
                    LoadError::FileRead {
                        path: dir_path,
                        reason: e.to_string(),
                    }
                    .into(),
                );
                continue;
            }
        };

        for entry_result in entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    errors.push(
                        LoadError::FileRead {
                            path: dir_path.clone(),
                            reason: e.to_string(),
                        }
                        .into(),
                    );
                    continue;
                }
            };
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("yaml") && ext != Some("yml") {
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("manifest.yaml") {
                continue;
            }

            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

            let fqid = format!("{}.{}", dir_entry.namespace, stem);

            let contents = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(
                        LoadError::FileRead {
                            path: path.clone(),
                            reason: e.to_string(),
                        }
                        .into(),
                    );
                    continue;
                }
            };

            let table: Table = match serde_yaml::from_str(&contents) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(Error::Yaml(e));
                    continue;
                }
            };

            // Validate that the table's id matches the filename stem
            if table.id() != stem {
                errors.push(
                    ValidationError::IdFilenameMismatch {
                        id: table.id().to_string(),
                        filename: stem.clone(),
                        path: path.clone(),
                    }
                    .into(),
                );
                continue;
            }

            // Per-type validation
            if let Err(e) = validate_table(&table) {
                errors.push(e.into());
                continue;
            }

            if let Err(e) = registry.register(fqid, table) {
                errors.push(e.into());
            }
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
