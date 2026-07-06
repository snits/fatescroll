// ABOUTME: Probability display for result cards: summing a dice distribution over
// ABOUTME: a [min,max] range and formatting the result as a percentage pill label.

export function rangeProbability(
  dist: [value: number, probability: number][] | null,
  min: number,
  max: number,
): number | null {
  if (!dist || Number.isNaN(min) || Number.isNaN(max)) return null;
  let sum = 0;
  for (const [v, p] of dist) if (v >= min && v <= max) sum += p;
  return sum;
}

export function formatPct(p: number | null): string {
  if (p === null) return '—';
  const pct = p * 100;
  if (pct === 0) return '0%';
  return pct < 10 ? `${pct.toFixed(1)}%` : `${Math.round(pct)}%`;
}
