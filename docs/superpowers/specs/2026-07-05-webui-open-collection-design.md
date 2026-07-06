# Table Forge: Open/Import Existing Collection

**Bead:** fatescroll-1z7
**Date:** 2026-07-05
**Status:** Approved

## Problem

Table Forge is create-and-export only. There is no way to reload exported
work in progress or open an existing on-disk collection (e.g.
`~/rpgs/tables/kal-arath`) for editing. The editor state (`ManifestState`,
`Dir[]`, `TableDraft[]`) serializes to YAML via `emit.ts` and zips via
`export/zip.ts`, but nothing parses YAML back into drafts.

## Decisions

| Question | Decision |
|---|---|
| Mechanism | Zip round-trip (open from zip or folder) |
| Input formats | `.zip` upload AND folder picker (`webkitdirectory`) |
| Error handling | All-or-nothing: any parse/representability failure aborts the open |
| Parser location | New wasm `parse_collection` using fatescroll-core serde models |

Out of scope: manifest `files:` entries, multi-manifest collections
(`*.manifest.yaml`), File System Access API save-back-in-place,
localStorage autosave.

## 1. UI Flow

`HeaderBar` gains an **Open** control next to Export with two entries:
*Open zip…* and *Open folder…*. Implementation: two hidden
`<input type="file">` elements — one with `accept=".zip"`, one with
`webkitdirectory` (folder reading works in Firefox/Chrome/Safari).

- If the editor currently has any tables, show a confirmation before
  replacing state.
- On success, replace the store contents wholesale and select the
  manifest view.
- On failure, show the full error list; editor state is untouched.

## 2. Ingest and Discovery (TypeScript)

Both inputs normalize to `{ manifestYaml, files: [{path, contents}] }`
with paths relative to the manifest's directory.

**Zip:** decode with `fflate.unzipSync`. Accept `manifest.yaml` at the
archive root or inside exactly one top-level directory (the layout
`buildCollectionZip` produces). Anything else — zero or multiple
candidate manifests — is an error.

**Folder:** read `File.webkitRelativePath`, strip the picked folder's
name, require `manifest.yaml` at the folder root.

**Discovery** mirrors `fatescroll-core/src/collection.rs` exactly:

- For each manifest `directories:` entry, take files *directly* in that
  directory (non-recursive — `kal-arath` lists `core/` and
  `core/weather` as separate entries for this reason).
- Match `.yaml` / `.yml` extensions only.
- Skip `manifest.yaml` and `*.manifest.yaml`.

YAML files present in the input but not matched by discovery are
reported as a **warning** (they would not survive re-export) but do not
block the open. This matches CLI semantics: the CLI never loads them
either.

## 3. Parsing (new wasm function)

```rust
pub fn parse_collection(manifest_yaml: &str, files_json: &str) -> String
```

- Parses the manifest and each file's table using fatescroll-core's
  serde models, reusing the loader's rules: `id` must match the filename
  stem; an absent `id` derives from the stem.
- Rejects manifests with `files:` entries with a clear message
  ("not supported by Table Forge").
- Returns JSON: parsed manifest plus per-file parsed tables on success,
  or a list of per-file errors. `Manifest`/`DirectoryEntry` need
  `Serialize` derives (or a hand-built JSON view) — they are currently
  `Deserialize`-only.
- **All-or-nothing** is enforced by the caller: any error in the result
  aborts the open.

## 4. Mapping to Drafts (TypeScript)

Parsed JSON → editor state, in a new module (e.g. `src/import/`):

- manifest → `ManifestState` (`~`/null author and `min_tool_version`
  map to `''`)
- `directories:` entries → `Dir[]` with fresh `uid()` ids
- tables → `TableDraft[]`: fresh `uid()`, `dirId` matched by directory
  path, fresh `rid` per result/chain row, numbers rendered to strings,
  `modifier_range` → `modOn`/`modMin`/`modMax`, chain `Modified`
  entries → `struct: true` with `reroll`, bare strings → `struct: false`
- New store action `loadCollection(data)` replaces
  `manifest`/`dirs`/`tables`, sets `view: 'manifest'`, `selUid: null`,
  `rollLines: null`.

## 5. Pre-existing Fix (own commit, first)

`emit.ts` drops `notes` on compound tables even though core supports
them (`Table::Compound { notes, .. }`). Left as-is this silently loses
data on an open → edit → export round-trip. Fix before building import.

## 6. Testing (TDD throughout)

- **Rust:** unit tests for `parse_collection` covering the valid
  fixture (`tests/fixtures/valid-collection`), id/stem mismatch, absent
  id, `files:` rejection, per-file error accumulation.
- **Vitest:** ingest normalization (zip layouts, folder paths),
  discovery (non-recursive, extension filter, manifest skip, warning
  list), draft mapping.
- **Round-trip properties:**
  - import → re-export reproduces the collection (via the existing
    golden-test setup);
  - export → import → export is byte-identical.
