# Authoring Guide

This guide covers how to create table collections for fatescroll, a structured randomness engine. If you are looking for CLI usage instructions, see the project README. This document is for people who want to author their own tables.

## Collections

A **collection** is a directory that contains a `manifest.yaml` file and references to one or more directories of table files. The manifest describes the collection metadata and maps each directory to a namespace.

### manifest.yaml Schema

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Display name for the collection. |
| `version` | string | yes | Version of the collection (e.g., `"1.0"`). |
| `namespace` | string | yes | Root namespace for the collection. Must follow namespace rules (see below). |
| `author` | string | no | Author name. Use `~` (YAML null) to omit. |
| `min_tool_version` | string | no | Minimum fatescroll version required. Use `~` to omit. |
| `directories` | list | no | List of directory entries. Each entry maps a directory to a namespace. Defaults to empty if omitted. |
| `files` | list | no | List of individual file entries. Defaults to empty if omitted. |

Each entry in `directories` has:

| Field | Type | Description |
|---|---|---|
| `path` | string | Path to the directory containing table YAML files. Can be relative to the manifest location (e.g., `terrain`, `../shared/npc`). |
| `namespace` | string | Dot-separated namespace assigned to tables in this directory. |

### File Entries

Each entry in `files` has:

| Field | Type | Description |
|---|---|---|
| `path` | string | Path to a single table YAML file. Can be relative to the manifest location (e.g., `shared/wilderness.yaml`, `../common/npc-occupation.yaml`). |
| `namespace` | string | Dot-separated namespace assigned to this table. |

File entries let you include individual table files without adding their entire directory. This is useful for sharing specific tables across collections or including files from outside the collection's directory tree.

The table's `id` field must match the filename stem (e.g., a file named `wilderness.yaml` must contain `id: wilderness`).

A manifest can use `directories`, `files`, or both. A files-only manifest (no `directories`) is valid.

### Example manifest.yaml

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
files:
  - path: ../shared/wilderness.yaml
    namespace: test.terrain
```

## Namespaces and FQIDs

Namespaces organize tables into a dot-separated hierarchy, similar to package names in many programming languages.

### Namespace Format

Each segment of a namespace (the parts between dots) must:

- Start with a lowercase letter (`a`-`z`)
- Contain only lowercase letters, digits, underscores, or hyphens (`[a-z][a-z0-9_-]*`)
- Not be empty (no double dots like `test..encounters`)

Valid examples: `test`, `dmg.treasure.gems`, `my-homebrew.encounters`

Invalid examples: `DMG` (uppercase), `2e-dmg` (starts with digit), `test..encounters` (empty segment)

### Fully Qualified IDs (FQIDs)

Every table loaded into fatescroll gets a **Fully Qualified ID (FQID)**. The FQID is constructed automatically from the directory's namespace and the YAML filename (without extension):

```
FQID = <directory namespace> + "." + <filename stem>
```

For example, a file named `wilderness-encounter.yaml` in a directory mapped to namespace `test.encounters` produces the FQID:

```
test.encounters.wilderness-encounter
```

You use FQIDs when rolling on tables from the command line:

```bash
fatescroll roll --collection ./my-collection test.encounters.wilderness-encounter
```

## Simple Tables

A **simple table** has a dice expression and a list of results. When rolled, fatescroll evaluates the dice expression and looks up the matching result by its range.

### Simple Table Schema

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Display name for the table. |
| `type` | string | yes | Must be `simple`. |
| `tags` | list of strings | no | Tags for searching. Defaults to an empty list if omitted. |
| `roll` | string | yes | Dice expression (e.g., `1d6`, `2d8+1`). Any valid diceman expression works. |
| `results` | list of result entries | yes | The possible outcomes. Each entry maps a range of roll values to a result. |

Each entry in `results` has:

| Field | Type | Required | Description |
|---|---|---|---|
| `min` | integer | yes | Minimum roll value (inclusive) that selects this result. |
| `max` | integer | yes | Maximum roll value (inclusive) that selects this result. |
| `text` | string | no | The result text displayed when this entry is selected. |
| `chain` | list of strings or chain references | no | Table references to roll on after this result (see Chaining). Each entry can be a plain string (table ID) or a structured reference with modifiers (see Reroll Modifiers). |

The result ranges must cover every possible value of the dice expression with no gaps and no overlaps. For example, `1d6` produces values 1 through 6, so your results must cover exactly that range.

### Example: wilderness.yaml

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

This table uses `1d6` (values 1-6). The results cover the full range: 1-3, 4-5, and 6.

## Compound Tables

A **compound table** groups multiple tables together. When rolled, it does not produce its own dice roll or text. Instead, it rolls each of its referenced tables and collects their results.

### Compound Table Schema

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Display name for the table. |
| `type` | string | yes | Must be `compound`. |
| `tags` | list of strings | no | Tags for searching. Defaults to an empty list if omitted. |
| `tables` | list of strings | yes | References to other tables. Each one is rolled when this compound table is rolled. |

References in `tables` follow the same resolution rules as chain references (see Reference Resolution below).

### Example: quick-npc.yaml

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

Rolling this compound table produces output like:

```
Quick NPC Generator
  NPC Occupation (rolled 3): Scholar
  NPC Disposition (rolled 2): Friendly
  NPC Quirk (rolled 4): Speaks in rhymes
```

The compound table itself has no roll value or text -- only its children produce results.

## Chaining

A result entry can include a `chain` field listing references to other tables. When that result is rolled, each chain reference triggers an additional roll on the referenced table. The chained results appear as children of the original result.

### Example: wilderness-encounter.yaml

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

If you roll a 4 (Bandit camp), fatescroll also rolls `bandit-strength` and `bandit-motivation`, producing nested output:

```
Wilderness Encounter (rolled 4): Bandit camp
  Bandit Strength (rolled 3): Medium group (4-6 bandits)
  Bandit Motivation (rolled 1): Desperate and hungry
```

Results without a `chain` field (like "Abandoned campsite") produce no children.

### Reroll Modifiers

Chain references can include a **reroll modifier** that causes fatescroll to reroll certain dice values when following that chain. This is useful for table entries that chain back to the same table but need to exclude certain results on subsequent rolls.

#### Structured Chain Reference Schema

Instead of a plain string, a chain entry can be an object:

| Field | Type | Required | Description |
|---|---|---|---|
| `table` | string | yes | Table reference (same resolution rules as plain string references). |
| `reroll` | list of integers | no | Dice values to reroll. If the chained roll produces one of these values, fatescroll rerolls until a non-excluded value appears. |

Plain string references and structured references can be mixed in the same `chain` list.

#### Example: Wizard Mishap Table

Some RPG systems have tables where rolling a specific value means "roll again on this table, but reroll that value." For example, Shadowdark's wizard mishap table says rolling a 1 means "roll twice on this table and combine the results, rerolling further 1s."

```yaml
name: Wizard Mishap
type: simple
tags:
  - wizard
  - mishap
roll: 1d12
results:
  - min: 1
    max: 1
    text: "Roll twice and combine"
    chain:
      - table: wizard-mishap
        reroll: [1]
      - table: wizard-mishap
        reroll: [1]
  - min: 2
    max: 2
    text: "Hands glow blue for {1d6} minutes"
  - min: 3
    max: 12
    text: "Other mishap effects..."
```

When a player rolls a 1 on this table, fatescroll chains to the same table twice. Each chained roll skips the value 1 -- if the dice land on 1, fatescroll rerolls automatically. This prevents the recursive explosion that would happen if the chained rolls also triggered the "roll twice" entry.

The reroll constraint only applies to the immediate chained roll. It does not propagate through further chains. Each chain reference independently specifies its own reroll values.

#### Mixing Plain and Structured References

You can use plain strings and structured references in the same chain list:

```yaml
chain:
  - animal-type
  - table: encounter-table
    reroll: [1, 2]
```

Here, `animal-type` is rolled normally, while `encounter-table` rerolls if the dice produce a 1 or 2.

#### Safety Limit

fatescroll limits reroll attempts to 100. If the reroll values cover all possible outcomes of the target table's dice expression, rolling stops with an error:

```
reroll attempts exhausted (100) for table 'Wizard Mishap' with reroll values [1, 2, 3, 4]
```

To avoid this, make sure your reroll values leave at least one reachable result in the target table.

### Reference Resolution

Chain references and compound table references resolve using a **relative-first** strategy:

1. **Relative lookup**: The reference is prefixed with the current table's namespace. For example, if the current table's FQID is `test.encounters.wilderness-encounter`, its namespace is `test.encounters`. A chain reference to `animal-type` first tries `test.encounters.animal-type`.
2. **FQID lookup**: If no relative match is found, the reference is tried as a fully qualified ID. For example, `other-collection.special-table` would be looked up directly.

This means you can use short names (like `animal-type`) when referencing tables in the same namespace, and full FQIDs when referencing tables in other namespaces.

### Sharing Tables Across Collections

Multiple manifests can reference the same table directories. Directory paths don't need to be subdirectories of the manifest — they can reach outside the manifest's directory tree using relative paths like `../shared/npc`.

For example, suppose you have a shared set of NPC tables and two campaigns that use them:

```
collections/
  shared/
    npc/
      npc-occupation.yaml
      npc-disposition.yaml
      npc-quirk.yaml
      quick-npc.yaml
  campaign-dark-forest/
    manifest.yaml
    encounters/
      forest-encounter.yaml
  campaign-seaside/
    manifest.yaml
    harbors/
      harbor-events.yaml
```

Both campaign manifests can reference the shared NPC tables:

```yaml
# campaign-dark-forest/manifest.yaml
name: Dark Forest Campaign
version: "1.0"
namespace: darkforest
directories:
  - path: ../shared/npc
    namespace: darkforest.npc
  - path: encounters
    namespace: darkforest.encounters
```

```yaml
# campaign-seaside/manifest.yaml
name: Seaside Campaign
version: "1.0"
namespace: seaside
directories:
  - path: ../shared/npc
    namespace: seaside.npc
  - path: harbors
    namespace: seaside.harbors
```

Each manifest assigns its own namespace to the shared directory. The table files are not copied — both manifests read the same files on disk.

**Chain references and shared tables:** Tables that use relative (bare name) chain references — like `quick-npc.yaml` referencing `npc-occupation` — work correctly when shared this way, because relative references resolve within whatever namespace the manifest assigns. However, if a shared table contains a chain reference using a full FQID (e.g., `darkforest.npc.npc-occupation`), that reference would break when loaded under a different namespace. For portable shared tables, prefer relative references for tables within the same directory.

### Chain Depth Limit

To prevent infinite loops (e.g., table A chains to table B which chains back to table A), fatescroll enforces a maximum chain depth of **10**. If chain resolution exceeds this depth, rolling stops with an error:

```
chain depth limit (10) exceeded at table 'A'
```

## Dice Interpolation

Result text can contain inline dice expressions enclosed in curly braces. When the result is selected, fatescroll evaluates each expression and replaces it with the rolled value.

### Syntax

```
{dice_expression}
```

Any valid diceman dice expression works inside the braces. Examples:

| Text in YAML | Possible output |
|---|---|
| `"Merchant with {2d6x10} gold"` | `Merchant with 70 gold` |
| `"Found {1d4} potions"` | `Found 3 potions` |
| `"Takes {2d6} days to arrive"` | `Takes 8 days to arrive` |

If a dice expression inside braces fails to evaluate (e.g., due to a syntax error), the original text including the braces is preserved unchanged. For example, `{not-valid}` would remain as `{not-valid}` in the output.

### Example from wilderness-encounter.yaml

```yaml
  - min: 8
    max: 8
    text: "Merchant with {2d6x10} gold"
    chain:
      - merchant-goods
```

When result 8 is rolled, `{2d6x10}` is evaluated (e.g., producing 80) and the output reads:

```
Wilderness Encounter (rolled 8): Merchant with 80 gold
  Merchant Goods (rolled 2): Exotic spices and silks
```

## Validation Rules

Running `fatescroll validate` checks your collection for errors. Validation happens in two phases: per-table checks (run as each file is loaded) and cross-reference checks (run after all tables are loaded).

### Per-Table Validation

These checks apply to each simple table individually:

**Dice expression must be parseable.** The `roll` field must be a valid diceman expression. If not, you will see:

```
invalid dice expression '1z6' in table 'My Table': <parse error details>
```

Fix: Check that your dice expression uses valid syntax (e.g., `1d6`, `2d8+1`, `3d6x10`).

**Result ranges must not be reversed.** Each result entry's `min` must be less than or equal to its `max`. If not:

```
range reversed: min 5 > max 2 in table 'My Table'
```

Fix: Swap the min and max values, or correct whichever one is wrong.

**Result ranges must not have gaps.** Every possible value of the dice expression must be covered by exactly one result entry. If values are missing:

```
range gap in table 'My Table': missing values [3, 4]
```

Fix: Add result entries that cover the missing values, or adjust existing ranges to close the gap.

**Result ranges must not overlap.** No value should be covered by more than one result entry. If values overlap:

```
range overlap in table 'My Table': values [3, 4] covered multiple times
```

Fix: Adjust the min/max boundaries so each value falls in exactly one result entry.

### Cross-Reference Validation

These checks run after all tables are loaded, verifying that references actually point to existing tables:

**Chain references must resolve.** Every reference in a result entry's `chain` list must resolve to an existing table (using relative-first resolution). If not:

```
unresolved chain reference 'nonexistent-table' in table 'My Table'
```

Fix: Check the spelling of the reference. If the target table is in a different namespace, use its full FQID. Make sure the referenced table file exists and is in a directory listed in the manifest.

**Compound table references must resolve.** Every reference in a compound table's `tables` list must resolve. If not:

```
unresolved compound table reference 'nonexistent-table' in table 'My Compound Table'
```

Fix: Same as for chain references -- check spelling, use FQIDs for cross-namespace references, and verify the target table exists.

### Namespace Validation

The root namespace in the manifest and every directory namespace are validated:

```
invalid namespace 'Bad.Name': segment 'Bad' must match [a-z][a-z0-9_-]*
```

Fix: Use only lowercase letters, digits, hyphens, and underscores. Each segment must start with a lowercase letter. Do not use empty segments (double dots).

## Example Collection

Here is a complete collection demonstrating simple tables, compound tables, chaining, and dice interpolation. This matches the structure of the test fixtures included with fatescroll.

### Directory Structure

```
my-collection/
  manifest.yaml
  terrain/
    wilderness.yaml
  encounters/
    wilderness-encounter.yaml
    animal-type.yaml
    bandit-strength.yaml
    bandit-motivation.yaml
    merchant-goods.yaml
  npc/
    npc-occupation.yaml
    npc-disposition.yaml
    npc-quirk.yaml
    quick-npc.yaml
```

### manifest.yaml

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

### Simple Table: terrain/wilderness.yaml

A basic table with no chaining:

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

### Simple Table with Chaining and Dice Interpolation: encounters/wilderness-encounter.yaml

This table chains to other tables and uses inline dice:

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

### Compound Table: npc/quick-npc.yaml

Rolls three sub-tables to generate a complete NPC:

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

### Validating the Collection

```bash
fatescroll validate ./my-collection
```

If everything is correct, you will see:

```
Collection is valid.
```

If there are errors, fatescroll reports all of them at once so you can fix multiple issues in a single pass.

### Rolling on Tables

Roll a simple table:

```bash
fatescroll roll --collection ./my-collection test.terrain.wilderness
```

```
Wilderness Terrain (rolled 4): Open plains
```

Roll a table with chaining:

```bash
fatescroll roll --collection ./my-collection test.encounters.wilderness-encounter
```

```
Wilderness Encounter (rolled 4): Bandit camp
  Bandit Strength (rolled 2): Small group (2-3 bandits)
  Bandit Motivation (rolled 3): Deserters from a nearby army
```

Roll a compound table:

```bash
fatescroll roll --collection ./my-collection test.npc.quick-npc
```

```
Quick NPC Generator
  NPC Occupation (rolled 1): Blacksmith
  NPC Disposition (rolled 3): Suspicious
  NPC Quirk (rolled 2): Collects odd trinkets
```
