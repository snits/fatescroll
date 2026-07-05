// ABOUTME: Tests for rangeProbability (summing a dice distribution over a result's
// ABOUTME: [min,max]) and formatPct (percentage formatting for the probability pill).

import { describe, expect, it } from 'vitest';
import { formatPct, rangeProbability } from '../src/logic/probability';

const dist: [number, number][] = [
  [2, 0.0278],
  [3, 0.0556],
  [4, 0.0833],
  [5, 0.1111],
  [6, 0.1389],
  [7, 0.1667],
  [8, 0.1389],
  [9, 0.1111],
  [10, 0.0833],
  [11, 0.0556],
  [12, 0.0278],
];

describe('rangeProbability', () => {
  it('sums in-range probabilities from the distribution', () => {
    const p = rangeProbability(dist, 2, 5);
    expect(p).toBeCloseTo(0.0278 + 0.0556 + 0.0833 + 0.1111, 6);
  });

  it('returns null when dist is null', () => {
    expect(rangeProbability(null, 2, 5)).toBeNull();
  });

  it('returns null when min is NaN', () => {
    expect(rangeProbability(dist, NaN, 5)).toBeNull();
  });

  it('returns null when max is NaN', () => {
    expect(rangeProbability(dist, 2, NaN)).toBeNull();
  });
});

describe('formatPct', () => {
  it('renders null as an em dash', () => {
    expect(formatPct(null)).toBe('—');
  });

  it('renders zero as 0%', () => {
    expect(formatPct(0)).toBe('0%');
  });

  it('renders sub-10% values with one decimal', () => {
    expect(formatPct(0.0278)).toBe('2.8%');
  });

  it('renders 10%-and-above values as a rounded integer', () => {
    expect(formatPct(0.1667)).toBe('17%');
  });
});
