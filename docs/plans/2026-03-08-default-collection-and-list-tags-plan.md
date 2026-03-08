# Default Collection Path, List Tags & Error Variant Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `--collection` optional by detecting CWD collections, add `search --tags` to list unique tags, and fix the misused error variant in fixer.

**Architecture:** Collection resolution is a CLI-level concern in `main.rs`. Tag listing is a search-layer function in `search.rs`. The error variant fix is in `error.rs` + `fixer.rs`.

**Tech Stack:** Rust, clap 4 (derive), serde_yaml, thiserror 2, BTreeSet

---

### Task 1: Add `collect_tags` to search.rs (fatescroll-2rw)

**Files:**
- Modify: `src/search.rs`

**Context:** `search.rs` has three search functions that all take `&Registry` and return `Vec<(&str, &Table)>`. The `build_search_registry()` test helper creates a registry with tags: `["treasure", "gems"]`, `["encounter", "wilderness"]`, `["npc"]`.

**Step 1: Write the failing test**

Add to `search::tests` module in `src/search.rs`:

```rust
#[test]
fn collect_tags_returns_sorted_unique_tags() {
    let reg = build_search_registry();
    let tags = collect_tags(&reg);
    assert_eq!(
        tags.iter().copied().collect::<Vec<_>>(),
        vec!["encounter", "gems", "npc", "treasure", "wilderness"]
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test search::tests::collect_tags_returns_sorted_unique_tags`
Expected: FAIL — `collect_tags` not found

**Step 3: Write minimal implementation**

Add to `src/search.rs` (after the existing search functions, before `#[cfg(test)]`):

```rust
use std::collections::BTreeSet;

/// Collect all unique tags across all tables, sorted alphabetically.
pub fn collect_tags<'a>(registry: &'a Registry) -> BTreeSet<&'a str> {
    registry
        .all_tables()
        .flat_map(|(_, table)| table.tags().iter().map(|t| t.as_str()))
        .collect()
}
```

Note: The `use` statement for `BTreeSet` should go at the top of the file with the other imports.

**Step 4: Run test to verify it passes**

Run: `cargo test search::tests::collect_tags_returns_sorted_unique_tags`
Expected: PASS

**Step 5: Write test for empty registry**

```rust
#[test]
fn collect_tags_empty_registry() {
    let reg = Registry::new();
    let tags = collect_tags(&reg);
    assert!(tags.is_empty());
}
```

Run: `cargo test search::tests::collect_tags_empty`
Expected: PASS

**Step 6: Commit**

```bash
git add src/search.rs
git commit -s -m "feat: add collect_tags function to search module

Collects all unique tags across tables in a registry, returned as a
sorted BTreeSet. Supports the search --tags CLI feature.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 2: Make --collection optional with CWD fallback (fatescroll-d4i)

**Files:**
- Modify: `src/main.rs`

**Context:** Currently all commands have `collection: PathBuf` as required. The `Validate` command uses it as a positional arg, others use `--collection` flag. We need to make them all optional and add a `resolve_collection()` helper.

**Step 1: Add resolve_collection helper and update CLI struct**

Change `--collection` from `PathBuf` to `Option<PathBuf>` on all commands. Change `Validate` from positional to `--collection` flag too (for consistency).

Update the `Commands` enum:

```rust
#[derive(Subcommand)]
enum Commands {
    /// Validate a table collection
    Validate {
        /// Path to collection directory (containing manifest.yaml)
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Automatically fix id field issues
        #[arg(long)]
        fix: bool,
    },
    /// Roll on a table
    Roll {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
    },
    /// Search for tables
    Search {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Search by table name
        #[arg(long)]
        name: Option<String>,
        /// Search by tag
        #[arg(long)]
        tag: Option<String>,
        /// Search by namespace
        #[arg(long)]
        namespace: Option<String>,
    },
    /// Import table files into a collection
    Import {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Directory within the collection to import into
        #[arg(long)]
        target_dir: String,
        /// Files to import
        files: Vec<PathBuf>,
    },
}
```

Add a `resolve_collection` function:

```rust
/// Resolve the collection path from explicit flag or CWD detection.
fn resolve_collection(explicit: Option<PathBuf>) -> Result<PathBuf, fatescroll::Error> {
    if let Some(path) = explicit {
        return Ok(path);
    }

    let cwd = std::env::current_dir()?;
    if cwd.join("manifest.yaml").exists() {
        return Ok(cwd);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No collection found. Provide --collection or run from a collection directory.",
    )
    .into())
}
```

Update the `main()` match arms to call `resolve_collection()`:

```rust
let result = match cli.command {
    Commands::Validate { collection, fix } => {
        let collection = resolve_collection(collection)?;
        if fix {
            cmd_fix(&collection)
        } else {
            cmd_validate(&collection)
        }
    }
    Commands::Roll {
        collection,
        table_id,
    } => {
        let collection = resolve_collection(collection)?;
        cmd_roll(&collection, &table_id)
    }
    Commands::Search {
        collection,
        name,
        tag,
        namespace,
    } => {
        let collection = resolve_collection(collection)?;
        cmd_search(&collection, name.as_deref(), tag.as_deref(), namespace.as_deref())
    }
    Commands::Import {
        collection,
        target_dir,
        files,
    } => {
        let collection = resolve_collection(collection)?;
        cmd_import(&collection, &target_dir, &files)
    }
};
```

Note: The `main()` function currently doesn't use `?` — it handles errors with `if let Err`. The `resolve_collection` call needs to be inside the match arms before calling `cmd_*` functions, and its error needs to flow through the existing error handling. Since `cmd_*` functions return `Result<(), fatescroll::Error>`, put the `resolve_collection` call inside each arm:

```rust
Commands::Validate { collection, fix } => {
    let collection = resolve_collection(collection)?;
    // ... rest unchanged
}
```

Wait — `main()` doesn't return `Result`. The outer match returns `Result<(), fatescroll::Error>` via `result`, so `?` works inside the match arms as long as the arm returns `Result<(), fatescroll::Error>`. Actually, looking more carefully, each arm calls a function that returns `Result<(), fatescroll::Error>`, and `result` captures that. To use `?` for `resolve_collection`, wrap each arm in a closure or block that returns `Result`. The simplest approach: call resolve_collection and handle its error the same way:

```rust
Commands::Roll { collection, table_id } => {
    resolve_collection(collection).and_then(|c| cmd_roll(&c, &table_id))
}
```

Or use a block:

```rust
Commands::Roll { collection, table_id } => {
    let collection = resolve_collection(collection)?;
    cmd_roll(&collection, &table_id)
}
```

The `?` works here because the match expression is bound to `result: Result<(), fatescroll::Error>`, and each arm is a block expression returning that same Result type. This is valid Rust — `?` works in blocks that evaluate to `Result`.

**Step 2: Run all tests to verify nothing broke**

Run: `cargo test`
Expected: All existing tests PASS, binary compiles

**Step 3: Commit**

```bash
git add src/main.rs
git commit -s -m "feat: make --collection optional with CWD fallback

Collection path is now resolved by: (1) explicit --collection flag,
(2) current working directory if manifest.yaml exists there,
(3) error with helpful message if neither found.

The validate command changes from positional to --collection flag for
consistency with other commands.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 3: Add --tags flag to search command (fatescroll-2rw)

**Files:**
- Modify: `src/main.rs`

**Context:** The `Search` command needs a `--tags` boolean flag that conflicts with `--name`, `--tag`, `--namespace`. When provided, it calls `collect_tags()` and prints one tag per line.

**Step 1: Add --tags flag to Search variant**

Add to the `Search` variant in `Commands` enum:

```rust
/// List all unique tags in the collection
#[arg(long, conflicts_with_all = ["name", "tag", "namespace"])]
tags: bool,
```

**Step 2: Update cmd_search signature and main() call**

Update the `Search` match arm in `main()` to pass `tags`:

```rust
Commands::Search {
    collection,
    name,
    tag,
    namespace,
    tags,
} => {
    let collection = resolve_collection(collection)?;
    cmd_search(&collection, name.as_deref(), tag.as_deref(), namespace.as_deref(), tags)
}
```

Update `cmd_search` signature to accept `tags: bool`:

```rust
fn cmd_search(
    collection: &Path,
    name: Option<&str>,
    tag: Option<&str>,
    namespace: Option<&str>,
    tags: bool,
) -> Result<(), fatescroll::Error> {
    let registry = fatescroll::load_collection(collection)?;

    if tags {
        let all_tags = fatescroll::search::collect_tags(&registry);
        if all_tags.is_empty() {
            println!("No tags found.");
        } else {
            for tag in &all_tags {
                println!("{tag}");
            }
        }
        return Ok(());
    }

    // ... rest of existing search logic unchanged ...
```

Update the error message for no criteria to include `--tags`:

```rust
return Err(std::io::Error::new(
    std::io::ErrorKind::InvalidInput,
    "specify --name, --tag, --namespace, or --tags",
)
.into());
```

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add src/main.rs
git commit -s -m "feat: add search --tags flag to list collection tags

Lists all unique tags across a collection, sorted alphabetically,
one per line. Mutually exclusive with --name, --tag, --namespace.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 4: Add proper error variant for fixer (fatescroll-owq)

**Files:**
- Modify: `src/error.rs`
- Modify: `src/fixer.rs`

**Context:** In `fixer.rs:59-65`, `LoadError::FileRead` is used for "YAML root is not a mapping" which is semantically wrong — it's not a file read failure, it's a structural problem with the YAML content.

**Step 1: Write failing test**

Add to `fixer::tests` in `src/fixer.rs`:

```rust
#[test]
fn fix_reports_non_mapping_yaml_as_format_error() {
    let dir = setup_collection(&[("scalar.yaml", "just a string, not a mapping")]);
    let manifest = dir.path().join("manifest.yaml");
    let result = fix_collection(&manifest).unwrap();

    assert!(result.actions.is_empty());
    assert_eq!(result.errors.len(), 1);

    // Verify the error is a LoadError::InvalidFormat, not FileRead
    match &result.errors[0] {
        Error::Load(LoadError::InvalidFormat { path, .. }) => {
            assert!(path.to_string_lossy().contains("scalar.yaml"));
        }
        other => panic!("Expected LoadError::InvalidFormat, got: {other:?}"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test fixer::tests::fix_reports_non_mapping_yaml_as_format_error`
Expected: FAIL — `InvalidFormat` variant doesn't exist

**Step 3: Add InvalidFormat variant to LoadError**

In `src/error.rs`, add to the `LoadError` enum (after `FileRead`):

```rust
#[error("invalid format in {path}: {reason}")]
InvalidFormat { path: PathBuf, reason: String },
```

**Step 4: Update fixer.rs to use the variant**

In `src/fixer.rs:59-65`, change `LoadError::FileRead` to `LoadError::InvalidFormat`:

```rust
None => {
    result.errors.push(
        LoadError::InvalidFormat {
            path: file.path.clone(),
            reason: "YAML root is not a mapping".into(),
        }
        .into(),
    );
    continue;
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test fixer::tests::fix_reports_non_mapping_yaml_as_format_error`
Expected: PASS

**Step 6: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add src/error.rs src/fixer.rs
git commit -s -m "fix: use dedicated error variant for invalid YAML format

Add LoadError::InvalidFormat for structural YAML problems like
non-mapping roots. The fixer previously used LoadError::FileRead
for this case, which was semantically incorrect.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 5: Final verification and cleanup

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Verify binary works**

Run from a collection directory:
```bash
cd /path/to/test/collection
fatescroll search --tags
fatescroll search --name something
fatescroll validate
```

Run with explicit collection:
```bash
fatescroll search --collection /path/to/collection --tags
```

Run from non-collection directory without --collection:
```bash
cd /tmp
fatescroll search --tags
# Expected: error about no collection found
```
