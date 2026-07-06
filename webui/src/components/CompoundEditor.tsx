// ABOUTME: Compound table body: the list of sub-table references rolled together
// ABOUTME: when the compound table is rolled.

import { cx } from './Field';

export function CompoundEditor({
  tableRefs,
  isUnresolved,
  onAdd,
  onPatch,
  onRemove,
}: {
  tableRefs: { rid: string; ref: string }[];
  isUnresolved: (ref: string) => boolean;
  onAdd: () => void;
  onPatch: (rid: string, ref: string) => void;
  onRemove: (rid: string) => void;
}) {
  return (
    <div>
      <div className="table-editor-section-header">
        <span className="table-editor-section-title">SUB-TABLES</span>
        <button type="button" className="table-editor-add-btn" onClick={onAdd}>
          + table
        </button>
      </div>
      <div className="compound-editor-note">
        Each sub-table is rolled when this compound table is rolled; results are combined.
      </div>
      {tableRefs.map((ref) => (
        <div className="compound-editor-row" key={ref.rid}>
          <span className="compound-editor-glyph">◈</span>
          <input
            className={cx(
              'field-input',
              'field-input--mono',
              'compound-editor-ref',
              ref.ref !== '' && isUnresolved(ref.ref) && 'field-input--invalid',
            )}
            placeholder="table reference"
            value={ref.ref}
            onChange={(e) => onPatch(ref.rid, e.target.value)}
          />
          <button
            type="button"
            className="compound-editor-remove"
            title="Remove"
            aria-label="Remove table reference"
            onClick={() => onRemove(ref.rid)}
          >
            ✕
          </button>
        </div>
      ))}
    </div>
  );
}
