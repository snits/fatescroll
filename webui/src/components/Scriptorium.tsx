// ABOUTME: Left-rail "Scriptorium" tree: manifest node, directories, and their
// ABOUTME: tables, wired directly to the Table Forge store for selection and creation.

import { tablesInDir, useForgeStore } from '../model/store';
import type { Dir, TableDraft, View } from '../model/types';

function TreeTable({
  table,
  selected,
  onSelect,
}: {
  table: TableDraft;
  selected: boolean;
  onSelect: () => void;
}) {
  const typeShort = table.type === 'compound' ? 'cmp' : 'smp';
  const badgeClass =
    table.type === 'compound' ? 'tree-table-badge tree-table-badge--cmp' : 'tree-table-badge';

  return (
    <button
      type="button"
      className={selected ? 'tree-table selected' : 'tree-table'}
      onClick={onSelect}
    >
      <span className={badgeClass}>{typeShort}</span>
      <span className="tree-table-name">{table.name}</span>
    </button>
  );
}

function TreeDir({
  dir,
  tables,
  selUid,
  view,
  onSelectManifest,
  onSelectTable,
  onAddTable,
}: {
  dir: Dir;
  tables: TableDraft[];
  selUid: string | null;
  view: View;
  onSelectManifest: () => void;
  onSelectTable: (uid: string) => void;
  onAddTable: (dirId: string) => void;
}) {
  return (
    <div className="tree-dir">
      <div className="tree-dir-header">
        <button type="button" className="tree-dir-label" onClick={onSelectManifest}>
          <span className="tree-dir-path">{dir.path}/</span>
          <span className="tree-dir-ns">{dir.namespace}</span>
        </button>
        <button
          type="button"
          className="tree-dir-add"
          aria-label={`Add table to ${dir.path || dir.namespace}`}
          onClick={() => onAddTable(dir.id)}
        >
          +
        </button>
      </div>
      {tables.map((table) => (
        <TreeTable
          key={table.uid}
          table={table}
          selected={view === 'table' && table.uid === selUid}
          onSelect={() => onSelectTable(table.uid)}
        />
      ))}
    </div>
  );
}

export function Scriptorium() {
  const dirs = useForgeStore((s) => s.dirs);
  const tables = useForgeStore((s) => s.tables);
  const view = useForgeStore((s) => s.view);
  const selUid = useForgeStore((s) => s.selUid);
  const selectManifest = useForgeStore((s) => s.selectManifest);
  const selectTable = useForgeStore((s) => s.selectTable);
  const addDir = useForgeStore((s) => s.addDir);
  const addTable = useForgeStore((s) => s.addTable);

  return (
    <aside className="pane-left">
      <div className="scriptorium-header">
        <span className="scriptorium-title">SCRIPTORIUM</span>
      </div>
      <div className="scriptorium-body">
        <button
          type="button"
          className={view === 'manifest' ? 'tree-manifest selected' : 'tree-manifest'}
          onClick={selectManifest}
        >
          <span className="tree-manifest-glyph">⚜</span>
          <span className="tree-manifest-name">manifest.yaml</span>
        </button>

        {dirs.map((dir) => (
          <TreeDir
            key={dir.id}
            dir={dir}
            tables={tablesInDir({ tables }, dir.id)}
            selUid={selUid}
            view={view}
            onSelectManifest={selectManifest}
            onSelectTable={selectTable}
            onAddTable={addTable}
          />
        ))}

        <button type="button" className="tree-add-dir" onClick={addDir}>
          + add directory
        </button>
      </div>
    </aside>
  );
}
