use serde::Serialize;
use tauri::{AppHandle, State};

use crate::service::{download, DshStatus, HarnessManager};

#[tauri::command]
pub async fn launch_harness(mgr: State<'_, HarnessManager>) -> Result<DshStatus, CmdError> {
    mgr.launch().await.map_err(Into::into)
}

#[tauri::command]
pub async fn install_dependencies(app: AppHandle) -> Result<(), CmdError> {
    download::ensure_installed(&app).await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_runtime_info(app: AppHandle) -> Result<RuntimeInfo, CmdError> {
    use crate::config::runtime as rt;
    Ok(RuntimeInfo {
        app_data: rt::app_data_dir(&app).ok().map(|p| p.display().to_string()),
        dsh_home: rt::dsh_home(&app).ok().map(|p| p.display().to_string()),
        dsh_pkg: rt::dsh_pkg_dir(&app).ok().map(|p| p.display().to_string()),
        node_dir: rt::node_runtime_dir(&app).ok().map(|p| p.display().to_string()),
        logs: rt::logs_dir(&app).ok().map(|p| p.display().to_string()),
        node_resolved: rt::resolve_node(&app).ok().map(|p| p.display().to_string()),
        dsh_resolved: rt::resolve_dsh_entry(&app).ok().map(|e| match e {
            rt::DshEntry::NodeScript(p) => format!("node {}", p.display()),
            rt::DshEntry::Executable(p) => p.display().to_string(),
        }),
    })
}

#[derive(Debug, Serialize)]
pub struct RuntimeInfo {
    pub app_data: Option<String>,
    pub dsh_home: Option<String>,
    pub dsh_pkg: Option<String>,
    pub node_dir: Option<String>,
    pub logs: Option<String>,
    pub node_resolved: Option<String>,
    pub dsh_resolved: Option<String>,
}

#[tauri::command]
pub async fn shutdown_harness(mgr: State<'_, HarnessManager>) -> Result<(), CmdError> {
    mgr.shutdown().await.map_err(Into::into)
}

#[tauri::command]
pub async fn restart_harness(mgr: State<'_, HarnessManager>) -> Result<DshStatus, CmdError> {
    mgr.restart().await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_dsh_status(mgr: State<'_, HarnessManager>) -> Result<DshStatus, CmdError> {
    Ok(mgr.status().await)
}

#[derive(Debug, Serialize)]
pub struct CmdError {
    message: String,
}

impl From<anyhow::Error> for CmdError {
    fn from(e: anyhow::Error) -> Self {
        Self { message: format!("{e:#}") }
    }
}
