# Design: Manifest File-Level Entries

## Problem

The manifest currently only supports `directories` entries, which load every YAML table file found in each listed directory. There is no way to selectively include individual files. This becomes problematic when an author wants to pull a few specific tables from a shared directory without importing everything in it.

## Solution

Add an optional `files` field to the manifest schema, parallel to `directories`. Each file entry specifies a path to a single YAML table file and the namespace it should be loaded under.

## Manifest Schema Change

New field added to `manifest.yaml`:

| Field | Type | Required | Description |
|---|---|---|---|
| `files` | list | no | List of individual file entries. Defaults to empty if omitted. |

Each entry in `files`:

| Field | Type | Description |
|---|---|---|
| `path` | string | Path to a single YAML table file. Can be relative to the manifest location. |
| `namespace` | string | Dot-separated namespace assigned to this table. |

### Example

```yaml
name: My Campaign
version: "1.0"
namespace: campaign
author: ~
min_tool_version: ~
directories:
  - path: encounters
    namespace: campaign.encounters
files:
  - path: ../shared/npc/npc-occupation.yaml
    namespace: campaign.npc
  - path: ../shared/npc/npc-quirk.yaml
    namespace: campaign.npc
```

## Implementation

### Data Model (models.rs)

Add `FileEntry` struct:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub namespace: String,
}
```

Add to `Manifest`:

```rust
#[serde(default)]
pub files: Vec<FileEntry>,
```

### Discovery (collection.rs)

In `discover_collection_files()`, after the existing directory iteration loop, add a second loop for file entries:

1. Resolve the file path relative to `manifest.base_path`
2. Validate the path exists and is a file (not a directory)
3. Validate it has a `.yaml` or `.yml` extension
4. Skip if it's a manifest file (`manifest.yaml` or `*.manifest.yaml`)
5. Extract the filename stem
6. Read file contents
7. Construct a `CollectionFile` with the entry's namespace and the stem

### Validation

- File must exist — soft error (accumulated, continues loading other entries), consistent with how `DirectoryNotFound` is handled for directory entries
- File must be a file, not a directory — soft error
- File must have `.yaml` or `.yml` extension — soft error
- Namespace must pass `validate_namespace()`
- The table's internal `id` field must match the filename stem (same constraint enforced for directory-discovered files via `IdFilenameMismatch` in loader.rs)
- Duplicate FQID detection works as-is — if a file entry and a directory entry both resolve to the same table with the same namespace, the registry produces a `DuplicateId` error. This is the correct behavior.
- Cross-reference validation works as-is (same resolution pipeline)

### Error Variants

Add to the error types:
- `FileEntryNotFound { path }` — file entry path does not exist
- `FileEntryNotAFile { path }` — file entry path is a directory, not a file
- `FileEntryInvalidExtension { path }` — file entry does not have `.yaml` or `.yml` extension

### Stale Reference Fixer (fixer.rs)

The fixer already operates on the registry after loading. File entries create the same `CollectionFile` structs, so no fixer changes needed.

## Backward Compatibility

Fully backward compatible. Existing manifests without a `files` field get an empty vec via `#[serde(default)]`. The `directories` field should also get `#[serde(default)]` to support files-only manifests. No migration needed.

## What This Does NOT Include

- Filtering within directory entries (e.g., include/exclude lists) — potential future enhancement
- Grouped file entries sharing a namespace — potential future ergonomic improvement
- Collection-awareness in the init command
