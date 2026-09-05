// ABOUTME: One named value row on a result card: its name, its expression,
// ABOUTME: move-up/move-down reordering, and removal.

import type { BindingDraft } from '../model/types';

export function BindingRow({
  binding,
  index,
  isFirst,
  isLast,
  onPatch,
  onRemove,
  onMoveUp,
  onMoveDown,
}: {
  binding: BindingDraft;
  index: number;
  isFirst: boolean;
  isLast: boolean;
  onPatch: (patch: Partial<Omit<BindingDraft, 'rid'>>) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}) {
  const identity = `Value ${index + 1}`;
  return (
    <div className="binding-row">
      <span className="binding-row-glyph">=</span>
      <input
        className="field-input field-input--mono binding-row-name"
        placeholder="name"
        aria-label={`${identity} name`}
        value={binding.name}
        onChange={(e) => onPatch({ name: e.target.value })}
      />
      <input
        className="field-input field-input--mono binding-row-expression"
        placeholder="expression"
        aria-label={`${identity} expression`}
        value={binding.value}
        onChange={(e) => onPatch({ value: e.target.value })}
      />
      <button
        type="button"
        className="binding-row-move"
        title="Move value up"
        aria-label={`Move ${identity} up`}
        disabled={isFirst}
        onClick={onMoveUp}
      >
        ↑
      </button>
      <button
        type="button"
        className="binding-row-move"
        title="Move value down"
        aria-label={`Move ${identity} down`}
        disabled={isLast}
        onClick={onMoveDown}
      >
        ↓
      </button>
      <button
        type="button"
        className="binding-row-remove"
        title="Remove value"
        aria-label={`Remove ${identity}`}
        onClick={onRemove}
      >
        ✕
      </button>
    </div>
  );
}
