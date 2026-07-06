// @vitest-environment node
// ABOUTME: Golden round-trip proof: editor state -> emitted YAML -> files on
// ABOUTME: disk -> the real fatescroll binary validates and rolls them.

import { describe, test, expect, beforeAll } from 'vitest';
import { execa } from 'execa';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { collectionFiles, manifestYaml } from '../src/yaml/emit';
import type { ChainDraft, Dir, ManifestState, ResultDraft, TableDraft } from '../src/model/types';

const repoRoot = fileURLToPath(new URL('../..', import.meta.url)); // tests/ -> webui/ -> repo root
const bin = `${repoRoot}/target/debug/fatescroll`;

beforeAll(async () => {
  await execa('cargo', ['build', '-p', 'fatescroll-cli'], { cwd: repoRoot });
}, 300_000);

function mkResult(overrides: Partial<ResultDraft> = {}): ResultDraft {
  return { rid: 'r', min: '1', max: '1', text: '', chain: [], ...overrides };
}

function mkChain(overrides: Partial<ChainDraft> = {}): ChainDraft {
  return { rid: 'c', struct: false, ref: '', reroll: [], ...overrides };
}

function mkTable(overrides: Partial<TableDraft> = {}): TableDraft {
  return {
    uid: 'u',
    dirId: 'd',
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

// Every valid D66 outcome (tens/ones each 1-6, 36 values). Entry 11 carries
// a structured self-chain with reroll: [11] — the only entry with a chain,
// so once the reroll excludes 11 the child necessarily lands on a
// chain-less entry and the recursion terminates at depth 1.
function d66Entries(): ResultDraft[] {
  const entries: ResultDraft[] = [];
  for (let tens = 1; tens <= 6; tens++) {
    for (let ones = 1; ones <= 6; ones++) {
      const value = tens * 10 + ones;
      if (value === 11) {
        entries.push(
          mkResult({
            rid: 'r11',
            min: '11',
            max: '11',
            text: 'A shadow follows you, {2d6} paces back.',
            chain: [mkChain({ rid: 'c11', struct: true, ref: 'omen', reroll: [11] })],
          }),
        );
      } else {
        entries.push(
          mkResult({ rid: `r${value}`, min: String(value), max: String(value), text: `Omen entry ${value}.` }),
        );
      }
    }
  }
  return entries;
}

function writeCollection(root: string, manifest: ManifestState, dirs: Dir[], tables: TableDraft[]): string {
  fs.writeFileSync(path.join(root, 'manifest.yaml'), manifestYaml(manifest, dirs));
  for (const file of collectionFiles(dirs, tables)) {
    const filePath = path.join(root, file.path);
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, file.contents);
  }
  return path.join(root, 'manifest.yaml');
}

describe('golden round-trip through the real fatescroll CLI', () => {
  test(
    'validates and rolls a collection covering every emitter feature',
    async () => {
      const manifest: ManifestState = {
        name: 'Golden Collection',
        version: '1.0',
        namespace: 'golden',
        author: 'Jerry',
        minToolVersion: '',
      };
      const dirs: Dir[] = [
        { id: 'd-core', path: 'core', namespace: 'golden.core' },
        { id: 'd-weather', path: 'core/weather', namespace: 'golden.core.weather' },
      ];
      const omen = mkTable({
        uid: 't-omen',
        dirId: 'd-core',
        stem: 'omen',
        name: 'Omen',
        tags: ['omen', 'fate'],
        roll: 'D66',
        results: d66Entries(),
      });
      const stormMood = mkTable({
        uid: 't-storm',
        dirId: 'd-weather',
        stem: 'storm-mood',
        name: 'Storm Mood',
        tags: ['weather'],
        roll: '1d8',
        modOn: true,
        modMin: '-2',
        modMax: '0',
        notes: ['Roll before generating a storm scene.', 'Reroll if the party is indoors.'],
        results: [
          mkResult({ rid: 's1', min: '-1', max: '0', text: 'Calm skies clear the storm.' }),
          mkResult({ rid: 's2', min: '1', max: '3', text: 'Distant thunder rumbles.' }),
          mkResult({ rid: 's3', min: '4', max: '6', text: 'Wind and rain intensify.' }),
          mkResult({ rid: 's4', min: '7', max: '8', text: 'A violent squall breaks loose.' }),
        ],
      });
      const encounterGenerator = mkTable({
        uid: 't-generator',
        dirId: 'd-core',
        stem: 'encounter-generator',
        name: 'Encounter Generator',
        type: 'compound',
        tags: ['generator'],
        tableRefs: [
          { rid: 'x1', ref: 'omen' },
          { rid: 'x2', ref: 'storm-mood' },
        ],
      });
      const tables = [omen, stormMood, encounterGenerator];

      const tmp = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'fatescroll-golden-'));
      const manifestPath = writeCollection(tmp, manifest, dirs, tables);

      const validate = await execa(bin, ['validate', '--collection', manifestPath], { reject: false });
      expect(validate.exitCode, validate.stderr + validate.stdout).toBe(0);

      // Deterministic chain proof: force the D66 lookup to entry 11 via
      // --value, which exercises {2d6} interpolation and the reroll chain.
      const chained = await execa(bin, ['roll', 'golden.core.omen', '--value', '11', '--collection', manifestPath], {
        reject: false,
      });
      expect(chained.exitCode, chained.stderr + chained.stdout).toBe(0);
      expect(chained.stdout).toContain('paces back.');
      const lines = chained.stdout.trim().split('\n');
      expect(lines.length).toBe(2);
      expect(lines[1].startsWith('  Omen')).toBe(true);

      // Smoke: compound wires the D66 and modifier tables together.
      const roll = await execa(bin, ['roll', 'golden.core.encounter-generator', '--collection', manifestPath], {
        reject: false,
      });
      expect(roll.exitCode, roll.stderr + roll.stdout).toBe(0);
    },
    120_000,
  );

  test(
    'rejects a collection with a range gap',
    async () => {
      const manifest: ManifestState = {
        name: 'Gappy Collection',
        version: '1.0',
        namespace: 'gap',
        author: '',
        minToolVersion: '',
      };
      const dirs: Dir[] = [{ id: 'd-core', path: 'core', namespace: 'gap.core' }];
      const gappy = mkTable({
        uid: 't-gappy',
        dirId: 'd-core',
        stem: 'gappy',
        name: 'Gappy',
        roll: '1d6',
        results: [
          mkResult({ rid: 'g1', min: '1', max: '2', text: 'Low.' }),
          mkResult({ rid: 'g2', min: '4', max: '6', text: 'High.' }),
        ],
      });

      const tmp = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'fatescroll-gap-'));
      const manifestPath = writeCollection(tmp, manifest, dirs, [gappy]);

      const validate = await execa(bin, ['validate', '--collection', manifestPath], { reject: false });
      expect(validate.exitCode).not.toBe(0);
      expect(validate.stderr + validate.stdout).toMatch(/gap|missing values/i);
    },
    120_000,
  );
});
