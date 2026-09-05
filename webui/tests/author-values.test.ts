// @vitest-environment node
// ABOUTME: Author-values proof for result expressions: binding reorder/delete
// ABOUTME: diagnostics through the real WASM validator, plus the end-to-end
// ABOUTME: acceptance (import -> edit bindings -> export -> reopen -> validate
// ABOUTME: -> roll) with the real CLI. No mocked engine results stand in for
// ABOUTME: language behavior anywhere in this file.

import { describe, test, expect, beforeAll, afterAll } from 'vitest';
import { execa } from 'execa';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import initWasm, * as rawWasm from '../src/wasm/pkg/fatescroll_wasm.js';
import { wrapEngine, type Engine, type RawEngine } from '../src/engine/engine';
import { ingest } from '../src/import/ingest';
import { readCollectionEntries, writeCollection } from './collection-io';
import { mapDrafts, type LoadedState } from '../src/import/mapDrafts';
import { collectionFiles, manifestYaml } from '../src/yaml/emit';
import { uid } from '../src/model/ids';

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

/** The result-expressions fixture, imported through the real editor pipeline. */
function importFixture(): LoadedState {
  const fixture = path.join(repoRoot, 'tests/fixtures/result-expressions');
  const raw = ingest(readCollectionEntries(fixture));
  const outcome = engine.parseCollection(raw.manifestYaml, raw.files);
  if (!outcome.ok) throw new Error(`import failed: ${outcome.errors.join('; ')}`);
  return mapDrafts(outcome.collection);
}

function validateDrafts(state: LoadedState): string[] {
  return engine.validate(manifestYaml(state.manifest, state.dirs), collectionFiles(state.dirs, state.tables));
}

/** Export drafts to a temp collection and reopen them through the real import pipeline. */
function exportAndReopen(state: LoadedState, prefix: string): { tmp: string; redrafts: LoadedState } {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  tmpDirs.push(tmp);
  writeCollection(tmp, manifestYaml(state.manifest, state.dirs), collectionFiles(state.dirs, state.tables));
  const reopenedRaw = ingest(readCollectionEntries(tmp));
  const reopened = engine.parseCollection(reopenedRaw.manifestYaml, reopenedRaw.files);
  if (!reopened.ok) throw new Error(`reopen failed: ${reopened.errors.join('; ')}`);
  return { tmp, redrafts: mapDrafts(reopened.collection) };
}

describe('author-values diagnostics', () => {
  test('reordering price above count fails validation; restoring order clears it', () => {
    const state = importFixture();
    expect(validateDrafts(state)).toEqual([]);

    const result = state.tables[0].results[0];
    expect(result.bindings.map((b) => b.name)).toEqual(['count', 'price']);

    // The exact operation the UI move-up control performs on Value 2.
    result.bindings = [result.bindings[1], result.bindings[0]];

    const errors = validateDrafts(state);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('let[0].price');
    expect(errors[0]).toContain('unknown name `count`');

    // Fixing the order (the UI move-down control) clears the diagnostic.
    result.bindings = [result.bindings[1], result.bindings[0]];
    expect(validateDrafts(state)).toEqual([]);
  }, 60_000);

  test('deleting count while price and the template use it reports an unknown name', () => {
    const state = importFixture();

    // The exact operation the UI remove control performs on Value 1.
    const result = state.tables[0].results[0];
    result.bindings = result.bindings.filter((b) => b.name !== 'count');

    const errors = validateDrafts(state);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors.some((e) => e.includes('unknown name `count`'))).toBe(true);
  }, 60_000);

  test('an invalid expression stays editable and round-trips through export/reopen', () => {
    const state = importFixture();

    // An author mid-edit: syntactically broken but structurally valid YAML.
    state.tables[0].results[0].bindings[1].value = 'count +';

    // Reopening parses (not a malformed document) and preserves the source.
    const { redrafts } = exportAndReopen(state, 'fatescroll-author-invalid-');
    expect(redrafts.tables[0].results[0].bindings.map((b) => `${b.name}=${b.value}`)).toEqual([
      'count=roll("1d1")',
      'price=count +',
    ]);

    // Validation flags it as an expression error, exactly as the pane shows.
    const errors = validateDrafts(redrafts);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('let[1].price');
  }, 60_000);
});

describe('author-values end-to-end acceptance', () => {
  test('import -> edit -> export -> reopen -> validate -> roll', async () => {
    const state = importFixture();
    expect(validateDrafts(state)).toEqual([]);

    // Author edits through the draft model: narrow result 1 to a single
    // outcome, and add a second result whose same-spelled bindings compute
    // a different price. Per-result scopes must keep the two apart.
    const [first] = state.tables[0].results;
    first.min = '1';
    first.max = '1';
    state.tables[0].results.push({
      rid: uid(),
      min: '2',
      max: '6',
      text: 'Found {= count} {= if count == 1 then "gem" else "gems"} worth {= price} gold.',
      chain: [],
      bindings: [
        { rid: uid(), name: 'count', value: 'roll("1d1")' },
        { rid: uid(), name: 'price', value: 'count * 10' },
      ],
    });
    expect(validateDrafts(state)).toEqual([]);

    const preExportRids = state.tables[0].results.flatMap((r) => r.bindings.map((b) => b.rid));

    // Reopen the exported documents through the real import pipeline.
    const { tmp, redrafts } = exportAndReopen(state, 'fatescroll-author-values-');
    const rebound = redrafts.tables[0].results.flatMap((r) => r.bindings.map((b) => `${b.name}=${b.value}`));
    expect(rebound).toEqual(['count=roll("1d1")', 'price=count * 25', 'count=roll("1d1")', 'price=count * 10']);
    // Reopening assigns fresh editor ids; no row keeps its pre-export id.
    const postExportRids = redrafts.tables[0].results.flatMap((r) => r.bindings.map((b) => b.rid));
    expect(postExportRids).toHaveLength(preExportRids.length);
    for (const rid of postExportRids) expect(preExportRids).not.toContain(rid);

    // The reopened collection validates in both engines.
    expect(validateDrafts(redrafts)).toEqual([]);
    const cliValidate = await execa(bin, ['validate', '--collection', tmp], { reject: false });
    expect(cliValidate.exitCode, `${cliValidate.stdout}\n${cliValidate.stderr}`).toBe(0);

    // Deterministic lookup (--value pins selection; 1d1 dice cannot vary),
    // proving same-spelled bindings stay local to their own result.
    const roll1 = await execa(bin, ['roll', '--collection', tmp, 'expressions.gems', '--value', '1', '--quiet']);
    expect(roll1.exitCode).toBe(0);
    expect(roll1.stdout.trim()).toBe('Found 1 gem worth 25 gold.');
    const roll2 = await execa(bin, ['roll', '--collection', tmp, 'expressions.gems', '--value', '2', '--quiet']);
    expect(roll2.exitCode).toBe(0);
    expect(roll2.stdout.trim()).toBe('Found 1 gem worth 10 gold.');
  }, 120_000);
});
