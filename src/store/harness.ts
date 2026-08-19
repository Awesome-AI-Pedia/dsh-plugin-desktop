import { proxy } from 'valtio'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type HarnessStatus =
  | 'idle'
  | 'installing'   // 首次下载 Node/dsh
  | 'starting'    // 拉起 dsh 子进程 / 等待就绪
  | 'ready'        // dsh web 已就绪
  | 'error'

export interface HarnessState {
  status: HarnessStatus
  serviceUrl: string | null
  port: number | null
  message: string
  errorDetail: string | null
  refreshKey: number
  // 下载进度（M3 使用）
  downloadStage: string | null
  downloadPercent: number
  isOwnedByThisApp: boolean
}

export const harnessStore = proxy<HarnessState>({
  status: 'idle',
  serviceUrl: null,
  port: null,
  message: '',
  errorDetail: null,
  refreshKey: 0,
  downloadStage: null,
  downloadPercent: 0,
  isOwnedByThisApp: false,
})

interface DshStatus {
  running: boolean
  port: number | null
  url: string | null
  owned_by_this_app: boolean
}

async function refreshStatus() {
  try {
    const s = await invoke<DshStatus>('get_dsh_status')
    if (s.running && s.url && s.port) {
      harnessStore.status = 'ready'
      harnessStore.serviceUrl = s.url
      harnessStore.port = s.port
      harnessStore.isOwnedByThisApp = s.owned_by_this_app
      harnessStore.message = s.owned_by_this_app ? 'DSH 服务已就绪' : '检测到外部 DSH 服务，直接复用'
    }
  } catch (e) {
    console.warn('get_dsh_status failed', e)
  }
}

let startupInFlight: Promise<void> | null = null

export async function startup() {
  // 防抖：同一时刻只允许一个 startup 在跑（避免 StrictMode / 双击重试导致并发调用）
  if (startupInFlight) return startupInFlight
  startupInFlight = (async () => {
    harnessStore.status = 'starting'
    harnessStore.message = '正在启动 DSH 服务…'
    harnessStore.errorDetail = null
    try {
      const result = await invoke<DshStatus>('launch_harness')
      harnessStore.status = 'ready'
      harnessStore.serviceUrl = result.url
      harnessStore.port = result.port
      harnessStore.isOwnedByThisApp = result.owned_by_this_app
      harnessStore.message = result.owned_by_this_app ? 'DSH 服务已就绪' : '检测到外部 DSH 服务，直接复用'
    } catch (e: any) {
      // 只有当前状态不是 ready 时才覆盖为 error，避免竞态覆盖已就绪状态
      if (harnessStore.status !== 'ready') {
        harnessStore.status = 'error'
        harnessStore.errorDetail = String(e?.message || e)
        harnessStore.message = '启动失败'
      }
    } finally {
      startupInFlight = null
    }
  })()
  return startupInFlight
}

export async function restartHarness() {
  harnessStore.status = 'starting'
  harnessStore.message = '正在重启 DSH 服务…'
  try {
    const result = await invoke<DshStatus>('restart_harness')
    harnessStore.serviceUrl = result.url
    harnessStore.port = result.port
    harnessStore.isOwnedByThisApp = result.owned_by_this_app
    harnessStore.status = 'ready'
    harnessStore.refreshKey += 1
    harnessStore.message = 'DSH 服务已重启'
  } catch (e: any) {
    harnessStore.status = 'error'
    harnessStore.errorDetail = String(e?.message || e)
  }
}

export function refreshWebview() {
  harnessStore.refreshKey += 1
}

// 监听 Rust 侧派发的状态/进度事件
listen<DshStatus>('dsh://status', (evt) => {
  const s = evt.payload
  if (s.running && s.url && s.port) {
    harnessStore.serviceUrl = s.url
    harnessStore.port = s.port
    harnessStore.isOwnedByThisApp = s.owned_by_this_app
    if (harnessStore.status !== 'ready') {
      harnessStore.status = 'ready'
      harnessStore.message = 'DSH 服务已就绪'
    }
  } else if (harnessStore.status === 'ready') {
    harnessStore.status = 'starting'
    harnessStore.message = '与 DSH 服务失去连接，正在重连…'
  }
})

listen<{ stage: string; percent: number }>('dsh://download', (evt) => {
  harnessStore.status = 'installing'
  harnessStore.downloadStage = evt.payload.stage
  harnessStore.downloadPercent = evt.payload.percent
  harnessStore.message = `${evt.payload.stage} · ${evt.payload.percent}%`
})

// 兜底刷新一次
refreshStatus()
