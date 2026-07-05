# Table Forge WebUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build "Table Forge" — a browser-based editor for authoring fatescroll YAML table collections, per the design handoff in `docs/design/table-forge/` — with all validation, dice, and roll logic supplied by `fatescroll-core` compiled to WASM.

**Architecture:** Three-layer split. (1) `fatescroll-core` gains one seam: `build_registry()` — the existing registry-building loop extracted from `load_collection()` so it works on in-memory file contents. (2) A new `fatescroll-wasm` cdylib crate wraps core + diceman behind five JSON-string functions (`validate_collection`, `dice_info`, `expected_values`, `histogram`, `roll_collection`). (3) A React + TypeScript + Vite SPA in `webui/` holds all UI state, emits YAML text, and feeds that YAML to the WASM engine for validation and rolling — the exact same code path the CLI runs on files, so the UI can never drift from `fatescroll validate`.

**Tech Stack:** Rust (wasm-bindgen, wasm-pack), React 19 + TypeScript + Vite, Zustand (state), fflate (zip export), @fontsource packages (IM Fell English SC, Spectral, JetBrains Mono), Vitest + Testing Library (unit), execa-driven golden round-trip test against the real CLI.

**Decisions locked (Jerry, 2026-07-05):** WASM engine from fatescroll-core (not a TS port, not a local server); React + TS + Vite; v1 is create-and-export only — importing existing collections is a fast-follow bead, not in this plan.

---

## Design source of truth

- `docs/design/table-forge/README.md` — the handoff spec (layout, tokens, algorithms, interactions). Task 0 copies it into the repo from `.claude/Fatescroll table interface.zip`.
- `docs/design/table-forge/Table Forge.dc.html` — working prototype; open in a browser to compare visuals.
- **Where the handoff and `fatescroll-core` disagree, core wins.** Known deltas (all resolved by using core via WASM):

| # | Handoff prototype says | fatescroll-core reality | Resolution |
|---|---|---|---|
| 1 | Own dice regex (`NdF`, `dF` fudge, `x` mult, `d66` special-cased) | diceman grammar; `D66`/`D666` digit-dice; broader modifiers | UI asks WASM `dice_info()`; never parses dice in TS |
| 2 | Own validation messages | `ValidationError` Display strings | Right pane shows core's messages verbatim |
| 3 | modifier_range + d66 allowed | `ModifierUnsupportedForDigitDice` error | UI disables the modifier checkbox when `dice_info.kind === 'digit'` |
| 4 | Coverage over exact expected values | Digit dice: exact values; other dice: contiguous envelope (analytic or simulated min/max) | `expected_values()` WASM fn mirrors validator logic; autofill uses it |
| 5 | notes on simple tables only | Core supports notes on compound too | Keep design as-is (simple only); noted as future nicety |
| 6 | Hand-rolled store-only zip writer | n/a | Use fflate |
| 7 | No `files:` manifest entries | Core supports them | Out of scope v1 (design has no UI for them) |
| 8 | modMin/modMax number inputs | `modifier_range` is i32 — negatives valid (e.g. `[-6, 0]`) | Inputs must accept a leading `-` |

Emitted table YAML includes `id:` (= stem). The loader requires id == filename stem, and `load_table_str` requires id present, so this is correct (only `init`-generated templates omit it).

## File structure

```
fatescroll/
├── Cargo.toml                        # MODIFY: add "fatescroll-wasm" to members
├── fatescroll-core/src/
│   ├── loader.rs                     # MODIFY: extract build_registry()
│   └── dice.rs                       # MODIFY: digit_dice_params pub(crate) → pub
├── fatescroll-wasm/                  # NEW crate
│   ├── Cargo.toml
│   └── src/lib.rs                    # five wasm-bindgen fns + serde structs
├── webui/                            # NEW Vite app (not a cargo member)
│   ├── package.json / vite.config.ts / tsconfig.json
│   ├── src/
│   │   ├── main.tsx / App.tsx
│   │   ├── model/types.ts            # ManifestState, Dir, TableDraft, ResultDraft, ChainDraft
│   │   ├── model/store.ts            # Zustand store + actions
│   │   ├── model/ids.ts              # uid()
│   │   ├── yaml/emit.ts              # yv(), manifestYaml(), tableYaml(), collectionFiles()
│   │   ├── engine/engine.ts          # Engine interface + WASM implementation
│   │   ├── engine/useEngine.ts       # init + debounced derived validation/yaml hooks
│   │   ├── logic/autofill.ts         # autofillRanges()
│   │   ├── logic/probability.ts      # rangeProbability(), formatPct()
│   │   ├── logic/slug.ts             # collectionSlug()
│   │   ├── export/zip.ts             # buildCollectionZip() via fflate
│   │   ├── components/HeaderBar.tsx
│   │   ├── components/Scriptorium.tsx        # left tree
│   │   ├── components/ManifestEditor.tsx
│   │   ├── components/TableEditor.tsx        # + ResultCard.tsx, ChainRow.tsx, CompoundEditor.tsx
│   │   ├── components/RightPane.tsx          # + YamlViewer.tsx, ValidationPanel.tsx, DiceRoller.tsx
│   │   └── styles/tokens.css / app.css
│   └── tests/
│       ├── emit.test.ts, autofill.test.ts, probability.test.ts, slug.test.ts, store.test.ts
│       ├── components/*.test.tsx
│       └── golden-roundtrip.test.ts  # emits a collection to tmpdir, runs real CLI validate + roll
└── docs/design/table-forge/          # NEW: unzipped design handoff (Task 0)
```

**Evaluation rubric** for reviewer agents: `docs/superpowers/plans/2026-07-05-table-forge-webui-rubric.md` (written alongside this plan). Every task-completion review and the final review score against it.

## Prerequisites (one-time setup)

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack          # skip if `wasm-pack --version` works
node --version                   # need >= 20
```

## Beads

At execution start create an epic `Table Forge webui` and one task bead per task below, wired `bd dep add` in numeric order (Tasks 7–12 depend on 4–6; 13 depends on 5+11). File a fast-follow bead now: *"Table Forge: open/import existing collection (directory picker or zip upload)"*.

**Branch:** all work on `feat/table-forge-webui` (never main). Commit per task with `git commit -s`, trailer `Assisted-by: Claude:<model-id>`.

---

### Task 0: Check in the design handoff

**Files:**
- Create: `docs/design/table-forge/README.md`, `docs/design/table-forge/Table Forge.dc.html`, `docs/design/table-forge/support.js`

- [ ] **Step 1: Unzip and commit**

```bash
git checkout -b feat/table-forge-webui
mkdir -p docs/design/table-forge
unzip -j ".claude/Fatescroll table interface.zip" -d docs/design/table-forge/
git add docs/design/table-forge
git commit -s -m "docs: check in Table Forge design handoff"
```

Expected: three files under `docs/design/table-forge/`, commit clean (docs-only, hooks pass trivially).

---

### Task 1: Core seam — `build_registry()` + public `digit_dice_params()`

Extract the in-memory registry-building loop from `load_collection` (`fatescroll-core/src/loader.rs:13-84`) so WASM can build a registry from browser-held YAML strings. Pure refactor + one visibility change; no behavior change.

**Files:**
- Modify: `fatescroll-core/src/loader.rs`
- Modify: `fatescroll-core/src/dice.rs:11` (`pub(crate) fn digit_dice_params` → `pub fn digit_dice_params`)
- Modify: `fatescroll-core/src/lib.rs` (re-export `build_registry`)
- Test: inline `#[cfg(test)]` in `loader.rs`

- [ ] **Step 1: Write the failing test** (in `loader.rs` tests module)

```rust
#[test]
fn build_registry_from_in_memory_files() {
    let manifest: Manifest = serde_yaml::from_str(
        "name: Mem\nversion: \"1.0\"\nnamespace: mem\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: core\n    namespace: mem.core\n",
    )
    .unwrap();
    let files = vec![crate::collection::CollectionFile {
        path: std::path::PathBuf::from("core/oracle.yaml"),
        namespace: "mem.core".into(),
        stem: "oracle".into(),
        contents: "id: oracle\nname: Oracle\ntype: simple\nroll: 1d6\nresults:\n  - min: 1\n    max: 6\n    text: X\n".into(),
    }];
    let (registry, errors) = build_registry(&manifest, &files);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert!(registry.get("mem.core.oracle").is_some());
}

#[test]
fn build_registry_collects_bad_namespace_error() {
    let manifest: Manifest = serde_yaml::from_str(
        "name: Mem\nversion: \"1.0\"\nnamespace: \"BAD NS\"\nauthor: ~\nmin_tool_version: ~\n",
    )
    .unwrap();
    let (_registry, errors) = build_registry(&manifest, &[]);
    assert_eq!(errors.len(), 1);
}
```

(Adjust imports: `use crate::models::Manifest;` in the test module.)

- [ ] **Step 2: Run to verify failure**

`cargo test -p fatescroll-core build_registry` — expect: does not compile, `build_registry` not found.

- [ ] **Step 3: Extract the function**

In `loader.rs`, move the body of `load_collection` after discovery into:

```rust
/// Build a registry from a manifest and pre-read collection files.
/// Filesystem-free core of [`load_collection`]; also used by fatescroll-wasm.
/// Returns the registry plus all per-file errors (bad namespace, YAML parse,
/// id/filename mismatch, table validation, duplicate registration).
pub fn build_registry(
    manifest: &Manifest,
    files: &[crate::collection::CollectionFile],
) -> (Registry, Vec<Error>) {
    let mut errors = Vec::new();
    let mut registry = Registry::new();

    if let Err(e) = validate_namespace(&manifest.namespace) {
        errors.push(e.into());
    }
    // ... existing namespace-cache + per-file loop from load_collection,
    // verbatim, pushing into `errors` ...
    (registry, errors)
}

pub fn load_collection(manifest_path: &Path) -> Result<Registry, Error> {
    let (manifest, files, mut errors) =
        crate::collection::discover_collection_files(manifest_path)?;
    let (registry, mut reg_errors) = build_registry(&manifest, &files);
    errors.append(&mut reg_errors);
    if errors.is_empty() {
        Ok(registry)
    } else {
        Err(LoadError::Multiple { errors }.into())
    }
}
```

Add `use crate::models::Manifest;` to loader.rs imports. In `dice.rs` change `pub(crate) fn digit_dice_params` to `pub fn digit_dice_params` (keep the doc comment). In `lib.rs` add `pub use loader::build_registry;` next to the existing loader re-exports.

- [ ] **Step 4: Run the full suite**

`cargo test` — expect: all existing tests + 2 new pass. `cargo clippy -- -D warnings` clean.

- [ ] **Step 5: Commit**

```bash
git add fatescroll-core/src/loader.rs fatescroll-core/src/dice.rs fatescroll-core/src/lib.rs
git commit -s -m "refactor(core): extract build_registry for in-memory collection loading"
```

---

### Task 2: `fatescroll-wasm` crate

Five plain-Rust functions (they compile natively, so TDD runs under `cargo test`), annotated `#[wasm_bindgen]` for the browser. All I/O is JSON strings — no serde-wasm-bindgen dependency. All randomness is seeded from JS (`u64` ↔ BigInt) because `fastrand::Rng::new()` has no entropy source on wasm32-unknown-unknown.

**Files:**
- Modify: `Cargo.toml` (workspace members: add `"fatescroll-wasm"`)
- Create: `fatescroll-wasm/Cargo.toml`
- Create: `fatescroll-wasm/src/lib.rs`

- [ ] **Step 1: Scaffold the crate**

`fatescroll-wasm/Cargo.toml`:

```toml
[package]
name = "fatescroll-wasm"
version = "0.1.0"
edition = "2024"
license = "MIT"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fatescroll-core = { path = "../fatescroll-core" }
diceman = { git = "https://github.com/snits/diceman.git", tag = "v0.4.0" }
wasm-bindgen = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = { package = "yaml_serde", version = "0.10" }
```

Add `"fatescroll-wasm"` to `members` in the root `Cargo.toml`. Create `src/lib.rs` with just the two ABOUTME lines for now; `cargo build` must pass.

- [ ] **Step 2: Write failing tests** (inline `#[cfg(test)]` in `lib.rs`)

```rust
const MANIFEST: &str = "name: T\nversion: \"1.0\"\nnamespace: t\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: core\n    namespace: t.core\n";

fn files_json() -> String {
    serde_json::json!([{
        "path": "core/oracle.yaml", "namespace": "t.core", "stem": "oracle",
        "contents": "id: oracle\nname: Oracle\ntype: simple\nroll: 1d6\nresults:\n  - min: 1\n    max: 6\n    text: \"Yes\"\n"
    }]).to_string()
}

#[test]
fn validate_collection_valid() {
    let out: serde_json::Value =
        serde_json::from_str(&validate_collection(MANIFEST, &files_json())).unwrap();
    assert_eq!(out["errors"].as_array().unwrap().len(), 0);
}

#[test]
fn validate_collection_reports_unresolved_chain() {
    let files = serde_json::json!([{
        "path": "core/a.yaml", "namespace": "t.core", "stem": "a",
        "contents": "id: a\nname: A\ntype: simple\nroll: 1d4\nresults:\n  - min: 1\n    max: 4\n    text: X\n    chain:\n      - missing-table\n"
    }]).to_string();
    let out: serde_json::Value =
        serde_json::from_str(&validate_collection(MANIFEST, &files)).unwrap();
    let errs = out["errors"].as_array().unwrap();
    assert_eq!(errs.len(), 1);
    assert!(errs[0].as_str().unwrap().contains("missing-table"));
}

#[test]
fn dice_info_standard_and_digit_and_bad() {
    let d6: serde_json::Value = serde_json::from_str(&dice_info("2d6")).unwrap();
    assert_eq!((d6["ok"].as_bool(), d6["min"].as_i64(), d6["max"].as_i64(), d6["kind"].as_str()),
               (Some(true), Some(2), Some(12), Some("range")));
    let d66: serde_json::Value = serde_json::from_str(&dice_info("D66")).unwrap();
    assert_eq!((d66["kind"].as_str(), d66["min"].as_i64(), d66["max"].as_i64(), d66["outcomes"].as_i64()),
               (Some("digit"), Some(11), Some(66), Some(36)));
    let bad: serde_json::Value = serde_json::from_str(&dice_info("not dice")).unwrap();
    assert_eq!(bad["ok"].as_bool(), Some(false));
    assert!(bad["reason"].is_string());
}

#[test]
fn expected_values_modifier_and_digit() {
    let v: serde_json::Value = serde_json::from_str(&expected_values("1d8", true, 0, 6)).unwrap();
    let vals: Vec<i64> = v["values"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
    assert_eq!(vals, (1..=14).collect::<Vec<i64>>());
    let d66: serde_json::Value = serde_json::from_str(&expected_values("D66", false, 0, 0)).unwrap();
    let dv = d66["values"].as_array().unwrap();
    assert_eq!(dv.len(), 36);
    assert_eq!(dv[6].as_i64(), Some(21)); // 11..16 then 21
    // digit dice + modifier is a core error
    let err: serde_json::Value = serde_json::from_str(&expected_values("D66", true, 0, 1)).unwrap();
    assert_eq!(err["ok"].as_bool(), Some(false));
}

#[test]
fn histogram_sums_to_iterations() {
    let h: serde_json::Value = serde_json::from_str(&histogram("2d6", 10_000, 42)).unwrap();
    let total: i64 = h["counts"].as_object().unwrap().values().map(|v| v.as_i64().unwrap()).sum();
    assert_eq!(total, 10_000);
}

#[test]
fn roll_collection_returns_result_tree() {
    let out: serde_json::Value =
        serde_json::from_str(&roll_collection(MANIFEST, &files_json(), "t.core.oracle", 7)).unwrap();
    assert_eq!(out["table_name"].as_str(), Some("Oracle"));
    assert!(out["roll"].as_i64().is_some());
}

#[test]
fn roll_collection_unknown_table_is_error() {
    let out: serde_json::Value =
        serde_json::from_str(&roll_collection(MANIFEST, &files_json(), "t.core.nope", 7)).unwrap();
    assert!(out["error"].is_string());
}
```

- [ ] **Step 3: Run to verify failure** — `cargo test -p fatescroll-wasm` → compile errors (fns missing).

- [ ] **Step 4: Implement `src/lib.rs`**

```rust
// ABOUTME: WASM bindings exposing fatescroll-core validation, dice info, and rolling
// ABOUTME: to the Table Forge webui. All I/O is JSON strings; RNG seeds come from JS.

use fatescroll_core::collection::CollectionFile;
use fatescroll_core::models::Manifest;
use fatescroll_core::validator::validate_references;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
struct FileInput {
    path: String,
    namespace: String,
    stem: String,
    contents: String,
}

fn parse_inputs(
    manifest_yaml: &str,
    files_json: &str,
) -> Result<(Manifest, Vec<CollectionFile>), String> {
    let manifest: Manifest =
        serde_yaml::from_str(manifest_yaml).map_err(|e| format!("manifest: {e}"))?;
    let inputs: Vec<FileInput> =
        serde_json::from_str(files_json).map_err(|e| format!("files: {e}"))?;
    let files = inputs
        .into_iter()
        .map(|f| CollectionFile {
            path: PathBuf::from(f.path),
            namespace: f.namespace,
            stem: f.stem,
            contents: f.contents,
        })
        .collect();
    Ok((manifest, files))
}

/// Validate a whole collection held in memory. Returns {"errors": [String]}.
/// Mirrors the CLI: build_registry per-file checks, then cross-reference checks.
#[wasm_bindgen]
pub fn validate_collection(manifest_yaml: &str, files_json: &str) -> String {
    let (manifest, files) = match parse_inputs(manifest_yaml, files_json) {
        Ok(v) => v,
        Err(e) => return json!({ "errors": [e] }).to_string(),
    };
    let (registry, errors) = fatescroll_core::build_registry(&manifest, &files);
    let mut messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
    if let Err(ref_errors) = validate_references(&registry) {
        messages.extend(ref_errors.iter().map(|e| e.to_string()));
    }
    json!({ "errors": messages }).to_string()
}

/// Dice expression info for the editor's roll-input hint.
/// {"ok":true,"kind":"digit"|"range"|"simulated","min":i64,"max":i64,"outcomes":usize}
/// or {"ok":false,"reason":String}.
#[wasm_bindgen]
pub fn dice_info(expr: &str) -> String {
    let parsed = match diceman::parse(expr) {
        Ok(p) => p,
        Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
    };
    if let Some((sides, count)) = fatescroll_core::dice::digit_dice_params(&parsed) {
        let values = fatescroll_core::dice::digit_dice_values(sides, count);
        return json!({
            "ok": true, "kind": "digit",
            "min": values.first().copied(), "max": values.last().copied(),
            "outcomes": values.len(),
        })
        .to_string();
    }
    match fatescroll_core::dice::dice_range(expr) {
        Ok((min, max)) => json!({
            "ok": true, "kind": "range", "min": min, "max": max,
            "outcomes": (max - min + 1),
        })
        .to_string(),
        // Analytically unsupported (keep/drop, exploding, ...): fall back to
        // the same seeded simulation the validator uses for envelopes.
        Err(_) => match diceman::simulate_seeded(expr, 100_000, 42) {
            Ok(sim) => json!({
                "ok": true, "kind": "simulated", "min": sim.min, "max": sim.max,
                "outcomes": (sim.max - sim.min + 1),
            })
            .to_string(),
            Err(e) => json!({ "ok": false, "reason": e.to_string() }).to_string(),
        },
    }
}

/// Expected coverage values for a table's results, mirroring validate_table:
/// digit dice -> exact digit values (modifier rejected); otherwise the
/// contiguous envelope [dice_min + mod_min, dice_max + mod_max].
/// {"ok":true,"values":[i64]} or {"ok":false,"reason":String}.
#[wasm_bindgen]
pub fn expected_values(expr: &str, mod_on: bool, mod_min: i32, mod_max: i32) -> String {
    let parsed = match diceman::parse(expr) {
        Ok(p) => p,
        Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
    };
    if let Some((sides, count)) = fatescroll_core::dice::digit_dice_params(&parsed) {
        if mod_on {
            return json!({ "ok": false, "reason": "modifier_range unsupported for digit dice" })
                .to_string();
        }
        return json!({ "ok": true, "values": fatescroll_core::dice::digit_dice_values(sides, count) })
            .to_string();
    }
    let (dmin, dmax) = match fatescroll_core::dice::dice_range(expr) {
        Ok(r) => (r.0 as i64, r.1 as i64),
        Err(_) => match diceman::simulate_seeded(expr, 100_000, 42) {
            Ok(sim) => (sim.min, sim.max),
            Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
        },
    };
    let (lo, hi) = if mod_on {
        (dmin + mod_min as i64, dmax + mod_max as i64)
    } else {
        (dmin, dmax)
    };
    if lo > hi || hi - lo > 100_000 {
        return json!({ "ok": false, "reason": "envelope reversed or too wide" }).to_string();
    }
    json!({ "ok": true, "values": (lo..=hi).collect::<Vec<i64>>() }).to_string()
}

/// Seeded sampling histogram for probability pills.
/// {"ok":true,"iterations":u32,"counts":{"<value>":count}} or {"ok":false,...}.
#[wasm_bindgen]
pub fn histogram(expr: &str, iterations: u32, seed: u64) -> String {
    if let Err(e) = diceman::parse(expr) {
        return json!({ "ok": false, "reason": e.to_string() }).to_string();
    }
    let mut rng = diceman::FastRng::with_seed(seed);
    let mut counts: std::collections::BTreeMap<i64, u32> = std::collections::BTreeMap::new();
    for _ in 0..iterations {
        match diceman::roll_with_rng(expr, &mut rng) {
            Ok(r) => match r.as_numeric() {
                Some(v) => *counts.entry(v).or_insert(0) += 1,
                None => {
                    return json!({ "ok": false, "reason": "non-numeric roll result" }).to_string()
                }
            },
            Err(e) => return json!({ "ok": false, "reason": e.to_string() }).to_string(),
        }
    }
    json!({ "ok": true, "iterations": iterations, "counts": counts }).to_string()
}

/// Roll a table (by FQID) against the in-memory collection. Returns the
/// serialized RollResult tree, or {"error": String}.
#[wasm_bindgen]
pub fn roll_collection(manifest_yaml: &str, files_json: &str, fqid: &str, seed: u64) -> String {
    let (manifest, files) = match parse_inputs(manifest_yaml, files_json) {
        Ok(v) => v,
        Err(e) => return json!({ "error": e }).to_string(),
    };
    // Roll best-effort: broken tables were dropped by build_registry; the
    // roller reports unresolved references itself.
    let (registry, _errors) = fatescroll_core::build_registry(&manifest, &files);
    let mut rng = diceman::FastRng::with_seed(seed);
    match fatescroll_core::roller::roll_with_rng(&registry, fqid, &mut rng) {
        Ok(result) => serde_json::to_string(&result)
            .unwrap_or_else(|e| json!({ "error": e.to_string() }).to_string()),
        Err(e) => json!({ "error": e.to_string() }).to_string(),
    }
}
```

Adjust to actual APIs while implementing (e.g. `RollResult::as_numeric` signature from diceman v0.4.0, `roller::roll_with_rng` path — see `fatescroll-core/src/roller.rs:32`). If `dice_range`'s error type needs unwrapping, match on `fatescroll_core::Error::Validation`.

- [ ] **Step 5: Run tests** — `cargo test -p fatescroll-wasm` → all pass. `cargo clippy -- -D warnings` and `cargo fmt --check` clean (pre-commit runs these).

- [ ] **Step 6: Verify the wasm target builds**

```bash
wasm-pack build fatescroll-wasm --target web --out-dir ../webui-pkg-check
rm -rf webui-pkg-check
```

Expected: build succeeds. If fastrand fails to compile for wasm32, add `fastrand = { version = "2", features = ["js"] }` to fatescroll-wasm deps (feature unification fixes the transitive dep) and note it in the commit message.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock fatescroll-wasm/
git commit -s -m "feat: add fatescroll-wasm crate exposing core engine to the browser"
```

---

### Task 3: webui scaffold + design tokens

**Files:**
- Create: `webui/` via Vite template, then `webui/src/styles/tokens.css`, `webui/src/styles/app.css`
- Create: `webui/.gitignore` entry for `src/wasm/pkg/`
- Modify: `webui/package.json` (scripts, deps)

- [ ] **Step 1: Scaffold**

```bash
npm create vite@latest webui -- --template react-ts
cd webui
npm install zustand fflate @fontsource/im-fell-english-sc @fontsource/spectral @fontsource/jetbrains-mono
npm install -D vitest @testing-library/react @testing-library/user-event jsdom execa @types/node
```

Delete template cruft (`App.css` demo content, logos). Add to `package.json` scripts:

```json
{
  "build:wasm": "wasm-pack build ../fatescroll-wasm --target web --out-dir ../webui/src/wasm/pkg",
  "dev": "npm run build:wasm && vite",
  "build": "npm run build:wasm && tsc -b && vite build",
  "test": "vitest run",
  "test:watch": "vitest"
}
```

Add `src/wasm/pkg/` to `webui/.gitignore`. `vite.config.ts` needs no wasm plugin — the engine module inits with `new URL('../wasm/pkg/fatescroll_wasm_bg.wasm', import.meta.url)`. Configure vitest in `vite.config.ts`: `test: { environment: 'jsdom' }`.

- [ ] **Step 2: Design tokens** — `webui/src/styles/tokens.css`, straight from the handoff §Design Tokens:

```css
:root {
  --bg-page: #e3d7bb;        --bg-panel: #efe6cf;      --bg-card: #f6efdc;
  --bg-input: #fbf6e9;       --bg-btn: #f7f0dc;        --bg-btn-alt: #f2ead6;
  --header-bg: #3a2f1c;      --header-fg: #eaddb9;
  --brand-gold: #e9d9a8;     --brand-gold-dim: #b79a5f;
  --text: #362d1e;           --text-2: #5c4a2c;        --text-muted: #8a744c;
  --text-faint: #a08a5f;     --text-faint-2: #b0996b;
  --border: #c8b28a;         --border-card: #cdb88f;   --border-btn: #c0a76c;
  --border-tree-idle: #d2c199;
  --ox: #8b2b2b;             --ox-hover: #a03434;      --ox-border: #7a2323;
  --ox-text: #5c1f1f;        --ox-active-bg: #e0d3b0;  --ox-on: #f4e3c8;
  --gold: #9c7a2f;           --gold-hover: #b18a37;    --gold-border: #c79a3d;
  --gold-on: #fbf3d8;
  --olive: #4f5d38;          --olive-border: #6f7d4f;
  --olive-bg: #eef0e0;       --olive-pill-bg: #e7ead6; --olive-pill-border: #c3cba3;
  --err: #a11f1f;            --err-2: #9a3030;         --err-border: #b05a5a;
  --err-bg: #f0dede;         --warn: #9c6a2f;
  --cmp-bg: #e2d3ea;         --cmp-fg: #6a4a86;
  --yaml-bg: #241d12;        --yaml-border: #2f2718;   --yaml-fg: #d9c99a;
  --chain-border: #c3a86e;
  --font-display: 'IM Fell English SC', serif;
  --font-body: 'Spectral', Georgia, serif;
  --font-mono: 'JetBrains Mono', monospace;
}
::selection { background: #d8c08a; color: #2a2314; }
* { scrollbar-width: thin; scrollbar-color: #c1a978 transparent; }
*::-webkit-scrollbar { width: 11px; }
*::-webkit-scrollbar-thumb {
  background: #c1a978; border: 3px solid transparent;
  background-clip: content-box; border-radius: 6px;
}
```

`app.css` sets the shell: `body { background: var(--bg-page); color: var(--text); font: 15px var(--font-body); }` plus the two radial-gradient overlays and the `100vh` flex column / three-pane flex row (264px / 1 / 440px) from handoff §Layout. Import fonts + both css files in `main.tsx` (`@fontsource/spectral/400.css` etc. — weights 300–700, italic 400; im-fell-english-sc 400; jetbrains-mono 400–600).

- [ ] **Step 3: Smoke test + commit**

`npm run dev` renders an empty three-pane shell (placeholder divs). `npm test` passes (no tests yet is fine — add a trivial `expect(true)` placeholder only if vitest errors on empty suite; remove it in Task 4).

```bash
git add webui
git commit -s -m "feat(webui): scaffold Vite React app with Table Forge design tokens"
```

---

### Task 4: Domain types + Zustand store

**Files:**
- Create: `webui/src/model/types.ts`, `webui/src/model/ids.ts`, `webui/src/model/store.ts`
- Test: `webui/tests/store.test.ts`

- [ ] **Step 1: Types** (`types.ts`) — matches handoff §Data model; numeric inputs stay raw strings:

```ts
export interface ManifestState {
  name: string; version: string; namespace: string;
  author: string; minToolVersion: string;   // '' means ~ (null)
}
export interface Dir { id: string; path: string; namespace: string }
export interface ChainDraft {
  rid: string;
  struct: boolean;        // structured entry: emits { table, reroll }
  ref: string;            // table reference (used for both forms)
  reroll: number[];       // only meaningful when struct
}
export interface ResultDraft {
  rid: string; min: string; max: string; text: string; chain: ChainDraft[];
}
export interface TableDraft {
  uid: string; dirId: string; stem: string; name: string;
  type: 'simple' | 'compound';
  tags: string[];
  roll: string;                                  // simple only
  modOn: boolean; modMin: string; modMax: string; // simple only
  notes: string[];                                // simple only
  results: ResultDraft[];                         // simple only
  tableRefs: { rid: string; ref: string }[];      // compound only
}
export type View = 'empty' | 'manifest' | 'table';
```

`ids.ts`: `export const uid = () => crypto.randomUUID();`

- [ ] **Step 2: Failing store tests** (`tests/store.test.ts`) — cover: initial state (`view === 'empty'`, one default dir? **No** — start with zero dirs, manifest defaults `{name:'New Collection', version:'1.0', namespace:'collection', author:'', minToolVersion:''}`); `addDir` selects manifest view; `addTable(dirId)` creates a simple table with one blank result and selects it; `updateTable` patches and clears `rollLines`; `deleteTable` selects next remaining table else `empty`; `deleteDir` removes its tables; `fqid(table)` = `dir.namespace + '.' + stem`; stem setter replaces whitespace with `-`.

- [ ] **Step 3: Run** — `npm test` → fails (store missing).

- [ ] **Step 4: Implement `store.ts`** — Zustand vanilla-compatible store:

```ts
interface ForgeState {
  manifest: ManifestState; dirs: Dir[]; tables: TableDraft[];
  view: View; selUid: string | null;
  rollLines: RollLine[] | null;      // cleared on every edit
  // actions
  selectManifest(): void; selectTable(uid: string): void;
  setManifest(patch: Partial<ManifestState>): void;
  addDir(): void; updateDir(id: string, patch: Partial<Dir>): void; deleteDir(id: string): void;
  addTable(dirId: string): void; updateTable(uid: string, patch: Partial<TableDraft>): void;
  deleteTable(uid: string): void;
  setRollLines(lines: RollLine[] | null): void;
}
export interface RollLine { indent: number; text: string; error?: boolean }
```

Every mutating action also sets `rollLines: null` (handoff §Interactions). New table defaults: `stem: 'new-table'`, `name: 'New Table'`, `type: 'simple'`, `roll: '1d6'`, one result `{min:'1',max:'6',text:'',chain:[]}`. Selectors as plain functions: `fqidOf(state, table)`, `tablesInDir(state, dirId)`.

- [ ] **Step 5: Run tests → pass. Commit** `feat(webui): domain model and store`.

---

### Task 5: YAML emitter

**Files:**
- Create: `webui/src/yaml/emit.ts`
- Test: `webui/tests/emit.test.ts`

- [ ] **Step 1: Failing tests.** Assert exact output strings for: (a) manifest with author set and empty minToolVersion (→ `min_tool_version: ~`), version always double-quoted, directories list matching `~/rpgs/tables/kal-arath/manifest.yaml` shape; (b) simple table with tags, d66 roll, quoted text containing `: ` and `{d6}` braces, plain + structured chain entries; (c) `modifier_range: [-2, 0]`; (d) notes list; (e) compound table `tables:` list; (f) `yv()` cases: `''`→`""`, `No, and`→needs quoting? (comma alone doesn't force quoting in YAML, but leading indicator/`: `/keywords do — assert `yes`→`"yes"`, `12`→`"12"`, `has: colon`→quoted, `{d6} braces`→quoted, plain `Bandits`→unquoted, backslash/quote escaping).

- [ ] **Step 2: Run → fail.**

- [ ] **Step 3: Implement** (complete — port of handoff §YAML emit):

```ts
const KEYWORDS = new Set(['true','false','yes','no','on','off','null','~','y','n']);

export function yv(s: string): string {
  const needsQuote =
    s === '' || /^\s|\s$/.test(s) ||
    /^[-?:,[\]{}#&*!|>'"%@`]/.test(s) ||
    /[{}[\]]/.test(s) ||
    s.includes(': ') || s.includes(' #') ||
    KEYWORDS.has(s.toLowerCase()) ||
    /^[+-]?\d+(\.\d+)?$/.test(s);
  if (!needsQuote) return s;
  return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

export function manifestYaml(m: ManifestState, dirs: Dir[]): string {
  const lines = [
    `name: ${yv(m.name)}`,
    `version: "${m.version.replace(/"/g, '\\"')}"`,
    `namespace: ${m.namespace}`,
    `author: ${m.author ? yv(m.author) : '~'}`,
    `min_tool_version: ${m.minToolVersion ? yv(m.minToolVersion) : '~'}`,
  ];
  if (dirs.length) {
    lines.push('directories:');
    for (const d of dirs) lines.push(`  - path: ${yv(d.path)}`, `    namespace: ${d.namespace}`);
  }
  return lines.join('\n') + '\n';
}

export function tableYaml(t: TableDraft): string {
  const lines = [`id: ${t.stem}`, `name: ${yv(t.name)}`, `type: ${t.type}`];
  if (t.tags.length) {
    lines.push('tags:');
    for (const tag of t.tags) lines.push(`  - ${yv(tag)}`);
  }
  if (t.type === 'compound') {
    lines.push('tables:');
    for (const r of t.tableRefs) lines.push(`  - ${yv(r.ref)}`);
  } else {
    lines.push(`roll: ${t.roll}`);
    if (t.modOn) lines.push(`modifier_range: [${t.modMin || 0}, ${t.modMax || 0}]`);
    if (t.notes.length) {
      lines.push('notes:');
      for (const n of t.notes) lines.push(`  - ${yv(n)}`);
    }
    lines.push('results:');
    for (const r of t.results) {
      lines.push(`  - min: ${r.min || 0}`, `    max: ${r.max || 0}`);
      if (r.text) lines.push(`    text: ${yv(r.text)}`);
      if (r.chain.length) {
        lines.push('    chain:');
        for (const c of r.chain) {
          if (c.struct) {
            lines.push(`      - table: ${yv(c.ref)}`);
            if (c.reroll.length) lines.push(`        reroll: [${c.reroll.join(', ')}]`);
          } else {
            lines.push(`      - ${yv(c.ref)}`);
          }
        }
      }
    }
  }
  return lines.join('\n') + '\n';
}

/** All files as the WASM engine / zip export consume them. */
export function collectionFiles(dirs: Dir[], tables: TableDraft[]) {
  return tables.flatMap((t) => {
    const dir = dirs.find((d) => d.id === t.dirId);
    if (!dir) return [];
    const cleanPath = dir.path.replace(/\/+$/, '');
    return [{
      path: `${cleanPath}/${t.stem}.yaml`,
      namespace: dir.namespace,
      stem: t.stem,
      contents: tableYaml(t),
    }];
  });
}
```

- [ ] **Step 4: Run → pass. Commit** `feat(webui): YAML emitter matching fatescroll file format`.

---

### Task 6: Engine bridge

**Files:**
- Create: `webui/src/engine/engine.ts`, `webui/src/engine/useEngine.ts`
- Test: `webui/tests/engine.test.ts` (interface-level, with a fake)

- [ ] **Step 1: Define the interface + WASM impl** (`engine.ts`):

```ts
export interface DiceInfo { ok: boolean; kind?: 'digit'|'range'|'simulated'; min?: number; max?: number; outcomes?: number; reason?: string }
export interface RollNode { table_name: string; roll: number | null; text: string | null; children: RollNode[] }
export interface FileInput { path: string; namespace: string; stem: string; contents: string }

export interface Engine {
  validate(manifestYaml: string, files: FileInput[]): string[];
  diceInfo(expr: string): DiceInfo;
  expectedValues(expr: string, modOn: boolean, modMin: number, modMax: number): number[] | null;
  histogram(expr: string): { iterations: number; counts: Record<string, number> } | null;
  roll(manifestYaml: string, files: FileInput[], fqid: string): RollNode | { error: string };
}
```

WASM impl: `async function initEngine(): Promise<Engine>` — calls the pkg's default `init(new URL('../wasm/pkg/fatescroll_wasm_bg.wasm', import.meta.url))`, then wraps each fn with `JSON.parse`. Seeds: `crypto.getRandomValues(new BigUint64Array(1))[0]` per call for `roll`; fixed `42n` for `histogram` (stable pills). Memoize `diceInfo`/`histogram`/`expectedValues` in a `Map` keyed by args (cleared never — expressions are tiny).

- [ ] **Step 2: `useEngine.ts`** — React context providing the Engine after async init (render "Loading engine…" until ready), plus a `useDerived()` hook: subscribes to the store, computes (150ms debounce) `{ files, manifestYaml, errors, currentYaml, currentTitle }` where `currentYaml` is `manifestYaml` for manifest/empty view or `tableYaml(selected)` for table view. Unit-test the derivation logic with a fake Engine (no WASM in jsdom).

- [ ] **Step 3: Commit** `feat(webui): engine bridge with debounced collection validation`.

---

### Task 7: Header bar + Scriptorium tree + app shell wiring

**Files:**
- Create: `webui/src/components/HeaderBar.tsx`, `webui/src/components/Scriptorium.tsx`
- Modify: `webui/src/App.tsx`
- Test: `webui/tests/components/scriptorium.test.tsx`

- [ ] **Step 1: Failing component tests** (Testing Library, fake engine ctx): tree renders manifest node + a dir header (`path/` + namespace) + table rows with `smp`/`cmp` badges; clicking a table row selects it (store `selUid` updates, row gets selected class); dir `+` adds a table to that dir; "+ add directory" adds a dir and opens manifest view; header status pill shows "Collection is valid" with 0 errors and "N error(s)" with N>0.

- [ ] **Step 2: Implement** per handoff §Header bar / §Left rail: brand block ("Fatescroll" / "TABLE FORGE" in `--font-display`), divider, COLLECTION label + manifest name, spacer, status pill (green/amber variants incl. dot glow `box-shadow: 0 0 8px`), "Export collection ▾" gold button (wired in Task 11 — until then `disabled`). Tree: manifest ⚜ node, per-dir header + `+`, indented table rows with left-accent selection, dashed "+ add directory". All colors via tokens; exact values in handoff §Screens.

- [ ] **Step 3: Tests pass. Commit** `feat(webui): header bar and scriptorium tree`.

---

### Task 8: Manifest editor

**Files:**
- Create: `webui/src/components/ManifestEditor.tsx`
- Test: `webui/tests/components/manifest-editor.test.tsx`

- [ ] **Step 1: Failing tests:** fields bound to store (name, version mono, namespace mono, author placeholder `~`, min tool version placeholder `~`); namespace input gets `.invalid` class when the collection errors include an invalid-namespace message for it (simplest: red border when `/^[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)*$/.test(ns) === false` — visual affordance only; authoritative errors still come from the engine); DIRECTORIES section: add row, edit path/namespace, delete dir with `window.confirm` when it has tables.

- [ ] **Step 2: Implement** per handoff §Manifest editor (2-col grid, 640px max, section header with `+ add`, `1fr 1fr auto` dir rows, ✕ delete). **Step 3: pass, commit** `feat(webui): manifest editor`.

---

### Task 9: Table editor (simple + compound) with autofill and probability pills

**Files:**
- Create: `webui/src/components/TableEditor.tsx`, `ResultCard.tsx`, `ChainRow.tsx`, `CompoundEditor.tsx`
- Create: `webui/src/logic/autofill.ts`, `webui/src/logic/probability.ts`
- Test: `webui/tests/autofill.test.ts`, `webui/tests/probability.test.ts`, `webui/tests/components/table-editor.test.tsx`

- [ ] **Step 1: Failing logic tests.**

`autofill.test.ts`: (a) `1d6` with 3 results → ranges `[1,2],[3,4],[5,6]`; (b) `2d6` (span 2–12, 11 values) with 3 results → sizes 4/4/3 → `[2,5],[6,9],[10,12]` (larger chunks first); (c) modifier `1d8 + [0,6]` span 1–14 with 2 results → `[1,7],[8,14]`; (d) digit values (D66) with 2 existing results → 36 rows, one per value, rows 0–1 keep their text/chain; (e) empty results → unchanged.

`probability.test.ts`: histogram `{counts:{'2':278,'3':556,...}, iterations:10000}` + range → summed pct; formatting: `0%`, `<10%` one decimal (`2.8%`), `≥10%` integer (`17%`), unparseable → `—`.

- [ ] **Step 2: Implement logic** (complete):

```ts
// autofill.ts — handoff §Auto-fill ranges, driven by engine expectedValues()
export function autofillRanges(
  results: ResultDraft[], values: number[], kind: 'digit' | 'contiguous',
): ResultDraft[] {
  if (values.length === 0) return results;
  if (kind === 'digit') {
    // one row per outcome; preserve existing text/chain by index
    return values.map((v, i) => {
      const prev = results[i];
      return {
        rid: prev?.rid ?? uid(),
        min: String(v), max: String(v),
        text: prev?.text ?? '', chain: prev?.chain ?? [],
      };
    });
  }
  const k = results.length;
  if (k === 0) return results;
  const n = values.length;
  const base = Math.floor(n / k), extra = n % k;
  let idx = 0;
  return results.map((r, i) => {
    const size = Math.min(base + (i < extra ? 1 : 0), n - idx) || 1;
    const lo = values[Math.min(idx, n - 1)];
    const hi = values[Math.min(idx + size - 1, n - 1)];
    idx += size;
    return { ...r, min: String(lo), max: String(hi) };
  });
}

// probability.ts
export function rangeProbability(
  hist: { iterations: number; counts: Record<string, number> } | null,
  min: number, max: number,
): number | null {
  if (!hist || Number.isNaN(min) || Number.isNaN(max)) return null;
  let sum = 0;
  for (const [v, c] of Object.entries(hist.counts)) {
    const n = Number(v);
    if (n >= min && n <= max) sum += c;
  }
  return sum / hist.iterations;
}
export function formatPct(p: number | null): string {
  if (p === null) return '—';
  const pct = p * 100;
  if (pct === 0) return '0%';
  return pct < 10 ? `${pct.toFixed(1)}%` : `${Math.round(pct)}%`;
}
```

- [ ] **Step 3: Failing component tests:** name/stem/type controls bound (stem sanitizes whitespace→`-`; type switch preserves the draft's other-type fields — no data loss switching simple→compound→simple); FQID line shows `namespace.stem`; roll input shows engine info line (`range 2–12 · 11 outcome(s)` / `D66 · 36 outcomes (11–66)` / `unparseable dice expression` styling per handoff); modifier checkbox disabled + unchecked when `kind === 'digit'`; modifier numeric inputs accept `-6`; Auto-fill button calls `autofillRanges` with `engine.expectedValues(...)`; result card shows probability pill; chain row `↺` toggles struct (revealing reroll input, comma-separated ints); delete table confirms; compound editor lists `◈` ref rows.

- [ ] **Step 4: Implement components** per handoff §Table editor (max-width 720px; segmented type control; result cards with gold left accent; dotted chain block; notes textarea one-per-line ↔ `notes: string[]`). Number inputs: `value` bound to raw string, `onChange` strips everything but digits and a leading `-`.

- [ ] **Step 5: All tests pass. Commit** `feat(webui): table editor with autofill, probability pills, and chains`.

---

### Task 10: Right pane — YAML viewer, validation panel, dice roller

**Files:**
- Create: `webui/src/components/RightPane.tsx`, `YamlViewer.tsx`, `ValidationPanel.tsx`, `DiceRoller.tsx`
- Test: `webui/tests/components/right-pane.test.tsx`

- [ ] **Step 1: Failing tests:** title reflects view (`MANIFEST.YAML` / `<STEM>.YAML` / `YAML`); copy button writes `currentYaml` to clipboard and flips label `⧉ copy`→`✓ copied` for 1.4s; ⬇ downloads current file; validation panel green `✓ Collection is valid.` when no errors, else one mono line per engine message with `✕` prefix; Roll button calls `engine.roll(manifest, files, fqidOf(selected))` and renders the tree flattened to `RollLine[]` (`indent*18px` padding, depth-0 dark, deeper `#6b5535`, `error` lines red); editing any field clears the output back to the placeholder (store already nulls `rollLines`).

Tree flattening:

```ts
export function flattenRoll(node: RollNode, indent = 0, out: RollLine[] = []): RollLine[] {
  const rolled = node.roll !== null ? ` (rolled ${node.roll})` : '';
  out.push({ indent, text: `${node.table_name}${rolled}${node.text ? ': ' + node.text : ''}` });
  for (const child of node.children) flattenRoll(child, indent + 1, out);
  return out;
}
```

`{error}` responses become a single `{indent: 0, error: true}` line. Roll button disabled when view !== 'table'.

- [ ] **Step 2: Implement** per handoff §Right pane (dark YAML panel `--yaml-bg`, JetBrains Mono 12.5px; VALIDATION max-height 34%; oxblood ⚄ Roll button; output panel). **Step 3: pass, commit** `feat(webui): YAML viewer, validation panel, and test roller`.

---

### Task 11: Zip export

**Files:**
- Create: `webui/src/logic/slug.ts`, `webui/src/export/zip.ts`
- Modify: `webui/src/components/HeaderBar.tsx` (enable button)
- Test: `webui/tests/slug.test.ts`, `webui/tests/zip.test.ts`

- [ ] **Step 1: Failing tests:** `collectionSlug('Kal-Arath Collection!')` → `kal-arath-collection`; zip built from a 2-dir/2-table state contains exactly `{slug}/manifest.yaml`, `{slug}/core/oracle.yaml`, `{slug}/core/weather/spring.yaml` with contents equal to the emitter output (unzip with fflate's `unzipSync` in the test).

- [ ] **Step 2: Implement:**

```ts
// slug.ts
export const collectionSlug = (name: string) =>
  name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'collection';

// zip.ts
import { zipSync, strToU8 } from 'fflate';
export function buildCollectionZip(
  slug: string, manifestYaml: string, files: FileInput[],
): Uint8Array {
  const entries: Record<string, Uint8Array> = {
    [`${slug}/manifest.yaml`]: strToU8(manifestYaml),
  };
  for (const f of files) entries[`${slug}/${f.path}`] = strToU8(f.contents);
  return zipSync(entries);
}
```

Header button: build zip → `Blob` → anchor download `<slug>.zip`. **Step 3: pass, commit** `feat(webui): collection zip export`.

---

### Task 12: Empty state + visual fidelity pass

**Files:**
- Create: `webui/src/components/EmptyState.tsx`
- Modify: any component css needing correction

- [ ] **Step 1:** Empty state per handoff §Center — Empty (✦ glyph, "Nothing selected").
- [ ] **Step 2:** Open `docs/design/table-forge/Table Forge.dc.html` and the running app side by side. Walk the handoff §Screens section top-to-bottom as a checklist (header, pill states, tree states incl. hover, both editors, right pane, buttons, borders `3px double`, `2px dotted` chain, scrollbars). Fix discrepancies.
- [ ] **Step 3:** Commit `style(webui): visual fidelity pass against design prototype`.

---

### Task 13: Golden round-trip test against the real CLI

The proof the whole stack works: state → emitter → files on disk → **actual `fatescroll` binary** validates and rolls them.

**Files:**
- Create: `webui/tests/golden-roundtrip.test.ts`

- [ ] **Step 1: Write the test** (vitest, `environment: 'node'` via `// @vitest-environment node`):

Build an in-memory state exercising every feature: two dirs (`core`, `core/weather`), a `D66` table with text interpolation `{2d6}` and a structured chain (`table: <self-ns ref>, reroll: [11]`), a `1d8` table with `modifier_range: [-2, 0]` and full envelope coverage `[-1..8]`, a compound table referencing both, tags and notes. Emit with `manifestYaml()`/`collectionFiles()`, write to `fs.mkdtempSync`, then:

```ts
import { execa } from 'execa';
const repoRoot = new URL('../../..', import.meta.url).pathname; // adjust to actual depth

const validate = await execa('cargo', ['run', '-p', 'fatescroll-cli', '--quiet', '--',
  'validate', `${tmp}/manifest.yaml`], { cwd: repoRoot, reject: false });
expect(validate.exitCode, validate.stderr + validate.stdout).toBe(0);

const roll = await execa('cargo', ['run', '-p', 'fatescroll-cli', '--quiet', '--',
  'roll', '<compound fqid>', '--manifest', `${tmp}/manifest.yaml`], { cwd: repoRoot, reject: false });
expect(roll.exitCode, roll.stderr + roll.stdout).toBe(0);
```

Check `fatescroll-cli/src/main.rs` for the actual `validate`/`roll` argument shapes before writing (e.g. whether it takes a manifest path positionally). Also add a negative case: a table with a range gap → expect exit code != 0 and stderr mentioning `gap`.

- [ ] **Step 2: Run** — `npm test` (cargo build makes first run slow; that's fine). Expect pass.
- [ ] **Step 3: Commit** `test(webui): golden round-trip through real fatescroll CLI`.

---

### Task 14: Docs + handoff

**Files:**
- Create: `webui/README.md` (prereqs, `npm run dev`, `npm test`, architecture sketch: store → emitter → WASM engine)
- Modify: `CLAUDE.md` (Build & Test: add `npm run build:wasm`, `npm test` under a webui section; architecture note for `fatescroll-wasm` + `webui/`)

- [ ] **Step 1:** Write both. **Step 2:** Full gate: `cargo test && cargo clippy -- -D warnings && cargo fmt --check && (cd webui && npm test && npm run build)`. **Step 3:** Commit `docs: webui build and architecture docs`. Close beads; report per conservative profile — merge to main only with Jerry's approval.

---

## Self-review notes

- Handoff §Dice engine is deliberately **not** ported (deltas table #1) — `dice_info`/`histogram`/`expected_values` replace `diceInfo`/`sampleRoll`; text interpolation happens inside core's roller (`roller.rs` `DICE_INTERPOLATION`), so the UI never interpolates.
- Handoff's per-view YAML/validation/roller behavior: covered by Tasks 6 + 10. Zip layout, slug, copy/download: Tasks 10–11. All §Interactions items are asserted in component tests (selection swap, rollLines clear, confirms, stem sanitize, tag split, copy animation).
- The prototype's `makeZip`/`crc32` are replaced by fflate per handoff's explicit permission ("or replace with the target platform's zip library").
- Verify during Task 2 whether `diceman::RollResult::as_numeric` returns `Option<i64>` (per commit c27ac93) and the exact `roll_with_rng` re-export path; the plan's code compiles against those assumptions and must be adjusted in-place if signatures differ.
