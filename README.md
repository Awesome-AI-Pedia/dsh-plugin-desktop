# DeepSeek Harness Desktop

DeepSeek Harness（DSH）的桌面客户端。采用 **Tauri v2 + 官方 Web UI**：
Rust 外壳负责窗口/托盘/子进程/下载/自动更新；业务界面（对话、Agent、插件页）**完全复用上游 DSH Web**，不重写。

## 架构一览

```
┌─────────────────────────────────────────┐
│  Tauri 主窗口                            │
│  ┌────────────────────────────────────┐ │
│  │ React 外壳（src/）                  │ │
│  │  - Navbar（刷新 / 重启服务）        │ │
│  │  - StatusOverlay（加载/下载/错误）  │ │
│  │  ┌──────────────────────────────┐  │ │
│  │  │ <iframe src=127.0.0.1:{port}>│  │ │
│  │  │   （官方 DSH Web，原样复用）  │  │ │
│  │  └──────────────────────────────┘  │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
         ↕ Tauri IPC / 事件
┌─────────────────────────────────────────┐
│  Rust（src-tauri/src/）                  │
│  bridge/    tauri command 桥            │
│  config/    路径 & 常量                  │
│  service/                               │
│    ├─ probe        端口/健康检查        │
│    ├─ launcher     spawn dsh 子进程     │
│    ├─ process      平台安全 kill        │
│    └─ download/    Node + dsh-pkg 下载  │
│  desktop/                               │
│    ├─ window       主窗口 + iframe shim │
│    ├─ tray         系统托盘             │
│    └─ shim         注入 iframe 的桥     │
└─────────────────────────────────────────┘
              ↕ spawn
       ┌──────────────────┐
       │  node bin.js web │
       │  --host 127.0.0.1 │
       │  --port {p}       │
       └──────────────────┘
```

## 关键行为

- **启动流程**：App 启动 → Rust 探测 `127.0.0.1:3080` 上是否有 dsh 存活。
  - 若有 → 直接复用（`owned_by_this_app = false`），iframe 加载；
  - 若无 → 首次运行下载 Node.js runtime（v22.22.0）+ DSH 运行时包（从 GitHub Release），
    然后 spawn `node <dsh>/lib/bin.js web --host 127.0.0.1 --port <ephemeral>`；
    健康检查最多 30s 等待就绪。
- **端口策略**：优先 3080；被占则用系统 ephemeral 端口。
- **进程隔离** ⚠️：**只回收本 App 拉起的子进程**。外部 `dsh web` 一根汗毛都不动。
  实现靠 Rust 侧 `Inner.child: Option<ChildHandle>`：外部服务下这个字段永远是 `None`，
  `stop_on_exit` 只对 `Some` 的情况执行 kill。
- **关闭 = 最小化到托盘**；托盘菜单：显示主窗口 / 重启 DSH 服务 / 完全退出。
- **iframe shim**：`window.open` 与 `<a target=_blank>` → 系统浏览器；
  `Notification` API → 系统通知；对 dsh 官方页面零改动。

## 开发

前置：
- Rust stable（`rustup`）
- Node ≥ 18（本项目自身开发用，运行时的 Node 由 App 自动下载）

```bash
# 装前端依赖
npm install

# 运行（开发模式：Rust + Vite 联动）
npm run tauri:dev
```

首次运行会往 App 数据目录下载 ~50MB Node runtime + ~200MB DSH pkg。
数据目录：
- macOS: `~/Library/Application Support/com.dsh.desktop/`
- Windows: `%APPDATA%/com.dsh.desktop/`
- Linux: `~/.local/share/com.dsh.desktop/`

结构：
```
<AppData>/
├── dsh-home/            # DSH_HOME（用户 profile、配置、缓存）
├── dependencies/dsh/    # DSH 运行时包解压
├── runtime/node/        # Node.js runtime 解压
├── downloads/           # 临时下载包（安装后自动清理）
└── logs/dsh-web.log     # dsh 子进程 stdout/stderr
```

## 打包

```bash
npm run tauri:build
```

产物：
- macOS: `.app` + `.dmg`（M1/Intel 各一份，需分别在两种架构上打）
- Windows: NSIS `.exe` 安装包
- Linux: `.AppImage` + `.deb`

产物路径：`src-tauri/target/release/bundle/`

### 自动更新（可选）

如需 GitHub Release 自动更新，追加 `tauri.conf.json`：

```json
"plugins": {
  "updater": {
    "endpoints": ["https://github.com/<you>/<repo>/releases/latest/download/latest.json"],
    "pubkey": "<你的 tauri signer public key>"
  }
}
```

密钥生成：`npx tauri signer generate -w ~/.tauri/dsh-desktop.key`。
签名后的 `.sig` 上传到 GitHub Release，前端调 `@tauri-apps/plugin-updater` 检查。

## 代码模块指引

| 目录 | 职责 |
|---|---|
| `src/store/harness.ts` | 前端状态机 + 事件监听（`dsh://status` / `dsh://download`） |
| `src/components/harness-webview.tsx` | iframe 宿主 + 顶栏 |
| `src/components/status-overlay.tsx` | 加载/下载/错误蒙层 |
| `src-tauri/src/bridge/mod.rs` | 6 个 tauri command：`launch_harness` / `shutdown_harness` / `restart_harness` / `get_dsh_status` / `install_dependencies` / `get_runtime_info` |
| `src-tauri/src/service/mod.rs` | `HarnessManager`：生命周期决策（复用外部 vs 自拉） |
| `src-tauri/src/service/launcher.rs` | 选端口 + 组装命令 + 健康检查轮询 |
| `src-tauri/src/service/process.rs` | 平台安全 kill（Unix setsid + TERM/KILL；Windows taskkill /T /F） |
| `src-tauri/src/service/download/` | Node + dsh-pkg 下载 + 解压 + 进度事件 |
| `src-tauri/src/service/probe.rs` | 端口探测 + HTTP 健康检查 |
| `src-tauri/src/desktop/window.rs` | 主窗口构造 + 注入 iframe shim |
| `src-tauri/src/desktop/tray.rs` | 系统托盘菜单 |
| `src-tauri/src/desktop/shim.rs` | 注入 iframe 的 JS 桥（window.open / Notification） |
| `src-tauri/src/config/runtime.rs` | 路径解析 + Node/DSH 定位 + 常量（`NODE_VERSION` / `PREFERRED_PORT`） |

## 常见问题

**Q: 我在终端手动跑着 dsh，会被 App 干掉吗？**
不会。App 检测到 3080 上有外部 dsh 会**直接复用**（前端状态显示"检测到外部 DSH 服务"），
退出时不会 kill 它。只有 App 自己拉起的子进程会被回收。

**Q: 首次下载卡住怎么办？**
- 检查网络到 `nodejs.org` 和 `github.com` 通不通。
- 手动删 `<AppData>/downloads/`、`<AppData>/dependencies/dsh/`、`<AppData>/runtime/node/` 再重启。
- 或提前把 `@deepseek-ai/dsh` 全局安装到系统 PATH，App 会优先探测系统 dsh。

**Q: 想换成固定端口？**
改 `src-tauri/src/config/runtime.rs` 里的 `PREFERRED_PORT`；或在 `launcher::pick_port` 里去掉 ephemeral fallback 强制 3080。

## 里程碑

- [x] M1 骨架：Tauri v2 + React 外壳 + iframe 加载 dsh web
- [x] M2 dsh 子进程 + 端口探测 + 生命周期（区分自拉/外部）
- [x] M3 首次下载 Node runtime + dsh-pkg（进度事件）
- [x] M4 系统托盘 + 关闭最小化到托盘 + 只杀自拉进程
- [x] M5 iframe 桥接（window.open / Notification）
- [x] M6 三平台打包配置（macOS/Windows/Linux）
- [x] M7 README + 调试信息

## 后续可选增强

- i18n（中/英切换）
- 首选项页（自定义端口、代理、日志级别）
- 调试侧边栏（实时看 dsh 日志、runtime 路径、进程 PID）
- 官方 DSH release 版本追踪（比对 commit，提示可更新）
- tauri-plugin-updater 自动更新（需密钥）
