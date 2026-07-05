// ABOUTME: Zustand store holding the Table Forge editor's manifest, directories, and table drafts.
// ABOUTME: Also tracks the current selection (view/selUid) and the last dice-roll preview.

import { create } from 'zustand';
import { uid } from './ids';
import type { Dir, ManifestState, TableDraft, View } from './types';

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

export const useForgeStore = create<ForgeState>()((set) => ({
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
        results: [{ rid: uid(), min: '1', max: '6', text: '', chain: [] }],
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

  setRollLines: (lines) => set({ rollLines: lines }),
}));

export function fqidOf(state: Pick<ForgeState, 'dirs'>, table: TableDraft): string {
  const dir = state.dirs.find((d) => d.id === table.dirId);
  return `${dir?.namespace ?? ''}.${table.stem}`;
}

export function tablesInDir(state: Pick<ForgeState, 'tables'>, dirId: string): TableDraft[] {
  return state.tables.filter((t) => t.dirId === dirId);
}

// Mirrors the registry's relative-first resolution: a ref resolves if
// `{fromNamespace}.{ref}` or the bare `{ref}` matches some table's FQID.
export function refResolves(
  state: Pick<ForgeState, 'dirs' | 'tables'>,
  fromNamespace: string,
  ref: string,
): boolean {
  const rel = `${fromNamespace}.${ref}`;
  return state.tables.some((t) => {
    const fq = fqidOf(state, t);
    return fq === rel || fq === ref;
  });
}
