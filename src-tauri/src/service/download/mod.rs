//! Download 子模块：负责首次运行时下载 Node runtime + DSH pkg。

pub mod core;
pub mod dsh_installable;
pub mod extract;
pub mod installable;
pub mod node_installable;

use anyhow::{Context, Result};
use tauri::AppHandle;

use crate::config::runtime;
use crate::service::download::dsh_installable::DshInstallable;
use crate::service::download::installable::Installable;
use crate::service::download::node_installable::NodeInstallable;

/// 保证 Node + DSH 已安装。已装则跳过；未装则下载→解压。
///
/// 进度通过 `dsh://download` 事件流出。
pub async fn ensure_installed(app: &AppHandle) -> Result<()> {
    let installables: Vec<Box<dyn Installable>> = vec![
        Box::new(NodeInstallable),
        Box::new(DshInstallable),
    ];

    for item in installables {
        if item.is_installed(app) {
            log::info!("{} 已安装，跳过", item.title());
            continue;
        }
        install_one(app, item.as_ref()).await
            .with_context(|| format!("安装 {} 失败", item.title()))?;
    }
    Ok(())
}

async fn install_one(app: &AppHandle, item: &dyn Installable) -> Result<()> {
    let url = item.download_url(app)?;
    let install_dir = item.install_dir(app)?;
    let tmp_root = runtime::app_data_dir(app)?.join("downloads");
    tokio::fs::create_dir_all(&tmp_root).await.ok();
    let archive_path = tmp_root.join(item.download_filename());

    let stage_download = format!("下载 {}", item.title());
    core::download_to(app, &stage_download, &url, &archive_path).await?;

    let stage_extract = format!("解压 {}", item.title());
    log::info!("{stage_extract} → {}", install_dir.display());
    // 解压前清目录（可选）
    if item.wipe_before_extract() && install_dir.exists() {
        // 保留目录本身，只删内部
        if let Ok(entries) = std::fs::read_dir(&install_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }
    std::fs::create_dir_all(&install_dir).ok();

    let archive_clone = archive_path.clone();
    let install_dir_clone = install_dir.clone();
    let is_targz = item.is_targz();
    let title = item.title().to_string();
    // 解压放在阻塞线程池
    tokio::task::spawn_blocking(move || -> Result<()> {
        if is_targz {
            extract::extract_targz(&archive_clone, &install_dir_clone)?;
        } else {
            extract::extract_zip(&archive_clone, &install_dir_clone)?;
        }
        Ok(())
    })
    .await
    .with_context(|| format!("解压 {title} 的阻塞任务 panicked"))??;

    // 清理压缩包
    let _ = std::fs::remove_file(&archive_path);

    // 通知前端本阶段完成（100%）
    use tauri::Emitter;
    let _ = app.emit(
        "dsh://download",
        core::DownloadProgress { stage: format!("{} 已就绪", item.title()), percent: 100 },
    );
    Ok(())
}
