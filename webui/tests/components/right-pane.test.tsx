// ABOUTME: Tests for the right pane: YamlViewer (title/pre/copy/download),
// ABOUTME: ValidationPanel (valid/error list), and DiceRoller (roll button,
// ABOUTME: fresh-state binding, output rendering, roll-clears-on-edit).

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { DiceRoller } from '../../src/components/DiceRoller';
import { RightPane } from '../../src/components/RightPane';
import { ValidationPanel } from '../../src/components/ValidationPanel';
import { COPIED_LABEL_MS, downloadTarget, YamlViewer } from '../../src/components/YamlViewer';
import { EngineProvider } from '../../src/engine/useEngine';
import type { Engine, RollNode } from '../../src/engine/engine';
import type { FileInput } from '../../src/yaml/emit';
import { initialState, useForgeStore } from '../../src/model/store';
import type { Dir, TableDraft } from '../../src/model/types';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

beforeEach(() => {
  useForgeStore.setState(initialState());
});

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
    ...overrides,
  };
}

function wrapper(engine: Engine, debounceMs = 0) {
  return ({ children }: { children: ReactNode }) => (
    <EngineProvider engine={engine} debounceMs={debounceMs}>
      {children}
    </EngineProvider>
  );
}

function mockClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    value: { writeText },
    configurable: true,
  });
  return writeText;
}

describe('YamlViewer', () => {
  it('renders the given title', () => {
    render(<YamlViewer title="MANIFEST.YAML" yaml={'name: x\n'} />);
    expect(screen.getByText('MANIFEST.YAML')).toBeTruthy();
  });

  it('renders a table view title', () => {
    render(<YamlViewer title="NEW-TABLE.YAML" yaml={'id: new-table\n'} />);
    expect(screen.getByText('NEW-TABLE.YAML')).toBeTruthy();
  });

  it('renders the current yaml text in a <pre>', () => {
    const yaml = 'name: Foo\nversion: "1.0"\n';
    const { container } = render(<YamlViewer title="MANIFEST.YAML" yaml={yaml} />);
    const pre = container.querySelector('pre.yaml-viewer-pre');
    expect(pre).toBeTruthy();
    expect(pre?.textContent).toBe(yaml);
  });

  it('copies the yaml to the clipboard and animates the button label', async () => {
    // userEvent.setup() installs its own clipboard stub unconditionally, which
    // would shadow this mock — use fireEvent instead so the app's real
    // navigator.clipboard.writeText call is observable.
    vi.useFakeTimers();
    try {
      const writeText = mockClipboard();
      render(<YamlViewer title="MANIFEST.YAML" yaml={'name: Foo\n'} />);

      fireEvent.click(screen.getByRole('button', { name: '⧉ copy' }));
      await act(async () => {}); // flush the resolved writeText promise

      expect(writeText).toHaveBeenCalledWith('name: Foo\n');
      expect(screen.getByRole('button', { name: '✓ copied' })).toBeTruthy();

      act(() => {
        vi.advanceTimersByTime(COPIED_LABEL_MS);
      });
      expect(screen.getByRole('button', { name: '⧉ copy' })).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });

  it('keeps the copy label unchanged when the clipboard write is rejected', async () => {
    const writeText = vi.fn().mockRejectedValue(new DOMException('Document is not focused.'));
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    render(<YamlViewer title="MANIFEST.YAML" yaml={'name: Foo\n'} />);

    fireEvent.click(screen.getByRole('button', { name: '⧉ copy' }));
    await act(async () => {}); // flush the rejected writeText promise

    expect(writeText).toHaveBeenCalledWith('name: Foo\n');
    // No unhandled rejection (vitest would fail the run) and no false success.
    expect(screen.getByRole('button', { name: '⧉ copy' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: '✓ copied' })).toBeNull();
  });

  it('downloads the manifest yaml/filename on the manifest view', () => {
    useForgeStore.setState({
      manifest: { ...initialState().manifest, name: 'My Collection' },
      dirs: [],
      tables: [],
      view: 'manifest',
      selUid: null,
    });
    const state = useForgeStore.getState();
    const target = downloadTarget(state.view, state.selUid, state);
    expect(target.filename).toBe('manifest.yaml');
    expect(target.content).toContain('name: My Collection');
  });

  it('downloads the selected table yaml/filename on the table view', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ stem: 'wandering-npc' })],
      view: 'table',
      selUid: 'table-1',
    });
    const state = useForgeStore.getState();
    const target = downloadTarget(state.view, state.selUid, state);
    expect(target.filename).toBe('wandering-npc.yaml');
    expect(target.content).toContain('id: wandering-npc');
  });

  it('download button creates an object URL and triggers an anchor click with the target filename', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable({ stem: 'wandering-npc' })],
      view: 'table',
      selUid: 'table-1',
    });
    const createObjectURL = vi.fn(() => 'blob:mock');
    const revokeObjectURL = vi.fn();
    vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });
    let downloadedName = '';
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      downloadedName = this.download;
    });

    render(<YamlViewer title="WANDERING-NPC.YAML" yaml={'id: wandering-npc\n'} />);
    fireEvent.click(screen.getByRole('button', { name: '⬇ .yaml' }));

    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect(revokeObjectURL).toHaveBeenCalledTimes(1);
    expect(downloadedName).toBe('wandering-npc.yaml');
  });
});

describe('ValidationPanel', () => {
  it('shows a valid message with no errors', () => {
    render(<ValidationPanel errors={[]} />);
    expect(screen.getByText('✓ Collection is valid.')).toBeTruthy();
  });

  it('lists each error prefixed with ✕', () => {
    render(<ValidationPanel errors={['bad namespace', 'bad dice']} />);
    expect(screen.getByText('✕ bad namespace')).toBeTruthy();
    expect(screen.getByText('✕ bad dice')).toBeTruthy();
  });
});

describe('DiceRoller', () => {
  it('disables the Roll button unless a table is selected', () => {
    render(<DiceRoller />, { wrapper: wrapper(makeEngine()) });
    expect((screen.getByRole('button', { name: /Roll/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('enables the Roll button on the table view', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      view: 'table',
      selUid: 'table-1',
    });
    render(<DiceRoller />, { wrapper: wrapper(makeEngine()) });
    expect((screen.getByRole('button', { name: /Roll/ }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('shows the placeholder when there is no roll preview', () => {
    render(<DiceRoller />, { wrapper: wrapper(makeEngine()) });
    expect(screen.getByText('— roll a table to preview an outcome —')).toBeTruthy();
  });

  it('rolls, flattens the result tree, and renders indented lines with depth classes', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      view: 'table',
      selUid: 'table-1',
    });
    const tree: RollNode = {
      table_name: 'wandering-npc',
      roll: null,
      text: null,
      children: [{ table_name: 'name', roll: 3, text: 'Bram', children: [] }],
    };
    const engine = makeEngine({ roll: () => tree });
    const { container } = render(<DiceRoller />, { wrapper: wrapper(engine) });

    fireEvent.click(screen.getByRole('button', { name: /Roll/ }));

    const lines = container.querySelectorAll('.dice-roller-line');
    expect(lines).toHaveLength(2);
    expect(lines[0].textContent).toBe('wandering-npc');
    expect(lines[0].className).toContain('dice-roller-line--root');
    expect((lines[0] as HTMLElement).style.paddingLeft).toBe('0px');
    expect(lines[1].textContent).toBe('name (rolled 3): Bram');
    expect(lines[1].className).toContain('dice-roller-line--nested');
    expect((lines[1] as HTMLElement).style.paddingLeft).toBe('18px');
  });

  it('renders a single error-styled line when the engine returns {error}', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      view: 'table',
      selUid: 'table-1',
    });
    const engine = makeEngine({ roll: () => ({ error: 'cyclic chain detected' }) });
    const { container } = render(<DiceRoller />, { wrapper: wrapper(engine) });

    fireEvent.click(screen.getByRole('button', { name: /Roll/ }));

    const lines = container.querySelectorAll('.dice-roller-line');
    expect(lines).toHaveLength(1);
    expect(lines[0].textContent).toBe('cyclic chain detected');
    expect(lines[0].className).toContain('dice-roller-line--error');
  });

  it('renders an error line when engine.roll throws instead of crashing the handler', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      view: 'table',
      selUid: 'table-1',
    });
    const engine = makeEngine({
      roll: () => {
        throw new Error('wasm panicked');
      },
    });
    const { container } = render(<DiceRoller />, { wrapper: wrapper(engine) });

    fireEvent.click(screen.getByRole('button', { name: /Roll/ }));

    const lines = container.querySelectorAll('.dice-roller-line');
    expect(lines).toHaveLength(1);
    expect(lines[0].textContent).toContain('engine failure');
    expect(lines[0].textContent).toContain('wasm panicked');
    expect(lines[0].className).toContain('dice-roller-line--error');
  });

  it('rolls using manifest/files/fqid computed from current store state, not stale props', () => {
    let seenFiles: FileInput[] = [];
    let seenFqid = '';
    let seenManifestYaml = '';
    const engine = makeEngine({
      roll: (manifestYamlArg, files, fqid) => {
        seenManifestYaml = manifestYamlArg;
        seenFiles = files;
        seenFqid = fqid;
        return { table_name: fqid, roll: null, text: null, children: [] };
      },
    });

    // Stale props: RightPane is handed yaml/title that do not reflect the
    // mutations below, standing in for a debounced `derived` that hasn't
    // caught up yet. DiceRoller must ignore them entirely.
    render(<RightPane title="MANIFEST.YAML" yaml={'stale: true\n'} errors={[]} />, {
      wrapper: wrapper(engine, 99999),
    });

    act(() => {
      useForgeStore.getState().addDir();
      const dirId = useForgeStore.getState().dirs[0].id;
      useForgeStore.getState().updateDir(dirId, { namespace: 'fresh-ns', path: 'fresh' });
      useForgeStore.getState().addTable(dirId);
      const tableUid = useForgeStore.getState().tables[0].uid;
      useForgeStore.getState().updateTable(tableUid, { stem: 'fresh-table' });
    });

    fireEvent.click(screen.getByRole('button', { name: /Roll/ }));

    expect(seenFqid).toBe('fresh-ns.fresh-table');
    expect(seenFiles).toHaveLength(1);
    expect(seenFiles[0].stem).toBe('fresh-table');
    expect(seenManifestYaml).toContain('fresh-ns');
  });

  it('clears the roll output back to the placeholder when the table is edited', () => {
    useForgeStore.setState({
      dirs: [makeDir()],
      tables: [makeTable()],
      view: 'table',
      selUid: 'table-1',
    });
    const engine = makeEngine({
      roll: () => ({ table_name: 'wandering-npc', roll: 4, text: 'hi', children: [] }),
    });
    render(<DiceRoller />, { wrapper: wrapper(engine) });

    fireEvent.click(screen.getByRole('button', { name: /Roll/ }));
    expect(screen.getByText('wandering-npc (rolled 4): hi')).toBeTruthy();

    act(() => {
      useForgeStore.getState().updateTable('table-1', { name: 'Renamed NPC' });
    });

    expect(screen.getByText('— roll a table to preview an outcome —')).toBeTruthy();
    expect(screen.queryByText('wandering-npc (rolled 4): hi')).toBeNull();
  });
});
