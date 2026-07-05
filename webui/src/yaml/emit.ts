// ABOUTME: Turns editor state (ManifestState/Dir/TableDraft) into fatescroll YAML text.
// ABOUTME: Output must parse byte-reliably in the real Rust CLI's serde_yaml loader.

import type { Dir, ManifestState, TableDraft } from '../model/types';

const KEYWORDS = new Set(['true', 'false', 'yes', 'no', 'on', 'off', 'null', '~', 'y', 'n']);

export function yv(s: string): string {
  const needsQuote =
    s === '' ||
    /^\s|\s$/.test(s) ||
    /^[-?:,[\]{}#&*!|>'"%@`]/.test(s) ||
    /[{}[\]]/.test(s) ||
    /:(\s|$)/.test(s) ||
    s.includes(' #') ||
    /[\n\r\t]/.test(s) ||
    KEYWORDS.has(s.toLowerCase()) ||
    /^[+-]?(\d|\.inf|\.nan)/i.test(s);
  if (!needsQuote) return s;
  // Escape order matters: backslashes first, then quotes and control chars.
  // Unescaped newlines inside a double-quoted scalar would FOLD to spaces on
  // parse (silent data loss), so \n\r\t must be escaped, not emitted raw.
  const escaped = s
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
  return `"${escaped}"`;
}

/** Raw numeric-input string -> integer for emission ('' or lone '-' -> 0). */
export function numOr0(raw: string): number {
  const n = parseInt(raw, 10);
  return Number.isNaN(n) ? 0 : n;
}

export function manifestYaml(m: ManifestState, dirs: Dir[]): string {
  const lines = [
    `name: ${yv(m.name)}`,
    `version: "${m.version.replace(/"/g, '\\"')}"`,
    `namespace: ${m.namespace}`,
    `author: ${m.author ? yv(m.author) : '~'}`,
    `min_tool_version: ${m.minToolVersion ? yv(m.minToolVersion) : '~'}`,
  ];
  if (dirs.length) {
    lines.push('directories:');
    for (const d of dirs) lines.push(`  - path: ${yv(d.path)}`, `    namespace: ${d.namespace}`);
  }
  return lines.join('\n') + '\n';
}

export function tableYaml(t: TableDraft): string {
  const lines = [`id: ${t.stem}`, `name: ${yv(t.name)}`, `type: ${t.type}`];
  if (t.tags.length) {
    lines.push('tags:');
    for (const tag of t.tags) lines.push(`  - ${yv(tag)}`);
  }
  if (t.type === 'compound') {
    lines.push('tables:');
    for (const r of t.tableRefs) lines.push(`  - ${yv(r.ref)}`);
  } else {
    lines.push(`roll: ${t.roll}`);
    if (t.modOn) lines.push(`modifier_range: [${numOr0(t.modMin)}, ${numOr0(t.modMax)}]`);
    if (t.notes.length) {
      lines.push('notes:');
      for (const n of t.notes) lines.push(`  - ${yv(n)}`);
    }
    lines.push('results:');
    for (const r of t.results) {
      lines.push(`  - min: ${numOr0(r.min)}`, `    max: ${numOr0(r.max)}`);
      if (r.text) lines.push(`    text: ${yv(r.text)}`);
      if (r.chain.length) {
        lines.push('    chain:');
        for (const c of r.chain) {
          if (c.struct) {
            lines.push(`      - table: ${yv(c.ref)}`);
            if (c.reroll.length) lines.push(`        reroll: [${c.reroll.join(', ')}]`);
          } else {
            lines.push(`      - ${yv(c.ref)}`);
          }
        }
      }
    }
  }
  return lines.join('\n') + '\n';
}

export interface FileInput {
  path: string;
  namespace: string;
  stem: string;
  contents: string;
}

/** All files as the WASM engine / zip export consume them. */
export function collectionFiles(dirs: Dir[], tables: TableDraft[]): FileInput[] {
  return tables.flatMap((t) => {
    const dir = dirs.find((d) => d.id === t.dirId);
    if (!dir) return [];
    const cleanPath = dir.path.replace(/\/+$/, '');
    return [
      {
        path: `${cleanPath}/${t.stem}.yaml`,
        namespace: dir.namespace,
        stem: t.stem,
        contents: tableYaml(t),
      },
    ];
  });
}
