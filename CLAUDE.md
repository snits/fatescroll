# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

fatescroll is a Rust CLI tool for managing and rolling on RPG random tables defined in YAML. Tables support chaining (results trigger rolls on other tables), compound tables (roll multiple sub-tables at once), dice interpolation in result text, and reroll modifiers on chain references.

## Build & Test Commands

```bash
cargo build                          # Build entire workspace
cargo test                           # Run all tests (all crates)
cargo test -p fatescroll-core        # Core library tests only
cargo test -p fatescroll-core --lib  # Core unit tests only
cargo test -p fatescroll-cli         # CLI + integration tests only
cargo test <test_name>               # Run a single test by name
cargo clippy -- -D warnings          # Lint (must pass clean)
cargo fmt                            # Auto-format code
cargo fmt --check                    # Check formatting without modifying
```

A pre-commit hook (managed by `cargo-husky`, installed automatically on `cargo test`) runs `cargo fmt --check` and `cargo clippy -- -D warnings` on every commit. Hook source lives in `fatescroll-cli/.cargo-husky/hooks/pre-commit`. Never bypass hooks.

## Architecture

fatescroll is a Cargo workspace with two crates:
- **`fatescroll-core`** — Library crate with all domain logic (no CLI dependencies). External consumers (e.g., hexwalker) depend on this.
- **`fatescroll-cli`** — Binary crate providing the `fatescroll` CLI, depends on `fatescroll-core`.

The data flow is: **manifest → discovery → loading → validation → registry → rolling/display**.

### Loading Pipeline

1. **`collection.rs`** — Discovers YAML table files by walking manifest `directories` and `files` entries. Returns `CollectionFile` structs (path, namespace, stem, contents). Soft errors accumulated, not fatal.

2. **`loader.rs`** — Reads the manifest, calls discovery, parses each YAML file into `Table` enums, validates namespaces and id-filename matches, registers tables into a `Registry` with fully qualified IDs (`{namespace}.{stem}`).

3. **`validator.rs`** — Per-table validation (range coverage, dice expression validity via `diceman::simulate_seeded`), then cross-reference validation after registry is populated (chain refs and compound sub-table refs resolve).

4. **`lib.rs`** — `load_collection()` orchestrates: load → validate refs → return `Registry`.

### Core Types (`models.rs`)

- **`Table`** enum: `Simple` (dice roll → result entries) or `Compound` (rolls multiple sub-tables)
- **`ChainRef`** enum: `Simple(String)` or `Modified { table, reroll }` — `#[serde(untagged)]` for backward-compatible YAML
- **`Manifest`** struct: `directories` and `files` (both `#[serde(default)]`), `base_path` set at load time via `#[serde(skip)]`
- **`RollResult`**: recursive tree of roll outcomes with optional children from chains

### Registry (`registry.rs`)

In-memory `BTreeMap<String, Table>` keyed by FQID. Reference resolution is relative-first: tries `{caller_namespace}.{ref}` before bare `{ref}`.

### CLI (`fatescroll-cli/src/main.rs`)

Thin clap v4 derive-macro wrapper. Subcommands: `validate`, `roll`, `search`, `show`, `init`, `import`. Most resolve a collection via `resolve_collection()` which accepts a manifest path, directory, or falls back to CWD. `init` is standalone (no collection needed).

### Error Handling (`error.rs`)

Top-level `Error` enum wraps `ValidationError`, `LoadError`, `RollError`, `serde_yaml::Error`, `std::io::Error`, and `diceman::Error` via `thiserror`.

## Key Dependencies

- **`diceman`** — Dice expression parser and roller (git dependency from `github.com/snits/diceman`). Key API: `diceman::parse()`, `diceman::roll()`, `diceman::simulate_seeded(expr, iterations, seed)` returns `SimResult { min, max }`.
- **`clap` v4** — CLI framework using derive macros (fatescroll-cli only)
- **`serde` / `serde_yaml`** — YAML deserialization
- **`thiserror`** — Error type derive macros

## Testing Patterns

- **Unit tests**: inline `#[cfg(test)] mod tests` in each `fatescroll-core` source file, using `fixtures_path()` helper that resolves to `tests/fixtures/` at the workspace root.
- **Integration tests**: `fatescroll-cli/tests/cli_integration.rs` runs the compiled binary via `Command::new(env!("CARGO_BIN_EXE_fatescroll"))`. Uses `fixtures_path()` for fixture collections and `tempfile::TempDir` for write tests.
- **Fixtures**: `tests/fixtures/valid-collection/` is the primary fixture with 11 tables. Other fixtures test specific scenarios (invalid, id-mismatch, file-entries, files-only). Fixtures live at the workspace root and are shared by both crates.

## Conventions

- All `.rs` files must start with two `// ABOUTME:` comment lines describing the file's purpose.
- Table `id` must match the filename stem (enforced by loader as `IdFilenameMismatch` error).
- Namespace format: lowercase dot-separated, validated by `validate_namespace()` in `validator.rs`.
- Generated YAML from `init` omits the `id` field — the loader derives it from the filename.
- `git commit -s` required (sign-off). Feature branches required — never commit to main.

## PROJECT SCALE CONTEXT

- **Users**: Single developer (Jerry), personal CLI tool for tabletop RPG prep
- **Codebase**: Small (~2K lines Rust), Cargo workspace with library + binary crate
- **Complexity preference**: Simple, pragmatic, YAGNI
- **Process**: TDD mandatory, frequent commits, beads for issue tracking
