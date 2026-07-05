# Table Forge WebUI — Evaluator Rubric

**Role framing for the evaluator agent:** You are a reviewer judging an implementation of the Table Forge webui (a browser editor that authors fatescroll YAML collections). The target is a **personal tool for a single developer** — pragmatic, small, YAGNI — but its *output correctness is non-negotiable*: every collection it exports must load cleanly in the real `fatescroll` CLI. Judge like a Cold War Russian Olympic judge: deduct for every flaw, verify every claim empirically, and never accept "looks right" without running the check. Report findings so a developer without frontend or fatescroll background can act on them: name the file and line, quote the evidence, say what to change.

**Authoritative references (read before judging):**
- Design spec: `docs/design/table-forge/README.md` (layout, tokens, algorithms, interactions)
- Prototype: `docs/design/table-forge/Table Forge.dc.html` (open in a browser for visual comparison)
- Plan + deltas table: `docs/superpowers/plans/2026-07-05-table-forge-webui.md` — **where the design spec and `fatescroll-core` disagree, core wins** (deltas #1–#8 in the plan). Do not penalize deviations the deltas table mandates.
- Real examples: `tests/fixtures/valid-collection/`, and `~/rpgs/tables/kal-arath/` if readable.

**How to score:** Run Gate 0 first — any gate failure means the review verdict is **FAIL** regardless of dimension scores; stop and report. Otherwise score each dimension 0–5 against its anchors, multiply by its weight, sum. **Pass ≥ 80/100 with no dimension below 3.** Every deduction must cite evidence (command output, file:line, or screenshot).

---

## Gate 0 — Automatic failures (verify all, in order)

Run from the repo root:

```bash
cargo test                                   # 1. all Rust tests pass
cargo clippy -- -D warnings                  # 2. lint clean
cargo fmt --check                            # 3. formatting clean
cd webui && npm test && npm run build        # 4. webui tests + production build pass
```

5. **Round-trip gate:** the golden round-trip test (`webui/tests/golden-roundtrip.test.ts`) exists, actually invokes the compiled `fatescroll` CLI (not a mock, not the WASM engine judging itself), and passes. If it's missing or mocked, FAIL.
6. **Engine gate:** grep the webui source for reimplemented validation or dice parsing (`rg -n "d(\d+)|regex.*d6|coverage|overlap" webui/src --type ts` and read hits). If validation rules or dice grammar are duplicated in TypeScript beyond cosmetic hints (the plan permits a cosmetic namespace-format check in the manifest editor and the pure-display probability/autofill helpers driven by engine data), FAIL — the entire architecture premise is single-source-of-truth via WASM.
7. **Data-loss gate:** in the running app (`npm run dev`), create a simple table with 3 results and text, switch type to compound and back. If results/text are gone, FAIL.
8. **Git hygiene:** work is on a feature branch, commits are signed off (`git log --format='%B' | grep -c Signed-off-by` > 0), no `--no-verify` evidence, no stray files (`git status` clean).

---

## Dimension 1 — Output correctness / round-trip (weight ×6, max 30)

The product of this tool is YAML. Judge the YAML.

**Checks (perform, don't trust):**
1. In the running app, build a collection exercising: two directories (one nested, e.g. `core/weather`), a `D66` table with `{2d6}` text interpolation and a structured chain (`table:` + `reroll:`), a `1d8` table with `modifier_range` (use a negative min) and full coverage, tags, notes, and a compound table. Export the zip.
2. Unzip and run `cargo run -p fatescroll-cli -- validate <dir>/manifest.yaml` → must exit 0.
3. Run `roll` on the compound table's FQID → must exit 0 and show chained children.
4. Diff the exported `manifest.yaml` shape against `~/rpgs/tables/kal-arath/manifest.yaml` / `tests/fixtures/valid-collection/` (field order, `version` double-quoted, `~` for absent author/min_tool_version, `directories:` entries).
5. Introduce a range gap in the UI; confirm the right pane shows the core error (message containing `gap`), export anyway, and confirm CLI `validate` rejects it with the same class of error.
6. Edge quoting: result text containing `: `, leading `-`, `{d6}` braces, a bare `yes`, and a numeric-looking string `"12"` must all survive export → CLI validate → `show` without changing meaning.

**Anchors:** 5 = all six checks pass exactly. 4 = cosmetic YAML differences only (extra quoting that still parses identically). 3 = one feature emits YAML the CLI accepts but semantically differs from the UI state (e.g. reroll dropped). ≤2 = any exported collection a reasonable user builds fails CLI validation.

## Dimension 2 — Engine parity (weight ×4, max 20)

**Checks:**
1. Validation messages in the right pane are byte-identical to core `ValidationError` Display strings (compare panel text vs `fatescroll validate` stderr for the same broken collection).
2. Dice hint honesty: `2d6` → range 2–12 · 11 outcomes; `D66` → digit kind, 36 outcomes, 11–66; garbage → unparseable with core's reason. `D66` + modifier_range: checkbox disabled (delta #3).
3. Roller parity: chain depth limit and reroll behavior come from core (verify the roll path calls the WASM `roll_collection`, and a self-chaining table terminates with core's depth error rather than hanging).
4. Probability pills: for `1d6` per-value rows expect ~16–17% each; for a `2d6` `[6,8]` range expect ~44%. Sampled values within ±1.5 points are fine.
5. RNG seeding: rolls vary between clicks (crypto-seeded), histograms are stable across reloads (fixed seed).

**Anchors:** 5 = all pass. 3 = messages paraphrased instead of verbatim, or one hint case wrong. ≤2 = any validation verdict (valid/invalid) disagrees with the CLI on the same YAML.

## Dimension 3 — Visual fidelity (weight ×4, max 20)

Open the prototype HTML and the app side by side. The spec says "recreate pixel-closely"; judge against `docs/design/table-forge/README.md` §Layout, §Screens, §Design Tokens.

**Checklist (sample all; deduct per miss):** three-pane 264 / flex / 440 layout; header `#3a2f1c` with `3px double #9c7a2f` bottom border, brand in IM Fell English SC; status pill green/amber variants with glowing dot; parchment background with both radial gradients; tree selection (oxblood left-accent, `#e0d3b0` bg) and hover states; `smp`/`cmp` badges (olive / purple); result cards with `3px solid #9c7a2f` left accent; chain block `2px dotted` left border; segmented type control active state `#8b2b2b`; dark YAML panel (`#241d12`, JetBrains Mono 12.5px); oxblood Roll button; error-red borders on invalid inputs; custom scrollbars; `::selection` colors; fonts actually loaded (inspect computed styles, not just CSS declarations).

**Anchors:** 5 = a side-by-side screenshot is hard to tell apart. 4 = ≤3 minor token/spacing misses. 3 = layout right but multiple wrong colors/fonts or missing states (hover/selected). 2 = structure right, theming generic. ≤1 = layout deviates.

## Dimension 4 — Interaction behavior (weight ×3, max 15)

Per README §Interactions & Behavior. Drive the real app:

1. Selecting manifest node / dir header / table swaps center editor AND right-pane YAML target + title.
2. Any edit re-derives YAML + validation live and clears the last roll output back to the placeholder.
3. `+` flows: add directory → opens manifest view; add table → selects it; add result/chain/sub-table rows.
4. Deletes confirm (dir-with-tables cascades; table delete selects next remaining or empty state).
5. Copy button animates `⧉ copy` → `✓ copied` (~1.4s) and the clipboard holds the current YAML; ⬇ downloads the current file with the right name.
6. Stem input converts whitespace to `-`; tags split on commas; numeric inputs reject letters but accept a leading `-` for modifier bounds.
7. Auto-fill: on `2d6` with 3 rows produces contiguous non-overlapping coverage of 2–12 with larger chunks first, preserving text by index; on `D66` produces 36 per-value rows.

**Anchors:** 5 = all seven. Deduct one point per broken item; interactions that throw console errors cap the score at 2.

## Dimension 5 — Code quality & conventions (weight ×2, max 10)

1. Rust: every new/modified `.rs` starts with two `// ABOUTME:` lines; `build_registry` extraction is a pure refactor (diff `load_collection` behavior — existing tests untouched, not rewritten); no `unwrap()` on user-input paths in the wasm crate (errors return JSON, never panic across the FFI boundary).
2. TS: strict mode on; no `any` leaking through the Engine interface; components under ~200 lines with logic extracted to `logic/`/`yaml/` modules; no dead code from the prototype (e.g. an unused hand-rolled zip/crc32 port).
3. Naming: no temporal/implementation names ("NewEditor", "WasmWrapper"-style); matches CLAUDE.md naming rules.
4. DRY: YAML emission exists in exactly one place; FQID computation in exactly one place.

**Anchors:** 5 = clean on all four. 3 = isolated violations. ≤2 = duplicated domain logic or panicking WASM boundary.

## Dimension 6 — Test quality (weight ×1, max 5)

1. Unit tests exist for: emitter (incl. quoting edge cases), autofill (incl. D66 and modifier cases), probability formatting, slug, store transitions, wasm crate functions (native `cargo test -p fatescroll-wasm`).
2. Tests assert real behavior, not mocks-testing-mocks (component tests may fake the Engine; the golden test may not).
3. Failure paths are tested (unparseable dice, unresolved refs, gap detection, unknown FQID roll).
4. Test output is pristine (no expected-error noise leaking to the console unasserted).

**Anchors:** 5 = all four. 3 = happy paths only. ≤2 = emitter or autofill untested.

---

## Report format

```markdown
## Verdict: PASS | FAIL (score N/100)
## Gate 0: pass/fail per gate, with command output excerpts
## Scores
| Dimension | Score /5 | Weighted | Evidence summary |
## Findings (most severe first)
- [severity: blocker|major|minor] file:line — defect, evidence, concrete fix
## What was verified empirically vs. inspected only
```

List anything you could not verify (e.g. no browser available) explicitly — never fill gaps with assumptions.
