# Table Forge Open/Import Collection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Open control to Table Forge that loads a fatescroll collection from a zip or a folder back into the editor, so work can be exported, reloaded, and continued.

**Architecture:** TypeScript ingest normalizes zip/folder input to `{manifestYaml, files:[{path, contents}]}`; a new wasm `parse_collection` does discovery + parsing with fatescroll-core's serde models (single source of truth); TS maps the parsed JSON onto fresh editor drafts and a new `loadCollection` store action replaces state wholesale. All-or-nothing: any parse/structure error aborts with the full error list.

**Tech Stack:** Rust (fatescroll-wasm, wasm-bindgen, serde), TypeScript/React 19, Zustand, fflate, Vitest + Testing Library, execa (CLI round-trip tests).

**Spec:** `docs/superpowers/specs/2026-07-05-webui-open-collection-design.md`
**Bead:** fatescroll-1z7 · **Branch:** `feat/webui-open-collection`

**Conventions that apply to every task:**
- Every new `.ts`/`.tsx`/`.rs` file starts with two `// ABOUTME:` comment lines.
- Commits: `git commit -s` and end the message with `Assisted-by: Claude:<model-id>` (use the actual model id of the executing agent).
- Rust tests: `cargo test -p fatescroll-wasm` (plain `#[test]`s run natively). Lint: `cargo clippy -- -D warnings`, `cargo fmt`.
- Webui tests: `cd webui && npx vitest run <file>`; full suite `npm test`. Lint: `npm run lint`.
- Tasks 4+ need a rebuilt wasm pkg: `cd webui && npm run build:wasm` (requires `wasm-pack`).

---

### Task 1: Fix compound-table notes emission (pre-existing round-trip gap)

Core's `Table::Compound` has a `notes` field, but `tableYaml` only emits
`notes:` for simple tables. An open → edit → export cycle would silently drop
compound notes.

**Files:**
- Modify: `webui/src/yaml/emit.ts` (compound branch of `tableYaml`, ~line 81)
- Test: `webui/tests/emit.test.ts`

- [ ] **Step 1: Write the failing test**

Add to `webui/tests/emit.test.ts` (reuse the file's existing draft-building
helpers if present; otherwise use this literal):

```ts
test('compound table emits notes', () => {
  const t: TableDraft = {
    uid: 'u1',
    dirId: 'd1',
    stem: 'combo',
    name: 'Combo',
    type: 'compound',
    tags: [],
    roll: '1d6',
    modOn: false,
    modMin: '',
    modMax: '',
    notes: ['Roll all sub-tables together'],
    results: [],
    tableRefs: [{ rid: 'r1', ref: 'oracle' }],
  };
  expect(tableYaml(t)).toBe(
    'id: combo\nname: Combo\ntype: compound\nnotes:\n  - Roll all sub-tables together\ntables:\n  - oracle\n',
  );
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd webui && npx vitest run tests/emit.test.ts`
Expected: FAIL — emitted YAML lacks the `notes:` block.

- [ ] **Step 3: Emit notes in the compound branch**

In `webui/src/yaml/emit.ts`, change the compound branch of `tableYaml`:

```ts
  if (t.type === 'compound') {
    if (t.notes.length) {
      lines.push('notes:');
      for (const n of t.notes) lines.push(`  - ${yv(n)}`);
    }
    if (t.tableRefs.length) {
      lines.push('tables:');
      for (const r of t.tableRefs) lines.push(`  - ${yv(r.ref)}`);
    }
  } else {
```

(`notes` before `tables`, matching the field order of core's
`Table::Compound`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd webui && npx vitest run tests/emit.test.ts`
Expected: PASS (all tests in the file).

- [ ] **Step 5: Commit**

```bash
git add webui/src/yaml/emit.ts webui/tests/emit.test.ts
git commit -s -m "fix(webui): emit notes on compound tables"
```

---

### Task 2: wasm `parse_collection` — discovery + happy path

**Files:**
- Modify: `fatescroll-wasm/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `fatescroll-wasm/src/lib.rs`:

```rust
    const OPEN_MANIFEST: &str = "name: T\nversion: \"1.0\"\nnamespace: t\nauthor: ~\nmin_tool_version: ~\ndirectories:\n  - path: core/\n    namespace: t.core\n  - path: core/deep\n    namespace: t.core.deep\n";

    const ORACLE_YAML: &str = "id: oracle\nname: Oracle\ntype: simple\nroll: 1d6\nresults:\n  - min: 1\n    max: 6\n    text: \"Yes\"\n";

    #[test]
    fn parse_collection_discovers_and_parses() {
        let files = serde_json::json!([
            {"path": "core/oracle.yaml", "contents": ORACLE_YAML},
            {"path": "core/deep/combo.yaml", "contents": "id: combo\nname: Combo\ntype: compound\ntables:\n  - oracle\n"},
            {"path": "core/deep/nested/too-deep.yaml", "contents": "id: too-deep\nname: X\ntype: compound\ntables: []\n"},
            {"path": "elsewhere/stray.yaml", "contents": "id: stray\nname: X\ntype: compound\ntables: []\n"},
            {"path": "core/readme.txt", "contents": "not yaml"},
            {"path": "core/other.manifest.yaml", "contents": "name: nested\n"}
        ])
        .to_string();
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, &files)).unwrap();
        assert!(out.get("errors").is_none(), "unexpected errors: {out}");

        assert_eq!(out["manifest"]["name"], "T");
        assert_eq!(out["manifest"]["namespace"], "t");
        assert!(out["manifest"]["author"].is_null());
        let dirs = out["manifest"]["directories"].as_array().unwrap();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0]["path"], "core/");
        assert_eq!(dirs[0]["namespace"], "t.core");

        // Discovery is non-recursive: nested/too-deep.yaml and elsewhere/
        // stray.yaml are not loaded; *.manifest.yaml and non-yaml are skipped.
        let tables = out["tables"].as_array().unwrap();
        assert_eq!(tables.len(), 2);
        let oracle = tables.iter().find(|t| t["stem"] == "oracle").unwrap();
        assert_eq!(oracle["path"], "core/oracle.yaml");
        assert_eq!(oracle["namespace"], "t.core");
        assert_eq!(oracle["table"]["type"], "simple");
        assert_eq!(oracle["table"]["roll"], "1d6");
        assert_eq!(oracle["table"]["results"][0]["text"], "Yes");
        let combo = tables.iter().find(|t| t["stem"] == "combo").unwrap();
        assert_eq!(combo["namespace"], "t.core.deep");
        assert_eq!(combo["table"]["tables"][0], "oracle");

        // Unloaded yaml is warned about; *.manifest.yaml inside a listed dir
        // is silently skipped (mirrors on-disk discovery), non-yaml ignored.
        let ignored: Vec<&str> = out["ignored_yaml"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            ignored,
            vec!["core/deep/nested/too-deep.yaml", "elsewhere/stray.yaml"]
        );
    }

    #[test]
    fn parse_collection_serializes_chain_forms() {
        let files = serde_json::json!([{
            "path": "core/a.yaml",
            "contents": "id: a\nname: A\ntype: simple\nroll: 1d4\nmodifier_range: [0, 2]\nresults:\n  - min: 1\n    max: 6\n    text: X\n    chain:\n      - plain-ref\n      - table: a\n        reroll: [1]\n"
        }])
        .to_string();
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, &files)).unwrap();
        assert!(out.get("errors").is_none(), "unexpected errors: {out}");
        let table = &out["tables"][0]["table"];
        assert_eq!(table["modifier_range"], serde_json::json!([0, 2]));
        let chain = table["results"][0]["chain"].as_array().unwrap();
        assert_eq!(chain[0], "plain-ref");
        assert_eq!(chain[1]["table"], "a");
        assert_eq!(chain[1]["reroll"], serde_json::json!([1]));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p fatescroll-wasm parse_collection`
Expected: COMPILE ERROR — `parse_collection` not defined.

- [ ] **Step 3: Implement `parse_collection`**

Add to `fatescroll-wasm/src/lib.rs` (after `parse_inputs`; add
`use fatescroll_core::models::Table;` to the existing `models` import):

```rust
#[derive(Deserialize)]
struct RawFile {
    path: String,
    contents: String,
}

/// `"core/"` and `"."` normalize to `"core"` / `""` so zip paths compare
/// against manifest directory entries verbatim.
fn normalize_dir(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed == "." { "" } else { trimmed }
}

/// `"core/oracle.yaml"` -> `("core", "oracle.yaml")`; `"oracle.yaml"` -> `("", "oracle.yaml")`.
fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn is_yaml_name(name: &str) -> bool {
    name.ends_with(".yaml") || name.ends_with(".yml")
}

fn is_manifest_name(name: &str) -> bool {
    name == "manifest.yaml" || name.ends_with(".manifest.yaml")
}

/// Parse a whole collection held in memory for import into the editor.
/// Takes ALL ingested files; discovery mirrors the on-disk loader
/// (collection.rs): non-recursive per `directories:` entry, `.yaml`/`.yml`
/// only, manifests skipped. Tables must carry an `id` matching the filename
/// stem, exactly as `build_registry` enforces. Does NOT run validate_table —
/// semantically invalid tables load fine and get fixed in the editor.
/// Duplicate-FQID detection is not mirrored either (the editor's live
/// validate_collection catches collisions after load). Directory paths must
/// be plain relative segments ("core/deep"); "./core" forms won't match.
/// Returns {"manifest": .., "tables": [{path, namespace, stem, table}],
/// "ignored_yaml": [..]} or {"errors": [String]} (all-or-nothing).
#[wasm_bindgen]
pub fn parse_collection(manifest_yaml: &str, files_json: &str) -> String {
    let manifest: Manifest = match serde_yaml::from_str(manifest_yaml) {
        Ok(m) => m,
        Err(e) => return json!({ "errors": [format!("manifest: {e}")] }).to_string(),
    };
    if !manifest.files.is_empty() {
        return json!({ "errors": ["manifest: `files:` entries are not supported by Table Forge"] })
            .to_string();
    }
    let raw_files: Vec<RawFile> = match serde_json::from_str(files_json) {
        Ok(v) => v,
        Err(e) => return json!({ "errors": [format!("files: {e}")] }).to_string(),
    };

    let mut errors: Vec<String> = Vec::new();
    let mut tables: Vec<serde_json::Value> = Vec::new();
    let mut matched = vec![false; raw_files.len()];

    for dir in &manifest.directories {
        let dir_path = dir.path.to_string_lossy();
        let dir_norm = normalize_dir(&dir_path);
        for (i, f) in raw_files.iter().enumerate() {
            let (parent, name) = split_parent(&f.path);
            if parent != dir_norm || !is_yaml_name(name) {
                continue;
            }
            matched[i] = true;
            if is_manifest_name(name) {
                continue;
            }
            let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
            match serde_yaml::from_str::<Table>(&f.contents) {
                Ok(table) => {
                    if table.id() != stem {
                        errors.push(format!(
                            "{}: table id '{}' does not match filename '{}'",
                            f.path,
                            table.id(),
                            stem
                        ));
                    } else {
                        tables.push(json!({
                            "path": f.path,
                            "namespace": dir.namespace,
                            "stem": stem,
                            "table": table,
                        }));
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", f.path)),
            }
        }
    }

    if !errors.is_empty() {
        return json!({ "errors": errors }).to_string();
    }

    let ignored_yaml: Vec<&str> = raw_files
        .iter()
        .enumerate()
        .filter(|(i, f)| !matched[*i] && is_yaml_name(&f.path))
        .map(|(_, f)| f.path.as_str())
        .collect();

    let directories: Vec<serde_json::Value> = manifest
        .directories
        .iter()
        .map(|d| json!({ "path": d.path.to_string_lossy(), "namespace": d.namespace }))
        .collect();

    json!({
        "manifest": {
            "name": manifest.name,
            "version": manifest.version,
            "namespace": manifest.namespace,
            "author": manifest.author,
            "min_tool_version": manifest.min_tool_version,
            "directories": directories,
        },
        "tables": tables,
        "ignored_yaml": ignored_yaml,
    })
    .to_string()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p fatescroll-wasm`
Expected: PASS (all wasm crate tests, including the two new ones).

- [ ] **Step 5: Lint and commit**

```bash
cargo clippy -p fatescroll-wasm -- -D warnings && cargo fmt
git add fatescroll-wasm/src/lib.rs
git commit -s -m "feat(wasm): parse_collection for importing collections"
```

---

### Task 3: wasm `parse_collection` — error cases (all-or-nothing)

**Files:**
- Modify: `fatescroll-wasm/src/lib.rs` (tests only; implementation from Task 2 should already satisfy them — if not, fix it here)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests`:

```rust
    #[test]
    fn parse_collection_rejects_bad_manifest_yaml() {
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(": not [ yaml", "[]")).unwrap();
        let errs = out["errors"].as_array().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].as_str().unwrap().starts_with("manifest:"));
    }

    #[test]
    fn parse_collection_rejects_files_entries() {
        let manifest = "name: T\nversion: \"1.0\"\nnamespace: t\nauthor: ~\nmin_tool_version: ~\nfiles:\n  - path: a.yaml\n    namespace: t\n";
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(manifest, "[]")).unwrap();
        let errs = out["errors"].as_array().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].as_str().unwrap().contains("not supported"));
    }

    #[test]
    fn parse_collection_rejects_malformed_files_json() {
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, "{ not json")).unwrap();
        assert!(out["errors"][0].as_str().unwrap().starts_with("files:"));
    }

    #[test]
    fn parse_collection_accumulates_all_errors() {
        // All-or-nothing: one bad table plus one id mismatch plus one missing
        // id -> three errors, no manifest/tables keys in the envelope.
        let files = serde_json::json!([
            {"path": "core/bad.yaml", "contents": ": not [ yaml"},
            {"path": "core/mismatch.yaml", "contents": "id: other\nname: X\ntype: compound\ntables: []\n"},
            {"path": "core/no-id.yaml", "contents": "name: X\ntype: compound\ntables: []\n"},
            {"path": "core/oracle.yaml", "contents": ORACLE_YAML}
        ])
        .to_string();
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, &files)).unwrap();
        let errs = out["errors"].as_array().unwrap();
        assert_eq!(errs.len(), 3, "expected 3 errors, got: {errs:?}");
        assert!(errs.iter().any(|e| e.as_str().unwrap().starts_with("core/bad.yaml:")));
        assert!(errs.iter().any(|e| e.as_str().unwrap().contains("does not match filename")));
        assert!(errs.iter().any(|e| e.as_str().unwrap().starts_with("core/no-id.yaml:")));
        assert!(out.get("manifest").is_none());
        assert!(out.get("tables").is_none());
    }

    #[test]
    fn parse_collection_does_not_validate_tables() {
        // A range gap is a validation error, not a parse error: the table
        // must import so the editor can fix it (ValidationPanel shows it).
        let files = serde_json::json!([{
            "path": "core/gappy.yaml",
            "contents": "id: gappy\nname: Gappy\ntype: simple\nroll: 1d6\nresults:\n  - min: 1\n    max: 2\n    text: Low\n  - min: 5\n    max: 6\n    text: High\n"
        }])
        .to_string();
        let out: serde_json::Value =
            serde_json::from_str(&parse_collection(OPEN_MANIFEST, &files)).unwrap();
        assert!(out.get("errors").is_none(), "unexpected errors: {out}");
        assert_eq!(out["tables"].as_array().unwrap().len(), 1);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p fatescroll-wasm parse_collection`
Expected: PASS if Task 2's implementation is correct; otherwise fix the
implementation (not the tests) until green.

- [ ] **Step 3: Lint and commit**

```bash
cargo clippy -p fatescroll-wasm -- -D warnings && cargo fmt
git add fatescroll-wasm/src/lib.rs
git commit -s -m "test(wasm): parse_collection error handling is all-or-nothing"
```

---

### Task 4: Engine binding — `Engine.parseCollection`

**Files:**
- Modify: `webui/src/engine/engine.ts`
- Modify: `webui/tests/engine.test.tsx` (wrapEngine coverage)
- Modify: every test file defining a fake `Engine` (they will fail to typecheck): `webui/tests/components/empty-state.test.tsx`, `headerbar.test.tsx`, `manifest-editor.test.tsx`, `right-pane.test.tsx`, `scriptorium.test.tsx`, `table-editor.test.tsx` — check for others with `grep -rln "Engine" webui/tests/`

- [ ] **Step 1: Rebuild the wasm pkg**

Run: `cd webui && npm run build:wasm`
Expected: `src/wasm/pkg/fatescroll_wasm.d.ts` now declares `parse_collection`.

- [ ] **Step 2: Write the failing test**

Add to `webui/tests/engine.test.tsx` (match the file's existing fake-raw-engine pattern):

```ts
describe('parseCollection', () => {
  const parsedEnvelope = JSON.stringify({
    manifest: {
      name: 'T', version: '1.0', namespace: 't',
      author: null, min_tool_version: null,
      directories: [{ path: 'core', namespace: 't.core' }],
    },
    tables: [{
      path: 'core/oracle.yaml', namespace: 't.core', stem: 'oracle',
      table: {
        type: 'simple', id: 'oracle', name: 'Oracle', tags: [], notes: [],
        roll: '1d6', modifier_range: null,
        results: [{ min: 1, max: 6, text: 'Yes', chain: null }],
      },
    }],
    ignored_yaml: ['stray.yaml'],
  });

  it('returns ok with collection and camelCased ignoredYaml', () => {
    const raw = { ...makeRawEngine(), parse_collection: () => parsedEnvelope };
    const engine = wrapEngine(raw);
    const outcome = engine.parseCollection('manifest', [{ path: 'core/oracle.yaml', contents: 'x' }]);
    if (!outcome.ok) throw new Error('expected ok');
    expect(outcome.collection.manifest.namespace).toBe('t');
    expect(outcome.collection.tables[0].stem).toBe('oracle');
    expect(outcome.collection.ignoredYaml).toEqual(['stray.yaml']);
  });

  it('returns errors envelope as not-ok', () => {
    const raw = { ...makeRawEngine(), parse_collection: () => JSON.stringify({ errors: ['manifest: bad'] }) };
    const outcome = wrapEngine(raw).parseCollection('m', []);
    expect(outcome).toEqual({ ok: false, errors: ['manifest: bad'] });
  });
});
```

(If `engine.test.tsx` has no reusable `makeRawEngine`, add a local helper
returning stub implementations of every `RawEngine` method.)

- [ ] **Step 3: Run test to verify it fails**

Run: `cd webui && npx vitest run tests/engine.test.tsx`
Expected: FAIL — `parseCollection` / `parse_collection` don't exist on the types.

- [ ] **Step 4: Implement the binding**

In `webui/src/engine/engine.ts`, add the parsed-collection types and wire the
method:

```ts
export interface ParsedChainModified {
  table: string;
  reroll: number[];
}
export type ParsedChain = string | ParsedChainModified;

export interface ParsedResult {
  min: number;
  max: number;
  text: string | null;
  chain: ParsedChain[] | null;
}

export interface ParsedTable {
  type: 'simple' | 'compound';
  id: string;
  name: string;
  tags: string[];
  notes: string[];
  roll?: string;
  modifier_range?: [number, number] | null;
  results?: ParsedResult[];
  tables?: string[];
}

export interface ParsedFile {
  path: string;
  namespace: string;
  stem: string;
  table: ParsedTable;
}

export interface ParsedManifest {
  name: string;
  version: string;
  namespace: string;
  author: string | null;
  min_tool_version: string | null;
  directories: { path: string; namespace: string }[];
}

export interface ParsedCollection {
  manifest: ParsedManifest;
  tables: ParsedFile[];
  ignoredYaml: string[];
}

export type ParseOutcome =
  | { ok: true; collection: ParsedCollection }
  | { ok: false; errors: string[] };
```

Extend the interfaces:

```ts
export interface Engine {
  // ...existing methods...
  parseCollection(manifestYaml: string, files: { path: string; contents: string }[]): ParseOutcome;
}

export interface RawEngine {
  // ...existing methods...
  parse_collection(manifestYaml: string, filesJson: string): string;
}
```

And in `wrapEngine`'s returned object:

```ts
    parseCollection(manifestYaml, files) {
      const parsed = JSON.parse(raw.parse_collection(manifestYaml, JSON.stringify(files))) as
        | { errors: string[] }
        | { manifest: ParsedManifest; tables: ParsedFile[]; ignored_yaml: string[] };
      if ('errors' in parsed) return { ok: false, errors: parsed.errors };
      return {
        ok: true,
        collection: {
          manifest: parsed.manifest,
          tables: parsed.tables,
          ignoredYaml: parsed.ignored_yaml,
        },
      };
    },
```

- [ ] **Step 5: Update every fake Engine in tests**

Each `makeFakeEngine()` (or equivalent) needs:

```ts
    parseCollection: () => ({ ok: false, errors: ['not used'] }),
```

Find them with `grep -rn "roll: () =>" webui/tests/`.

- [ ] **Step 6: Run the full webui suite**

Run: `cd webui && npm test`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add webui/src/engine/engine.ts webui/tests/
git commit -s -m "feat(webui): engine binding for parse_collection"
```

---

### Task 5: Ingest — zip and folder normalization

**Files:**
- Create: `webui/src/import/ingest.ts`
- Test: `webui/tests/ingest.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `webui/tests/ingest.test.ts`:

```ts
// ABOUTME: Tests for import ingest: locating manifest.yaml in zip/folder
// ABOUTME: entries, rebasing paths, and zip/FileList entry extraction.

import { describe, expect, it } from 'vitest';
import { strToU8, zipSync } from 'fflate';
import { entriesFromZip, ingest, isYamlPath } from '../src/import/ingest';

describe('ingest', () => {
  it('accepts manifest.yaml at the root', () => {
    const raw = ingest([
      { path: 'manifest.yaml', contents: 'name: T' },
      { path: 'core/oracle.yaml', contents: 'id: oracle' },
    ]);
    expect(raw.manifestYaml).toBe('name: T');
    expect(raw.files).toEqual([{ path: 'core/oracle.yaml', contents: 'id: oracle' }]);
  });

  it('accepts manifest.yaml inside exactly one top-level directory and rebases paths', () => {
    const raw = ingest([
      { path: 'my-tables/manifest.yaml', contents: 'name: T' },
      { path: 'my-tables/core/oracle.yaml', contents: 'id: oracle' },
    ]);
    expect(raw.manifestYaml).toBe('name: T');
    expect(raw.files).toEqual([{ path: 'core/oracle.yaml', contents: 'id: oracle' }]);
  });

  it('drops entries outside the manifest root', () => {
    const raw = ingest([
      { path: 'my-tables/manifest.yaml', contents: 'name: T' },
      { path: '__MACOSX/junk.yaml', contents: '' },
    ]);
    expect(raw.files).toEqual([]);
  });

  it('rejects zero manifests', () => {
    expect(() => ingest([{ path: 'core/oracle.yaml', contents: '' }])).toThrow(/no manifest\.yaml/i);
  });

  it('rejects multiple candidate manifests', () => {
    expect(() =>
      ingest([
        { path: 'a/manifest.yaml', contents: '' },
        { path: 'b/manifest.yaml', contents: '' },
      ]),
    ).toThrow(/multiple/i);
  });

  it('does not treat deeper manifests as candidates', () => {
    const raw = ingest([
      { path: 'manifest.yaml', contents: 'name: T' },
      { path: 'a/b/manifest.yaml', contents: 'name: nested' },
    ]);
    expect(raw.manifestYaml).toBe('name: T');
    expect(raw.files).toEqual([{ path: 'a/b/manifest.yaml', contents: 'name: nested' }]);
  });
});

describe('entriesFromZip', () => {
  it('extracts yaml entries and skips directories and non-yaml', () => {
    const zip = zipSync({
      'kal/manifest.yaml': strToU8('name: K'),
      'kal/core/oracle.yaml': strToU8('id: oracle'),
      'kal/readme.txt': strToU8('nope'),
    });
    const entries = entriesFromZip(zip);
    expect(entries).toEqual([
      { path: 'kal/manifest.yaml', contents: 'name: K' },
      { path: 'kal/core/oracle.yaml', contents: 'id: oracle' },
    ]);
  });
});

describe('entriesFromFileList', () => {
  function fakeFile(rel: string, contents: string): File {
    const f = new File([contents], rel.slice(rel.lastIndexOf('/') + 1));
    Object.defineProperty(f, 'webkitRelativePath', { value: rel });
    return f;
  }

  it('strips the picked folder name and reads only yaml files', async () => {
    const list = [
      fakeFile('kal/manifest.yaml', 'name: K'),
      fakeFile('kal/core/oracle.yaml', 'id: oracle'),
      fakeFile('kal/readme.txt', 'nope'),
    ] as unknown as FileList;
    expect(await entriesFromFileList(list)).toEqual([
      { path: 'manifest.yaml', contents: 'name: K' },
      { path: 'core/oracle.yaml', contents: 'id: oracle' },
    ]);
  });
});

describe('isYamlPath', () => {
  it('accepts .yaml and .yml, rejects others', () => {
    expect(isYamlPath('a/b.yaml')).toBe(true);
    expect(isYamlPath('a/b.yml')).toBe(true);
    expect(isYamlPath('a/b.txt')).toBe(false);
  });
});
```

(Import `entriesFromFileList` alongside the other ingest imports. This test
needs jsdom — the file must NOT carry a `@vitest-environment node` comment.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd webui && npx vitest run tests/ingest.test.ts`
Expected: FAIL — module `../src/import/ingest` does not exist.

- [ ] **Step 3: Implement ingest**

Create `webui/src/import/ingest.ts`:

```ts
// ABOUTME: Normalizes user-picked zip archives or folders into collection
// ABOUTME: entries (manifest YAML + relative file paths) for the wasm parser.

import { unzipSync } from 'fflate';

export interface CollectionEntry {
  path: string;
  contents: string;
}

export interface RawCollection {
  manifestYaml: string;
  files: CollectionEntry[];
}

export function isYamlPath(path: string): boolean {
  return path.endsWith('.yaml') || path.endsWith('.yml');
}

function basename(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

function depth(path: string): number {
  return path.split('/').length - 1;
}

/** Locate manifest.yaml at the root, or inside exactly one top-level
 * directory (the layout buildCollectionZip produces), and rebase all entry
 * paths relative to it. Entries outside the manifest's directory are
 * dropped. Throws with a user-facing message when no unambiguous manifest
 * exists. */
export function ingest(entries: CollectionEntry[]): RawCollection {
  const candidates = entries.filter((e) => basename(e.path) === 'manifest.yaml' && depth(e.path) <= 1);
  if (candidates.length === 0) {
    throw new Error('No manifest.yaml found at the collection root.');
  }
  if (candidates.length > 1) {
    throw new Error(
      `Multiple manifests found (${candidates.map((c) => c.path).join(', ')}); open a single collection.`,
    );
  }
  const manifest = candidates[0];
  const prefix = manifest.path.slice(0, -'manifest.yaml'.length);
  const files = entries
    .filter((e) => e !== manifest && e.path.startsWith(prefix))
    .map((e) => ({ path: e.path.slice(prefix.length), contents: e.contents }));
  return { manifestYaml: manifest.contents, files };
}

/** YAML entries from a zip archive; directory entries and non-YAML skipped. */
export function entriesFromZip(data: Uint8Array): CollectionEntry[] {
  const decoder = new TextDecoder();
  return Object.entries(unzipSync(data))
    .filter(([path]) => !path.endsWith('/') && isYamlPath(path))
    .map(([path, bytes]) => ({ path, contents: decoder.decode(bytes) }));
}

/** YAML entries from a webkitdirectory FileList; the picked folder's own
 * name (the first webkitRelativePath segment) is stripped so the manifest
 * sits at the root. */
export async function entriesFromFileList(list: FileList): Promise<CollectionEntry[]> {
  const files = Array.from(list).filter((f) => isYamlPath(f.name));
  return Promise.all(
    files.map(async (f) => ({
      path: f.webkitRelativePath.split('/').slice(1).join('/'),
      contents: await f.text(),
    })),
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd webui && npx vitest run tests/ingest.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add webui/src/import/ingest.ts webui/tests/ingest.test.ts
git commit -s -m "feat(webui): ingest zip/folder input into collection entries"
```

---

### Task 6: Map parsed collection onto editor drafts

**Files:**
- Create: `webui/src/import/mapDrafts.ts`
- Test: `webui/tests/map-drafts.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `webui/tests/map-drafts.test.ts`:

```ts
// ABOUTME: Tests for mapDrafts: parsed wasm collection JSON onto ManifestState,
// ABOUTME: Dir, and TableDraft models with fresh uids and stringified numbers.

import { describe, expect, it } from 'vitest';
import type { ParsedCollection } from '../src/engine/engine';
import { mapDrafts } from '../src/import/mapDrafts';

function parsed(): ParsedCollection {
  return {
    manifest: {
      name: 'Kal-Arath Collection',
      version: '1.0',
      namespace: 'kal-arath',
      author: null,
      min_tool_version: null,
      directories: [
        { path: 'core/', namespace: 'kal-arath.core' },
        { path: 'core/weather', namespace: 'kal-arath.core.weather' },
      ],
    },
    tables: [
      {
        path: 'core/oracle.yaml',
        namespace: 'kal-arath.core',
        stem: 'oracle',
        table: {
          type: 'simple',
          id: 'oracle',
          name: 'Oracle',
          tags: ['divination'],
          notes: ['Ask a yes/no question'],
          roll: '2d6',
          modifier_range: [0, 3],
          results: [
            {
              min: 2,
              max: 12,
              text: 'Yes, and [1d4]',
              chain: ['plain-ref', { table: 'oracle', reroll: [2] }],
            },
          ],
        },
      },
      {
        path: 'core/weather/storms.yaml',
        namespace: 'kal-arath.core.weather',
        stem: 'storms',
        table: {
          type: 'compound',
          id: 'storms',
          name: 'Storms',
          tags: [],
          notes: [],
          tables: ['wind', 'rain'],
        },
      },
    ],
    ignoredYaml: [],
  };
}

describe('mapDrafts', () => {
  it('maps manifest with null author/min_tool_version to empty strings', () => {
    const { manifest } = mapDrafts(parsed());
    expect(manifest).toEqual({
      name: 'Kal-Arath Collection',
      version: '1.0',
      namespace: 'kal-arath',
      author: '',
      minToolVersion: '',
    });
  });

  it('maps directories to Dirs with fresh distinct ids', () => {
    const { dirs } = mapDrafts(parsed());
    expect(dirs.map((d) => ({ path: d.path, namespace: d.namespace }))).toEqual([
      { path: 'core/', namespace: 'kal-arath.core' },
      { path: 'core/weather', namespace: 'kal-arath.core.weather' },
    ]);
    expect(dirs[0].id).not.toBe(dirs[1].id);
  });

  it('maps a simple table with modifier, results, and both chain forms', () => {
    const { dirs, tables } = mapDrafts(parsed());
    const oracle = tables.find((t) => t.stem === 'oracle')!;
    expect(oracle.dirId).toBe(dirs[0].id); // trailing-slash dir matches 'core' parent
    expect(oracle.type).toBe('simple');
    expect(oracle.roll).toBe('2d6');
    expect(oracle.modOn).toBe(true);
    expect(oracle.modMin).toBe('0');
    expect(oracle.modMax).toBe('3');
    expect(oracle.tags).toEqual(['divination']);
    expect(oracle.notes).toEqual(['Ask a yes/no question']);
    const r = oracle.results[0];
    expect(r.min).toBe('2');
    expect(r.max).toBe('12');
    expect(r.text).toBe('Yes, and [1d4]');
    expect(r.chain[0]).toMatchObject({ struct: false, ref: 'plain-ref', reroll: [] });
    expect(r.chain[1]).toMatchObject({ struct: true, ref: 'oracle', reroll: [2] });
  });

  it('maps a compound table to tableRefs with draft defaults for simple-only fields', () => {
    const { dirs, tables } = mapDrafts(parsed());
    const storms = tables.find((t) => t.stem === 'storms')!;
    expect(storms.dirId).toBe(dirs[1].id);
    expect(storms.type).toBe('compound');
    expect(storms.tableRefs.map((r) => r.ref)).toEqual(['wind', 'rain']);
    expect(storms.roll).toBe('1d6');
    expect(storms.modOn).toBe(false);
    expect(storms.results).toEqual([]);
  });

  it('maps null result text to empty string', () => {
    const p = parsed();
    p.tables[0].table.results![0].text = null;
    p.tables[0].table.results![0].chain = null;
    const { tables } = mapDrafts(p);
    expect(tables[0].results[0].text).toBe('');
    expect(tables[0].results[0].chain).toEqual([]);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd webui && npx vitest run tests/map-drafts.test.ts`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Implement mapDrafts**

Create `webui/src/import/mapDrafts.ts`:

```ts
// ABOUTME: Maps a parsed collection (wasm parse_collection JSON) onto the
// ABOUTME: editor's draft models, assigning fresh uids/rids.

import type {
  ParsedChain,
  ParsedCollection,
  ParsedFile,
  ParsedResult,
} from '../engine/engine';
import { uid } from '../model/ids';
import type { ChainDraft, Dir, ManifestState, ResultDraft, TableDraft } from '../model/types';

export interface LoadedState {
  manifest: ManifestState;
  dirs: Dir[];
  tables: TableDraft[];
}

// Mirrors normalize_dir in fatescroll-wasm: "core/" -> "core", "." -> "".
const normPath = (p: string) => {
  const trimmed = p.replace(/\/+$/, '');
  return trimmed === '.' ? '' : trimmed;
};

function mapChain(c: ParsedChain): ChainDraft {
  if (typeof c === 'string') return { rid: uid(), struct: false, ref: c, reroll: [] };
  return { rid: uid(), struct: true, ref: c.table, reroll: c.reroll };
}

function mapResult(r: ParsedResult): ResultDraft {
  return {
    rid: uid(),
    min: String(r.min),
    max: String(r.max),
    text: r.text ?? '',
    chain: (r.chain ?? []).map(mapChain),
  };
}

function mapTable(f: ParsedFile, dirs: Dir[]): TableDraft {
  const parent = f.path.includes('/') ? f.path.slice(0, f.path.lastIndexOf('/')) : '';
  const dir = dirs.find((d) => normPath(d.path) === parent && d.namespace === f.namespace);
  const t = f.table;
  const mod = t.modifier_range ?? null;
  return {
    uid: uid(),
    dirId: dir?.id ?? '',
    stem: f.stem,
    name: t.name,
    type: t.type,
    tags: t.tags,
    roll: t.roll ?? '1d6',
    modOn: mod !== null,
    modMin: mod ? String(mod[0]) : '',
    modMax: mod ? String(mod[1]) : '',
    notes: t.notes,
    results: (t.results ?? []).map(mapResult),
    tableRefs: (t.tables ?? []).map((ref) => ({ rid: uid(), ref })),
  };
}

export function mapDrafts(parsed: ParsedCollection): LoadedState {
  const m = parsed.manifest;
  const manifest: ManifestState = {
    name: m.name,
    version: m.version,
    namespace: m.namespace,
    author: m.author ?? '',
    minToolVersion: m.min_tool_version ?? '',
  };
  const dirs: Dir[] = m.directories.map((d) => ({ id: uid(), path: d.path, namespace: d.namespace }));
  const tables = parsed.tables.map((f) => mapTable(f, dirs));
  return { manifest, dirs, tables };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd webui && npx vitest run tests/map-drafts.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add webui/src/import/mapDrafts.ts webui/tests/map-drafts.test.ts
git commit -s -m "feat(webui): map parsed collections onto editor drafts"
```

---

### Task 7: Store action `loadCollection`

**Files:**
- Modify: `webui/src/model/store.ts`
- Test: `webui/tests/store.test.ts`

- [ ] **Step 1: Write the failing test**

Add to `webui/tests/store.test.ts` (follow the file's existing setup/reset
pattern):

```ts
describe('loadCollection', () => {
  it('replaces manifest/dirs/tables and resets selection to the manifest view', () => {
    const store = useForgeStore;
    store.getState().addDir();
    const dirId = store.getState().dirs[0].id;
    store.getState().addTable(dirId);
    store.getState().setRollLines([{ indent: 0, text: 'old roll' }]);

    const manifest = {
      name: 'Kal-Arath',
      version: '2.0',
      namespace: 'kal-arath',
      author: 'Jerry',
      minToolVersion: '',
    };
    const dirs = [{ id: 'nd1', path: 'core', namespace: 'kal-arath.core' }];
    const tables = [
      {
        uid: 'nt1',
        dirId: 'nd1',
        stem: 'oracle',
        name: 'Oracle',
        type: 'simple' as const,
        tags: [],
        roll: '1d6',
        modOn: false,
        modMin: '',
        modMax: '',
        notes: [],
        results: [],
        tableRefs: [],
      },
    ];
    store.getState().loadCollection({ manifest, dirs, tables });

    const s = store.getState();
    expect(s.manifest).toEqual(manifest);
    expect(s.dirs).toEqual(dirs);
    expect(s.tables).toEqual(tables);
    expect(s.view).toBe('manifest');
    expect(s.selUid).toBeNull();
    expect(s.rollLines).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd webui && npx vitest run tests/store.test.ts`
Expected: FAIL — `loadCollection` is not a function.

- [ ] **Step 3: Implement the action**

In `webui/src/model/store.ts`, add to the `ForgeState` interface:

```ts
  loadCollection(data: { manifest: ManifestState; dirs: Dir[]; tables: TableDraft[] }): void;
```

And to the store object (after `deleteTable`):

```ts
  loadCollection: (data) =>
    set({ ...data, view: 'manifest', selUid: null, rollLines: null }),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd webui && npx vitest run tests/store.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add webui/src/model/store.ts webui/tests/store.test.ts
git commit -s -m "feat(webui): loadCollection store action"
```

---

### Task 8: HeaderBar Open control — UI and wiring

**Files:**
- Modify: `webui/src/components/HeaderBar.tsx`
- Modify: `webui/src/styles/components.css` (open-menu styles)
- Test: `webui/tests/components/headerbar.test.tsx`

- [ ] **Step 1: Wrap existing bare HeaderBar renders in an EngineProvider**

The first `describe('HeaderBar')` block in
`webui/tests/components/headerbar.test.tsx` renders `<HeaderBar />` bare
(~8 tests); only the later `AppContent wiring` block uses the `wrapper()`
helper. This task adds `useEngine()` to HeaderBar, which throws without an
`EngineProvider` ancestor — so those bare renders must be wrapped first.
Hoist the `wrapper`/`makeFakeEngine` helpers above the HeaderBar describe
block and pass `{ wrapper: wrapper(makeFakeEngine([])) }` to each bare
`render(...)` call (the existing helper signature is
`makeFakeEngine(errors: string[])`). Run the file's tests — they still pass
before HeaderBar changes.

- [ ] **Step 2: Write the failing tests**

Add to `webui/tests/components/headerbar.test.tsx`, following the file's
helpers. New tests:

```ts
describe('open collection', () => {
  function zipWith(entries: Record<string, string>): File {
    const bytes = zipSync(
      Object.fromEntries(Object.entries(entries).map(([p, c]) => [p, strToU8(c)])),
    );
    return new File([bytes.slice().buffer], 'collection.zip', { type: 'application/zip' });
  }

  const parsedOutcome = {
    ok: true as const,
    collection: {
      manifest: {
        name: 'Imported',
        version: '1.0',
        namespace: 'imp',
        author: null,
        min_tool_version: null,
        directories: [{ path: 'core', namespace: 'imp.core' }],
      },
      tables: [],
      ignoredYaml: [],
    },
  };

  beforeEach(() => {
    useForgeStore.setState(initialState());
    vi.restoreAllMocks();
  });

  it('opens a zip: ingests entries, parses via engine, loads the store', async () => {
    const parseCollection = vi.fn().mockReturnValue(parsedOutcome);
    const engine = { ...makeFakeEngine([]), parseCollection };
    render(<HeaderBar collectionName="New Collection" errorCount={0} />, {
      wrapper: wrapper(engine),
    });

    fireEvent.click(screen.getByRole('button', { name: /open collection/i }));
    const input = screen.getByTestId('open-zip-input');
    fireEvent.change(input, {
      target: {
        files: [
          zipWith({
            'imp/manifest.yaml': 'name: Imported',
            'imp/core/oracle.yaml': 'id: oracle',
          }),
        ],
      },
    });

    await waitFor(() => expect(parseCollection).toHaveBeenCalled());
    expect(parseCollection).toHaveBeenCalledWith('name: Imported', [
      { path: 'core/oracle.yaml', contents: 'id: oracle' },
    ]);
    await waitFor(() => expect(useForgeStore.getState().manifest.name).toBe('Imported'));
    expect(useForgeStore.getState().view).toBe('manifest');
  });

  it('shows parse errors and leaves state untouched', async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const engine = {
      ...makeFakeEngine([]),
      parseCollection: () => ({ ok: false as const, errors: ['core/bad.yaml: mapping error'] }),
    };
    render(<HeaderBar collectionName="New Collection" errorCount={0} />, {
      wrapper: wrapper(engine),
    });

    fireEvent.click(screen.getByRole('button', { name: /open collection/i }));
    fireEvent.change(screen.getByTestId('open-zip-input'), {
      target: { files: [zipWith({ 'manifest.yaml': 'name: X' })] },
    });

    await waitFor(() => expect(alert).toHaveBeenCalled());
    expect(alert.mock.calls[0][0]).toContain('core/bad.yaml: mapping error');
    expect(useForgeStore.getState().manifest.name).toBe('New Collection');
  });

  it('asks for confirmation when tables exist and aborts on cancel', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    useForgeStore.getState().addDir();
    useForgeStore.getState().addTable(useForgeStore.getState().dirs[0].id);
    const parseCollection = vi.fn().mockReturnValue(parsedOutcome);
    const engine = { ...makeFakeEngine([]), parseCollection };
    render(<HeaderBar collectionName="New Collection" errorCount={0} />, {
      wrapper: wrapper(engine),
    });

    fireEvent.click(screen.getByRole('button', { name: /open collection/i }));
    fireEvent.change(screen.getByTestId('open-zip-input'), {
      target: { files: [zipWith({ 'manifest.yaml': 'name: X' })] },
    });

    await waitFor(() => expect(confirm).toHaveBeenCalled());
    expect(parseCollection).not.toHaveBeenCalled();
    expect(useForgeStore.getState().tables).toHaveLength(1);
  });
});
```

Imports needed at the top of the test file: `strToU8, zipSync` from
`fflate`; `waitFor` from `@testing-library/react`; `initialState,
useForgeStore` from `../../src/model/store` (some already present).

Note: jsdom's `File` supports `.arrayBuffer()`; the `fireEvent.change`
pattern with `target.files` is standard for file inputs.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd webui && npx vitest run tests/components/headerbar.test.tsx`
Expected: FAIL — no "open collection" button.

- [ ] **Step 4: Implement the Open control**

Rewrite `webui/src/components/HeaderBar.tsx` additions (keep everything
existing; new imports, state, handlers, and markup):

```tsx
import { useRef, useState } from 'react';
import { useEngine } from '../engine/useEngine';
import { entriesFromFileList, entriesFromZip, ingest, type CollectionEntry } from '../import/ingest';
import { mapDrafts } from '../import/mapDrafts';
```

Inside the component:

```tsx
  const { engine } = useEngine();
  const [openMenu, setOpenMenu] = useState(false);
  const zipInputRef = useRef<HTMLInputElement>(null);
  const folderInputRef = useRef<HTMLInputElement>(null);

  function openFromEntries(entries: CollectionEntry[]) {
    const { tables } = useForgeStore.getState();
    if (
      tables.length > 0 &&
      !window.confirm('Replace the current collection? Unexported work will be lost.')
    ) {
      return;
    }
    let raw;
    try {
      raw = ingest(entries);
    } catch (err) {
      window.alert(String(err instanceof Error ? err.message : err));
      return;
    }
    const outcome = engine.parseCollection(raw.manifestYaml, raw.files);
    if (!outcome.ok) {
      const shown = outcome.errors.slice(0, 10);
      const more = outcome.errors.length - shown.length;
      window.alert(
        `Cannot open collection:\n${shown.join('\n')}${more > 0 ? `\n…and ${more} more` : ''}`,
      );
      return;
    }
    if (outcome.collection.ignoredYaml.length > 0) {
      window.alert(
        `Opened with warnings — these YAML files are not listed in the manifest and will not be part of exports:\n${outcome.collection.ignoredYaml.join('\n')}`,
      );
    }
    // mapDrafts throws if a parsed table matches no manifest directory — a
    // drifted-contract guard against silent data loss on re-export.
    try {
      useForgeStore.getState().loadCollection(mapDrafts(outcome.collection));
    } catch (err) {
      window.alert(String(err instanceof Error ? err.message : err));
    }
  }

  async function handleZipPicked(e: React.ChangeEvent<HTMLInputElement>) {
    setOpenMenu(false);
    const file = e.target.files?.[0];
    e.target.value = '';
    if (!file) return;
    openFromEntries(entriesFromZip(new Uint8Array(await file.arrayBuffer())));
  }

  async function handleFolderPicked(e: React.ChangeEvent<HTMLInputElement>) {
    setOpenMenu(false);
    const list = e.target.files;
    if (!list || list.length === 0) return;
    const entries = await entriesFromFileList(list);
    e.target.value = '';
    openFromEntries(entries);
  }
```

Markup, before the export button:

```tsx
      <div className="header-open">
        <button type="button" className="header-export" onClick={() => setOpenMenu((o) => !o)}>
          Open collection ▾
        </button>
        {openMenu && (
          <div className="header-open-menu" role="menu">
            <button type="button" role="menuitem" onClick={() => zipInputRef.current?.click()}>
              From zip…
            </button>
            <button type="button" role="menuitem" onClick={() => folderInputRef.current?.click()}>
              From folder…
            </button>
          </div>
        )}
        <input
          ref={zipInputRef}
          data-testid="open-zip-input"
          type="file"
          accept=".zip"
          hidden
          onChange={handleZipPicked}
        />
        <input
          ref={(el) => {
            folderInputRef.current = el;
            if (el) el.webkitdirectory = true;
          }}
          data-testid="open-folder-input"
          type="file"
          hidden
          onChange={handleFolderPicked}
        />
      </div>
```

(`webkitdirectory` is a real `HTMLInputElement` DOM property in lib.dom but
is absent from @types/react's `InputHTMLAttributes`, so the JSX-attribute
form fails `tsc -b` — set it via the callback ref instead.)

Add to `webui/src/styles/components.css`, near the header styles (match the
file's existing formatting and color values — reuse the export button's
palette):

```css
.header-open {
  position: relative;
}

.header-open-menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  z-index: 10;
  display: flex;
  flex-direction: column;
  min-width: 160px;
  border: 1px solid #c0a76c;
  background: #f7f0dc;
}

.header-open-menu button {
  padding: 8px 12px;
  border: none;
  background: none;
  text-align: left;
  cursor: pointer;
}

.header-open-menu button:hover {
  background: #eadfbf;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd webui && npx vitest run tests/components/headerbar.test.tsx`
Expected: PASS.

- [ ] **Step 6: Full suite + lint**

Run: `cd webui && npm test && npm run lint`
Expected: PASS, no lint errors.

- [ ] **Step 7: Commit**

```bash
git add webui/src/components/HeaderBar.tsx webui/src/styles/components.css webui/tests/components/headerbar.test.tsx
git commit -s -m "feat(webui): Open collection from zip or folder"
```

---

### Task 9: Round-trip integration tests

Two proofs: (A) a real on-disk fixture collection imports and re-exports to
something the real CLI validates; (B) export → import → export is
byte-identical.

**Files:**
- Create: `webui/tests/import-roundtrip.test.ts`

- [ ] **Step 1: Write the tests**

Create `webui/tests/import-roundtrip.test.ts` (modeled on
`golden-roundtrip.test.ts` — same node environment, CLI-build `beforeAll`,
tmpdir cleanup):

```ts
// @vitest-environment node
// ABOUTME: Round-trip proof for import: a real fixture collection imports via
// ABOUTME: wasm parse_collection, and export -> import -> export is byte-identical.

import { describe, test, expect, beforeAll, afterAll } from 'vitest';
import { execa } from 'execa';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import initWasm, * as rawWasm from '../src/wasm/pkg/fatescroll_wasm.js';
import { wrapEngine, type Engine, type RawEngine } from '../src/engine/engine';
import { ingest, isYamlPath, type CollectionEntry } from '../src/import/ingest';
import { mapDrafts } from '../src/import/mapDrafts';
import { collectionFiles, manifestYaml } from '../src/yaml/emit';

const webuiRoot = fileURLToPath(new URL('..', import.meta.url));
const repoRoot = path.join(webuiRoot, '..');
const bin = path.join(repoRoot, 'target/debug/fatescroll');

let engine: Engine;

beforeAll(async () => {
  const bytes = fs.readFileSync(path.join(webuiRoot, 'src/wasm/pkg/fatescroll_wasm_bg.wasm'));
  await initWasm({ module_or_path: bytes });
  engine = wrapEngine(rawWasm as unknown as RawEngine);
  await execa('cargo', ['build', '-p', 'fatescroll-cli'], { cwd: repoRoot });
}, 300_000);

const tmpDirs: string[] = [];
afterAll(() => {
  for (const dir of tmpDirs) fs.rmSync(dir, { recursive: true, force: true });
});

function readCollectionEntries(root: string): CollectionEntry[] {
  const entries: CollectionEntry[] = [];
  for (const p of fs.readdirSync(root, { recursive: true, encoding: 'utf8' })) {
    const full = path.join(root, p);
    const rel = p.split(path.sep).join('/');
    if (fs.statSync(full).isFile() && isYamlPath(rel)) {
      entries.push({ path: rel, contents: fs.readFileSync(full, 'utf8') });
    }
  }
  return entries;
}

function writeCollection(dir: string, manifest: string, files: { path: string; contents: string }[]) {
  fs.writeFileSync(path.join(dir, 'manifest.yaml'), manifest);
  for (const f of files) {
    const target = path.join(dir, f.path);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, f.contents);
  }
}

describe('import round-trip', () => {
  test('valid-collection fixture imports and re-exports to a CLI-valid collection', async () => {
    const fixture = path.join(repoRoot, 'tests/fixtures/valid-collection');
    const raw = ingest(readCollectionEntries(fixture));
    const outcome = engine.parseCollection(raw.manifestYaml, raw.files);
    if (!outcome.ok) throw new Error(`import failed: ${outcome.errors.join('; ')}`);
    expect(outcome.collection.tables.length).toBeGreaterThanOrEqual(7);

    const { manifest, dirs, tables } = mapDrafts(outcome.collection);
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'fatescroll-import-'));
    tmpDirs.push(tmp);
    writeCollection(tmp, manifestYaml(manifest, dirs), collectionFiles(dirs, tables));

    const result = await execa(bin, ['validate', '--collection', tmp], { reject: false });
    expect(result.exitCode, `${result.stdout}\n${result.stderr}`).toBe(0);
  }, 60_000);

  test('export -> import -> export is byte-identical', () => {
    const manifest = {
      name: 'Round Trip',
      version: '1.0',
      namespace: 'rt',
      author: 'Jerry',
      minToolVersion: '',
    };
    const dirs = [
      { id: 'd1', path: 'core/', namespace: 'rt.core' },
      { id: 'd2', path: 'core/deep', namespace: 'rt.core.deep' },
    ];
    const tables = [
      {
        uid: 'u1',
        dirId: 'd1',
        stem: 'oracle',
        name: 'Oracle: "quoted"',
        type: 'simple' as const,
        tags: ['divination', 'true'],
        roll: '2d6',
        modOn: true,
        modMin: '0',
        modMax: '3',
        notes: ['Ask a question'],
        results: [
          {
            rid: 'r1',
            min: '2',
            max: '15',
            text: 'Yes, and [1d4] omens',
            chain: [
              { rid: 'c1', struct: false, ref: 'portent', reroll: [] },
              { rid: 'c2', struct: true, ref: 'oracle', reroll: [2, 3] },
            ],
          },
        ],
        tableRefs: [],
      },
      {
        uid: 'u2',
        dirId: 'd2',
        stem: 'portent',
        name: 'Portent',
        type: 'compound' as const,
        tags: [],
        roll: '1d6',
        modOn: false,
        modMin: '',
        modMax: '',
        notes: ['Roll everything'],
        results: [],
        tableRefs: [{ rid: 'p1', ref: 'oracle' }],
      },
    ];

    const manifest1 = manifestYaml(manifest, dirs);
    const files1 = collectionFiles(dirs, tables);

    const outcome = engine.parseCollection(
      manifest1,
      files1.map((f) => ({ path: f.path, contents: f.contents })),
    );
    if (!outcome.ok) throw new Error(`import failed: ${outcome.errors.join('; ')}`);
    const loaded = mapDrafts(outcome.collection);

    expect(manifestYaml(loaded.manifest, loaded.dirs)).toBe(manifest1);
    expect(collectionFiles(loaded.dirs, loaded.tables)).toEqual(files1);
  });
});
```

Note: `initWasm({ module_or_path: bytes })` is the non-deprecated init form
for current wasm-bindgen; if the generated `fatescroll_wasm.js` predates it,
`initWasm(bytes)` works too (deprecation warning only). The web-target pkg
runs fine in Node once bytes are passed explicitly (no `fetch` involved).

Note: `collectionFiles` emits table paths as `core/oracle.yaml` (trailing
slash on the dir is trimmed at emission), so `parseCollection` sees paths
whose parent matches the manifest's `core/` entry after normalization — this
is exactly the trailing-slash case Task 2 covers.

- [ ] **Step 2: Run the tests**

Run: `cd webui && npx vitest run tests/import-roundtrip.test.ts`
Expected: PASS. If the fixture test fails, the failure list pinpoints which
loader rule diverges — fix the implementation (Tasks 2/5/6), never the
assertion.

- [ ] **Step 3: Full suites**

Run: `cargo test && cd webui && npm test && npm run lint`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add webui/tests/import-roundtrip.test.ts
git commit -s -m "test(webui): import round-trip against fixture and byte-identical re-export"
```

---

### Task 10: Documentation

**Files:**
- Modify: `webui/README.md`
- Modify: `CLAUDE.md` (workspace root — wasm function list)
- Modify: `AGENTS.md` (workspace root — only if it duplicates the same wasm function list; check first)

- [ ] **Step 1: Update webui README**

Add an "Opening collections" paragraph under the appropriate features/usage
section: Open (header bar) accepts a collection zip (the Export format) or a
folder picked via the browser's directory picker; opening runs the same
parsing and discovery as the CLI (fatescroll-core via wasm
`parse_collection` — semantic validation happens live in the editor after
load, so collections the CLI rejects for range gaps or bad dice can still
be opened and fixed); opening is all-or-nothing with a full error list, and
YAML files not reachable from the manifest's `directories:` entries are
warned about. Match the README's existing tone and structure.

- [ ] **Step 2: Update the wasm function lists**

In root `CLAUDE.md`, the `fatescroll-wasm` bullet lists the exposed
functions — add `parse_collection`. Run
`grep -n "roll_collection" AGENTS.md CLAUDE.md webui/README.md` and update
every list that enumerates the wasm functions.

- [ ] **Step 3: Commit**

```bash
git add webui/README.md CLAUDE.md AGENTS.md
git commit -s -m "docs(webui): document opening collections"
```

---

## Completion

After all tasks: run the full quality gates (`cargo test`,
`cargo clippy -- -D warnings`, `cargo fmt --check`, `cd webui && npm test &&
npm run lint`), then follow superpowers:finishing-a-development-branch —
and close the bead (`bd close fatescroll-1z7`) once merged.
