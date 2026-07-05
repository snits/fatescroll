// ABOUTME: Header bar for Table Forge: brand block, collection name, live
// ABOUTME: validation status pill, and the (disabled until Task 11) export button.

import type { ManifestState } from '../model/types';

export interface HeaderBarProps {
  manifest: ManifestState;
  errorCount: number;
}

export function HeaderBar({ manifest, errorCount }: HeaderBarProps) {
  const valid = errorCount === 0;
  const statusText = valid
    ? 'Collection is valid'
    : `${errorCount} ${errorCount === 1 ? 'error' : 'errors'}`;

  return (
    <header className="app-header">
      <div className="header-brand">
        <div className="header-brand-name">Fatescroll</div>
        <div className="header-brand-tag">TABLE FORGE</div>
      </div>
      <div className="header-divider" />
      <div className="header-collection">
        <span className="header-collection-label">Collection</span>
        <span className="header-collection-name">{manifest.name}</span>
      </div>
      <div className="header-spacer" />
      <div className={`header-status ${valid ? 'header-status--valid' : 'header-status--invalid'}`}>
        <span className="header-status-dot" />
        <span className="header-status-text">{statusText}</span>
      </div>
      <button type="button" className="header-export" disabled>
        Export collection ▾
      </button>
    </header>
  );
}
