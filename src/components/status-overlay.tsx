import { useSnapshot } from 'valtio'
import { harnessStore, startup } from '../store/harness'

export function StatusOverlay() {
  const s = useSnapshot(harnessStore)

  return (
    <div style={overlay}>
      <div style={{ maxWidth: 480, textAlign: 'center' }}>
        <h1 style={{ fontSize: 20, marginBottom: 8 }}>DeepSeek Harness Desktop</h1>
        <p style={{ color: '#9ca3af', fontSize: 14 }}>{s.message || '准备中…'}</p>

        {s.status === 'installing' && (
          <div style={{ marginTop: 24 }}>
            <div style={{ fontSize: 12, color: '#9ca3af', marginBottom: 6 }}>
              {s.downloadStage} · {s.downloadPercent}%
            </div>
            <div style={progressTrack}>
              <div style={{ ...progressBar, width: `${s.downloadPercent}%` }} />
            </div>
          </div>
        )}

        {s.status === 'starting' && <Spinner />}

        {s.status === 'error' && (
          <div style={{ marginTop: 20 }}>
            <pre style={errorBox}>{s.errorDetail}</pre>
            <button style={retryBtn} onClick={startup}>重试</button>
          </div>
        )}
      </div>
    </div>
  )
}

function Spinner() {
  return (
    <div style={{ marginTop: 20 }}>
      <div style={{
        width: 32, height: 32, margin: '0 auto',
        border: '3px solid #374151', borderTopColor: '#60a5fa',
        borderRadius: '50%', animation: 'spin 0.8s linear infinite',
      }} />
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  )
}

const overlay: React.CSSProperties = {
  position: 'absolute', inset: 0,
  display: 'flex', alignItems: 'center', justifyContent: 'center',
  background: '#0b0d10', color: '#e5e7eb', padding: 24,
}

const progressTrack: React.CSSProperties = {
  width: '100%', height: 6, background: '#1f2937', borderRadius: 3, overflow: 'hidden',
}
const progressBar: React.CSSProperties = {
  height: '100%', background: '#60a5fa', transition: 'width 200ms',
}
const errorBox: React.CSSProperties = {
  background: '#1f2937', padding: 12, borderRadius: 6, fontSize: 12,
  textAlign: 'left', maxHeight: 200, overflow: 'auto', color: '#fca5a5',
}
const retryBtn: React.CSSProperties = {
  marginTop: 12, background: '#2563eb', border: 'none', color: '#fff',
  padding: '8px 20px', borderRadius: 4, cursor: 'pointer',
}
