// ABOUTME: Tests for autofillRanges: distributing a dice expression's value
// ABOUTME: range across result rows, contiguously or one-row-per-digit-outcome.

import { describe, expect, it } from 'vitest';
import { autofillRanges } from '../src/logic/autofill';
import type { ResultDraft } from '../src/model/types';

function makeResult(overrides: Partial<ResultDraft> = {}): ResultDraft {
  return { rid: 'r', min: '0', max: '0', text: '', chain: [], bindings: [], ...overrides };
}

function range(lo: number, hi: number): number[] {
  const out: number[] = [];
  for (let v = lo; v <= hi; v++) out.push(v);
  return out;
}

describe('autofillRanges', () => {
  it('splits 1d6 into 3 even contiguous ranges', () => {
    const results = [makeResult({ rid: 'a' }), makeResult({ rid: 'b' }), makeResult({ rid: 'c' })];
    const out = autofillRanges(results, range(1, 6), 'contiguous');
    expect(out.map((r) => [r.min, r.max])).toEqual([
      ['1', '2'],
      ['3', '4'],
      ['5', '6'],
    ]);
  });

  it('splits 2d6 (11 values) into 3 ranges, larger chunks first: 4/4/3', () => {
    const results = [makeResult({ rid: 'a' }), makeResult({ rid: 'b' }), makeResult({ rid: 'c' })];
    const out = autofillRanges(results, range(2, 12), 'contiguous');
    expect(out.map((r) => [r.min, r.max])).toEqual([
      ['2', '5'],
      ['6', '9'],
      ['10', '12'],
    ]);
  });

  it('splits a modifier-shifted span (1d8+[0,6] => 1..14) into 2 even ranges', () => {
    const results = [makeResult({ rid: 'a' }), makeResult({ rid: 'b' })];
    const out = autofillRanges(results, range(1, 14), 'contiguous');
    expect(out.map((r) => [r.min, r.max])).toEqual([
      ['1', '7'],
      ['8', '14'],
    ]);
  });

  it('digit kind emits one row per value, preserving existing rows by index and adding fresh rows after', () => {
    const values = range(11, 66).filter((v) => {
      const tens = Math.floor(v / 10);
      const units = v % 10;
      return tens >= 1 && tens <= 6 && units >= 1 && units <= 6;
    });
    expect(values).toHaveLength(36);

    const results = [
      makeResult({ rid: 'first', text: 'Alpha', chain: [{ rid: 'c1', struct: false, ref: 'x', reroll: [] }] }),
      makeResult({ rid: 'second', text: 'Beta', chain: [] }),
    ];
    const out = autofillRanges(results, values, 'digit');

    expect(out).toHaveLength(36);
    expect(out[0]).toMatchObject({ rid: 'first', min: String(values[0]), max: String(values[0]), text: 'Alpha' });
    expect(out[0].chain).toEqual(results[0].chain);
    expect(out[1]).toMatchObject({ rid: 'second', min: String(values[1]), max: String(values[1]), text: 'Beta' });

    for (let i = 2; i < 36; i++) {
      expect(out[i].min).toBe(String(values[i]));
      expect(out[i].max).toBe(String(values[i]));
      expect(out[i].text).toBe('');
      expect(out[i].chain).toEqual([]);
      expect(out[i].rid).toBeTruthy();
    }
    // Fresh rows get distinct generated rids, not reused from the originals.
    const freshRids = out.slice(2).map((r) => r.rid);
    expect(new Set(freshRids).size).toBe(freshRids.length);
    expect(freshRids).not.toContain('first');
    expect(freshRids).not.toContain('second');
  });

  it('returns results unchanged when values is empty', () => {
    const results = [makeResult({ rid: 'a' })];
    expect(autofillRanges(results, [], 'contiguous')).toBe(results);
  });

  it('drops surplus rows when there are more rows than values (1d6 with 8 results -> exactly 6 rows)', () => {
    const results = Array.from({ length: 8 }, (_, i) =>
      makeResult({ rid: `r${i}`, text: `text${i}`, chain: [] }),
    );
    const out = autofillRanges(results, range(1, 6), 'contiguous');
    expect(out).toHaveLength(6);
    expect(out.map((r) => [r.min, r.max])).toEqual([
      ['1', '1'],
      ['2', '2'],
      ['3', '3'],
      ['4', '4'],
      ['5', '5'],
      ['6', '6'],
    ]);
    // First 6 rows keep their identity/text.
    for (let i = 0; i < 6; i++) {
      expect(out[i].rid).toBe(`r${i}`);
      expect(out[i].text).toBe(`text${i}`);
    }
  });

  it('handles a contiguous span with negative bounds', () => {
    const results = [makeResult({ rid: 'a' }), makeResult({ rid: 'b' })];
    const out = autofillRanges(results, range(-1, 8), 'contiguous');
    expect(out.map((r) => [r.min, r.max])).toEqual([
      ['-1', '3'],
      ['4', '8'],
    ]);
  });

  it('digit kind preserves bindings on retained rows and defaults fresh rows to empty', () => {
    const kept = [
      makeResult({
        rid: 'first',
        text: 'Alpha',
        bindings: [{ rid: 'b1', name: 'one', value: '1' }],
      }),
      makeResult({
        rid: 'second',
        text: 'Beta',
        bindings: [{ rid: 'b2', name: 'flag', value: 'true' }],
      }),
    ];
    const out = autofillRanges(kept, [11, 12, 13], 'digit');

    expect(out).toHaveLength(3);
    expect(out[0].bindings).toEqual([{ rid: 'b1', name: 'one', value: '1' }]);
    expect(out[1].bindings).toEqual([{ rid: 'b2', name: 'flag', value: 'true' }]);
    expect(out[2].bindings).toEqual([]);
  });

  it('contiguous kind preserves bindings with the rest of each retained result', () => {
    const results = [
      makeResult({ rid: 'a', bindings: [{ rid: 'b1', name: 'one', value: '1' }] }),
      makeResult({ rid: 'b', bindings: [] }),
    ];
    const out = autofillRanges(results, range(1, 6), 'contiguous');
    expect(out[0].bindings).toEqual([{ rid: 'b1', name: 'one', value: '1' }]);
    expect(out[1].bindings).toEqual([]);
  });
});
