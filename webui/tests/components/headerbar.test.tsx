// ABOUTME: Tests for the header bar's presentational rendering and for the
// ABOUTME: App-level wiring that feeds it a validation error count from the engine.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { strToU8, zipSync } from 'fflate';
import type { ReactNode } from 'react';
import { HeaderBar } from '../../src/components/HeaderBar';
import { AppContent } from '../../src/App';
import { EngineProvider } from '../../src/engine/useEngine';
import type { Engine } from '../../src/engine/engine';
import { initialState, useForgeStore } from '../../src/model/store';
import type { Dir, TableDraft } from '../../src/model/types';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function makeDir(overrides: Partial<Dir> = {}): Dir {
  return { id: 'dir-1', path: 'core', namespace: 'ns', ...overrides };
}

function makeTable(overrides: Partial<TableDraft> = {}): TableDraft {
  return {
    uid: 'table-1',
    dirId: 'dir-1',
    stem: 'oracle',
    name: 'Oracle',
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

function makeFakeEngine(errors: string[]): Engine {
  return {
    validate: () => errors,
    diceInfo: () => ({ ok: false }),
    expectedValues: () => null,
    histogram: () => null,
    roll: () => ({ error: 'not used' }),
    parseCollection: () => ({ ok: false, errors: ['not used'] }),
  };
}

function wrapper(engine: Engine) {
  return ({ children }: { children: ReactNode }) => (
    <EngineProvider engine={engine} debounceMs={0}>
      {children}
    </EngineProvider>
  );
}

describe('HeaderBar', () => {
  beforeEach(() => {
    useForgeStore.setState(initialState());
  });

  it('renders the brand block and the collection name', () => {
    render(<HeaderBar collectionName="Border Marches" errorCount={0} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });

    expect(screen.getByText('Fatescroll')).toBeTruthy();
    expect(screen.getByText('TABLE FORGE')).toBeTruthy();
    expect(screen.getByText('Collection')).toBeTruthy();
    expect(screen.getByText('Border Marches')).toBeTruthy();
  });

  it('shows a valid status pill with zero errors', () => {
    render(<HeaderBar collectionName="Border Marches" errorCount={0} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    const text = screen.getByText('Collection is valid');
    expect(text).toBeTruthy();
    expect(text.closest('.header-status')?.className).toContain('header-status--valid');
  });

  it('shows an invalid status pill with a pluralized error count', () => {
    render(<HeaderBar collectionName="Border Marches" errorCount={3} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    const text = screen.getByText('3 errors');
    expect(text).toBeTruthy();
    expect(text.closest('.header-status')?.className).toContain('header-status--invalid');
  });

  it('singularizes a single error', () => {
    render(<HeaderBar collectionName="Border Marches" errorCount={1} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    expect(screen.getByText('1 error')).toBeTruthy();
  });

  it('renders an enabled export button', () => {
    render(<HeaderBar collectionName="Border Marches" errorCount={0} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    const button = screen.getByRole('button', { name: /export collection/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('stays enabled even when the collection has validation errors', () => {
    render(<HeaderBar collectionName="Border Marches" errorCount={3} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    const button = screen.getByRole('button', { name: /export collection/i }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it('clicking export builds a zip Blob named after the collection slug and downloads it', () => {
    useForgeStore.setState({
      manifest: { ...initialState().manifest, name: 'Kal-Arath Collection!' },
      dirs: [makeDir()],
      tables: [makeTable()],
    });

    const createObjectURL = vi.fn((_obj: Blob) => 'blob:mock');
    const revokeObjectURL = vi.fn();
    vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });
    let downloadedName = '';
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      downloadedName = this.download;
    });

    render(<HeaderBar collectionName="stale prop name" errorCount={0} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    fireEvent.click(screen.getByRole('button', { name: /export collection/i }));

    expect(createObjectURL).toHaveBeenCalledTimes(1);
    const blob = createObjectURL.mock.calls[0][0] as Blob;
    expect(blob.type).toBe('application/zip');
    expect(downloadedName).toBe('kal-arath-collection.zip');
    expect(revokeObjectURL).toHaveBeenCalledTimes(1);
  });

  it('alerts instead of downloading when the zip build fails (duplicate table path)', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [
        makeTable({ uid: 't1', stem: 'oracle' }),
        makeTable({ uid: 't2', stem: 'oracle' }),
      ],
    });

    const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const createObjectURL = vi.fn(() => 'blob:mock');
    vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL: vi.fn() });

    render(<HeaderBar collectionName="New Collection" errorCount={1} />, {
      wrapper: wrapper(makeFakeEngine([])),
    });
    fireEvent.click(screen.getByRole('button', { name: /export collection/i }));

    expect(alertSpy).toHaveBeenCalledTimes(1);
    expect(String(alertSpy.mock.calls[0][0])).toContain('core/oracle.yaml');
    expect(createObjectURL).not.toHaveBeenCalled();
  });
});

describe('open collection', () => {
  function zipWith(entries: Record<string, string>): File {
    const bytes = zipSync(
      Object.fromEntries(Object.entries(entries).map(([p, c]) => [p, strToU8(c)])),
    );
    return new File([bytes.slice().buffer], 'collection.zip', { type: 'application/zip' });
  }

  const parsedOutcome = {
    ok: true as const,
    collection: {
      manifest: {
        name: 'Imported',
        version: '1.0',
        namespace: 'imp',
        author: null,
        min_tool_version: null,
        directories: [{ path: 'core', namespace: 'imp.core' }],
      },
      tables: [],
      ignoredYaml: [],
    },
  };

  beforeEach(() => {
    useForgeStore.setState(initialState());
    vi.restoreAllMocks();
  });

  it('opens a zip: ingests entries, parses via engine, loads the store', async () => {
    const parseCollection = vi.fn().mockReturnValue(parsedOutcome);
    const engine = { ...makeFakeEngine([]), parseCollection };
    render(<HeaderBar collectionName="New Collection" errorCount={0} />, {
      wrapper: wrapper(engine),
    });

    fireEvent.click(screen.getByRole('button', { name: /open collection/i }));
    const input = screen.getByTestId('open-zip-input');
    fireEvent.change(input, {
      target: {
        files: [
          zipWith({
            'imp/manifest.yaml': 'name: Imported',
            'imp/core/oracle.yaml': 'id: oracle',
          }),
        ],
      },
    });

    await waitFor(() => expect(parseCollection).toHaveBeenCalled());
    expect(parseCollection).toHaveBeenCalledWith('name: Imported', [
      { path: 'core/oracle.yaml', contents: 'id: oracle' },
    ]);
    await waitFor(() => expect(useForgeStore.getState().manifest.name).toBe('Imported'));
    expect(useForgeStore.getState().view).toBe('manifest');
  });

  it('shows parse errors and leaves state untouched', async () => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const engine = {
      ...makeFakeEngine([]),
      parseCollection: () => ({ ok: false as const, errors: ['core/bad.yaml: mapping error'] }),
    };
    render(<HeaderBar collectionName="New Collection" errorCount={0} />, {
      wrapper: wrapper(engine),
    });

    fireEvent.click(screen.getByRole('button', { name: /open collection/i }));
    fireEvent.change(screen.getByTestId('open-zip-input'), {
      target: { files: [zipWith({ 'manifest.yaml': 'name: X' })] },
    });

    await waitFor(() => expect(alert).toHaveBeenCalled());
    expect(alert.mock.calls[0][0]).toContain('core/bad.yaml: mapping error');
    expect(useForgeStore.getState().manifest.name).toBe('New Collection');
  });

  it('asks for confirmation when tables exist and aborts on cancel', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    useForgeStore.getState().addDir();
    useForgeStore.getState().addTable(useForgeStore.getState().dirs[0].id);
    const parseCollection = vi.fn().mockReturnValue(parsedOutcome);
    const engine = { ...makeFakeEngine([]), parseCollection };
    render(<HeaderBar collectionName="New Collection" errorCount={0} />, {
      wrapper: wrapper(engine),
    });

    fireEvent.click(screen.getByRole('button', { name: /open collection/i }));
    fireEvent.change(screen.getByTestId('open-zip-input'), {
      target: { files: [zipWith({ 'manifest.yaml': 'name: X' })] },
    });

    await waitFor(() => expect(confirm).toHaveBeenCalled());
    expect(parseCollection).not.toHaveBeenCalled();
    expect(useForgeStore.getState().tables).toHaveLength(1);
  });
});

describe('AppContent wiring', () => {
  beforeEach(() => {
    useForgeStore.setState(initialState());
  });

  it('passes zero engine validation errors through to a valid status pill', () => {
    const Wrapper = wrapper(makeFakeEngine([]));
    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );
    expect(screen.getByText('Collection is valid')).toBeTruthy();
  });

  it('passes engine validation errors through to the error-count status pill', () => {
    const Wrapper = wrapper(makeFakeEngine(['bad namespace', 'bad dice']));
    render(
      <Wrapper>
        <AppContent />
      </Wrapper>,
    );
    expect(screen.getByText('2 errors')).toBeTruthy();
  });
});
