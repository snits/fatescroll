// ABOUTME: Discovers table files in a collection by walking manifest directories.
// ABOUTME: Provides shared file discovery logic used by both loader and fixer.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, LoadError};
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

    let manifest_contents =
        fs::read_to_string(manifest_path).map_err(|e| LoadError::FileRead {
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
            if path.file_name().and_then(|n| n.to_str()) == Some("manifest.yaml") {
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

    Ok((manifest, files, errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn discovers_all_files_in_valid_collection() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let (_manifest, files, errors) = discover_collection_files(&manifest_path).unwrap();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(files.len(), 10, "expected 10 files, got {}", files.len());
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
    fn skips_non_yaml_files() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let (_manifest, files, _) = discover_collection_files(&manifest_path).unwrap();
        for f in &files {
            let ext = f.path.extension().and_then(|e| e.to_str()).unwrap();
            assert!(ext == "yaml" || ext == "yml", "unexpected ext: {}", ext);
        }
    }
}
