// ABOUTME: React wiring for the engine bridge: an EngineProvider that lazily
// ABOUTME: loads WASM (or accepts an injected engine for tests), plus useDerived,
// ABOUTME: the debounced view of the current collection's YAML/validation state.

import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import type { Dir, ManifestState, TableDraft, View } from '../model/types';
import { useForgeStore } from '../model/store';
import { collectionFiles, manifestYaml as buildManifestYaml, tableYaml, type FileInput } from '../yaml/emit';
import { initEngine, type Engine } from './engine';

interface EngineContextValue {
  engine: Engine;
  debounceMs: number;
}

const EngineContext = createContext<EngineContextValue | null>(null);

export function useEngine(): EngineContextValue {
  const ctx = useContext(EngineContext);
  if (!ctx) throw new Error('useEngine must be called within an EngineProvider');
  return ctx;
}

type LoadState =
  | { status: 'ready'; engine: Engine }
  | { status: 'loading' }
  | { status: 'error'; message: string };

export function EngineProvider({
  engine,
  debounceMs = 150,
  children,
}: {
  engine?: Engine;
  debounceMs?: number;
  children: ReactNode;
}) {
  const [state, setState] = useState<LoadState>(() =>
    engine ? { status: 'ready', engine } : { status: 'loading' },
  );

  useEffect(() => {
    if (engine) return;
    let cancelled = false;
    initEngine()
      .then((loaded) => {
        if (!cancelled) setState({ status: 'ready', engine: loaded });
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setState({ status: 'error', message: err instanceof Error ? err.message : String(err) });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [engine]);

  if (state.status === 'loading') return <div>Loading engine…</div>;
  if (state.status === 'error') return <div>Engine failed to load: {state.message}</div>;

  return <EngineContext.Provider value={{ engine: state.engine, debounceMs }}>{children}</EngineContext.Provider>;
}

export interface Derived {
  files: FileInput[];
  manifestYaml: string;
  errors: string[];
  currentYaml: string;
  currentTitle: string;
}

function computeDerived(
  engine: Engine,
  manifest: ManifestState,
  dirs: Dir[],
  tables: TableDraft[],
  view: View,
  selUid: string | null,
): Derived {
  const files = collectionFiles(dirs, tables);
  const manifestYamlText = buildManifestYaml(manifest, dirs);
  const errors = engine.validate(manifestYamlText, files);

  if (view === 'table') {
    const selected = tables.find((t) => t.uid === selUid);
    if (selected) {
      return {
        files,
        manifestYaml: manifestYamlText,
        errors,
        currentYaml: tableYaml(selected),
        currentTitle: `${selected.stem.toUpperCase()}.YAML`,
      };
    }
  }

  return {
    files,
    manifestYaml: manifestYamlText,
    errors,
    currentYaml: manifestYamlText,
    currentTitle: 'MANIFEST.YAML',
  };
}

export function useDerived(): Derived {
  const { engine, debounceMs } = useEngine();
  const manifest = useForgeStore((s) => s.manifest);
  const dirs = useForgeStore((s) => s.dirs);
  const tables = useForgeStore((s) => s.tables);
  const view = useForgeStore((s) => s.view);
  const selUid = useForgeStore((s) => s.selUid);

  const [derived, setDerived] = useState<Derived>(() =>
    computeDerived(engine, manifest, dirs, tables, view, selUid),
  );

  useEffect(() => {
    const timer = setTimeout(() => {
      setDerived(computeDerived(engine, manifest, dirs, tables, view, selUid));
    }, debounceMs);
    return () => clearTimeout(timer);
  }, [engine, debounceMs, manifest, dirs, tables, view, selUid]);

  return derived;
}
