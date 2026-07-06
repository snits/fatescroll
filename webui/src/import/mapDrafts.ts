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
  // parse_collection only emits tables discovered under a manifest directory
  // entry, so a miss here means the contract has drifted — fail loud instead
  // of silently orphaning the table (emit's collectionFiles would drop it).
  if (!dir) throw new Error(`table "${f.path}" does not match any manifest directory`);
  const t = f.table;
  const mod = t.modifier_range ?? null;
  return {
    uid: uid(),
    dirId: dir.id,
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
