// ABOUTME: Tests for the engine bridge: wrapEngine's JSON parsing/memoization
// ABOUTME: over a fake raw engine, plus EngineProvider/useEngine/useDerived wiring.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, renderHook, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import type { Engine, RawEngine, RollNode } from '../src/engine/engine';
import { wrapEngine } from '../src/engine/engine';
import { EngineProvider, useDerived, useEngine } from '../src/engine/useEngine';
import { initialState, useForgeStore } from '../src/model/store';

// @testing-library/react's auto-cleanup only registers when `afterEach` is a
// global (vitest config here does not set `globals: true`), so unmount
// rendered trees explicitly to avoid stale components staying subscribed to
// useForgeStore across tests.
afterEach(cleanup);

function makeRawEngine(): RawEngine {
  return {
    validate_collection: () => '',
    dice_info: () => '',
    expected_values: () => '',
    histogram: () => '',
    roll_collection: () => '',
    parse_collection: () => '',
  };
}

describe('wrapEngine', () => {
  it('validate parses errors out of the JSON envelope', () => {
    const raw = {
      ...makeRawEngine(),
      validate_collection: (_m: string, _f: string) => JSON.stringify({ errors: ['boom'] }),
    };
    const engine = wrapEngine(raw);
    expect(engine.validate('manifest', [])).toEqual(['boom']);
  });

  it('diceInfo parses the ok envelope', () => {
    const raw = {
      ...makeRawEngine(),
      dice_info: () => JSON.stringify({ ok: true, kind: 'range', min: 1, max: 6, outcomes: 6 }),
    };
    const engine = wrapEngine(raw);
    expect(engine.diceInfo('1d6')).toEqual({
      ok: true,
      kind: 'range',
      min: 1,
      max: 6,
      outcomes: 6,
    });
  });

  it('diceInfo memoizes by expr: identical calls hit the raw engine once', () => {
    let calls = 0;
    const raw = {
      ...makeRawEngine(),
      dice_info: () => {
        calls++;
        return JSON.stringify({ ok: true, kind: 'range', min: 1, max: 6, outcomes: 6 });
      },
    };
    const engine = wrapEngine(raw);
    engine.diceInfo('1d6');
    engine.diceInfo('1d6');
    expect(calls).toBe(1);
  });

  it('diceInfo does not conflate different exprs in the cache', () => {
    let calls = 0;
    const raw = {
      ...makeRawEngine(),
      dice_info: (expr: string) => {
        calls++;
        return JSON.stringify({ ok: true, kind: 'digit', min: 1, max: expr === '1d6' ? 6 : 8, outcomes: 1 });
      },
    };
    const engine = wrapEngine(raw);
    expect(engine.diceInfo('1d6').max).toBe(6);
    expect(engine.diceInfo('1d8').max).toBe(8);
    expect(calls).toBe(2);
  });

  it('expectedValues returns values on ok, memoized by expr+modifier tuple', () => {
    let calls = 0;
    const raw = {
      ...makeRawEngine(),
      expected_values: () => {
        calls++;
        return JSON.stringify({ ok: true, values: [1, 2, 3] });
      },
    };
    const engine = wrapEngine(raw);
    expect(engine.expectedValues('1d6', false, 0, 0)).toEqual([1, 2, 3]);
    expect(engine.expectedValues('1d6', false, 0, 0)).toEqual([1, 2, 3]);
    expect(calls).toBe(1);
    // Different modifier args are a different cache key.
    engine.expectedValues('1d6', true, 1, 2);
    expect(calls).toBe(2);
  });

  it('expectedValues returns null when the engine reports not-ok', () => {
    const raw = {
      ...makeRawEngine(),
      expected_values: () => JSON.stringify({ ok: false, reason: 'bad expr' }),
    };
    const engine = wrapEngine(raw);
    expect(engine.expectedValues('bogus', false, 0, 0)).toBeNull();
  });

  it('histogram returns outcomes on ok, memoized by expr, null on not-ok', () => {
    let calls = 0;
    const raw = {
      ...makeRawEngine(),
      histogram: (expr: string) => {
        calls++;
        if (expr === 'bogus') return JSON.stringify({ ok: false, reason: 'bad' });
        return JSON.stringify({ ok: true, outcomes: [[1, 0.5], [2, 0.5]] });
      },
    };
    const engine = wrapEngine(raw);
    expect(engine.histogram('1d2')).toEqual([[1, 0.5], [2, 0.5]]);
    expect(engine.histogram('1d2')).toEqual([[1, 0.5], [2, 0.5]]);
    expect(calls).toBe(1);
    expect(engine.histogram('bogus')).toBeNull();
    expect(calls).toBe(2);
  });

  it('roll passes a bigint seed through and returns the parsed RollResult tree', () => {
    let seenSeed: bigint | null = null;
    const tree: RollNode = { table_name: 'oracle', roll: 4, text: 'yes', children: [] };
    const raw = {
      ...makeRawEngine(),
      roll_collection: (_m: string, _f: string, _fqid: string, seed: bigint) => {
        seenSeed = seed;
        return JSON.stringify(tree);
      },
    };
    const engine = wrapEngine(raw);
    const result = engine.roll('manifest', [], 'ns.oracle');
    expect(result).toEqual(tree);
    expect(typeof seenSeed).toBe('bigint');
  });

  it('roll surfaces {error} responses from the engine', () => {
    const raw = {
      ...makeRawEngine(),
      roll_collection: () => JSON.stringify({ error: 'not found' }),
    };
    const engine = wrapEngine(raw);
    expect(engine.roll('manifest', [], 'ns.missing')).toEqual({ error: 'not found' });
  });

  it('roll uses a fresh seed on each call', () => {
    const seeds: bigint[] = [];
    const raw = {
      ...makeRawEngine(),
      roll_collection: (_m: string, _f: string, _fqid: string, seed: bigint) => {
        seeds.push(seed);
        return JSON.stringify({ table_name: 't', roll: 1, text: null, children: [] });
      },
    };
    const engine = wrapEngine(raw);
    engine.roll('manifest', [], 'ns.t');
    engine.roll('manifest', [], 'ns.t');
    expect(seeds).toHaveLength(2);
    expect(seeds[0]).not.toBe(seeds[1]);
  });

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
});

function makeFakeEngine(): Engine & { diceInfoCalls: number } {
  const fake = {
    diceInfoCalls: 0,
    validate(_manifestYaml: string, files: { stem: string }[]) {
      return files.some((f) => f.stem === 'bad') ? ['err1'] : [];
    },
    diceInfo(_expr: string) {
      fake.diceInfoCalls++;
      return { ok: true as const, kind: 'range' as const, min: 1, max: 6, outcomes: 6 };
    },
    expectedValues(_expr: string) {
      return [1, 2, 3];
    },
    histogram(_expr: string) {
      return [[1, 0.5], [2, 0.5]] as [number, number][];
    },
    roll(_manifestYaml: string, _files: unknown[], fqid: string) {
      return { table_name: fqid, roll: 3, text: 'a result', children: [] };
    },
    parseCollection(_manifestYaml: string, _files: { path: string; contents: string }[]) {
      return { ok: false as const, errors: ['not used'] };
    },
  };
  return fake;
}

function wrapper(engine: Engine, debounceMs = 0) {
  return ({ children }: { children: ReactNode }) => (
    <EngineProvider engine={engine} debounceMs={debounceMs}>
      {children}
    </EngineProvider>
  );
}

describe('EngineProvider / useEngine', () => {
  it('provides the injected engine immediately, without a loading fallback', () => {
    const fake = makeFakeEngine();
    render(
      <EngineProvider engine={fake} debounceMs={0}>
        <div>ready</div>
      </EngineProvider>,
    );
    expect(screen.getByText('ready')).toBeTruthy();
    expect(screen.queryByText(/Loading engine/i)).toBeNull();
  });

  it('useEngine throws outside a provider', () => {
    const { result } = renderHook(() => {
      try {
        useEngine();
        return null;
      } catch (err) {
        return err;
      }
    });
    expect(result.current).toBeInstanceOf(Error);
  });
});

describe('useDerived', () => {
  beforeEach(() => {
    useForgeStore.setState(initialState());
  });

  it('computes the manifest derivation synchronously on first render', () => {
    const fake = makeFakeEngine();
    const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake) });
    expect(result.current.errors).toEqual([]);
    expect(result.current.manifestYaml).toContain('name: New Collection');
    // Initial view is 'empty' — nothing is selected, so the yaml pane shows
    // the plain "YAML" title with no content, not the manifest.
    expect(result.current.currentTitle).toBe('YAML');
    expect(result.current.currentYaml).toBe('');
  });

  it('updates after a store mutation (debounced trailing edge)', async () => {
    const fake = makeFakeEngine();
    const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake) });

    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);

    await waitFor(() => {
      expect(result.current.files).toHaveLength(1);
    });
    expect(result.current.currentTitle).toBe('NEW-TABLE.YAML');
    expect(result.current.currentYaml).toContain('id: new-table');
  });

  it('surfaces validation errors from the engine', async () => {
    const fake = makeFakeEngine();
    const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake) });

    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    const tableUid = useForgeStore.getState().tables[0].uid;
    useForgeStore.getState().updateTable(tableUid, { stem: 'bad' });

    await waitFor(() => {
      expect(result.current.errors).toEqual(['err1']);
    });
  });

  it('maps an engine throw into derived errors instead of crashing or freezing', async () => {
    const fake = makeFakeEngine();
    let shouldThrow = true;
    fake.validate = () => {
      if (shouldThrow) throw new Error('wasm panicked');
      return [];
    };

    const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake) });

    // Eager first-render computation must not white-screen: the throw maps
    // into errors while the pure yaml parts still derive.
    expect(result.current.errors).toHaveLength(1);
    expect(result.current.errors[0]).toContain('engine failure');
    expect(result.current.errors[0]).toContain('wasm panicked');
    expect(result.current.manifestYaml).toContain('name: New Collection');

    // Debounced recompute path must not freeze on stale state: once the
    // engine recovers, a store change clears the failure.
    shouldThrow = false;
    useForgeStore.getState().addDir();

    await waitFor(() => {
      expect(result.current.errors).toEqual([]);
    });
    expect(result.current.files).toEqual([]);
  });

  it('debounces recomputes: rapid mutations collapse to one trailing validate', async () => {
    vi.useFakeTimers();
    try {
      const fake = makeFakeEngine();
      let validateCalls = 0;
      fake.validate = () => {
        validateCalls++;
        return [];
      };

      const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake, 100) });
      expect(validateCalls).toBe(1); // eager initial computation

      act(() => {
        useForgeStore.getState().addDir();
        const dirId = useForgeStore.getState().dirs[0].id;
        useForgeStore.getState().addTable(dirId);
        const tableUid = useForgeStore.getState().tables[0].uid;
        useForgeStore.getState().updateTable(tableUid, { name: 'Renamed' });
      });
      expect(validateCalls).toBe(1); // still pending — nothing fired yet

      act(() => {
        vi.advanceTimersByTime(100);
      });

      expect(validateCalls).toBe(2); // one trailing-edge recompute, not one per mutation
      expect(result.current.files).toHaveLength(1);
      expect(result.current.currentTitle).toBe('NEW-TABLE.YAML');
    } finally {
      vi.useRealTimers();
    }
  });

  it('derives the manifest yaml/title on the manifest view', async () => {
    const fake = makeFakeEngine();
    const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake) });

    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    await waitFor(() => expect(result.current.currentTitle).toBe('NEW-TABLE.YAML'));

    useForgeStore.getState().selectManifest();

    await waitFor(() => {
      expect(result.current.currentTitle).toBe('MANIFEST.YAML');
      expect(result.current.currentYaml).toContain('name: New Collection');
    });
  });
});
