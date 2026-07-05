import { EngineProvider, useDerived } from './engine/useEngine';
import { useForgeStore } from './model/store';
import { HeaderBar } from './components/HeaderBar';
import { ManifestEditor } from './components/ManifestEditor';
import { Scriptorium } from './components/Scriptorium';

export function AppContent() {
  const collectionName = useForgeStore((s) => s.manifest.name);
  const view = useForgeStore((s) => s.view);
  const { errors } = useDerived();

  return (
    <div className="app">
      <HeaderBar collectionName={collectionName} errorCount={errors.length} />
      <div className="app-body">
        <Scriptorium />
        <div className="pane-center">{view === 'manifest' && <ManifestEditor />}</div>
        <div className="pane-right" />
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
