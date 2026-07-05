// ABOUTME: Tests for the manifest editor: the five bound manifest fields, the
// ABOUTME: directory rows (add/edit/delete with confirmation), and App center-pane wiring.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { ManifestEditor } from '../../src/components/ManifestEditor';
import { AppContent } from '../../src/App';
import { EngineProvider } from '../../src/engine/useEngine';
import type { Engine } from '../../src/engine/engine';
import { initialState, useForgeStore } from '../../src/model/store';
import type { Dir, TableDraft } from '../../src/model/types';

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

describe('ManifestEditor fields', () => {
  it('renders the current values of all five manifest fields', () => {
    useForgeStore.setState({
      manifest: {
        name: 'Border Marches',
        version: '2.3',
        namespace: 'marches',
        author: 'Jerry',
        minToolVersion: '0.4.0',
      },
    });

    render(<ManifestEditor />);

    expect((screen.getByLabelText(/^Name$/) as HTMLInputElement).value).toBe('Border Marches');
    expect((screen.getByLabelText(/^Version$/) as HTMLInputElement).value).toBe('2.3');
    expect((screen.getByLabelText(/^Root namespace$/) as HTMLInputElement).value).toBe('marches');
    expect((screen.getByLabelText(/^Author/) as HTMLInputElement).value).toBe('Jerry');
    expect((screen.getByLabelText(/^Min tool version/) as HTMLInputElement).value).toBe('0.4.0');
  });

  it('typing in Name updates the store', async () => {
    const user = userEvent.setup();
    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^Name$/);
    await user.clear(input);
    await user.type(input, 'Border Marches');
    expect(useForgeStore.getState().manifest.name).toBe('Border Marches');
  });

  it('typing in Version updates the store', async () => {
    const user = userEvent.setup();
    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^Version$/);
    await user.clear(input);
    await user.type(input, '3.0');
    expect(useForgeStore.getState().manifest.version).toBe('3.0');
  });

  it('typing in Root namespace updates the store', async () => {
    const user = userEvent.setup();
    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^Root namespace$/);
    await user.clear(input);
    await user.type(input, 'marches');
    expect(useForgeStore.getState().manifest.namespace).toBe('marches');
  });

  it('typing in Author updates the store', async () => {
    const user = userEvent.setup();
    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^Author/);
    await user.type(input, 'Jerry');
    expect(useForgeStore.getState().manifest.author).toBe('Jerry');
  });

  it('typing in Min tool version updates the store', async () => {
    const user = userEvent.setup();
    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^Min tool version/);
    await user.type(input, '0.4.0');
    expect(useForgeStore.getState().manifest.minToolVersion).toBe('0.4.0');
  });

  it('shows ~ placeholders on the optional Author and Min tool version fields', () => {
    render(<ManifestEditor />);
    expect((screen.getByLabelText(/^Author/) as HTMLInputElement).placeholder).toBe('~');
    expect((screen.getByLabelText(/^Min tool version/) as HTMLInputElement).placeholder).toBe('~');
  });

  it('marks the root namespace input invalid when it fails the namespace pattern', () => {
    useForgeStore.setState({ manifest: { ...initialState().manifest, namespace: 'Bad NS!' } });
    render(<ManifestEditor />);
    expect(screen.getByLabelText(/^Root namespace$/).className).toContain('field-input--invalid');
  });

  it('does not mark the root namespace input invalid when it is valid', () => {
    useForgeStore.setState({ manifest: { ...initialState().manifest, namespace: 'fatescroll.core' } });
    render(<ManifestEditor />);
    expect(screen.getByLabelText(/^Root namespace$/).className).not.toContain('field-input--invalid');
  });
});

describe('ManifestEditor directories', () => {
  it('renders one row per directory with path and namespace bound', () => {
    useForgeStore.setState({
      dirs: [
        makeDir({ id: 'd1', path: 'npcs', namespace: 'fatescroll.npcs' }),
        makeDir({ id: 'd2', path: 'items', namespace: 'fatescroll.items' }),
      ],
    });

    render(<ManifestEditor />);

    const paths = screen.getAllByLabelText(/^path$/) as HTMLInputElement[];
    const namespaces = screen.getAllByLabelText(/^namespace$/) as HTMLInputElement[];
    expect(paths.map((i) => i.value)).toEqual(['npcs', 'items']);
    expect(namespaces.map((i) => i.value)).toEqual(['fatescroll.npcs', 'fatescroll.items']);
  });

  it('typing in a directory path input updates that directory in the store', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ dirs: [makeDir({ id: 'd1', path: 'npcs' })] });

    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^path$/);
    await user.clear(input);
    await user.type(input, 'monsters');

    expect(useForgeStore.getState().dirs[0].path).toBe('monsters');
  });

  it('typing in a directory namespace input updates that directory in the store', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ dirs: [makeDir({ id: 'd1', namespace: 'fatescroll.npcs' })] });

    render(<ManifestEditor />);
    const input = screen.getByLabelText(/^namespace$/);
    await user.clear(input);
    await user.type(input, 'fatescroll.monsters');

    expect(useForgeStore.getState().dirs[0].namespace).toBe('fatescroll.monsters');
  });

  it('marks a directory namespace input invalid when it fails the namespace pattern', () => {
    useForgeStore.setState({ dirs: [makeDir({ namespace: 'Bad!' })] });
    render(<ManifestEditor />);
    expect(screen.getByLabelText(/^namespace$/).className).toContain('field-input--invalid');
  });

  it('does not mark a directory namespace input invalid when it is valid', () => {
    useForgeStore.setState({ dirs: [makeDir({ namespace: 'fatescroll.npcs' })] });
    render(<ManifestEditor />);
    expect(screen.getByLabelText(/^namespace$/).className).not.toContain('field-input--invalid');
  });

  it('"+ add" adds a directory', async () => {
    const user = userEvent.setup();
    render(<ManifestEditor />);

    await user.click(screen.getByRole('button', { name: '+ add' }));

    expect(useForgeStore.getState().dirs).toHaveLength(1);
  });

  it('deletes a directory immediately when it has no tables', async () => {
    const user = userEvent.setup();
    useForgeStore.setState({ dirs: [makeDir({ id: 'd1' })], tables: [] });

    render(<ManifestEditor />);
    await user.click(screen.getByRole('button', { name: /remove directory/i }));

    expect(useForgeStore.getState().dirs).toHaveLength(0);
  });

  it('confirms before deleting a directory with tables, naming the path and table count', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    useForgeStore.setState({
      dirs: [makeDir({ id: 'd1', path: 'core' })],
      tables: [
        makeTable({ uid: 't1', dirId: 'd1' }),
        makeTable({ uid: 't2', dirId: 'd1' }),
        makeTable({ uid: 't3', dirId: 'd1' }),
      ],
    });

    render(<ManifestEditor />);
    await user.click(screen.getByRole('button', { name: /remove directory/i }));

    expect(confirmSpy).toHaveBeenCalledWith('Delete directory "core/" and its 3 table(s)?');
    const state = useForgeStore.getState();
    expect(state.dirs).toHaveLength(0);
    expect(state.tables).toHaveLength(0);
  });

  it('does not delete the directory or its tables when confirm is declined', async () => {
    const user = userEvent.setup();
    vi.spyOn(window, 'confirm').mockReturnValue(false);
    useForgeStore.setState({
      dirs: [makeDir({ id: 'd1', path: 'core' })],
      tables: [makeTable({ uid: 't1', dirId: 'd1' })],
    });

    render(<ManifestEditor />);
    await user.click(screen.getByRole('button', { name: /remove directory/i }));

    const state = useForgeStore.getState();
    expect(state.dirs).toHaveLength(1);
    expect(state.tables).toHaveLength(1);
  });
});

function makeFakeEngine(errors: string[]): Engine {
  return {
    validate: () => errors,
    diceInfo: () => ({ ok: false }),
    expectedValues: () => null,
    histogram: () => null,
    roll: () => ({ error: 'not used' }),
  };
}

function wrapper(engine: Engine) {
  return ({ children }: { children: ReactNode }) => (
    <EngineProvider engine={engine} debounceMs={0}>
      {children}
    </EngineProvider>
  );
}

describe('AppContent center-pane wiring', () => {
  it('shows the manifest editor when view is "manifest"', () => {
    useForgeStore.setState({ view: 'manifest' });
    const Wrapper = wrapper(makeFakeEngine([]));

    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );

    expect(screen.getByText('Collection Manifest')).toBeTruthy();
  });

  it('does not show the manifest editor when view is "empty"', () => {
    useForgeStore.setState({ view: 'empty' });
    const Wrapper = wrapper(makeFakeEngine([]));

    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );

    expect(screen.queryByText('Collection Manifest')).toBeNull();
  });
});
