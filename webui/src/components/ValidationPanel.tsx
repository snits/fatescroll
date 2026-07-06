// ABOUTME: Right-pane validation summary: a green "valid" line when the engine
// ABOUTME: reports no errors, otherwise one mono error line per message.

export function ValidationPanel({ errors }: { errors: string[] }) {
  return (
    <div className="validation-panel">
      <div className="validation-panel-header">VALIDATION</div>
      <div className="validation-panel-subtitle">fatescroll validate</div>
      {errors.length === 0 ? (
        <div className="validation-panel-ok">✓ Collection is valid.</div>
      ) : (
        <ul className="validation-panel-list">
          {errors.map((error, i) => (
            <li key={i} className="validation-panel-item">
              ✕ {error}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
