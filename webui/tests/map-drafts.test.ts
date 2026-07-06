// ABOUTME: Tests for mapDrafts: parsed wasm collection JSON onto ManifestState,
// ABOUTME: Dir, and TableDraft models with fresh uids and stringified numbers.

import { describe, expect, it } from 'vitest';
import type { ParsedCollection } from '../src/engine/engine';
import { mapDrafts } from '../src/import/mapDrafts';

function parsed(): ParsedCollection {
  return {
    manifest: {
      name: 'Kal-Arath Collection',
      version: '1.0',
      namespace: 'kal-arath',
      author: null,
      min_tool_version: null,
      directories: [
        { path: 'core/', namespace: 'kal-arath.core' },
        { path: 'core/weather', namespace: 'kal-arath.core.weather' },
      ],
    },
    tables: [
      {
        path: 'core/oracle.yaml',
        namespace: 'kal-arath.core',
        stem: 'oracle',
        table: {
          type: 'simple',
          id: 'oracle',
          name: 'Oracle',
          tags: ['divination'],
          notes: ['Ask a yes/no question'],
          roll: '2d6',
          modifier_range: [0, 3],
          results: [
            {
              min: 2,
              max: 12,
              text: 'Yes, and [1d4]',
              chain: ['plain-ref', { table: 'oracle', reroll: [2] }],
            },
          ],
        },
      },
      {
        path: 'core/weather/storms.yaml',
        namespace: 'kal-arath.core.weather',
        stem: 'storms',
        table: {
          type: 'compound',
          id: 'storms',
          name: 'Storms',
          tags: [],
          notes: [],
          tables: ['wind', 'rain'],
        },
      },
    ],
    ignoredYaml: [],
  };
}

describe('mapDrafts', () => {
  it('maps manifest with null author/min_tool_version to empty strings', () => {
    const { manifest } = mapDrafts(parsed());
    expect(manifest).toEqual({
      name: 'Kal-Arath Collection',
      version: '1.0',
      namespace: 'kal-arath',
      author: '',
      minToolVersion: '',
    });
  });

  it('maps directories to Dirs with fresh distinct ids', () => {
    const { dirs } = mapDrafts(parsed());
    expect(dirs.map((d) => ({ path: d.path, namespace: d.namespace }))).toEqual([
      { path: 'core/', namespace: 'kal-arath.core' },
      { path: 'core/weather', namespace: 'kal-arath.core.weather' },
    ]);
    expect(dirs[0].id).not.toBe(dirs[1].id);
  });

  it('maps a simple table with modifier, results, and both chain forms', () => {
    const { dirs, tables } = mapDrafts(parsed());
    const oracle = tables.find((t) => t.stem === 'oracle')!;
    expect(oracle.dirId).toBe(dirs[0].id); // trailing-slash dir matches 'core' parent
    expect(oracle.type).toBe('simple');
    expect(oracle.roll).toBe('2d6');
    expect(oracle.modOn).toBe(true);
    expect(oracle.modMin).toBe('0');
    expect(oracle.modMax).toBe('3');
    expect(oracle.tags).toEqual(['divination']);
    expect(oracle.notes).toEqual(['Ask a yes/no question']);
    const r = oracle.results[0];
    expect(r.min).toBe('2');
    expect(r.max).toBe('12');
    expect(r.text).toBe('Yes, and [1d4]');
    expect(r.chain[0]).toMatchObject({ struct: false, ref: 'plain-ref', reroll: [] });
    expect(r.chain[1]).toMatchObject({ struct: true, ref: 'oracle', reroll: [2] });
  });

  it('maps a compound table to tableRefs with draft defaults for simple-only fields', () => {
    const { dirs, tables } = mapDrafts(parsed());
    const storms = tables.find((t) => t.stem === 'storms')!;
    expect(storms.dirId).toBe(dirs[1].id);
    expect(storms.type).toBe('compound');
    expect(storms.tableRefs.map((r) => r.ref)).toEqual(['wind', 'rain']);
    expect(storms.roll).toBe('1d6');
    expect(storms.modOn).toBe(false);
    expect(storms.results).toEqual([]);
  });

  it('maps null result text to empty string', () => {
    const p = parsed();
    p.tables[0].table.results![0].text = null;
    p.tables[0].table.results![0].chain = null;
    const { tables } = mapDrafts(p);
    expect(tables[0].results[0].text).toBe('');
    expect(tables[0].results[0].chain).toEqual([]);
  });
});
