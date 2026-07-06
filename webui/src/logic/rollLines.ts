// ABOUTME: Flattens an engine RollNode tree into the indented lines the
// ABOUTME: dice roller output panel renders.

import type { RollNode } from '../engine/engine';
import type { RollLine } from '../model/store';

export function flattenRoll(node: RollNode, indent = 0, out: RollLine[] = []): RollLine[] {
  const rolled = node.roll !== null ? ` (rolled ${node.roll})` : '';
  out.push({ indent, text: `${node.table_name}${rolled}${node.text ? ': ' + node.text : ''}` });
  for (const child of node.children) flattenRoll(child, indent + 1, out);
  return out;
}
