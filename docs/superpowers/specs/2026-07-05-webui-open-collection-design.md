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

## 2. Ingest (TypeScript)

Both inputs normalize to a flat list of YAML entries
`[{path, contents}]` plus the manifest's YAML, with paths relative to
the manifest's directory.

**Zip:** decode with `fflate.unzipSync`. Accept `manifest.yaml` at the
archive root or inside exactly one top-level directory (the layout
`buildCollectionZip` produces). Anything else — zero or multiple
candidate manifests — is an error.

**Folder:** read `File.webkitRelativePath`, strip the picked folder's
name, require `manifest.yaml` at the folder root.

Only `.yaml`/`.yml` entries are read; everything else is ignored.

## 3. Parsing and Discovery (new wasm function)

```rust
pub fn parse_collection(manifest_yaml: &str, files_json: &str) -> String
```

Takes ALL ingested files; discovery happens here, in Rust, next to the
core semantics it mirrors — not in TypeScript.

- **Discovery** mirrors `fatescroll-core/src/collection.rs` exactly:
  for each manifest `directories:` entry, take files *directly* in that
  directory (non-recursive — `kal-arath` lists `core/` and
  `core/weather` as separate entries for this reason); match `.yaml` /
  `.yml` only; skip `manifest.yaml` and `*.manifest.yaml`.
- Parses the manifest and each discovered table using fatescroll-core's
  serde models. Core requires `id` in table YAML (a missing `id` is a
  parse error, exactly as in `build_registry`) and `id` must match the
  filename stem (mirrors `IdFilenameMismatch`).
- Does **not** run `validate_table`: a table with a range gap or bad
  dice expression parses and is fully representable — the editor and
  its live ValidationPanel are where such problems get fixed. The
  all-or-nothing rule applies to parse/structure errors only.
- Rejects manifests with `files:` entries with a clear message
  ("not supported by Table Forge").
- Returns JSON: parsed manifest, per-file parsed tables, and an
  `ignored_yaml` list (YAML files not matched by discovery — they would
  not survive re-export, surfaced as a warning; the CLI never loads
  them either), or `{"errors": [...]}`. The manifest JSON view is
  hand-built with `serde_json::json!` — `Manifest`/`DirectoryEntry` are
  `Deserialize`-only and stay that way.
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

- **Rust:** unit tests for `parse_collection` covering discovery
  (non-recursive, extension filter, manifest skip, ignored list),
  id/stem mismatch, absent id, `files:` rejection, per-file error
  accumulation.
- **Vitest:** ingest normalization (zip layouts, folder paths), draft
  mapping, store action, HeaderBar open flow.
- **Round-trip properties:**
  - import → re-export reproduces the collection (via the existing
    golden-test setup);
  - export → import → export is byte-identical.
