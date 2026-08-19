mod bridge;
mod config;
mod desktop;
mod service;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // 初始化 dsh 服务管理器（挂到 State）
            let manager = service::HarnessManager::new(app.handle().clone());
            app.manage(manager);
            // 主窗口（手动构建以注入 iframe shim）
            desktop::window::build(app)?;
            // 系统托盘
            if let Err(e) = desktop::tray::install(app) {
                log::error!("安装托盘失败：{e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bridge::launch_harness,
            bridge::shutdown_harness,
            bridge::restart_harness,
            bridge::get_dsh_status,
            bridge::install_dependencies,
            bridge::get_runtime_info,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 关闭主窗口 = 最小化到托盘，不真正退出
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // 退出时清理本 APP 拉起的 dsh 子进程
                if let Some(mgr) = app_handle.try_state::<service::HarnessManager>() {
                    mgr.stop_on_exit();
                }
            }
        });
}
