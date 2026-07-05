// ABOUTME: Tests for the Table Forge Zustand store: defaults, mutations, selection
// ABOUTME: fallback, roll-preview invalidation, and the fqidOf/tablesInDir selectors.

import { beforeEach, describe, expect, it } from 'vitest';
import { fqidOf, initialState, tablesInDir, useForgeStore } from '../src/model/store';

beforeEach(() => {
  useForgeStore.setState(initialState());
});

describe('initial state', () => {
  it('starts on the empty view with no selection', () => {
    const state = useForgeStore.getState();
    expect(state.view).toBe('empty');
    expect(state.selUid).toBeNull();
    expect(state.dirs).toHaveLength(0);
    expect(state.tables).toHaveLength(0);
    expect(state.rollLines).toBeNull();
  });

  it('defaults the manifest', () => {
    expect(useForgeStore.getState().manifest).toEqual({
      name: 'New Collection',
      version: '1.0',
      namespace: 'collection',
      author: '',
      minToolVersion: '',
    });
  });
});

describe('addDir', () => {
  it('appends a dir defaulted to the manifest namespace and switches to the manifest view', () => {
    useForgeStore.getState().addDir();
    const state = useForgeStore.getState();
    expect(state.dirs).toHaveLength(1);
    expect(state.dirs[0]).toMatchObject({ path: '', namespace: 'collection' });
    expect(state.dirs[0].id).toBeTruthy();
    expect(state.view).toBe('manifest');
  });

  it('gives each dir a fresh id', () => {
    useForgeStore.getState().addDir();
    useForgeStore.getState().addDir();
    const [first, second] = useForgeStore.getState().dirs;
    expect(first.id).not.toBe(second.id);
  });

  it('clears rollLines', () => {
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });
    useForgeStore.getState().addDir();
    expect(useForgeStore.getState().rollLines).toBeNull();
  });

  it('clears the table selection when switching to the manifest view', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    expect(useForgeStore.getState().selUid).not.toBeNull();

    useForgeStore.getState().addDir();

    const state = useForgeStore.getState();
    expect(state.view).toBe('manifest');
    expect(state.selUid).toBeNull();
  });
});

describe('addTable', () => {
  it('creates a simple table with defaults and selects it', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const state = useForgeStore.getState();
    expect(state.tables).toHaveLength(1);
    const table = state.tables[0];
    expect(table).toMatchObject({
      dirId,
      stem: 'new-table',
      name: 'New Table',
      type: 'simple',
      tags: [],
      roll: '1d6',
      modOn: false,
      notes: [],
      tableRefs: [],
    });
    expect(table.results).toEqual([{ rid: table.results[0].rid, min: '1', max: '6', text: '', chain: [] }]);
    expect(state.view).toBe('table');
    expect(state.selUid).toBe(table.uid);
  });

  it('clears rollLines', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });
    useForgeStore.getState().addTable(dirId);
    expect(useForgeStore.getState().rollLines).toBeNull();
  });
});

describe('updateTable', () => {
  it('patches the table and clears rollLines', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const uid = useForgeStore.getState().tables[0].uid;
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });

    useForgeStore.getState().updateTable(uid, { name: 'Renamed' });

    const state = useForgeStore.getState();
    expect(state.tables[0].name).toBe('Renamed');
    expect(state.rollLines).toBeNull();
  });

  it('sanitizes stem by replacing whitespace runs with a hyphen', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const uid = useForgeStore.getState().tables[0].uid;

    useForgeStore.getState().updateTable(uid, { stem: 'my  cool table' });

    expect(useForgeStore.getState().tables[0].stem).toBe('my-cool-table');
  });

  it('trims leading and trailing whitespace from stem before hyphenating', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const uid = useForgeStore.getState().tables[0].uid;

    useForgeStore.getState().updateTable(uid, { stem: ' my table ' });

    expect(useForgeStore.getState().tables[0].stem).toBe('my-table');
  });
});

describe('setManifest', () => {
  it('patches the manifest and clears rollLines', () => {
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });
    useForgeStore.getState().setManifest({ author: 'Jerry' });
    const state = useForgeStore.getState();
    expect(state.manifest.author).toBe('Jerry');
    expect(state.rollLines).toBeNull();
  });
});

describe('updateDir', () => {
  it('patches the dir and clears rollLines', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });

    useForgeStore.getState().updateDir(dirId, { path: 'tables/misc' });

    const state = useForgeStore.getState();
    expect(state.dirs[0].path).toBe('tables/misc');
    expect(state.rollLines).toBeNull();
  });
});

describe('deleteTable', () => {
  it('removes the table and selects the next remaining table', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    useForgeStore.getState().addTable(dirId);
    useForgeStore.getState().addTable(dirId);
    const [first, second, third] = useForgeStore.getState().tables;
    useForgeStore.getState().selectTable(second.uid);

    useForgeStore.getState().deleteTable(second.uid);

    const state = useForgeStore.getState();
    expect(state.tables.map((t) => t.uid)).toEqual([first.uid, third.uid]);
    expect(state.view).toBe('table');
    expect(state.selUid).toBe(third.uid);
  });

  it('falls back to the previous table when the last one is deleted', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    useForgeStore.getState().addTable(dirId);
    const [first, second] = useForgeStore.getState().tables;
    useForgeStore.getState().selectTable(second.uid);

    useForgeStore.getState().deleteTable(second.uid);

    const state = useForgeStore.getState();
    expect(state.selUid).toBe(first.uid);
    expect(state.view).toBe('table');
  });

  it('falls back to the empty view when no tables remain', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const only = useForgeStore.getState().tables[0];

    useForgeStore.getState().deleteTable(only.uid);

    const state = useForgeStore.getState();
    expect(state.tables).toHaveLength(0);
    expect(state.view).toBe('empty');
    expect(state.selUid).toBeNull();
  });

  it('leaves the current selection untouched when deleting a non-selected table', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    useForgeStore.getState().addTable(dirId);
    const [first, second] = useForgeStore.getState().tables;
    useForgeStore.getState().selectTable(first.uid);

    useForgeStore.getState().deleteTable(second.uid);

    const state = useForgeStore.getState();
    expect(state.tables.map((t) => t.uid)).toEqual([first.uid]);
    expect(state.selUid).toBe(first.uid);
    expect(state.view).toBe('table');
  });

  it('clears rollLines', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const only = useForgeStore.getState().tables[0];
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });

    useForgeStore.getState().deleteTable(only.uid);

    expect(useForgeStore.getState().rollLines).toBeNull();
  });
});

describe('deleteDir', () => {
  it('removes the dir and its tables', () => {
    useForgeStore.getState().addDir();
    useForgeStore.getState().addDir();
    const [dir1, dir2] = useForgeStore.getState().dirs;
    useForgeStore.getState().addTable(dir1.id);
    useForgeStore.getState().addTable(dir2.id);

    useForgeStore.getState().deleteDir(dir1.id);

    const state = useForgeStore.getState();
    expect(state.dirs.map((d) => d.id)).toEqual([dir2.id]);
    expect(state.tables).toHaveLength(1);
    expect(state.tables[0].dirId).toBe(dir2.id);
  });

  it('falls back selection when the selected table was in the deleted dir', () => {
    useForgeStore.getState().addDir();
    useForgeStore.getState().addDir();
    const [dir1, dir2] = useForgeStore.getState().dirs;
    useForgeStore.getState().addTable(dir1.id);
    useForgeStore.getState().addTable(dir2.id);
    const tableInDir2 = useForgeStore.getState().tables.find((t) => t.dirId === dir2.id)!;
    useForgeStore.getState().selectTable(tableInDir2.uid);

    useForgeStore.getState().deleteDir(dir2.id);

    const state = useForgeStore.getState();
    expect(state.tables).toHaveLength(1);
    expect(state.view).toBe('table');
    expect(state.selUid).toBe(state.tables[0].uid);
  });

  it('falls back to empty view when deleting the only dir with the selected table', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);

    useForgeStore.getState().deleteDir(dirId);

    const state = useForgeStore.getState();
    expect(state.dirs).toHaveLength(0);
    expect(state.tables).toHaveLength(0);
    expect(state.view).toBe('empty');
    expect(state.selUid).toBeNull();
  });

  it('clears rollLines', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });

    useForgeStore.getState().deleteDir(dirId);

    expect(useForgeStore.getState().rollLines).toBeNull();
  });

  it('leaves selection untouched when the selected table is in a different dir', () => {
    useForgeStore.getState().addDir();
    useForgeStore.getState().addDir();
    const [dir1, dir2] = useForgeStore.getState().dirs;
    useForgeStore.getState().addTable(dir1.id);
    useForgeStore.getState().addTable(dir2.id);
    const tableInDir1 = useForgeStore.getState().tables.find((t) => t.dirId === dir1.id)!;
    useForgeStore.getState().selectTable(tableInDir1.uid);

    useForgeStore.getState().deleteDir(dir2.id);

    const state = useForgeStore.getState();
    expect(state.selUid).toBe(tableInDir1.uid);
    expect(state.view).toBe('table');
  });
});

describe('selection actions', () => {
  it('selectManifest sets view to manifest and clears selUid', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);

    useForgeStore.getState().selectManifest();

    const state = useForgeStore.getState();
    expect(state.view).toBe('manifest');
    expect(state.selUid).toBeNull();
  });

  it('selectTable sets view to table and selUid', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const uid = useForgeStore.getState().tables[0].uid;
    useForgeStore.getState().selectManifest();

    useForgeStore.getState().selectTable(uid);

    const state = useForgeStore.getState();
    expect(state.view).toBe('table');
    expect(state.selUid).toBe(uid);
  });
});

describe('setRollLines', () => {
  it('sets rollLines without being cleared by itself', () => {
    const lines = [{ indent: 0, text: '3 on 1d6' }];
    useForgeStore.getState().setRollLines(lines);
    expect(useForgeStore.getState().rollLines).toEqual(lines);
  });
});

describe('fqidOf', () => {
  it('joins the dir namespace and table stem', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().updateDir(dirId, { namespace: 'wild' });
    useForgeStore.getState().addTable(dirId);
    const state = useForgeStore.getState();
    const table = state.tables[0];

    expect(fqidOf(state, table)).toBe('wild.new-table');
  });
});

describe('tablesInDir', () => {
  it('returns only the tables belonging to the given dir', () => {
    useForgeStore.getState().addDir();
    useForgeStore.getState().addDir();
    const [dir1, dir2] = useForgeStore.getState().dirs;
    useForgeStore.getState().addTable(dir1.id);
    useForgeStore.getState().addTable(dir2.id);
    useForgeStore.getState().addTable(dir1.id);

    const state = useForgeStore.getState();
    const inDir1 = tablesInDir(state, dir1.id);

    expect(inDir1).toHaveLength(2);
    expect(inDir1.every((t) => t.dirId === dir1.id)).toBe(true);
  });
});
