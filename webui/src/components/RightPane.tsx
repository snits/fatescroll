// ABOUTME: Right pane: the current view's YAML viewer, the validation summary,
// ABOUTME: and the test-roll section, stacked in a vertical flex column.

import { DiceRoller } from './DiceRoller';
import { ValidationPanel } from './ValidationPanel';
import { YamlViewer } from './YamlViewer';

export interface RightPaneProps {
  title: string;
  yaml: string;
  errors: string[];
}

export function RightPane({ title, yaml, errors }: RightPaneProps) {
  return (
    <div className="right-pane">
      <YamlViewer title={title} yaml={yaml} />
      <ValidationPanel errors={errors} />
      <DiceRoller />
    </div>
  );
}
