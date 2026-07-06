// ABOUTME: Tests for the table editor: name/stem/type/tags binding, the simple body
// ABOUTME: (roll info, modifier range, auto-fill, results, chains, notes), the
// ABOUTME: compound body, delete confirmation, and App center-pane wiring.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { AppContent } from '../../src/App';
import { TableEditor } from '../../src/components/TableEditor';
import { EngineProvider } from '../../src/engine/useEngine';
import type { Engine } from '../../src/engine/engine';
import { initialState, useForgeStore } from '../../src/model/store';
import type { Dir, TableDraft } from '../../src/model/types';

afterEach(cleanup);

function makeDir(overrides: Partial<Dir> = {}): Dir {
  return { id: 'dir-1', path: 'npcs', namespace: 'ns', ...overrides };
}

function makeTable(overrides: Partial<TableDraft> = {}): TableDraft {
  return {
    uid: 'table-1',
    dirId: 'dir-1',
    stem: 'wandering-npc',
    name: 'Wandering NPC',
    type: 'simple',
    tags: [],
    roll: '1d6',
    modOn: false,
    modMin: '',
    modMax: '',
    notes: [],
    results: [],
    tableRefs: [],
    ...overrides,
  };
}

function makeEngine(overrides: Partial<Engine> = {}): Engine {
  return {
    validate: () => [],
    diceInfo: () => ({ ok: false }),
    expectedValues: () => null,
    histogram: () => null,
    roll: () => ({ error: 'not used' }),
    parseCollection: () => ({ ok: false, errors: ['not used'] }),
    ...overrides,
  };
}

function wrapper(engine: Engine) {
  return ({ children }: { children: ReactNode }) => (
    <EngineProvider engine={engine} debounceMs={0}>
      {children}
    </EngineProvider>
  );
}

function renderEditor(engine: Engine) {
  return render(<TableEditor />, { wrapper: wrapper(engine) });
}

beforeEach(() => {
  useForgeStore.setState(initialState());
});

describe('TableEditor header', () => {
  it('binds name and stem to the store, and shows the FQID line', () => {
    useForgeStore.setState({
      dirs: [makeDir({ namespace: 'fatescroll.npcs' })],
      tables: [makeTable({ name: 'Wandering NPC', stem: 'wandering-npc' })],
      selUid: 'table-1',
      view: 'table',
    });

    const { container } = renderEditor(makeEngine());

    expect((screen.getByLabelText(/^Display name$/) as HTMLInputElement).value).toBe('Wandering NPC');
    expect((screen.getByLabelText(/^File \/ id/) as HTMLInputElement).value).toBe('wandering-npc');
    expect(container.querySelector('.table-editor-fqid')?.textContent).toBe(
      'FQID · fatescroll.npcs.wandering-npc',
    );
  });

  it('typing in Display name updates the store', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    const input = screen.getByLabelText(/^Display name$/);
    await user.clear(input);
    await user.type(input, 'Renamed');

    expect(useForgeStore.getState().tables[0].name).toBe('Renamed');
  });

  it('typing in File / id updates the store stem', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    const input = screen.getByLabelText(/^File \/ id/);
    await user.clear(input);
    await user.type(input, 'renamed-stem');

    expect(useForgeStore.getState().tables[0].stem).toBe('renamed-stem');
  });

  it('re-initializes the Tags draft when a different table is selected', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [
        makeTable({ uid: 'table-1', tags: ['first'] }),
        makeTable({ uid: 'table-2', stem: 'other', tags: ['second'] }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());
    expect((screen.getByLabelText(/^Tags/) as HTMLInputElement).value).toBe('first');

    act(() => {
      useForgeStore.getState().selectTable('table-2');
    });

    expect((screen.getByLabelText(/^Tags/) as HTMLInputElement).value).toBe('second');
  });

  it('typing in Tags keystroke-by-keystroke commits comma-split tags on blur', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.type(screen.getByLabelText(/^Tags/), 'encounter, wilderness');
    await user.tab();

    expect(useForgeStore.getState().tables[0].tags).toEqual(['encounter', 'wilderness']);
  });

  it('tabbing through Tags without typing does not commit (roll preview survives)', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ tags: ['encounter'] })],
      selUid: 'table-1',
      view: 'table',
      rollLines: [{ indent: 0, text: 'a preview' }],
    });
    renderEditor(makeEngine());

    await user.click(screen.getByLabelText(/^Tags/));
    await user.tab();

    expect(useForgeStore.getState().rollLines).toEqual([{ indent: 0, text: 'a preview' }]);

    await user.click(screen.getByLabelText(/^Tags/));
    await user.type(screen.getByLabelText(/^Tags/), ', wilderness');
    await user.tab();

    expect(useForgeStore.getState().rollLines).toBeNull();
    expect(useForgeStore.getState().tables[0].tags).toEqual(['encounter', 'wilderness']);
  });
});

describe('TableEditor type toggle', () => {
  it('switching to compound and back to simple preserves the other type\'s fields', async () => {
    const user = userEvent.setup();
    const results = [{ rid: 'r1', min: '1', max: '6', text: 'hit', chain: [] }];
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ results })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: 'compound' }));
    expect(useForgeStore.getState().tables[0].type).toBe('compound');
    expect(useForgeStore.getState().tables[0].results).toEqual(results);

    await user.click(screen.getByRole('button', { name: 'simple' }));
    expect(useForgeStore.getState().tables[0].type).toBe('simple');
    expect(useForgeStore.getState().tables[0].results).toEqual(results);
  });
});

describe('TableEditor roll info', () => {
  it('shows a range info line for kind "range"', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: '2d6' })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: true, kind: 'range', min: 2, max: 12, outcomes: 11 }) }));

    expect(screen.getByText('range 2–12 · 11 outcomes')).toBeTruthy();
  });

  it('shows a D66-style info line for kind "digit"', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: 'd66' })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: true, kind: 'digit', min: 11, max: 66, outcomes: 36 }) }));

    expect(screen.getByText('D66 · 36 outcomes (11–66)')).toBeTruthy();
  });

  it('shows an error message and an invalid class for an unparseable expression', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: 'nonsense' })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: false }) }));

    expect(screen.getByText('unparseable dice expression')).toBeTruthy();
    expect(screen.getByLabelText(/^Roll$/).className).toContain('field-input--invalid');
  });
});

describe('TableEditor modifier range', () => {
  it('renders the modifier checkbox disabled and unchecked when kind is "digit"', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: 'd66', modOn: true })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: true, kind: 'digit', min: 11, max: 66, outcomes: 36 }) }));

    const checkbox = screen.getByRole('checkbox') as HTMLInputElement;
    expect(checkbox.disabled).toBe(true);
    expect(checkbox.checked).toBe(false);
  });

  it('accepts a negative modMin/modMax while typing', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: '1d8', modOn: true, modMin: '', modMax: '' })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: true, kind: 'range', min: 1, max: 8, outcomes: 8 }) }));

    const checkbox = screen.getByRole('checkbox') as HTMLInputElement;
    expect(checkbox.checked).toBe(true);
    expect(checkbox.disabled).toBe(false);

    const minInput = screen.getByLabelText('modifier minimum');
    await user.type(minInput, '-6');

    expect(useForgeStore.getState().tables[0].modMin).toBe('-6');
  });

  it('strips non-numeric characters typed into modMin', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: '1d8', modOn: true })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: true, kind: 'range', min: 1, max: 8, outcomes: 8 }) }));

    const minInput = screen.getByLabelText('modifier minimum');
    await user.type(minInput, 'a1b2');

    expect(useForgeStore.getState().tables[0].modMin).toBe('12');
  });
});

describe('TableEditor auto-fill', () => {
  it('derives contiguous kind and autofills ranges from expectedValues', async () => {
    const user = userEvent.setup();
    const results = [
      { rid: 'r1', min: '', max: '', text: '', chain: [] },
      { rid: 'r2', min: '', max: '', text: '', chain: [] },
    ];
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: '1d6', results })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(
      makeEngine({
        diceInfo: () => ({ ok: true, kind: 'range', min: 1, max: 6, outcomes: 6 }),
        expectedValues: () => [1, 2, 3, 4, 5, 6],
      }),
    );

    await user.click(screen.getByRole('button', { name: /Auto-fill ranges/ }));

    const updated = useForgeStore.getState().tables[0].results;
    expect(updated.map((r) => [r.min, r.max])).toEqual([
      ['1', '3'],
      ['4', '6'],
    ]);
  });

  it('derives digit kind for a D66 roll', async () => {
    const user = userEvent.setup();
    const results = [{ rid: 'r1', min: '', max: '', text: '', chain: [] }];
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: 'd66', results })],
      selUid: 'table-1',
      view: 'table',
    });
    const values = [11, 12, 13];
    renderEditor(
      makeEngine({
        diceInfo: () => ({ ok: true, kind: 'digit', min: 11, max: 66, outcomes: 36 }),
        expectedValues: () => values,
      }),
    );

    await user.click(screen.getByRole('button', { name: /Auto-fill ranges/ }));

    const updated = useForgeStore.getState().tables[0].results;
    expect(updated).toHaveLength(3);
    expect(updated.map((r) => [r.min, r.max])).toEqual([
      ['11', '11'],
      ['12', '12'],
      ['13', '13'],
    ]);
  });

  it('ignores a stale modOn on a digit roll: auto-fill stays enabled and fills per-value rows', async () => {
    const user = userEvent.setup();
    // modOn was left true from an earlier range roll; the checkbox is forced
    // off for digit rolls, so auto-fill must query the unmodified values.
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [
        makeTable({
          roll: 'd66',
          modOn: true,
          modMin: '1',
          modMax: '3',
          results: [{ rid: 'r1', min: '', max: '', text: '', chain: [] }],
        }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
    const values: number[] = [];
    for (let t = 1; t <= 6; t++) for (let u = 1; u <= 6; u++) values.push(t * 10 + u);
    renderEditor(
      makeEngine({
        diceInfo: () => ({ ok: true, kind: 'digit', min: 11, max: 66, outcomes: 36 }),
        // A modifier on a digit roll is invalid: only the modifier-off query succeeds.
        expectedValues: (_expr, modOn) => (modOn ? null : values),
      }),
    );

    const button = screen.getByRole('button', { name: /Auto-fill ranges/ }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);

    await user.click(button);

    const updated = useForgeStore.getState().tables[0].results;
    expect(updated).toHaveLength(36);
    expect(updated[0]).toMatchObject({ min: '11', max: '11' });
    expect(updated[35]).toMatchObject({ min: '66', max: '66' });
  });

  it('disables the auto-fill button when expectedValues is null', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ roll: 'nonsense' })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine({ diceInfo: () => ({ ok: false }), expectedValues: () => null }));

    expect((screen.getByRole('button', { name: /Auto-fill ranges/ }) as HTMLButtonElement).disabled).toBe(true);
  });
});

describe('TableEditor results', () => {
  it('shows a probability pill computed from the histogram', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [
        makeTable({
          roll: '2d6',
          results: [{ rid: 'r1', min: '2', max: '5', text: '', chain: [] }],
        }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(
      makeEngine({
        diceInfo: () => ({ ok: true, kind: 'range', min: 2, max: 12, outcomes: 11 }),
        histogram: () => [
          [2, 0.0278],
          [3, 0.0556],
          [4, 0.0833],
          [5, 0.1111],
          [6, 0.1389],
        ],
      }),
    );

    expect(screen.getByText('28%')).toBeTruthy();
  });

  it('shows an em dash for the probability pill when modifier_range is on', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [
        makeTable({
          roll: '2d6',
          modOn: true,
          modMin: '0',
          modMax: '0',
          results: [{ rid: 'r1', min: '2', max: '5', text: '', chain: [] }],
        }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(
      makeEngine({
        diceInfo: () => ({ ok: true, kind: 'range', min: 2, max: 12, outcomes: 11 }),
        histogram: () => [
          [2, 0.0278],
          [3, 0.0556],
        ],
      }),
    );

    expect(screen.getByTitle('probability on the base roll').textContent).toBe('—');
  });

  it('"+ result" adds a result card', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ results: [] })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: '+ result' }));

    expect(useForgeStore.getState().tables[0].results).toHaveLength(1);
  });

  it('✕ removes a result card', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [
        makeTable({
          results: [
            { rid: 'r1', min: '1', max: '3', text: '', chain: [] },
            { rid: 'r2', min: '4', max: '6', text: '', chain: [] },
          ],
        }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    const removeButtons = screen.getAllByRole('button', { name: 'Remove result' });
    await user.click(removeButtons[0]);

    const remaining = useForgeStore.getState().tables[0].results;
    expect(remaining).toHaveLength(1);
    expect(remaining[0].rid).toBe('r2');
  });
});

describe('TableEditor chains', () => {
  function setUpWithChainTarget(chainRef: string) {
    useForgeStore.setState({
      dirs: [makeDir({ id: 'dir-1', namespace: 'ns' })],
      tables: [
        makeTable({
          uid: 'table-1',
          dirId: 'dir-1',
          results: [
            {
              rid: 'r1',
              min: '1',
              max: '6',
              text: '',
              chain: [{ rid: 'c1', struct: false, ref: chainRef, reroll: [] }],
            },
          ],
        }),
        makeTable({ uid: 'table-2', dirId: 'dir-1', stem: 'other-table', name: 'Other' }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
  }

  it('shows no invalid class on a chain ref that resolves', () => {
    setUpWithChainTarget('other-table');
    renderEditor(makeEngine());

    expect(screen.getByPlaceholderText('table reference').className).not.toContain('field-input--invalid');
  });

  it('shows an invalid class on a chain ref that does not resolve', () => {
    setUpWithChainTarget('does-not-exist');
    renderEditor(makeEngine());

    expect(screen.getByPlaceholderText('table reference').className).toContain('field-input--invalid');
  });

  it('does not mark an empty chain ref as invalid', () => {
    setUpWithChainTarget('');
    renderEditor(makeEngine());

    expect(screen.getByPlaceholderText('table reference').className).not.toContain('field-input--invalid');
  });

  it('↺ toggle reveals a reroll input that parses comma-separated ints', async () => {
    const user = userEvent.setup();
    setUpWithChainTarget('other-table');
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: 'Toggle reroll modifier' }));
    expect(useForgeStore.getState().tables[0].results[0].chain[0].struct).toBe(true);

    const rerollInput = screen.getByPlaceholderText('1, 2');
    await user.type(rerollInput, '1, 2, 3');
    await user.tab();

    expect(useForgeStore.getState().tables[0].results[0].chain[0].reroll).toEqual([1, 2, 3]);
  });

  it('"+ chain to table" adds a chain entry', async () => {
    const user = userEvent.setup();
    setUpWithChainTarget('other-table');
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: '+ chain to table' }));

    expect(useForgeStore.getState().tables[0].results[0].chain).toHaveLength(2);
  });

  it('✕ removes a chain entry', async () => {
    const user = userEvent.setup();
    setUpWithChainTarget('other-table');
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: 'Remove chain' }));

    expect(useForgeStore.getState().tables[0].results[0].chain).toHaveLength(0);
  });
});

describe('TableEditor notes', () => {
  it('binds notes to one line per array entry', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ notes: ['line one', 'line two'] })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    expect((screen.getByLabelText(/^Notes/) as HTMLTextAreaElement).value).toBe('line one\nline two');
  });

  it('splits typed text into notes on blur, preserving interior blank lines', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ notes: [] })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.type(screen.getByLabelText(/^Notes/), 'a{Enter}{Enter}b');
    await user.tab();

    expect(useForgeStore.getState().tables[0].notes).toEqual(['a', '', 'b']);
  });

  it('drops trailing empty lines only', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ notes: [] })],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.type(screen.getByLabelText(/^Notes/), 'a{Enter}b{Enter}{Enter}{Enter}');
    await user.tab();

    expect(useForgeStore.getState().tables[0].notes).toEqual(['a', 'b']);
  });
});

describe('TableEditor delete', () => {
  it('deletes the table on confirm', async () => {
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    expect(useForgeStore.getState().tables).toHaveLength(0);
  });

  it('does not delete the table when confirm is declined', async () => {
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(false);
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      selUid: 'table-1',
      view: 'table',
    });
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    expect(useForgeStore.getState().tables).toHaveLength(1);
  });
});

describe('CompoundEditor', () => {
  function setUpCompound(ref: string) {
    useForgeStore.setState({
      dirs: [makeDir({ id: 'dir-1', namespace: 'ns' })],
      tables: [
        makeTable({
          uid: 'table-1',
          dirId: 'dir-1',
          type: 'compound',
          tableRefs: [{ rid: 't1', ref }],
        }),
        makeTable({ uid: 'table-2', dirId: 'dir-1', stem: 'other-table', name: 'Other' }),
      ],
      selUid: 'table-1',
      view: 'table',
    });
  }

  it('shows the explanatory line and a row per sub-table ref', () => {
    setUpCompound('other-table');
    renderEditor(makeEngine());

    expect(
      screen.getByText('Each sub-table is rolled when this compound table is rolled; results are combined.'),
    ).toBeTruthy();
    expect((screen.getByPlaceholderText('table reference') as HTMLInputElement).value).toBe('other-table');
  });

  it('marks an unresolved sub-table ref invalid', () => {
    setUpCompound('missing');
    renderEditor(makeEngine());

    expect(screen.getByPlaceholderText('table reference').className).toContain('field-input--invalid');
  });

  it('does not mark an empty sub-table ref invalid', () => {
    setUpCompound('');
    renderEditor(makeEngine());

    expect(screen.getByPlaceholderText('table reference').className).not.toContain('field-input--invalid');
  });

  it('"+ table" adds a sub-table ref row', async () => {
    const user = userEvent.setup();
    setUpCompound('other-table');
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: '+ table' }));

    expect(useForgeStore.getState().tables[0].tableRefs).toHaveLength(2);
  });

  it('✕ removes a sub-table ref row', async () => {
    const user = userEvent.setup();
    setUpCompound('other-table');
    renderEditor(makeEngine());

    await user.click(screen.getByRole('button', { name: 'Remove table reference' }));

    expect(useForgeStore.getState().tables[0].tableRefs).toHaveLength(0);
  });
});

describe('AppContent center-pane wiring', () => {
  it('renders TableEditor for the selected table when view is "table"', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ name: 'Wandering NPC' })],
      selUid: 'table-1',
      view: 'table',
    });

    render(
      <EngineProvider engine={makeEngine()} debounceMs={0}>
        <AppContent />
      </EngineProvider>,
    );

    expect((screen.getByLabelText(/^Display name$/) as HTMLInputElement).value).toBe('Wandering NPC');
  });
});
