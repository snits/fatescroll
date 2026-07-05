// ABOUTME: Tests for the YAML emitter that turns editor state into fatescroll
// ABOUTME: manifest/table YAML files, byte-exact against the real Rust CLI's serde format.

import { describe, expect, it } from 'vitest';
import { collectionFiles, manifestYaml, numOr0, tableYaml, yv } from '../src/yaml/emit';
import type { ChainDraft, Dir, ManifestState, ResultDraft, TableDraft } from '../src/model/types';

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

function mkResult(overrides: Partial<ResultDraft> = {}): ResultDraft {
  return { rid: 'r1', min: '1', max: '1', text: '', chain: [], ...overrides };
}

function mkChain(overrides: Partial<ChainDraft> = {}): ChainDraft {
  return { rid: 'c1', struct: false, ref: 'ref-table', reroll: [], ...overrides };
}

describe('manifestYaml', () => {
  it('emits author set, empty minToolVersion as ~, and a directories list', () => {
    const m: ManifestState = {
      name: 'Test Collection',
      version: '1.0',
      namespace: 'test',
      author: 'Jerry',
      minToolVersion: '',
    };
    const dirs: Dir[] = [
      { id: 'd1', path: 'terrain', namespace: 'test.terrain' },
      { id: 'd2', path: 'encounters', namespace: 'test.encounters' },
    ];
    expect(manifestYaml(m, dirs)).toBe(
      `name: Test Collection
version: "1.0"
namespace: test
author: Jerry
min_tool_version: ~
directories:
  - path: terrain
    namespace: test.terrain
  - path: encounters
    namespace: test.encounters
`,
    );
  });

  it('emits author and minToolVersion as ~ when both empty, no directories block when dirs is empty', () => {
    const m: ManifestState = {
      name: 'Empty Author',
      version: '2.5',
      namespace: 'foo',
      author: '',
      minToolVersion: '',
    };
    expect(manifestYaml(m, [])).toBe(
      `name: Empty Author
version: "2.5"
namespace: foo
author: ~
min_tool_version: ~
`,
    );
  });

  it('quotes version always, even when it looks like a plain number', () => {
    const m: ManifestState = {
      name: 'V',
      version: '1',
      namespace: 'v',
      author: '',
      minToolVersion: '3.2.1',
    };
    expect(manifestYaml(m, [])).toBe(
      `name: V
version: "1"
namespace: v
author: ~
min_tool_version: "3.2.1"
`,
    );
  });
});

describe('tableYaml - simple table', () => {
  it('emits tags, D66 roll, quoted text with colon-space and braces, plain and structured chain entries', () => {
    const t = mkTable({
      stem: 'mishap',
      name: 'Wizard Mishap',
      tags: ['wizard', 'mishap'],
      roll: 'D66',
      results: [
        mkResult({
          min: '1',
          max: '1',
          text: 'Roll again: {d6} extra',
          chain: [
            mkChain({ struct: false, ref: 'mishap', reroll: [] }),
            mkChain({ struct: true, ref: 'mishap', reroll: [1, 2] }),
            mkChain({ struct: true, ref: 'other-table', reroll: [] }),
          ],
        }),
      ],
    });
    expect(tableYaml(t)).toBe(
      `id: mishap
name: Wizard Mishap
type: simple
tags:
  - wizard
  - mishap
roll: D66
results:
  - min: 1
    max: 1
    text: "Roll again: {d6} extra"
    chain:
      - mishap
      - table: mishap
        reroll: [1, 2]
      - table: other-table
`,
    );
  });

  it('emits modifier_range from modOn/modMin/modMax', () => {
    const t = mkTable({
      stem: 'weighted',
      name: 'Weighted Table',
      modOn: true,
      modMin: '-2',
      modMax: '0',
      results: [mkResult()],
    });
    expect(tableYaml(t)).toBe(
      `id: weighted
name: Weighted Table
type: simple
roll: 1d6
modifier_range: [-2, 0]
results:
  - min: 1
    max: 1
`,
    );
  });

  it('emits a notes list', () => {
    const t = mkTable({
      stem: 'noted',
      name: 'Noted Table',
      notes: ['Plain note', 'Note with: colon'],
      results: [mkResult()],
    });
    expect(tableYaml(t)).toBe(
      `id: noted
name: Noted Table
type: simple
roll: 1d6
notes:
  - Plain note
  - "Note with: colon"
results:
  - min: 1
    max: 1
`,
    );
  });

  it('omits text and chain when the result has neither', () => {
    const t = mkTable({ stem: 'bare', name: 'Bare', results: [mkResult({ min: '1', max: '6', text: '' })] });
    expect(tableYaml(t)).toBe(
      `id: bare
name: Bare
type: simple
roll: 1d6
results:
  - min: 1
    max: 6
`,
    );
  });
});

describe('tableYaml - compound table', () => {
  it('emits a tables: list of refs', () => {
    const t = mkTable({
      stem: 'quick-npc',
      name: 'Quick NPC Generator',
      type: 'compound',
      tags: ['npc', 'generator'],
      tableRefs: [
        { rid: 'x1', ref: 'npc-occupation' },
        { rid: 'x2', ref: 'npc-disposition' },
      ],
    });
    expect(tableYaml(t)).toBe(
      `id: quick-npc
name: Quick NPC Generator
type: compound
tags:
  - npc
  - generator
tables:
  - npc-occupation
  - npc-disposition
`,
    );
  });
});

describe('yv', () => {
  const cases: Array<[string, string]> = [
    ['', '""'],
    ['yes', '"yes"'],
    ['12', '"12"'],
    ['has: colon', '"has: colon"'],
    ['Roll again:', '"Roll again:"'],
    ['a\nb', '"a\\nb"'],
    ['a\tb', '"a\\tb"'],
    ['a\rb', '"a\\rb"'],
    ['{d6} braces', '"{d6} braces"'],
    ['.inf', '".inf"'],
    ['Bandits', 'Bandits'],
  ];

  for (const [input, expected] of cases) {
    it(`renders ${JSON.stringify(input)} as ${expected}`, () => {
      expect(yv(input)).toBe(expected);
    });
  }

  it('escapes an embedded newline as \\n, not a raw newline in the output', () => {
    const out = yv('a\nb');
    expect(out).toBe('"a\\nb"');
    expect(out.includes('\n')).toBe(false);
  });

  it('escapes backslashes and embedded quotes together when quoting is triggered', () => {
    expect(yv('has "quotes" and \\ backslash: yes')).toBe(
      '"has \\"quotes\\" and \\\\ backslash: yes"',
    );
  });
});

describe('numOr0', () => {
  it('parses a plain integer string', () => {
    expect(numOr0('-3')).toBe(-3);
  });

  it('returns 0 for an empty string', () => {
    expect(numOr0('')).toBe(0);
  });

  it('returns 0 for a lone dash', () => {
    expect(numOr0('-')).toBe(0);
  });
});

describe('collectionFiles', () => {
  it('builds path/namespace/stem/contents per table, joining dir path and stem with a slash', () => {
    const dirs: Dir[] = [{ id: 'd1', path: 'npc', namespace: 'test.npc' }];
    const t = mkTable({ dirId: 'd1', stem: 'npc-quirk', name: 'NPC Quirk', results: [mkResult()] });
    const files = collectionFiles(dirs, [t]);
    expect(files).toEqual([
      {
        path: 'npc/npc-quirk.yaml',
        namespace: 'test.npc',
        stem: 'npc-quirk',
        contents: tableYaml(t),
      },
    ]);
  });

  it('strips a trailing slash from the dir path before joining', () => {
    const dirs: Dir[] = [{ id: 'd1', path: 'core/', namespace: 'test.core' }];
    const t = mkTable({ dirId: 'd1', stem: 'oracle', name: 'Oracle', results: [mkResult()] });
    const files = collectionFiles(dirs, [t]);
    expect(files[0].path).toBe('core/oracle.yaml');
  });

  it('skips tables whose dirId does not match any dir', () => {
    const dirs: Dir[] = [{ id: 'd1', path: 'core', namespace: 'test.core' }];
    const t = mkTable({ dirId: 'dangling', stem: 'orphan', name: 'Orphan', results: [mkResult()] });
    expect(collectionFiles(dirs, [t])).toEqual([]);
  });
});
