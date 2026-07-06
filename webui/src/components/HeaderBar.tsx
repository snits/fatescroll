// ABOUTME: Header bar for Table Forge: brand block, collection name, live
// ABOUTME: validation status pill, and the collection zip export button.

import { buildCollectionZip } from '../export/zip';
import { collectionSlug } from '../logic/slug';
import { useForgeStore } from '../model/store';
import { collectionFiles, manifestYaml } from '../yaml/emit';

export interface HeaderBarProps {
  collectionName: string;
  errorCount: number;
}

export function HeaderBar({ collectionName, errorCount }: HeaderBarProps) {
  const valid = errorCount === 0;
  const statusText = valid
    ? 'Collection is valid'
    : `${errorCount} ${errorCount === 1 ? 'error' : 'errors'}`;

  function handleExport() {
    const { manifest, dirs, tables } = useForgeStore.getState();
    const slug = collectionSlug(manifest.name);
    const zip = buildCollectionZip(slug, manifestYaml(manifest, dirs), collectionFiles(dirs, tables));
    const url = URL.createObjectURL(new Blob([zip], { type: 'application/zip' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${slug}.zip`;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <header className="app-header">
      <div className="header-brand">
        <div className="header-brand-name">Fatescroll</div>
        <div className="header-brand-tag">TABLE FORGE</div>
      </div>
      <div className="header-divider" />
      <div className="header-collection">
        <span className="header-collection-label">Collection</span>
        <span className="header-collection-name">{collectionName}</span>
      </div>
      <div className="header-spacer" />
      <div
        className={`header-status ${valid ? 'header-status--valid' : 'header-status--invalid'}`}
        aria-live="polite"
      >
        <span className="header-status-dot" />
        <span className="header-status-text">{statusText}</span>
      </div>
      <button type="button" className="header-export" onClick={handleExport}>
        Export collection ▾
      </button>
    </header>
  );
}
