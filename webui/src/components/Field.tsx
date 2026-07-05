// ABOUTME: Shared labeled-input control used across the manifest and table editors.
// ABOUTME: Also exports `cx`, a tiny class-name joiner used throughout the editor components.

export function cx(...parts: Array<string | false | undefined>): string {
  return parts.filter(Boolean).join(' ');
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
