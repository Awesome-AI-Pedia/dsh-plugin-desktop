import { useEffect } from 'react'
import { useSnapshot } from 'valtio'
import { harnessStore, startup } from './store/harness'
import { HarnessWebview } from './components/harness-webview'
import { StatusOverlay } from './components/status-overlay'

export function App() {
  const state = useSnapshot(harnessStore)

  useEffect(() => {
    startup()
  }, [])

  return (
    <div className="app-root" style={{ position: 'relative', height: '100vh', width: '100vw', overflow: 'hidden' }}>
      {state.status === 'ready' && state.serviceUrl
        ? <HarnessWebview url={state.serviceUrl} refreshKey={state.refreshKey} />
        : <StatusOverlay />}
    </div>
  )
}
