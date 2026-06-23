# Bound dice-range allocation & i64→i32 truncation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the modifier path's allocation/overflow defenses to the non-modifier validation and roller paths, behind pure, directly-testable helpers.

**Architecture:** Two independent guards. (A) A pure `bounded_envelope(min, max) -> Result<(i32,i32), i64>` in `validator.rs` caps the coverage-vec width and i32 range for both the modifier branch (mapped to the existing `ModifierRangeTooWide`) and the non-modifier branch (mapped to a new additive `DiceRangeTooWide`). (B) A pure `checked_total_to_i32(total) -> Option<i32>` in `roller.rs` guards the `i64 → i32` cast; `None` maps to the existing `RollOutOfRange`.

**Tech Stack:** Rust, `diceman` v0.2.0 (`SimResult.min/max: i64`, `RollResult.total: i64`), `thiserror`. Run tests with `cargo test -p fatescroll-core`.

---

## File Structure

- Modify `fatescroll-core/src/error.rs` — add `ValidationError::DiceRangeTooWide`.
- Modify `fatescroll-core/src/validator.rs` — add `bounded_envelope` helper, rewire modifier + non-modifier branches, add unit tests.
- Modify `fatescroll-core/src/roller.rs` — add `checked_total_to_i32` helper, rewire the cast, add unit tests.

No new files. All changes stay within `fatescroll-core`.

---

### Task 1: `DiceRangeTooWide` error variant

**Files:**
- Modify: `fatescroll-core/src/error.rs` (after `ModifierRangeTooWide`, ~line 103)

- [ ] **Step 1: Add the variant**

In `enum ValidationError`, immediately after the `ModifierRangeTooWide { .. }` variant, add:

```rust
    #[error(
        "dice expression produces an outcome envelope too wide to validate (width {width}, max {max}) in table '{table}'"
    )]
    DiceRangeTooWide { table: String, width: i64, max: i64 },
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p fatescroll-core`
Expected: builds clean (variant is unused for now — that is fine, it is wired in Task 2).

- [ ] **Step 3: Commit**

```bash
git add fatescroll-core/src/error.rs
git commit -s -m "feat: add DiceRangeTooWide validation error" -m "Assisted-by: Claude:claude-opus-4-8"
```

---

### Task 2: `bounded_envelope` helper + cap both validation branches

**Files:**
- Modify: `fatescroll-core/src/validator.rs` (helper near top after the `MAX_ENVELOPE_WIDTH` const; rewire `validate_table`; tests in the `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write failing unit tests for the helper**

Add to the `#[cfg(test)] mod tests` block in `validator.rs`:

```rust
#[test]
fn bounded_envelope_accepts_normal_width() {
    assert_eq!(bounded_envelope(1, 6), Ok((1, 6)));
    // exact cap boundary: width == MAX_ENVELOPE_WIDTH is accepted (guard is `>`)
    assert_eq!(bounded_envelope(0, MAX_ENVELOPE_WIDTH), Ok((0, 100_000)));
}

#[test]
fn bounded_envelope_rejects_too_wide() {
    // width = MAX_ENVELOPE_WIDTH + 1
    let err = bounded_envelope(0, MAX_ENVELOPE_WIDTH + 1).unwrap_err();
    assert_eq!(err, MAX_ENVELOPE_WIDTH + 1);
}

#[test]
fn bounded_envelope_rejects_endpoint_beyond_i32() {
    // width is small, but max exceeds i32::MAX
    let big = i32::MAX as i64 + 1;
    assert!(bounded_envelope(big - 5, big).is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p fatescroll-core --lib bounded_envelope`
Expected: FAIL — `cannot find function bounded_envelope`.

- [ ] **Step 3: Add the helper**

Immediately after the `MAX_ENVELOPE_WIDTH` const (~line 17) in `validator.rs`:

```rust
/// Narrow an i64 outcome envelope `[min, max]` to i32, rejecting envelopes too
/// wide to allocate a coverage vec for, or whose endpoints fall outside i32.
/// On rejection returns the offending `width` (`Err`) for the caller's error.
/// Callers pass non-overflowing i64 endpoints (i32-derived or non-negative),
/// so `max - min` cannot overflow.
fn bounded_envelope(min: i64, max: i64) -> Result<(i32, i32), i64> {
    let width = max - min;
    if width > MAX_ENVELOPE_WIDTH || min < i32::MIN as i64 || max > i32::MAX as i64 {
        return Err(width);
    }
    Ok((min as i32, max as i32))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p fatescroll-core --lib bounded_envelope`
Expected: PASS (3 tests).

- [ ] **Step 5: Rewire the modifier branch to use the helper**

In `validate_table`, replace the modifier-branch width check (currently lines ~102-117: the `let env_min = ...; let env_max = ...; let width = ...; if width > MAX_ENVELOPE_WIDTH ... { return Err(ModifierRangeTooWide ...) } (env_min as i32, env_max as i32)`) with:

```rust
                    let env_min = d_min as i64 + mr.min as i64;
                    let env_max = d_max as i64 + mr.max as i64;
                    match bounded_envelope(env_min, env_max) {
                        Ok(pair) => pair,
                        Err(width) => {
                            return Err(ValidationError::ModifierRangeTooWide {
                                table: name.clone(),
                                width,
                                max: MAX_ENVELOPE_WIDTH,
                            });
                        }
                    }
```

- [ ] **Step 6: Write the failing validator-level test for the non-modifier cap (RED FIRST)**

Write this test BEFORE rewiring the non-modifier branch, so it observes a genuine
red. Add to `#[cfg(test)] mod tests` in `validator.rs`. Match the exact
`Table::Simple` field set used by existing tests like `entry_above_dice_max`
(`id`, `name`, `tags`, `roll`, `modifier_range`, `results`):

```rust
#[test]
fn non_modifier_dice_range_too_wide_is_rejected() {
    // 1dN with N well past MAX_ENVELOPE_WIDTH: a single entry covers the whole
    // span. Over 100_000 seeded samples of 1d400000 the observed span is ~400k,
    // comfortably above the 100k cap, so the width guard must reject it.
    let sides = MAX_ENVELOPE_WIDTH * 4; // 400_000
    let table = Table::Simple {
        id: "wide".into(),
        name: "Wide".into(),
        tags: vec![],
        roll: format!("1d{sides}"),
        modifier_range: None,
        results: vec![ResultEntry {
            min: 1,
            max: sides as i32,
            text: Some("x".into()),
            chain: None,
        }],
    };
    let err = validate_table(&table).unwrap_err();
    assert!(
        matches!(err, ValidationError::DiceRangeTooWide { .. }),
        "expected DiceRangeTooWide, got: {err:?}"
    );
}
```

- [ ] **Step 7: Run it to verify it fails (genuine red)**

Run: `cargo test -p fatescroll-core --lib non_modifier_dice_range_too_wide`
Expected: FAIL. Before the non-modifier rewire, the observed envelope is
`(~4, ~399999)`; the entry `min: 1` is below `envelope_min`, so
`validate_envelope_coverage` returns `EntryOutOfRange` *before* it allocates the
coverage vec. The assert sees `EntryOutOfRange`, not `DiceRangeTooWide`, and
fails — a fast, safe red (no 4 GB allocation).

- [ ] **Step 8: Rewire the non-modifier branch to use the helper (GREEN)**

In the same `match modifier_range`, replace the `None` branch tail `(sim.min as i32, sim.max as i32)` (currently line ~137) with:

```rust
                    match bounded_envelope(sim.min, sim.max) {
                        Ok(pair) => pair,
                        Err(width) => {
                            return Err(ValidationError::DiceRangeTooWide {
                                table: name.clone(),
                                width,
                                max: MAX_ENVELOPE_WIDTH,
                            });
                        }
                    }
```

Leave the preceding `if sim.min < 0 || sim.max < 0 { ... }` check intact.

- [ ] **Step 9: Run it to verify it passes, then the full core suite + clippy/fmt**

Run: `cargo test -p fatescroll-core --lib non_modifier_dice_range_too_wide` → PASS
(now `bounded_envelope` rejects the wide envelope and `DiceRangeTooWide` fires
before allocation).
Then: `cargo test -p fatescroll-core && cargo clippy -p fatescroll-core -- -D warnings && cargo fmt --check`
Expected: all green. Existing `ModifierRangeTooWide` tests still pass (message/variant unchanged).

- [ ] **Step 10: Commit**

```bash
git add fatescroll-core/src/validator.rs
git commit -s -m "feat: cap non-modifier coverage envelope width" -m "Assisted-by: Claude:claude-opus-4-8"
```

---

### Task 3: `checked_total_to_i32` helper + guard the roller cast

**Files:**
- Modify: `fatescroll-core/src/roller.rs` (helper near other free functions; rewire the cast at ~line 111; tests in the `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write failing unit tests for the helper**

Add to the `#[cfg(test)] mod tests` block in `roller.rs`:

```rust
#[test]
fn checked_total_to_i32_in_range() {
    assert_eq!(checked_total_to_i32(0), Some(0));
    assert_eq!(checked_total_to_i32(i32::MAX as i64), Some(i32::MAX));
}

#[test]
fn checked_total_to_i32_out_of_range() {
    assert_eq!(checked_total_to_i32(i32::MAX as i64 + 1), None);
    assert_eq!(checked_total_to_i32(i64::MAX), None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p fatescroll-core --lib checked_total_to_i32`
Expected: FAIL — `cannot find function checked_total_to_i32`.

- [ ] **Step 3: Add the helper**

Place it with the other free functions in `roller.rs` (e.g. near `interpolate_dice`):

```rust
/// Narrow an i64 dice total to i32, returning `None` when it falls outside the
/// i32 range. Entries and lookups are i32, so an out-of-range total matches no
/// entry and is reported as `RollOutOfRange` by the caller.
fn checked_total_to_i32(total: i64) -> Option<i32> {
    i32::try_from(total).ok()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p fatescroll-core --lib checked_total_to_i32`
Expected: PASS (2 tests).

- [ ] **Step 5: Rewire the cast in `roll_recursive`**

Replace `let roll_i32 = roll_value as i32;` (currently line ~111, immediately after the `NegativeRoll` guard) with:

```rust
                    let roll_i32 = match checked_total_to_i32(roll_value) {
                        Some(v) => v,
                        None => {
                            return Err(RollError::RollOutOfRange {
                                table: name.clone(),
                                value: roll_value,
                            });
                        }
                    };
```

Leave the `roll_i32 as u32` reroll cast (~line 143) unchanged — the `NegativeRoll` guard already guarantees `roll_i32 >= 0`.

- [ ] **Step 6: Run the full core suite + clippy/fmt**

Run: `cargo test -p fatescroll-core && cargo clippy -p fatescroll-core -- -D warnings && cargo fmt --check`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add fatescroll-core/src/roller.rs
git commit -s -m "feat: guard i64 dice total against i32 truncation in roller" -m "Assisted-by: Claude:claude-opus-4-8"
```

---

### Task 4: Full-workspace verification

- [ ] **Step 1: Run the whole suite + lint + format across the workspace**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: all tests pass, no clippy warnings, formatting clean.

- [ ] **Step 2: No commit** — verification only. If anything fails, return to the relevant task.

---

## Notes for the implementer

- `MAX_ENVELOPE_WIDTH` is a `const i64 = 100_000` already defined at the top of `validator.rs`.
- `bounded_envelope` returns `Err(width: i64)` (not a full error) so each caller attaches its own table name and the right error variant — keeps the helper pure and reused by both branches.
- The reroll cast `roll_i32 as u32` is intentionally untouched; it is already lossless given the `NegativeRoll` guard. Do not "fix" it.
- Do not rename `ModifierRangeTooWide`; it is part of the public `ValidationError` consumed by external crates. `DiceRangeTooWide` is additive.
