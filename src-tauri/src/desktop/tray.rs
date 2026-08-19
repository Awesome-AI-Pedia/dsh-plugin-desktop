//! 系统托盘：菜单（显示主窗口 / 重启 dsh / 完全退出）、左键点击图标唤起窗口。

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

use crate::service::HarnessManager;

pub fn install(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    let show_i = MenuItem::with_id(handle, "show", "显示主窗口", true, None::<&str>)?;
    let restart_i =
        MenuItem::with_id(handle, "restart", "重启 DSH 服务", true, None::<&str>)?;
    let sep = tauri::menu::PredefinedMenuItem::separator(handle)?;
    let quit_i = MenuItem::with_id(handle, "quit", "完全退出", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&show_i, &restart_i, &sep, &quit_i])?;

    // 图标：用默认应用图标
    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| Image::from_bytes(include_bytes!("../../icons/32x32.png"))
            .expect("加载托盘图标失败"));

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("DeepSeek Harness Desktop")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "restart" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(mgr) = app.try_state::<HarnessManager>() {
                        if let Err(e) = mgr.restart().await {
                            log::error!("重启 DSH 失败：{e:#}");
                        }
                    }
                });
            }
            "quit" => {
                // 真正退出：会触发 RunEvent::Exit → stop_on_exit
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_main_window(app);
            }
        })
        .build(handle)?;

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
