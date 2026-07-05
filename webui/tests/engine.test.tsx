// ABOUTME: Tests for the engine bridge: wrapEngine's JSON parsing/memoization
// ABOUTME: over a fake raw engine, plus EngineProvider/useEngine/useDerived wiring.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, renderHook, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import type { Engine, RollNode } from '../src/engine/engine';
import { wrapEngine } from '../src/engine/engine';
import { EngineProvider, useDerived, useEngine } from '../src/engine/useEngine';
import { initialState, useForgeStore } from '../src/model/store';

// @testing-library/react's auto-cleanup only registers when `afterEach` is a
// global (vitest config here does not set `globals: true`), so unmount
// rendered trees explicitly to avoid stale components staying subscribed to
// useForgeStore across tests.
afterEach(cleanup);

describe('wrapEngine', () => {
  it('validate parses errors out of the JSON envelope', () => {
    const raw = {
      validate_collection: (_m: string, _f: string) => JSON.stringify({ errors: ['boom'] }),
      dice_info: () => '',
      expected_values: () => '',
      histogram: () => '',
      roll_collection: () => '',
    };
    const engine = wrapEngine(raw);
    expect(engine.validate('manifest', [])).toEqual(['boom']);
  });

  it('diceInfo parses the ok envelope', () => {
    const raw = {
      validate_collection: () => '',
      dice_info: () => JSON.stringify({ ok: true, kind: 'range', min: 1, max: 6, outcomes: 6 }),
      expected_values: () => '',
      histogram: () => '',
      roll_collection: () => '',
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
      validate_collection: () => '',
      dice_info: () => {
        calls++;
        return JSON.stringify({ ok: true, kind: 'range', min: 1, max: 6, outcomes: 6 });
      },
      expected_values: () => '',
      histogram: () => '',
      roll_collection: () => '',
    };
    const engine = wrapEngine(raw);
    engine.diceInfo('1d6');
    engine.diceInfo('1d6');
    expect(calls).toBe(1);
  });

  it('diceInfo does not conflate different exprs in the cache', () => {
    let calls = 0;
    const raw = {
      validate_collection: () => '',
      dice_info: (expr: string) => {
        calls++;
        return JSON.stringify({ ok: true, kind: 'digit', min: 1, max: expr === '1d6' ? 6 : 8, outcomes: 1 });
      },
      expected_values: () => '',
      histogram: () => '',
      roll_collection: () => '',
    };
    const engine = wrapEngine(raw);
    expect(engine.diceInfo('1d6').max).toBe(6);
    expect(engine.diceInfo('1d8').max).toBe(8);
    expect(calls).toBe(2);
  });

  it('expectedValues returns values on ok, memoized by expr+modifier tuple', () => {
    let calls = 0;
    const raw = {
      validate_collection: () => '',
      dice_info: () => '',
      expected_values: () => {
        calls++;
        return JSON.stringify({ ok: true, values: [1, 2, 3] });
      },
      histogram: () => '',
      roll_collection: () => '',
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
      validate_collection: () => '',
      dice_info: () => '',
      expected_values: () => JSON.stringify({ ok: false, reason: 'bad expr' }),
      histogram: () => '',
      roll_collection: () => '',
    };
    const engine = wrapEngine(raw);
    expect(engine.expectedValues('bogus', false, 0, 0)).toBeNull();
  });

  it('histogram returns outcomes on ok, memoized by expr, null on not-ok', () => {
    let calls = 0;
    const raw = {
      validate_collection: () => '',
      dice_info: () => '',
      expected_values: () => '',
      histogram: (expr: string) => {
        calls++;
        if (expr === 'bogus') return JSON.stringify({ ok: false, reason: 'bad' });
        return JSON.stringify({ ok: true, outcomes: [[1, 0.5], [2, 0.5]] });
      },
      roll_collection: () => '',
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
      validate_collection: () => '',
      dice_info: () => '',
      expected_values: () => '',
      histogram: () => '',
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
      validate_collection: () => '',
      dice_info: () => '',
      expected_values: () => '',
      histogram: () => '',
      roll_collection: () => JSON.stringify({ error: 'not found' }),
    };
    const engine = wrapEngine(raw);
    expect(engine.roll('manifest', [], 'ns.missing')).toEqual({ error: 'not found' });
  });

  it('roll uses a fresh seed on each call', () => {
    const seeds: bigint[] = [];
    const raw = {
      validate_collection: () => '',
      dice_info: () => '',
      expected_values: () => '',
      histogram: () => '',
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
    expect(result.current.currentTitle).toBe('MANIFEST.YAML');
    expect(result.current.errors).toEqual([]);
    expect(result.current.currentYaml).toContain('name: New Collection');
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

  it('falls back to the manifest yaml/title on the manifest view', async () => {
    const fake = makeFakeEngine();
    const { result } = renderHook(() => useDerived(), { wrapper: wrapper(fake) });

    useForgeStore.getState().addDir();
    const dirId = useForgeStore.getState().dirs[0].id;
    useForgeStore.getState().addTable(dirId);
    await waitFor(() => expect(result.current.currentTitle).toBe('NEW-TABLE.YAML'));

    useForgeStore.getState().selectManifest();

    await waitFor(() => {
      expect(result.current.currentTitle).toBe('MANIFEST.YAML');
    });
  });
});
