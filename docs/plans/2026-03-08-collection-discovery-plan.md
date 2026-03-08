# Collection Discovery Extraction — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract duplicated manifest-walking logic from loader.rs and fixer.rs into a shared `discover_collection_files` function in a new `collection.rs` module.

**Architecture:** A new `collection.rs` module provides `CollectionFile` struct and `discover_collection_files()` function. Both loader and fixer call this function instead of duplicating the manifest/directory/file discovery logic. Each consumer handles its own parsing (loader → `Table`, fixer → `serde_yaml::Value`).

**Tech Stack:** Rust, serde_yaml, std::fs

---

### Task 1: Create collection.rs with CollectionFile and discover_collection_files

**Files:**
- Create: `src/collection.rs`
- Modify: `src/lib.rs:4` (add `pub mod collection;`)

**Step 1: Write the failing test**

In `src/collection.rs`, create the module with tests first:

```rust
// ABOUTME: Discovers table files in a collection by walking manifest directories.
// ABOUTME: Provides shared file discovery logic used by both loader and fixer.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, LoadError};
use crate::models::Manifest;

/// A discovered table file with its metadata and raw contents.
pub struct CollectionFile {
    /// Full path to the YAML file.
    pub path: PathBuf,
    /// Fully-qualified namespace for this file's directory.
    pub namespace: String,
    /// Filename stem (e.g. "wilderness" from "wilderness.yaml").
    pub stem: String,
    /// Raw file contents as a string.
    pub contents: String,
}

/// Discover all table files in a collection.
///
/// Reads the manifest, walks its directories, and returns the raw contents
/// of each `.yaml`/`.yml` file found (excluding `manifest.yaml` itself).
///
/// Returns `Err` for unrecoverable errors (manifest missing/unparseable).
/// Per-file IO errors are accumulated in the returned `Vec<Error>`.
pub fn discover_collection_files(
    manifest_path: &Path,
) -> Result<(Vec<CollectionFile>, Vec<Error>), Error> {
    todo!()
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
    fn discovers_all_files_in_valid_collection() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let (files, errors) = discover_collection_files(&manifest_path).unwrap();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        // valid-collection has 7 table files
        assert_eq!(files.len(), 7, "expected 7 files, got {}", files.len());
        // Each file should have non-empty contents, stem, and namespace
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
        let (files, _) = discover_collection_files(&manifest_path).unwrap();
        for f in &files {
            let ext = f.path.extension().and_then(|e| e.to_str()).unwrap();
            assert!(ext == "yaml" || ext == "yml", "unexpected ext: {}", ext);
        }
    }
}
```

**Step 2: Add module declaration to lib.rs**

Add `pub mod collection;` to `src/lib.rs` (alphabetically before `error`).

**Step 3: Run tests to verify they fail**

Run: `cargo test collection::tests -v`
Expected: FAIL — `todo!()` panics

**Step 4: Implement discover_collection_files**

Replace the `todo!()` body with the shared logic extracted from loader.rs lines 14-111 (the discovery portion only — no validation, no deserialization to Table):

```rust
pub fn discover_collection_files(
    manifest_path: &Path,
) -> Result<(Vec<CollectionFile>, Vec<Error>), Error> {
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

    Ok((files, errors))
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test collection::tests -v`
Expected: PASS (3 tests)

**Step 6: Commit**

```bash
git add src/collection.rs src/lib.rs
git commit -s -m "feat: add collection file discovery module"
```

---

### Task 2: Refactor loader.rs to use discover_collection_files

**Files:**
- Modify: `src/loader.rs`

**Step 1: Run existing loader tests to capture baseline**

Run: `cargo test loader::tests -v`
Expected: PASS (5 tests)

**Step 2: Rewrite load_collection to use discover_collection_files**

Replace `load_collection` body. The function should:
1. Call `discover_collection_files(manifest_path)?` to get files and discovery errors
2. Seed the errors vec with any discovery errors
3. Validate the manifest namespace (need to re-parse manifest for this — see note below)
4. For each `CollectionFile`, do: validate directory namespace, deserialize to `Table`, check id-filename match, validate_table, register

**Important detail:** The loader validates namespaces (both manifest-level and per-directory). The manifest namespace isn't per-file — it's checked once. The directory namespace comes from `CollectionFile.namespace`. The loader needs the manifest's top-level namespace too. Two options:
- Re-read and parse the manifest in the loader (wasteful, defeats purpose)
- Add `manifest: Manifest` to the return from `discover_collection_files`

Add `manifest` to the return type: `Result<(Manifest, Vec<CollectionFile>, Vec<Error>), Error>`.

Updated `load_collection`:

```rust
pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    let (manifest, files, mut errors) = crate::collection::discover_collection_files(manifest_path)?;

    let mut registry = Registry::new();

    // Validate manifest namespace
    if let Err(e) = validate_namespace(&manifest.namespace) {
        errors.push(e.into());
    }

    // Track which directory namespaces we've validated
    let mut validated_namespaces = std::collections::HashSet::new();

    for file in &files {
        // Validate directory namespace once per unique namespace
        if validated_namespaces.insert(&file.namespace) {
            if let Err(e) = validate_namespace(&file.namespace) {
                errors.push(e.into());
                continue;
            }
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
```

**Step 3: Update discover_collection_files signature to return Manifest**

Back in `src/collection.rs`, change return type to `Result<(Manifest, Vec<CollectionFile>, Vec<Error>), Error>` and return the parsed manifest alongside the files. Update collection.rs tests to destructure the new tuple.

**Step 4: Run loader tests to verify no regression**

Run: `cargo test loader::tests -v`
Expected: PASS (5 tests)

**Step 5: Run full test suite**

Run: `cargo test`
Expected: all pass

**Step 6: Commit**

```bash
git add src/collection.rs src/loader.rs
git commit -s -m "refactor: loader uses shared collection discovery"
```

---

### Task 3: Refactor fixer.rs to use discover_collection_files

**Files:**
- Modify: `src/fixer.rs`

**Step 1: Run existing fixer tests to capture baseline**

Run: `cargo test fixer::tests -v`
Expected: PASS (4 tests)

**Step 2: Rewrite fix_collection to use discover_collection_files**

Replace the manifest reading + directory walking + file reading with `discover_collection_files`. Keep the id patching logic. The fixer doesn't validate namespaces — it just patches ids.

```rust
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
```

**Step 3: Clean up fixer.rs imports**

Remove unused imports that were only needed for the old discovery logic (keep `fs` since write still needs it, keep `LoadError` for the "not a mapping" error).

**Step 4: Run fixer tests to verify no regression**

Run: `cargo test fixer::tests -v`
Expected: PASS (4 tests)

**Step 5: Run full test suite + integration tests**

Run: `cargo test`
Expected: all pass (57 unit + 6 integration)

**Step 6: Commit**

```bash
git add src/fixer.rs
git commit -s -m "refactor: fixer uses shared collection discovery"
```

---

### Task 4: Clean up unused imports and verify

**Files:**
- Modify: `src/loader.rs` (remove now-unused imports)
- Modify: `src/fixer.rs` (remove now-unused imports)

**Step 1: Run `cargo clippy` to catch dead imports**

Run: `cargo clippy 2>&1`
Expected: warnings about unused imports if any remain

**Step 2: Fix any warnings**

Remove unused `use` statements from both files.

**Step 3: Run full test suite**

Run: `cargo test`
Expected: all pass

**Step 4: Commit**

```bash
git add src/loader.rs src/fixer.rs
git commit -s -m "chore: remove unused imports after collection refactor"
```
