//! spawn dsh 子进程 + 健康检查等待就绪

use std::net::TcpListener;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tauri::AppHandle;
use tokio::process::Command;

use crate::config::runtime::{
    self, DshEntry, HEALTH_CHECK_TIMEOUT_SECS, PREFERRED_PORT,
};
use crate::service::probe::{is_dsh_alive, is_port_in_use};
use crate::service::process::ChildHandle;

/// 选一个可用端口。首选 PREFERRED_PORT，被占则让内核分配。
pub fn pick_port() -> Result<u16> {
    if !is_port_in_use(PREFERRED_PORT) {
        return Ok(PREFERRED_PORT);
    }
    let l = TcpListener::bind("127.0.0.1:0").context("无法绑定 ephemeral 端口")?;
    let p = l.local_addr()?.port();
    drop(l);
    Ok(p)
}

/// 启动 dsh 子进程并等待就绪。
///
/// 返回：(ChildHandle, port)
pub async fn spawn_dsh(app: &AppHandle, port: u16) -> Result<ChildHandle> {
    let entry = runtime::resolve_dsh_entry(app)?;
    let dsh_home = runtime::dsh_home(app)?;
    let node_path = runtime::resolve_node(app)?;
    let log_path = runtime::logs_dir(app)?.join("dsh-web.log");

    log::info!(
        "启动 DSH：node={}, dsh_home={}, port={}",
        node_path.display(),
        dsh_home.display(),
        port
    );

    let mut cmd = match &entry {
        DshEntry::NodeScript(bin_js) => {
            let mut c = Command::new(&node_path);
            c.arg(bin_js).arg("web");
            c
        }
        DshEntry::Executable(exe) => {
            let mut c = Command::new(exe);
            c.arg("web");
            c
        }
    };
    cmd.arg("--host").arg("127.0.0.1")
       .arg("--port").arg(port.to_string());

    cmd.env("DSH_HOME", &dsh_home)
       .env("DSH_TELEMETRY_DISABLED", "1")
       .env("NO_COLOR", "1")
       .env("DSH_WEB_PORT", port.to_string());

    // 让 dsh 内部若再启动 node 子进程能找到 node
    if let Some(node_dir) = node_path.parent() {
        let existing = std::env::var_os("PATH").unwrap_or_default();
        let mut paths: Vec<_> = std::env::split_paths(&existing).collect();
        paths.insert(0, node_dir.to_path_buf());
        if let Ok(joined) = std::env::join_paths(&paths) {
            cmd.env("PATH", joined);
        }
    }

    // Unix: 让子进程成为 group leader，方便 kill -pid 灭整树
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setsid 在 pre_exec 里调用是标准做法
        unsafe {
            cmd.pre_exec(|| {
                if libc_setsid() < 0 {
                    // 忽略失败，回退到杀单进程
                }
                Ok(())
            });
        }
    }

    let handle = ChildHandle::spawn(cmd, port, log_path).await?;

    // 健康检查轮询
    for i in 0..HEALTH_CHECK_TIMEOUT_SECS {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if is_dsh_alive(port).await {
            log::info!("DSH 就绪：{i}s 后，端口 {port}");
            return Ok(handle);
        }
    }

    Err(anyhow!(
        "等待 DSH 服务就绪超时（{}s）。查看日志：{}",
        HEALTH_CHECK_TIMEOUT_SECS,
        handle.log_path.display()
    ))
}

#[cfg(unix)]
fn libc_setsid() -> i32 {
    extern "C" { fn setsid() -> i32; }
    unsafe { setsid() }
}
