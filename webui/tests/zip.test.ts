// ABOUTME: Tests for buildCollectionZip: packs manifest.yaml and every table's
// ABOUTME: YAML into a zip archive rooted at the collection's slug directory.

import { describe, expect, it } from 'vitest';
import { strFromU8, unzipSync } from 'fflate';
import { buildCollectionZip } from '../src/export/zip';
import { collectionFiles, manifestYaml, tableYaml } from '../src/yaml/emit';
import type { Dir, ManifestState, TableDraft } from '../src/model/types';

function mkTable(overrides: Partial<TableDraft> = {}): TableDraft {
  return {
    uid: 'u1',
    dirId: 'd1',
    stem: 'table',
    name: 'Table',
    type: 'simple',
    tags: [],
    roll: '1d6',
    modOn: false,
    modMin: '',
    modMax: '',
    notes: [],
    results: [],
    tableRefs: [],
    ...overrides,
  };
}

describe('buildCollectionZip', () => {
  it('packs manifest.yaml and every table under the slug root, matching emitted YAML exactly', () => {
    const manifest: ManifestState = {
      name: 'Kal-Arath Collection',
      version: '1.0',
      namespace: 'kal-arath',
      author: '',
      minToolVersion: '',
    };
    const dirs: Dir[] = [
      { id: 'd1', path: 'core', namespace: 'kal-arath.core' },
      { id: 'd2', path: 'core/weather', namespace: 'kal-arath.core.weather' },
    ];
    const tables: TableDraft[] = [
      mkTable({ uid: 't1', dirId: 'd1', stem: 'oracle', name: 'Oracle' }),
      mkTable({ uid: 't2', dirId: 'd2', stem: 'spring', name: 'Spring' }),
    ];
    const slug = 'kal-arath-collection';
    const manifestContents = manifestYaml(manifest, dirs);
    const files = collectionFiles(dirs, tables);

    const zip = buildCollectionZip(slug, manifestContents, files);
    const unzipped = unzipSync(zip);

    expect(Object.keys(unzipped).sort()).toEqual([
      `${slug}/core/oracle.yaml`,
      `${slug}/core/weather/spring.yaml`,
      `${slug}/manifest.yaml`,
    ]);
    expect(strFromU8(unzipped[`${slug}/manifest.yaml`])).toBe(manifestContents);
    expect(strFromU8(unzipped[`${slug}/core/oracle.yaml`])).toBe(tableYaml(tables[0]));
    expect(strFromU8(unzipped[`${slug}/core/weather/spring.yaml`])).toBe(tableYaml(tables[1]));
  });
});
