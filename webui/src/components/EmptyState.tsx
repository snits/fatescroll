// ABOUTME: Center-pane placeholder shown when nothing is selected: a glyph,
// ABOUTME: a title, and a helper line pointing at the scriptorium.

export function EmptyState() {
  return (
    <div className="empty-state">
      <div className="empty-state-glyph" aria-hidden="true">
        ✦
      </div>
      <div className="empty-state-title">Nothing selected</div>
      <div className="empty-state-helper">
        Pick a table from the scriptorium, or add one to a directory to begin inscribing.
      </div>
    </div>
  );
}
