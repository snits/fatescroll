// ABOUTME: Shared filesystem helpers for round-trip tests: reading a
// ABOUTME: collection directory into ingest entries and writing a manifest
// ABOUTME: plus emitted files back out. One copy for every round-trip test.

import fs from 'node:fs';
import path from 'node:path';
import { isYamlPath, type CollectionEntry } from '../src/import/ingest';

export function readCollectionEntries(root: string): CollectionEntry[] {
  const entries: CollectionEntry[] = [];
  for (const p of fs.readdirSync(root, { recursive: true, encoding: 'utf8' })) {
    const full = path.join(root, p);
    const rel = p.split(path.sep).join('/');
    if (fs.statSync(full).isFile() && isYamlPath(rel)) {
      entries.push({ path: rel, contents: fs.readFileSync(full, 'utf8') });
    }
  }
  return entries;
}

export function writeCollection(dir: string, manifest: string, files: { path: string; contents: string }[]) {
  fs.writeFileSync(path.join(dir, 'manifest.yaml'), manifest);
  for (const f of files) {
    const target = path.join(dir, f.path);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, f.contents);
  }
}
