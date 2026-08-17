# 术语表 (Glossary) — N-Top 置顶文本浮层

> 本文件在 Grilling 过程中持续更新。每轮拷问可能新增或修订条目。

## G-001 · 置顶浮层 (Top Overlay)
永远位于所有应用窗口之上的文本展示窗口。即使其他 App 全屏也可被覆盖于其下。

## G-002 · 点击穿透 (Click-through)
窗口不接受任何鼠标事件，指针"穿过"它作用到下方应用。实现层面对应 NSWindow level + `ignoresMouseEvents = true`。

## G-003 · 背景透明 / 文字不透明 (Background-only Opacity)
窗口背景可设置 0–100% 透明度；文字始终保持 100% 不透明以保证可读性。

## G-004 · 编辑态 / 穿透态 (Edit / Pass-through Mode)
互斥的两个工作模式。编辑态可接收键盘与鼠标输入修改文本；穿透态完全忽略鼠标（仅显示）。

## G-005 · 菜单栏控制面板 (Menu-bar Control Panel)
位于系统菜单栏的图标，下拉弹出一个含"切换可编辑"按钮和"透明度滑块"的小面板。

## G-006 · 全局热键 (Global Hotkey)
系统级快捷键（如 ⌥⌘E），任意应用前台时均可触发"切换编辑/穿透"。需要 Accessibility 或 Input Monitoring 权限授权。

## 待补充条目
- 文本持久化策略 (Persistence)
- 多显示器行为
- 文本字号/字体是否可配
- 窗口尺寸与定位行为

## G-007 · 整窗 Alpha vs 前端 rgba 背景 (Window Alpha vs CSS RGBA)
两种"背景透明"的实现路径：
- 整窗 alpha：Tauri `transparent: true` + 窗口 alpha 值，整个窗口（含文字）一起变淡。
- 前端 rgba 背景：窗口本身完全透明，前端用 `background: rgba(0,0,0,0.3)` 控制背景透明度，文字 `color: rgba(255,255,255,1)` 保持不透明。
后者才能满足"仅背景透明、文字清晰"的需求。

## G-008 · Tauri 窗口穿透 (Tauri Cursor Events)
Tauri v2 `WebviewWindow::set_ignore_cursor_events(true)` 对应底层 NSWindow `setIgnoresMouseEvents:`，实现鼠标穿透。

## G-009 · 全局热键插件 (Global Shortcut Plugin)
Tauri 官方 `tauri-plugin-global-shortcut`，在 macOS 上注册系统级快捷键，首次使用需用户在系统设置中授权辅助功能/输入监控权限。

## G-010 · 待定义：持久化策略 (Persistence)
文本内容的存储位置、格式、是否支持多实例。第 2 轮拷问中确定。

## G-011 · 单实例持久化 (Single-instance Persistence)
v1 仅一个浮层窗口。文本内容、窗口位置、外观设置统一存储到 `app_data_dir/n-top/` 下的 JSON 文件。重启时恢复全部状态。

## G-012 · 可拖拽不可缩放 (Draggable, Non-resizable)
窗口固定尺寸，用户可拖拽改变位置，位置会被记住。不提供缩放手柄或尺寸设置。

## G-013 · 外观参数 (Appearance Settings)
可配置项：背景透明度 α（0–1）、字号、字体颜色。纯文本+全局样式，不做行内富文本。

## G-014 · 待定义：菜单栏实现方式
NSStatusItem 原生 vs Tauri tray 插件。第 3 轮确定。

## G-015 · Tauri Tray 插件 (tauri-plugin-tray)
Tauri v2 官方菜单栏图标插件，提供系统托盘图标、点击事件、右键菜单能力。本项目用它实现菜单栏图标，不写原生 NSStatusItem。

## G-016 · 控制面板窗口 (Control Panel Window)
点击 tray 图标弹出的独立 WebviewWindow，anchor 到菜单栏图标位置。承载：切换编辑、透明度滑块、字号、字体颜色、热键设置。

## G-017 · 全屏置顶 (Always-on-top across Spaces)
Tauri 窗口设 `always_on_top: true` + `all_workspaces: true`，使浮层在所有 Space 和全屏 App 上方均可见。

## G-018 · 权限引导窗口 (Permission Onboarding Window)
首次启动时检测全局热键权限，未授权则弹出引导窗口，含"打开系统设置"按钮，授权后自动关闭。

## G-019 · 待定义：分发方式
DMG / Homebrew / App Store。第 4 轮确定。

## G-020 · DMG + Homebrew 分发 (DMG + Homebrew Cask)
通过 GitHub Release 托管 DMG 下载，同时自建 Homebrew tap 提供 `brew install --cask n-top` 安装。不上 App Store。

## G-021 · Gatekeeper 绕过 (Gatekeeper Bypass)
v1 无 Apple 代码签名，用户首次打开需"右键 → 打开"或"系统设置 → 隐私与安全性 → 仍要打开"。

## G-022 · Tauri Updater 签名 (Tauri Updater Signing)
`tauri-plugin-updater` 使用 Tauri 自有密钥对（`tauri signer generate`），与 Apple 代码签名独立。私钥签发更新包，公钥嵌入 App 验证。无 Apple Developer 账号也可实现自动更新。

## G-023 · 永久非目标 (Permanent Non-goals)
v1 和 v2 均不做：多实例浮层、窗口缩放、富文本、跨平台、App Store、云同步。唯一例外：自动更新是 v1 功能。
