# Extract Shared Collection File Discovery

## Problem

loader.rs and fixer.rs contain ~40 lines of identical boilerplate: read manifest,
parse YAML, set base_path, iterate directories, read_dir, filter for .yaml/.yml,
extract file_stem, read contents.

## Design

### New file: `src/collection.rs`

**Struct:**

```rust
pub struct CollectionFile {
    pub path: PathBuf,
    pub namespace: String,
    pub stem: String,
    pub contents: String,
}
```

**Function:**

```rust
pub fn discover_collection_files(manifest_path: &Path) -> Result<(Vec<CollectionFile>, Vec<Error>), Error>
```

Handles manifest reading/parsing, directory iteration, file filtering (.yaml/.yml,
skip manifest.yaml), stem extraction, and content reading.

Return semantics:
- `Err(Error)`: unrecoverable (manifest not found, manifest unparseable)
- `Ok((files, errors))`: per-file IO errors accumulated in errors vec

### Changes to loader.rs

Replace manifest reading + directory walking + file reading with
`discover_collection_files`. Keep namespace validation, Table deserialization,
id-filename check, validate_table, and registry registration.

### Changes to fixer.rs

Replace manifest reading + directory walking + file reading with
`discover_collection_files`. Keep id field patching logic.

### Testing

- collection.rs gets discovery-focused tests
- Existing loader and fixer tests unchanged (end-to-end coverage)
