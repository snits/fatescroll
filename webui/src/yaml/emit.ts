// ABOUTME: Turns editor state (ManifestState/Dir/TableDraft) into fatescroll YAML text.
// ABOUTME: Output must parse byte-reliably in the real Rust CLI's serde_yaml loader.

import type { Dir, ManifestState, TableDraft } from '../model/types';

const KEYWORDS = new Set(['true', 'false', 'yes', 'no', 'on', 'off', 'null', '~', 'y', 'n']);

// Escape order matters: backslashes first, then quotes and control chars.
// Unescaped newlines inside a double-quoted scalar would FOLD to spaces on
// parse (silent data loss), so \n\r\t must be escaped, not emitted raw.
// Remaining C0 control chars use YAML's \u00XX escape form.
function dq(s: string): string {
  return (
    s
      .replace(/\\/g, '\\\\')
      .replace(/"/g, '\\"')
      .replace(/\n/g, '\\n')
      .replace(/\r/g, '\\r')
      .replace(/\t/g, '\\t')
      // eslint-disable-next-line no-control-regex
      .replace(/[\x00-\x08\x0b\x0c\x0e-\x1f]/g, (c) => {
        return `\\u00${c.charCodeAt(0).toString(16).padStart(2, '0')}`;
      })
  );
}

export function yv(s: string): string {
  const needsQuote =
    s === '' ||
    /^\s|\s$/.test(s) ||
    /^[-?:,[\]{}#&*!|>'"%@`]/.test(s) ||
    /[{}[\]]/.test(s) ||
    /:(\s|$)/.test(s) ||
    s.includes(' #') ||
    // eslint-disable-next-line no-control-regex
    /[\x00-\x1f]/.test(s) ||
    KEYWORDS.has(s.toLowerCase()) ||
    // Full-string numeric lookalikes only: YAML resolves a plain scalar to a
    // number only when the whole scalar parses as one, so dice expressions
    // like 1d6 or 2d6x10 stay raw while 12, 007, 3.7, .5, 1e5, 0x1f quote.
    /^[+-]?(\d[\d_]*(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/.test(s) ||
    /^[+-]?\.(inf|nan)$/i.test(s) ||
    /^[+-]?0[xob][0-9a-fA-F_]+$/i.test(s);
  if (!needsQuote) return s;
  return `"${dq(s)}"`;
}

/**
 * Raw numeric-input string -> integer for emission ('' or lone '-' -> 0).
 * Uses parseInt semantics: decimals truncate ('3.7' -> 3) and trailing
 * garbage is dropped ('12abc' -> 12).
 */
export function numOr0(raw: string): number {
  const n = parseInt(raw, 10);
  return Number.isNaN(n) ? 0 : n;
}

export function manifestYaml(m: ManifestState, dirs: Dir[]): string {
  const lines = [
    `name: ${yv(m.name)}`,
    `version: "${dq(m.version)}"`,
    `namespace: ${yv(m.namespace)}`,
    `author: ${m.author ? yv(m.author) : '~'}`,
    `min_tool_version: ${m.minToolVersion ? yv(m.minToolVersion) : '~'}`,
  ];
  if (dirs.length) {
    lines.push('directories:');
    for (const d of dirs) {
      lines.push(`  - path: ${yv(d.path)}`, `    namespace: ${yv(d.namespace)}`);
    }
  }
  return lines.join('\n') + '\n';
}

export function tableYaml(t: TableDraft): string {
  const lines = [`id: ${yv(t.stem)}`, `name: ${yv(t.name)}`, `type: ${t.type}`];
  if (t.tags.length) {
    lines.push('tags:');
    for (const tag of t.tags) lines.push(`  - ${yv(tag)}`);
  }
  if (t.type === 'compound') {
    if (t.tableRefs.length) {
      lines.push('tables:');
      for (const r of t.tableRefs) lines.push(`  - ${yv(r.ref)}`);
    }
  } else {
    lines.push(`roll: ${yv(t.roll)}`);
    if (t.modOn) lines.push(`modifier_range: [${numOr0(t.modMin)}, ${numOr0(t.modMax)}]`);
    if (t.notes.length) {
      lines.push('notes:');
      for (const n of t.notes) lines.push(`  - ${yv(n)}`);
    }
    if (t.results.length) lines.push('results:');
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
