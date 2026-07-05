// ABOUTME: Shared input controls for the editors: Field (labeled input), DraftText
// ABOUTME: (raw-draft input committed on blur), plus the cx class joiner and
// ABOUTME: isNumericDraft numeric-keystroke filter.

import { useRef, useState } from 'react';

export function cx(...parts: Array<string | false | undefined>): string {
  return parts.filter(Boolean).join(' ');
}

const NUMERIC_DRAFT = /^-?\d*$/;

/** True for a valid in-progress numeric input: digits with an optional leading
 * minus, including the transient lone "-" before more digits are typed. */
export function isNumericDraft(value: string): boolean {
  return NUMERIC_DRAFT.test(value);
}

/** Text control whose store value is a parsed form of what the user types
 * (comma-split tags, newline-split notes, csv reroll sets). Holds the raw
 * draft locally so separators aren't normalized away mid-keystroke, and
 * commits on blur — skipped when the draft is unchanged, so merely tabbing
 * through doesn't trigger store side effects (e.g. clearing the roll
 * preview). Mount it with a `key` tied to the entity it edits (table uid /
 * chain rid) so the draft re-initializes when the target changes.
 *
 * Invariant: while a DraftText is mounted, no other code path may write its
 * backing store field (tags/notes/reroll) — the draft would go stale and
 * clobber the external write on the next commit. External writers must
 * change the mount `key` so the draft re-initializes. */
export function DraftText({
  multiline,
  className,
  placeholder,
  rows,
  initial,
  onCommit,
}: {
  multiline?: boolean;
  className?: string;
  placeholder?: string;
  rows?: number;
  initial: string;
  onCommit: (raw: string) => void;
}) {
  const [draft, setDraft] = useState(initial);
  const committed = useRef(initial);
  const props = {
    className,
    placeholder,
    value: draft,
    onChange: (e: { target: { value: string } }) => setDraft(e.target.value),
    onBlur: () => {
      if (draft === committed.current) return;
      committed.current = draft;
      onCommit(draft);
    },
  };
  return multiline ? <textarea rows={rows} {...props} /> : <input {...props} />;
}

export function Field({
  label,
  optional,
  mono,
  invalid,
  placeholder,
  value,
  onChange,
}: {
  label: string;
  optional?: boolean;
  mono?: boolean;
  invalid?: boolean;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="field-label">
      <span className="field-label-text">
        {label}
        {optional && <span className="field-label-optional"> (optional)</span>}
      </span>
      <input
        className={cx('field-input', mono && 'field-input--mono', invalid && 'field-input--invalid')}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}
