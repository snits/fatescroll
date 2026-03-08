# Default Collection Path & List Tags Design

## Overview

Two related CLI ergonomics features:
1. **Default collection path** (fatescroll-d4i): Eliminate mandatory `--collection` flag when running from a collection directory.
2. **List tags** (fatescroll-2rw): Add `search --tags` flag to list all unique tags in a collection.

## Feature 1: Default Collection Path

### Behavior

Resolution order:
1. If `--collection` is provided explicitly, use it.
2. Otherwise, check if `./manifest.yaml` exists in the current working directory.
3. If neither found, error: "No collection found. Provide --collection or run from a collection directory."

### CLI Changes

- `--collection` becomes `Option<PathBuf>` (optional) on all commands.
- `Validate` changes from positional to optional `--collection` with the same fallback.
- A `resolve_collection()` helper in `main.rs` handles the resolution logic.

### Design Decisions

- **No config file**: YAGNI. CWD detection plus explicit `--collection` covers the multi-collection workflow without added complexity.
- **No parent directory traversal**: Only checks CWD, avoiding permission concerns and surprising behavior.

## Feature 2: List Tags

### Behavior

`fatescroll search --tags` lists all unique tags across the loaded collection, sorted alphabetically, one per line.

### CLI Changes

- Add `--tags` boolean flag to `Search` command.
- Mutually exclusive with `--name`, `--tag`, `--namespace` via clap conflict group.
- Error message for "no search criteria" updated to include `--tags`.

### Implementation

- `collect_tags()` function in `search.rs`: iterates all tables, collects unique tags into a `BTreeSet<&str>`, returns it.
- CLI prints one tag per line (pipeable output).

## Testing

- **Collection resolution**: Test explicit path wins, CWD detection works, missing manifest errors.
- **`collect_tags`**: Unit test using existing `build_search_registry()` helper — verify sorted unique tags.
