# fatescroll-wrf — Bound dice-range allocation & i64→i32 truncation (non-modifier paths)

## Problem

Two latent, defense-in-depth gaps surfaced during the fatescroll-0ax code review.
The 0ax modifier work already hardened the *modifier* path; this bead extends the
same defenses to the *non-modifier* paths.

1. **Unbounded coverage allocation (validation).** In `validate_table`, the
   non-modifier branch derives `(sim.min, sim.max)` from
   `diceman::simulate_seeded` and passes it to `validate_envelope_coverage`,
   which allocates `vec![0u32; max - min + 1]` with no width cap. A table such as
   `roll: "1d1000000000"` with a single entry covering `1..1000000000` passes the
   per-entry range check and then allocates ~4 GB. The modifier branch already
   guards against this with `MAX_ENVELOPE_WIDTH` (100_000) plus i32-bounds checks.

2. **Silent i64→i32 truncation (roller).** In `roll_recursive`,
   `let roll_i32 = roll_value as i32;` truncates silently when
   `dice_result.total > i32::MAX`. Only a `< 0` guard (`NegativeRoll`) exists; no
   upper bound.

Both are unreachable for real RPG dice — no plausible table authors a contiguous
outcome envelope near i32::MAX, and producing such a total would require
simulating absurd dice. The fix is a pure backstop, matching the protection the
modifier path already has.

## Design

Each guard lives behind a **pure helper** that takes raw `i64` inputs, so the
TDD red phase can exercise the boundary directly instead of coercing diceman into
an overflow (which would either be seed-fragile or hang for billions of rolls).

### Part A — Validation: cap the non-modifier coverage envelope

- Extract a pure helper in `validator.rs`:
  ```rust
  fn bounded_envelope(min: i64, max: i64) -> Result<(i32, i32), i64>
  ```
  Returns `Ok((min as i32, max as i32))` when `max - min <= MAX_ENVELOPE_WIDTH`
  and both endpoints fall within i32; otherwise `Err(width)`. Unit-tested with
  raw i64 inputs.
- The **non-modifier** branch calls `bounded_envelope` after the existing
  `sim.min/sim.max < 0` check. On `Err(width)` it returns a **new additive**
  variant `ValidationError::DiceRangeTooWide { table, width, max: MAX_ENVELOPE_WIDTH }`.
- The **modifier** branch reuses `bounded_envelope` for its width/i32 check but
  maps `Err` to its existing `ModifierRangeTooWide` — so its message and tests are
  unchanged.

`DiceRangeTooWide` is **additive** (not a rename of `ModifierRangeTooWide`):
`ValidationError` is public and consumed by external crates (e.g. hexwalker), and
`ModifierRangeTooWide`'s message names `modifier_range`, which would lie for a
plain dice table.

### Part B — Roller: guard the i64→i32 cast

- Extract a pure helper in `roller.rs`:
  ```rust
  fn checked_total_to_i32(total: i64) -> Option<i32>  // None when outside i32 range
  ```
  Unit-tested with `i32::MAX as i64 + 1`, `i64::MAX`, and in-range values.
- After the existing `NegativeRoll` guard, replace `let roll_i32 = roll_value as i32;`
  with a `checked_total_to_i32(roll_value)` call; map `None` to the existing
  `RollError::RollOutOfRange { table, value: roll_value }` (its `value` field is
  already `i64`, and a roll beyond i32 matches no entry — the semantics fit).
- The `roll_i32 as u32` reroll cast needs **no change**: the `NegativeRoll` guard
  guarantees `roll_i32 >= 0`, so the cast is lossless. The bead lumped it in, but
  it is a non-issue once the i64→i32 guard lands.

## Testing (TDD, red first)

- `bounded_envelope`: accepts a normal width; rejects `width > MAX_ENVELOPE_WIDTH`;
  rejects an endpoint beyond i32.
- Validator integration: a table whose dice envelope exceeds the cap fails
  validation with `DiceRangeTooWide` — fast, because the guard fires before the
  `vec![0u32; len]` allocation.
- `checked_total_to_i32`: `Some` for in-range, `None` for `i32::MAX as i64 + 1`
  and `i64::MAX`.
- Existing modifier tests (`ModifierRangeTooWide`) remain green.

## Non-goals

- The digit-dice path (`validate_digit_dice_coverage`) keys coverage by a
  `HashMap` of valid values, not a contiguous vec — no unbounded allocation.
  Untouched.
- No behavior change for any real table; widths there are tiny. This is a pure
  backstop.
