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
    let (manifest, files, mut errors) =
        crate::collection::discover_collection_files(manifest_path)?;

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

/// Parse and validate a single table from a YAML string.
///
/// Validates the table's internal consistency (dice expression, range
/// coverage, entry bounds) via [`validate_table`]. Does NOT resolve chain or
/// compound references — those require a populated [`Registry`]; check them
/// with [`crate::validator::validate_references`] after registering the table.
///
/// The YAML must include the `id` field. (Files emitted by `init` omit `id`
/// and rely on the manifest loader deriving it from the filename; load those
/// via [`load_collection`], not this function.)
pub fn load_table_str(yaml: &str) -> Result<Table, Error> {
    let table: Table = serde_yaml::from_str(yaml)?;
    validate_table(&table)?;
    Ok(table)
}

/// Read, parse, and validate a single table from a YAML file.
///
/// Same validation scope as [`load_table_str`]: internal consistency only,
/// chain/compound references left unresolved.
pub fn load_table(path: &Path) -> Result<Table, Error> {
    let contents = std::fs::read_to_string(path).map_err(|e| LoadError::FileRead {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    load_table_str(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::fixtures_path;
    use std::path::PathBuf;

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

    #[test]
    fn load_table_str_valid_simple() {
        let yaml = r#"
type: simple
id: goblins
name: Goblins
roll: 1d6
results:
  - min: 1
    max: 3
    text: Few
  - min: 4
    max: 6
    text: Many
"#;
        let table = load_table_str(yaml).unwrap();
        assert_eq!(table.id(), "goblins");
        assert_eq!(table.name(), "Goblins");
    }

    #[test]
    fn load_table_str_rejects_range_gap() {
        let yaml = r#"
type: simple
id: gappy
name: Gappy
roll: 1d6
results:
  - min: 1
    max: 2
    text: Low
  - min: 5
    max: 6
    text: High
"#;
        let err = load_table_str(yaml).unwrap_err();
        assert!(matches!(
            err,
            Error::Validation(ValidationError::RangeGap { .. })
        ));
    }

    #[test]
    fn load_table_str_rejects_bad_dice() {
        let yaml = r#"
type: simple
id: bad-dice
name: Bad Dice
roll: 1z6
results:
  - min: 1
    max: 6
    text: Whatever
"#;
        let err = load_table_str(yaml).unwrap_err();
        assert!(matches!(
            err,
            Error::Validation(ValidationError::InvalidDiceExpression { .. })
        ));
    }

    #[test]
    fn load_table_str_rejects_malformed_yaml() {
        let err = load_table_str("\t not: [valid").unwrap_err();
        assert!(matches!(err, Error::Yaml(_)));
    }

    #[test]
    fn load_table_str_rejects_missing_id() {
        // Standalone tables must include `id`; the loader does not derive it
        // (only the manifest loader does, from the filename).
        let yaml = r#"
type: simple
name: No Id
roll: 1d6
results:
  - min: 1
    max: 6
    text: Whatever
"#;
        let err = load_table_str(yaml).unwrap_err();
        assert!(matches!(err, Error::Yaml(_)));
    }

    #[test]
    fn load_table_str_allows_unresolved_chain_ref() {
        // Locks the contract: chain references are NOT validated here.
        // Resolution is deferred to validate_references against a registry.
        let yaml = r#"
type: simple
id: with-chain
name: With Chain
roll: 1d6
results:
  - min: 1
    max: 6
    text: Triggers another table
    chain:
      - does-not-exist
"#;
        let table = load_table_str(yaml).unwrap();
        assert_eq!(table.id(), "with-chain");
    }

    #[test]
    fn load_table_str_allows_unresolved_compound_ref() {
        // Compound references are deferred too; validate_table is a no-op
        // for compound tables.
        let yaml = r#"
type: compound
id: combo
name: Combo
tables:
  - does-not-exist-a
  - does-not-exist-b
"#;
        let table = load_table_str(yaml).unwrap();
        assert_eq!(table.id(), "combo");
    }

    #[test]
    fn load_table_reads_valid_file() {
        let table = load_table(&fixtures_path("valid-collection/terrain/wilderness.yaml")).unwrap();
        assert_eq!(table.name(), "Wilderness Terrain");
    }

    #[test]
    fn load_table_missing_path_returns_file_read_error() {
        let err = load_table(&PathBuf::from("/nonexistent/table.yaml")).unwrap_err();
        assert!(matches!(err, Error::Load(LoadError::FileRead { .. })));
    }
}
