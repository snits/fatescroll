// ABOUTME: Right-pane YAML viewer: dark <pre> panel for the current view's
// ABOUTME: emitted YAML, plus copy-to-clipboard and download-as-file buttons.

import { useEffect, useRef, useState } from 'react';
import { useForgeStore } from '../model/store';
import { manifestYaml, tableYaml } from '../yaml/emit';
import type { Dir, ManifestState, TableDraft, View } from '../model/types';

const COPIED_LABEL_MS = 1400;

/** Which file the download button writes: the selected table's YAML on the
 * table view, else the manifest's. Pure so it's testable without a DOM click. */
export function downloadTarget(
  view: View,
  selUid: string | null,
  state: { manifest: ManifestState; dirs: Dir[]; tables: TableDraft[] },
): { filename: string; content: string } {
  if (view === 'table') {
    const table = state.tables.find((t) => t.uid === selUid);
    if (table) return { filename: `${table.stem}.yaml`, content: tableYaml(table) };
  }
  return { filename: 'manifest.yaml', content: manifestYaml(state.manifest, state.dirs) };
}

export function YamlViewer({ title, yaml }: { title: string; yaml: string }) {
  const [copied, setCopied] = useState(false);
  const revertTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (revertTimer.current) clearTimeout(revertTimer.current);
    },
    [],
  );

  async function handleCopy() {
    await navigator.clipboard.writeText(yaml);
    setCopied(true);
    if (revertTimer.current) clearTimeout(revertTimer.current);
    revertTimer.current = setTimeout(() => setCopied(false), COPIED_LABEL_MS);
  }

  function handleDownload() {
    const { view, selUid, manifest, dirs, tables } = useForgeStore.getState();
    const { filename, content } = downloadTarget(view, selUid, { manifest, dirs, tables });
    const url = URL.createObjectURL(new Blob([content], { type: 'text/yaml' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <div className="yaml-viewer">
      <div className="yaml-viewer-header">
        <span className="yaml-viewer-title">{title}</span>
        <button type="button" className="yaml-viewer-copy" onClick={handleCopy}>
          {copied ? '✓ copied' : '⧉ copy'}
        </button>
        <button type="button" className="yaml-viewer-download" onClick={handleDownload}>
          ⬇ .yaml
        </button>
      </div>
      <div className="yaml-viewer-panel">
        <pre className="yaml-viewer-pre">{yaml}</pre>
      </div>
    </div>
  );
}
