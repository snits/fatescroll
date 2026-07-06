// @vitest-environment node
// ABOUTME: Round-trip proof for import: a real fixture collection imports via
// ABOUTME: wasm parse_collection, and export -> import -> export is byte-identical.

import { describe, test, expect, beforeAll, afterAll } from 'vitest';
import { execa } from 'execa';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import initWasm, * as rawWasm from '../src/wasm/pkg/fatescroll_wasm.js';
import { wrapEngine, type Engine, type RawEngine } from '../src/engine/engine';
import { ingest, isYamlPath, type CollectionEntry } from '../src/import/ingest';
import { mapDrafts } from '../src/import/mapDrafts';
import { collectionFiles, manifestYaml } from '../src/yaml/emit';

const webuiRoot = fileURLToPath(new URL('..', import.meta.url));
const repoRoot = path.join(webuiRoot, '..');
const bin = path.join(repoRoot, 'target/debug/fatescroll');

let engine: Engine;

beforeAll(async () => {
  const bytes = fs.readFileSync(path.join(webuiRoot, 'src/wasm/pkg/fatescroll_wasm_bg.wasm'));
  await initWasm({ module_or_path: bytes });
  engine = wrapEngine(rawWasm as unknown as RawEngine);
  await execa('cargo', ['build', '-p', 'fatescroll-cli'], { cwd: repoRoot });
}, 300_000);

const tmpDirs: string[] = [];
afterAll(() => {
  for (const dir of tmpDirs) fs.rmSync(dir, { recursive: true, force: true });
});

function readCollectionEntries(root: string): CollectionEntry[] {
  const entries: CollectionEntry[] = [];
  for (const p of fs.readdirSync(root, { recursive: true, encoding: 'utf8' })) {
    const full = path.join(root, p);
    const rel = p.split(path.sep).join('/');
    if (fs.statSync(full).isFile() && isYamlPath(rel)) {
      entries.push({ path: rel, contents: fs.readFileSync(full, 'utf8') });
    }
  }
  return entries;
}

function writeCollection(dir: string, manifest: string, files: { path: string; contents: string }[]) {
  fs.writeFileSync(path.join(dir, 'manifest.yaml'), manifest);
  for (const f of files) {
    const target = path.join(dir, f.path);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, f.contents);
  }
}

describe('import round-trip', () => {
  test('valid-collection fixture imports and re-exports to a CLI-valid collection', async () => {
    const fixture = path.join(repoRoot, 'tests/fixtures/valid-collection');
    const raw = ingest(readCollectionEntries(fixture));
    const outcome = engine.parseCollection(raw.manifestYaml, raw.files);
    if (!outcome.ok) throw new Error(`import failed: ${outcome.errors.join('; ')}`);
    expect(outcome.collection.tables.length).toBeGreaterThanOrEqual(7);

    const { manifest, dirs, tables } = mapDrafts(outcome.collection);
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'fatescroll-import-'));
    tmpDirs.push(tmp);
    writeCollection(tmp, manifestYaml(manifest, dirs), collectionFiles(dirs, tables));

    const result = await execa(bin, ['validate', '--collection', tmp], { reject: false });
    expect(result.exitCode, `${result.stdout}\n${result.stderr}`).toBe(0);
  }, 60_000);

  test('export -> import -> export is byte-identical', () => {
    const manifest = {
      name: 'Round Trip',
      version: '1.0',
      namespace: 'rt',
      author: 'Jerry',
      minToolVersion: '',
    };
    const dirs = [
      { id: 'd1', path: 'core/', namespace: 'rt.core' },
      { id: 'd2', path: 'core/deep', namespace: 'rt.core.deep' },
    ];
    const tables = [
      {
        uid: 'u1',
        dirId: 'd1',
        stem: 'oracle',
        name: 'Oracle: "quoted"',
        type: 'simple' as const,
        tags: ['divination', 'true'],
        roll: '2d6',
        modOn: true,
        modMin: '0',
        modMax: '3',
        notes: ['Ask a question'],
        results: [
          {
            rid: 'r1',
            min: '2',
            max: '15',
            text: 'Yes, and [1d4] omens',
            chain: [
              { rid: 'c1', struct: false, ref: 'portent', reroll: [] },
              { rid: 'c2', struct: true, ref: 'oracle', reroll: [2, 3] },
            ],
          },
        ],
        tableRefs: [],
      },
      {
        uid: 'u2',
        dirId: 'd2',
        stem: 'portent',
        name: 'Portent',
        type: 'compound' as const,
        tags: [],
        roll: '1d6',
        modOn: false,
        modMin: '',
        modMax: '',
        notes: ['Roll everything'],
        results: [],
        tableRefs: [{ rid: 'p1', ref: 'oracle' }],
      },
    ];

    const manifest1 = manifestYaml(manifest, dirs);
    const files1 = collectionFiles(dirs, tables);

    const outcome = engine.parseCollection(
      manifest1,
      files1.map((f) => ({ path: f.path, contents: f.contents })),
    );
    if (!outcome.ok) throw new Error(`import failed: ${outcome.errors.join('; ')}`);
    const loaded = mapDrafts(outcome.collection);

    expect(manifestYaml(loaded.manifest, loaded.dirs)).toBe(manifest1);
    expect(collectionFiles(loaded.dirs, loaded.tables)).toEqual(files1);
  });
});
