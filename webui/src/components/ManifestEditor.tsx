// ABOUTME: Center-pane editor for the collection manifest: name/version/namespace/
// ABOUTME: author/min-tool-version fields plus the directory-to-namespace map.

import { isValidDirPath } from '../logic/dirpath';
import { isValidNamespace } from '../logic/namespace';
import { tablesInDir, useForgeStore } from '../model/store';
import type { Dir } from '../model/types';
import { cx, Field } from './Field';

function DirRow({
  dir,
  tableCount,
  onUpdate,
  onDelete,
}: {
  dir: Dir;
  tableCount: number;
  onUpdate: (patch: Partial<Omit<Dir, 'id'>>) => void;
  onDelete: () => void;
}) {
  function handleDelete() {
    if (tableCount > 0) {
      const ok = window.confirm(`Delete directory "${dir.path}/" and its ${tableCount} table(s)?`);
      if (!ok) return;
    }
    onDelete();
  }

  return (
    <div className="manifest-editor-dir-row">
      <label className="field-label field-label--sm">
        <span className="field-label-text">path</span>
        <input
          className={cx('field-input', 'field-input--mono', !isValidDirPath(dir.path) && 'field-input--invalid')}
          value={dir.path}
          onChange={(e) => onUpdate({ path: e.target.value })}
        />
      </label>
      <label className="field-label field-label--sm">
        <span className="field-label-text">namespace</span>
        <input
          className={cx('field-input', 'field-input--mono', !isValidNamespace(dir.namespace) && 'field-input--invalid')}
          value={dir.namespace}
          onChange={(e) => onUpdate({ namespace: e.target.value })}
        />
      </label>
      <button
        type="button"
        className="manifest-editor-dir-delete"
        title="Remove directory"
        aria-label={`Remove directory ${dir.path || dir.namespace}`}
        onClick={handleDelete}
      >
        ✕
      </button>
    </div>
  );
}

export function ManifestEditor() {
  const manifest = useForgeStore((s) => s.manifest);
  const dirs = useForgeStore((s) => s.dirs);
  const tables = useForgeStore((s) => s.tables);
  const setManifest = useForgeStore((s) => s.setManifest);
  const addDir = useForgeStore((s) => s.addDir);
  const updateDir = useForgeStore((s) => s.updateDir);
  const deleteDir = useForgeStore((s) => s.deleteDir);

  return (
    <div className="manifest-editor">
      <div className="manifest-editor-title">Collection Manifest</div>
      <div className="manifest-editor-subtitle">
        Metadata and the directory-to-namespace map. Emitted as{' '}
        <span className="manifest-editor-subtitle-mono">manifest.yaml</span>.
      </div>

      <div className="manifest-editor-fields">
        <Field label="Name" value={manifest.name} onChange={(name) => setManifest({ name })} />
        <Field
          label="Version"
          mono
          value={manifest.version}
          onChange={(version) => setManifest({ version })}
        />
        <Field
          label="Root namespace"
          mono
          invalid={!isValidNamespace(manifest.namespace)}
          value={manifest.namespace}
          onChange={(namespace) => setManifest({ namespace })}
        />
        <Field
          label="Author"
          optional
          placeholder="~"
          value={manifest.author}
          onChange={(author) => setManifest({ author })}
        />
        <Field
          label="Min tool version"
          optional
          mono
          placeholder="~"
          value={manifest.minToolVersion}
          onChange={(minToolVersion) => setManifest({ minToolVersion })}
        />
      </div>

      <div className="manifest-editor-dirs-header">
        <span className="manifest-editor-dirs-title">DIRECTORIES</span>
        <button type="button" className="manifest-editor-dirs-add" onClick={addDir}>
          + add
        </button>
      </div>

      {dirs.map((dir) => (
        <DirRow
          key={dir.id}
          dir={dir}
          tableCount={tablesInDir({ tables }, dir.id).length}
          onUpdate={(patch) => updateDir(dir.id, patch)}
          onDelete={() => deleteDir(dir.id)}
        />
      ))}
    </div>
  );
}
