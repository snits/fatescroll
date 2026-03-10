# Design: Table Template Generator (`fatescroll init`)

## Problem

Creating table YAML files by hand is tedious. Authors need to calculate dice ranges, write boilerplate structure, and ensure min/max values cover the full range without gaps or overlaps. A scaffolding tool would eliminate this repetitive work.

## Solution

Add a `fatescroll init` subcommand that generates table YAML skeletons. It supports three modes:

1. **Explicit** — author provides a dice expression directly
2. **Flat distribution** — author specifies entry count, tool picks `1dN`
3. **Bell curve distribution** — author specifies entry count, tool finds best `XdY` using multiple summed dice

Output goes to stdout by default, with an optional flag to write to a file.

## CLI Interface

```
fatescroll init [OPTIONS]
```

### Options

| Flag | Type | Description |
|---|---|---|
| `--roll <EXPR>` | string | Dice expression (explicit mode). Mutually exclusive with `--entries`. |
| `--entries <N>` | integer | Number of result entries desired. Requires `--distribution`. |
| `--distribution <TYPE>` | string | Distribution type: `flat` or `bell`. Requires `--entries`. |
| `--name <NAME>` | string | Table display name. Defaults to `"Untitled Table"`. |
| `--output <FILE>` | path | Write to file instead of stdout. |

### Mutual Exclusivity

- `--roll` and `--entries` are mutually exclusive
- `--entries` requires `--distribution`
- `--distribution` requires `--entries`
- At least one of `--roll` or `--entries` is required

## Modes

### Explicit Mode (`--roll`)

Parse the dice expression with diceman, then use `diceman::simulate_seeded(expr, 100_000, 42)` to determine the min/max value range (same approach used by the validator in `validator.rs`). Generate a skeleton with one result entry per value in the range.

```bash
fatescroll init --roll 1d6 --name "Wilderness Terrain"
```

Produces:

```yaml
name: Wilderness Terrain
type: simple
tags: []
roll: 1d6
results:
  - min: 1
    max: 1
    text: ""
  - min: 2
    max: 2
    text: ""
  # ... through 6
```

### Flat Distribution (`--entries N --distribution flat`)

Map N entries to `1dN`. Each entry gets a single value.

```bash
fatescroll init --entries 8 --distribution flat --name "Random Events"
```

Produces a skeleton with `roll: 1d8` and 8 entries covering 1-8.

### Bell Curve Distribution (`--entries N --distribution bell`)

Find a multi-dice expression `XdY` where the number of distinct values in the range matches N. The tool tries `X=2` first (most common bell curve), then `X=3`, etc.

**Math:** For `XdY`, the range is `X` to `X*Y`, giving `X*(Y-1) + 1` distinct values.

Solving for Y given N entries and X dice: `Y = (N - 1 + X) / X = (N - 1) / X + 1`

For X=2: `Y = (N + 1) / 2`
For X=3: `Y = (N + 2) / 3`

Y must be a positive integer >= 2 (a d1 is meaningless).

**When exact match exists:** Generate the skeleton directly.

**When no exact match for a given X:** Show the two nearest options and exit with guidance:

```
No exact bell curve match for 12 entries with 2 dice.
  Nearest options:
    2d6  → 11 entries (range 2-12)
    2d7  → 13 entries (range 2-14)
  Use --roll 2d6 or --roll 2d7 to generate.
```

The tool tries X=2 and X=3. For each X value, if no exact match exists, it shows the floor and ceiling options (the two Y values that bracket the desired entry count). All suggestions are shown together so the author can choose. The tool caps at X=3 — higher dice counts produce very narrow bell curves that are rarely useful for RPG tables.

**Example with no exact match at any X:**

```
No exact bell curve match for 12 entries.
  With 2 dice:
    2d6  → 11 entries (range 2-12)
    2d7  → 13 entries (range 2-14)
  With 3 dice:
    3d4  → 10 entries (range 3-12)
    3d5  → 13 entries (range 3-15)
  Use --roll <expression> to generate.
```

### Range Assignment for Bell Curves

For multi-dice expressions, each result entry covers exactly one value in the range (min == max). The bell curve weighting is inherent in the dice probability — the author doesn't need to adjust ranges for probability, just fill in the text for each possible sum.

## Output Format

Generated YAML uses the same structure as hand-authored tables:

```yaml
name: <name>
type: simple
tags: []
roll: <dice expression>
results:
  - min: <start>
    max: <start>
    text: ""
  - min: <start+1>
    max: <start+1>
    text: ""
  # ... one entry per distinct value
```

Each entry has `min == max` (one value per entry). Authors can merge adjacent entries to create ranges covering multiple values if desired.

## CLI Notes

This subcommand intentionally omits `--collection` — it generates standalone YAML, it does not need an existing collection. The `--output` flag refuses to overwrite an existing file (use stdout + redirect to force overwrite).

The generated YAML omits the `id` field. The table's `id` is derived from the filename by the loader, so it is not needed in the YAML. Authors who want to set an explicit `id` can add it after generation.

## Error Cases

- Invalid dice expression in `--roll` — parse error from diceman
- `--entries 0` or negative — validation error
- `--entries 1` or `--entries 2` with `--distribution bell` — "bell curves require at least 3 entries (minimum is 2d2)"
- Bell curve with no exact match — show suggestions, exit with non-zero status
- `--output` path already exists — refuse to overwrite
- `--output` path not writable — IO error

## What This Does NOT Include

- Compound table generation (only simple tables)
- Tag suggestions or auto-population
- Collection-aware output path validation
- Interactive/wizard mode
