// ABOUTME: Tests for flattenRoll: RollNode tree -> indented RollLine list
// ABOUTME: (roll/text presence formatting, nested indent).

import { describe, expect, it } from 'vitest';
import { flattenRoll } from '../src/logic/rollLines';
import type { RollNode } from '../src/engine/engine';

describe('flattenRoll', () => {
  it('formats a single node with a roll and text', () => {
    const node: RollNode = { table_name: 'oracle', roll: 4, text: 'yes', children: [] };
    expect(flattenRoll(node)).toEqual([{ indent: 0, text: 'oracle (rolled 4): yes' }]);
  });

  it('omits "(rolled …)" when roll is null', () => {
    const node: RollNode = { table_name: 'compound-table', roll: null, text: null, children: [] };
    expect(flattenRoll(node)).toEqual([{ indent: 0, text: 'compound-table' }]);
  });

  it('omits ": <text>" when text is null', () => {
    const node: RollNode = { table_name: 'oracle', roll: 2, text: null, children: [] };
    expect(flattenRoll(node)).toEqual([{ indent: 0, text: 'oracle (rolled 2)' }]);
  });

  it('flattens nested children at indent + 1', () => {
    const node: RollNode = {
      table_name: 'npc',
      roll: null,
      text: null,
      children: [
        { table_name: 'name', roll: 3, text: 'Bram', children: [] },
        {
          table_name: 'trait',
          roll: 1,
          text: 'gruff',
          children: [{ table_name: 'quirk', roll: 5, text: 'limps', children: [] }],
        },
      ],
    };

    expect(flattenRoll(node)).toEqual([
      { indent: 0, text: 'npc' },
      { indent: 1, text: 'name (rolled 3): Bram' },
      { indent: 1, text: 'trait (rolled 1): gruff' },
      { indent: 2, text: 'quirk (rolled 5): limps' },
    ]);
  });
});
