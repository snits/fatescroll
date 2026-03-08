# Stale Reference Detection & Warnings Design

## Overview

When `fix_collection` corrects an id field (e.g., `wolf-counter` → `wolf-count`), chain and compound table references in other files that used the old id become stale. This design adds detection of stale references with warnings by default, and optional auto-fix with an explicit flag.

Covers beads fatescroll-3cf and fatescroll-6mw.

## Problem

1. File `wolf-count.yaml` has `id: wolf-counter` (wrong — doesn't match filename)
2. File `wilderness-encounter.yaml` has `chain: [wolf-counter]`
3. `validate --fix` corrects the id to `wolf-count`
4. Now `chain: [wolf-counter]` is broken — should be `chain: [wolf-count]`
5. The validator catches this post-hoc, but the user must fix manually

## Design

### Behavior

- **Default** (`validate --fix`): Correct ids. Warn about stale references but don't touch them.
- **Forced** (`validate --fix --update-refs`): Correct ids AND update stale references.

### Detection Algorithm

Two-pass approach within `fix_collection`:

1. **First pass** (existing): Fix id fields. Collect `Corrected { old_id, new_id }` mappings.
2. **Second pass** (new): If any corrections were made, **re-read all files from disk** (to pick up pass 1's writes), then scan for chain/tables references that match any `old_id`. Report each match as a `FixWarning::StaleReference`.

**Critical: Pass 2 must re-read from disk, not use stale in-memory contents.** Files modified in pass 1 would have their id corrections overwritten if pass 2 parses old contents and writes back. Re-reading is cheap at this tool's scale.

**Write batching:** Pass 2 groups all reference updates by file — parse once, update all stale references in that file, write once. Report one `FixAction::UpdatedReference` per individual reference updated.

### Data Model Changes

```rust
/// How to handle stale references found during fix.
pub enum RefHandling {
    WarnOnly,
    Update,
}

pub enum FixWarning {
    StaleReference {
        path: PathBuf,      // file containing the stale reference
        reference: String,  // the stale reference value (old_id)
        suggested: String,  // what it should be (new_id)
    },
}

pub struct FixResult {
    pub actions: Vec<FixAction>,
    pub warnings: Vec<FixWarning>,  // NEW
    pub errors: Vec<Error>,
}
```

### fix_collection Signature Change

```rust
pub fn fix_collection(manifest_path: &Path, ref_handling: RefHandling) -> Result<FixResult, Error>
```

When `ref_handling` is `Update` and stale references are found, the fixer replaces old_id with new_id in the chain/tables arrays and writes the file back. These get reported as a new `FixAction` variant:

```rust
FixAction::UpdatedReference {
    path: PathBuf,
    old_ref: String,
    new_ref: String,
}
```

When `ref_handling` is `WarnOnly`, stale references are reported as warnings only.

### Safety: False Positive Guard

When `ref_handling` is `Update`, before replacing a reference, verify that the `new_id` actually exists as a valid file stem in the collection. This prevents false positives where a reference coincidentally matches an old_id but actually refers to a different table.

### Edge Case: Duplicate old_id Collisions

If two files in different directories both had the same wrong id (e.g., both had `id: wrong`), the old_id→new_id map would have a collision. This is unlikely since the validator catches duplicate ids, but the implementation should use a `HashMap<String, Vec<String>>` or warn if a collision is detected rather than silently picking one.

### CLI Changes

- Add `--update-refs` flag to the `Validate` command (requires `--fix`)
- `cmd_fix` prints warnings after actions, before errors
- Warning format: `Warning: stale reference 'wolf-counter' in path/to/file.yaml (should be 'wolf-count')`

### Reference Scan Details

References live in two places in the YAML:
- `results[].chain[]` — array of strings in Simple tables
- `tables[]` — array of strings in Compound tables

The second pass re-reads each file from disk as `serde_yaml::Value`, navigates to these arrays, and checks each string against the old_id→new_id map.

### Scope Limitations

- Only detects bare id references matching corrected old_ids
- Does NOT handle FQID-based references (e.g., `test.encounters.wolf-counter`) — that requires registry-level resolution (fatescroll-s33)
- Does NOT detect references that were already broken before the fix

## Testing Strategy

- **Unit test:** Collection with file A referencing file B's old (wrong) id. After fix with `WarnOnly`, verify warning is emitted for the stale reference.
- **Unit test:** Same setup with `Update`. Verify the reference is updated in the file.
- **Unit test:** File needing both id correction AND reference update (e.g., wrong id AND stale chain reference). Verify both corrections survive — neither overwrites the other.
- **Unit test:** No corrections → no warnings (second pass skipped).
- **Unit test:** Existing callers pass `WarnOnly` and behavior is unchanged.
- **Integration test:** `validate --fix` shows warning output. `validate --fix --update-refs` shows updated reference output.
