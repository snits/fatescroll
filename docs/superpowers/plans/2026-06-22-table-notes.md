# Table Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `notes` list to tables and a `fatescroll show --notes` flag that displays them, leaving default `show`, the roller, and search untouched.

**Architecture:** Add a `notes: Vec<String>` field (serde-default, mirroring `tags`) to both `Table::Simple` and `Table::Compound` plus a `notes()` accessor. `display::format_table` gains a `show_notes: bool` param that renders a `Notes:` block between the header and the results. The CLI `Show` subcommand gains a `--notes` flag threaded through `cmd_show`.

**Tech Stack:** Rust, serde / serde_yaml, clap v4 (derive), diceman.

Spec: `docs/superpowers/specs/2026-06-22-table-notes-design.md`

---

## Context engineers will need

- `Table` is a `#[serde(tag = "type")]` enum with struct variants `Simple` and `Compound` (`fatescroll-core/src/models.rs:78`). Accessors `id()`, `name()`, `tags()` already span both variants (`models.rs:102-120`) — `notes()` follows the same shape.
- Adding a field to a struct-variant enum **breaks every struct-literal construction** of that variant (pattern matches that use `..` are unaffected). Constructions exist in these files and must each gain `notes: vec![]` to compile:
  `display.rs`, `roller.rs`, `validator.rs`, `registry.rs`, `search.rs`, and the `models.rs` tests. Let `cargo build`/`cargo test` enumerate the exact sites — fix each missing-field error by adding `notes: vec![]`. This is expected, tedious-but-correct mechanical work, not a sign anything is wrong.
- Nothing re-serializes a `Table` struct to disk (`fixer.rs` edits a `serde_yaml::Value` mapping; `init.rs` builds YAML from a string template), so `#[serde(default)]` with no `skip_serializing_if` — exactly matching `tags` — is correct and will not write `notes: []` into table files.
- `format_table` has exactly one production caller: `cmd_show` in `fatescroll-cli/src/main.rs:328-331`.
- Roller test convention: `let mut rng = diceman::FastRng::with_seed(42);` then `roll_with_rng(&reg, "ns.id", &mut rng)` (`roller.rs:320`).
- Integration test convention: `fatescroll_bin().args([...]).output()`, fixtures via `fixtures_path("valid-collection")` (`cli_integration.rs:336`).

---

## Task 1: Data model — `notes` field + accessor

**Files:**
- Modify: `fatescroll-core/src/models.rs` (add field to both `Table` variants ~`:80-100`, add `notes()` accessor ~`:115-120`, add tests)
- Modify (compile fixes only): `fatescroll-core/src/display.rs`, `fatescroll-core/src/roller.rs`, `fatescroll-core/src/validator.rs`, `fatescroll-core/src/registry.rs`, `fatescroll-core/src/search.rs` — add `notes: vec![]` to each `Table::Simple { .. }` / `Table::Compound { .. }` construction the compiler flags.

- [ ] **Step 1: Write the failing tests** (append to `models.rs` `mod tests`)

```rust
#[test]
fn deserialize_simple_table_with_notes() {
    let yaml = r#"
id: boarding
name: Boarding
type: simple
roll: 2d6
notes:
  - "Attacker rolls 2d6 minus defender 2d6"
  - "DMs: +2 boarding equipment, -1 per 1000 tons difference"
results:
  - min: 1
    max: 12
    text: Outcome
"#;
    let table: Table = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        table.notes(),
        &[
            "Attacker rolls 2d6 minus defender 2d6".to_string(),
            "DMs: +2 boarding equipment, -1 per 1000 tons difference".to_string(),
        ]
    );
}

#[test]
fn notes_absent_defaults_to_empty() {
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
    assert!(table.notes().is_empty());
}

#[test]
fn compound_table_carries_notes() {
    let yaml = r#"
id: quick-npc
name: Quick NPC
type: compound
notes:
  - "Combine occupation and disposition into one line"
tables:
  - npc-occupation
  - npc-disposition
"#;
    let table: Table = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        table.notes(),
        &["Combine occupation and disposition into one line".to_string()]
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p fatescroll-core --lib 2>&1 | tail -20`
Expected: compilation error — no method `notes` on `Table` / missing field `notes`.

- [ ] **Step 3: Add the field to both variants**

In `models.rs`, add to `Table::Simple` (alongside `tags`) and to `Table::Compound` (alongside `tags`):

```rust
        #[serde(default)]
        notes: Vec<String>,
```

- [ ] **Step 4: Add the `notes()` accessor**

In `impl Table`, next to `tags()`:

```rust
    pub fn notes(&self) -> &[String] {
        match self {
            Table::Simple { notes, .. } | Table::Compound { notes, .. } => notes,
        }
    }
```

- [ ] **Step 5: Fix every construction site so the crate compiles**

Run `cargo build -p fatescroll-core 2>&1 | grep -E "missing field|-->"` and add `notes: vec![]` to each flagged `Table::Simple { .. }` / `Table::Compound { .. }` literal. (Place it next to `tags: vec![...]` for readability.) Repeat until the build is clean. Pattern-match arms using `..` need no change.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p fatescroll-core --lib 2>&1 | tail -15`
Expected: PASS, including the three new tests, with no other test regressions.

- [ ] **Step 7: Commit**

```bash
git add fatescroll-core/src
git commit -s -m "feat: add optional notes field to tables (fatescroll-6ff)

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 2: Display — `show_notes` param + `Notes:` block

**Files:**
- Modify: `fatescroll-core/src/display.rs` (signature + rendering + update existing test call sites)
- Modify: `fatescroll-cli/src/main.rs:328-331` (`cmd_show` call site → pass `false` for now)
- Test: `fatescroll-core/src/display.rs` `mod tests`

- [ ] **Step 1: Write the failing tests** (append to `display.rs` `mod tests`)

```rust
#[test]
fn format_table_shows_notes_when_requested() {
    let table = Table::Simple {
        id: "boarding".into(),
        name: "Boarding".into(),
        tags: vec![],
        roll: "2d6".into(),
        modifier_range: None,
        notes: vec![
            "Attacker rolls 2d6 minus defender 2d6".into(),
            "DMs: +2 boarding equipment".into(),
        ],
        results: vec![ResultEntry {
            min: 1,
            max: 12,
            text: Some("Outcome".into()),
            chain: None,
        }],
    };
    let output = format_table("ns.boarding", &table, true);
    assert!(output.contains("Notes:"));
    assert!(output.contains("- Attacker rolls 2d6 minus defender 2d6"));
    assert!(output.contains("- DMs: +2 boarding equipment"));
    // Notes block precedes the results grid.
    assert!(output.find("Notes:").unwrap() < output.find("Outcome").unwrap());
}

#[test]
fn format_table_hides_notes_by_default() {
    let table = Table::Simple {
        id: "boarding".into(),
        name: "Boarding".into(),
        tags: vec![],
        roll: "2d6".into(),
        modifier_range: None,
        notes: vec!["Attacker rolls 2d6 minus defender 2d6".into()],
        results: vec![ResultEntry {
            min: 1,
            max: 12,
            text: Some("Outcome".into()),
            chain: None,
        }],
    };
    let output = format_table("ns.boarding", &table, false);
    assert!(!output.contains("Notes:"));
    assert!(!output.contains("Attacker rolls"));
}

#[test]
fn format_table_without_notes_omits_block_even_when_requested() {
    let table = Table::Simple {
        id: "plain".into(),
        name: "Plain".into(),
        tags: vec![],
        roll: "1d6".into(),
        modifier_range: None,
        notes: vec![],
        results: vec![ResultEntry {
            min: 1,
            max: 6,
            text: Some("X".into()),
            chain: None,
        }],
    };
    let output = format_table("ns.plain", &table, true);
    assert!(!output.contains("Notes:"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p fatescroll-core --lib format_table 2>&1 | tail -20`
Expected: compilation error — `format_table` takes 2 arguments but 3 were supplied / existing literals missing `notes` (if not already added in Task 1 they are; this task's new literals include `notes`).

- [ ] **Step 3: Change the signature and render the block**

In `display.rs`, change the function signature:

```rust
pub fn format_table(fqid: &str, table: &Table, show_notes: bool) -> String {
```

Add a helper invoked from both match arms (or inline in each). For the `Simple` arm, after the `Modifier:` line and before the trailing `writeln!(out).unwrap();` blank line + results, insert:

```rust
            render_notes(&mut out, show_notes, table.notes());
```

For the `Compound` arm, after the `Tags:` line and before `Tables:`, insert the same call. Add the helper at module scope:

```rust
/// Append a `Notes:` block when notes should be shown and exist.
fn render_notes(out: &mut String, show_notes: bool, notes: &[String]) {
    if show_notes && !notes.is_empty() {
        writeln!(out, "Notes:").unwrap();
        for note in notes {
            writeln!(out, "  - {note}").unwrap();
        }
    }
}
```

Note: in the `Simple` arm the match destructures fields; call `table.notes()` is not available inside the match (it borrows `table` which is already destructured). Instead bind `notes` in the `Simple`/`Compound` destructure (add `notes,` to the field list) and pass `notes` directly: `render_notes(&mut out, show_notes, notes);`.

- [ ] **Step 4: Update existing `format_table` call sites to compile**

In `display.rs` tests, update the existing calls `format_table(fqid, &table)` → `format_table(fqid, &table, false)`.
In `fatescroll-cli/src/main.rs` `cmd_show`, update the call to `format_table(table_id, table, false)` (the `--notes` flag is wired in Task 3).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p fatescroll-core --lib 2>&1 | tail -15 && cargo build 2>&1 | tail -5`
Expected: all display tests pass; whole workspace builds.

- [ ] **Step 6: Commit**

```bash
git add fatescroll-core/src/display.rs fatescroll-cli/src/main.rs
git commit -s -m "feat: render notes block in format_table behind show_notes flag (fatescroll-6ff)

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 3: CLI `--notes` flag + fixture + integration tests

**Files:**
- Modify: `fatescroll-cli/src/main.rs` (`Show` subcommand variant ~`:62-69`, its match arm ~`:209-212`, `cmd_show` signature ~`:320`)
- Modify: `tests/fixtures/valid-collection/encounters/wilderness-encounter.yaml` (add `notes`)
- Test: `fatescroll-cli/tests/cli_integration.rs`

- [ ] **Step 1: Add notes to the fixture**

Edit `tests/fixtures/valid-collection/encounters/wilderness-encounter.yaml`, adding a top-level `notes` key (e.g. after `roll:`):

```yaml
notes:
  - "Roll once per watch; apply terrain modifiers at the GM's discretion"
```

- [ ] **Step 2: Write the failing integration tests** (append to `cli_integration.rs`)

```rust
#[test]
fn show_notes_flag_displays_notes() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.wilderness-encounter",
            "--notes",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Notes:"));
    assert!(stdout.contains("Roll once per watch"));
}

#[test]
fn show_without_notes_flag_omits_notes() {
    let output = fatescroll_bin()
        .args([
            "show",
            "--collection",
            &fixtures_path("valid-collection").to_string_lossy(),
            "test.encounters.wilderness-encounter",
        ])
        .output()
        .expect("failed to run fatescroll");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Notes:"));
    assert!(!stdout.contains("Roll once per watch"));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p fatescroll-cli show_notes 2>&1 | tail -20`
Expected: FAIL — `--notes` is an unexpected argument (clap exits non-zero), so `show_notes_flag_displays_notes` fails the `status.success()` assertion.

- [ ] **Step 4: Add the `--notes` flag and thread it through**

In `main.rs`, add to the `Show` variant:

```rust
        /// Include the table's notes in the output
        #[arg(long)]
        notes: bool,
```

Update the `Commands::Show { collection, table_id }` match arm to destructure `notes` and pass it:

```rust
        Commands::Show {
            collection,
            table_id,
            notes,
        } => resolve_collection(collection)
            .and_then(|collection| cmd_show(&collection, &table_id, notes)),
```

Change `cmd_show`:

```rust
fn cmd_show(collection: &Path, table_id: &str, show_notes: bool) -> Result<(), fatescroll_core::Error> {
    let registry = fatescroll_core::load_collection(collection)?;
    let table = registry.get(table_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("table not found: '{table_id}'"),
        )
    })?;
    print!(
        "{}",
        fatescroll_core::display::format_table(table_id, table, show_notes)
    );
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p fatescroll-cli 2>&1 | tail -15`
Expected: PASS — both new tests pass and no existing integration test regresses.

- [ ] **Step 6: Commit**

```bash
git add fatescroll-cli/src/main.rs fatescroll-cli/tests/cli_integration.rs tests/fixtures/valid-collection/encounters/wilderness-encounter.yaml
git commit -s -m "feat: add show --notes flag (fatescroll-6ff)

Assisted-by: Claude:claude-opus-4-8"
```

---

## Task 4: Roller-ignores-notes guard test

**Files:**
- Test: `fatescroll-core/src/roller.rs` `mod tests`

This task adds a regression guard. No production change is needed because the roller never reads `notes`; the test locks that contract. It is expected to pass on first run (no red phase) — that is correct for a characterization/guard test.

- [ ] **Step 1: Write the guard test** (append to `roller.rs` `mod tests`)

```rust
#[test]
fn roller_ignores_notes() {
    let mut reg = Registry::new();
    reg.register(
        "ns.noted".into(),
        Table::Simple {
            id: "noted".into(),
            name: "Noted".into(),
            tags: vec![],
            roll: "1d6".into(),
            modifier_range: None,
            notes: vec!["This note must not affect rolling".into()],
            results: vec![ResultEntry {
                min: 1,
                max: 6,
                text: Some("Only outcome".into()),
                chain: None,
            }],
        },
    )
    .unwrap();

    let mut rng = diceman::FastRng::with_seed(42);
    let result = roll_with_rng(&reg, "ns.noted", &mut rng).unwrap();
    assert_eq!(result.table_name, "Noted");
    assert_eq!(result.text.as_deref(), Some("Only outcome"));
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p fatescroll-core --lib roller_ignores_notes 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add fatescroll-core/src/roller.rs
git commit -s -m "test: assert roller ignores table notes (fatescroll-6ff)

Assisted-by: Claude:claude-opus-4-8"
```

---

## Final verification (after all tasks)

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

All must pass clean. Then review roborev on each commit, address relevant findings, and merge the branch into local `main` with `--no-ff`.

## Self-review notes

- **Spec coverage:** notes field (T1) ✓, `notes()` accessor (T1) ✓, both variants (T1) ✓, `show_notes` display block placement before results (T2) ✓, `--notes` CLI flag (T3) ✓, default-clean output (T2 + T3) ✓, roller ignores notes (T4) ✓, init/search/roller code untouched (no task modifies them) ✓.
- **Type consistency:** `format_table(fqid, table, show_notes)` 3-arg signature used identically in T2 (definition, display tests, cmd_show stub) and T3 (cmd_show final). `notes()` returns `&[String]`, asserted against `&[String]` in T1. `render_notes` takes `&[String]`.
- **No placeholders:** every code step has concrete code; fixture content concrete; commands concrete.
