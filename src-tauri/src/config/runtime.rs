//! 运行时路径 & 常量。M3 会扩展下载相关字段。

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use tauri::{AppHandle, Manager};

/// 首次自拉尝试的端口。被占则改用 ephemeral。
pub const PREFERRED_PORT: u16 = 3080;

/// 健康检查轮询：最多 SEC 秒，每秒一次
pub const HEALTH_CHECK_TIMEOUT_SECS: u64 = 30;

/// AppData 根目录（macOS: ~/Library/Application Support/<identifier>）
pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| anyhow!("无法获取 app_data 目录: {e}"))
}

/// DSH 服务的工作目录，作为 DSH_HOME 传入
pub fn dsh_home(app: &AppHandle) -> Result<PathBuf> {
    let p = app_data_dir(app)?.join("dsh-home");
    std::fs::create_dir_all(&p).ok();
    Ok(p)
}

/// dsh-pkg 解压位置（M3 下载后落这里）
pub fn dsh_pkg_dir(app: &AppHandle) -> Result<PathBuf> {
    let p = app_data_dir(app)?.join("dependencies").join("dsh");
    std::fs::create_dir_all(&p).ok();
    Ok(p)
}

/// 内置 Node runtime 位置（M3 下载后落这里）
pub fn node_runtime_dir(app: &AppHandle) -> Result<PathBuf> {
    let p = app_data_dir(app)?.join("runtime").join("node");
    std::fs::create_dir_all(&p).ok();
    Ok(p)
}

/// 日志目录
pub fn logs_dir(app: &AppHandle) -> Result<PathBuf> {
    let p = app_data_dir(app)?.join("logs");
    std::fs::create_dir_all(&p).ok();
    Ok(p)
}

/// 寻找 node 可执行文件。
/// 优先级：AppData/runtime/node/... > 系统 PATH。
pub fn resolve_node(app: &AppHandle) -> Result<PathBuf> {
    let bundled = node_runtime_dir(app)?;
    // 常见解压结构：node/bin/node（unix）或 node/node.exe（win）
    #[cfg(unix)]
    {
        let candidate = bundled.join("bin").join("node");
        if candidate.is_file() {
            return Ok(candidate);
        }
        // node-vXXX-<os>-<arch>/bin/node
        if let Ok(rd) = std::fs::read_dir(&bundled) {
            for e in rd.flatten() {
                let p = e.path().join("bin").join("node");
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
    }
    #[cfg(windows)]
    {
        let candidate = bundled.join("node.exe");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if let Ok(rd) = std::fs::read_dir(&bundled) {
            for e in rd.flatten() {
                let p = e.path().join("node.exe");
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
    }
    which("node").ok_or_else(|| anyhow!("找不到 node，可执行文件既不在内置 runtime 也不在系统 PATH。"))
}

/// 寻找 dsh 入口（lib/bin.js）。
/// 优先级：AppData/dependencies/dsh > 系统全局安装的 dsh。
pub fn resolve_dsh_entry(app: &AppHandle) -> Result<DshEntry> {
    let pkg = dsh_pkg_dir(app)?;
    // 结构 1：pkg/node_modules/@deepseek-ai/dsh/lib/bin.js
    let bin1 = pkg
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js");
    if bin1.is_file() {
        return Ok(DshEntry::NodeScript(bin1));
    }
    // 结构 2：pkg/lib/bin.js（如果发布包结构较扁）
    let bin2 = pkg.join("lib").join("bin.js");
    if bin2.is_file() {
        return Ok(DshEntry::NodeScript(bin2));
    }
    // 系统 PATH 上的 dsh
    if let Some(dsh) = which("dsh") {
        return Ok(DshEntry::Executable(dsh));
    }
    Err(anyhow!("找不到 DSH：请先运行 M3 的自动下载，或将 dsh 安装到系统 PATH。"))
}

pub enum DshEntry {
    /// node <path> 方式启动
    NodeScript(PathBuf),
    /// 直接执行的 dsh 可执行文件
    Executable(PathBuf),
}

/// 简易 which 实现，避免额外依赖
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let mut p = dir.join(name);
        #[cfg(windows)]
        {
            p.set_extension("cmd");
            if p.is_file() { return Some(p); }
            p.set_extension("exe");
            if p.is_file() { return Some(p); }
            p.set_extension("");
        }
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
