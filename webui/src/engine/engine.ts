// ABOUTME: Engine bridge between the editor and fatescroll-wasm — JSON parsing,
// ABOUTME: per-call memoization for dice queries, and per-roll seed generation.

import type { FileInput } from '../yaml/emit';

export interface DiceInfo {
  ok: boolean;
  kind?: 'digit' | 'range' | 'simulated';
  min?: number;
  max?: number;
  outcomes?: number;
  reason?: string;
}

export function isUsableDiceInfo(info: DiceInfo): boolean {
  return (
    info.ok &&
    Number.isFinite(info.min) &&
    Number.isFinite(info.max) &&
    Number.isFinite(info.outcomes) &&
    info.min! <= info.max! &&
    info.outcomes! > 0
  );
}

export interface RollNode {
  table_name: string;
  roll: number | null;
  text: string | null;
  children: RollNode[];
}

export interface ImportFile {
  path: string;
  contents: string;
}

export interface ParsedChainModified {
  table: string;
  reroll: number[];
}
export type ParsedChain = string | ParsedChainModified;

export interface ParsedResult {
  min: number;
  max: number;
  text: string | null;
  chain: ParsedChain[] | null;
}

export interface ParsedTable {
  type: 'simple' | 'compound';
  id: string;
  name: string;
  tags: string[];
  notes: string[];
  roll?: string;
  modifier_range?: [number, number] | null;
  results?: ParsedResult[];
  tables?: string[];
}

export interface ParsedFile {
  path: string;
  namespace: string;
  stem: string;
  table: ParsedTable;
}

export interface ParsedManifest {
  name: string;
  version: string;
  namespace: string;
  author: string | null;
  min_tool_version: string | null;
  directories: { path: string; namespace: string }[];
}

export interface ParsedCollection {
  manifest: ParsedManifest;
  tables: ParsedFile[];
  ignoredYaml: string[];
}

export type ParseOutcome =
  | { ok: true; collection: ParsedCollection }
  | { ok: false; errors: string[] };

export interface Engine {
  validate(manifestYaml: string, files: FileInput[]): string[];
  diceInfo(expr: string): DiceInfo;
  expectedValues(expr: string, modOn: boolean, modMin: number, modMax: number): number[] | null;
  histogram(expr: string): [value: number, probability: number][] | null;
  roll(manifestYaml: string, files: FileInput[], fqid: string): RollNode | { error: string };
  parseCollection(manifestYaml: string, files: ImportFile[]): ParseOutcome;
}

/** The raw fatescroll-wasm module surface: string-in/string-out JSON envelopes. */
export interface RawEngine {
  validate_collection(manifestYaml: string, filesJson: string): string;
  dice_info(expr: string): string;
  expected_values(expr: string, modOn: boolean, modMin: number, modMax: number): string;
  histogram(expr: string): string;
  roll_collection(manifestYaml: string, filesJson: string, fqid: string, seed: bigint): string;
  parse_collection(manifestYaml: string, filesJson: string): string;
}

function nextSeed(): bigint {
  const bytes = new BigUint64Array(1);
  crypto.getRandomValues(bytes);
  return bytes[0];
}

/** Wraps a raw JSON-string engine (the wasm module, or a test double) with
 * envelope parsing and memoization. Kept separate from `initEngine` so the
 * memoization behavior is testable without loading WASM. */
export function wrapEngine(raw: RawEngine): Engine {
  const diceInfoCache = new Map<string, DiceInfo>();
  const expectedValuesCache = new Map<string, number[] | null>();
  const histogramCache = new Map<string, [number, number][] | null>();

  return {
    validate(manifestYaml, files) {
      const parsed = JSON.parse(raw.validate_collection(manifestYaml, JSON.stringify(files))) as {
        errors: string[];
      };
      return parsed.errors;
    },

    diceInfo(expr) {
      const cached = diceInfoCache.get(expr);
      if (cached) return cached;
      const info = JSON.parse(raw.dice_info(expr)) as DiceInfo;
      diceInfoCache.set(expr, info);
      return info;
    },

    expectedValues(expr, modOn, modMin, modMax) {
      const key = `${expr}|${modOn}|${modMin}|${modMax}`;
      if (expectedValuesCache.has(key)) return expectedValuesCache.get(key) ?? null;
      const parsed = JSON.parse(raw.expected_values(expr, modOn, modMin, modMax)) as {
        ok: boolean;
        values?: number[];
      };
      const value = parsed.ok ? (parsed.values ?? []) : null;
      expectedValuesCache.set(key, value);
      return value;
    },

    histogram(expr) {
      if (histogramCache.has(expr)) return histogramCache.get(expr) ?? null;
      const parsed = JSON.parse(raw.histogram(expr)) as {
        ok: boolean;
        outcomes?: [number, number][];
      };
      const value = parsed.ok ? (parsed.outcomes ?? []) : null;
      histogramCache.set(expr, value);
      return value;
    },

    roll(manifestYaml, files, fqid) {
      const parsed = JSON.parse(raw.roll_collection(manifestYaml, JSON.stringify(files), fqid, nextSeed())) as
        | RollNode
        | { error: string };
      return parsed;
    },

    parseCollection(manifestYaml, files) {
      const parsed = JSON.parse(raw.parse_collection(manifestYaml, JSON.stringify(files))) as
        | { errors: string[] }
        | { manifest: ParsedManifest; tables: ParsedFile[]; ignored_yaml: string[] };
      if ('errors' in parsed) return { ok: false, errors: parsed.errors };
      return {
        ok: true,
        collection: {
          manifest: parsed.manifest,
          tables: parsed.tables,
          ignoredYaml: parsed.ignored_yaml,
        },
      };
    },
  };
}

export async function initEngine(): Promise<Engine> {
  const wasm = await import('../wasm/pkg/fatescroll_wasm.js');
  await wasm.default();
  return wrapEngine(wasm);
}
