//! HTTP 流式下载 + 进度事件 emit

use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub stage: String,
    pub percent: u32,
}

pub async fn download_to(
    app: &AppHandle,
    stage: &str,
    url: &str,
    dest: &Path,
) -> Result<()> {
    log::info!("下载 {stage} 从 {url} → {}", dest.display());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(30))
        .build()?;
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("发起下载失败: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "下载失败: {} → HTTP {}",
            url,
            resp.status()
        ));
    }
    let total = resp.content_length().unwrap_or(0);

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let mut file = File::create(dest)
        .await
        .with_context(|| format!("无法创建下载文件 {}", dest.display()))?;

    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_percent: u32 = u32::MAX;

    emit(app, stage, 0);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("读取响应流失败")?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let p = ((downloaded * 100) / total) as u32;
            if p != last_percent {
                last_percent = p;
                emit(app, stage, p);
            }
        }
    }
    file.flush().await?;
    emit(app, stage, 100);
    Ok(())
}

fn emit(app: &AppHandle, stage: &str, percent: u32) {
    let _ = app.emit(
        "dsh://download",
        DownloadProgress { stage: stage.to_string(), percent },
    );
}
