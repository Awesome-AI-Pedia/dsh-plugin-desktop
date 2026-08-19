//! 子进程句柄 + 日志重定向 + 平台安全 kill

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

pub struct ChildHandle {
    pub pid: u32,
    child: Option<Child>,
    /// 记录用来诊断
    pub port: u16,
    pub log_path: PathBuf,
}

impl ChildHandle {
    /// 启动 dsh 子进程。
    ///
    /// `cmd` 已经组装好（node + bin.js + 参数、env、cwd）。
    /// stdout / stderr 全部落到 `log_path`，避免阻塞与丢日志。
    pub async fn spawn(mut cmd: Command, port: u16, log_path: PathBuf) -> Result<Self> {
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW = 0x08000000，避免子进程闪出黑窗
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        let mut child = cmd.spawn().context("spawn dsh 子进程失败")?;
        let pid = child.id().ok_or_else(|| anyhow!("spawn 后无法取得 pid"))?;

        // 打开日志文件（追加）
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("无法打开日志文件 {}", log_path.display()))?;
        let log_file = std::sync::Arc::new(std::sync::Mutex::new(log_file));

        if let Some(stdout) = child.stdout.take() {
            let lf = log_file.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    use std::io::Write;
                    if let Ok(mut f) = lf.lock() {
                        let _ = writeln!(f, "[stdout] {line}");
                    }
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            let lf = log_file.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    use std::io::Write;
                    if let Ok(mut f) = lf.lock() {
                        let _ = writeln!(f, "[stderr] {line}");
                    }
                }
            });
        }

        Ok(Self { pid, child: Some(child), port, log_path })
    }

    /// 优雅 kill：先 TERM 等 2s，然后 KILL（含进程树）。
    pub async fn kill(&mut self) -> Result<()> {
        let pid = self.pid;
        log::info!("停止 dsh 子进程 pid={pid}");
        kill_tree(pid).await;

        if let Some(mut child) = self.child.take() {
            // 兜底：等待 tokio Child 退出，避免僵尸
            let _ = tokio::time::timeout(std::time::Duration::from_secs(3), child.wait()).await;
        }
        Ok(())
    }

    /// 同步版：仅在 App 退出事件中使用。
    pub fn kill_blocking(&mut self) -> Result<()> {
        let pid = self.pid;
        log::info!("[Exit] 同步停止 dsh 子进程 pid={pid}");
        kill_tree_blocking(pid);
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
        Ok(())
    }
}

// --- 平台特定 kill ---

#[cfg(unix)]
async fn kill_tree(pid: u32) {
    // Unix：先 TERM 到进程组（-pid），等 2s，再 KILL
    tokio::task::spawn_blocking(move || unsafe {
        // 尝试杀进程组（如果 spawn 时是 group leader；否则退化为杀单进程）
        libc_kill(-(pid as i32), libc_SIGTERM);
        libc_kill(pid as i32, libc_SIGTERM);
    })
    .await
    .ok();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    tokio::task::spawn_blocking(move || unsafe {
        libc_kill(-(pid as i32), libc_SIGKILL);
        libc_kill(pid as i32, libc_SIGKILL);
    })
    .await
    .ok();
}

#[cfg(unix)]
fn kill_tree_blocking(pid: u32) {
    unsafe {
        libc_kill(-(pid as i32), libc_SIGTERM);
        libc_kill(pid as i32, libc_SIGTERM);
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    unsafe {
        libc_kill(-(pid as i32), libc_SIGKILL);
        libc_kill(pid as i32, libc_SIGKILL);
    }
}

#[cfg(unix)]
const libc_SIGTERM: i32 = 15;
#[cfg(unix)]
const libc_SIGKILL: i32 = 9;

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) {
    extern "C" { fn kill(pid: i32, sig: i32) -> i32; }
    unsafe { kill(pid, sig); }
}

#[cfg(windows)]
async fn kill_tree(pid: u32) {
    tokio::task::spawn_blocking(move || {
        // taskkill /T = 树；/F = 强制
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .creation_flags_no_window()
            .status();
    })
    .await
    .ok();
}

#[cfg(windows)]
fn kill_tree_blocking(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .creation_flags_no_window()
        .status();
}

#[cfg(windows)]
trait CommandExtNoWindow {
    fn creation_flags_no_window(&mut self) -> &mut Self;
}
#[cfg(windows)]
impl CommandExtNoWindow for std::process::Command {
    fn creation_flags_no_window(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(0x08000000);
        self
    }
}
