// ABOUTME: Normalizes user-picked zip archives or folders into collection
// ABOUTME: entries (manifest YAML + relative file paths) for the wasm parser.

import { unzipSync } from 'fflate';

export interface CollectionEntry {
  path: string;
  contents: string;
}

export interface RawCollection {
  manifestYaml: string;
  files: CollectionEntry[];
}

export function isYamlPath(path: string): boolean {
  return path.endsWith('.yaml') || path.endsWith('.yml');
}

function basename(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

function depth(path: string): number {
  return path.split('/').length - 1;
}

// Zip-slip guard, mirroring assertSafePath in ../export/zip.ts: an absolute
// path or a `..` segment could extract outside the collection root.
function assertSafePath(path: string): void {
  if (path.startsWith('/') || path.split('/').includes('..')) {
    throw new Error(`Unsafe path in collection input: "${path}"`);
  }
}

/** Locate manifest.yaml at the root, or inside exactly one top-level
 * directory (the layout buildCollectionZip produces), and rebase all entry
 * paths relative to it. Entries outside the manifest's directory are
 * dropped. Throws with a user-facing message when no unambiguous manifest
 * exists. */
export function ingest(entries: CollectionEntry[]): RawCollection {
  for (const e of entries) {
    assertSafePath(e.path);
  }
  const candidates = entries.filter((e) => basename(e.path) === 'manifest.yaml' && depth(e.path) <= 1);
  if (candidates.length === 0) {
    throw new Error('No manifest.yaml found at the collection root or in a single top-level folder.');
  }
  if (candidates.length > 1) {
    throw new Error(
      `Multiple manifests found (${candidates.map((c) => c.path).join(', ')}); open a single collection.`,
    );
  }
  const manifest = candidates[0];
  const prefix = manifest.path.slice(0, -'manifest.yaml'.length);
  const files = entries
    .filter((e) => e.path !== manifest.path && e.path.startsWith(prefix))
    .map((e) => ({ path: e.path.slice(prefix.length), contents: e.contents }));
  return { manifestYaml: manifest.contents, files };
}

/** YAML entries from a zip archive; directory entries and non-YAML skipped. */
export function entriesFromZip(data: Uint8Array): CollectionEntry[] {
  const decoder = new TextDecoder();
  return Object.entries(unzipSync(data))
    .filter(([path]) => !path.endsWith('/') && isYamlPath(path))
    .map(([path, bytes]) => ({ path, contents: decoder.decode(bytes) }));
}

/** YAML entries from a webkitdirectory FileList; the picked folder's own
 * name (the first webkitRelativePath segment) is stripped so the manifest
 * sits at the root. */
export async function entriesFromFileList(list: FileList): Promise<CollectionEntry[]> {
  const files = Array.from(list).filter((f) => isYamlPath(f.webkitRelativePath));
  return Promise.all(
    files.map(async (f) => ({
      path: f.webkitRelativePath.split('/').slice(1).join('/'),
      contents: await f.text(),
    })),
  );
}
