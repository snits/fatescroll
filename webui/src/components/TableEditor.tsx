// ABOUTME: Center-pane editor for a simple or compound table: name/stem/type/tags,
// ABOUTME: the simple body (roll, modifier range, results, notes) or the compound body.

import { useEngine } from '../engine/useEngine';
import type { DiceInfo } from '../engine/engine';
import { CompoundEditor } from './CompoundEditor';
import { cx, isNumericDraft } from './Field';
import { ResultCard } from './ResultCard';
import { autofillRanges } from '../logic/autofill';
import { uid } from '../model/ids';
import { fqidOf, refResolves, useForgeStore } from '../model/store';
import type { ResultDraft, TableDraft } from '../model/types';

function numOr0(value: string): number {
  const n = parseInt(value, 10);
  return Number.isNaN(n) ? 0 : n;
}

function rollInfoText(info: DiceInfo, roll: string): { text: string; error: boolean } {
  if (!info.ok) return { text: 'unparseable dice expression', error: true };
  const outcomes = info.outcomes ?? 0;
  const plural = outcomes === 1 ? '' : 's';
  if (info.kind === 'digit') {
    return { text: `${roll.toUpperCase()} · ${outcomes} outcome${plural} (${info.min}–${info.max})`, error: false };
  }
  return { text: `range ${info.min}–${info.max} · ${outcomes} outcome${plural}`, error: false };
}

function notesFromText(text: string): string[] {
  const lines = text.split('\n').map((l) => l.replace(/\r$/, ''));
  while (lines.length > 0 && lines[lines.length - 1] === '') lines.pop();
  return lines;
}

export function TableEditor() {
  const { engine } = useEngine();
  const selUid = useForgeStore((s) => s.selUid);
  const tables = useForgeStore((s) => s.tables);
  const dirs = useForgeStore((s) => s.dirs);
  const updateTable = useForgeStore((s) => s.updateTable);
  const deleteTable = useForgeStore((s) => s.deleteTable);

  const found = tables.find((t) => t.uid === selUid);
  if (!found) return null;
  // Closures below capture `current` instead of `found` so TypeScript keeps
  // the non-undefined narrowing across the nested function declarations.
  const current: TableDraft = found;

  const namespace = dirs.find((d) => d.id === current.dirId)?.namespace ?? '';
  const fqid = fqidOf({ dirs }, current);

  function patch(p: Partial<Omit<TableDraft, 'uid'>>) {
    updateTable(current.uid, p);
  }

  function isUnresolved(ref: string): boolean {
    return !refResolves({ dirs, tables }, namespace, ref);
  }

  function handleDelete() {
    if (window.confirm(`Delete table "${current.name}"?`)) {
      deleteTable(current.uid);
    }
  }

  const info = current.type === 'simple' ? engine.diceInfo(current.roll) : { ok: false };
  const rollInfo = current.type === 'simple' ? rollInfoText(info, current.roll) : null;
  const digitKind = info.ok && info.kind === 'digit';
  const modActive = current.modOn && !digitKind;
  const effSpan =
    modActive && info.ok
      ? `${(info.min ?? 0) + numOr0(current.modMin)}–${(info.max ?? 0) + numOr0(current.modMax)}`
      : '';
  const expectedValues =
    current.type === 'simple'
      ? engine.expectedValues(current.roll, current.modOn, numOr0(current.modMin), numOr0(current.modMax))
      : null;
  const dist = current.type === 'simple' ? engine.histogram(current.roll) : null;

  function handleAutoFill() {
    if (expectedValues === null) return;
    const kind = digitKind ? 'digit' : 'contiguous';
    patch({ results: autofillRanges(current.results, expectedValues, kind) });
  }

  function addResult() {
    const result: ResultDraft = { rid: uid(), min: '', max: '', text: '', chain: [] };
    patch({ results: [...current.results, result] });
  }

  function patchResult(rid: string, resultPatch: Partial<Omit<ResultDraft, 'rid'>>) {
    patch({ results: current.results.map((r) => (r.rid === rid ? { ...r, ...resultPatch } : r)) });
  }

  function removeResult(rid: string) {
    patch({ results: current.results.filter((r) => r.rid !== rid) });
  }

  function addTableRef() {
    patch({ tableRefs: [...current.tableRefs, { rid: uid(), ref: '' }] });
  }

  function patchTableRef(rid: string, ref: string) {
    patch({ tableRefs: current.tableRefs.map((r) => (r.rid === rid ? { ...r, ref } : r)) });
  }

  function removeTableRef(rid: string) {
    patch({ tableRefs: current.tableRefs.filter((r) => r.rid !== rid) });
  }

  return (
    <div className="table-editor">
      <div className="table-editor-header">
        <label className="field-label table-editor-name-label">
          <span className="field-label-text">Display name</span>
          <input
            className={cx('field-input', 'table-editor-name-input')}
            value={current.name}
            onChange={(e) => patch({ name: e.target.value })}
          />
        </label>
        <button type="button" className="table-editor-delete" title="Delete table" onClick={handleDelete}>
          Delete
        </button>
      </div>

      <div className="table-editor-file-row">
        <label className="field-label">
          <span className="field-label-text">
            File / id{' '}
            <span className="field-label-optional">— becomes {current.stem}.yaml</span>
          </span>
          <input
            className="field-input field-input--mono"
            value={current.stem}
            onChange={(e) => patch({ stem: e.target.value })}
          />
        </label>
        <div className="table-editor-type">
          <span className="field-label-text">Type</span>
          <div className="table-editor-type-toggle">
            <button
              type="button"
              className={cx('table-editor-type-btn', current.type === 'simple' && 'table-editor-type-btn--active')}
              onClick={() => patch({ type: 'simple' })}
            >
              simple
            </button>
            <button
              type="button"
              className={cx('table-editor-type-btn', current.type === 'compound' && 'table-editor-type-btn--active')}
              onClick={() => patch({ type: 'compound' })}
            >
              compound
            </button>
          </div>
        </div>
      </div>

      <div className="table-editor-fqid">
        FQID · <span className="table-editor-fqid-value">{fqid}</span>
      </div>

      <label className="field-label table-editor-tags">
        <span className="field-label-text">
          Tags <span className="field-label-optional">(comma separated)</span>
        </span>
        <input
          className="field-input"
          placeholder="encounter, wilderness"
          value={current.tags.join(', ')}
          onChange={(e) =>
            patch({
              tags: e.target.value
                .split(',')
                .map((t) => t.trim())
                .filter(Boolean),
            })
          }
        />
      </label>

      {current.type === 'simple' && rollInfo ? (
        <>
          <div className="table-editor-roll-row">
            <label className="field-label">
              <span className="field-label-text">Roll</span>
              <input
                className={cx(
                  'field-input',
                  'field-input--mono',
                  'table-editor-roll-input',
                  rollInfo.error && 'field-input--invalid',
                )}
                value={current.roll}
                onChange={(e) => patch({ roll: e.target.value })}
              />
            </label>
            <div className={cx('table-editor-roll-info', rollInfo.error && 'table-editor-roll-info--error')}>
              {rollInfo.text}
            </div>
          </div>

          <div className="table-editor-mod-row">
            <label className="table-editor-mod-checkbox">
              <input
                type="checkbox"
                checked={modActive}
                disabled={digitKind}
                onChange={(e) => patch({ modOn: e.target.checked })}
              />
              <span>modifier_range</span>
            </label>
            {modActive && (
              <div className="table-editor-mod-range">
                <span>[</span>
                <input
                  className="field-input table-editor-mod-input"
                  aria-label="modifier minimum"
                  value={current.modMin}
                  onChange={(e) => {
                    if (isNumericDraft(e.target.value)) patch({ modMin: e.target.value });
                  }}
                />
                <span>,</span>
                <input
                  className="field-input table-editor-mod-input"
                  aria-label="modifier maximum"
                  value={current.modMax}
                  onChange={(e) => {
                    if (isNumericDraft(e.target.value)) patch({ modMax: e.target.value });
                  }}
                />
                <span>]</span>
                <span className="table-editor-mod-span">effective span {effSpan}</span>
              </div>
            )}
          </div>

          <div className="table-editor-section-header">
            <span className="table-editor-section-title">RESULTS</span>
            <div className="table-editor-section-actions">
              <button
                type="button"
                className="table-editor-autofill-btn"
                title="Distribute ranges to cover the dice with no gaps"
                disabled={expectedValues === null}
                onClick={handleAutoFill}
              >
                ⚖ Auto-fill ranges
              </button>
              <button type="button" className="table-editor-add-btn" onClick={addResult}>
                + result
              </button>
            </div>
          </div>

          {current.results.map((result) => (
            <ResultCard
              key={result.rid}
              result={result}
              dist={dist}
              modOn={modActive}
              isUnresolved={isUnresolved}
              onPatch={(p) => patchResult(result.rid, p)}
              onRemove={() => removeResult(result.rid)}
            />
          ))}

          <label className="field-label table-editor-notes">
            <span className="field-label-text">
              Notes <span className="field-label-optional">(one per line, optional)</span>
            </span>
            <textarea
              rows={2}
              placeholder="GM guidance, provenance, usage…"
              value={current.notes.join('\n')}
              onChange={(e) => patch({ notes: notesFromText(e.target.value) })}
            />
          </label>
        </>
      ) : (
        <CompoundEditor
          tableRefs={current.tableRefs}
          isUnresolved={isUnresolved}
          onAdd={addTableRef}
          onPatch={patchTableRef}
          onRemove={removeTableRef}
        />
      )}
    </div>
  );
}
