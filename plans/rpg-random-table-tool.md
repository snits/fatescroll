# RPG Random Table Tool — Design Document

## Overview

A Rust library with CLI wrapper for managing and rolling on RPG random tables
stored as YAML files. Tables are organized in directory-based collections with
manifests for namespace management. The tool handles import, validation, manifest
management, roll execution, and search.

**Name**: `fatescroll`

## Table Schema

### Structural Types

Two types only — **simple** and **compound**.

#### Simple Table

A single roll against a single result set. Results use range objects to support
non-uniform probability distributions (e.g., "1-3: forest, 4-5: plains, 6: hills").

```yaml
name: Wilderness Terrain
type: simple
tags:
  - terrain
  - wilderness
  - OSR
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

#### Simple Table with Chains

Any result can optionally chain to one or more follow-up tables. The `chain`
field is always a list, even for single references, so consumers never deal with
scalar vs list ambiguity.

A result with both `text` and `chain` produces a mixed result — the text is kept
and the chain rolls are appended. This handles cases like "Orc warband (roll on
Orc Warband Composition table)".

Inline dice expressions use `{NdM}` syntax embedded in the text field. At roll
time, the roller scans result text for `{...}` delimiters, evaluates each as a
dice expression via diceman, and replaces it with the numeric result (template
string interpolation). The resolved text is returned in the RollResult.

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

#### Compound Table

An ordered sequence of simple table references, rolled together. Used for
generators like "Quick NPC" that combine occupation + disposition + quirk.

Output labels each result with the child table's name for context.

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

### Design Decisions

- **No `id` field in table files.** The table's identity is derived from its
  filename (minus extension). This eliminates the consistency bug where a
  declared `id` disagrees with the filename. The fully qualified ID is
  constructed as `{namespace}.{filename-stem}`.

- **`chain` is always a list.** Simplifies consumer code — no type checking
  needed.

- **Range objects over array indexing.** `{min, max, text}` is honest about
  non-uniform probability and makes the data model self-documenting.

- **Result coverage validation.** The validator checks that result ranges for a
  simple table fully cover the roll expression's range without gaps or overlaps.
  A `1d6` table missing a result for roll value 4 is caught at load time, not at
  roll time.

### Reference Resolution

Chain references and compound table references use **relative-first resolution**:

1. Resolve relative to the current table's namespace
2. If no match, resolve as a fully qualified ID

This follows the same precedent as Python imports — local scope wins. If a table
`dmg.treasure.gems` chains to `quality`, the tool first looks for
`dmg.treasure.quality`, then falls back to `quality` as a global ID.

To explicitly reference a table in a different namespace, use the fully qualified
ID: `core.terrain.rocky-detail`.

## File Organization

### One Table Per File

Each table is a single YAML file. Benefits:
- Clean git diffs when editing individual tables
- Tables are naturally shareable as single artifacts
- Maps to how people actually distribute tables

Filename (without `.yaml` extension) becomes the table's local ID within its
namespace.

### Directory-Based Namespacing

Directory structure provides namespace hierarchy. A table at
`treasure/gems.yaml` under namespace `dmg` has fully qualified ID
`dmg.treasure.gems`.

### Namespace Identifiers

Namespace segments allow lowercase letters, digits, hyphens, and underscores.
Must start with a letter. This is looser than Python identifiers to accommodate
natural naming like `dmg.random-encounters` and `2e-dmg` (though segments cannot
start with a digit).

Pattern: `[a-z][a-z0-9_-]*` per segment, dot-separated.

### Collection Manifest

Each collection has a `manifest.yaml` at its root:

```yaml
name: Dungeon Master's Guide Tables
version: "1.0"
namespace: dmg
author: ~
min_tool_version: ~
directories:
  - path: treasure
    namespace: dmg.treasure
  - path: encounters
    namespace: dmg.encounters
  - path: npc
    namespace: dmg.npc
```

- `namespace`: root namespace for the collection
- `min_tool_version`: reserved for future schema evolution
- `directories`: maps relative paths to namespaces
- All paths are relative to the manifest file's location (portable collections)
- Manifest validator checks that declared directories exist on disk
- No alias support initially (deferred to follow-on work)

## Tool Scope

### In Scope (v0.1)

| Command    | Description                                              |
|------------|----------------------------------------------------------|
| `validate` | Check referential integrity, namespace consistency, manifest accuracy, result coverage |
| `import`   | Copy table files to target location, update manifest, validate via load |
| `roll`     | Execute a roll against a named table, resolve chains, format output |
| `search`   | Find tables by name, tag, or namespace                   |

### Explicitly Deferred

- **Format conversion** (CSV, JSON → YAML) — revisit later
- **Deck-style tables** (roll without replacement, stateful) — separate tool/follow-on
- **Manifest aliases** (third-party collections under different namespace prefix)
- **Table creation UI** — YAML in a text editor is already a good authoring experience
- **Roll modifiers from external variables** (e.g., "add your Charisma modifier")

## Architecture

### Language & Stack

- **Rust** — native performance, single-binary distribution, shared toolchain
  with hexwalker (Tauri) and diceman
- **serde + serde_yaml** — YAML deserialization with type-safe validation at
  parse time
- **Custom validation** — post-deserialization checks using Rust's type system
  ("parse, don't validate" idiom). Validated types enforce invariants at the
  type level once constructed.
- **clap** — CLI framework
- **diceman** — Rust crate dependency for dice evaluation (no FFI, direct crate
  import)
- **Python bindings (optional, deferred)** — PyO3 bindings for use as a Python
  package. Not required for the hexwalker interface; provided as a convenience
  for table authors and third-party projects.

### Crate Structure

```
fatescroll/
    src/
        lib.rs          # public API
        models.rs       # serde models: ResultEntry, SimpleTable, CompoundTable, Manifest
        registry.rs     # in-memory table store, lookup by fully qualified ID
        loader.rs       # file discovery, YAML parsing, registry building
        validator.rs    # referential integrity, namespace consistency, coverage checks
        roller.rs       # roll execution, chain resolution, result formatting
        search.rs       # tag, name, namespace queries against registry
    src/bin/
        main.rs         # thin CLI wrapper (clap) over the library
```

### Data Models

```rust
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct ResultEntry {
    pub min: u32,
    pub max: u32,
    pub text: Option<String>,
    pub chain: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub namespace: String,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub author: Option<String>,
    pub min_tool_version: Option<String>,
    pub directories: Vec<DirectoryEntry>,
    #[serde(skip)]
    pub base_path: PathBuf, // injected at load time, not from YAML
}
```

Post-deserialization validation (max >= min, range coverage, directory existence)
is performed by a `validate()` method on each type, returning `Result<ValidatedT, ValidationError>`.
This follows the "parse, don't validate" pattern — raw deserialized structs are
converted into validated wrapper types that guarantee invariants hold.

### Registry

Central in-memory store. All other components either populate it (loader) or
query it (roller, search).

```rust
use std::collections::HashMap;

pub struct Registry {
    tables: HashMap<String, Table>, // keyed by fully qualified ID
}

impl Registry {
    pub fn get(&self, fqid: &str) -> Option<&Table>;
    pub fn register(&mut self, fqid: String, table: Table) -> Result<(), DuplicateIdError>;
    pub fn resolve(&self, reference: &str, current_namespace: &str) -> Option<&Table>;
    // 1. Try current_namespace + "." + reference
    // 2. Try reference as fully qualified ID
    // 3. Return None (caller reports as validation error)
}
```

### Loader

Two-phase operation:
1. Walk manifest, discover files, parse YAML, build registry
2. Validate references (needs full registry before checking)

Reference validation collects all errors before raising — user gets the full
picture in one shot rather than fix-one-discover-another.

Duplicate fully qualified ID detection during registration.

### Import

Import is a convenience workflow, not a separate engine:
1. Copy source table files to the target directory within a collection
2. Add or update the directory entry in the collection's `manifest.yaml`
3. Attempt a full load of the manifest (which triggers all validation)
4. Report any errors (broken references, range gaps, namespace conflicts, etc.)

Validation happens through the normal load path — import doesn't need its own
validation logic.

### Roller

The roller executes a roll against a named table and recursively resolves chains:

1. Look up the table in the registry
2. For simple tables: evaluate the dice expression via diceman, match result
   to the appropriate range entry
3. Resolve inline dice expressions in result text via template interpolation —
   scan for `\{([^}]+)\}`, evaluate each match via diceman, replace with the
   numeric result
4. If the matched entry has chains, recursively roll on each chained table
5. For compound tables: roll on each sub-table in order, collecting results
6. Chain depth limit: **10**. If exceeded, return an error rather than
   continuing. Prevents infinite loops from circular chain references.

Returns a `RollResult` tree (see below).

### RollResult

```rust
pub struct RollResult {
    pub table_name: String,
    pub roll: Option<u32>,          // the die result; None for compound table parents
    pub text: Option<String>,       // resolved text (dice expressions interpolated)
    pub children: Vec<RollResult>,  // results from chains or compound sub-tables
}
```

Consumers (hexwalker, CLI) decide how to format or display the tree. The CLI
combines information from the RollResult and the registry to produce formatted
output (e.g., "Wilderness Encounter (1d8 → 7): Bandit camp"). Library consumers
can look up the dice expression from the registry if needed — it's not
duplicated in the result.

### Validation Checklist

Run at load time via `validate()` methods on deserialized types:
- [ ] Manifest directories exist on disk
- [ ] Namespace identifiers are well-formed
- [ ] No duplicate fully qualified IDs
- [ ] All chain references resolve (relative-first, then absolute)
- [ ] All compound table references resolve
- [ ] Result ranges cover the full roll expression range (no gaps, no overlaps)
- [ ] Dice expressions in `roll` field are parseable (validated via diceman)

## Open Questions

- **Roll-twice mechanics**: Some tables say "roll twice, keep both" or "roll
  twice, take higher." Could be a table-level property like
  `roll_mode: normal | advantage | keep_all`. Not critical for v0.1 but worth
  designing the extension point.

## Related Projects

- **diceman** — Rust dice notation crate, direct crate dependency
- **hexwalker** — Tauri-based solo RPG hex-crawl companion, primary consumer
  of fatescroll as a Rust crate (see `plans/hexwalker-design-notes.md`)
- **iOS card deck manager app** — separate tool for stateful deck-style
  mechanics (see `plans/swift-card-deck-manager-app.md`)
