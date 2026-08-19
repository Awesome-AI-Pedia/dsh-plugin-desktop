//! DSH 运行时包（含 node_modules，开箱即用）。
//!
//! 从 GitHub Releases 下载预打包 zip：
//! https://github.com/hairyf/deepseek-harness-pkg/releases/latest/download/deepseek-harness-pkg-{platform}.zip

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tauri::AppHandle;

use crate::config::runtime;
use crate::service::download::installable::Installable;

const DSH_PKG_BASE: &str =
    "https://github.com/hairyf/deepseek-harness-pkg/releases/latest/download";

pub struct DshInstallable;

impl DshInstallable {
    fn platform_tag() -> Result<&'static str> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match (os, arch) {
            ("macos", "aarch64") => Ok("macos-arm64"),
            ("macos", "x86_64") => Ok("macos-x64"),
            ("linux", _) => Ok("linux"),
            ("windows", _) => Ok("windows"),
            _ => Err(anyhow!("dsh-pkg 不支持的平台 {os}/{arch}")),
        }
    }
}

#[async_trait]
impl Installable for DshInstallable {
    fn title(&self) -> &'static str { "DeepSeek Harness" }

    fn is_installed(&self, app: &AppHandle) -> bool {
        runtime::resolve_dsh_entry(app).is_ok()
    }

    fn download_url(&self, _app: &AppHandle) -> Result<String> {
        let tag = Self::platform_tag()?;
        Ok(format!("{DSH_PKG_BASE}/deepseek-harness-pkg-{tag}.zip"))
    }

    fn download_filename(&self) -> String {
        format!(
            "deepseek-harness-pkg-{}.zip",
            Self::platform_tag().unwrap_or("unknown")
        )
    }

    fn install_dir(&self, app: &AppHandle) -> Result<PathBuf> {
        runtime::dsh_pkg_dir(app)
    }

    fn is_targz(&self) -> bool { false }
}
