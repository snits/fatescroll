# D66 Table Support Design

## Overview

Add support for Traveller-style D66 (digit-dice) tables. diceman now parses `D66`
as `Expr::DigitRoll { sides: 6, count: 2 }` and produces non-contiguous results
(11-16, 21-26, ..., 61-66 = 36 valid values). fatescroll needs to handle these
in init scaffolding, validation, and rolling.

## Key Insight

D66 values are non-contiguous: values like 17-20, 27-30 are impossible. The existing
validator assumes contiguous coverage (every value from dice_min to dice_max must have
an entry). For D66 tables, coverage checking must only consider the 36 valid values.

## Design

### New helper: `digit_dice_values(sides, count) -> Vec<u32>`

Computes all valid digit-dice outcomes. For D66 (sides=6, count=2): generates all
combinations where each digit is 1-6, concatenated as decimal digits.

Lives in `init.rs` as a pub function (used by both init and validator).

### Changes to `init.rs`

- `dice_range()`: Add `Expr::DigitRoll` arm, return (min, max) of valid values
- `generate_template()`: Detect digit-dice via `diceman::parse()`. For digit-dice,
  iterate `digit_dice_values()` instead of `min..=max`

### Changes to `validator.rs`

- Parse the roll expression to check for `Expr::DigitRoll`
- If digit-dice: use `digit_dice_values()` for coverage checking
- If standard: use `simulate_seeded()` as before (no behavior change)

### No changes needed

- `models.rs`: ResultEntry min/max u32 handles D66 values (11, 66 etc.)
- `roller.rs`: diceman::roll("D66") returns valid values, entry lookup works as-is
- `display.rs`: Displays min/max/text, works as-is
- `error.rs`: Existing error types suffice

## Test Plan

1. `digit_dice_values` correctness: D66 (36 values), D44 (16 values), D666 (216 values)
2. `dice_range` for D66: returns (11, 66)
3. `generate_template` for D66: produces 36 entries with correct values
4. Validator: valid D66 table passes, gaps detected, out-of-range entries caught
5. Integration: roll on D66 fixture table works end-to-end
