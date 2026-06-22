# Roll Modifier Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional table-level roll modifiers (`modifier_range` + runtime `--modifier`) with strict-expand validation and result clamping, including the `u32 → i32` entry-value migration that negative modifiers require.

**Architecture:** Tables may declare `modifier_range [min,max]`. The validator requires entries to *exactly* cover `[dice_min+mod_min, dice_max+mod_max]`. At roll time `--modifier N` yields `clamp(raw_dice + N, entry_min, entry_max)`. Range computation is dual-path: `simulate_seeded` for non-modifier tables (unchanged), analytical `dice_range()` for the modifier envelope. A single `i32` coverage routine checks gaps/overlaps/out-of-range for both.

**Tech Stack:** Rust, Cargo workspace (`fatescroll-core` lib + `fatescroll-cli` bin), serde/serde_yaml, clap v4, diceman, thiserror.

**Reference:** `docs/superpowers/specs/2026-06-22-roll-modifiers-design.md`

**Conventions:** `cargo test` runs all tests. `cargo clippy -- -D warnings` and `cargo fmt --check` must pass (pre-commit hook enforces). Every `.rs` file starts with two `// ABOUTME:` lines. Commit with `git commit -s` and an `Assisted-by: Claude:claude-opus-4-8` trailer. We are on branch `worktree-feat+roll-modifiers-0ax`; never commit to `main`.

---

## File map

| File | Responsibility | Change |
|------|----------------|--------|
| `fatescroll-core/src/models.rs` | Data types | `ResultEntry.min/max → i32`; `RollResult.roll → Option<i32>`; new `ModifierRange`; `Table::Simple.modifier_range` field |
| `fatescroll-core/src/error.rs` | Error enums | coverage error fields → `i32`; 2 new `ValidationError`, 1 new `RollError` |
| `fatescroll-core/src/dice.rs` | Dice utilities | receives `dice_range()` (moved from init) |
| `fatescroll-core/src/init.rs` | Scaffolding | imports `dice_range` from `dice` (no behavior change) |
| `fatescroll-core/src/validator.rs` | Validation | unified `i32` coverage; modifier envelope; digit-dice + reversed-range guards |
| `fatescroll-core/src/roller.rs` | Roll engine | modifier-aware entry points; clamp; `ModifierNotSupported` |
| `fatescroll-core/src/display.rs` | Show formatting | sign-aware width; `modifier_range` line |
| `fatescroll-cli/src/main.rs` | CLI | `roll --modifier <i32>` flag |
| `tests/fixtures/modifier-collection/**` | Test data | new isolated fixture collection |
| `fatescroll-cli/tests/cli_integration.rs` | Integration tests | `--modifier` happy/clamp/error cases |

---

## Task 1: Migrate entry/roll values to `i32`

Foundational, compiler-driven cascade. No `modifier_range` yet. All 175 existing tests must stay green; one new test proves negative entries are now representable.

**Files:**
- Modify: `fatescroll-core/src/models.rs` (`ResultEntry`, `RollResult`)
- Modify: `fatescroll-core/src/error.rs` (`ValidationError` field types)
- Modify: `fatescroll-core/src/validator.rs` (coverage arithmetic)
- Modify: `fatescroll-core/src/roller.rs` (roll value type)
- Modify: `fatescroll-core/src/display.rs` (sign-aware width)

- [ ] **Step 1: Write the failing test** — in `models.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn deserialize_simple_table_with_negative_entries() {
    let yaml = r#"
id: aging
name: Aging
type: simple
roll: 1d6
results:
  - min: -2
    max: -1
    text: Decline
  - min: 0
    max: 6
    text: Stable
"#;
    let table: Table = serde_yaml::from_str(yaml).unwrap();
    match table {
        Table::Simple { results, .. } => {
            assert_eq!(results[0].min, -2);
            assert_eq!(results[0].max, -1);
        }
        _ => panic!("Expected Simple table"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fatescroll-core --lib deserialize_simple_table_with_negative_entries`
Expected: FAIL — serde rejects `-2` for `u32` ("invalid value: integer `-2`").

- [ ] **Step 3: Migrate the types.** Make these exact edits, then let the compiler drive the rest:

`models.rs`:
- `ResultEntry`: `pub min: i32,` `pub max: i32,`
- `RollResult`: `pub roll: Option<i32>,`

`error.rs` — change `u32 → i32` in these `ValidationError` variants:
- `RangeReversed { table: String, min: i32, max: i32 }`
- `EntryOutOfRange { table, entry_min: i32, entry_max: i32, dice_min: i32, dice_max: i32 }`
- `RangeGap { table: String, missing: Vec<i32> }`
- `RangeOverlap { table: String, overlapping: Vec<i32> }`

`validator.rs`:
- `validate_result_entry`: unchanged logic (the `max < min` check works for `i32`).
- `validate_contiguous_coverage`: after the existing `sim.min < 0 || sim.max < 0` rejection (KEEP IT — non-modifier tables stay non-negative), bind `let dice_min = sim.min as i32; let dice_max = sim.max as i32;`. The coverage vec stays `vec![0u32; (dice_max - dice_min + 1) as usize]` (count is non-negative); index with `(entry.min - dice_min) as usize` and `(entry.max - dice_min) as usize`. The `missing`/`overlapping` collectors now produce `i32`: `i as i32 + dice_min`. The pre-check `entry.min < dice_min || entry.max > dice_max` compares `i32`.
- `validate_digit_dice_coverage`: `valid_values` stays `HashSet<u32>`. Guard negatives before casting in BOTH loops, or it won't compile (the `i32` entry value can't index a `u32`-keyed structure):
  - Pre-check loop: `for v in entry.min..=entry.max { if v < 0 || !valid_values.contains(&(v as u32)) { return Err(EntryOutOfRange{..}) } }`.
  - **Coverage-count loop (current `validator.rs:122-128`):** `for v in entry.min..=entry.max { if v >= 0 { if let Some(c) = coverage.get_mut(&(v as u32)) { *c += 1; } } }` — note the explicit `&(v as u32)` cast; the `v >= 0` guard makes it safe (negatives can't be valid digit-dice values and were already rejected by the pre-check).
  - `dice_min`/`dice_max` for the `EntryOutOfRange` error become `i32` via `as i32`. `missing`/`overlapping` collectors cast `u32 → i32` (values ≤ 666, safe).

`roller.rs` — in the `Table::Simple` arm: keep the raw `roll_value: i64` and its `< 0 → NegativeRoll` guard. Replace `let roll_u32 = roll_value as u32;` with `let roll_i32 = roll_value as i32;`. Entry lookup: `roll_i32 >= e.min && roll_i32 <= e.max`. Reroll check (dice value is non-negative here): `reroll_values.contains(&(roll_i32 as u32))`. `RollResult.roll: Some(roll_i32)`. The reroll-exhaustion `break (roll_i32, entry.clone())` and downstream usage follow.

`display.rs` — `digit_count` panics on `≤ 0`. Replace the width calculation: compute each entry's rendered range string first, then `let range_width = results.iter().map(|r| render_range(r.min, r.max).len()).max().unwrap_or(1);` where `render_range` is the same `if min==max { format!("{min}") } else { format!("{min}-{max}") }` already used in the loop. Delete `digit_count` (now unused). This is sign-aware automatically (negative sign counts toward width).

- [ ] **Step 4: Update existing tests that assert `roll`/error field types.** `models.rs` `roll_result_serializes_to_json` still works (`v["roll"]` compares to `4`). No existing test asserts negative values; the migration is type-only. Run the full suite.

Run: `cargo test`
Expected: PASS — all existing tests (≈175 at baseline) plus the new negative-deserialize test green.

- [ ] **Step 5: Add a display test for negative alignment** — in `display.rs` tests:

```rust
#[test]
fn format_table_with_negative_entries_aligns() {
    let table = Table::Simple {
        id: "aging".into(),
        name: "Aging".into(),
        tags: vec![],
        roll: "1d6".into(),
        results: vec![
            ResultEntry { min: -2, max: -1, text: Some("Decline".into()), chain: None },
            ResultEntry { min: 0, max: 6, text: Some("Stable".into()), chain: None },
        ],
    };
    let output = format_table("ns.aging", &table);
    // With min=-2, max=-1 the renderer emits "-2--1". Both rows align to the
    // widest rendered range string ("-2--1" = 5 chars).
    assert!(output.contains("-2--1"));
    assert!(output.contains("Decline"));
    assert!(output.contains("Stable"));
}
```

- [ ] **Step 6: Run, then clippy + fmt**

Run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -s -m "refactor: migrate entry and roll values from u32 to i32

Enables negative entry values required by negative roll modifiers
(e.g. Traveller aging). Pure type migration; no behavior change for
existing tables. Sign-aware display width replaces digit_count.

Refs: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 2: Move `dice_range()` to `dice.rs`

Pure relocation so both `validator` and `init` can call it without `validator → init` coupling.

**Files:**
- Modify: `fatescroll-core/src/dice.rs` (gain `dice_range` + its tests)
- Modify: `fatescroll-core/src/init.rs` (remove `dice_range`, import from `dice`)

- [ ] **Step 1: Move the function.** Cut `pub fn dice_range(...)` and the private `fn unsupported(...)` from `init.rs` and paste into `dice.rs`. Move the `use diceman::{Expr, Op};` import to `dice.rs` (and `use crate::error::{Error, ValidationError};`). In `init.rs`, add `use crate::dice::dice_range;` and keep `use diceman::Expr;` only if still referenced (it is, in `generate_template`). Move the `dice_range_*` unit tests from `init.rs`'s test module into `dice.rs`'s test module.

- [ ] **Step 2: Run tests to verify the move is clean**

Run: `cargo test -p fatescroll-core --lib dice_range`
Expected: PASS — all `dice_range_*` tests pass from their new home.

- [ ] **Step 3: Full build + lint**

Run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -s -m "refactor: move dice_range from init to dice module

Both the validator (modifier envelope) and init scaffolding need it;
dice.rs is its natural home. Pure move, no behavior change.

Refs: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 3: Add `ModifierRange` and `Table::Simple.modifier_range`

**Files:**
- Modify: `fatescroll-core/src/models.rs`

- [ ] **Step 1: Write the failing tests** — in `models.rs` tests:

```rust
#[test]
fn deserialize_table_with_modifier_range() {
    let yaml = r#"
id: carousing
name: Carousing
type: simple
roll: 1d8
modifier_range: [0, 6]
results:
  - min: 1
    max: 14
    text: Outcome
"#;
    let table: Table = serde_yaml::from_str(yaml).unwrap();
    match table {
        Table::Simple { modifier_range, .. } => {
            let mr = modifier_range.expect("expected modifier_range");
            assert_eq!(mr.min, 0);
            assert_eq!(mr.max, 6);
        }
        _ => panic!("Expected Simple table"),
    }
}

#[test]
fn modifier_range_absent_defaults_to_none() {
    let yaml = r#"
id: plain
name: Plain
type: simple
roll: 1d6
results:
  - min: 1
    max: 6
    text: X
"#;
    let table: Table = serde_yaml::from_str(yaml).unwrap();
    match table {
        Table::Simple { modifier_range, .. } => assert!(modifier_range.is_none()),
        _ => panic!("Expected Simple table"),
    }
}

#[test]
fn modifier_range_round_trips_as_sequence() {
    // Symmetric serde: serialize emits a 2-seq, deserialize accepts it back.
    let mr = ModifierRange { min: -6, max: 0 };
    let v = serde_json::to_value(mr).unwrap();
    assert!(v.is_array(), "expected sequence form, got: {v}");
    let back: ModifierRange = serde_json::from_value(v).unwrap();
    assert_eq!(back, mr);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p fatescroll-core --lib modifier_range`
Expected: FAIL — `ModifierRange` undefined / no `modifier_range` field.

- [ ] **Step 3: Implement.** In `models.rs`:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct ModifierRange {
    pub min: i32,
    pub max: i32,
}
```

Deserializing `[0, 6]` into a struct with two fields does NOT work out of the box — serde expects a map. Make the type *symmetric*: deserialize from and serialize to a `[i32; 2]` sequence via `#[serde(from = ..., into = ...)]` plus the two `From` impls. Symmetry keeps `Table` round-trippable (serialize → YAML/JSON → deserialize), which the existing `table_simple_json_round_trips_with_chain_refs` test relies on:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "[i32; 2]", into = "[i32; 2]")]
pub struct ModifierRange {
    pub min: i32,
    pub max: i32,
}

impl From<[i32; 2]> for ModifierRange {
    fn from(v: [i32; 2]) -> Self {
        ModifierRange { min: v[0], max: v[1] }
    }
}

impl From<ModifierRange> for [i32; 2] {
    fn from(m: ModifierRange) -> Self {
        [m.min, m.max]
    }
}
```

Add the field to `Table::Simple`:

```rust
    Simple {
        id: String,
        name: String,
        #[serde(default)]
        tags: Vec<String>,
        roll: String,
        #[serde(default)]
        modifier_range: Option<ModifierRange>,
        results: Vec<ResultEntry>,
    },
```

Every existing `Table::Simple { .. }` construction in non-test code uses `..` or names fields; update any exhaustive constructors. **All `Table::Simple { ... }` literals in tests across `models.rs`, `validator.rs`, `roller.rs`, `display.rs` must add `modifier_range: None,`** — the compiler will list each site.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test`
Expected: PASS — new tests green, all prior tests green after adding `modifier_range: None` to test constructors.

- [ ] **Step 5: Lint + commit**

Run: `cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

```bash
git add -A
git commit -s -m "feat: add modifier_range field to simple tables

Optional [min,max] declaration, deserialized from a YAML sequence.
Absent => None, so existing tables remain valid.

Refs: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 4: Validator — unified `i32` coverage + modifier envelope

Refactor the two contiguous coverage checks to share one `i32` routine, and add the modifier-envelope path with its guards.

**Files:**
- Modify: `fatescroll-core/src/error.rs` (2 new variants)
- Modify: `fatescroll-core/src/validator.rs`

- [ ] **Step 1: Add the error variants** to `ValidationError` in `error.rs`:

```rust
    #[error("modifier_range reversed: min {min} > max {max} in table '{table}'")]
    ModifierRangeReversed { table: String, min: i32, max: i32 },

    #[error("modifier_range not supported for digit-dice expression '{expr}' in table '{table}'")]
    ModifierUnsupportedForDigitDice { table: String, expr: String },

    #[error("modifier_range envelope too wide ({width}) in table '{table}'; max {max}")]
    ModifierRangeTooWide { table: String, width: i64, max: i64 },
```

Also reword the existing `EntryOutOfRange` message so it reads correctly for
modifier tables (the fields now carry the *envelope* for those). Change its
`#[error(...)]` text from `... outside dice range [{dice_min}..{dice_max}] ...`
to envelope-neutral wording:

```rust
    #[error(
        "entry range [{entry_min}..{entry_max}] outside valid range [{dice_min}..{dice_max}] in table '{table}'"
    )]
    EntryOutOfRange { table: String, entry_min: i32, entry_max: i32, dice_min: i32, dice_max: i32 },
```

- [ ] **Step 2: Write failing validator tests** — in `validator.rs` tests. Add a small constructor helper if convenient, otherwise inline. (Remember `modifier_range:` field on every `Table::Simple`.)

```rust
#[test]
fn modifier_table_strict_expand_valid_shadowdark() {
    // 1d8 (1-8) + [0,6] => envelope [1,14]; entries exactly cover 1..=14
    let results = (1..=14)
        .map(|v| ResultEntry { min: v, max: v, text: Some(format!("E{v}")), chain: None })
        .collect();
    let table = Table::Simple {
        id: "carousing".into(), name: "Carousing".into(), tags: vec![],
        roll: "1d8".into(), modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
        results,
    };
    assert!(validate_table(&table).is_ok());
}

#[test]
fn modifier_table_strict_expand_valid_traveller_negative() {
    // 1d6 (1-6) + [-6,0] => envelope [-5,6]; entries exactly cover -5..=6
    let results = (-5..=6)
        .map(|v| ResultEntry { min: v, max: v, text: Some(format!("E{v}")), chain: None })
        .collect();
    let table = Table::Simple {
        id: "aging".into(), name: "Aging".into(), tags: vec![],
        roll: "1d6".into(), modifier_range: Some(crate::models::ModifierRange { min: -6, max: 0 }),
        results,
    };
    assert!(validate_table(&table).is_ok());
}

#[test]
fn modifier_table_gap_errors() {
    // envelope [1,14] but entry 7 missing
    let results: Vec<ResultEntry> = (1..=14).filter(|&v| v != 7)
        .map(|v| ResultEntry { min: v, max: v, text: Some("E".into()), chain: None })
        .collect();
    let table = Table::Simple {
        id: "carousing".into(), name: "Carousing".into(), tags: vec![],
        roll: "1d8".into(), modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
        results,
    };
    assert!(matches!(validate_table(&table).unwrap_err(), ValidationError::RangeGap { .. }));
}

#[test]
fn modifier_table_entry_beyond_envelope_errors() {
    // envelope [1,14] but an entry at 15
    let mut results: Vec<ResultEntry> = (1..=14)
        .map(|v| ResultEntry { min: v, max: v, text: Some("E".into()), chain: None }).collect();
    results.push(ResultEntry { min: 15, max: 15, text: Some("Over".into()), chain: None });
    let table = Table::Simple {
        id: "carousing".into(), name: "Carousing".into(), tags: vec![],
        roll: "1d8".into(), modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
        results,
    };
    assert!(matches!(validate_table(&table).unwrap_err(), ValidationError::EntryOutOfRange { .. }));
}

#[test]
fn modifier_range_reversed_errors() {
    let table = Table::Simple {
        id: "bad".into(), name: "Bad".into(), tags: vec![],
        roll: "1d8".into(), modifier_range: Some(crate::models::ModifierRange { min: 6, max: 0 }),
        results: vec![ResultEntry { min: 1, max: 8, text: Some("X".into()), chain: None }],
    };
    assert!(matches!(validate_table(&table).unwrap_err(), ValidationError::ModifierRangeReversed { .. }));
}

#[test]
fn modifier_range_on_digit_dice_errors() {
    let table = Table::Simple {
        id: "d66".into(), name: "D66".into(), tags: vec![],
        roll: "D66".into(), modifier_range: Some(crate::models::ModifierRange { min: 0, max: 1 }),
        results: vec![ResultEntry { min: 11, max: 11, text: Some("X".into()), chain: None }],
    };
    assert!(matches!(validate_table(&table).unwrap_err(), ValidationError::ModifierUnsupportedForDigitDice { .. }));
}

#[test]
fn modifier_range_on_complex_expr_errors() {
    // dice_range() rejects keep/exploding etc. with UnsupportedDiceExpression
    let table = Table::Simple {
        id: "kh".into(), name: "KH".into(), tags: vec![],
        roll: "4d6kh3".into(), modifier_range: Some(crate::models::ModifierRange { min: 0, max: 1 }),
        results: vec![ResultEntry { min: 3, max: 19, text: Some("X".into()), chain: None }],
    };
    assert!(validate_table(&table).is_err());
}

#[test]
fn absurd_modifier_range_errors_not_panics() {
    // An i32::MAX-wide envelope must error cleanly, not overflow or OOM.
    let table = Table::Simple {
        id: "huge".into(), name: "Huge".into(), tags: vec![],
        roll: "1d8".into(),
        modifier_range: Some(crate::models::ModifierRange { min: 0, max: i32::MAX }),
        results: vec![ResultEntry { min: 1, max: 8, text: Some("X".into()), chain: None }],
    };
    assert!(matches!(validate_table(&table).unwrap_err(), ValidationError::ModifierRangeTooWide { .. }));
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test -p fatescroll-core --lib modifier_table`
Expected: FAIL — modifier path not implemented; `modifier_range` ignored.

- [ ] **Step 4: Implement.** Refactor `validator.rs`:

Extract the shared coverage logic into one routine over an `i32` envelope:

```rust
/// Check that `results` form a contiguous, non-overlapping cover of
/// [envelope_min, envelope_max] exactly. Any value outside the envelope,
/// any gap, or any overlap is an error.
fn validate_envelope_coverage(
    name: &str,
    envelope_min: i32,
    envelope_max: i32,
    results: &[ResultEntry],
) -> Result<(), ValidationError> {
    for entry in results {
        if entry.min < envelope_min || entry.max > envelope_max {
            return Err(ValidationError::EntryOutOfRange {
                table: name.to_string(),
                entry_min: entry.min,
                entry_max: entry.max,
                dice_min: envelope_min,
                dice_max: envelope_max,
            });
        }
    }
    let len = (envelope_max - envelope_min + 1) as usize;
    let mut coverage = vec![0u32; len];
    for entry in results {
        let start = (entry.min - envelope_min) as usize;
        let end = (entry.max - envelope_min) as usize;
        for slot in coverage.iter_mut().take(end + 1).skip(start) {
            *slot += 1;
        }
    }
    let missing: Vec<i32> = coverage.iter().enumerate()
        .filter(|(_, c)| **c == 0).map(|(i, _)| i as i32 + envelope_min).collect();
    if !missing.is_empty() {
        return Err(ValidationError::RangeGap { table: name.to_string(), missing });
    }
    let overlapping: Vec<i32> = coverage.iter().enumerate()
        .filter(|(_, c)| **c > 1).map(|(i, _)| i as i32 + envelope_min).collect();
    if !overlapping.is_empty() {
        return Err(ValidationError::RangeOverlap { table: name.to_string(), overlapping });
    }
    Ok(())
}
```

Rewrite the `Table::Simple` arm of `validate_table` to compute the envelope and dispatch:

```rust
Table::Simple { name, roll, results, modifier_range, .. } => {
    let parsed = diceman::parse(roll).map_err(|e| ValidationError::InvalidDiceExpression {
        table: name.clone(), expr: roll.clone(), reason: e.to_string(),
    })?;
    for entry in results { validate_result_entry(entry, name)?; }

    if let Expr::DigitRoll { sides, count } = parsed {
        if modifier_range.is_some() {
            return Err(ValidationError::ModifierUnsupportedForDigitDice {
                table: name.clone(), expr: roll.clone(),
            });
        }
        return validate_digit_dice_coverage(name, roll, results, sides, count);
    }

    let (envelope_min, envelope_max) = match modifier_range {
        Some(mr) => {
            if mr.min > mr.max {
                return Err(ValidationError::ModifierRangeReversed {
                    table: name.clone(), min: mr.min, max: mr.max,
                });
            }
            // Analytical dice range; propagates UnsupportedDiceExpression for
            // complex expressions, and InvalidDiceExpression for parse issues.
            let (d_min, d_max) = crate::dice::dice_range(roll)
                .map_err(|e| match e {
                    crate::error::Error::Validation(v) => v,
                    other => ValidationError::InvalidDiceExpression {
                        table: name.clone(), expr: roll.clone(), reason: other.to_string(),
                    },
                })?;
            // Widen to i64: modifier_range bounds are unbounded i32, so
            // `d_max + mr.max` can overflow. Bound the envelope width before
            // allocating the coverage vec (an unbounded width would also OOM).
            const MAX_ENVELOPE_WIDTH: i64 = 100_000;
            let env_min = d_min as i64 + mr.min as i64;
            let env_max = d_max as i64 + mr.max as i64;
            let width = env_max - env_min;
            if width > MAX_ENVELOPE_WIDTH {
                return Err(ValidationError::ModifierRangeTooWide {
                    table: name.clone(), width, max: MAX_ENVELOPE_WIDTH,
                });
            }
            (env_min as i32, env_max as i32)
        }
        None => {
            // Unchanged path: simulate, reject negative dice ranges.
            let sim = diceman::simulate_seeded(roll, 100_000, 42).map_err(|e| {
                ValidationError::InvalidDiceExpression {
                    table: name.clone(), expr: roll.clone(), reason: e.to_string(),
                }
            })?;
            if sim.min < 0 || sim.max < 0 {
                return Err(ValidationError::InvalidDiceExpression {
                    table: name.clone(), expr: roll.clone(),
                    reason: format!("dice range [{}, {}] includes negative values", sim.min, sim.max),
                });
            }
            (sim.min as i32, sim.max as i32)
        }
    };
    validate_envelope_coverage(name, envelope_min, envelope_max, results)
}
```

Delete the now-superseded `validate_contiguous_coverage` (its body is replaced by `validate_envelope_coverage` + the envelope computation above). Keep `validate_digit_dice_coverage` (now with `i32`-safe casts from Task 1). Ensure `use crate::models::ModifierRange;` if referenced, and `dice_range` is reachable via `crate::dice`.

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p fatescroll-core`
Expected: PASS — new modifier tests green; the existing contiguous-coverage tests (`valid_simple_table_full_coverage`, `simple_table_with_gap`, etc.) still pass through the unified routine.

- [ ] **Step 6: Lint + commit**

Run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

```bash
git add -A
git commit -s -m "feat: validate modifier_range with strict-expand coverage

Entries must exactly cover [dice_min+mod_min, dice_max+mod_max].
Unifies contiguous coverage into one i32 routine; rejects modifier_range
on digit-dice and reversed ranges.

Refs: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 5: Roller — `--modifier` clamping and errors

Add modifier-aware entry points; modifier applies to the top-level table only.

**Files:**
- Modify: `fatescroll-core/src/error.rs` (1 new `RollError`)
- Modify: `fatescroll-core/src/roller.rs`
- Modify: `fatescroll-cli/src/main.rs` (call-site compile fix only)

- [ ] **Step 1: Add the error variant** to `RollError` in `error.rs`:

```rust
    #[error("table '{table}' does not support a roll modifier (no modifier_range declared, or it is a compound table)")]
    ModifierNotSupported { table: String },
```

- [ ] **Step 2: Write failing roller tests** — in `roller.rs` tests (use `ModifierRange` import). Add a helper to build a carousing table inline.

```rust
fn carousing_registry() -> Registry {
    let mut reg = Registry::new();
    let results = (1..=14)
        .map(|v| ResultEntry { min: v, max: v, text: Some(format!("E{v}")), chain: None })
        .collect();
    reg.register("ns.carousing".into(), Table::Simple {
        id: "carousing".into(), name: "Carousing".into(), tags: vec![],
        roll: "1d8".into(),
        modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
        results,
    }).unwrap();
    reg
}

#[test]
fn modifier_shifts_lookup_value() {
    let reg = carousing_registry();
    // With a fixed seed the raw 1d8 is deterministic; assert roll == raw + modifier
    let mut rng = diceman::FastRng::with_seed(7);
    let raw = roll_with_rng_modifier(&reg, "ns.carousing", None, &mut rng).unwrap().roll.unwrap();
    let mut rng2 = diceman::FastRng::with_seed(7);
    let modded = roll_with_rng_modifier(&reg, "ns.carousing", Some(3), &mut rng2).unwrap().roll.unwrap();
    assert_eq!(modded, raw + 3);
}

#[test]
fn modifier_clamps_overflow_high() {
    let reg = carousing_registry();
    // Any raw 1-8 plus +100 clamps to entry max 14
    for seed in 0..50 {
        let mut rng = diceman::FastRng::with_seed(seed);
        let r = roll_with_rng_modifier(&reg, "ns.carousing", Some(100), &mut rng).unwrap();
        assert_eq!(r.roll.unwrap(), 14);
    }
}

#[test]
fn modifier_clamps_overflow_low() {
    let reg = carousing_registry();
    for seed in 0..50 {
        let mut rng = diceman::FastRng::with_seed(seed);
        let r = roll_with_rng_modifier(&reg, "ns.carousing", Some(-100), &mut rng).unwrap();
        assert_eq!(r.roll.unwrap(), 1);
    }
}

#[test]
fn modifier_on_table_without_modifier_range_errors() {
    let reg = build_test_registry(); // existing helper; test.simple has no modifier_range
    let mut rng = diceman::FastRng::with_seed(1);
    let err = roll_with_rng_modifier(&reg, "test.simple", Some(2), &mut rng).unwrap_err();
    assert!(matches!(err, RollError::ModifierNotSupported { .. }));
}

#[test]
fn modifier_on_compound_errors() {
    let reg = build_test_registry(); // existing helper has test.compound
    let mut rng = diceman::FastRng::with_seed(1);
    let err = roll_with_rng_modifier(&reg, "test.compound", Some(1), &mut rng).unwrap_err();
    assert!(matches!(err, RollError::ModifierNotSupported { .. }));
}

#[test]
fn modifier_does_not_leak_to_children() {
    // A modifier table that chains: child must roll un-modified (its own range).
    let mut reg = Registry::new();
    let parent_results = (1..=14).map(|v| {
        let chain = if v == 1 { Some(vec![ChainRef::Simple("child".into())]) } else { None };
        ResultEntry { min: v, max: v, text: Some("P".into()), chain }
    }).collect();
    reg.register("ns.parent".into(), Table::Simple {
        id: "parent".into(), name: "Parent".into(), tags: vec![], roll: "1d8".into(),
        modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
        results: parent_results,
    }).unwrap();
    reg.register("ns.child".into(), Table::Simple {
        id: "child".into(), name: "Child".into(), tags: vec![], roll: "1d6".into(),
        modifier_range: None,
        results: vec![ResultEntry { min: 1, max: 6, text: Some("C".into()), chain: None }],
    }).unwrap();
    for seed in 0..100 {
        let mut rng = diceman::FastRng::with_seed(seed);
        let r = roll_with_rng_modifier(&reg, "ns.parent", Some(6), &mut rng).unwrap();
        for child in &r.children {
            assert!(child.roll.unwrap() >= 1 && child.roll.unwrap() <= 6);
        }
    }
}

#[test]
fn validated_modifier_table_never_out_of_range() {
    let reg = carousing_registry();
    for seed in 0..500 {
        for m in [-100, -6, 0, 6, 100] {
            let mut rng = diceman::FastRng::with_seed(seed);
            assert!(roll_with_rng_modifier(&reg, "ns.carousing", Some(m), &mut rng).is_ok());
        }
    }
}

#[test]
fn extreme_modifier_clamps_without_overflow() {
    // i32::MAX / i32::MIN must clamp to entry bounds, not panic or wrap.
    let reg = carousing_registry();
    let mut rng = diceman::FastRng::with_seed(3);
    assert_eq!(roll_with_rng_modifier(&reg, "ns.carousing", Some(i32::MAX), &mut rng).unwrap().roll.unwrap(), 14);
    let mut rng = diceman::FastRng::with_seed(3);
    assert_eq!(roll_with_rng_modifier(&reg, "ns.carousing", Some(i32::MIN), &mut rng).unwrap().roll.unwrap(), 1);
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test -p fatescroll-core --lib modifier`
Expected: FAIL — `roll_with_rng_modifier` undefined.

- [ ] **Step 4: Implement.** In `roller.rs`:

Add public entry points alongside the existing `roll`/`roll_with_rng`:

```rust
pub fn roll_with_modifier(
    registry: &Registry,
    table_id: &str,
    modifier: Option<i32>,
) -> Result<RollResult, RollError> {
    roll_with_rng_modifier(registry, table_id, modifier, &mut diceman::FastRng::new())
}

pub fn roll_with_rng_modifier(
    registry: &Registry,
    table_id: &str,
    modifier: Option<i32>,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(registry, table_id, "", rng, 0, &[], modifier)
}
```

Keep `roll` and `roll_with_rng` as-is but route them through the new param:

```rust
pub fn roll_with_rng(
    registry: &Registry,
    table_id: &str,
    rng: &mut impl diceman::Rng,
) -> Result<RollResult, RollError> {
    roll_recursive(registry, table_id, "", rng, 0, &[], None)
}
```

Add `modifier: Option<i32>` as the last param of `roll_recursive`. In the `Table::Simple` arm (bind `modifier_range` in the match):
- If `modifier.is_some() && modifier_range.is_none()` → `return Err(RollError::ModifierNotSupported { table: name.clone() })`.
- **Clamp only for `modifier_range` tables.** Non-modifier tables keep the existing find-or-`RollOutOfRange` behavior — clamping them would silently swallow the out-of-range case (and break `roll_out_of_range_errors`). Compute the lookup value conditionally:

```rust
let lookup = match modifier_range {
    Some(_) => match (
        results.iter().map(|e| e.min).min(),
        results.iter().map(|e| e.max).max(),
    ) {
        (Some(entry_min), Some(entry_max)) => {
            // Widen to i64 before adding: --modifier is an unbounded i32, so
            // `roll_i32 + modifier` would overflow (debug panic / release wrap)
            // for extreme values. The spec's "allow overflow" means clamp, not wrap.
            ((roll_i32 as i64) + (modifier.unwrap_or(0) as i64))
                .clamp(entry_min as i64, entry_max as i64) as i32
        }
        // Empty results: don't clamp. The entry search below then yields
        // RollOutOfRange — parity with the non-modifier path, no panic. The
        // roller is a public API callable on an unvalidated registry, so we
        // must not `.unwrap()` here even though load_collection rejects empties.
        _ => roll_i32,
    },
    None => roll_i32, // unchanged path; modifier is guaranteed None here
};
```

  Use `lookup` for entry finding (`lookup >= e.min && lookup <= e.max`) and for `RollResult.roll: Some(lookup)`. The `RollOutOfRange` arm stays — it is now only reachable on the `None` branch (a modifier table's clamped `lookup` is always in `[entry_min, entry_max]`).
- Reroll comparison stays on the **raw dice value** (`reroll_values.contains(&(roll_i32 as u32))`) — reroll is a chain/dice concept; top-level `reroll_values` is empty so it never combines with a modifier.
- Recursive calls for chains pass `None` as the modifier: `roll_recursive(registry, chain_ref.table_id(), namespace, rng, depth + 1, chain_ref.reroll_values(), None)`.

In the `Table::Compound` arm:
- If `modifier.is_some()` → `return Err(RollError::ModifierNotSupported { table: name.clone() })`.
- Sub-table recursion passes `None`.

In `main.rs` (compile fix only — real wiring is Task 7): `cmd_roll` currently calls `fatescroll_core::roller::roll(&registry, table_id)`. Leave it calling `roll` (still valid). No change needed yet unless the signature of `roll` changed — it did not. So `main.rs` may need **no** change here; confirm it still compiles.

- [ ] **Step 5: Run to verify pass**

Run: `cargo test`
Expected: PASS — all new modifier roller tests green; existing roller tests unaffected.

- [ ] **Step 6: Lint + commit**

Run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

```bash
git add -A
git commit -s -m "feat: apply and clamp roll modifiers in the roller

roll_with_modifier applies --modifier to the top-level table only:
lookup = clamp(raw_dice + modifier, entry_min, entry_max). Modifier on a
table without modifier_range, or on a compound, errors. Modifier never
leaks into chained sub-tables.

Refs: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 6: Display — `modifier_range` line

(Sign-aware width already landed in Task 1.)

**Files:**
- Modify: `fatescroll-core/src/display.rs`

- [ ] **Step 1: Write the failing test** — in `display.rs` tests:

```rust
#[test]
fn format_table_shows_modifier_range() {
    let table = Table::Simple {
        id: "carousing".into(), name: "Carousing".into(), tags: vec![],
        roll: "1d8".into(),
        modifier_range: Some(crate::models::ModifierRange { min: 0, max: 6 }),
        results: (1..=14).map(|v| ResultEntry { min: v, max: v, text: Some("E".into()), chain: None }).collect(),
    };
    let output = format_table("ns.carousing", &table);
    assert!(output.contains("Modifier: 0 to 6"));
}

#[test]
fn format_table_without_modifier_range_omits_line() {
    let table = Table::Simple {
        id: "plain".into(), name: "Plain".into(), tags: vec![],
        roll: "1d6".into(), modifier_range: None,
        results: vec![ResultEntry { min: 1, max: 6, text: Some("X".into()), chain: None }],
    };
    let output = format_table("ns.plain", &table);
    assert!(!output.contains("Modifier:"));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p fatescroll-core --lib modifier_range`
Expected: FAIL — no `Modifier:` line emitted.

- [ ] **Step 3: Implement.** In `display.rs`, in the `Table::Simple` arm, bind `modifier_range` in the match and emit a line after the `Roll:` line and before the blank line:

```rust
            writeln!(out, "Roll: {roll}").unwrap();
            if let Some(mr) = modifier_range {
                writeln!(out, "Modifier: {} to {}", mr.min, mr.max).unwrap();
            }
            writeln!(out).unwrap();
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p fatescroll-core --lib`
Expected: PASS.

- [ ] **Step 5: Lint + commit**

Run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

```bash
git add -A
git commit -s -m "feat: show modifier_range in table display

Refs: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 7: CLI `--modifier` flag + fixtures + integration tests

**Files:**
- Modify: `fatescroll-cli/src/main.rs`
- Create: `tests/fixtures/modifier-collection/manifest.yaml`
- Create: `tests/fixtures/modifier-collection/shadowdark/carousing.yaml`
- Create: `tests/fixtures/modifier-collection/traveller/aging.yaml`
- Modify: `fatescroll-cli/tests/cli_integration.rs`

- [ ] **Step 1: Create the fixture collection.**

`tests/fixtures/modifier-collection/manifest.yaml`:

```yaml
name: Modifier Test Collection
version: "1.0"
namespace: mod
author: ~
min_tool_version: ~
directories:
  - path: shadowdark
    namespace: mod.shadowdark
  - path: traveller
    namespace: mod.traveller
```

`tests/fixtures/modifier-collection/shadowdark/carousing.yaml` (file stem `carousing`; `id` omitted — loader derives it):

```yaml
name: Carousing
type: simple
tags: [shadowdark]
roll: 1d8
modifier_range: [0, 6]
results:
  - { min: 1,  max: 1,  text: "Thrown in jail" }
  - { min: 2,  max: 2,  text: "Robbed blind" }
  - { min: 3,  max: 4,  text: "Hungover, lose a day" }
  - { min: 5,  max: 7,  text: "A good night" }
  - { min: 8,  max: 10, text: "Made a friend" }
  - { min: 11, max: 13, text: "Lucky windfall" }
  - { min: 14, max: 14, text: "Legendary carouse" }
```

`tests/fixtures/modifier-collection/traveller/aging.yaml` (stem `aging`):

```yaml
name: Aging
type: simple
tags: [traveller]
roll: 1d6
modifier_range: [-6, 0]
results:
  - { min: -5, max: -3, text: "Severe decline" }
  - { min: -2, max: -1, text: "Decline" }
  - { min: 0,  max: 2,  text: "Aging sets in" }
  - { min: 3,  max: 6,  text: "No effect" }
```

Verify the fixture validates:

Run: `cargo run -p fatescroll-cli -- validate --collection tests/fixtures/modifier-collection`
Expected: `Collection is valid.`

- [ ] **Step 2: Write failing integration tests** — in `fatescroll-cli/tests/cli_integration.rs` (follow the existing `Command::new(env!("CARGO_BIN_EXE_fatescroll"))` + `fixtures_path()` pattern):

```rust
#[test]
fn roll_with_modifier_clamps_high() {
    let manifest = fixtures_path("modifier-collection/manifest.yaml");
    let output = Command::new(env!("CARGO_BIN_EXE_fatescroll"))
        .args(["roll", "--collection"]).arg(&manifest)
        .args(["mod.shadowdark.carousing", "--modifier", "100"])
        .output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("rolled 14"), "got: {stdout}");
}

#[test]
fn roll_with_modifier_on_plain_table_errors() {
    // A non-modifier table from valid-collection
    let manifest = fixtures_path("valid-collection/manifest.yaml");
    let output = Command::new(env!("CARGO_BIN_EXE_fatescroll"))
        .args(["roll", "--collection"]).arg(&manifest)
        .args(["test.encounters.animal-type", "--modifier", "2"])
        .output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("does not support a roll modifier"), "got: {stderr}");
}

#[test]
fn roll_without_modifier_still_works() {
    let manifest = fixtures_path("modifier-collection/manifest.yaml");
    let output = Command::new(env!("CARGO_BIN_EXE_fatescroll"))
        .args(["roll", "--collection"]).arg(&manifest)
        .arg("mod.traveller.aging")
        .output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}
```

(`test.encounters.animal-type` is a confirmed non-modifier table in `valid-collection`.)

- [ ] **Step 3: Run to verify failure**

Run: `cargo test -p fatescroll-cli roll_with_modifier`
Expected: FAIL — `--modifier` is an unknown argument (clap error), so the command fails for the wrong reason / clamp assertion unmet.

- [ ] **Step 4: Implement the flag.** In `main.rs`:

Add to the `Roll` subcommand variant:

```rust
    Roll {
        /// Path to collection directory or manifest file
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
        /// Apply a roll modifier (requires the table to declare modifier_range)
        #[arg(long, allow_negative_numbers = true)]
        modifier: Option<i32>,
    },
```

Update the dispatch arm:

```rust
        Commands::Roll { collection, table_id, modifier } => resolve_collection(collection)
            .and_then(|collection| cmd_roll(&collection, &table_id, modifier)),
```

Update `cmd_roll`:

```rust
fn cmd_roll(collection: &Path, table_id: &str, modifier: Option<i32>) -> Result<(), fatescroll_core::Error> {
    let registry = fatescroll_core::load_collection(collection)?;
    let result = fatescroll_core::roller::roll_with_modifier(&registry, table_id, modifier)?;
    print_roll_result(&result, 0);
    Ok(())
}
```

(`print_roll_result` already formats `roll` as an integer; `i32` works unchanged.)

- [ ] **Step 5: Run to verify pass**

Run: `cargo test -p fatescroll-cli`
Expected: PASS — modifier integration tests green; existing CLI tests unaffected.

- [ ] **Step 6: Full suite + lint + commit**

Run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
Expected: PASS clean.

```bash
git add -A
git commit -s -m "feat: add roll --modifier flag with fixtures and integration tests

--modifier N applies a clamped modifier to a modifier_range table.
Adds shadowdark-carousing and traveller-aging fixtures.

Closes: fatescroll-0ax

Assisted-by: Claude:claude-opus-4-8"
```

---

## Known behavior notes (documented decisions, from plan review)

- **`modifier_range` with `min > 0`, rolled without `--modifier`:** the modifier
  defaults to 0, so low raw rolls clamp up to the bottom entry and the top of
  the envelope is unreachable. Safe (no panic, no `RollOutOfRange`) and an
  inherent consequence of "modifier optional + clamp." Real tables (both
  fixtures) use `mod_min ≤ 0`, so they are unaffected. Accepted for a personal
  tool; not worth extra validation.
- **A `roll` expression with an intrinsic negative range (e.g. `1d6-2`) plus a
  `modifier_range`** is rejected by `dice_range()` (`UnsupportedDiceExpression`)
  even if the modifier would lift the envelope positive. The dice expression's
  own range must be non-negative; negativity must come from `modifier_range`.
  Accepted known limitation.
- **The clamp path assumes a validated (non-empty) table** for correctness but
  does not *panic* on an empty one (guarded — falls through to `RollOutOfRange`).

## Final verification (after all tasks)

- [ ] `cargo test` — full workspace green.
- [ ] `cargo clippy -- -D warnings` — clean.
- [ ] `cargo fmt --check` — clean.
- [ ] Manual smoke: `cargo run -p fatescroll-cli -- show --collection tests/fixtures/modifier-collection mod.shadowdark.carousing` shows the `Modifier: 0 to 6` line.
- [ ] Manual smoke: `cargo run -p fatescroll-cli -- roll --collection tests/fixtures/modifier-collection mod.traveller.aging --modifier -6` produces a result in `[-5, 6]`.
- [ ] Spec requirements all covered (see spec §Testing).
