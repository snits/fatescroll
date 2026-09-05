# Result Expressions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` to implement task-by-task, with fresh
> implementation agents and review after each task. Track execution in kata, not
> Markdown task-status checklists.

**Goal:** Author and evaluate reusable local values and conditional result text
across FateScroll CLI and Table Forge.

**Architecture:** Core owns an expression parser/type checker/evaluator and a
result preparation/rendering boundary. YAML stores ordered source expressions;
WASM and the editor preserve them and use core for validation and rolling.

**Tech Stack:** Existing Rust workspace, serde/yaml_serde, pinned diceman v0.6.4,
wasm-bindgen, React/TypeScript/Zustand, Vitest, Rust unit and CLI integration tests.

**Status:** Proposed plan, awaiting Jerry's review of the complete design.
No implementation or RED/GREEN execution has occurred. This document gives
implementation contracts and representative executable tests, not finished code.

**Design:** [Result expressions design](../specs/2026-09-04-result-expressions-design.md)

## Global constraints

- Values are signed 64-bit integers, booleans, or strings.
- Each binding may refer to `value` and earlier bindings only.
- Dice calls are prohibited anywhere in `{= ...}` expressions.
- No additional runtime dependency is proposed.
- Expression source is limited to 4,096 UTF-8 bytes, syntax-tree depth to 64,
  and bindings per result to 128.
- For entries using bindings or strict markers, source and rendered text are
  each limited to 65,536 UTF-8 bytes.
- `roll()` dice: `[N]dS`, count 1–1,000, sides 1–1,000,000; no internal whitespace.
- Result values are local to one selected result; chains receive fresh scopes.
- Jerry approved the one-time saved-draft migration from version 1 to version 2.
- All grammar, marker escape, type, evaluation-order, diagnostic, and scope rules
  in the design are normative for every task once Jerry approves it.
- No feature release or merge until all tasks pass. Intermediate model changes
  do not constitute a working or releasable feature.
- Use TDD for every behavior: add test, observe the intended failure, implement
  only that behavior, rerun, review, commit. Compile errors alone do not establish
  behavioral RED: introduce only necessary signatures/types, then observe the
  assertion fail before filling in behavior. Do not claim unexecuted RED evidence.
- Use `git commit -s`, hooks enabled, and `Assisted-by: Codex:<actual model>`.
  Do not push or merge without Jerry's integration instruction.

## Preparation and ownership

Re-read live instructions, `kata show kmgh --agent`, Git status, and the reviewed
design. Resolve existing dirty work with Jerry before edits. Use an isolated
feature worktree through `superpowers:using-git-worktrees` for implementation.
Jerry requested linked kata tasks from these six deliverables at planning
closeout, with dependencies in order, then a pause. Those issues describe future
implementation; creating them does not authorize starting it. Keep this plan as
the contract.

Before implementation, review the complete design/plan with domain experts and
resolve findings. Each worker owns its listed files for its task, is told that
others share the codebase, and must preserve others' edits. Quote the task and
global contract in its prompt; report deviations for a focused brief review.

Run baseline `cargo test --workspace`, `cargo fmt --check`, and
`cargo clippy --workspace --all-targets -- -D warnings`; in `webui`, run
`npm run build` followed by `npm test` and `npm run lint`. Record actual outcomes.

## File boundaries and interfaces

| File | Responsibility |
| --- | --- |
| `fatescroll-core/src/expression.rs` (create) | Tokenization, expression AST, types, pure checks, checked evaluation |
| `fatescroll-core/src/result_text.rs` (create) | Binding scope, template scan, result preparation and rendering |
| `models.rs`, `error.rs`, `lib.rs` | Source data, contextual public errors, private module declarations |
| `validator.rs`, `roller.rs`, `display.rs` | Validate every entry; render selected entry; explain source in `show` |
| `fatescroll-wasm/src/lib.rs` | Verify parsed-source and roll/diagnostic JSON contracts |
| `webui/src/engine/engine.ts`, `model/types.ts` | Source bindings in parsed and draft models |
| `import/mapDrafts.ts`, `yaml/emit.ts`, `model/store.ts`, `logic/autofill.ts` | Preserve bindings and migrate saved drafts |
| `components/ResultCard.tsx`, `components/TableEditor.tsx` | Author ordered values in existing result cards |

Paths abbreviated in the table after the first row are relative to the same
crate's `src` or to `webui/src` as indicated. Exact task paths follow below.

## Task 1 — Parse and check the expression language

**Files:** Create `fatescroll-core/src/expression.rs`; modify
`fatescroll-core/src/lib.rs` to declare `#[cfg(test)] mod expression;`. Tests inline in
`expression.rs`. Source types remain internal (`pub(crate)` where shared).
The test-only module declaration stages the component before its production
caller exists; Task 4 removes that gate. Do not suppress dead-code warnings or
claim a production feature exists during these intermediate tasks.

**Consumes:** The design's grammar and dice subset; `diceman::parse` without RNG.
**Produces:**

```rust
enum ValueType { Integer, Boolean, Text }
struct Expression { /* private AST plus byte spans */ }
struct ExpressionError { offset: usize, reason: String }
type TypeScope = std::collections::BTreeMap<String, ValueType>;
fn parse(source: &str) -> Result<Expression, ExpressionError>;
fn check(expr: &Expression, scope: &TypeScope, allow_roll: bool)
    -> Result<ValueType, ExpressionError>;
```

The AST comments above describe opaque internals, not unspecified public fields.
Derive `Debug` and `PartialEq` on value/type/error data needed for assertions.

1. Add a first parser/checker test, introduce only compiling signatures, and
   observe its failure:

```rust
#[test]
fn expression_checks_conditional_types() {
    let scope = TypeScope::from([("count".into(), ValueType::Integer)]);
    let expr = parse(r#"if count == 1 then "gem" else "gems""#).unwrap();
    assert_eq!(check(&expr, &scope, false).unwrap(), ValueType::Text);
    let invalid = parse(r#"if true then 1 else "gems""#).unwrap();
    assert!(check(&invalid, &scope, false).is_err());
}
```

2. Run `cargo test -p fatescroll-core expression_checks_conditional_types`.
3. Implement only enough tokenizer/parser/checker behavior for the preceding
   failing assertion. Use a tokenizer retaining byte offsets and recursive-descent
   precedence functions matching the EBNF as the eventual structure, but do not
   fill unused operators or validation branches before their test cycles.
4. Add/run one RED→GREEN cycle at a time for precedence and parentheses, whole-token keywords,
   rejection of repeated same-level comparisons, accepted mixed precedence,
   nonboolean conditions even in dead branches, trailing junk, Unicode strings/offsets,
   each supported/unsupported escape, unknown names, operator type mismatches,
   both-branch checking, malformed/out-of-range integers, depth/source limits,
   valid bounded `[N]dS`, zero/oversized dice even in dead branches, and all
   prohibited dice syntax. Pin the root-at-1 parser nesting convention separately
   from AST depth, including discarded group parentheses.
   Add bounded AST construction in its own failing-test cycle: calculate depth
   before attaching a node, and check source bytes before tokenization. For each
   case, implement only the behavior its failing assertion requires. A test that
   already passes is regression coverage, never retroactively labeled RED.
   A dice node stores a checked literal plus parsed diceman expression; no
   string-based dice code generation. Reject dice anywhere when `allow_roll=false`.
5. Run `cargo test -p fatescroll-core expression::tests` and relevant lint/format
   checks. Review and commit as `feat: parse and check result expressions`.

## Task 2 — Evaluate with checked arithmetic and one RNG

**Files:** Modify `fatescroll-core/src/expression.rs`; tests inline.
**Consumes:** Task 1 types, checked AST, and `diceman::Rng`.
**Produces:**

```rust
enum Value { Integer(i64), Boolean(bool), Text(String) }
type ValueScope = std::collections::BTreeMap<String, Value>;
fn evaluate(expr: &Expression, scope: &ValueScope, rng: &mut impl diceman::Rng)
    -> Result<Value, ExpressionError>;
```

1. Add this test and observe behavioral RED with a compiling evaluator boundary:

```rust
#[test]
fn expression_skips_unselected_arithmetic() {
    let mut rng = diceman::FastRng::with_seed(19);
    let expr = parse("if true then 7 else 1 / 0").unwrap();
    assert_eq!(evaluate(&expr, &ValueScope::new(), &mut rng).unwrap(),
               Value::Integer(7));
    let bad = parse("9223372036854775807 + 1").unwrap();
    assert!(evaluate(&bad, &ValueScope::new(), &mut rng).is_err());
}
```

2. Run `cargo test -p fatescroll-core expression_skips_unselected_arithmetic`.
3. Implement only the currently failing behavior (selected conditional evaluation
   and checked addition for the shown assertions). Keep other operators for
   their own cycles. Conditional control chooses a branch before evaluating it.
4. Cycle tests, implementing only after each intended failure, for each operator,
   negative division/remainder, zero divisors,
   `i64::MIN / -1`, `i64::MIN % -1`, negation overflow, lazy `&&`/`||`, string and
   boolean equality, pure-expression RNG checkpoints, and left-to-right dice
   order. Use real `FastRng` instances with identical seeds: evaluate one side,
   manually roll the specified dice with diceman on the other, then compare the
   next real dice result/checkpoint. A lazy dice branch must not advance state.
   Use `checked_add/sub/mul/div/rem/neg` as each operation is introduced. Add
   distinct failing tests before missing/wrong-type runtime-scope error handling,
   boolean short-circuiting, or diceman parsed-AST evaluation. Never create or
   reseed an RNG inside evaluation. Already-passing cases are regression evidence.
5. Run `cargo test -p fatescroll-core expression::tests`, review, and commit
   `feat: evaluate local formulas with checked arithmetic`.

## Task 3 — Prepare result bindings and templates

**Files:** Create `fatescroll-core/src/result_text.rs`; modify
`fatescroll-core/src/{lib,models}.rs`; update every `ResultEntry` literal found
by `rg -n 'ResultEntry\s*\{' fatescroll-core fatescroll-cli fatescroll-wasm`.
Tests inline in models/result_text. Mechanical literal changes are owned by this
worker across the workspace; coordinate before any concurrent source work.
Declare `#[cfg(test)] mod result_text;` until Task 4 adds production consumers.
Preparation tests inspect the ordered checked bindings and segment contents;
keep the intermediate unit-test build warning-free without lint suppression.

**Consumes:** Tasks 1–2 parser/checker; existing optional result text and chains.
**Produces:**

```rust
// Public source model in models.rs:
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultBinding { pub name: String, pub value: String }
// Field on ResultEntry:
#[serde(rename = "let", default, skip_serializing_if = "Vec::is_empty")]
pub bindings: Vec<ResultBinding>,

// Internal result_text boundary:
struct PreparedResult { /* ordered checked bindings and source segments */ }
struct ResultTextError { location: String, offset: usize, reason: String }
fn prepare(entry: &ResultEntry) -> Result<PreparedResult, ResultTextError>;
```

1. Start with serde RED proving nonempty bindings survive serialization; then
   add the field and update literals to `bindings: vec![]`. Explicitly test
   omitted/empty/null `let`, unknown/duplicate binding fields, non-string values,
   and both YAML and JSON round trips. If serde coerces scalars into strings,
   add a string-only binding-value deserializer and discriminating tests.
2. Add and fail preparation tests for ordered dependencies and unknown names:

```rust
#[test]
fn result_text_rejects_forward_references() {
    let entry: ResultEntry = serde_yaml::from_str(r#"
min: 1
max: 6
let:
  - name: price
    value: 'count * 25'
  - name: count
    value: 'roll("1d4")'
text: '{= price}'
"#).unwrap();
    let error = prepare(&entry).unwrap_err();
    assert!(error.reason.contains("count"));
    assert!(error.location.contains("price"));
}
```

3. Run `cargo test -p fatescroll-core result_text_rejects_forward_references`.
4. Implement only the tested forward-reference boundary: seed the type scope
   with integer `value` and check each binding before adding its name. Keep
   decoded expression offsets and binding locations in the resulting errors.
5. Cycle tests before implementing duplicate/self/reserved names, all binding types, pure marker
   without bindings, dice prohibition even in dead branches, literal/quoted
   braces, marker precedence in `{note {= value}}`, literal `{{1d6}}`, an escaped
   marker followed by an actual marker, empty/unterminated markers, absent versus
   empty text, and all preparation limits. Introduce template scanning in these
   cycles: strict openings partition source before ordinary dice matching,
   quotes/escapes determine closing braces, and `{{=` takes priority. Check
   template expressions with all bindings and dice disabled. Preparation should
   not require any RNG argument. Already-passing cases are regression coverage.
6. Run `cargo test -p fatescroll-core` and compile/test the workspace after the
   model changes. Review and commit `feat: prepare result bindings and templates`.

## Task 4 — Integrate validation, rendering, CLI, and WASM

**Files:** Modify `fatescroll-core/src/{result_text,validator,roller,error,display}.rs`,
`fatescroll-cli/tests/cli_integration.rs`, `fatescroll-wasm/src/lib.rs`.
Modify `fatescroll-core/src/lib.rs` to make `expression` and `result_text`
unconditional private modules when their validator/roller callers are introduced.
Create `tests/fixtures/result-expressions/{manifest,gems}.yaml`. Inspect CLI output
consumers in `fatescroll-cli/src/main.rs`; change only if contextual errors require
wiring, preserving the current output modes.

**Consumes:** `prepare`, `evaluate`, selected lookup, and the caller's RNG.
**Produces:**

```rust
fn render(prepared: &PreparedResult, lookup: i32, rng: &mut impl diceman::Rng)
    -> Result<Option<String>, ResultTextError>;
// Public contextual variants, fields are source strings and zero-based index:
// ValidationError::InvalidResultExpression { table, entry: usize, location,
//                                          offset: usize, reason }
// RollError::ResultExpression { table, entry: usize, location,
//                              offset: usize, reason }
```

1. Write a render RED test using a deserialized entry with count `roll("1d1")`,
   price `count * 25`, and the design's conditional template. After `prepare`,
   `render(..., 6, ...)` must equal `Some("Found 1 gem worth 25 gold.")`.
2. Run `cargo test -p fatescroll-core result_text`; implement ordered evaluation
   into a fresh value scope, inserting integer `value`. Render source segments
   once; add checked output-budget accounting in a separate failing-limit-test
   cycle. Extract the existing
   ordinary-dice helper into this module with its tests and preserve behavior.
3. Add distinct RED→GREEN integration cycles: validator prepares every row,
   including unselected invalid rows; roller prepares and renders the selected
   row before chains; display prints source bindings without RNG. Find selected
   row index directly during selection, without comparing cloned rows. Map errors
   with table identity and display entry indices one-based. Do not reject valid
   but unreachable runtime branches by constant-folding evaluation at validation.
   Call `prepare` in `validate_table`'s enumerated result loop so errors have
   real entry indices. Preserve the public range-only
   `validate_result_entry(entry, table_name)` helper and its signature.
4. Pin deterministic RNG semantics: count used twice draws once; unused bindings
   still draw; rejected table rerolls never evaluate bindings; ordinary dice
   render after bindings; a string containing `{1d6}` is not rescanned. Check
   missing/empty text, boolean rendering, chosen runtime errors, and failure
   before child RNG use. Compare with explicit real diceman calls, not mocked
   result values. Test local scope in chains, repeated self-chain visits within
   current depth limits, and compound siblings with identical binding names.
5. Verify built-in `value` with direct `--value`, clamped modifier lookup,
   negative modified lookup, and D66. No new CLI flags or RollResult fields.
6. Create a complete real fixture (the implementation uses deterministic count):

```yaml
# manifest.yaml
name: Result Expressions
version: "1.0"
namespace: expressions
directories:
  - path: .
    namespace: expressions
```

```yaml
# gems.yaml
id: gems
name: Gems
type: simple
roll: 1d6
results:
  - min: 1
    max: 6
    let:
      - name: count
        value: 'roll("1d1")'
      - name: price
        value: 'count * 25'
    text: 'Found {= count} {= if count == 1 then "gem" else "gems"} worth {= price} gold.'
```

7. CLI tests run the real binary: `validate`, `show`, `roll --value 1`, `--quiet`,
   and `--json`. Assert exact text and JSON keys, exit statuses, and captured
   stdout/stderr for invalid syntax and runtime division by zero. WASM tests
   parse source bindings, report validation diagnostics, and return the same
   text/tree through `roll_collection` with a fixed seed. Preserve the current
   WASM invalid-table behavior; test its actual envelope rather than assuming
   CLI-identical error shapes.
8. Run `cargo test --workspace`, format/lint checks. Review and commit
   `feat: render result expressions across core roll surfaces`.

## Task 5 — Preserve expressions through Table Forge data flows

**Files:** Modify `webui/src/engine/engine.ts`, `webui/src/model/{types,store}.ts`,
`webui/src/import/mapDrafts.ts`, `webui/src/yaml/emit.ts`,
`webui/src/logic/autofill.ts`, and row constructors in
`webui/src/components/TableEditor.tsx`. Tests:
`webui/tests/{map-drafts,emit,autofill,import-roundtrip,golden-roundtrip}.test.ts`
and `webui/tests/store-persistence.test.ts` plus `webui/tests/store.test.ts`.
This worker also owns mechanical `bindings: []` additions and affected exact
assertions across all `webui/tests`, including result literals in
`webui/tests/components/table-editor.test.tsx`. TypeScript's build includes tests;
do not leave these fixture repairs for Task 6.

**Consumes:** Parsed Rust JSON `let?: Array<{ name: string; value: string }>`.
**Produces:**

```typescript
export interface BindingDraft { rid: string; name: string; value: string }
// ResultDraft gains: bindings: BindingDraft[]
// ParsedResult gains: let?: { name: string; value: string }[]
```

1. Add a round-trip failure: parse the core fixture through real WASM, map to
   drafts, emit YAML, parse emitted YAML through real WASM, and assert the ordered
   `let` list is equal. Include values `"1"`, `"true"`, quotes, backslashes, and
   newline escapes so YAML cannot coerce expression sources into other types.
2. Run the focused Vitest case after `npm run build:wasm` in `webui`. Add the types,
   mapping with fresh row IDs, empty defaults in all constructors, and YAML `let`
   emission using the existing string quoting function. Never convert expression
   strings into JS numbers. Add separate failing preservation tests before
   updating digit or contiguous autofill. Import gives binding rows fresh IDs;
   no duplicate/clone UI is added.
3. Add persistence RED before changing its version: a version-1 stored document
   must rehydrate with its original tables/text/chains/selection and empty binding
   arrays. Implement the approved v1→v2 migration, checking nested container
   shapes before traversal. Cycle tests for malformed v1 data, unknown versions,
   version-2 binding roundtrip, no preview restoration, and pristine console output.
4. Add import/export tests retaining invalid expression strings for repair;
   structural errors must still fail. Exercise ZIP export and real CLI validation
   in `golden-roundtrip.test.ts`; ensure no missing `let` fields or reordered values.
5. Run relevant Vitest suites, `npm run build`, `npm run lint`, and review.
   Commit `feat: preserve local values in Table Forge documents`.

## Task 6 — Author values and finish end-to-end verification

**Files:** Modify `webui/src/components/ResultCard.tsx`,
`webui/src/components/TableEditor.tsx`, `webui/src/styles/components.css` only as
needed for existing styling, and `webui/tests/components/table-editor.test.tsx`.
Update `docs/authoring-guide.md`, `README.md`, and `webui/README.md`.

**Consumes:** `BindingDraft[]`, normal editor update flow, real core diagnostics.
**Produces:** Add/edit/remove/reorder named-value controls in each result card;
text help for strict expressions; documented working grammar and examples.

1. Write an authoring failure using the rendered editor and store: add `count`
   with `roll("1d1")`, add `price` with `count * 25`, enter the conditional template,
   and assert emitted YAML contains both bindings in order. Labels include the
   row identity for accessible Name/Expression controls.
2. Run the focused component test. Implement the Values section and source
   editing needed by that test using the existing result update callback.
   Use text inputs for expressions and escape output through normal React text.
3. Add a failing interaction test before each move/remove control and its
   move-up/down bounds. Cycle tests for reorder causing a forward-reference diagnostic, fixing it,
   deletion causing an unknown-name diagnostic, stale preview clearing, and
   reopening exported documents. Component interaction tests may use existing
   UI harnesses; evaluator acceptance must use real WASM and real CLI, with no
   mocked engine results standing in for language behavior.
4. Add/run a real-engine acceptance test: import → edit bindings → export → reopen
   → validate → roll produces `Found 1 gem worth 25 gold.`. Include two bindings
   with the same spelling in different results and prove edits do not leak.
5. Document the design's grammar, `value` meaning, source order, local scope,
   lazy conditions, roll subset, integer errors, marker escape/reserved syntax,
   bounds, and migration. Show `roll("2d6") * 10` and the acceptance collection.
   Explain that ordinary dice are tolerant and strict expression errors fail.
6. Run final `cargo test --workspace`, `cargo fmt --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`; in `webui` run
   `npm run build`, `npm test`, `npm run lint`. Review captured output, not just
   exit codes. No full-suite pass is claimed by this planning document.
7. Perform spec coverage and fresh-eyes review, then review the final branch with
   roborev per project policy. Address findings, commit with sign-off/attribution,
   and present verified evidence for Jerry's integration decision. Leave kmgh
   open until its agreed implementation scope is actually verified.

## Review checklist and handoff

For plan review, trace each acceptance requirement to Tasks 1–6. Pay special
attention to lazy evaluation versus static checking (1–2), markers/generated text
(3–4), RNG/local scope (4), persisted data and autofill (5), and actual user
authoring/roundtrip (6). Review both bounded AST construction and later traversal.

The execution default is already set by Jerry's project instructions:
subagent-driven development. Once the design is approved and implementation is
requested, execute this plan in order; no further execution-mode selection needed.
