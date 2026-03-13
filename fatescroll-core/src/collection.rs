// ABOUTME: Discovers table files in a collection by walking manifest directories.
// ABOUTME: Provides shared file discovery logic used by both loader and fixer.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, LoadError, ValidationError};
use crate::models::Manifest;

/// A discovered YAML file within a collection directory.
#[derive(Debug)]
pub struct CollectionFile {
    pub path: PathBuf,
    pub namespace: String,
    pub stem: String,
    pub contents: String,
}

/// Discover all YAML table files in a collection by reading the manifest
/// and walking its configured directories.
///
/// Returns the parsed manifest, all discovered files, and any per-file
/// IO errors that were accumulated (hard errors like missing manifest
/// return Err instead).
pub fn discover_collection_files(
    manifest_path: &Path,
) -> Result<(Manifest, Vec<CollectionFile>, Vec<Error>), Error> {
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

    let mut files = Vec::new();
    let mut errors: Vec<Error> = Vec::new();

    for dir_entry in &manifest.directories {
        let dir_path = manifest.base_path.join(&dir_entry.path);
        if !dir_path.is_dir() {
            errors.push(ValidationError::DirectoryNotFound { path: dir_path }.into());
            continue;
        }

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
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name == "manifest.yaml" || name.ends_with(".manifest.yaml"))
            {
                continue;
            }

            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

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

            files.push(CollectionFile {
                path,
                namespace: dir_entry.namespace.clone(),
                stem,
                contents,
            });
        }
    }

    for file_entry in &manifest.files {
        let file_path = manifest.base_path.join(&file_entry.path);
        if !file_path.exists() {
            errors.push(ValidationError::FileEntryNotFound { path: file_path }.into());
            continue;
        }
        if !file_path.is_file() {
            errors.push(ValidationError::FileEntryNotAFile { path: file_path }.into());
            continue;
        }
        let ext = file_path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            errors.push(ValidationError::FileEntryInvalidExtension { path: file_path }.into());
            continue;
        }
        if let Some(name) = file_path.file_name().and_then(|n| n.to_str())
            && (name == "manifest.yaml" || name.ends_with(".manifest.yaml"))
        {
            continue;
        }
        let stem = match file_path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let contents = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(
                    LoadError::FileRead {
                        path: file_path,
                        reason: e.to_string(),
                    }
                    .into(),
                );
                continue;
            }
        };
        files.push(CollectionFile {
            path: file_path,
            namespace: file_entry.namespace.clone(),
            stem,
            contents,
        });
    }

    Ok((manifest, files, errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures")
            .join(name)
    }

    #[test]
    fn discovers_all_files_in_valid_collection() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let (_manifest, files, errors) = discover_collection_files(&manifest_path).unwrap();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(files.len(), 12, "expected 12 files, got {}", files.len());
        for f in &files {
            assert!(!f.stem.is_empty());
            assert!(!f.namespace.is_empty());
            assert!(!f.contents.is_empty());
        }
    }

    #[test]
    fn missing_manifest_returns_error() {
        let result = discover_collection_files(Path::new("/nonexistent/manifest.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn discovers_file_entries() {
        let manifest_path = fixtures_path("file-entries-collection/manifest.yaml");
        let (_manifest, files, errors) = discover_collection_files(&manifest_path).unwrap();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(files.len(), 2, "expected 2 files, got {}", files.len());
        let file_entry = files.iter().find(|f| f.stem == "wilderness").unwrap();
        assert_eq!(file_entry.namespace, "filetest.terrain");
    }

    #[test]
    fn file_entry_not_found_is_soft_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = r#"name: T
version: "1.0"
namespace: t
files:
  - path: nonexistent.yaml
    namespace: t.x
"#;
        std::fs::write(dir.path().join("manifest.yaml"), manifest).unwrap();
        let (_m, files, errors) =
            discover_collection_files(&dir.path().join("manifest.yaml")).unwrap();
        assert!(files.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("not found"));
    }

    #[test]
    fn file_entry_pointing_to_directory_is_error() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        let manifest = r#"name: T
version: "1.0"
namespace: t
files:
  - path: subdir
    namespace: t.x
"#;
        std::fs::write(dir.path().join("manifest.yaml"), manifest).unwrap();
        let (_m, files, errors) =
            discover_collection_files(&dir.path().join("manifest.yaml")).unwrap();
        assert!(files.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("not a file"));
    }

    #[test]
    fn file_entry_invalid_extension_is_error() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("table.json"), "{}").unwrap();
        let manifest = r#"name: T
version: "1.0"
namespace: t
files:
  - path: table.json
    namespace: t.x
"#;
        std::fs::write(dir.path().join("manifest.yaml"), manifest).unwrap();
        let (_m, files, errors) =
            discover_collection_files(&dir.path().join("manifest.yaml")).unwrap();
        assert!(files.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("invalid extension"));
    }

    #[test]
    fn file_entry_invalid_namespace_caught_by_loader() {
        let dir = tempfile::TempDir::new().unwrap();
        let table_yaml = r#"id: test-table
name: Test
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 4
    text: X
"#;
        std::fs::write(dir.path().join("test-table.yaml"), table_yaml).unwrap();
        let manifest = r#"name: T
version: "1.0"
namespace: t
files:
  - path: test-table.yaml
    namespace: INVALID
"#;
        std::fs::write(dir.path().join("manifest.yaml"), manifest).unwrap();
        let result = crate::loader::load_collection(&dir.path().join("manifest.yaml"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid namespace")
        );
    }

    #[test]
    fn skips_non_yaml_files() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let (_manifest, files, _) = discover_collection_files(&manifest_path).unwrap();
        for f in &files {
            let ext = f.path.extension().and_then(|e| e.to_str()).unwrap();
            assert!(ext == "yaml" || ext == "yml", "unexpected ext: {}", ext);
        }
    }
}
