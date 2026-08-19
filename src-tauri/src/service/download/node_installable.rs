//! Node.js runtime 安装源。
//!
//! Node 版本固定 v22.22.0，从 nodejs.org 官方分发下载。
//! macOS: node-v22.22.0-darwin-{arm64|x64}.tar.gz
//! Windows: node-v22.22.0-win-x64.zip
//! Linux: node-v22.22.0-linux-x64.tar.gz

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tauri::AppHandle;

use crate::config::runtime;
use crate::service::download::installable::Installable;

pub const NODE_VERSION: &str = "v22.22.0";
const NODE_BASE_URL: &str = "https://nodejs.org/dist";

pub struct NodeInstallable;

impl NodeInstallable {
    fn arch_triplet() -> Result<(&'static str, &'static str, bool)> {
        // (os, arch, is_targz)
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match (os, arch) {
            ("macos", "aarch64") => Ok(("darwin", "arm64", true)),
            ("macos", "x86_64") => Ok(("darwin", "x64", true)),
            ("linux", "x86_64") => Ok(("linux", "x64", true)),
            ("linux", "aarch64") => Ok(("linux", "arm64", true)),
            ("windows", "x86_64") => Ok(("win", "x64", false)),
            _ => Err(anyhow!("不支持的平台 {os}/{arch}")),
        }
    }

    fn asset_name() -> Result<String> {
        let (os, arch, targz) = Self::arch_triplet()?;
        let ext = if targz { "tar.gz" } else { "zip" };
        Ok(format!("node-{NODE_VERSION}-{os}-{arch}.{ext}"))
    }
}

#[async_trait]
impl Installable for NodeInstallable {
    fn title(&self) -> &'static str { "Node.js" }

    fn is_installed(&self, app: &AppHandle) -> bool {
        runtime::resolve_node(app)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn download_url(&self, _app: &AppHandle) -> Result<String> {
        Ok(format!("{NODE_BASE_URL}/{NODE_VERSION}/{}", Self::asset_name()?))
    }

    fn download_filename(&self) -> String {
        Self::asset_name().unwrap_or_else(|_| "node.tar.gz".to_string())
    }

    fn install_dir(&self, app: &AppHandle) -> Result<PathBuf> {
        runtime::node_runtime_dir(app)
    }

    fn is_targz(&self) -> bool {
        Self::arch_triplet().map(|(_, _, tg)| tg).unwrap_or(true)
    }
}
