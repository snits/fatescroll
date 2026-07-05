import { EngineProvider, useDerived } from './engine/useEngine';
import { useForgeStore } from './model/store';
import { HeaderBar } from './components/HeaderBar';
import { Scriptorium } from './components/Scriptorium';

export function AppContent() {
  const manifest = useForgeStore((s) => s.manifest);
  const { errors } = useDerived();

  return (
    <div className="app">
      <HeaderBar manifest={manifest} errorCount={errors.length} />
      <div className="app-body">
        <Scriptorium />
        <div className="pane-center" />
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
