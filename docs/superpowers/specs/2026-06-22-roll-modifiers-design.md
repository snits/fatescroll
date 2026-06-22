# Roll Modifier Support for Tables — Design

**Bead:** fatescroll-0ax
**Date:** 2026-06-22
**Status:** Approved

## Summary

Add optional roll modifiers to tables. A table may declare a `modifier_range`
(e.g. Shadowdark carousing: `1d8` with a spending bonus of `[0, 6]` across
entries 1–14). The `roll` subcommand accepts `--modifier N`; the looked-up value
is `clamp(dice_roll + modifier, entry_min, entry_max)`. Supporting negative
modifiers (e.g. Traveller aging: `1d6` minus terms completed) requires entry
values to allow negatives, so `ResultEntry.min/max` migrate from `u32` to `i32`.

## Scope

In scope (this bead):

- **A — Negative entry values:** `ResultEntry.min/max` become `i32`.
- **B — Roll modifiers:** table-level `modifier_range`, runtime `--modifier`,
  clamping, and the validation changes that support them.

Explicitly out of scope (split into follow-up beads, both depend on this one):

- **`--value N` direct lookup** → fatescroll-tv5
- **`notes` field + `--notes` on `show`** → fatescroll-6ff
- **`init` scaffolding of `modifier_range`** — deferred (YAGNI; authors write
  `modifier_range` by hand). The only `init` change here is an import move.

## Locked decisions

1. **Validation = strict expand.** When a table declares `modifier_range
   [mod_min, mod_max]`, its entries must *exactly* cover the envelope
   `[dice_min + mod_min, dice_max + mod_max]` — contiguous, no gaps, no
   overlaps, and no entry outside the envelope. Same strictness as today,
   applied over the modifier-expanded range.
2. **`--modifier` requires a declaration, allows overflow.** `--modifier` only
   works on tables that declare `modifier_range` (else error). The runtime value
   *may* exceed the declared range; the result then clamps to the entry bounds
   (matches real play where a bonus can be unusually high).
3. **Compound + `--modifier` = error.** `modifier_range` is a Simple-table
   concept; passing `--modifier` to a compound table is a clear error.
4. **Modifier is top-level only.** `--modifier` applies to the named table's roll
   only — never to chained sub-tables or compound sub-tables.

## Worked examples (envelope math)

- Shadowdark carousing: `1d8`, `modifier_range [0, 6]` → envelope `[1, 14]`,
  entries 1–14. Contiguous and fully reachable. (Clamp is a no-op here.)
- Traveller aging: `1d6`, `modifier_range [-6, 0]` → envelope `[-5, 6]`,
  entries −5…6. Negative entries; clamp meaningful when terms exceed 6.

## Approach

**Dual-path range, unified coverage.** Keep `simulate_seeded` for non-modifier
tables' dice-range computation (no regression for users' richer expressions —
exploding/keep/drop — even though no current fixture uses them). Use the
analytical `dice_range()` only for the modifier envelope, where determinism,
negatives, and envelope arithmetic are required. The gap/overlap/out-of-range
**coverage check is a single `i32` routine**; only the envelope *source* branches.

Rejected: replacing `simulate_seeded` with `dice_range()` everywhere — silently
drops exploding/keep/drop support, larger behavior change, no benefit here.

## Component changes

### 1. Data model — `models.rs`

- `ResultEntry.min/max`: `u32 → i32`.
- New `ModifierRange { min: i32, max: i32 }` (derives `Serialize`,
  `Deserialize`), deserialized from YAML `modifier_range: [min, max]`.
- `Table::Simple` gains `#[serde(default)] modifier_range: Option<ModifierRange>`.
  Absent → `None`, so every existing table stays valid (backward compatible).
- `Serialize` retained on all boundary types (Tauri consumers).

### 2. Range computation — `dice.rs`

- Move `dice_range()` from `init.rs` to `dice.rs` (pure move, no behavior
  change; both validator and init call it). `init.rs` imports it from `dice`.
- `dice_range()` stays non-negative: dice expressions themselves never go
  negative. Negativity enters only via `modifier_range`.

### 3. Validation — `validator.rs`

One `i32` coverage routine over `[envelope_min, envelope_max]`:
`idx = value − envelope_min`, `len = envelope_max − envelope_min + 1`. Reports
`RangeGap` / `RangeOverlap` / `EntryOutOfRange` (the latter reports the
**envelope** for modifier tables, not the raw dice range).

Envelope source per Simple table:

- **Digit-dice (D66, …):** `modifier_range` must be `None`, else error
  (`ModifierUnsupportedForDigitDice`). Existing non-contiguous digit coverage.
- **Contiguous *with* `modifier_range`:** require an analytical dice expression
  (`dice_range()`; a complex expr → its existing `UnsupportedDiceExpression`).
  Validate `mod_min ≤ mod_max` (else `ModifierRangeReversed`). Envelope =
  `[dice_min + mod_min, dice_max + mod_max]`.
- **Contiguous *without* `modifier_range`:** unchanged — `simulate_seeded`,
  reject negative dice range, envelope = `[dice_min, dice_max]`.

### 4. Roller — `roller.rs`

- Public entry point gains a modifier: `roll(registry, table_id, modifier:
  Option<i32>)`. The modifier reaches the top-level table only; recursion into
  chains/compound sub-tables passes `None`.
- Simple-table evaluation order (critical):
  1. Evaluate raw dice total (`i64`).
  2. **Guard `raw < 0` → `NegativeRoll`** on the *raw* total (unchanged — the
     dice expression's own range is validated ≥ 0).
  3. `lookup = raw + modifier.unwrap_or(0)` as `i32`.
  4. `clamp(lookup, entry_min, entry_max)` where the bounds are the min/max over
     the table's entries.
  5. Find the entry containing the clamped value.
- A validated modifier table can never produce `RollOutOfRange` (clamp +
  no-gaps guarantee a hit) — asserted by test.
- `--modifier` on a table without `modifier_range`, or on a compound table →
  `RollError::ModifierNotSupported { table }`.
- Modifier and chain-reroll are orthogonal: reroll is chain-level (children),
  modifier is top-level only; they never interact.

### 5. Display — `display.rs`

- Show `modifier_range` when present (e.g. `Modifier: -6 to 0`).
- Fix range alignment for negative entries: `digit_count` / `ilog10` panics on
  `≤ 0`. Replace with width measured from the formatted range strings
  (sign-aware).

### 6. CLI — `main.rs`

- `roll` subcommand gains `--modifier <i32>` (accepts negatives). Wired through
  `cmd_roll` → `roller::roll(registry, table_id, modifier)`.

### 7. Errors — `error.rs`

- Coverage error fields (`RangeReversed`, `EntryOutOfRange`, `RangeGap`,
  `RangeOverlap`) → `i32`.
- Add `ValidationError::ModifierRangeReversed { table, min, max }`.
- Add `ValidationError::ModifierUnsupportedForDigitDice { table, expr }`.
- Add `RollError::ModifierNotSupported { table }`.
- `RollError::RollOutOfRange.value` stays `i64`.

## Testing (TDD throughout)

Unit and integration tests, each decision backed by at least one test:

- **models:** `modifier_range` deserializes from `[min, max]`; absent → `None`
  (backward compat); negative `i32` entries deserialize.
- **dice:** `dice_range()` tests move with the function; behavior unchanged.
- **validator:**
  - Strict-expand happy paths: Shadowdark (`1d8` + `[0,6]`, entries 1–14);
    Traveller (`1d6` + `[-6,0]`, entries −5…6).
  - Gap, overlap, and beyond-envelope (`EntryOutOfRange`) all error.
  - `ModifierRangeReversed` when `mod_min > mod_max`.
  - `ModifierUnsupportedForDigitDice` for `modifier_range` on D66.
  - `modifier_range` on a complex/non-analytical expr → unsupported error.
- **roller:**
  - Clamp high (runtime modifier overflows top) and clamp low (underflows).
  - Negative-entry lookup returns the correct entry.
  - Raw-`NegativeRoll` guard still fires for a genuinely negative dice expr.
  - `ModifierNotSupported`: modifier on a no-declaration table; on a compound.
  - Modifier does not leak into chained children.
  - Validated modifier table never yields `RollOutOfRange` (seed sweep).
- **display:** negative-entry range alignment; `modifier_range` line shown.
- **CLI integration:** `roll --modifier` against the Shadowdark fixture (in-range
  and overflow/clamp); error cases (no declaration, compound).

New fixtures under `tests/fixtures/`:

- `shadowdark-carousing` — `1d8`, `modifier_range [0, 6]`, entries 1–14.
- `traveller-aging` — `1d6`, `modifier_range [-6, 0]`, negative entries.

## Risks / notes

- The `i32` migration is wide but mechanical: models, validator coverage arrays,
  error fields, display width. The compiler surfaces every site.
- `digit_dice_values` returns `Vec<u32>`; the digit-dice path stays `u32`
  internally and only the unified coverage routine works in `i32` — reconcile at
  that boundary (digit-dice tables forbid `modifier_range`, so no negative
  values ever reach digit coverage).
