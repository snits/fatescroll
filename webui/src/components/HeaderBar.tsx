// ABOUTME: Header bar for Table Forge: brand block, collection name, live
// ABOUTME: validation status pill, the open-collection control (zip or
// ABOUTME: folder), and the collection zip export button.

import { useRef, useState } from 'react';
import { useEngine } from '../engine/useEngine';
import { buildCollectionZip } from '../export/zip';
import { entriesFromFileList, entriesFromZip, ingest, type CollectionEntry } from '../import/ingest';
import { mapDrafts } from '../import/mapDrafts';
import { triggerDownload } from '../logic/download';
import { collectionSlug } from '../logic/slug';
import { useForgeStore } from '../model/store';
import { collectionFiles, manifestYaml } from '../yaml/emit';

export interface HeaderBarProps {
  collectionName: string;
  errorCount: number;
}

export function HeaderBar({ collectionName, errorCount }: HeaderBarProps) {
  const { engine } = useEngine();
  const [openMenu, setOpenMenu] = useState(false);
  const zipInputRef = useRef<HTMLInputElement>(null);
  const folderInputRef = useRef<HTMLInputElement>(null);
  const valid = errorCount === 0;
  const statusText = valid
    ? 'Collection is valid'
    : `${errorCount} ${errorCount === 1 ? 'error' : 'errors'}`;

  function handleExport() {
    const { manifest, dirs, tables } = useForgeStore.getState();
    const slug = collectionSlug(manifest.name);
    let zip: Uint8Array<ArrayBuffer>;
    try {
      zip = buildCollectionZip(slug, manifestYaml(manifest, dirs), collectionFiles(dirs, tables));
    } catch (err) {
      window.alert(String(err));
      return;
    }
    triggerDownload(`${slug}.zip`, new Blob([zip], { type: 'application/zip' }));
  }

  function openFromEntries(entries: CollectionEntry[]) {
    const { tables } = useForgeStore.getState();
    if (
      tables.length > 0 &&
      !window.confirm('Replace the current collection? Unexported work will be lost.')
    ) {
      return;
    }
    let raw;
    try {
      raw = ingest(entries);
    } catch (err) {
      window.alert(String(err instanceof Error ? err.message : err));
      return;
    }
    const outcome = engine.parseCollection(raw.manifestYaml, raw.files);
    if (!outcome.ok) {
      const shown = outcome.errors.slice(0, 10);
      const more = outcome.errors.length - shown.length;
      window.alert(
        `Cannot open collection:\n${shown.join('\n')}${more > 0 ? `\n…and ${more} more` : ''}`,
      );
      return;
    }
    if (outcome.collection.ignoredYaml.length > 0) {
      window.alert(
        `Opened with warnings — these YAML files are not listed in the manifest and will not be part of exports:\n${outcome.collection.ignoredYaml.join('\n')}`,
      );
    }
    // mapDrafts throws if a parsed table matches no manifest directory — a
    // drifted-contract guard against silent data loss on re-export.
    try {
      useForgeStore.getState().loadCollection(mapDrafts(outcome.collection));
    } catch (err) {
      window.alert(String(err instanceof Error ? err.message : err));
    }
  }

  async function handleZipPicked(e: React.ChangeEvent<HTMLInputElement>) {
    setOpenMenu(false);
    const file = e.target.files?.[0];
    e.target.value = '';
    if (!file) return;
    openFromEntries(entriesFromZip(new Uint8Array(await file.arrayBuffer())));
  }

  async function handleFolderPicked(e: React.ChangeEvent<HTMLInputElement>) {
    setOpenMenu(false);
    const list = e.target.files;
    if (!list || list.length === 0) return;
    const entries = await entriesFromFileList(list);
    e.target.value = '';
    openFromEntries(entries);
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
      <div className="header-open">
        <button type="button" className="header-export" onClick={() => setOpenMenu((o) => !o)}>
          Open collection ▾
        </button>
        {openMenu && (
          <div className="header-open-menu" role="menu">
            <button type="button" role="menuitem" onClick={() => zipInputRef.current?.click()}>
              From zip…
            </button>
            <button type="button" role="menuitem" onClick={() => folderInputRef.current?.click()}>
              From folder…
            </button>
          </div>
        )}
        <input
          ref={zipInputRef}
          data-testid="open-zip-input"
          type="file"
          accept=".zip"
          hidden
          onChange={handleZipPicked}
        />
        <input
          ref={(el) => {
            folderInputRef.current = el;
            if (el) el.webkitdirectory = true;
          }}
          data-testid="open-folder-input"
          type="file"
          hidden
          onChange={handleFolderPicked}
        />
      </div>
      <button type="button" className="header-export" onClick={handleExport}>
        Export collection ▾
      </button>
    </header>
  );
}
