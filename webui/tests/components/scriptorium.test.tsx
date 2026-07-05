// ABOUTME: Tests for the Scriptorium tree: manifest node, directory headers,
// ABOUTME: table rows, selection wiring, and the add-directory/add-table actions.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Scriptorium } from '../../src/components/Scriptorium';
import { initialState, useForgeStore } from '../../src/model/store';
import type { Dir, TableDraft } from '../../src/model/types';

// @testing-library/react's auto-cleanup only registers when `afterEach` is a
// global (vitest config here does not set `globals: true`), so unmount
// rendered trees explicitly to avoid stale components staying subscribed to
// useForgeStore across tests.
afterEach(cleanup);

function makeDir(overrides: Partial<Dir> = {}): Dir {
  return { id: 'dir-1', path: 'npcs', namespace: 'fatescroll.npcs', ...overrides };
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

beforeEach(() => {
  useForgeStore.setState(initialState());
});

describe('Scriptorium', () => {
  it('renders the manifest node, a directory header, and its table rows', () => {
    useForgeStore.setState({ dirs: [makeDir()], tables: [makeTable()] });

    render(<Scriptorium />);

    expect(screen.getByText('manifest.yaml')).toBeTruthy();
    expect(screen.getByText('npcs/')).toBeTruthy();
    expect(screen.getByText('fatescroll.npcs')).toBeTruthy();
    expect(screen.getByText('smp')).toBeTruthy();
    expect(screen.getByText('Wandering NPC')).toBeTruthy();
  });

  it('shows the cmp badge for compound tables', () => {
    useForgeStore.setState({ dirs: [makeDir()], tables: [makeTable({ type: 'compound' })] });

    render(<Scriptorium />);

    expect(screen.getByText('cmp')).toBeTruthy();
  });

  it('clicking a table row selects it', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ dirs: [makeDir()], tables: [makeTable()] });

    render(<Scriptorium />);
    const row = screen.getByRole('button', { name: /Wandering NPC/ });
    await user.click(row);

    expect(useForgeStore.getState().view).toBe('table');
    expect(useForgeStore.getState().selUid).toBe('table-1');
    expect(row.className).toContain('tree-table--selected');
  });

  it('clicking a directory header opens the manifest view', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ dirs: [makeDir()], view: 'table', selUid: 'x' });

    render(<Scriptorium />);
    await user.click(screen.getByRole('button', { name: /npcs\// }));

    expect(useForgeStore.getState().view).toBe('manifest');
    expect(useForgeStore.getState().selUid).toBeNull();
  });

  it('clicking the manifest node opens the manifest view', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ view: 'table', selUid: 'x' });

    render(<Scriptorium />);
    await user.click(screen.getByRole('button', { name: /manifest\.yaml/ }));

    expect(useForgeStore.getState().view).toBe('manifest');
    expect(screen.getByRole('button', { name: /manifest\.yaml/ }).className).toContain(
      'tree-manifest--selected',
    );
  });

  it('the directory + button adds a table to that directory and selects it', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ dirs: [makeDir()], tables: [] });

    render(<Scriptorium />);
    await user.click(screen.getByRole('button', { name: /add table/i }));

    const state = useForgeStore.getState();
    expect(state.tables).toHaveLength(1);
    expect(state.tables[0].dirId).toBe('dir-1');
    expect(state.view).toBe('table');
    expect(state.selUid).toBe(state.tables[0].uid);
  });

  it('"+ add directory" adds a directory and opens the manifest view', async () => {
    const user = userEvent.setup();
    render(<Scriptorium />);

    await user.click(screen.getByRole('button', { name: '+ add directory' }));

    const state = useForgeStore.getState();
    expect(state.dirs).toHaveLength(1);
    expect(state.view).toBe('manifest');
  });
});
