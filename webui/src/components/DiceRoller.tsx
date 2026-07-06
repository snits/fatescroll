// ABOUTME: "Cast the Dice" test roller: rolls the selected table against the
// ABOUTME: engine using freshly-read store state (never the debounced derived
// ABOUTME: values, so a roll right after an edit never uses stale YAML).

import { cx } from './Field';
import { flattenRoll } from '../logic/rollLines';
import { fqidOf, useForgeStore } from '../model/store';
import { useEngine } from '../engine/useEngine';
import { collectionFiles, manifestYaml } from '../yaml/emit';

export function DiceRoller() {
  const { engine } = useEngine();
  const view = useForgeStore((s) => s.view);
  const rollLines = useForgeStore((s) => s.rollLines);
  const setRollLines = useForgeStore((s) => s.setRollLines);

  function handleRoll() {
    const { manifest, dirs, tables, view: currentView, selUid } = useForgeStore.getState();
    if (currentView !== 'table') return;
    const table = tables.find((t) => t.uid === selUid);
    if (!table) return;

    const fqid = fqidOf({ dirs }, table);
    const result = engine.roll(manifestYaml(manifest, dirs), collectionFiles(dirs, tables), fqid);
    if ('error' in result) {
      setRollLines([{ indent: 0, text: result.error, error: true }]);
    } else {
      setRollLines(flattenRoll(result));
    }
  }

  return (
    <div className="dice-roller">
      <div className="dice-roller-header">
        <span className="dice-roller-label">CAST THE DICE</span>
        <button
          type="button"
          className="dice-roller-btn"
          disabled={view !== 'table'}
          onClick={handleRoll}
        >
          ⚄ Roll
        </button>
      </div>
      <div className="dice-roller-output">
        {rollLines === null ? (
          <div className="dice-roller-placeholder">— roll a table to preview an outcome —</div>
        ) : (
          rollLines.map((line, i) => (
            <div
              key={i}
              className={cx(
                'dice-roller-line',
                line.error ? 'dice-roller-line--error' : line.indent === 0 ? 'dice-roller-line--root' : 'dice-roller-line--nested',
              )}
              style={{ paddingLeft: line.indent * 18 }}
            >
              {line.text}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
