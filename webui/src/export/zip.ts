// ABOUTME: Packs the collection's manifest and table YAML into a zip archive
// ABOUTME: rooted at a slug directory, mirroring the on-disk layout fatescroll expects.

import { zipSync, strToU8 } from 'fflate';
import type { FileInput } from '../yaml/emit';

// Zip-slip guard: an entry with `..` segments extracts outside the target
// directory; an absolute path produces a broken/non-portable archive. Dir
// paths are user-typed, so this is the enforcing check behind the editor's
// red-border affordance (isValidDirPath).
function assertSafePath(path: string): void {
  if (path.startsWith('/') || path.split('/').includes('..')) {
    throw new Error(`Cannot export: unsafe directory path "${path}"`);
  }
}

export function buildCollectionZip(
  slug: string,
  manifestYaml: string,
  files: FileInput[],
): Uint8Array<ArrayBuffer> {
  const entries: Record<string, Uint8Array> = {
    [`${slug}/manifest.yaml`]: strToU8(manifestYaml),
  };
  for (const f of files) {
    assertSafePath(f.path);
    const entryPath = `${slug}/${f.path}`;
    if (entryPath in entries) {
      throw new Error(`Cannot export: duplicate file path "${entryPath}"`);
    }
    entries[entryPath] = strToU8(f.contents);
  }
  return zipSync(entries);
}
