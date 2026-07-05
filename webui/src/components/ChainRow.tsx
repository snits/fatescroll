// ABOUTME: One chain entry on a result card: a table reference plus an optional
// ABOUTME: structured reroll set, toggled by the ↺ button.

import { cx } from './Field';
import type { ChainDraft } from '../model/types';

function parseReroll(value: string): number[] {
  return value
    .split(',')
    .map((s) => parseInt(s.trim(), 10))
    .filter((n) => !Number.isNaN(n));
}

export function ChainRow({
  chain,
  unresolved,
  onPatch,
  onRemove,
}: {
  chain: ChainDraft;
  unresolved: boolean;
  onPatch: (patch: Partial<Omit<ChainDraft, 'rid'>>) => void;
  onRemove: () => void;
}) {
  return (
    <div className="chain-row">
      <span className="chain-row-glyph">↳</span>
      <input
        className={cx('field-input', 'field-input--mono', 'chain-row-ref', unresolved && 'field-input--invalid')}
        placeholder="table reference"
        value={chain.ref}
        onChange={(e) => onPatch({ ref: e.target.value })}
      />
      {chain.struct && (
        <div className="chain-row-reroll">
          <span className="chain-row-reroll-label">reroll</span>
          <input
            className="field-input field-input--mono chain-row-reroll-input"
            placeholder="1, 2"
            value={chain.reroll.join(', ')}
            onChange={(e) => onPatch({ reroll: parseReroll(e.target.value) })}
          />
        </div>
      )}
      <button
        type="button"
        className={cx('chain-row-toggle', chain.struct && 'chain-row-toggle--active')}
        title="Toggle reroll modifier"
        aria-label="Toggle reroll modifier"
        onClick={() => onPatch({ struct: !chain.struct })}
      >
        ↺
      </button>
      <button
        type="button"
        className="chain-row-remove"
        title="Remove chain"
        aria-label="Remove chain"
        onClick={onRemove}
      >
        ✕
      </button>
    </div>
  );
}
