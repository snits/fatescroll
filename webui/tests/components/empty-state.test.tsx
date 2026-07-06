// ABOUTME: Tests for EmptyState: renders the placeholder glyph/title/helper text,
// ABOUTME: and App center-pane wiring shows it only for the "empty" view.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { AppContent } from '../../src/App';
import { EmptyState } from '../../src/components/EmptyState';
import { EngineProvider } from '../../src/engine/useEngine';
import type { Engine } from '../../src/engine/engine';
import { initialState, useForgeStore } from '../../src/model/store';
import type { Dir, TableDraft } from '../../src/model/types';

afterEach(cleanup);

function makeFakeEngine(): Engine {
  return {
    validate: () => [],
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

describe('EmptyState', () => {
  it('renders the glyph, title, and helper text', () => {
    render(<EmptyState />);

    expect(screen.getByText('✦')).toBeTruthy();
    expect(screen.getByText('Nothing selected')).toBeTruthy();
    expect(
      screen.getByText('Pick a table from the scriptorium, or add one to a directory to begin inscribing.'),
    ).toBeTruthy();
  });
});

describe('AppContent center-pane wiring', () => {
  it('shows the empty state when view is "empty"', () => {
    useForgeStore.setState({ view: 'empty' });
    const Wrapper = wrapper(makeFakeEngine());

    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );

    expect(screen.getByText('Nothing selected')).toBeTruthy();
  });

  it('does not show the empty state when view is "manifest"', () => {
    useForgeStore.setState({ view: 'manifest' });
    const Wrapper = wrapper(makeFakeEngine());

    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );

    expect(screen.queryByText('Nothing selected')).toBeNull();
  });

  it('does not show the empty state when view is "table"', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      selUid: 'table-1',
      view: 'table',
    });
    const Wrapper = wrapper(makeFakeEngine());

    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );

    expect(screen.queryByText('Nothing selected')).toBeNull();
  });
});
