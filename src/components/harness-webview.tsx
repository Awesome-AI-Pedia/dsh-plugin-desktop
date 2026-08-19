import { useMemo } from 'react'
import { restartHarness, refreshWebview } from '../store/harness'

interface Props {
  url: string
  refreshKey: number
}

export function HarnessWebview({ url, refreshKey }: Props) {
  // refreshKey 变化会强制重挂 iframe
  const iframeSrc = useMemo(() => url, [url])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <Navbar />
      <iframe
        key={refreshKey}
        src={iframeSrc}
        title="DeepSeek Harness"
        style={{ flex: 1, width: '100%', border: 'none', background: '#0b0d10' }}
        sandbox="allow-same-origin allow-scripts allow-popups allow-forms allow-modals allow-downloads allow-storage-access-by-user-activation allow-popups-to-escape-sandbox"
        allow="clipboard-read; clipboard-write; camera; microphone; fullscreen"
      />
    </div>
  )
}

function Navbar() {
  return (
    <div
      data-tauri-drag-region
      style={{
        height: 36,
        background: '#111418',
        borderBottom: '1px solid #1f2937',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-end',
        padding: '0 12px',
        gap: 8,
        userSelect: 'none',
      }}
    >
      <button style={btnStyle} onClick={refreshWebview} title="刷新页面">刷新</button>
      <button style={btnStyle} onClick={restartHarness} title="重启 DSH 服务">重启服务</button>
    </div>
  )
}

const btnStyle: React.CSSProperties = {
  background: 'transparent',
  border: '1px solid #374151',
  color: '#e5e7eb',
  padding: '4px 10px',
  borderRadius: 4,
  cursor: 'pointer',
  fontSize: 12,
}
