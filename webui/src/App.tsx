import { EngineProvider, useDerived } from './engine/useEngine';
import { useForgeStore } from './model/store';
import { EmptyState } from './components/EmptyState';
import { HeaderBar } from './components/HeaderBar';
import { ManifestEditor } from './components/ManifestEditor';
import { RightPane } from './components/RightPane';
import { Scriptorium } from './components/Scriptorium';
import { TableEditor } from './components/TableEditor';

export function AppContent() {
  const collectionName = useForgeStore((s) => s.manifest.name);
  const view = useForgeStore((s) => s.view);
  const { errors, currentYaml, currentTitle } = useDerived();

  return (
    <div className="app">
      <HeaderBar collectionName={collectionName} errorCount={errors.length} />
      <div className="app-body">
        <Scriptorium />
        <div className="pane-center">
          {view === 'empty' && <EmptyState />}
          {view === 'manifest' && <ManifestEditor />}
          {view === 'table' && <TableEditor />}
        </div>
        <div className="pane-right">
          <RightPane title={currentTitle} yaml={currentYaml} errors={errors} />
        </div>
      </div>
    </div>
  )
}

function App() {
  return (
    <EngineProvider>
      <AppContent />
    </EngineProvider>
  )
}

export default App
