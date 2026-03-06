# Fatescroll v0.1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build fatescroll, a Rust library with CLI for managing and rolling on YAML-based RPG random tables with dice evaluation via diceman.

**Architecture:** Library crate (`src/lib.rs`) with thin CLI binary (`src/main.rs`). Tables are YAML files organized in directory-based collections with manifests. Core pipeline: load manifest → discover files → parse YAML → validate → populate registry → roll/search. Dice evaluation delegated to the diceman crate (local path dependency). Results returned as a tree structure (`RollResult`) for consumer-controlled formatting.

**Tech Stack:** Rust 2024 edition, serde + serde_yaml (YAML parsing), clap 4 with derive (CLI), diceman (dice evaluation, path dep at `../diceman/crates/diceman`), thiserror 2 (error types), regex (dice interpolation in result text)

**Key reference:** Design doc at `plans/rpg-random-table-tool.md`

---

## Dependency Graph

```
Task 1: Project Setup
  └→ Task 2: Error Types
       └→ Task 3: Data Models & Deserialization
            ├→ Task 4: Validation (per-type)
            │    └→ Task 6: Loader
            │         └→ Task 7: Cross-Reference Validation
            ├→ Task 5: Registry
            │    ├→ Task 6 (also)
            │    ├→ Task 8: Roller
            │    └→ Task 9: Search
            └→ Task 10: CLI (depends on all above)
```

Tasks 8 and 9 are independent of each other and of tasks 6-7. They can be parallelized.

---

## Test Fixtures

Create these files as part of Task 1. All subsequent tasks reuse them.

### `tests/fixtures/valid-collection/manifest.yaml`

```yaml
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
  - path: encounters
    namespace: test.encounters
  - path: npc
    namespace: test.npc
```

### `tests/fixtures/valid-collection/terrain/wilderness.yaml`

```yaml
name: Wilderness Terrain
type: simple
tags:
  - terrain
  - wilderness
roll: 1d6
results:
  - min: 1
    max: 3
    text: Dense forest
  - min: 4
    max: 5
    text: Open plains
  - min: 6
    max: 6
    text: Rocky hills
```

### `tests/fixtures/valid-collection/encounters/wilderness-encounter.yaml`

```yaml
name: Wilderness Encounter
type: simple
tags:
  - encounter
  - wilderness
roll: 1d8
results:
  - min: 1
    max: 3
    text: Animal encounter
    chain:
      - animal-type
  - min: 4
    max: 5
    text: Bandit camp
    chain:
      - bandit-strength
      - bandit-motivation
  - min: 6
    max: 7
    text: Abandoned campsite
  - min: 8
    max: 8
    text: "Merchant with {2d6x10} gold"
    chain:
      - merchant-goods
```

### `tests/fixtures/valid-collection/encounters/animal-type.yaml`

```yaml
name: Animal Type
type: simple
tags:
  - encounter
  - animal
roll: 1d4
results:
  - min: 1
    max: 1
    text: Wolves
  - min: 2
    max: 2
    text: Bear
  - min: 3
    max: 3
    text: Deer
  - min: 4
    max: 4
    text: Wild boar
```

### `tests/fixtures/valid-collection/npc/npc-occupation.yaml`

```yaml
name: NPC Occupation
type: simple
tags:
  - npc
roll: 1d6
results:
  - min: 1
    max: 1
    text: Blacksmith
  - min: 2
    max: 2
    text: Merchant
  - min: 3
    max: 3
    text: Scholar
  - min: 4
    max: 4
    text: Farmer
  - min: 5
    max: 5
    text: Soldier
  - min: 6
    max: 6
    text: Priest
```

### `tests/fixtures/valid-collection/npc/npc-disposition.yaml`

```yaml
name: NPC Disposition
type: simple
tags:
  - npc
roll: 1d4
results:
  - min: 1
    max: 1
    text: Friendly
  - min: 2
    max: 2
    text: Neutral
  - min: 3
    max: 3
    text: Suspicious
  - min: 4
    max: 4
    text: Hostile
```

### `tests/fixtures/valid-collection/npc/npc-quirk.yaml`

```yaml
name: NPC Quirk
type: simple
tags:
  - npc
roll: 1d6
results:
  - min: 1
    max: 1
    text: Speaks in riddles
  - min: 2
    max: 2
    text: Missing a finger
  - min: 3
    max: 3
    text: Hums constantly
  - min: 4
    max: 4
    text: Paranoid about magic
  - min: 5
    max: 5
    text: Collects odd trinkets
  - min: 6
    max: 6
    text: Has a pet raven
```

### `tests/fixtures/valid-collection/npc/quick-npc.yaml`

```yaml
name: Quick NPC Generator
type: compound
tags:
  - npc
  - generator
tables:
  - npc-occupation
  - npc-disposition
  - npc-quirk
```

### `tests/fixtures/invalid-collection/manifest.yaml`

```yaml
name: Invalid Collection
version: "1.0"
namespace: invalid
author: ~
min_tool_version: ~
directories:
  - path: tables
    namespace: invalid.tables
```

### `tests/fixtures/invalid-collection/tables/gap-ranges.yaml`

```yaml
name: Gap Ranges Table
type: simple
tags: []
roll: 1d6
results:
  - min: 1
    max: 2
    text: Low
  - min: 5
    max: 6
    text: High
```

### `tests/fixtures/invalid-collection/tables/overlap-ranges.yaml`

```yaml
name: Overlap Ranges Table
type: simple
tags: []
roll: 1d6
results:
  - min: 1
    max: 4
    text: Low
  - min: 3
    max: 6
    text: High
```

### `tests/fixtures/invalid-collection/tables/bad-chain.yaml`

```yaml
name: Bad Chain Table
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 2
    text: Something
    chain:
      - nonexistent-table
  - min: 3
    max: 4
    text: Something else
```

### `tests/fixtures/invalid-collection/tables/bad-compound.yaml`

```yaml
name: Bad Compound
type: compound
tags: []
tables:
  - nonexistent-table-a
  - nonexistent-table-b
```

### `tests/fixtures/invalid-collection/tables/bad-dice.yaml`

```yaml
name: Bad Dice Expression
type: simple
tags: []
roll: 1z6
results:
  - min: 1
    max: 6
    text: Something
```

### `tests/fixtures/invalid-collection/tables/reversed-range.yaml`

```yaml
name: Reversed Range
type: simple
tags: []
roll: 1d6
results:
  - min: 5
    max: 2
    text: Backwards
  - min: 1
    max: 1
    text: One
  - min: 3
    max: 4
    text: Middle
  - min: 6
    max: 6
    text: Six
```

---

## Task 1: Project Setup & Crate Skeleton

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs` (replace cargo init stub)
- Create: `src/lib.rs`
- Create: `src/error.rs`
- Create: `src/models.rs`
- Create: `src/registry.rs`
- Create: `src/loader.rs`
- Create: `src/validator.rs`
- Create: `src/roller.rs`
- Create: `src/search.rs`
- Create: all test fixture files listed above

**Step 1: Update Cargo.toml**

```toml
[package]
name = "fatescroll"
version = "0.1.0"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
diceman = { path = "../diceman/crates/diceman" }
regex = "1"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create src/lib.rs**

```rust
// ABOUTME: Public API for the fatescroll random table library.
// ABOUTME: Re-exports core types and provides top-level convenience functions.

pub mod error;
pub mod loader;
pub mod models;
pub mod registry;
pub mod roller;
pub mod search;
pub mod validator;
```

**Step 3: Create module stubs**

Each module file gets ABOUTME comments and placeholder content. Example for `src/error.rs`:

```rust
// ABOUTME: Error types for fatescroll operations.
// ABOUTME: Covers validation, loading, rolling, and search errors.
```

Similarly for all other modules (`models.rs`, `registry.rs`, `loader.rs`, `validator.rs`, `roller.rs`, `search.rs`).

**Step 4: Replace src/main.rs**

```rust
// ABOUTME: CLI binary for fatescroll random table tool.
// ABOUTME: Thin wrapper over the fatescroll library using clap.

fn main() {
    println!("fatescroll v0.1.0");
}
```

**Step 5: Create all test fixture files**

Create the directory structure and all YAML files listed in the Test Fixtures section above.

**Step 6: Verify the project compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 7: Commit**

```bash
git add -A
git commit -s -m "feat: project skeleton with module stubs and test fixtures

Set up crate structure with all module files, dependencies
(diceman, serde, clap, regex, thiserror), and YAML test
fixtures for both valid and invalid collections.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 2: Error Types

**Files:**
- Modify: `src/error.rs`
- Test: `src/error.rs` (inline tests)

**Step 1: Write the error types**

```rust
// ABOUTME: Error types for fatescroll operations.
// ABOUTME: Covers validation, loading, rolling, and search errors.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("loading error: {0}")]
    Load(#[from] LoadError),

    #[error("roll error: {0}")]
    Roll(#[from] RollError),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("dice error: {0}")]
    Dice(#[from] diceman::Error),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("range reversed: min {min} > max {max} in table '{table}'")]
    RangeReversed { table: String, min: u32, max: u32 },

    #[error("range gap in table '{table}': missing values {missing:?}")]
    RangeGap { table: String, missing: Vec<u32> },

    #[error("range overlap in table '{table}': values {overlapping:?} covered multiple times")]
    RangeOverlap { table: String, overlapping: Vec<u32> },

    #[error("invalid dice expression '{expr}' in table '{table}': {reason}")]
    InvalidDiceExpression { table: String, expr: String, reason: String },

    #[error("invalid namespace '{namespace}': {reason}")]
    InvalidNamespace { namespace: String, reason: String },

    #[error("directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("unresolved chain reference '{reference}' in table '{table}'")]
    UnresolvedChain { table: String, reference: String },

    #[error("unresolved compound table reference '{reference}' in table '{table}'")]
    UnresolvedCompoundRef { table: String, reference: String },

    #[error("duplicate table ID '{id}'")]
    DuplicateId { id: String },
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("manifest not found at {path}")]
    ManifestNotFound { path: PathBuf },

    #[error("failed to read file {path}: {reason}")]
    FileRead { path: PathBuf, reason: String },

    #[error("multiple errors during load:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    Multiple { errors: Vec<Error> },
}

#[derive(Debug, Error)]
pub enum RollError {
    #[error("table not found: '{id}'")]
    TableNotFound { id: String },

    #[error("roll value {value} out of range for table '{table}'")]
    RollOutOfRange { table: String, value: i64 },

    #[error("chain depth limit ({limit}) exceeded at table '{table}'")]
    ChainDepthExceeded { table: String, limit: usize },

    #[error("negative dice result ({value}) not supported")]
    NegativeRoll { value: i64 },
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add src/error.rs
git commit -s -m "feat: define error types for validation, loading, and rolling

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 3: Data Models & Deserialization

**Files:**
- Modify: `src/models.rs`
- Test: `src/models.rs` (inline tests)

**Step 1: Write failing tests for YAML deserialization**

Add to `src/models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_simple_table() {
        let yaml = r#"
name: Test Table
type: simple
tags:
  - test
roll: 1d6
results:
  - min: 1
    max: 3
    text: Low
  - min: 4
    max: 6
    text: High
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple { name, tags, roll, results } => {
                assert_eq!(name, "Test Table");
                assert_eq!(tags, vec!["test"]);
                assert_eq!(roll, "1d6");
                assert_eq!(results.len(), 2);
                assert_eq!(results[0].min, 1);
                assert_eq!(results[0].max, 3);
                assert_eq!(results[0].text.as_deref(), Some("Low"));
                assert!(results[0].chain.is_none());
            }
            _ => panic!("Expected Simple table"),
        }
    }

    #[test]
    fn deserialize_simple_table_with_chains() {
        let yaml = r#"
name: Encounter
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 2
    text: Wolves
    chain:
      - wolf-count
  - min: 3
    max: 4
    text: Bandits
    chain:
      - bandit-strength
      - bandit-motivation
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple { results, .. } => {
                assert_eq!(results[0].chain.as_ref().unwrap(), &["wolf-count"]);
                assert_eq!(
                    results[1].chain.as_ref().unwrap(),
                    &["bandit-strength", "bandit-motivation"]
                );
            }
            _ => panic!("Expected Simple table"),
        }
    }

    #[test]
    fn deserialize_compound_table() {
        let yaml = r#"
name: Quick NPC
type: compound
tags:
  - npc
  - generator
tables:
  - npc-occupation
  - npc-disposition
  - npc-quirk
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Compound { name, tags, tables } => {
                assert_eq!(name, "Quick NPC");
                assert_eq!(tags, vec!["npc", "generator"]);
                assert_eq!(tables, vec!["npc-occupation", "npc-disposition", "npc-quirk"]);
            }
            _ => panic!("Expected Compound table"),
        }
    }

    #[test]
    fn deserialize_manifest() {
        let yaml = r#"
name: Test Collection
version: "1.0"
namespace: test
author: ~
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
  - path: encounters
    namespace: test.encounters
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "Test Collection");
        assert_eq!(manifest.version, "1.0");
        assert_eq!(manifest.namespace, "test");
        assert!(manifest.author.is_none());
        assert_eq!(manifest.directories.len(), 2);
        assert_eq!(manifest.directories[0].namespace, "test.terrain");
    }

    #[test]
    fn deserialize_simple_table_default_tags() {
        let yaml = r#"
name: Minimal
type: simple
roll: 1d4
results:
  - min: 1
    max: 4
    text: Something
"#;
        let table: Table = serde_yaml::from_str(yaml).unwrap();
        match table {
            Table::Simple { tags, .. } => assert!(tags.is_empty()),
            _ => panic!("Expected Simple table"),
        }
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib models`
Expected: FAIL (structs not defined yet)

**Step 3: Implement the data models**

```rust
// ABOUTME: Data models for tables, manifests, and roll results.
// ABOUTME: Serde structs for YAML deserialization and RollResult output type.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct ResultEntry {
    pub min: u32,
    pub max: u32,
    pub text: Option<String>,
    pub chain: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Table {
    #[serde(rename = "simple")]
    Simple {
        name: String,
        #[serde(default)]
        tags: Vec<String>,
        roll: String,
        results: Vec<ResultEntry>,
    },
    #[serde(rename = "compound")]
    Compound {
        name: String,
        #[serde(default)]
        tags: Vec<String>,
        tables: Vec<String>,
    },
}

impl Table {
    pub fn name(&self) -> &str {
        match self {
            Table::Simple { name, .. } | Table::Compound { name, .. } => name,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            Table::Simple { tags, .. } | Table::Compound { tags, .. } => tags,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub namespace: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub author: Option<String>,
    pub min_tool_version: Option<String>,
    pub directories: Vec<DirectoryEntry>,
    #[serde(skip)]
    pub base_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RollResult {
    pub table_name: String,
    pub roll: Option<u32>,
    pub text: Option<String>,
    pub children: Vec<RollResult>,
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib models`
Expected: all 5 tests PASS

**Step 5: Commit**

```bash
git add src/models.rs
git commit -s -m "feat: data models for tables, manifests, and roll results

Serde-derived types for YAML deserialization of simple tables,
compound tables, manifests with directory entries, and the
RollResult tree output type.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 4: Per-Type Validation

**Files:**
- Modify: `src/validator.rs`
- Test: `src/validator.rs` (inline tests)

**Context for implementer:** Validation is split into two phases. This task covers per-type validation (checks that need only the type itself plus diceman for dice parsing). Cross-reference validation (chain/compound refs that need the full registry) is Task 7.

**Diceman API needed:**
- `diceman::parse(expr)` — verify dice expression is valid
- `diceman::simulate_seeded(expr, 100_000, 42)` — get min/max range of dice expression

**Namespace format:** `[a-z][a-z0-9_-]*` per dot-separated segment.

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_namespace_single_segment() {
        assert!(validate_namespace("test").is_ok());
    }

    #[test]
    fn valid_namespace_multi_segment() {
        assert!(validate_namespace("dmg.treasure.gems").is_ok());
    }

    #[test]
    fn invalid_namespace_starts_with_digit() {
        assert!(validate_namespace("2e-dmg").is_err());
    }

    #[test]
    fn invalid_namespace_uppercase() {
        assert!(validate_namespace("DMG").is_err());
    }

    #[test]
    fn invalid_namespace_empty_segment() {
        assert!(validate_namespace("dmg..treasure").is_err());
    }

    #[test]
    fn valid_result_entry() {
        let entry = ResultEntry { min: 1, max: 3, text: Some("test".into()), chain: None };
        assert!(validate_result_entry(&entry, "test-table").is_ok());
    }

    #[test]
    fn reversed_range_entry() {
        let entry = ResultEntry { min: 5, max: 2, text: Some("test".into()), chain: None };
        let err = validate_result_entry(&entry, "test-table").unwrap_err();
        assert!(matches!(err, ValidationError::RangeReversed { .. }));
    }

    #[test]
    fn valid_simple_table_full_coverage() {
        let table = Table::Simple {
            name: "Test".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 3, text: Some("Low".into()), chain: None },
                ResultEntry { min: 4, max: 6, text: Some("High".into()), chain: None },
            ],
        };
        assert!(validate_table(&table).is_ok());
    }

    #[test]
    fn simple_table_with_gap() {
        let table = Table::Simple {
            name: "Gappy".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 2, text: Some("Low".into()), chain: None },
                ResultEntry { min: 5, max: 6, text: Some("High".into()), chain: None },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, ValidationError::RangeGap { .. }));
    }

    #[test]
    fn simple_table_with_overlap() {
        let table = Table::Simple {
            name: "Overlapping".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 4, text: Some("Low".into()), chain: None },
                ResultEntry { min: 3, max: 6, text: Some("High".into()), chain: None },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, ValidationError::RangeOverlap { .. }));
    }

    #[test]
    fn simple_table_bad_dice_expression() {
        let table = Table::Simple {
            name: "BadDice".into(),
            tags: vec![],
            roll: "1z6".into(),
            results: vec![
                ResultEntry { min: 1, max: 6, text: Some("X".into()), chain: None },
            ],
        };
        let err = validate_table(&table).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidDiceExpression { .. }));
    }

    #[test]
    fn compound_table_validates_ok() {
        let table = Table::Compound {
            name: "Compound".into(),
            tags: vec![],
            tables: vec!["a".into(), "b".into()],
        };
        // Per-type validation for compound tables always passes
        // (reference resolution is cross-ref validation in Task 7)
        assert!(validate_table(&table).is_ok());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib validator`
Expected: FAIL (functions not defined)

**Step 3: Implement validation functions**

```rust
// ABOUTME: Per-type validation for tables, result entries, and namespaces.
// ABOUTME: Cross-reference validation (chain/compound refs) is separate; see loader.

use crate::error::ValidationError;
use crate::models::{ResultEntry, Table};
use regex::Regex;
use std::sync::LazyLock;

static NAMESPACE_SEGMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_-]*$").unwrap());

pub fn validate_namespace(namespace: &str) -> Result<(), ValidationError> {
    if namespace.is_empty() {
        return Err(ValidationError::InvalidNamespace {
            namespace: namespace.to_string(),
            reason: "namespace cannot be empty".into(),
        });
    }
    for segment in namespace.split('.') {
        if segment.is_empty() {
            return Err(ValidationError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: "empty segment (double dot)".into(),
            });
        }
        if !NAMESPACE_SEGMENT.is_match(segment) {
            return Err(ValidationError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: format!(
                    "segment '{segment}' must match [a-z][a-z0-9_-]*"
                ),
            });
        }
    }
    Ok(())
}

pub fn validate_result_entry(
    entry: &ResultEntry,
    table_name: &str,
) -> Result<(), ValidationError> {
    if entry.max < entry.min {
        return Err(ValidationError::RangeReversed {
            table: table_name.to_string(),
            min: entry.min,
            max: entry.max,
        });
    }
    Ok(())
}

/// Validates a table's internal consistency (ranges, dice expression).
/// Does NOT check cross-references (chains, compound refs).
pub fn validate_table(table: &Table) -> Result<(), ValidationError> {
    match table {
        Table::Simple { name, roll, results, .. } => {
            // Validate dice expression is parseable
            diceman::parse(roll).map_err(|e| ValidationError::InvalidDiceExpression {
                table: name.clone(),
                expr: roll.clone(),
                reason: e.to_string(),
            })?;

            // Validate each result entry
            for entry in results {
                validate_result_entry(entry, name)?;
            }

            // Get dice expression range via simulation
            let sim = diceman::simulate_seeded(roll, 100_000, 42)
                .map_err(|e| ValidationError::InvalidDiceExpression {
                    table: name.clone(),
                    expr: roll.clone(),
                    reason: e.to_string(),
                })?;
            let dice_min = sim.min as u32;
            let dice_max = sim.max as u32;

            // Check range coverage: every value in [dice_min, dice_max]
            // must be covered exactly once
            let mut coverage = vec![0u32; (dice_max - dice_min + 1) as usize];
            for entry in results {
                let start = entry.min.saturating_sub(dice_min) as usize;
                let end = entry.max.saturating_sub(dice_min) as usize;
                for i in start..=end.min(coverage.len() - 1) {
                    coverage[i] += 1;
                }
            }

            let missing: Vec<u32> = coverage.iter().enumerate()
                .filter(|(_, &count)| count == 0)
                .map(|(i, _)| i as u32 + dice_min)
                .collect();
            if !missing.is_empty() {
                return Err(ValidationError::RangeGap {
                    table: name.clone(),
                    missing,
                });
            }

            let overlapping: Vec<u32> = coverage.iter().enumerate()
                .filter(|(_, &count)| count > 1)
                .map(|(i, _)| i as u32 + dice_min)
                .collect();
            if !overlapping.is_empty() {
                return Err(ValidationError::RangeOverlap {
                    table: name.clone(),
                    overlapping,
                });
            }

            Ok(())
        }
        Table::Compound { .. } => {
            // Per-type validation for compound tables is minimal.
            // Reference resolution checked in cross-ref validation.
            Ok(())
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib validator`
Expected: all 11 tests PASS

**Step 5: Commit**

```bash
git add src/validator.rs
git commit -s -m "feat: per-type validation for tables, ranges, and namespaces

Validates dice expressions via diceman, checks range coverage
(gaps and overlaps), namespace format, and result entry ordering.
Uses simulate_seeded for deterministic range bounds.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 5: Registry

**Files:**
- Modify: `src/registry.rs`
- Test: `src/registry.rs` (inline tests)

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};

    fn simple_table(name: &str) -> Table {
        Table::Simple {
            name: name.to_string(),
            tags: vec!["test".to_string()],
            roll: "1d6".to_string(),
            results: vec![
                ResultEntry { min: 1, max: 6, text: Some("X".into()), chain: None },
            ],
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = Registry::new();
        reg.register("test.foo".into(), simple_table("Foo")).unwrap();
        assert!(reg.get("test.foo").is_some());
        assert!(reg.get("test.bar").is_none());
    }

    #[test]
    fn duplicate_registration_fails() {
        let mut reg = Registry::new();
        reg.register("test.foo".into(), simple_table("Foo")).unwrap();
        let err = reg.register("test.foo".into(), simple_table("Foo2"));
        assert!(err.is_err());
    }

    #[test]
    fn resolve_relative_first() {
        let mut reg = Registry::new();
        reg.register("ns.sub.target".into(), simple_table("Local")).unwrap();
        reg.register("target".into(), simple_table("Global")).unwrap();

        // Relative resolution: "target" in namespace "ns.sub" finds "ns.sub.target"
        let (fqid, table) = reg.resolve("target", "ns.sub").unwrap();
        assert_eq!(fqid, "ns.sub.target");
        assert_eq!(table.name(), "Local");
    }

    #[test]
    fn resolve_falls_back_to_fqid() {
        let mut reg = Registry::new();
        reg.register("other.target".into(), simple_table("Other")).unwrap();

        // No relative match, but "other.target" works as FQID
        let (fqid, table) = reg.resolve("other.target", "ns.sub").unwrap();
        assert_eq!(fqid, "other.target");
        assert_eq!(table.name(), "Other");
    }

    #[test]
    fn resolve_not_found() {
        let reg = Registry::new();
        assert!(reg.resolve("nonexistent", "ns").is_none());
    }

    #[test]
    fn all_tables_iterates_everything() {
        let mut reg = Registry::new();
        reg.register("a.one".into(), simple_table("One")).unwrap();
        reg.register("b.two".into(), simple_table("Two")).unwrap();
        assert_eq!(reg.all_tables().count(), 2);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib registry`
Expected: FAIL

**Step 3: Implement the registry**

```rust
// ABOUTME: In-memory table store keyed by fully qualified ID.
// ABOUTME: Supports relative-first reference resolution for chain/compound lookups.

use std::collections::HashMap;

use crate::error::ValidationError;
use crate::models::Table;

pub struct Registry {
    tables: HashMap<String, Table>,
}

impl Registry {
    pub fn new() -> Self {
        Self { tables: HashMap::new() }
    }

    pub fn register(&mut self, fqid: String, table: Table) -> Result<(), ValidationError> {
        if self.tables.contains_key(&fqid) {
            return Err(ValidationError::DuplicateId { id: fqid });
        }
        self.tables.insert(fqid, table);
        Ok(())
    }

    pub fn get(&self, fqid: &str) -> Option<&Table> {
        self.tables.get(fqid)
    }

    /// Resolve a reference using relative-first resolution:
    /// 1. Try current_namespace + "." + reference
    /// 2. Try reference as a fully qualified ID
    /// Returns (fqid, &Table) on success.
    pub fn resolve(&self, reference: &str, current_namespace: &str) -> Option<(&str, &Table)> {
        // Try relative resolution
        let relative_id = format!("{current_namespace}.{reference}");
        if let Some(table) = self.tables.get(&relative_id) {
            return Some((self.tables.get_key_value(&relative_id).unwrap().0, table));
        }

        // Fall back to fully qualified ID
        if let Some((key, table)) = self.tables.get_key_value(reference) {
            return Some((key, table));
        }

        None
    }

    pub fn all_tables(&self) -> impl Iterator<Item = (&str, &Table)> {
        self.tables.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib registry`
Expected: all 6 tests PASS

**Step 5: Commit**

```bash
git add src/registry.rs
git commit -s -m "feat: registry with relative-first reference resolution

In-memory HashMap store keyed by fully qualified table ID.
Resolve tries namespace-relative first, then falls back to
the reference as a fully qualified ID.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 6: Loader

**Files:**
- Modify: `src/loader.rs`
- Test: `src/loader.rs` (inline tests)

**Context for implementer:** The loader reads a manifest file, walks its declared directories, parses each `.yaml` file (skipping `manifest.yaml`), validates per-type, and registers tables in the registry. It collects ALL errors before returning, so users see the full picture in one shot.

The fully qualified ID for a table is `{directory_namespace}.{filename_stem}`. For example, `terrain/wilderness.yaml` under namespace `test.terrain` becomes `test.terrain.wilderness`.

**Step 1: Write failing tests**

```rust
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
    fn load_valid_collection() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let registry = load_collection(&manifest_path).unwrap();

        // Should have loaded all tables
        assert!(registry.get("test.terrain.wilderness").is_some());
        assert!(registry.get("test.npc.npc-occupation").is_some());
        assert!(registry.get("test.npc.npc-disposition").is_some());
        assert!(registry.get("test.npc.npc-quirk").is_some());
        assert!(registry.get("test.npc.quick-npc").is_some());
        assert!(registry.get("test.encounters.wilderness-encounter").is_some());
        assert!(registry.get("test.encounters.animal-type").is_some());
    }

    #[test]
    fn load_manifest_not_found() {
        let result = load_collection(&PathBuf::from("/nonexistent/manifest.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn loaded_table_has_correct_data() {
        let manifest_path = fixtures_path("valid-collection/manifest.yaml");
        let registry = load_collection(&manifest_path).unwrap();
        let table = registry.get("test.terrain.wilderness").unwrap();
        assert_eq!(table.name(), "Wilderness Terrain");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib loader`
Expected: FAIL

**Step 3: Implement the loader**

```rust
// ABOUTME: Loads table collections from the filesystem into a registry.
// ABOUTME: Reads manifests, discovers YAML files, parses, validates, and registers.

use std::fs;
use std::path::Path;

use crate::error::{Error, LoadError, ValidationError};
use crate::models::{Manifest, Table};
use crate::registry::Registry;
use crate::validator::{validate_namespace, validate_table};

/// Load a collection from a manifest file path.
/// Returns a populated Registry or collected errors.
pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    if !manifest_path.exists() {
        return Err(LoadError::ManifestNotFound {
            path: manifest_path.to_path_buf(),
        }.into());
    }

    let manifest_contents = fs::read_to_string(manifest_path)
        .map_err(|e| LoadError::FileRead {
            path: manifest_path.to_path_buf(),
            reason: e.to_string(),
        })?;

    let mut manifest: Manifest = serde_yaml::from_str(&manifest_contents)?;
    manifest.base_path = manifest_path.parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let mut registry = Registry::new();
    let mut errors: Vec<Error> = Vec::new();

    // Validate manifest namespace
    if let Err(e) = validate_namespace(&manifest.namespace) {
        errors.push(e.into());
    }

    for dir_entry in &manifest.directories {
        // Validate directory namespace
        if let Err(e) = validate_namespace(&dir_entry.namespace) {
            errors.push(e.into());
            continue;
        }

        let dir_path = manifest.base_path.join(&dir_entry.path);
        if !dir_path.is_dir() {
            errors.push(ValidationError::DirectoryNotFound {
                path: dir_path,
            }.into());
            continue;
        }

        // Discover and load YAML files
        let entries = match fs::read_dir(&dir_path) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(LoadError::FileRead {
                    path: dir_path,
                    reason: e.to_string(),
                }.into());
                continue;
            }
        };

        for entry in entries.flatten() {
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

            let fqid = format!("{}.{}", dir_entry.namespace, stem);

            let contents = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(LoadError::FileRead {
                        path: path.clone(),
                        reason: e.to_string(),
                    }.into());
                    continue;
                }
            };

            let table: Table = match serde_yaml::from_str(&contents) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(Error::Yaml(e));
                    continue;
                }
            };

            // Per-type validation
            if let Err(e) = validate_table(&table) {
                errors.push(e.into());
                continue;
            }

            if let Err(e) = registry.register(fqid, table) {
                errors.push(e.into());
            }
        }
    }

    if errors.is_empty() {
        Ok(registry)
    } else {
        Err(LoadError::Multiple { errors }.into())
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib loader`
Expected: all 3 tests PASS

**Step 5: Commit**

```bash
git add src/loader.rs
git commit -s -m "feat: collection loader with error accumulation

Reads manifest, walks declared directories, parses YAML files,
runs per-type validation, and registers tables. Collects all
errors before returning so users see the full picture.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 7: Cross-Reference Validation

**Files:**
- Modify: `src/validator.rs` (add cross-ref functions)
- Test: `src/validator.rs` (add cross-ref tests)

**Context for implementer:** This validates that all chain references and compound table references actually resolve in the registry. This runs AFTER the registry is fully populated (post-loading).

**Step 1: Write failing tests**

Add to the existing tests module in `src/validator.rs`:

```rust
    #[test]
    fn validate_refs_valid_collection() {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/valid-collection/manifest.yaml");
        let registry = crate::loader::load_collection(&manifest_path).unwrap();
        // The valid collection has chains to animal-type which exists
        // in the same namespace. Should pass.
        assert!(validate_references(&registry).is_ok());
    }

    #[test]
    fn validate_refs_catches_broken_chain() {
        let mut registry = Registry::new();
        registry.register("test.broken".into(), Table::Simple {
            name: "Broken".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry { min: 1, max: 2, text: Some("X".into()),
                    chain: Some(vec!["nonexistent".into()]) },
                ResultEntry { min: 3, max: 4, text: Some("Y".into()), chain: None },
            ],
        }).unwrap();

        let errors = validate_references(&registry).unwrap_err();
        assert!(!errors.is_empty());
        assert!(matches!(&errors[0], ValidationError::UnresolvedChain { .. }));
    }

    #[test]
    fn validate_refs_catches_broken_compound() {
        let mut registry = Registry::new();
        registry.register("test.comp".into(), Table::Compound {
            name: "Bad Compound".into(),
            tags: vec![],
            tables: vec!["nonexistent-a".into(), "nonexistent-b".into()],
        }).unwrap();

        let errors = validate_references(&registry).unwrap_err();
        assert_eq!(errors.len(), 2);
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib validator`
Expected: new tests FAIL (function not defined), old tests still PASS

**Step 3: Implement cross-reference validation**

Add to `src/validator.rs`:

```rust
use crate::registry::Registry;

/// Validate that all chain and compound table references resolve in the registry.
/// Returns collected errors (not just the first one).
pub fn validate_references(registry: &Registry) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    for (fqid, table) in registry.all_tables() {
        // Extract the namespace from the FQID (everything up to the last dot)
        let current_namespace = fqid.rsplit_once('.')
            .map(|(ns, _)| ns)
            .unwrap_or("");

        match table {
            Table::Simple { name, results, .. } => {
                for entry in results {
                    if let Some(chains) = &entry.chain {
                        for chain_ref in chains {
                            if registry.resolve(chain_ref, current_namespace).is_none() {
                                errors.push(ValidationError::UnresolvedChain {
                                    table: name.clone(),
                                    reference: chain_ref.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Table::Compound { name, tables, .. } => {
                for table_ref in tables {
                    if registry.resolve(table_ref, current_namespace).is_none() {
                        errors.push(ValidationError::UnresolvedCompoundRef {
                            table: name.clone(),
                            reference: table_ref.clone(),
                        });
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib validator`
Expected: all tests PASS (old + new)

**Step 5: Commit**

```bash
git add src/validator.rs
git commit -s -m "feat: cross-reference validation for chains and compound tables

Checks all chain and compound table references resolve in the
registry using relative-first resolution. Collects all errors
before returning.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 8: Roller

**Files:**
- Modify: `src/roller.rs`
- Test: `src/roller.rs` (inline tests)

**Context for implementer:** The roller executes a roll against a named table. For simple tables, it evaluates the dice expression via diceman, matches the result to a range entry, interpolates inline `{dice}` expressions in the result text, and recursively resolves chains. For compound tables, it rolls each sub-table in order.

**Diceman API:**
- `diceman::roll_with_rng(expr, &mut rng)` — returns `RollResult { total: i64, ... }`
- `diceman::FastRng::with_seed(seed)` — deterministic RNG for testing
- `diceman::Rng` trait — implement or use FastRng

**Chain depth limit:** 10

**Dice interpolation:** Scan result text for `{...}`, evaluate each as dice expression, replace with numeric result.

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};
    use crate::registry::Registry;

    fn build_test_registry() -> Registry {
        let mut reg = Registry::new();

        reg.register("test.simple".into(), Table::Simple {
            name: "Simple Test".into(),
            tags: vec![],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 3, text: Some("Low".into()), chain: None },
                ResultEntry { min: 4, max: 6, text: Some("High".into()), chain: None },
            ],
        }).unwrap();

        reg.register("test.chained".into(), Table::Simple {
            name: "Chained".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry { min: 1, max: 2, text: Some("Follow up".into()),
                    chain: Some(vec!["simple".into()]) },
                ResultEntry { min: 3, max: 4, text: Some("No chain".into()), chain: None },
            ],
        }).unwrap();

        reg.register("test.compound".into(), Table::Compound {
            name: "Compound Test".into(),
            tags: vec![],
            tables: vec!["simple".into()],
        }).unwrap();

        reg.register("test.interpolated".into(), Table::Simple {
            name: "Interpolated".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry { min: 1, max: 4,
                    text: Some("Found {2d6} gold coins".into()), chain: None },
            ],
        }).unwrap();

        reg
    }

    #[test]
    fn roll_simple_table() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.simple", &mut rng).unwrap();
        assert_eq!(result.table_name, "Simple Test");
        assert!(result.roll.is_some());
        assert!(result.text.is_some());
        assert!(result.children.is_empty());
    }

    #[test]
    fn roll_not_found() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "nonexistent", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::TableNotFound { .. }));
    }

    #[test]
    fn roll_compound_table() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.compound", &mut rng).unwrap();
        assert_eq!(result.table_name, "Compound Test");
        assert!(result.roll.is_none()); // compound parents have no roll
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].table_name, "Simple Test");
    }

    #[test]
    fn roll_with_chain() {
        let reg = build_test_registry();
        // Use a seed that produces a roll of 1-2 on 1d4 to trigger the chain
        // We may need to try different seeds; the test verifies the chain
        // mechanism works when triggered.
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.chained", &mut rng).unwrap();
        // Either has children (chain triggered) or doesn't (roll 3-4)
        if result.roll.unwrap() <= 2 {
            assert_eq!(result.children.len(), 1);
            assert_eq!(result.children[0].table_name, "Simple Test");
        } else {
            assert!(result.children.is_empty());
        }
    }

    #[test]
    fn roll_interpolates_dice_in_text() {
        let reg = build_test_registry();
        let mut rng = diceman::FastRng::with_seed(42);
        let result = roll_with_rng(&reg, "test.interpolated", &mut rng).unwrap();
        let text = result.text.as_ref().unwrap();
        // The {2d6} should be replaced with a number
        assert!(!text.contains('{'));
        assert!(text.starts_with("Found "));
        assert!(text.ends_with(" gold coins"));
    }

    #[test]
    fn chain_depth_limit() {
        // Create a circular chain: A chains to B, B chains to A
        let mut reg = Registry::new();
        reg.register("loop.a".into(), Table::Simple {
            name: "A".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry { min: 1, max: 4, text: Some("Loop".into()),
                    chain: Some(vec!["b".into()]) },
            ],
        }).unwrap();
        reg.register("loop.b".into(), Table::Simple {
            name: "B".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry { min: 1, max: 4, text: Some("Loop".into()),
                    chain: Some(vec!["a".into()]) },
            ],
        }).unwrap();

        let mut rng = diceman::FastRng::with_seed(42);
        let err = roll_with_rng(&reg, "loop.a", &mut rng).unwrap_err();
        assert!(matches!(err, RollError::ChainDepthExceeded { .. }));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib roller`
Expected: FAIL

**Step 3: Implement the roller**

```rust
// ABOUTME: Roll execution engine for simple and compound tables.
// ABOUTME: Handles dice evaluation, chain resolution, and result text interpolation.

use regex::Regex;
use std::sync::LazyLock;

use crate::error::RollError;
use crate::models::{RollResult, Table};
use crate::registry::Registry;

const MAX_CHAIN_DEPTH: usize = 10;

static DICE_INTERPOLATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([^}]+)\}").unwrap());

pub fn roll(registry: &Registry, table_id: &str) -> Result<RollResult, RollError> {
    roll_with_rng(registry, table_id, &mut diceman::FastRng::new())
}

pub fn roll_with_rng(
    registry: &Registry,
    table_id: &str,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(registry, table_id, "", rng, 0)
}

fn roll_recursive(
    registry: &Registry,
    table_id: &str,
    current_namespace: &str,
    rng: &mut impl diceman::Rng,
    depth: usize,
) -> Result<RollResult, RollError> {
    if depth > MAX_CHAIN_DEPTH {
        return Err(RollError::ChainDepthExceeded {
            table: table_id.to_string(),
            limit: MAX_CHAIN_DEPTH,
        });
    }

    // Resolve the table: try as FQID first, then resolve with namespace
    let (resolved_fqid, table) = if let Some(t) = registry.get(table_id) {
        (table_id, t)
    } else if !current_namespace.is_empty() {
        registry.resolve(table_id, current_namespace)
            .ok_or_else(|| RollError::TableNotFound { id: table_id.to_string() })?
    } else {
        return Err(RollError::TableNotFound { id: table_id.to_string() });
    };

    // Extract namespace from resolved FQID for child resolution
    let namespace = resolved_fqid.rsplit_once('.')
        .map(|(ns, _)| ns)
        .unwrap_or("");

    match table.clone() {
        Table::Simple { name, roll: roll_expr, results, .. } => {
            let dice_result = diceman::roll_with_rng(&roll_expr, rng)
                .map_err(|_| RollError::RollOutOfRange {
                    table: name.clone(),
                    value: 0,
                })?;

            let roll_value = dice_result.total;
            if roll_value < 0 {
                return Err(RollError::NegativeRoll { value: roll_value });
            }
            let roll_u32 = roll_value as u32;

            // Find matching range entry
            let entry = results.iter()
                .find(|e| roll_u32 >= e.min && roll_u32 <= e.max)
                .ok_or_else(|| RollError::RollOutOfRange {
                    table: name.clone(),
                    value: roll_value,
                })?;

            // Interpolate dice expressions in result text
            let text = entry.text.as_ref().map(|t| interpolate_dice(t, rng));

            // Resolve chains
            let mut children = Vec::new();
            if let Some(chains) = &entry.chain {
                for chain_ref in chains {
                    let child = roll_recursive(
                        registry, chain_ref, namespace, rng, depth + 1,
                    )?;
                    children.push(child);
                }
            }

            Ok(RollResult {
                table_name: name,
                roll: Some(roll_u32),
                text,
                children,
            })
        }
        Table::Compound { name, tables: sub_tables, .. } => {
            let mut children = Vec::new();
            for table_ref in &sub_tables {
                let child = roll_recursive(
                    registry, table_ref, namespace, rng, depth + 1,
                )?;
                children.push(child);
            }

            Ok(RollResult {
                table_name: name,
                roll: None,
                text: None,
                children,
            })
        }
    }
}

fn interpolate_dice(text: &str, rng: &mut impl diceman::Rng) -> String {
    DICE_INTERPOLATION.replace_all(text, |caps: &regex::Captures| {
        let expr = &caps[1];
        match diceman::roll_with_rng(expr, rng) {
            Ok(result) => result.total.to_string(),
            Err(_) => caps[0].to_string(), // Leave unresolved on error
        }
    }).to_string()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib roller`
Expected: all 6 tests PASS

**Step 5: Commit**

```bash
git add src/roller.rs
git commit -s -m "feat: roll execution with chains, compounds, and dice interpolation

Evaluates dice via diceman, matches results to range entries,
recursively resolves chain references (depth limit 10), rolls
compound sub-tables in order, and interpolates {dice} in text.

Co-authored-by: Claude <noreply@anthropic.com>"
```

**Design note for implementer:** The `table.clone()` in the match is needed because we borrow from the registry and also need to pass the registry to recursive calls. If this becomes a performance concern, consider storing table data in an `Arc` or restructuring to avoid the clone. For v0.1, the clone is fine — tables are small.

---

## Task 9: Search

**Files:**
- Modify: `src/search.rs`
- Test: `src/search.rs` (inline tests)

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ResultEntry, Table};
    use crate::registry::Registry;

    fn build_search_registry() -> Registry {
        let mut reg = Registry::new();
        reg.register("dmg.treasure.gems".into(), Table::Simple {
            name: "Gem Type".into(),
            tags: vec!["treasure".into(), "gems".into()],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 6, text: Some("Ruby".into()), chain: None },
            ],
        }).unwrap();
        reg.register("dmg.encounters.wilderness".into(), Table::Simple {
            name: "Wilderness Encounter".into(),
            tags: vec!["encounter".into(), "wilderness".into()],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 6, text: Some("Wolves".into()), chain: None },
            ],
        }).unwrap();
        reg.register("core.npc.occupation".into(), Table::Simple {
            name: "NPC Occupation".into(),
            tags: vec!["npc".into()],
            roll: "1d6".into(),
            results: vec![
                ResultEntry { min: 1, max: 6, text: Some("Smith".into()), chain: None },
            ],
        }).unwrap();
        reg
    }

    #[test]
    fn search_by_name_substring() {
        let reg = build_search_registry();
        let results = search_by_name(&reg, "wilderness");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "dmg.encounters.wilderness");
    }

    #[test]
    fn search_by_name_case_insensitive() {
        let reg = build_search_registry();
        let results = search_by_name(&reg, "GEM");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_by_tag() {
        let reg = build_search_registry();
        let results = search_by_tag(&reg, "npc");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "core.npc.occupation");
    }

    #[test]
    fn search_by_namespace() {
        let reg = build_search_registry();
        let results = search_by_namespace(&reg, "dmg");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_no_results() {
        let reg = build_search_registry();
        assert!(search_by_name(&reg, "nonexistent").is_empty());
        assert!(search_by_tag(&reg, "nonexistent").is_empty());
        assert!(search_by_namespace(&reg, "nonexistent").is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib search`
Expected: FAIL

**Step 3: Implement search functions**

```rust
// ABOUTME: Search functions for finding tables by name, tag, or namespace.
// ABOUTME: All searches are case-insensitive for names, exact-match for tags.

use crate::models::Table;
use crate::registry::Registry;

/// Search by table name (case-insensitive substring match).
pub fn search_by_name<'a>(registry: &'a Registry, query: &str) -> Vec<(&'a str, &'a Table)> {
    let query_lower = query.to_lowercase();
    registry.all_tables()
        .filter(|(_, table)| table.name().to_lowercase().contains(&query_lower))
        .collect()
}

/// Search by tag (exact match).
pub fn search_by_tag<'a>(registry: &'a Registry, tag: &str) -> Vec<(&'a str, &'a Table)> {
    registry.all_tables()
        .filter(|(_, table)| table.tags().iter().any(|t| t == tag))
        .collect()
}

/// Search by namespace prefix (FQID starts with the given namespace).
pub fn search_by_namespace<'a>(registry: &'a Registry, namespace: &str) -> Vec<(&'a str, &'a Table)> {
    let prefix = if namespace.ends_with('.') {
        namespace.to_string()
    } else {
        format!("{namespace}.")
    };
    registry.all_tables()
        .filter(|(fqid, _)| fqid.starts_with(&prefix))
        .collect()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib search`
Expected: all 5 tests PASS

**Step 5: Commit**

```bash
git add src/search.rs
git commit -s -m "feat: search tables by name, tag, and namespace

Case-insensitive substring match for names, exact match for
tags, prefix match for namespace queries.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Task 10: CLI

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs` (add public API convenience functions)
- Test: integration test `tests/cli_integration.rs`

**Context for implementer:** The CLI is a thin wrapper over the library. Four subcommands: `validate`, `roll`, `search`, `import`. Import reuses the loader; validate reuses loader + cross-ref validation; roll and search operate on a loaded registry.

**Step 1: Write a failing integration test**

Create `tests/cli_integration.rs`:

```rust
// ABOUTME: Integration tests for the fatescroll CLI.
// ABOUTME: Tests subcommands against fixture collections.

use std::path::PathBuf;
use std::process::Command;

fn fatescroll_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fatescroll"))
}

fn fixtures_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn validate_valid_collection() {
    let output = fatescroll_bin()
        .args(["validate", &fixtures_path("valid-collection").to_string_lossy()])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn validate_invalid_collection_fails() {
    let output = fatescroll_bin()
        .args(["validate", &fixtures_path("invalid-collection").to_string_lossy()])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}

#[test]
fn roll_on_table() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection", &fixtures_path("valid-collection").to_string_lossy(),
            "test.terrain.wilderness",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wilderness Terrain"));
}

#[test]
fn search_by_tag() {
    let output = fatescroll_bin()
        .args([
            "search",
            "--collection", &fixtures_path("valid-collection").to_string_lossy(),
            "--tag", "npc",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("NPC Occupation"));
}

#[test]
fn roll_nonexistent_table() {
    let output = fatescroll_bin()
        .args([
            "roll",
            "--collection", &fixtures_path("valid-collection").to_string_lossy(),
            "nonexistent.table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_integration`
Expected: FAIL (binary doesn't implement subcommands yet)

**Step 3: Add public API convenience functions to lib.rs**

Update `src/lib.rs`:

```rust
// ABOUTME: Public API for the fatescroll random table library.
// ABOUTME: Re-exports core types and provides top-level convenience functions.

pub mod error;
pub mod loader;
pub mod models;
pub mod registry;
pub mod roller;
pub mod search;
pub mod validator;

use std::path::Path;

pub use error::Error;
pub use models::{RollResult, Table};
pub use registry::Registry;

/// Load and validate a collection from a directory containing manifest.yaml.
pub fn load_collection(collection_dir: &Path) -> Result<Registry, Error> {
    let manifest_path = collection_dir.join("manifest.yaml");
    let registry = loader::load_collection(&manifest_path)?;

    // Run cross-reference validation
    if let Err(errors) = validator::validate_references(&registry) {
        return Err(error::LoadError::Multiple {
            errors: errors.into_iter().map(Error::from).collect(),
        }.into());
    }

    Ok(registry)
}
```

**Step 4: Implement the CLI**

```rust
// ABOUTME: CLI binary for fatescroll random table tool.
// ABOUTME: Thin wrapper over the fatescroll library using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "fatescroll", version, about = "RPG random table manager and roller")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a table collection
    Validate {
        /// Path to collection directory (containing manifest.yaml)
        collection: PathBuf,
    },
    /// Roll on a table
    Roll {
        /// Path to collection directory
        #[arg(long)]
        collection: PathBuf,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
    },
    /// Search for tables
    Search {
        /// Path to collection directory
        #[arg(long)]
        collection: PathBuf,
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
        collection: PathBuf,
        /// Directory within the collection to import into
        #[arg(long)]
        target_dir: String,
        /// Files to import
        files: Vec<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Validate { collection } => cmd_validate(&collection),
        Commands::Roll { collection, table_id } => cmd_roll(&collection, &table_id),
        Commands::Search { collection, name, tag, namespace } => {
            cmd_search(&collection, name.as_deref(), tag.as_deref(), namespace.as_deref())
        }
        Commands::Import { collection, target_dir, files } => {
            cmd_import(&collection, &target_dir, &files)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn cmd_validate(collection: &PathBuf) -> Result<(), fatescroll::Error> {
    let _registry = fatescroll::load_collection(collection)?;
    println!("Collection is valid.");
    Ok(())
}

fn cmd_roll(collection: &PathBuf, table_id: &str) -> Result<(), fatescroll::Error> {
    let registry = fatescroll::load_collection(collection)?;
    let result = fatescroll::roller::roll(&registry, table_id)?;
    print_roll_result(&result, 0);
    Ok(())
}

fn print_roll_result(result: &fatescroll::RollResult, indent: usize) {
    let pad = "  ".repeat(indent);
    match (result.roll, &result.text) {
        (Some(roll), Some(text)) => {
            println!("{pad}{} (rolled {}): {}", result.table_name, roll, text);
        }
        (Some(roll), None) => {
            println!("{pad}{} (rolled {})", result.table_name, roll);
        }
        (None, Some(text)) => {
            println!("{pad}{}: {}", result.table_name, text);
        }
        (None, None) => {
            println!("{pad}{}", result.table_name);
        }
    }
    for child in &result.children {
        print_roll_result(child, indent + 1);
    }
}

fn cmd_search(
    collection: &PathBuf,
    name: Option<&str>,
    tag: Option<&str>,
    namespace: Option<&str>,
) -> Result<(), fatescroll::Error> {
    let registry = fatescroll::load_collection(collection)?;

    let results: Vec<(&str, &fatescroll::Table)> = if let Some(name) = name {
        fatescroll::search::search_by_name(&registry, name)
    } else if let Some(tag) = tag {
        fatescroll::search::search_by_tag(&registry, tag)
    } else if let Some(ns) = namespace {
        fatescroll::search::search_by_namespace(&registry, ns)
    } else {
        eprintln!("Specify --name, --tag, or --namespace");
        process::exit(1);
    };

    if results.is_empty() {
        println!("No tables found.");
    } else {
        for (fqid, table) in &results {
            let tags = table.tags();
            if tags.is_empty() {
                println!("  {fqid} — {}", table.name());
            } else {
                println!("  {fqid} — {} [{}]", table.name(), tags.join(", "));
            }
        }
    }
    Ok(())
}

fn cmd_import(
    collection: &PathBuf,
    target_dir: &str,
    files: &[PathBuf],
) -> Result<(), fatescroll::Error> {
    let dest = collection.join(target_dir);
    if !dest.is_dir() {
        std::fs::create_dir_all(&dest)?;
    }

    for file in files {
        let filename = file.file_name()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("no filename in path: {}", file.display()),
            ))?;
        std::fs::copy(file, dest.join(filename))?;
        println!("Imported: {}", filename.to_string_lossy());
    }

    // Validate the collection after import
    println!("Validating collection...");
    let _registry = fatescroll::load_collection(collection)?;
    println!("Collection is valid after import.");
    Ok(())
}
```

**Step 5: Run integration tests**

Run: `cargo test --test cli_integration`
Expected: all 5 tests PASS

**Step 6: Run all tests**

Run: `cargo test`
Expected: all tests PASS

**Step 7: Commit**

```bash
git add src/main.rs src/lib.rs tests/cli_integration.rs
git commit -s -m "feat: CLI with validate, roll, search, and import commands

Thin clap wrapper over library functions. Validate loads and
checks a collection, roll executes against a named table with
tree-formatted output, search finds by name/tag/namespace,
import copies files and re-validates.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Post-Implementation Checklist

After all tasks are complete:

1. **Run full test suite:** `cargo test`
2. **Run clippy:** `cargo clippy -- -D warnings`
3. **Format:** `cargo fmt`
4. **Manual smoke test:**
   ```bash
   cargo run -- validate tests/fixtures/valid-collection
   cargo run -- roll --collection tests/fixtures/valid-collection test.terrain.wilderness
   cargo run -- roll --collection tests/fixtures/valid-collection test.npc.quick-npc
   cargo run -- search --collection tests/fixtures/valid-collection --tag npc
   ```
5. **Review and merge feature branch to main**
