// ABOUTME: Domain types for the Table Forge editor state.
// ABOUTME: Mirrors the fatescroll manifest/table YAML shape as in-browser draft models.

export interface ManifestState {
  name: string;
  version: string;
  namespace: string;
  author: string;
  minToolVersion: string; // '' means ~ (null)
}

export interface Dir {
  id: string;
  path: string;
  namespace: string;
}

export interface ChainDraft {
  rid: string;
  struct: boolean; // structured entry: emits { table, reroll }
  ref: string; // table reference (used for both forms)
  reroll: number[]; // only meaningful when struct
}

export interface BindingDraft {
  rid: string;
  name: string;
  value: string;
}

export interface ResultDraft {
  rid: string;
  min: string;
  max: string;
  text: string;
  chain: ChainDraft[];
  bindings: BindingDraft[];
}

export interface TableDraft {
  uid: string;
  dirId: string;
  stem: string;
  name: string;
  type: 'simple' | 'compound';
  tags: string[];
  roll: string; // simple only
  modOn: boolean;
  modMin: string;
  modMax: string; // simple only
  notes: string[];
  results: ResultDraft[]; // simple only
  tableRefs: { rid: string; ref: string }[]; // compound only
}

export type View = 'empty' | 'manifest' | 'table';
