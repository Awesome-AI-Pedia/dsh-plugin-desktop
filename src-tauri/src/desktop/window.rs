//! 手动构建主窗口，以便注入 iframe shim 脚本

use tauri::{App, WebviewUrl, WebviewWindowBuilder};

use crate::desktop::shim::IFRAME_SHIM_JS;

pub fn build(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DeepSeek Harness Desktop")
        .inner_size(1280.0, 820.0)
        .min_inner_size(900.0, 600.0)
        .resizable(true)
        .center();

    // Tauri v2：initialization_script_for_all_frames 让脚本注入到主 frame 和 iframe
    builder = builder.initialization_script(IFRAME_SHIM_JS);

    let _win = builder.build()?;
    Ok(())
}
