# Multiple Manifests Design

## Overview

Support multiple collections in a shared directory by allowing named manifest files alongside the default `manifest.yaml`. Backward compatible — single-manifest directories work unchanged.

Covers bead fatescroll-rwt.

## Problem

Currently `resolve_collection` looks for exactly `manifest.yaml` in CWD or `--collection` directory. If two collections share a directory (e.g., two campaigns referencing shared NPC tables), there's no way to have multiple manifests.

## Design

### Naming Convention

- `manifest.yaml` — default manifest (backward compatible)
- `<name>.manifest.yaml` — named manifest (e.g., `campaign1.manifest.yaml`)

### Resolution Logic

`resolve_collection` changes from returning a collection directory to returning a **manifest file path**.

**When `--collection` is provided:**
1. If it points to a file → use it directly as the manifest
2. If it points to a directory → scan for manifests, require exactly one

**When falling back to CWD:**
1. Scan CWD for manifests, require exactly one

**Scanning:** Find `manifest.yaml` and `*.manifest.yaml` files in the directory.

**Disambiguation errors:**
- Zero manifests: "No collection found. Provide --collection or run from a collection directory."
- Multiple manifests: "Multiple manifests found: campaign1.manifest.yaml, campaign2.manifest.yaml. Specify one with --collection <path>."

### Downstream Changes

All `cmd_*` functions currently receive a collection directory and derive the manifest path as `dir.join("manifest.yaml")`. After this change:

- `resolve_collection` returns `PathBuf` (manifest file path, not directory)
- `load_collection` in `lib.rs` changes to accept a manifest path instead of a directory
- Collection directory derived as `manifest_path.parent()` where needed (e.g., import)
- `cmd_fix` already works with manifest paths internally — simplified

### Scope

- Named manifest file support
- Updated CWD detection with disambiguation
- `--collection` accepts both file and directory paths
- No changes to manifest format or content
- No multi-collection loading (one collection at a time)
