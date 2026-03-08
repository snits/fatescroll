# Show Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `fatescroll show <table_id>` to display formatted table contents.

**Architecture:** New `display.rs` module with a `format_table` function. CLI wiring in `main.rs`. Pure formatting — no side effects.

**Tech Stack:** Rust, clap 4 (derive), std::fmt::Write

---

### Task 1: Add format_table function with tests

**Files:**
- Create: `src/display.rs`
- Modify: `src/lib.rs`

**Step 1: Create src/display.rs with ABOUTME and failing test**

Create `src/display.rs`:

```rust
// ABOUTME: Formats table data for human-readable display output.
// ABOUTME: Renders simple tables as range/text grids and compound tables as sub-table lists.

use crate::models::Table;

/// Format a table for display. Returns the formatted string.
pub fn format_table(fqid: &str, table: &Table) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ResultEntry;

    fn simple_table() -> Table {
        Table::Simple {
            id: "wilderness-encounter".into(),
            name: "Wilderness Encounter".into(),
            tags: vec!["encounter".into(), "wilderness".into()],
            roll: "1d8".into(),
            results: vec![
                ResultEntry { min: 1, max: 3, text: Some("Animal encounter".into()), chain: Some(vec!["animal-type".into()]) },
                ResultEntry { min: 4, max: 5, text: Some("Bandit camp".into()), chain: Some(vec!["bandit-strength".into(), "bandit-motivation".into()]) },
                ResultEntry { min: 6, max: 7, text: Some("Abandoned campsite".into()), chain: None },
                ResultEntry { min: 8, max: 8, text: Some("Merchant".into()), chain: Some(vec!["merchant-goods".into()]) },
            ],
        }
    }

    fn compound_table() -> Table {
        Table::Compound {
            id: "quick-npc".into(),
            name: "Quick NPC Generator".into(),
            tags: vec!["npc".into(), "generator".into()],
            tables: vec!["npc-occupation".into(), "npc-disposition".into(), "npc-quirk".into()],
        }
    }

    #[test]
    fn format_simple_table_output() {
        let table = simple_table();
        let output = format_table("test.encounters.wilderness-encounter", &table);

        // Header
        assert!(output.contains("Wilderness Encounter (test.encounters.wilderness-encounter)"));
        // Tags
        assert!(output.contains("Tags: encounter, wilderness"));
        // Roll
        assert!(output.contains("Roll: 1d8"));
        // Range collapse: 8-8 should display as just 8
        assert!(output.contains("  8  ") || output.contains("  8 "));
        assert!(!output.contains("8-8"));
        // Chain references with arrow
        assert!(output.contains("→ animal-type"));
        assert!(output.contains("→ bandit-strength, bandit-motivation"));
        // Regular range
        assert!(output.contains("1-3"));
    }

    #[test]
    fn format_compound_table_output() {
        let table = compound_table();
        let output = format_table("test.npc.quick-npc", &table);

        assert!(output.contains("Quick NPC Generator (test.npc.quick-npc)"));
        assert!(output.contains("Tags: npc, generator"));
        assert!(output.contains("Tables:"));
        assert!(output.contains("  - npc-occupation"));
        assert!(output.contains("  - npc-disposition"));
        assert!(output.contains("  - npc-quirk"));
    }

    #[test]
    fn format_table_no_tags() {
        let table = Table::Simple {
            id: "minimal".into(),
            name: "Minimal".into(),
            tags: vec![],
            roll: "1d4".into(),
            results: vec![
                ResultEntry { min: 1, max: 4, text: Some("Something".into()), chain: None },
            ],
        };
        let output = format_table("test.minimal", &table);

        assert!(output.contains("Minimal (test.minimal)"));
        assert!(!output.contains("Tags:"));
    }
}
```

Add `pub mod display;` to `src/lib.rs` after the existing module declarations.

**Step 2: Run tests to verify they fail**

Run: `cargo test display::tests`
Expected: FAIL — `todo!()` panics

**Step 3: Implement format_table**

Replace `todo!()` with:

```rust
pub fn format_table(fqid: &str, table: &Table) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    match table {
        Table::Simple { name, tags, roll, results, .. } => {
            writeln!(out, "{name} ({fqid})").unwrap();
            if !tags.is_empty() {
                writeln!(out, "Tags: {}", tags.join(", ")).unwrap();
            }
            writeln!(out, "Roll: {roll}").unwrap();
            writeln!(out).unwrap();

            // Calculate range column width for alignment
            let range_width = results.iter().map(|r| {
                if r.min == r.max {
                    digit_count(r.min)
                } else {
                    digit_count(r.min) + 1 + digit_count(r.max) // "min-max"
                }
            }).max().unwrap_or(1);

            for entry in results {
                let range_str = if entry.min == entry.max {
                    format!("{}", entry.min)
                } else {
                    format!("{}-{}", entry.min, entry.max)
                };

                let text = entry.text.as_deref().unwrap_or("");
                let chain_str = match &entry.chain {
                    Some(chains) if !chains.is_empty() => {
                        format!(" → {}", chains.join(", "))
                    }
                    _ => String::new(),
                };

                writeln!(out, "  {:>width$}  {text}{chain_str}", range_str, width = range_width).unwrap();
            }
        }
        Table::Compound { name, tags, tables, .. } => {
            writeln!(out, "{name} ({fqid})").unwrap();
            if !tags.is_empty() {
                writeln!(out, "Tags: {}", tags.join(", ")).unwrap();
            }
            writeln!(out, "Tables:").unwrap();
            for t in tables {
                writeln!(out, "  - {t}").unwrap();
            }
        }
    }

    out
}

fn digit_count(n: u32) -> usize {
    if n == 0 { return 1; }
    (n as f64).log10().floor() as usize + 1
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test display::tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/display.rs src/lib.rs
git commit -s -m "feat: add format_table function for displaying table contents

Formats simple tables as range/text grids with chain arrow notation.
Collapses min==max ranges to single values. Formats compound tables
as sub-table lists.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add show subcommand to CLI

**Files:**
- Modify: `src/main.rs`

**Step 1: Add Show variant to Commands enum**

After the Search variant, add:

```rust
    /// Display a table's contents
    Show {
        /// Path to collection directory
        #[arg(long)]
        collection: Option<PathBuf>,
        /// Fully qualified table ID (e.g., "dmg.treasure.gems")
        table_id: String,
    },
```

**Step 2: Add match arm in main()**

After the Search match arm:

```rust
        Commands::Show {
            collection,
            table_id,
        } => resolve_collection(collection).and_then(|collection| cmd_show(&collection, &table_id)),
```

**Step 3: Add cmd_show function**

After `cmd_roll`:

```rust
fn cmd_show(collection: &Path, table_id: &str) -> Result<(), fatescroll::Error> {
    let registry = fatescroll::load_collection(collection)?;
    let (fqid, table) = registry.resolve(table_id)?;
    print!("{}", fatescroll::display::format_table(fqid, table));
    Ok(())
}
```

**Step 4: Run all tests and clippy**

Run: `cargo test && cargo clippy -- -D warnings`

**Step 5: Commit**

```bash
git add src/main.rs
git commit -s -m "feat: add show subcommand to display table contents

fatescroll show <table_id> displays a formatted view of a table
without rolling on it. Uses the format_table function from display.rs.

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 3: Add integration tests

**Files:**
- Modify: `tests/cli_integration.rs`

**Step 1: Add integration test for simple table**

```rust
#[test]
fn show_displays_simple_table() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.wilderness-encounter",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wilderness Encounter"));
    assert!(stdout.contains("Roll: 1d8"));
    assert!(stdout.contains("→ animal-type"));
}
```

**Step 2: Add integration test for compound table**

```rust
#[test]
fn show_displays_compound_table() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.npc.quick-npc",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Quick NPC Generator"));
    assert!(stdout.contains("Tables:"));
    assert!(stdout.contains("npc-occupation"));
}
```

**Step 3: Add integration test for nonexistent table**

```rust
#[test]
fn show_nonexistent_table_fails() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "nonexistent.table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}
```

**Step 4: Run all tests and clippy**

Run: `cargo test && cargo clippy -- -D warnings`

**Step 5: Commit**

```bash
git add tests/cli_integration.rs
git commit -s -m "test: add integration tests for show subcommand

Co-authored-by: Claude <noreply@anthropic.com>"
```

---

### Task 4: Final verification

**Step 1: Run full test suite**

Run: `cargo test`

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`

**Step 3: Manual smoke test**

Run against the fixture collection:
```bash
cargo run -- show --collection tests/fixtures/valid-collection test.encounters.wilderness-encounter
cargo run -- show --collection tests/fixtures/valid-collection test.npc.quick-npc
```

Verify output looks correct.
