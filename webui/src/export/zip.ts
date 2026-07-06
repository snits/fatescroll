// ABOUTME: Packs the collection's manifest and table YAML into a zip archive
// ABOUTME: rooted at a slug directory, mirroring the on-disk layout fatescroll expects.

import { zipSync, strToU8 } from 'fflate';
import type { FileInput } from '../yaml/emit';

export function buildCollectionZip(
  slug: string,
  manifestYaml: string,
  files: FileInput[],
): Uint8Array<ArrayBuffer> {
  const entries: Record<string, Uint8Array> = {
    [`${slug}/manifest.yaml`]: strToU8(manifestYaml),
  };
  for (const f of files) entries[`${slug}/${f.path}`] = strToU8(f.contents);
  return zipSync(entries);
}
