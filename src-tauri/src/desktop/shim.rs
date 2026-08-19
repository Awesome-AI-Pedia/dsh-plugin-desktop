//! 注入 iframe 的 shim 脚本：
//! - `window.open(url)` / `<a target=_blank>` → 通过 tauri opener 走系统浏览器；
//! - `Notification` API → 通过 tauri notification 插件；
//! - `window.close()` → 隐藏主窗口。
//!
//! 通过 `initialization_script_for_all_frames` 在页面加载前注入。

pub const IFRAME_SHIM_JS: &str = r#"
(function() {
  if (window.__DSH_DESKTOP_SHIM__) return;
  window.__DSH_DESKTOP_SHIM__ = true;

  // ---- window.open → 系统浏览器 ----
  const originalOpen = window.open;
  window.open = function(url, target, features) {
    try {
      if (url && /^https?:\/\//i.test(String(url))) {
        if (window.__TAURI__ && window.__TAURI__.opener) {
          window.__TAURI__.opener.openUrl(String(url)).catch(function(){});
          return null;
        }
      }
    } catch (e) { console.warn('[dsh-shim] open failed', e); }
    return originalOpen ? originalOpen.call(window, url, target, features) : null;
  };

  // ---- <a target="_blank"> 拦截 ----
  document.addEventListener('click', function(e) {
    const a = (e.target instanceof Element) ? e.target.closest('a[href]') : null;
    if (!a) return;
    const href = a.getAttribute('href') || '';
    const target = a.getAttribute('target');
    if (target === '_blank' && /^https?:\/\//i.test(href)) {
      e.preventDefault();
      if (window.__TAURI__ && window.__TAURI__.opener) {
        window.__TAURI__.opener.openUrl(href).catch(function(){});
      }
    }
  }, true);

  // ---- Notification 桥 ----
  if (window.__TAURI__ && window.__TAURI__.notification) {
    const notif = window.__TAURI__.notification;
    const OriginalNotification = window.Notification;
    function DshNotification(title, options) {
      try {
        notif.sendNotification({
          title: String(title || 'DeepSeek Harness'),
          body: options && options.body ? String(options.body) : '',
        });
      } catch (e) { console.warn('[dsh-shim] notify failed', e); }
      if (OriginalNotification) {
        try { return new OriginalNotification(title, options); } catch (_) {}
      }
      return { close: function(){}, addEventListener: function(){}, removeEventListener: function(){}};
    }
    DshNotification.permission = 'granted';
    DshNotification.requestPermission = function() { return Promise.resolve('granted'); };
    try { window.Notification = DshNotification; } catch (_) {}
  }
})();
"#;
