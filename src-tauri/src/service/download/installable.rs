//! Installable：可安装依赖的抽象。
//! 目前有两个实现：Node runtime、DSH 运行时包。

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use tauri::AppHandle;

#[async_trait]
pub trait Installable: Send + Sync {
    /// 显示名，比如 "Node.js" / "DeepSeek Harness"
    fn title(&self) -> &'static str;

    /// 是否已安装到可用状态
    fn is_installed(&self, app: &AppHandle) -> bool;

    /// 下载 URL
    fn download_url(&self, app: &AppHandle) -> Result<String>;

    /// 下载后要落到磁盘的临时文件名（含扩展）
    fn download_filename(&self) -> String;

    /// 解压/安装目标目录
    fn install_dir(&self, app: &AppHandle) -> Result<PathBuf>;

    /// 是否为 tar.gz（否则默认 zip）
    fn is_targz(&self) -> bool { false }

    /// 解压前是否清空目标目录
    fn wipe_before_extract(&self) -> bool { true }
}
