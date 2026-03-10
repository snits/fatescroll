# Reroll on Chain Reference — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reroll modifier support to chain references so table authors can express "reroll certain values when following this chain."

**Architecture:** Introduce a `ChainRef` enum (simple string or structured with modifiers) using `serde(untagged)` for backward compatibility. Thread reroll values through the roller's recursive execution. Add safety limit for reroll attempts.

**Tech Stack:** Rust, serde with `#[serde(untagged)]`, diceman (dice library)

**Spec:** `docs/plans/2026-03-10-reroll-on-chain-design.md`

---

## Chunk 1: Data Model and Deserialization

### Task 1: ChainRef enum and deserialization tests

**Files:**
- Modify: `src/models.rs` (add ChainRef enum, change ResultEntry.chain type)

- [ ] **Step 1: Write failing tests for ChainRef deserialization**

Add these tests to `src/models.rs` in the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn deserialize_chain_ref_simple_string() {
    let yaml = r#""animal-type""#;
    let chain_ref: ChainRef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(chain_ref.table_id(), "animal-type");
    assert!(chain_ref.reroll_values().is_empty());
}

#[test]
fn deserialize_chain_ref_with_reroll() {
    let yaml = r#"
table: wizard-mishap
reroll: [1]
"#;
    let chain_ref: ChainRef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(chain_ref.table_id(), "wizard-mishap");
    assert_eq!(chain_ref.reroll_values(), &[1]);
}

#[test]
fn deserialize_chain_ref_modified_no_reroll() {
    let yaml = r#"
table: some-table
"#;
    let chain_ref: ChainRef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(chain_ref.table_id(), "some-table");
    assert!(chain_ref.reroll_values().is_empty());
}

#[test]
fn deserialize_mixed_chain_list() {
    let yaml = r#"
- animal-type
- table: wizard-mishap
  reroll: [1]
"#;
    let chains: Vec<ChainRef> = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(chains.len(), 2);
    assert_eq!(chains[0].table_id(), "animal-type");
    assert!(chains[0].reroll_values().is_empty());
    assert_eq!(chains[1].table_id(), "wizard-mishap");
    assert_eq!(chains[1].reroll_values(), &[1]);
}

#[test]
fn deserialize_simple_table_with_reroll_chain() {
    let yaml = r#"
id: mishap
name: Wizard Mishap
type: simple
tags: []
roll: 1d12
results:
  - min: 1
    max: 1
    text: "Roll twice and combine"
    chain:
      - table: mishap
        reroll: [1]
      - table: mishap
        reroll: [1]
  - min: 2
    max: 12
    text: Other effect
"#;
    let table: Table = serde_yaml::from_str(yaml).unwrap();
    match table {
        Table::Simple { results, .. } => {
            let chains = results[0].chain.as_ref().unwrap();
            assert_eq!(chains.len(), 2);
            assert_eq!(chains[0].table_id(), "mishap");
            assert_eq!(chains[0].reroll_values(), &[1]);
        }
        _ => panic!("Expected Simple table"),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib models::tests`
Expected: FAIL — `ChainRef` type not found

- [ ] **Step 3: Implement ChainRef enum and update ResultEntry**

Add the `ChainRef` enum before the `ResultEntry` struct in `src/models.rs`:

```rust
#[derive(Debug, Deserialize, Clone, PartialEq)]
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

Change `ResultEntry.chain` from `Option<Vec<String>>` to `Option<Vec<ChainRef>>`:

```rust
pub struct ResultEntry {
    pub min: u32,
    pub max: u32,
    pub text: Option<String>,
    pub chain: Option<Vec<ChainRef>>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib models::tests`
Expected: PASS

- [ ] **Step 5a: Update existing model tests**

The existing test `deserialize_simple_table_with_chains` in `src/models.rs` (line ~151) compares `chain` against `&["wolf-count"]` which won't compile with `Vec<ChainRef>`. Update assertions to use `table_id()`:

```rust
// Replace:
//   assert_eq!(results[0].chain.as_ref().unwrap(), &["wolf-count"]);
// With:
assert_eq!(results[0].chain.as_ref().unwrap().len(), 1);
assert_eq!(results[0].chain.as_ref().unwrap()[0].table_id(), "wolf-count");
// Same pattern for bandit-strength/bandit-motivation assertions
```

Also update all `chain: Some(vec!["foo".into()])` occurrences across model tests to `chain: Some(vec![ChainRef::Simple("foo".into())])`.

- [ ] **Step 5b: Update roller.rs**

In `src/roller.rs` line 93-98, update chain iteration to use `table_id()`:
```rust
// Change: roll_recursive(registry, chain_ref, namespace, rng, depth + 1)
// To:     roll_recursive(registry, chain_ref.table_id(), namespace, rng, depth + 1)
```

Also update all test `ResultEntry` constructions from `chain: Some(vec!["foo".into()])` to `chain: Some(vec![ChainRef::Simple("foo".into())])`.

- [ ] **Step 5c: Update validator.rs**

In `src/validator.rs` lines 164-169, update to use `table_id()`:
```rust
// Change: registry.resolve(chain_ref, current_namespace)
// To:     registry.resolve(chain_ref.table_id(), current_namespace)
// Change: reference: chain_ref.clone()
// To:     reference: chain_ref.table_id().to_string()
```

Also update all test `ResultEntry` constructions from `chain: Some(vec!["foo".into()])` to `chain: Some(vec![ChainRef::Simple("foo".into())])`.

- [ ] **Step 5d: Update display.rs (minimal compile fix only)**

In `src/display.rs` lines 48-53, replace `chains.join(", ")` with a minimal compile fix. Do NOT add reroll display logic — that is Task 7's scope.

```rust
let chain_str = match &entry.chain {
    Some(chains) if !chains.is_empty() => {
        let refs: Vec<&str> = chains.iter().map(|c| c.table_id()).collect();
        format!(" → {}", refs.join(", "))
    }
    _ => String::new(),
};
```

Also update all test `ResultEntry` constructions from `chain: Some(vec!["foo".into()])` to `chain: Some(vec![ChainRef::Simple("foo".into())])`.

**Note:** Do NOT modify `src/fixer.rs` here — it operates on raw `serde_yaml::Value`, not typed models, so changing `ResultEntry.chain` causes zero compile errors there. Fixer changes are handled in Task 6.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: ALL PASS (all 92 existing tests continue to pass)

- [ ] **Step 7: Commit**

```bash
git add src/models.rs src/roller.rs src/validator.rs src/display.rs
git commit -s -m "feat: add ChainRef enum for structured chain references

Introduces ChainRef enum with Simple (plain string) and Modified
(table + reroll modifiers) variants using serde(untagged) for
backward compatibility. Changes ResultEntry.chain from Vec<String>
to Vec<ChainRef>. Updates all consumers to use ChainRef::table_id().

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Chunk 2: Roller Reroll Logic

### Task 2: Reroll error variant

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Add RerollExhausted variant to RollError**

In `src/error.rs`, add to the `RollError` enum:

```rust
#[error("reroll attempts exhausted ({attempts}) for table '{table}' with reroll values {reroll_values:?}")]
RerollExhausted {
    table: String,
    attempts: usize,
    reroll_values: Vec<u32>,
},
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compiles (warning about unused variant is fine)

- [ ] **Step 3: Commit**

```bash
git add src/error.rs
git commit -s -m "feat: add RerollExhausted error variant

Co-authored-by: Claude <noreply@anthropic.com>"
```

### Task 3: Reroll logic in roller

**Files:**
- Modify: `src/roller.rs`

- [ ] **Step 1: Write failing test for basic reroll behavior**

Add to `src/roller.rs` tests:

```rust
#[test]
fn roll_with_reroll_avoids_excluded_values() {
    // Table with 1d4, where result 1 chains to self with reroll:[1]
    // Entry 1 has chain to child with reroll, entries 2-4 are plain results
    let mut reg = Registry::new();
    reg.register(
        "ns.reroll-parent".into(),
        Table::Simple {
            id: "reroll-parent".into(),
            name: "Reroll Parent".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 1,
                    text: Some("Chains with reroll".into()),
                    chain: Some(vec![ChainRef::Modified {
                        table: "reroll-target".into(),
                        reroll: vec![1],
                    }]),
                },
                ResultEntry {
                    min: 2,
                    max: 4,
                    text: Some("Normal".into()),
                    chain: None,
                },
            ],
        },
    )
    .unwrap();
    reg.register(
        "ns.reroll-target".into(),
        Table::Simple {
            id: "reroll-target".into(),
            name: "Reroll Target".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 1,
                    text: Some("Should be skipped".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 2,
                    max: 4,
                    text: Some("Valid result".into()),
                    chain: None,
                },
            ],
        },
    )
    .unwrap();

    // Run many times to exercise the reroll path
    for seed in 0..100 {
        let mut rng = diceman::FastRng::with_seed(seed);
        let result = roll_with_rng(&reg, "ns.reroll-parent", &mut rng).unwrap();
        if !result.children.is_empty() {
            // When chain fires, the child should never have roll value 1
            assert_ne!(result.children[0].roll.unwrap(), 1,
                "seed {seed}: reroll should have prevented value 1");
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib roller::tests::roll_with_reroll_avoids_excluded_values`
Expected: FAIL — reroll not implemented, child may have roll value 1

- [ ] **Step 3: Implement reroll logic in roll_recursive**

Modify `roll_recursive` signature to accept reroll values:

```rust
const MAX_REROLL_ATTEMPTS: usize = 100;

fn roll_recursive(
    registry: &Registry,
    table_id: &str,
    current_namespace: &str,
    rng: &mut impl diceman::Rng,
    depth: usize,
    reroll_values: &[u32],
) -> Result<RollResult, RollError> {
```

Update the two public entry points to pass `&[]`:
- `roll()` → `roll_recursive(registry, table_id, "", rng, 0, &[])`
- `roll_with_rng()` → `roll_recursive(registry, table_id, "", rng, 0, &[])`

In the Simple table arm, replace the single dice roll with a reroll loop:

```rust
let (roll_u32, entry) = {
    let mut attempts = 0;
    loop {
        let dice_result = diceman::roll_with_rng(&roll_expr, rng)
            .map_err(|e| RollError::DiceEvaluation {
                table: name.clone(),
                expr: roll_expr.clone(),
                reason: e.to_string(),
            })?;

        let roll_value = dice_result.total;
        if roll_value < 0 {
            return Err(RollError::NegativeRoll { value: roll_value });
        }
        let roll_u32 = roll_value as u32;

        let entry = results
            .iter()
            .find(|e| roll_u32 >= e.min && roll_u32 <= e.max)
            .ok_or_else(|| RollError::RollOutOfRange {
                table: name.clone(),
                value: roll_value,
            })?;

        if reroll_values.contains(&roll_u32) {
            attempts += 1;
            if attempts >= MAX_REROLL_ATTEMPTS {
                return Err(RollError::RerollExhausted {
                    table: name.clone(),
                    attempts,
                    reroll_values: reroll_values.to_vec(),
                });
            }
            continue;
        }

        break (roll_u32, entry.clone());
    }
};
```

Update the chain iteration to pass reroll values:

```rust
if let Some(chains) = &entry.chain {
    for chain_ref in chains {
        let child = roll_recursive(
            registry,
            chain_ref.table_id(),
            namespace,
            rng,
            depth + 1,
            chain_ref.reroll_values(),
        )?;
        children.push(child);
    }
}
```

Also update compound table recursive calls to pass `&[]` for reroll (compound tables don't support reroll modifiers on their sub-table references):

```rust
let child = roll_recursive(registry, table_ref, namespace, rng, depth + 1, &[])?;
```

Wait — compound table refs are still `Vec<String>`, not `Vec<ChainRef>`. That's fine, no change needed there.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib roller::tests::roll_with_reroll_avoids_excluded_values`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src/roller.rs
git commit -s -m "feat: implement reroll logic for chain references

When a chain reference includes reroll values, the roller loops
until it gets a non-excluded value. Safety limit of 100 attempts
with RerollExhausted error for pathological cases.

Co-authored-by: Claude <noreply@anthropic.com>"
```

### Task 4: Reroll exhaustion test

**Files:**
- Modify: `src/roller.rs` (add test)

- [ ] **Step 1: Write test for reroll exhaustion**

```rust
#[test]
fn reroll_exhaustion_returns_error() {
    // Table with 1d4, chain rerolls ALL values 1-4 → should exhaust
    let mut reg = Registry::new();
    reg.register(
        "ns.exhaust-parent".into(),
        Table::Simple {
            id: "exhaust-parent".into(),
            name: "Exhaust Parent".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![ResultEntry {
                min: 1,
                max: 4,
                text: Some("Always chains".into()),
                chain: Some(vec![ChainRef::Modified {
                    table: "exhaust-target".into(),
                    reroll: vec![1, 2, 3, 4],
                }]),
            }],
        },
    )
    .unwrap();
    reg.register(
        "ns.exhaust-target".into(),
        Table::Simple {
            id: "exhaust-target".into(),
            name: "Exhaust Target".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![ResultEntry {
                min: 1,
                max: 4,
                text: Some("Unreachable".into()),
                chain: None,
            }],
        },
    )
    .unwrap();

    let mut rng = diceman::FastRng::with_seed(42);
    let err = roll_with_rng(&reg, "ns.exhaust-parent", &mut rng).unwrap_err();
    assert!(matches!(err, RollError::RerollExhausted { .. }));
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --lib roller::tests::reroll_exhaustion_returns_error`
Expected: PASS (implementation from Task 3 already handles this)

- [ ] **Step 3: Commit**

```bash
git add src/roller.rs
git commit -s -m "test: add reroll exhaustion test

Co-authored-by: Claude <noreply@anthropic.com>"
```

### Task 5: Self-referential reroll test (the Shadowdark pattern)

**Files:**
- Modify: `src/roller.rs` (add test)

- [ ] **Step 1: Write test for self-referential chain with reroll**

```rust
#[test]
fn self_referential_chain_with_reroll() {
    // Shadowdark pattern: entry 1 chains to self twice with reroll:[1]
    let mut reg = Registry::new();
    reg.register(
        "ns.mishap".into(),
        Table::Simple {
            id: "mishap".into(),
            name: "Wizard Mishap".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry {
                    min: 1,
                    max: 1,
                    text: Some("Roll twice and combine".into()),
                    chain: Some(vec![
                        ChainRef::Modified {
                            table: "mishap".into(),
                            reroll: vec![1],
                        },
                        ChainRef::Modified {
                            table: "mishap".into(),
                            reroll: vec![1],
                        },
                    ]),
                },
                ResultEntry {
                    min: 2,
                    max: 2,
                    text: Some("Hands glow blue".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 3,
                    max: 3,
                    text: Some("Lose sense of smell".into()),
                    chain: None,
                },
                ResultEntry {
                    min: 4,
                    max: 4,
                    text: Some("Hair turns white".into()),
                    chain: None,
                },
            ],
        },
    )
    .unwrap();

    // Run many seeds — when entry 1 hits, children must never roll 1
    for seed in 0..200 {
        let mut rng = diceman::FastRng::with_seed(seed);
        let result = roll_with_rng(&reg, "ns.mishap", &mut rng).unwrap();
        if result.roll == Some(1) {
            assert_eq!(result.children.len(), 2);
            for child in &result.children {
                assert_ne!(child.roll.unwrap(), 1,
                    "seed {seed}: self-referential reroll should prevent value 1");
            }
        }
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --lib roller::tests::self_referential_chain_with_reroll`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/roller.rs
git commit -s -m "test: add self-referential reroll test (Shadowdark pattern)

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

## Chunk 3: Fixer, Display, and Integration

### Task 6: Update fixer for structured chain refs

**Files:**
- Modify: `src/fixer.rs`

- [ ] **Step 1: Write failing test for fixer with structured chain ref**

Add a test to `src/fixer.rs` tests that creates a YAML string with structured chain refs and verifies `extract_references` handles them:

```rust
#[test]
fn extract_references_from_structured_chain() {
    let yaml = r#"
id: mishap
name: Wizard Mishap
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 1
    text: Roll twice
    chain:
      - table: mishap
        reroll: [1]
      - plain-ref
  - min: 2
    max: 4
    text: Normal
"#;
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let refs = extract_references(&value);
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&"mishap".to_string()));
    assert!(refs.contains(&"plain-ref".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib fixer::tests::extract_references_from_structured_chain`
Expected: FAIL — only finds "plain-ref", not "mishap" (structured form not handled)

- [ ] **Step 3: Update extract_references to handle mappings**

In `extract_references`, update the chain iteration (lines 70-73) to also handle mapping entries:

```rust
for chain in chains {
    if let Some(s) = chain.as_str() {
        refs.push(s.to_string());
    } else if let Some(mapping) = chain.as_mapping() {
        let table_key = serde_yaml::Value::String("table".into());
        if let Some(serde_yaml::Value::String(s)) = mapping.get(&table_key) {
            refs.push(s.clone());
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib fixer::tests::extract_references_from_structured_chain`
Expected: PASS

- [ ] **Step 5: Write failing test for update_references with structured chain ref**

```rust
#[test]
fn update_references_in_structured_chain() {
    let yaml = r#"
id: mishap
name: Wizard Mishap
type: simple
tags: []
roll: 1d4
results:
  - min: 1
    max: 1
    text: Roll twice
    chain:
      - table: old-name
        reroll: [1]
      - plain-old-ref
  - min: 2
    max: 4
    text: Normal
"#;
    let mut value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let mut corrections = std::collections::HashMap::new();
    corrections.insert("old-name".to_string(), "new-name".to_string());
    corrections.insert("plain-old-ref".to_string(), "plain-new-ref".to_string());

    let updated = update_references(&mut value, &corrections);
    assert_eq!(updated.len(), 2);
    assert!(updated.contains(&("old-name".to_string(), "new-name".to_string())));
    assert!(updated.contains(&("plain-old-ref".to_string(), "plain-new-ref".to_string())));

    // Verify the YAML was actually modified
    let refs = extract_references(&value);
    assert!(refs.contains(&"new-name".to_string()));
    assert!(refs.contains(&"plain-new-ref".to_string()));
}
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test --lib fixer::tests::update_references_in_structured_chain`
Expected: FAIL — structured chain ref not updated (only plain-old-ref is updated)

- [ ] **Step 7: Update update_references for structured chain refs**

In `update_references`, update the chain iteration (lines 111-117) to also handle mapping entries:

```rust
for chain in chains {
    if let Some(old) = chain.as_str()
        && let Some(new_id) = corrections.get(old)
    {
        let old_str = old.to_string();
        *chain = serde_yaml::Value::String(new_id.clone());
        updated.push((old_str, new_id.clone()));
    } else if let Some(mapping) = chain.as_mapping_mut() {
        let table_key = serde_yaml::Value::String("table".into());
        if let Some(serde_yaml::Value::String(old)) = mapping.get(&table_key)
            && let Some(new_id) = corrections.get(old.as_str()).cloned()
        {
            let old_str = old.clone();
            mapping.insert(table_key, serde_yaml::Value::String(new_id.clone()));
            updated.push((old_str, new_id));
        }
    }
}
```

- [ ] **Step 8: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 9: Commit**

```bash
git add src/fixer.rs
git commit -s -m "feat: support structured chain refs in stale reference fixer

extract_references and update_references now handle both plain
string chains and structured {table:, reroll:} chain references.

Co-authored-by: Claude <noreply@anthropic.com>"
```

### Task 7: Update display for structured chain refs

**Files:**
- Modify: `src/display.rs`

- [ ] **Step 1: Write failing test for display with reroll chain**

Add to `src/display.rs` tests:

```rust
#[test]
fn format_table_with_reroll_chain() {
    let table = Table::Simple {
        id: "mishap".into(),
        name: "Wizard Mishap".into(),
        tags: vec![],
        roll: "1d4".into(),
        results: vec![
            ResultEntry {
                min: 1,
                max: 1,
                text: Some("Roll twice and combine".into()),
                chain: Some(vec![
                    ChainRef::Modified {
                        table: "mishap".into(),
                        reroll: vec![1],
                    },
                    ChainRef::Modified {
                        table: "mishap".into(),
                        reroll: vec![1],
                    },
                ]),
            },
            ResultEntry {
                min: 2,
                max: 4,
                text: Some("Normal".into()),
                chain: None,
            },
        ],
    };
    let output = format_table("ns.mishap", &table);
    // Should show chain arrows with reroll annotation
    assert!(output.contains("mishap"));
    assert!(output.contains("reroll"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib display::tests::format_table_with_reroll_chain`
Expected: FAIL — current display doesn't show reroll info

- [ ] **Step 3: Implement Display for ChainRef and update display.rs**

Add a `Display` impl to `ChainRef` in `src/models.rs`:

```rust
impl std::fmt::Display for ChainRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainRef::Simple(id) => write!(f, "{id}"),
            ChainRef::Modified { table, reroll } if reroll.is_empty() => {
                write!(f, "{table}")
            }
            ChainRef::Modified { table, reroll } => {
                write!(f, "{table} (reroll {:?})", reroll)
            }
        }
    }
}
```

In `src/display.rs`, the chain rendering at lines 48-53 currently calls `chains.join(", ")`. Since `ChainRef` now implements `Display`, update this to use the iterator:

```rust
let chain_str = match &entry.chain {
    Some(chains) if !chains.is_empty() => {
        let refs: Vec<String> = chains.iter().map(|c| c.to_string()).collect();
        format!(" → {}", refs.join(", "))
    }
    _ => String::new(),
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib display::tests::format_table_with_reroll_chain`
Expected: PASS

- [ ] **Step 5: Run full test suite including integration tests**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src/models.rs src/display.rs
git commit -s -m "feat: display reroll modifiers in table show output

ChainRef implements Display, showing reroll annotation when present.

Co-authored-by: Claude <noreply@anthropic.com>"
```

### Task 8: Integration test with YAML fixture

**Files:**
- Create: `tests/fixtures/valid-collection/encounters/mishap.yaml`
- Modify: `tests/cli_integration.rs`

- [ ] **Step 1: Create a mishap table YAML fixture**

Create `tests/fixtures/valid-collection/encounters/mishap.yaml`:

```yaml
id: mishap
name: Wizard Mishap
type: simple
tags:
  - wizard
  - mishap
roll: 1d4
results:
  - min: 1
    max: 1
    text: "Roll twice and combine"
    chain:
      - table: mishap
        reroll: [1]
      - table: mishap
        reroll: [1]
  - min: 2
    max: 2
    text: "Hands glow blue for {1d6} minutes"
  - min: 3
    max: 3
    text: "Lose sense of smell for {1d6} days"
  - min: 4
    max: 4
    text: "Hair turns white"
```

- [ ] **Step 2: Run validate to confirm fixture loads correctly**

Run: `cargo run -- validate tests/fixtures/valid-collection`
Expected: "Collection is valid." (or exit 0, no errors)

- [ ] **Step 3: Write integration test for roll with reroll**

Add to `tests/cli_integration.rs`:

```rust
#[test]
fn roll_mishap_table_with_reroll_chain() {
    // Test that the mishap table with structured chain refs loads and rolls
    let output = fatescroll_bin()
        .args(["roll", "--collection", "tests/fixtures/valid-collection",
               "test.encounters.mishap"])
        .output()
        .unwrap();
    assert!(output.status.success(),
        "roll failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wizard Mishap"));
}
```

- [ ] **Step 4: Write integration test for show with reroll chain**

```rust
#[test]
fn show_mishap_table_displays_reroll() {
    let output = fatescroll_bin()
        .args(["show", "--collection", "tests/fixtures/valid-collection",
               "test.encounters.mishap"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wizard Mishap"));
    assert!(stdout.contains("reroll"));
}
```

- [ ] **Step 5: Run integration tests**

Run: `cargo test --test cli_integration`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures/valid-collection/encounters/mishap.yaml tests/cli_integration.rs
git commit -s -m "test: add integration tests for reroll chain references

Adds mishap table fixture with structured chain refs and integration
tests for roll and show commands.

Co-authored-by: Claude <noreply@anthropic.com>"
```

### Task 9: Run clippy and final verification

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 2: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 3: Fix any issues found**

- [ ] **Step 4: Final commit if needed**
