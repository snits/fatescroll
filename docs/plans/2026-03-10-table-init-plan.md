# Table Template Generator (`fatescroll init`) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `fatescroll init` subcommand that generates table YAML skeletons from dice expressions or entry count specifications.

**Architecture:** New `init` module containing the template generation logic (dice range calculation, bell curve matching, YAML output formatting). CLI subcommand in `main.rs` dispatches to this module. Three modes: explicit (--roll), flat distribution (--entries + --distribution flat), bell curve (--entries + --distribution bell). Output to stdout by default, optional --output flag for file.

**Tech Stack:** Rust, clap (derive), diceman (simulation for range detection), serde_yaml (output formatting or manual YAML generation)

---

## Task 1: Create init module with range calculation

**Files:**
- Create: `src/init.rs`
- Modify: `src/lib.rs` (add module)

- [ ] **Step 1: Write failing tests for dice range calculation**

Create `src/init.rs` with tests:

```rust
// ABOUTME: Generates table YAML skeletons from dice expressions or entry counts.
// ABOUTME: Supports explicit, flat, and bell curve distribution modes.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_range_1d6() {
        let (min, max) = dice_range("1d6").unwrap();
        assert_eq!(min, 1);
        assert_eq!(max, 6);
    }

    #[test]
    fn dice_range_2d6() {
        let (min, max) = dice_range("2d6").unwrap();
        assert_eq!(min, 2);
        assert_eq!(max, 12);
    }

    #[test]
    fn dice_range_1d8_plus_1() {
        let (min, max) = dice_range("1d8+1").unwrap();
        assert_eq!(min, 2);
        assert_eq!(max, 9);
    }

    #[test]
    fn dice_range_invalid_expression() {
        assert!(dice_range("1z6").is_err());
    }
}
```

- [ ] **Step 2: Run tests to confirm compilation fails**

Run: `cargo test -p fatescroll dice_range`
Expected: compilation error — module and function don't exist

- [ ] **Step 3: Implement dice_range function**

Add to `src/init.rs` (above the tests module):

```rust
use crate::error::Error;

/// Determine the min and max values of a dice expression via simulation.
/// Uses diceman::simulate_seeded which returns SimResult with min: i64, max: i64.
/// Same approach used by the validator (validator.rs:71).
pub fn dice_range(expr: &str) -> Result<(u32, u32), Error> {
    diceman::parse(expr)?;
    let sim = diceman::simulate_seeded(expr, 100_000, 42)?;
    if sim.min < 0 || sim.max < 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("dice expression '{expr}' produces negative values"),
        )
        .into());
    }
    Ok((sim.min as u32, sim.max as u32))
}
```

Add `pub mod init;` to `src/lib.rs`.

- [ ] **Step 4: Run tests to confirm they pass**

Run: `cargo test -p fatescroll dice_range -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/init.rs src/lib.rs
git commit -s -m "feat: add init module with dice range calculation"
```

---

## Task 2: Template generation from explicit dice expression

**Files:**
- Modify: `src/init.rs`

- [ ] **Step 1: Write failing test for template generation**

Add to `src/init.rs` tests:

```rust
#[test]
fn generate_template_1d6() {
    let output = generate_template("1d6", "Test Table").unwrap();
    assert!(output.contains("name: Test Table"));
    assert!(output.contains("type: simple"));
    assert!(output.contains("roll: 1d6"));
    assert!(output.contains("min: 1"));
    assert!(output.contains("max: 6"));
    // Should have 6 entries
    let min_count = output.matches("  - min:").count();
    assert_eq!(min_count, 6);
}

#[test]
fn generate_template_2d6() {
    let output = generate_template("2d6", "Bell Table").unwrap();
    assert!(output.contains("roll: 2d6"));
    assert!(output.contains("min: 2"));
    assert!(output.contains("max: 12"));
    let min_count = output.matches("  - min:").count();
    assert_eq!(min_count, 11); // 2-12 = 11 values
}
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p fatescroll generate_template -- --nocapture`
Expected: compilation error — function doesn't exist

- [ ] **Step 3: Implement generate_template function**

Add to `src/init.rs`:

```rust
/// Generate a table YAML skeleton from a dice expression.
/// Each possible value gets its own result entry with empty text.
pub fn generate_template(roll_expr: &str, name: &str) -> Result<String, Error> {
    let (min, max) = dice_range(roll_expr)?;
    let mut output = String::new();
    output.push_str(&format!("name: {name}\n"));
    output.push_str("type: simple\n");
    output.push_str("tags: []\n");
    output.push_str(&format!("roll: {roll_expr}\n"));
    output.push_str("results:\n");
    for value in min..=max {
        output.push_str(&format!("  - min: {value}\n"));
        output.push_str(&format!("    max: {value}\n"));
        output.push_str("    text: \"\"\n");
    }
    Ok(output)
}
```

- [ ] **Step 4: Run tests to confirm they pass**

Run: `cargo test -p fatescroll generate_template -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/init.rs
git commit -s -m "feat: generate table YAML skeleton from dice expression"
```

---

## Task 3: Bell curve matching logic

**Files:**
- Modify: `src/init.rs`

- [ ] **Step 1: Write failing tests for bell curve calculation**

Add to `src/init.rs` tests:

```rust
#[test]
fn flat_distribution_8_entries() {
    let result = calculate_distribution(8, Distribution::Flat);
    assert_eq!(result, DistributionResult::Exact("1d8".to_string()));
}

#[test]
fn bell_exact_match_2d6() {
    // 11 entries: 2d6 gives range 2-12 = 11 distinct values
    let result = calculate_distribution(11, Distribution::Bell);
    assert_eq!(result, DistributionResult::Exact("2d6".to_string()));
}

#[test]
fn bell_exact_match_3d6() {
    // 16 entries: 3d6 gives range 3-18 = 16 distinct values
    let result = calculate_distribution(16, Distribution::Bell);
    assert_eq!(result, DistributionResult::Exact("3d6".to_string()));
}

#[test]
fn bell_no_exact_match_shows_suggestions() {
    // 12 entries: no exact match at X=2 (2d6=11, 2d7=13)
    // 3d5=13, 3d4=10 — no exact match at X=3 either
    let result = calculate_distribution(12, Distribution::Bell);
    match result {
        DistributionResult::Suggestions(suggestions) => {
            assert!(!suggestions.is_empty());
            // Should include 2d6 and 2d7 at minimum
            assert!(suggestions.iter().any(|s| s.expression == "2d6"));
            assert!(suggestions.iter().any(|s| s.expression == "2d7"));
        }
        other => panic!("Expected Suggestions, got {:?}", other),
    }
}

#[test]
fn bell_minimum_entries() {
    // 3 entries: 2d2 gives range 2-4 = 3 distinct values
    let result = calculate_distribution(3, Distribution::Bell);
    assert_eq!(result, DistributionResult::Exact("2d2".to_string()));
}
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p fatescroll bell_ flat_ -- --nocapture`
Expected: compilation error — types don't exist

- [ ] **Step 3: Implement distribution types and calculation**

Add to `src/init.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Distribution {
    Flat,
    Bell,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub expression: String,
    pub num_dice: u32,
    pub entries: u32,
    pub range_min: u32,
    pub range_max: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DistributionResult {
    Exact(String),
    Suggestions(Vec<Suggestion>),
}

/// Calculate a dice expression for a given entry count and distribution.
///
/// For flat distribution: returns 1dN.
/// For bell distribution: tries XdY for X=2,3 where distinct values = X*(Y-1)+1.
/// Returns exact match if found, otherwise nearest suggestions.
pub fn calculate_distribution(entries: u32, dist: Distribution) -> DistributionResult {
    match dist {
        Distribution::Flat => {
            DistributionResult::Exact(format!("1d{entries}"))
        }
        Distribution::Bell => {
            let mut suggestions = Vec::new();

            for num_dice in 2..=3u32 {
                // Distinct values = num_dice * (sides - 1) + 1
                // Solving: sides = (entries - 1) / num_dice + 1
                let numerator = entries - 1;
                if numerator % num_dice == 0 {
                    let sides = numerator / num_dice + 1;
                    if sides >= 2 {
                        return DistributionResult::Exact(
                            format!("{num_dice}d{sides}")
                        );
                    }
                }

                // No exact match for this X — find floor and ceiling
                let y_floor = numerator / num_dice + 1;
                let y_ceil = y_floor + 1;

                if y_floor >= 2 {
                    let floor_entries = num_dice * (y_floor - 1) + 1;
                    suggestions.push(Suggestion {
                        expression: format!("{num_dice}d{y_floor}"),
                        num_dice,
                        entries: floor_entries,
                        range_min: num_dice,
                        range_max: num_dice * y_floor,
                    });
                }

                let ceil_entries = num_dice * (y_ceil - 1) + 1;
                suggestions.push(Suggestion {
                    expression: format!("{num_dice}d{y_ceil}"),
                    num_dice,
                    entries: ceil_entries,
                    range_min: num_dice,
                    range_max: num_dice * y_ceil,
                });
            }

            DistributionResult::Suggestions(suggestions)
        }
    }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

Run: `cargo test -p fatescroll bell_ flat_ -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/init.rs
git commit -s -m "feat: add bell curve and flat distribution calculation"
```

---

## Task 4: Add `init` CLI subcommand with integration tests

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_integration.rs`

Note: This task combines the CLI subcommand implementation and integration tests
into a single task to maintain TDD flow. The integration tests are written first,
then the implementation makes them pass.

- [ ] **Step 1: Add Init variant to Commands enum**

Add to the `Commands` enum in `src/main.rs`:

```rust
/// Generate a table YAML template
Init {
    /// Dice expression (e.g., "1d6", "2d8+1")
    #[arg(long, conflicts_with = "entries")]
    roll: Option<String>,
    /// Number of result entries desired
    #[arg(long, conflicts_with = "roll", requires = "distribution")]
    entries: Option<u32>,
    /// Distribution type: flat or bell
    #[arg(long, requires = "entries")]
    distribution: Option<String>,
    /// Table display name
    #[arg(long, default_value = "Untitled Table")]
    name: String,
    /// Write output to file instead of stdout
    #[arg(long)]
    output: Option<PathBuf>,
},
```

- [ ] **Step 2: Add match arm and handler function**

Add match arm in `main()`:

```rust
Commands::Init {
    roll,
    entries,
    distribution,
    name,
    output,
} => cmd_init(roll, entries, distribution, &name, output),
```

Add handler function:

```rust
fn cmd_init(
    roll: Option<String>,
    entries: Option<u32>,
    distribution: Option<String>,
    name: &str,
    output: Option<PathBuf>,
) -> Result<(), fatescroll::Error> {
    let roll_expr = if let Some(expr) = roll {
        expr
    } else if let Some(count) = entries {
        let dist = match distribution.as_deref() {
            Some("flat") => fatescroll::init::Distribution::Flat,
            Some("bell") => fatescroll::init::Distribution::Bell,
            Some(other) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown distribution type: '{other}' (expected 'flat' or 'bell')"),
                )
                .into());
            }
            None => unreachable!("clap requires --distribution with --entries"),
        };

        if count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "entries must be at least 1",
            )
            .into());
        }

        if matches!(dist, fatescroll::init::Distribution::Bell) && count < 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bell curves require at least 3 entries (minimum is 2d2)",
            )
            .into());
        }

        match fatescroll::init::calculate_distribution(count, dist) {
            fatescroll::init::DistributionResult::Exact(expr) => expr,
            fatescroll::init::DistributionResult::Suggestions(suggestions) => {
                eprintln!("No exact match for {count} entries with bell curve.");
                let mut current_dice = 0;
                for s in &suggestions {
                    if s.num_dice != current_dice {
                        current_dice = s.num_dice;
                        eprintln!("  With {} dice:", s.num_dice);
                    }
                    eprintln!(
                        "    {} → {} entries (range {}-{})",
                        s.expression, s.entries, s.range_min, s.range_max
                    );
                }
                eprintln!("Use --roll <expression> to generate.");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "no exact bell curve match",
                )
                .into());
            }
        }
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "specify --roll <expression> or --entries <count> --distribution <type>",
        )
        .into());
    };

    let template = fatescroll::init::generate_template(&roll_expr, name)?;

    if let Some(path) = output {
        if path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("output file already exists: {}", path.display()),
            )
            .into());
        }
        std::fs::write(&path, &template)?;
        eprintln!("Wrote template to {}", path.display());
    } else {
        print!("{template}");
    }

    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles successfully

- [ ] **Step 4: Write integration tests**

Add to `tests/cli_integration.rs`:

```rust
#[test]
fn init_explicit_1d6() {
    let output = fatescroll_bin()
        .args(["init", "--roll", "1d6", "--name", "Test Table"])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name: Test Table"));
    assert!(stdout.contains("roll: 1d6"));
    assert!(stdout.contains("type: simple"));
    assert_eq!(stdout.matches("  - min:").count(), 6);
}

#[test]
fn init_flat_distribution() {
    let output = fatescroll_bin()
        .args([
            "init", "--entries", "8", "--distribution", "flat",
            "--name", "Flat Table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roll: 1d8"));
    assert_eq!(stdout.matches("  - min:").count(), 8);
}

#[test]
fn init_bell_exact_match() {
    let output = fatescroll_bin()
        .args([
            "init", "--entries", "11", "--distribution", "bell",
            "--name", "Bell Table",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roll: 2d6"));
    assert_eq!(stdout.matches("  - min:").count(), 11);
}

#[test]
fn init_bell_no_exact_match_shows_suggestions() {
    let output = fatescroll_bin()
        .args([
            "init", "--entries", "12", "--distribution", "bell",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("2d6"));
    assert!(stderr.contains("2d7"));
    assert!(stderr.contains("With 2 dice:"));
    assert!(stderr.contains("With 3 dice:"));
}

#[test]
fn init_bell_too_few_entries() {
    let output = fatescroll_bin()
        .args([
            "init", "--entries", "2", "--distribution", "bell",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("at least 3 entries"));
}

#[test]
fn init_output_to_file() {
    let dir = TempDir::new().unwrap();
    let output_file = dir.path().join("test-table.yaml");
    let output = fatescroll_bin()
        .args([
            "init", "--roll", "1d4", "--name", "File Table",
            "--output", &output_file.to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let contents = std::fs::read_to_string(&output_file).unwrap();
    assert!(contents.contains("roll: 1d4"));
    assert_eq!(contents.matches("  - min:").count(), 4);
}

#[test]
fn init_output_refuses_overwrite() {
    let dir = TempDir::new().unwrap();
    let output_file = dir.path().join("existing.yaml");
    std::fs::write(&output_file, "existing content").unwrap();
    let output = fatescroll_bin()
        .args([
            "init", "--roll", "1d4",
            "--output", &output_file.to_string_lossy(),
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn init_invalid_dice_expression() {
    let output = fatescroll_bin()
        .args(["init", "--roll", "1z6"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}

#[test]
fn init_zero_entries() {
    let output = fatescroll_bin()
        .args([
            "init", "--entries", "0", "--distribution", "flat",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("at least 1"));
}

#[test]
fn init_requires_roll_or_entries() {
    let output = fatescroll_bin()
        .args(["init", "--name", "No Dice"])
        .output()
        .expect("failed to run fatescroll");
    assert!(!output.status.success());
}
```

- [ ] **Step 5: Run integration tests**

Run: `cargo test init_ -- --nocapture`
Expected: all pass

- [ ] **Step 6: Run full test suite and clippy**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass, clippy clean

- [ ] **Step 7: Commit**

```bash
git add src/main.rs tests/cli_integration.rs
git commit -s -m "feat: add init subcommand for table template generation"
```
