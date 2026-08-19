//! DSH 服务生命周期管理。
//!
//! 关键原则：
//! - **区分「本 APP 拉起的 dsh」和「用户外部启动的 dsh」**：靠 `Inner.child` 是否 Some
//!   —— 只有我们自己 spawn 的才有 Child 句柄，退出时**只杀这个**；
//! - 端口策略：先看首选端口 3080 是否有 dsh 存活 → 复用（owned=false）；否则 ephemeral 自拉；
//! - 所有状态变更通过 `dsh://status` 事件通知前端。

use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

pub mod download;
pub mod launcher;
pub mod probe;
pub mod process;

pub use probe::{is_dsh_alive, is_port_in_use};

use crate::config::runtime::PREFERRED_PORT;

#[derive(Debug, Clone, Serialize)]
pub struct DshStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub url: Option<String>,
    pub owned_by_this_app: bool,
}

impl DshStatus {
    fn stopped() -> Self {
        Self { running: false, port: None, url: None, owned_by_this_app: false }
    }
    fn running_at(port: u16, owned: bool) -> Self {
        Self {
            running: true,
            port: Some(port),
            url: Some(format!("http://127.0.0.1:{port}")),
            owned_by_this_app: owned,
        }
    }
}

struct Inner {
    port: Option<u16>,
    owned_by_this_app: bool,
    /// 本 APP 拉起的子进程句柄。None = 未拉起或已复用外部。
    child: Option<process::ChildHandle>,
}

impl Inner {
    fn status(&self) -> DshStatus {
        match self.port {
            Some(p) => DshStatus::running_at(p, self.owned_by_this_app),
            None => DshStatus::stopped(),
        }
    }
}

pub struct HarnessManager {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

impl HarnessManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::new(Mutex::new(Inner {
                port: None,
                owned_by_this_app: false,
                child: None,
            })),
        }
    }

    pub async fn status(&self) -> DshStatus {
        self.inner.lock().await.status()
    }

    /// 启动或复用 DSH。
    ///
    /// 决策树：
    /// 1) 已经拉起过 & 仍存活 → 直接返回；
    /// 2) PREFERRED_PORT 上有外部 dsh 存活 → 复用（owned=false）；
    /// 3) PREFERRED_PORT 空闲 → 用它 spawn；
    /// 4) PREFERRED_PORT 被非 dsh 占用 → ephemeral spawn。
    pub async fn launch(&self) -> Result<DshStatus> {
        // 先看自己是否已经在跑
        {
            let g = self.inner.lock().await;
            if let Some(p) = g.port {
                if is_dsh_alive(p).await {
                    return Ok(g.status());
                }
            }
        }

        // 探测首选端口
        if is_dsh_alive(PREFERRED_PORT).await {
            let mut g = self.inner.lock().await;
            g.port = Some(PREFERRED_PORT);
            g.owned_by_this_app = false; // 我们没拉起
            let status = g.status();
            drop(g);
            self.emit_status(&status);
            log::info!("复用外部 DSH，端口 {PREFERRED_PORT}");
            return Ok(status);
        }

        // 保证依赖已装（首次运行会走下载）
        download::ensure_installed(&self.app).await?;

        // 选端口 + spawn
        let port = launcher::pick_port()?;
        let handle = launcher::spawn_dsh(&self.app, port).await?;

        let mut g = self.inner.lock().await;
        g.port = Some(port);
        g.owned_by_this_app = true;
        g.child = Some(handle);
        let status = g.status();
        drop(g);
        self.emit_status(&status);
        Ok(status)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let mut g = self.inner.lock().await;
        if let Some(mut child) = g.child.take() {
            child.kill().await?;
        }
        // 外部 dsh：清状态但不杀
        g.port = None;
        g.owned_by_this_app = false;
        let status = g.status();
        drop(g);
        self.emit_status(&status);
        Ok(())
    }

    pub async fn restart(&self) -> Result<DshStatus> {
        // 只有我们自己拉起的才 shutdown；外部 dsh 的"重启"逻辑上不成立
        {
            let is_owned = {
                let g = self.inner.lock().await;
                g.owned_by_this_app && g.child.is_some()
            };
            if !is_owned {
                // 外部 dsh：只清状态，重新走 launch（届时会再次探测/复用/自拉）
                let mut g = self.inner.lock().await;
                g.port = None;
                g.owned_by_this_app = false;
                drop(g);
            } else {
                self.shutdown().await?;
            }
        }
        self.launch().await
    }

    /// 仅在 App 退出事件里调用；只回收本 APP 拉起的子进程。
    pub fn stop_on_exit(&self) {
        let mut g = match self.inner.try_lock() {
            Ok(g) => g,
            Err(_) => {
                log::warn!("[Exit] 无法获取 inner 锁，跳过清理（可能有正在进行的 spawn）");
                return;
            }
        };
        if !g.owned_by_this_app {
            log::info!("[Exit] 当前 DSH 为外部启动，保留不动");
            return;
        }
        if let Some(mut child) = g.child.take() {
            let _ = child.kill_blocking();
            log::info!("[Exit] 已回收 pid={}", child.pid);
        } else {
            log::info!("[Exit] 无子进程句柄，无需清理");
        }
    }

    fn emit_status(&self, s: &DshStatus) {
        let _ = self.app.emit("dsh://status", s.clone());
    }
}

/// 兼容旧引入：re-export
pub use launcher as _launcher;
