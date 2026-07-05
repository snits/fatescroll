// ABOUTME: One result row on a simple table: its range, probability pill, text,
// ABOUTME: and chain-to-table entries.

import { ChainRow } from './ChainRow';
import { cx, isNumericDraft } from './Field';
import { formatPct, rangeProbability } from '../logic/probability';
import { uid } from '../model/ids';
import type { ChainDraft, ResultDraft } from '../model/types';

export function ResultCard({
  result,
  dist,
  modOn,
  isUnresolved,
  onPatch,
  onRemove,
}: {
  result: ResultDraft;
  dist: [number, number][] | null;
  modOn: boolean;
  isUnresolved: (ref: string) => boolean;
  onPatch: (patch: Partial<Omit<ResultDraft, 'rid'>>) => void;
  onRemove: () => void;
}) {
  const min = parseInt(result.min, 10);
  const max = parseInt(result.max, 10);
  const probability = modOn ? '—' : formatPct(rangeProbability(dist, min, max));

  function patchChain(rid: string, patch: Partial<Omit<ChainDraft, 'rid'>>) {
    onPatch({ chain: result.chain.map((c) => (c.rid === rid ? { ...c, ...patch } : c)) });
  }

  function removeChain(rid: string) {
    onPatch({ chain: result.chain.filter((c) => c.rid !== rid) });
  }

  function addChain() {
    onPatch({ chain: [...result.chain, { rid: uid(), struct: false, ref: '', reroll: [] }] });
  }

  return (
    <div className="result-card">
      <div className="result-card-top">
        <div className="result-card-range">
          <input
            className="field-input field-input--mono result-card-range-input"
            inputMode="numeric"
            value={result.min}
            onChange={(e) => {
              if (isNumericDraft(e.target.value)) onPatch({ min: e.target.value });
            }}
          />
          <span className="result-card-range-sep">–</span>
          <input
            className="field-input field-input--mono result-card-range-input"
            inputMode="numeric"
            value={result.max}
            onChange={(e) => {
              if (isNumericDraft(e.target.value)) onPatch({ max: e.target.value });
            }}
          />
        </div>
        <span className="result-card-prob" title="probability on the base roll">
          {probability}
        </span>
        <div className="result-card-spacer" />
        <button
          type="button"
          className="result-card-remove"
          title="Remove result"
          aria-label="Remove result"
          onClick={onRemove}
        >
          ✕
        </button>
      </div>
      <input
        className={cx('field-input', 'result-card-text')}
        placeholder="Result text — supports {2d6x10} dice interpolation"
        value={result.text}
        onChange={(e) => onPatch({ text: e.target.value })}
      />
      <div className="result-card-chain">
        {result.chain.map((chain) => (
          <ChainRow
            key={chain.rid}
            chain={chain}
            unresolved={chain.ref !== '' && isUnresolved(chain.ref)}
            onPatch={(patch) => patchChain(chain.rid, patch)}
            onRemove={() => removeChain(chain.rid)}
          />
        ))}
        <button type="button" className="result-card-add-chain" onClick={addChain}>
          + chain to table
        </button>
      </div>
    </div>
  );
}
