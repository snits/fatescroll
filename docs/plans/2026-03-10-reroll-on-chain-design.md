# Reroll on Chain Reference — Design

## Problem

Some TTRPG tables have entries that chain back to the same table but require certain roll values to be rerolled on subsequent rolls. The canonical example is Shadowdark's wizard mishap table:

> 1: Roll twice on this table, combining results. Reroll further 1s.

fatescroll can already express the self-referential chain (chain to the same table twice), but has no mechanism to constrain what values are acceptable on the chained roll. Without reroll support, chained rolls that land on the same self-referencing entry cause exponential branching until the chain depth limit is hit.

## Decision: Targeted Feature, Not Expression Language

We considered whether this should be part of a broader result expression system (fatescroll-h4r). Analysis showed:

- The concrete TTRPG mechanics that the current schema can't handle are small, discrete features (reroll, repeat-N, chain modifier) — not a grammar/language problem
- An expression parser would roughly double the codebase for speculative benefit
- The reroll feature doesn't conflict with or block any future expression system
- fatescroll-h4r stays at P4 in the backlog

## Design: Reroll Modifier on Chain References

### Where Reroll Lives

Reroll belongs on the **chain reference** (caller-side), not on the result entry (callee-side). Reasons:

1. The result entry describes what you rolled. Reroll constrains the *chained* roll, not the current one.
2. In the Shadowdark example, rolling a 1 IS a valid result with an effect. Entry-level `reroll: true` would prevent reaching that result on a direct roll, breaking the mechanic.
3. Different chain references from the same entry could have different reroll conditions.
4. The constraint is contextual — the target table works normally when rolled directly.

### YAML Schema

Chain references become a tagged union: either a plain string (backward compatible) or a structured object with modifiers.

**Before (still works):**
```yaml
chain:
  - animal-type
  - bandit-strength
```

**With reroll modifier:**
```yaml
chain:
  - table: wizard-mishap
    reroll: [1]
  - table: wizard-mishap
    reroll: [1]
```

**Mixed (plain and structured in same list):**
```yaml
chain:
  - animal-type
  - table: wizard-mishap
    reroll: [1]
```

### Data Model

```rust
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ChainRef {
    Simple(String),
    Modified {
        table: String,
        #[serde(default)]
        reroll: Vec<u32>,
    },
}

impl ChainRef {
    pub fn table_id(&self) -> &str {
        match self {
            ChainRef::Simple(id) => id,
            ChainRef::Modified { table, .. } => table,
        }
    }

    pub fn reroll_values(&self) -> &[u32] {
        match self {
            ChainRef::Simple(_) => &[],
            ChainRef::Modified { reroll, .. } => reroll,
        }
    }
}
```

`ResultEntry.chain` changes from `Option<Vec<String>>` to `Option<Vec<ChainRef>>`.

### Roller Behavior

`roll_recursive()` gains a `reroll_values: &[u32]` parameter. When non-empty, the dice roll loops until it produces a value not in the reroll set:

```
loop {
    roll dice
    if value not in reroll set → break with value
    if attempts >= MAX_REROLL_ATTEMPTS → return error
}
```

The reroll values do NOT propagate through further chains. Each chain reference independently specifies its own reroll set. In the Shadowdark example, each of the two chain refs to `wizard-mishap` specifies `reroll: [1]` independently.

Safety: `MAX_REROLL_ATTEMPTS = 100` constant with a `RerollExhausted` error variant for pathological cases (e.g., reroll set covers all possible dice outcomes).

### Change Surface

| File | Change |
|------|--------|
| `models.rs` | Add `ChainRef` enum, change `ResultEntry.chain` type |
| `roller.rs` | Add `reroll_values` param to `roll_recursive`, add reroll loop, extract modifiers from `ChainRef` in chain iteration |
| `validator.rs` | Use `ChainRef::table_id()` for reference validation |
| `display.rs` | Update chain rendering for `ChainRef` variants |
| `error.rs` | Add `RerollExhausted` variant |

Estimated: ~60-80 lines of production code, plus tests.

### Validation

- Existing chain validation uses `ChainRef::table_id()` — no new cross-reference logic needed
- Optional future enhancement: warn if reroll values cover all possible outcomes of the target table's dice expression (catches authoring errors at validate time rather than runtime)

### Test Plan

1. **Backward compatibility**: Plain string chains still deserialize and roll correctly
2. **Reroll triggers**: Structured chain ref with reroll values causes rerolling
3. **Self-referential with reroll**: Table that chains to itself with reroll (the Shadowdark pattern)
4. **Reroll exhaustion**: Error when reroll set covers all possible values
5. **Mixed chains**: List with both plain strings and structured refs
6. **Display**: Chain arrows show correctly for both ref types
7. **Validation**: Structured chain refs validate their table references

### Example: Shadowdark Wizard Mishap

```yaml
name: Wizard Mishap
type: simple
tags:
  - shadowdark
  - wizard
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
    text: "You lose your sense of smell for 1d6 days"
  - min: 3
    max: 12
    text: "Other mishap effects..."
```

Rolling this table: if you roll a 1, fatescroll chains to the same table twice. Each chained roll rerolls if it lands on 1, preventing recursive explosion while still allowing the full range of other results.
