# Documentation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create user documentation for fatescroll — a README for end users and an authoring guide for table creators.

**Architecture:** Two markdown documents. README.md at project root covers installation and CLI usage. docs/authoring-guide.md covers the YAML table format, collection structure, and authoring workflow.

**Tech Stack:** Markdown. Examples use the test fixture collection at `tests/fixtures/valid-collection/`.

**Context for implementor:** fatescroll is an RPG random table manager written in Rust. It loads YAML table collections, validates them, and rolls dice on them. The CLI has four subcommands: validate, roll, search, import. Tables can chain to other tables (a result triggers additional rolls) and compound tables roll on multiple sub-tables. Dice expressions like `{2d6x10}` in result text get evaluated inline. Tables are identified by fully-qualified IDs (FQIDs) built from namespace + filename stem (e.g., `test.encounters.wilderness-encounter`).

**Key source files to reference:**
- `src/main.rs` — CLI definition (clap), subcommand flags and arguments
- `src/models.rs` — Table, ResultEntry, Manifest, RollResult structs and YAML deserialization
- `src/roller.rs` — Roll engine, chain resolution, dice interpolation
- `src/loader.rs` — Collection loading from manifest
- `src/validator.rs` — Validation rules (ranges, dice, cross-references)
- `src/registry.rs` — Table storage and relative-first resolution
- `tests/fixtures/valid-collection/` — Example collection with manifest, simple tables, compound tables, chains, and dice interpolation

---

### Task 1: README.md

**Files:**
- Create: `README.md`

**Reference files (read these first):**
- `src/main.rs` — CLI structure, subcommand names, flags, argument descriptions
- `tests/fixtures/valid-collection/` — Example collection structure for the quick-start section
- `Cargo.toml` — Project name, version, dependencies

**Step 1: Write README.md**

The README must contain these sections in order:

1. **Header** — Project name, one-line description: "RPG random table manager and roller"

2. **Overview** — 2-3 sentences. fatescroll loads collections of random tables defined in YAML, validates them, and rolls on them from the command line. Tables support chaining (a result triggers rolls on other tables), compound tables (roll multiple tables at once), and inline dice interpolation in result text.

3. **Installation** — Build from source:
   ```
   git clone https://github.com/snits/fatescroll.git
   cd fatescroll
   cargo install --path .
   ```

4. **Quick Start** — Walk through creating a minimal collection:
   - Create a directory with a `manifest.yaml`
   - Create one simple table YAML file
   - Run `fatescroll validate` on it
   - Run `fatescroll roll` on it
   - Use realistic but simple examples (e.g., a "Tavern Events" table with 1d6)
   - Show example CLI output for the roll

5. **CLI Usage** — One example with sample output per subcommand. Read `src/main.rs` for exact flag names:
   - `validate <collection>` — validate a collection directory
   - `roll --collection <path> <table_id>` — roll on a table by FQID
   - `search --collection <path> --name|--tag|--namespace <query>` — find tables
   - `import --collection <path> --target-dir <dir> <files...>` — import table files
   - Show realistic sample output for each (read `main.rs` print functions for output format)

6. **Documentation** — Link to `docs/authoring-guide.md` for table authoring details

7. **License** — "MIT — see LICENSE"

**Style guidance:**
- Keep it concise. This is a CLI tool README, not a textbook.
- Use fenced code blocks for all commands and YAML examples.
- Sample CLI output should match what the code actually prints (check `main.rs` print functions).
- The quick-start example collection should be self-contained — a reader should be able to copy-paste and have it work.

**Step 2: Verify accuracy**

Run these against the test fixture collection to confirm output format matches what you documented:
```bash
cargo run -- validate tests/fixtures/valid-collection
cargo run -- roll --collection tests/fixtures/valid-collection test.terrain.wilderness
cargo run -- search --collection tests/fixtures/valid-collection --tag encounter
```

**Step 3: Commit**

```bash
git add README.md
git commit -s -m "docs: add README with installation, quick start, and CLI usage"
```

---

### Task 2: docs/authoring-guide.md

**Files:**
- Create: `docs/authoring-guide.md`

**Reference files (read these first):**
- `src/models.rs` — Table enum (Simple/Compound), ResultEntry struct, Manifest struct — these define the YAML schema
- `src/loader.rs` — How collections are loaded, how FQIDs are constructed from namespace + filename stem
- `src/roller.rs` — Chain resolution logic, dice interpolation regex `\{([^}]+)\}`, compound table rolling
- `src/validator.rs` — All validation rules: `validate_table()` for per-type checks, `validate_references()` for cross-reference checks, `validate_namespace()` for namespace format
- `src/registry.rs` — `resolve()` method for relative-first reference resolution
- `tests/fixtures/valid-collection/` — Complete working example collection

**Step 1: Write docs/authoring-guide.md**

The guide must contain these sections:

1. **Header** — "Authoring Guide" with a brief intro: this guide covers how to create table collections for fatescroll.

2. **Collections** — What a collection is (a directory with a manifest.yaml and subdirectories of table files). Document the manifest.yaml schema based on the `Manifest` struct in `models.rs`:
   - `name` (string, required) — collection name
   - `version` (string, required) — collection version
   - `namespace` (string, required) — root namespace
   - `author` (string, optional) — author name
   - `min_tool_version` (string, optional) — minimum fatescroll version
   - `directories` (list, required) — each entry has `path` (relative directory) and `namespace` (dot-separated)
   - Show the test fixture manifest as an example

3. **Namespaces and FQIDs** — Explain how namespaces work:
   - Dot-separated hierarchy (e.g., `test.encounters`)
   - FQID = directory namespace + "." + filename stem (e.g., file `wilderness-encounter.yaml` in namespace `test.encounters` becomes `test.encounters.wilderness-encounter`)
   - Namespace validation rules: check `validate_namespace()` in `validator.rs` for the exact rules (lowercase alphanumeric + dots + hyphens)

4. **Simple Tables** — Document the YAML schema based on the `Table::Simple` variant:
   - `name` (string) — display name
   - `type: simple` (required discriminator)
   - `tags` (list of strings, optional, defaults to empty)
   - `roll` (string) — dice expression (e.g., "1d6", "2d8+1")
   - `results` (list of ResultEntry) — each has:
     - `min` (integer) — minimum roll value for this result
     - `max` (integer) — maximum roll value for this result
     - `text` (string, optional) — result text
     - `chain` (list of strings, optional) — table references to roll on after this result
   - Show `wilderness.yaml` from test fixtures as example

5. **Compound Tables** — Document `Table::Compound`:
   - `name`, `type: compound`, `tags`
   - `tables` (list of strings) — references to other tables, all rolled when this table is rolled
   - Show `quick-npc.yaml` from test fixtures as example
   - Explain that compound tables produce no roll/text of their own, just children

6. **Chaining** — How chain references work:
   - A result entry's `chain` field lists table references
   - When that result is rolled, each chain reference triggers an additional roll
   - References resolve relative to the current namespace first, then as FQIDs (check `registry.rs` `resolve()` method)
   - Show `wilderness-encounter.yaml` as an example — some results chain to `animal-type`, `bandit-strength`, etc.
   - Depth limit of 10 prevents infinite loops (from `roller.rs` MAX_CHAIN_DEPTH)

7. **Dice Interpolation** — Inline dice in result text:
   - Syntax: `{dice_expression}` in result text (e.g., `"Merchant with {2d6x10} gold"`)
   - The expression inside braces is evaluated when the result is rolled
   - Any valid diceman dice expression works
   - If evaluation fails, the original `{expression}` text is preserved
   - Show the merchant example from `wilderness-encounter.yaml`

8. **Validation Rules** — What `fatescroll validate` checks. Read `validator.rs` carefully for the complete list:
   - Per-table validation (`validate_table`):
     - Dice expression must be parseable
     - Result ranges must not have gaps
     - Result ranges must not overlap
     - Result ranges must not be reversed (min > max)
   - Cross-reference validation (`validate_references`):
     - All chain references must resolve to existing tables
     - All compound table references must resolve to existing tables
   - Namespace validation:
     - Must match expected format (check the regex/rules in `validate_namespace`)
   - Document what error messages look like so authors can fix problems

9. **Example Collection** — A complete worked example. Use the test fixture collection structure as the basis, but present it as a tutorial:
   - Show the directory tree
   - Show the manifest
   - Show 2-3 tables demonstrating simple, compound, chaining, and dice interpolation
   - Show running validate and roll against it

**Style guidance:**
- Write for someone who has never seen the project. Define terms before using them.
- Every YAML field gets a type and description. Show don't tell — use examples from the test fixtures.
- Validation rules section should help authors fix problems, not just list what's checked.

**Step 2: Verify accuracy**

Cross-check all YAML schemas against the actual structs in `models.rs`. Cross-check validation rules against `validator.rs`. Cross-check chain resolution description against `roller.rs` and `registry.rs`.

Run the example collection through fatescroll to confirm it works:
```bash
cargo run -- validate tests/fixtures/valid-collection
```

**Step 3: Commit**

```bash
git add docs/authoring-guide.md
git commit -s -m "docs: add table authoring guide"
```
