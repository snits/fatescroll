// ABOUTME: Zustand store holding the Table Forge editor's manifest, directories, and table drafts.
// ABOUTME: Also tracks the current selection (view/selUid) and the last dice-roll preview.

import { create, type StateCreator } from 'zustand';
import { persist } from 'zustand/middleware';
import { uid } from './ids';
import type { Dir, ManifestState, TableDraft, View } from './types';

// localStorage key under which the working draft is auto-saved.
export const STORAGE_KEY = 'fatescroll-table-forge';

export interface RollLine {
  indent: number;
  text: string;
  error?: boolean;
}

interface ForgeData {
  manifest: ManifestState;
  dirs: Dir[];
  tables: TableDraft[];
  view: View;
  selUid: string | null;
  rollLines: RollLine[] | null;
}

export interface ForgeState extends ForgeData {
  selectManifest(): void;
  selectTable(uid: string): void;
  setManifest(patch: Partial<ManifestState>): void;
  addDir(): void;
  updateDir(id: string, patch: Partial<Omit<Dir, 'id'>>): void;
  deleteDir(id: string): void;
  addTable(dirId: string): void;
  updateTable(uid: string, patch: Partial<Omit<TableDraft, 'uid'>>): void;
  deleteTable(uid: string): void;
  loadCollection(data: { manifest: ManifestState; dirs: Dir[]; tables: TableDraft[] }): void;
  startNewCollection(): void;
  setRollLines(lines: RollLine[] | null): void;
}

export function initialState(): ForgeData {
  return {
    manifest: {
      name: 'New Collection',
      version: '1.0',
      namespace: 'collection',
      author: '',
      minToolVersion: '',
    },
    dirs: [],
    tables: [],
    view: 'empty',
    selUid: null,
    rollLines: null,
  };
}

// Every edit to the manifest/dirs/tables invalidates the last roll preview.
function withClearedRoll<T extends Partial<ForgeData>>(patch: T): T & { rollLines: null } {
  return { ...patch, rollLines: null };
}

// The document fields written to localStorage. Ephemeral preview state
// (rollLines) is deliberately excluded so a reload never restores a stale roll.
type PersistedDoc = Pick<ForgeData, 'manifest' | 'dirs' | 'tables' | 'view' | 'selUid'>;

const VIEWS: readonly View[] = ['empty', 'manifest', 'table'];

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

// Validates a blob read back from localStorage. Returns a usable document, or
// null when the shape has drifted or is corrupt — the caller then keeps the
// fresh initial state instead of crashing on a stale schema. Elements must be
// objects: a primitive dir/table would slip past the arrays and later throw in
// the editor (e.g. reading `table.results`). Missing manifest fields inherit
// their defaults so consumers never read undefined.
//
// Element *contents* are trusted once the container shape checks out: partialize
// only ever writes complete drafts, and a shape change bumps the version (which
// discards the old blob), so deep per-field validation would earn nothing here.
function coerceDocument(persisted: unknown): PersistedDoc | null {
  if (!isObject(persisted)) return null;
  const p = persisted;
  if (!isObject(p.manifest)) return null;
  if (!Array.isArray(p.dirs) || !p.dirs.every(isObject)) return null;
  if (!Array.isArray(p.tables) || !p.tables.every(isObject)) return null;
  const tables = p.tables as unknown as TableDraft[];
  const selUid = typeof p.selUid === 'string' ? p.selUid : null;
  const view = VIEWS.includes(p.view as View) ? (p.view as View) : 'manifest';
  // A table view whose selection no longer resolves is incoherent (only
  // reachable via drift/tampering, since the two are written atomically); fall
  // back to the manifest view so no component renders a dangling selection.
  const danglingSelection = view === 'table' && !tables.some((t) => t.uid === selUid);
  return {
    manifest: { ...initialState().manifest, ...(p.manifest as Partial<ManifestState>) },
    dirs: p.dirs as unknown as Dir[],
    tables,
    view: danglingSelection ? 'manifest' : view,
    selUid: danglingSelection ? null : selUid,
  };
}

// Migrates a version-1 persisted document to the version-2 shape by adding
// `bindings: []` to every result while preserving all other data and editor
// ids. Only the tables/results nesting is checked here: manifest/dirs shape
// is left to coerceDocument, which discards malformed blobs after migration.
// Malformed table/result container shapes return undefined and follow the
// existing discard path without throwing.
function migrateV1ToV2(persisted: unknown): Record<string, unknown> | undefined {
  if (!isObject(persisted)) return undefined;
  if (!Array.isArray(persisted.tables) || !persisted.tables.every(isObject)) return undefined;
  const tables: Record<string, unknown>[] = [];
  for (const table of persisted.tables as Record<string, unknown>[]) {
    if (table.results === undefined) {
      tables.push(table);
      continue;
    }
    if (!Array.isArray(table.results) || !table.results.every(isObject)) return undefined;
    const results: Record<string, unknown>[] = [];
    for (const result of table.results as Record<string, unknown>[]) {
      if (result.bindings === undefined) results.push({ ...result, bindings: [] });
      else if (Array.isArray(result.bindings)) results.push(result);
      else return undefined;
    }
    tables.push({ ...table, results });
  }
  return { ...persisted, tables };
}

// Finds the uid of the surviving table adjacent to a just-removed one: the
// next table after it, falling back to the previous one, else none.
function selectNextSurviving(
  tables: TableDraft[],
  removedIndex: number,
  isRemoved: (table: TableDraft) => boolean,
): string | null {
  for (let i = removedIndex + 1; i < tables.length; i++) {
    if (!isRemoved(tables[i])) return tables[i].uid;
  }
  for (let i = removedIndex - 1; i >= 0; i--) {
    if (!isRemoved(tables[i])) return tables[i].uid;
  }
  return null;
}

const forgeState: StateCreator<ForgeState, [['zustand/persist', unknown]], []> = (set) => ({
  ...initialState(),

  selectManifest: () => set({ view: 'manifest', selUid: null }),

  selectTable: (tableUid) => set({ view: 'table', selUid: tableUid }),

  setManifest: (patch) =>
    set((state) => withClearedRoll({ manifest: { ...state.manifest, ...patch } })),

  addDir: () =>
    set((state) =>
      withClearedRoll({
        dirs: [...state.dirs, { id: uid(), path: '', namespace: state.manifest.namespace }],
        view: 'manifest' as View,
        selUid: null,
      }),
    ),

  updateDir: (id, patch) =>
    set((state) =>
      withClearedRoll({
        dirs: state.dirs.map((d) => (d.id === id ? { ...d, ...patch } : d)),
      }),
    ),

  deleteDir: (id) =>
    set((state) => {
      const tables = state.tables.filter((t) => t.dirId !== id);
      const dirs = state.dirs.filter((d) => d.id !== id);
      const selectedTable = state.tables.find((t) => t.uid === state.selUid);
      if (!selectedTable || selectedTable.dirId !== id) {
        return withClearedRoll({ dirs, tables });
      }
      const removedIndex = state.tables.findIndex((t) => t.uid === state.selUid);
      const nextUid = selectNextSurviving(state.tables, removedIndex, (t) => t.dirId === id);
      return withClearedRoll({
        dirs,
        tables,
        selUid: nextUid,
        view: (nextUid ? 'table' : 'empty') as View,
      });
    }),

  addTable: (dirId) =>
    set((state) => {
      const newUid = uid();
      const table: TableDraft = {
        uid: newUid,
        dirId,
        stem: 'new-table',
        name: 'New Table',
        type: 'simple',
        tags: [],
        roll: '1d6',
        modOn: false,
        modMin: '',
        modMax: '',
        notes: [],
        results: [{ rid: uid(), min: '1', max: '6', text: '', chain: [], bindings: [] }],
        tableRefs: [],
      };
      return withClearedRoll({
        tables: [...state.tables, table],
        selUid: newUid,
        view: 'table' as View,
      });
    }),

  updateTable: (tableUid, patch) =>
    set((state) => {
      const sanitized =
        patch.stem !== undefined ? { ...patch, stem: patch.stem.trim().replace(/\s+/g, '-') } : patch;
      return withClearedRoll({
        tables: state.tables.map((t) => (t.uid === tableUid ? { ...t, ...sanitized } : t)),
      });
    }),

  deleteTable: (tableUid) =>
    set((state) => {
      const tables = state.tables.filter((t) => t.uid !== tableUid);
      if (state.selUid !== tableUid) {
        return withClearedRoll({ tables });
      }
      const removedIndex = state.tables.findIndex((t) => t.uid === tableUid);
      const nextUid = selectNextSurviving(state.tables, removedIndex, (t) => t.uid === tableUid);
      return withClearedRoll({
        tables,
        selUid: nextUid,
        view: (nextUid ? 'table' : 'empty') as View,
      });
    }),

  loadCollection: (data) =>
    set({ ...data, view: 'manifest', selUid: null, rollLines: null }),

  startNewCollection: () => {
    set(initialState());
    useForgeStore.persist.clearStorage();
  },

  setRollLines: (lines) => set({ rollLines: lines }),
});

export const useForgeStore = create<ForgeState>()(
  persist(forgeState, {
    name: STORAGE_KEY,
    // Bump this whenever the persisted shape (PersistedDoc / TableDraft / Dir /
    // ManifestState) changes: an old-schema save is dropped instead of
    // rehydrated into a mismatched shape that could crash the editor.
    version: 2,
    // Version 1 drafts gain `bindings: []` on every result. Returning
    // undefined discards a stored draft on an unknown version mismatch. It
    // also stands in for the default migration so zustand does not log an
    // error to the console when an old draft is dropped.
    migrate: (persisted, version) => {
      if (version === 1) return migrateV1ToV2(persisted);
      return undefined;
    },
    partialize: ({ manifest, dirs, tables, view, selUid }): PersistedDoc => ({
      manifest,
      dirs,
      tables,
      view,
      selUid,
    }),
    merge: (persisted, current) => {
      const doc = coerceDocument(persisted);
      // A rehydrated draft starts with no roll preview, mirroring loadCollection.
      return doc ? { ...current, ...doc, rollLines: null } : current;
    },
  }),
);

export function fqidOf(state: Pick<ForgeState, 'dirs'>, table: TableDraft): string {
  const dir = state.dirs.find((d) => d.id === table.dirId);
  return `${dir?.namespace ?? ''}.${table.stem}`;
}

export function tablesInDir(state: Pick<ForgeState, 'tables'>, dirId: string): TableDraft[] {
  return state.tables.filter((t) => t.dirId === dirId);
}

// Mirrors the registry's three-step resolution (fatescroll-core registry.rs):
// 1. relative `{fromNamespace}.{ref}`, 2. `{ref}` as an absolute FQID,
// 3. globally unique bare-id suffix match on `.{ref}`.
export function refResolves(
  state: Pick<ForgeState, 'dirs' | 'tables'>,
  fromNamespace: string,
  ref: string,
): boolean {
  const fqids = state.tables.map((t) => fqidOf(state, t));
  const rel = `${fromNamespace}.${ref}`;
  if (fqids.some((fq) => fq === rel || fq === ref)) return true;
  const suffix = `.${ref}`;
  return fqids.filter((fq) => fq.endsWith(suffix)).length === 1;
}
