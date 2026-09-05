// ABOUTME: Tests for the Table Forge store's localStorage auto-save: what gets
// ABOUTME: written, rehydration of a saved draft, and defensive fallback on bad data.

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { initialState, STORAGE_KEY, useForgeStore } from '../src/model/store';

function readPersisted(): { state: Record<string, unknown>; version: number } | null {
  const raw = localStorage.getItem(STORAGE_KEY);
  return raw ? JSON.parse(raw) : null;
}

beforeEach(() => {
  useForgeStore.setState(initialState());
  localStorage.clear();
});

describe('auto-save: writing', () => {
  it('writes the document to localStorage after a mutation', () => {
    useForgeStore.getState().addDir();

    const persisted = readPersisted();
    expect(persisted).not.toBeNull();
    expect(persisted!.state.dirs).toHaveLength(1);
    expect((persisted!.state.manifest as { name: string }).name).toBe('New Collection');
  });

  it('does not persist the rollLines preview', () => {
    useForgeStore.getState().setRollLines([{ indent: 0, text: 'preview' }]);
    // A document mutation is what triggers the write; rollLines must be excluded.
    useForgeStore.getState().addDir();

    const persisted = readPersisted();
    expect(persisted!.state).not.toHaveProperty('rollLines');
  });

  it('persists the current selection so a reload resumes where you left off', () => {
    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const uid = useForgeStore.getState().tables[0].uid;

    const persisted = readPersisted();
    expect(persisted!.state.view).toBe('table');
    expect(persisted!.state.selUid).toBe(uid);
  });
});

describe('auto-save: rehydration', () => {
  it('restores a persisted document on rehydrate', async () => {
    const doc = {
      state: {
        manifest: {
          name: 'Saved',
          version: '2.0',
          namespace: 'saved',
          author: 'Jerry',
          minToolVersion: '',
        },
        dirs: [{ id: 'd1', path: 'core', namespace: 'saved' }],
        tables: [],
        view: 'manifest',
        selUid: null,
      },
      version: 1,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(doc));

    await useForgeStore.persist.rehydrate();

    const s = useForgeStore.getState();
    expect(s.manifest.name).toBe('Saved');
    expect(s.dirs).toEqual([{ id: 'd1', path: 'core', namespace: 'saved' }]);
    expect(s.view).toBe('manifest');
  });

  it('never persists rollLines, so a rehydrated draft has a cleared preview', async () => {
    const doc = {
      state: {
        manifest: initialState().manifest,
        dirs: [],
        tables: [],
        view: 'empty',
        selUid: null,
      },
      version: 1,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(doc));
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });

    await useForgeStore.persist.rehydrate();

    expect(useForgeStore.getState().rollLines).toBeNull();
  });
});

describe('auto-save: defensive fallback', () => {
  it('falls back to initial state when the persisted document is malformed', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ state: { tables: 'not-an-array' }, version: 1 }),
    );

    await useForgeStore.persist.rehydrate();

    const s = useForgeStore.getState();
    expect(s.tables).toEqual([]);
    expect(s.dirs).toEqual([]);
    expect(s.manifest.name).toBe('New Collection');
  });

  it('does not throw and keeps initial state when storage holds corrupt JSON', async () => {
    localStorage.setItem(STORAGE_KEY, '{ this is not json');

    await expect(useForgeStore.persist.rehydrate()).resolves.not.toThrow();

    expect(useForgeStore.getState().tables).toEqual([]);
  });

  it('falls back to initial state when a table element is not an object', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: { manifest: initialState().manifest, dirs: [], tables: [42, 'x'] },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    // A primitive table would blow up the editor (e.g. current.results.map),
    // so a save carrying one must be rejected wholesale.
    expect(useForgeStore.getState().tables).toEqual([]);
  });

  it('falls back to initial state when a dir element is not an object', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: { manifest: initialState().manifest, dirs: ['nope'], tables: [] },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    expect(useForgeStore.getState().dirs).toEqual([]);
  });

  it('fills missing manifest fields from defaults so consumers never read undefined', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: { manifest: { name: 'Partial' }, dirs: [], tables: [] },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    const manifest = useForgeStore.getState().manifest;
    expect(manifest.name).toBe('Partial');
    expect(manifest.namespace).toBe('collection');
    expect(manifest.version).toBe('1.0');
    expect(manifest.author).toBe('');
  });

  it('discards a persisted draft written under a different schema version, without logging', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: { ...initialState().manifest, name: 'Old Schema' },
          dirs: [],
          tables: [],
        },
        version: 0,
      }),
    );
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    await useForgeStore.persist.rehydrate();

    // A version bump means the shape may have drifted; the old draft is dropped
    // rather than rehydrated into a mismatched schema — and quietly, without a
    // console error reaching real users on a future bump.
    expect(useForgeStore.getState().manifest.name).toBe('New Collection');
    expect(errorSpy).not.toHaveBeenCalled();
  });

  it('preserves a valid table selection and its nested arrays through rehydration', async () => {
    const table = {
      uid: 't1',
      dirId: 'd1',
      stem: 'oracle',
      name: 'Oracle',
      type: 'simple',
      tags: ['omen'],
      roll: '1d6',
      modOn: false,
      modMin: '',
      modMax: '',
      notes: ['a note'],
      results: [{ rid: 'r1', min: '1', max: '6', text: 'fate stirs', chain: [], bindings: [] }],
      tableRefs: [],
    };
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [{ id: 'd1', path: 'core', namespace: 'collection' }],
          tables: [table],
          view: 'table',
          selUid: 't1',
        },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    const s = useForgeStore.getState();
    expect(s.view).toBe('table');
    expect(s.selUid).toBe('t1');
    expect(s.tables[0]).toEqual(table);
  });

  it('resets an incoherent table view whose selUid resolves to no table', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [],
          tables: [],
          view: 'table',
          selUid: 'ghost',
        },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    const s = useForgeStore.getState();
    expect(s.view).toBe('manifest');
    expect(s.selUid).toBeNull();
  });

  it('repairs an unknown persisted view to the manifest view', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: { manifest: initialState().manifest, dirs: [], tables: [], view: 'garbage' },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    expect(useForgeStore.getState().view).toBe('manifest');
  });

  it('coerces a non-string persisted selUid to null', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: { manifest: initialState().manifest, dirs: [], tables: [], view: 'manifest', selUid: 42 },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    expect(useForgeStore.getState().selUid).toBeNull();
  });

  it('strips a stray persisted rollLines key on rehydrate', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [],
          tables: [],
          rollLines: [{ indent: 0, text: 'should not survive' }],
        },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    expect(useForgeStore.getState().rollLines).toBeNull();
  });
});

describe('auto-save: v1 to v2 migration', () => {
  it('rehydrates a version-1 document with empty binding arrays on every result', async () => {
    const table = {
      uid: 't1',
      dirId: 'd1',
      stem: 'oracle',
      name: 'Oracle',
      type: 'simple',
      tags: ['omen'],
      roll: '1d6',
      modOn: false,
      modMin: '',
      modMax: '',
      notes: ['a note'],
      results: [
        {
          rid: 'r1',
          min: '1',
          max: '6',
          text: 'fate stirs',
          chain: [{ rid: 'c1', struct: false, ref: 'omen', reroll: [] }],
        },
      ],
      tableRefs: [],
    };
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [{ id: 'd1', path: 'core', namespace: 'collection' }],
          tables: [table],
          view: 'table',
          selUid: 't1',
        },
        version: 1,
      }),
    );

    await useForgeStore.persist.rehydrate();

    const s = useForgeStore.getState();
    expect(s.view).toBe('table');
    expect(s.selUid).toBe('t1');
    // All original data survives: text, chains, notes, and editor ids.
    expect(s.tables[0]).toMatchObject({
      uid: 't1',
      stem: 'oracle',
      name: 'Oracle',
      notes: ['a note'],
    });
    expect(s.tables[0].results[0]).toMatchObject({
      rid: 'r1',
      min: '1',
      max: '6',
      text: 'fate stirs',
      chain: [{ rid: 'c1', struct: false, ref: 'omen', reroll: [] }],
    });
    // ...and every result gains an empty values list.
    expect(s.tables[0].results[0].bindings).toEqual([]);
  });

  it('discards malformed version-1 data without throwing', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [{ id: 'd1', path: 'core', namespace: 'collection' }],
          tables: [{ uid: 't1', results: 'not-an-array' }],
          view: 'manifest',
          selUid: null,
        },
        version: 1,
      }),
    );

    await expect(useForgeStore.persist.rehydrate()).resolves.not.toThrow();

    const s = useForgeStore.getState();
    expect(s.tables).toEqual([]);
    expect(s.dirs).toEqual([]);
    expect(s.manifest.name).toBe('New Collection');
  });

  it('discards a future unknown version carrying tables, without logging', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: { ...initialState().manifest, name: 'Future Schema' },
          dirs: [{ id: 'd1', path: 'core', namespace: 'collection' }],
          tables: [
            {
              uid: 't1',
              dirId: 'd1',
              stem: 'oracle',
              name: 'Oracle',
              type: 'simple',
              tags: [],
              roll: '1d6',
              modOn: false,
              modMin: '',
              modMax: '',
              notes: [],
              results: [
                {
                  rid: 'r1',
                  min: '1',
                  max: '6',
                  text: 'fate stirs',
                  chain: [],
                  bindings: [{ rid: 'b1', name: 'one', value: '1' }],
                },
              ],
              tableRefs: [],
            },
          ],
          view: 'table',
          selUid: 't1',
        },
        version: 99,
      }),
    );
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    try {
      await useForgeStore.persist.rehydrate();

      expect(useForgeStore.getState().manifest.name).toBe('New Collection');
      expect(useForgeStore.getState().tables).toEqual([]);
      expect(errorSpy).not.toHaveBeenCalled();
      expect(warnSpy).not.toHaveBeenCalled();
    } finally {
      errorSpy.mockRestore();
      warnSpy.mockRestore();
    }
  });

  it('round-trips version-2 binding names, sources, order, and row ids', async () => {
    const bindings = [
      { rid: 'b1', name: 'one', value: '1' },
      { rid: 'b2', name: 'price', value: 'count * 25' },
    ];
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [{ id: 'd1', path: 'core', namespace: 'collection' }],
          tables: [
            {
              uid: 't1',
              dirId: 'd1',
              stem: 'gems',
              name: 'Gems',
              type: 'simple',
              tags: [],
              roll: '1d6',
              modOn: false,
              modMin: '',
              modMax: '',
              notes: [],
              results: [{ rid: 'r1', min: '1', max: '6', text: 'Found {= one}.', chain: [], bindings }],
              tableRefs: [],
            },
          ],
          view: 'table',
          selUid: 't1',
        },
        version: 2,
      }),
    );

    await useForgeStore.persist.rehydrate();

    expect(useForgeStore.getState().tables[0].results[0].bindings).toEqual(bindings);
  });

  it('migrates version-1 data without logging and without restoring a preview', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: {
          manifest: initialState().manifest,
          dirs: [],
          tables: [],
          rollLines: [{ indent: 0, text: 'should not survive' }],
        },
        version: 1,
      }),
    );
    useForgeStore.setState({ rollLines: [{ indent: 0, text: 'stale' }] });
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    try {
      await useForgeStore.persist.rehydrate();

      expect(useForgeStore.getState().rollLines).toBeNull();
      expect(errorSpy).not.toHaveBeenCalled();
      expect(warnSpy).not.toHaveBeenCalled();
    } finally {
      errorSpy.mockRestore();
      warnSpy.mockRestore();
    }
  });
});
