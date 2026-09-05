// ABOUTME: Auto-fill ranges: distributes a dice expression's outcome values across
// ABOUTME: result rows, either one row per digit outcome or evenly split contiguous ranges.

import { uid } from '../model/ids';
import type { ResultDraft } from '../model/types';

export function autofillRanges(
  results: ResultDraft[],
  values: number[],
  kind: 'digit' | 'contiguous',
): ResultDraft[] {
  if (values.length === 0) return results;
  if (kind === 'digit') {
    // one row per outcome; preserve existing text/chain by index
    return values.map((v, i) => {
      const prev = results[i];
      return {
        rid: prev?.rid ?? uid(),
        min: String(v),
        max: String(v),
        text: prev?.text ?? '',
        chain: prev?.chain ?? [],
        bindings: prev?.bindings ?? [],
      };
    });
  }
  // Contiguous path: at most one row per value; surplus rows are dropped
  // (collapsing them onto the last value would create overlaps core rejects).
  const rows = results.slice(0, values.length);
  const k = rows.length;
  if (k === 0) return results;
  const n = values.length;
  const base = Math.floor(n / k);
  const extra = n % k;
  let idx = 0;
  return rows.map((r, i) => {
    const size = base + (i < extra ? 1 : 0);
    const lo = values[idx];
    const hi = values[idx + size - 1];
    idx += size;
    return { ...r, min: String(lo), max: String(hi) };
  });
}
