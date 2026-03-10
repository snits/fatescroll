# Manifest File-Level Entries Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow manifests to reference individual YAML table files in addition to directories.

**Architecture:** Add a `FileEntry` struct and optional `files` field to the `Manifest` struct. Extend `discover_collection_files()` to process file entries after directory entries, producing the same `CollectionFile` structs. Add error variants for file-specific validation failures.

**Tech Stack:** Rust, serde, serde_yaml, thiserror, clap, tempfile (tests)

---

## Task 1: Add FileEntry struct and Manifest field

**Files:**
- Modify: `src/models.rs:98-114` (add FileEntry, modify Manifest)

- [ ] **Step 1: Write failing test for FileEntry deserialization**

Add to `src/models.rs` tests module:

```rust
#[test]
fn deserialize_manifest_with_files() {
    let yaml = r#"
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
files:
  - path: ../shared/npc-occupation.yaml
    namespace: test.npc
"#;
    let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].namespace, "test.npc");
}

#[test]
fn deserialize_manifest_without_files_defaults_empty() {
    let yaml = r#"
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
"#;
    let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert!(manifest.files.is_empty());
}

#[test]
fn deserialize_files_only_manifest() {
    let yaml = r#"
name: Files Only
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
files:
  - path: some-table.yaml
    namespace: test.tables
"#;
    let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert!(manifest.directories.is_empty());
    assert_eq!(manifest.files.len(), 1);
}
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p fatescroll deserialize_manifest_with_files deserialize_manifest_without_files deserialize_files_only -- --nocapture`
Expected: compilation error — `Manifest` has no `files` field

- [ ] **Step 3: Add FileEntry struct and Manifest field**

In `src/models.rs`, after the `DirectoryEntry` struct (line 102), add:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub namespace: String,
}
```

Modify the `Manifest` struct to add `files` with `#[serde(default)]`, and also add `#[serde(default)]` to `directories`:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub author: Option<String>,
    pub min_tool_version: Option<String>,
    #[serde(default)]
    pub directories: Vec<DirectoryEntry>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
    #[serde(skip)]
    pub base_path: PathBuf,
}
```

- [ ] **Step 4: Run tests to confirm they pass**

Run: `cargo test -p fatescroll deserialize_manifest -- --nocapture`
Expected: all manifest deserialization tests pass

- [ ] **Step 5: Run full test suite to check for regressions**

Run: `cargo test`
Expected: all existing tests still pass

- [ ] **Step 6: Commit**

```bash
git add src/models.rs
git commit -s -m "feat: add FileEntry struct and files field to Manifest"
```

---

## Task 2: Add error variants for file entry validation

**Files:**
- Modify: `src/error.rs:28-79` (add variants to ValidationError)

- [ ] **Step 1: Add new error variants**

Add to `ValidationError` enum in `src/error.rs`:

```rust
#[error("file entry not found: {path}")]
FileEntryNotFound { path: PathBuf },

#[error("file entry is not a file: {path}")]
FileEntryNotAFile { path: PathBuf },

#[error("file entry has invalid extension (expected .yaml or .yml): {path}")]
FileEntryInvalidExtension { path: PathBuf },
```

- [ ] **Step 2: Run full test suite**

Run: `cargo test`
Expected: all tests pass (new variants are unused but compile)

- [ ] **Step 3: Commit**

```bash
git add src/error.rs
git commit -s -m "feat: add validation error variants for manifest file entries"
```

---

## Task 3: Implement file entry discovery in collection.rs

**Files:**
- Modify: `src/collection.rs:25-126` (extend discover_collection_files)

- [ ] **Step 1: Create test fixture — collection with file entries**

Create directory structure:

```
tests/fixtures/file-entries-collection/
  manifest.yaml
  encounters/
    animal-type.yaml
  shared/
    wilderness.yaml
```

`manifest.yaml`:
```yaml
name: File Entries Test
version: "1.0"
namespace: filetest
author: ~
min_tool_version: ~
directories:
  - path: encounters
    namespace: filetest.encounters
files:
  - path: shared/wilderness.yaml
    namespace: filetest.terrain
```

`encounters/animal-type.yaml`:
```yaml
id: animal-type
name: Animal Type
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 2
    text: Wolf
  - min: 3
    max: 4
    text: Bear
```

`shared/wilderness.yaml`:
```yaml
id: wilderness
name: Wilderness Terrain
type: simple
tags:
  - terrain
roll: 1d4
results:
  - min: 1
    max: 2
    text: Forest
  - min: 3
    max: 4
    text: Plains
```

- [ ] **Step 2: Write failing test for file entry discovery**

Add to `src/collection.rs` tests module:

```rust
#[test]
fn discovers_file_entries() {
    let manifest_path = fixtures_path("file-entries-collection/manifest.yaml");
    let (_manifest, files, errors) = discover_collection_files(&manifest_path).unwrap();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    // 1 from directory (animal-type) + 1 from file entry (wilderness)
    assert_eq!(files.len(), 2, "expected 2 files, got {}", files.len());
    let file_entry = files.iter().find(|f| f.stem == "wilderness").unwrap();
    assert_eq!(file_entry.namespace, "filetest.terrain");
}
```

- [ ] **Step 3: Run test to confirm it fails**

Run: `cargo test -p fatescroll discovers_file_entries -- --nocapture`
Expected: FAIL — only 1 file discovered (directory entry only)

- [ ] **Step 4: Implement file entry processing in discover_collection_files**

In `src/collection.rs`, after the directory loop (after line 123, before the `Ok` return), add file entry processing:

```rust
for file_entry in &manifest.files {
    let file_path = manifest.base_path.join(&file_entry.path);

    if !file_path.exists() {
        errors.push(
            ValidationError::FileEntryNotFound {
                path: file_path,
            }
            .into(),
        );
        continue;
    }

    if !file_path.is_file() {
        errors.push(
            ValidationError::FileEntryNotAFile {
                path: file_path,
            }
            .into(),
        );
        continue;
    }

    // Extension check comes after is_file() — a directory gets "not a file", not "invalid extension"
    let ext = file_path.extension().and_then(|e| e.to_str());
    if ext != Some("yaml") && ext != Some("yml") {
        errors.push(
            ValidationError::FileEntryInvalidExtension {
                path: file_path,
            }
            .into(),
        );
        continue;
    }

    // Skip manifest files
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
```

- [ ] **Step 5: Run test to confirm it passes**

Run: `cargo test -p fatescroll discovers_file_entries -- --nocapture`
Expected: PASS

- [ ] **Step 6: Write error case tests**

Add to `src/collection.rs` tests module:

```rust
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
    std::fs::write(dir.path().join("manifest.yaml"), &manifest).unwrap();
    let (_m, files, errors) = discover_collection_files(&dir.path().join("manifest.yaml")).unwrap();
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
    std::fs::write(dir.path().join("manifest.yaml"), &manifest).unwrap();
    let (_m, files, errors) = discover_collection_files(&dir.path().join("manifest.yaml")).unwrap();
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
    let (_m, files, errors) = discover_collection_files(&dir.path().join("manifest.yaml")).unwrap();
    assert!(files.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].to_string().contains("invalid extension"));
}
```

- [ ] **Step 7: Write namespace validation test for file entries**

Namespace validation for file entry namespaces is handled automatically by `loader.rs`
(the same `validate_namespace()` check runs on every `CollectionFile`'s namespace).
Add a test to verify this works for file entries:

```rust
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
    assert!(result.unwrap_err().to_string().contains("invalid namespace"));
}
```

- [ ] **Step 8: Run all new tests**

Run: `cargo test -p fatescroll file_entry -- --nocapture`
Expected: all pass

- [ ] **Step 8: Run full test suite**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 9: Commit**

```bash
git add src/collection.rs tests/fixtures/file-entries-collection/
git commit -s -m "feat: discover file entries from manifest during collection loading"
```

---

## Task 4: Integration tests for file entries

**Files:**
- Modify: `tests/cli_integration.rs`

- [ ] **Step 1: Write integration test — validate collection with file entries**

Add to `tests/cli_integration.rs`:

```rust
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
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test validate_collection_with_file_entries roll_on_file_entry_table -- --nocapture`
Expected: PASS

- [ ] **Step 3: Write integration test — files-only manifest**

Create `tests/fixtures/files-only-collection/manifest.yaml`:
```yaml
name: Files Only Test
version: "1.0"
namespace: filesonly
author: ~
min_tool_version: ~
files:
  - path: ../file-entries-collection/shared/wilderness.yaml
    namespace: filesonly.terrain
```

Add test:
```rust
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
```

- [ ] **Step 4: Run test**

Run: `cargo test validate_files_only_manifest -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full test suite and clippy**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass, no warnings

- [ ] **Step 6: Commit**

```bash
git add tests/cli_integration.rs tests/fixtures/files-only-collection/
git commit -s -m "test: add integration tests for manifest file entries"
```

---

## Task 5: Update authoring guide documentation

**Files:**
- Modify: `docs/authoring-guide.md`

- [ ] **Step 1: Add files section to manifest schema documentation**

In `docs/authoring-guide.md`, after the `directories` field in the manifest schema table (around line 18), add `files` field documentation. Then add a subsection showing file entry usage with an example manifest that uses both `directories` and `files`.

Key content to add:
- `files` field description in the schema table
- `FileEntry` schema table (path, namespace)
- Example manifest using files
- Note about files-only manifests being supported
- Note that the table's `id` must match the filename stem (same as directory entries)

- [ ] **Step 2: Commit**

```bash
git add docs/authoring-guide.md
git commit -s -m "docs: add file entries section to authoring guide"
```

---

## Task 6: Update discovery test count

**Files:**
- Modify: `src/collection.rs` tests (if discovery count changed for valid-collection)

- [ ] **Step 1: Run full test suite one final time**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass, clippy clean

Note: The valid-collection fixture is unchanged, so the `discovers_all_files_in_valid_collection` test should still pass with count 11. If any tests break, fix them.

- [ ] **Step 2: Final commit if any fixes needed**
