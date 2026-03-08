# Multiple Manifests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Support named manifest files (`<name>.manifest.yaml`) alongside `manifest.yaml` for multi-collection directories.

**Architecture:** Change `resolve_collection` to return a manifest file path instead of a directory. Update `load_collection` API to accept manifest path. Scan for `manifest.yaml` and `*.manifest.yaml` when detecting collections.

**Tech Stack:** Rust, clap 4, std::fs

---

### Task 1: Change load_collection to accept manifest path

**Files:**
- Modify: `src/lib.rs`

**Step 1: Write failing test**

The existing test in `loader.rs` already calls `load_collection` with a manifest path. The public API in `lib.rs` takes a directory. Change `lib.rs::load_collection` to accept a manifest path directly.

Since this is a signature change, update the doc comment and parameter name:

```rust
/// Load and validate a collection from a manifest file path.
pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    let registry = loader::load_collection(manifest_path)?;

    if let Err(errors) = validator::validate_references(&registry) {
        return Err(error::LoadError::Multiple {
            errors: errors.into_iter().map(Error::from).collect(),
        }
        .into());
    }

    Ok(registry)
}
```

This removes the `collection_dir.join("manifest.yaml")` line — callers now provide the manifest path directly.

**Step 2: Update all callers in main.rs**

Every `cmd_*` function that calls `fatescroll::load_collection(collection)` needs to change. Since `resolve_collection` will return a manifest path after Task 2, but we need the code to compile NOW, temporarily change `cmd_*` functions to do `collection.join("manifest.yaml")` themselves.

Actually, a simpler approach: do Task 1 and Task 2 together as one step since they're tightly coupled. But for TDD cleanliness, let's keep them separate.

For now, update each call site:
- `cmd_validate`: `fatescroll::load_collection(&collection.join("manifest.yaml"))?`
- `cmd_roll`: same
- `cmd_show`: same
- `cmd_search`: same
- `cmd_import`: same (the validate call at the end)
- `cmd_fix`: already uses `collection.join("manifest.yaml")` for `fix_collection` — also update the implicit `load_collection` if any

Wait — `cmd_fix` doesn't call `load_collection`, it calls `fix_collection` with a manifest path already. So only `cmd_validate`, `cmd_roll`, `cmd_show`, `cmd_search`, and `cmd_import` need updating.

**Step 3: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`

**Step 4: Commit**

```bash
git add src/lib.rs src/main.rs
git commit -s -m "refactor: change load_collection to accept manifest path

The public API now takes a manifest file path directly instead of
deriving it from a collection directory. Callers provide the path.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 2: Update resolve_collection to return manifest path

**Files:**
- Modify: `src/main.rs`

**Step 1: Add find_manifests helper**

Add before `resolve_collection`:

```rust
/// Find all manifest files in a directory.
fn find_manifests(dir: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    let default = dir.join("manifest.yaml");
    if default.exists() {
        manifests.push(default);
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".manifest.yaml") {
                manifests.push(entry.path());
            }
        }
    }
    manifests.sort();
    manifests
}
```

**Step 2: Update resolve_collection**

```rust
/// Resolve the manifest path from explicit flag or CWD detection.
///
/// Accepts either a manifest file path or a directory containing manifests.
/// When scanning a directory, requires exactly one manifest file.
fn resolve_collection(explicit: Option<PathBuf>) -> Result<PathBuf, fatescroll::Error> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path);
        }
        if path.is_dir() {
            return resolve_manifest_in_dir(&path);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path not found: {}", path.display()),
        )
        .into());
    }

    let cwd = std::env::current_dir()?;
    resolve_manifest_in_dir(&cwd)
}

fn resolve_manifest_in_dir(dir: &Path) -> Result<PathBuf, fatescroll::Error> {
    let manifests = find_manifests(dir);
    match manifests.len() {
        0 => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No collection found. Provide --collection or run from a collection directory.",
        )
        .into()),
        1 => Ok(manifests.into_iter().next().unwrap()),
        _ => {
            let names: Vec<String> = manifests
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect();
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Multiple manifests found: {}. Specify one with --collection <path>.",
                    names.join(", ")
                ),
            )
            .into())
        }
    }
}
```

**Step 3: Update cmd_* functions**

Now that `resolve_collection` returns a manifest path, remove the `.join("manifest.yaml")` calls added in Task 1. Each `cmd_*` function receives the manifest path directly:

- `cmd_validate`: `fatescroll::load_collection(&collection)?`
- `cmd_roll`: `fatescroll::load_collection(&collection)?`
- `cmd_show`: `fatescroll::load_collection(&collection)?`
- `cmd_search`: `fatescroll::load_collection(&collection)?`
- `cmd_fix`: `fatescroll::fixer::fix_collection(&collection, ref_handling)?`
- `cmd_import`: needs the directory for file copying, so derive it: `let collection_dir = collection.parent().unwrap();` then use `collection_dir` for the dest path and `&collection` for load_collection.

Also update the `--collection` help text from "Path to collection directory" to "Path to collection directory or manifest file".

Rename the `collection` variable in match arms to `manifest` for clarity, OR keep as `collection` since the resolve function handles both. Keep as `collection` for minimal diff.

**Step 4: Run all tests**

Run: `cargo test && cargo clippy -- -D warnings`
Note: integration tests that use `--collection <dir>` should still work since `resolve_collection` handles directories.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -s -m "feat: support named manifests and manifest file paths

resolve_collection now returns a manifest file path. Accepts both
directory paths (scans for manifests) and direct file paths.
Supports <name>.manifest.yaml alongside manifest.yaml.
Errors clearly when multiple manifests found in a directory.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 3: Add tests for multi-manifest behavior

**Files:**
- Modify: `tests/cli_integration.rs`

**Step 1: Test single named manifest works**

Create a temp dir with `campaign.manifest.yaml` (no `manifest.yaml`). Run a command with `--collection <dir>`. Should succeed.

**Step 2: Test multiple manifests error**

Create a temp dir with both `manifest.yaml` and `other.manifest.yaml`. Run a command with `--collection <dir>`. Should fail with "Multiple manifests found" error.

**Step 3: Test direct manifest file path**

Create a temp dir with `campaign.manifest.yaml`. Run with `--collection <dir>/campaign.manifest.yaml`. Should succeed.

**Step 4: Run all tests**

Run: `cargo test && cargo clippy -- -D warnings`

**Step 5: Commit**

```bash
git add tests/cli_integration.rs
git commit -s -m "test: add integration tests for named manifest support

Tests cover: single named manifest in directory, multiple manifests
disambiguation error, and direct manifest file path.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 4: Final verification

**Step 1:** Run `cargo test && cargo clippy -- -D warnings`
**Step 2:** Smoke test with fixture collection
**Step 3:** Review `git log --oneline feature/stale-reference-detection..HEAD`
