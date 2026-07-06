// ABOUTME: Tests for import ingest: locating manifest.yaml in zip/folder
// ABOUTME: entries, rebasing paths, and zip/FileList entry extraction.

import { describe, expect, it } from 'vitest';
import { strToU8, zipSync } from 'fflate';
import { entriesFromFileList, entriesFromZip, ingest, isYamlPath } from '../src/import/ingest';

describe('ingest', () => {
  it('accepts manifest.yaml at the root', () => {
    const raw = ingest([
      { path: 'manifest.yaml', contents: 'name: T' },
      { path: 'core/oracle.yaml', contents: 'id: oracle' },
    ]);
    expect(raw.manifestYaml).toBe('name: T');
    expect(raw.files).toEqual([{ path: 'core/oracle.yaml', contents: 'id: oracle' }]);
  });

  it('accepts manifest.yaml inside exactly one top-level directory and rebases paths', () => {
    const raw = ingest([
      { path: 'my-tables/manifest.yaml', contents: 'name: T' },
      { path: 'my-tables/core/oracle.yaml', contents: 'id: oracle' },
    ]);
    expect(raw.manifestYaml).toBe('name: T');
    expect(raw.files).toEqual([{ path: 'core/oracle.yaml', contents: 'id: oracle' }]);
  });

  it('drops entries outside the manifest root', () => {
    const raw = ingest([
      { path: 'my-tables/manifest.yaml', contents: 'name: T' },
      { path: '__MACOSX/junk.yaml', contents: '' },
    ]);
    expect(raw.files).toEqual([]);
  });

  it('rejects zero manifests', () => {
    expect(() => ingest([{ path: 'core/oracle.yaml', contents: '' }])).toThrow(/no manifest\.yaml/i);
  });

  it('rejects multiple candidate manifests', () => {
    expect(() =>
      ingest([
        { path: 'a/manifest.yaml', contents: '' },
        { path: 'b/manifest.yaml', contents: '' },
      ]),
    ).toThrow(/multiple/i);
  });

  it('does not treat deeper manifests as candidates', () => {
    const raw = ingest([
      { path: 'manifest.yaml', contents: 'name: T' },
      { path: 'a/b/manifest.yaml', contents: 'name: nested' },
    ]);
    expect(raw.manifestYaml).toBe('name: T');
    expect(raw.files).toEqual([{ path: 'a/b/manifest.yaml', contents: 'name: nested' }]);
  });

  it('rejects an absolute path', () => {
    expect(() => ingest([{ path: '/manifest.yaml', contents: '' }])).toThrow(/unsafe path/i);
  });

  it('rejects a path with a .. segment', () => {
    expect(() => ingest([{ path: '../manifest.yaml', contents: '' }])).toThrow(/unsafe path/i);
  });

  it('rejects a path with a nested .. segment', () => {
    expect(() =>
      ingest([
        { path: 'manifest.yaml', contents: 'name: T' },
        { path: 'a/../manifest.yaml', contents: '' },
      ]),
    ).toThrow(/unsafe path/i);
  });
});

describe('entriesFromZip', () => {
  it('extracts yaml entries and skips directories and non-yaml', () => {
    const zip = zipSync({
      'kal/manifest.yaml': strToU8('name: K'),
      'kal/core/oracle.yaml': strToU8('id: oracle'),
      'kal/readme.txt': strToU8('nope'),
    });
    const entries = entriesFromZip(zip);
    expect(entries).toEqual([
      { path: 'kal/manifest.yaml', contents: 'name: K' },
      { path: 'kal/core/oracle.yaml', contents: 'id: oracle' },
    ]);
  });
});

describe('entriesFromFileList', () => {
  function fakeFile(rel: string, contents: string): File {
    const f = new File([contents], rel.slice(rel.lastIndexOf('/') + 1));
    Object.defineProperty(f, 'webkitRelativePath', { value: rel });
    return f;
  }

  it('strips the picked folder name and reads only yaml files', async () => {
    const list = [
      fakeFile('kal/manifest.yaml', 'name: K'),
      fakeFile('kal/core/oracle.yaml', 'id: oracle'),
      fakeFile('kal/readme.txt', 'nope'),
    ] as unknown as FileList;
    expect(await entriesFromFileList(list)).toEqual([
      { path: 'manifest.yaml', contents: 'name: K' },
      { path: 'core/oracle.yaml', contents: 'id: oracle' },
    ]);
  });
});

describe('isYamlPath', () => {
  it('accepts .yaml and .yml, rejects others', () => {
    expect(isYamlPath('a/b.yaml')).toBe(true);
    expect(isYamlPath('a/b.yml')).toBe(true);
    expect(isYamlPath('a/b.txt')).toBe(false);
  });
});
