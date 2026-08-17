# ADR-0002 · N-Top 设计冻结 (Design Freeze)

## 状态: 已冻结 (Frozen)

## 概述
N-Top 是一个 macOS 置顶文本浮层工具，用 Tauri 构建。用户在屏幕上始终可见一个可编辑的文本提醒框，背景可透明、文字保持清晰，支持鼠标穿透（不影响操作），通过菜单栏图标和全局热键控制。

## 核心 API 选型
- **框架**：Tauri v2
- **窗口透明**：`transparent: true` + 前端 `rgba` 背景控制
- **鼠标穿透**：`WebviewWindow::set_ignore_cursor_events(true)`
- **置顶**：`always_on_top: true` + `all_workspaces: true`
- **菜单栏**：`tauri-plugin-tray`
- **全局热键**：`tauri-plugin-global-shortcut`，默认 ⌥⌘E，可自定义
- **自动更新**：`tauri-plugin-updater`，Tauri 自有密钥签名

## 状态机
```
        ⌥⌘E / 控制面板按钮
编辑态 ←──────────────→ 穿透态
(ignoresMouseEvents=false)  (ignoresMouseEvents=true)
(textarea 获焦)              (键盘焦点交还下层 App)
```

## 数据持久化
单实例，存储到 `app_data_dir/n-top/`：
- `content.json`：文本内容 + 窗口位置 (x, y) + 外观设置 (背景透明度 α、字号、字体颜色、自定义热键)

## 控制面板
点击 tray 图标弹出独立 WebviewWindow，anchor 到 tray 位置：
- 切换可编辑按钮
- 背景透明度滑块（0–100%）
- 字号设置
- 字体颜色选择
- 全局热键设置

## 权限引导
首次启动检测全局热键权限，未授权弹引导窗口 → 一键跳转 `系统设置 → 隐私 → 辅助功能`。持续检测，授权后自动关闭。

## 分发
- DMG（GitHub Release）+ Homebrew Cask（自建 tap）
- 无 Apple 代码签名/公证（无 Developer 账号），用户首次需右键打开绕过 Gatekeeper
- 自动更新走 `tauri-plugin-updater` + Tauri 自有密钥签名，更新源为 GitHub Release

## 永久非目标（v1/v2 均不做）
多实例浮层 · 窗口缩放 · 富文本 · 跨平台 · App Store · 云同步

## 实现阶段待验证
- T-001：Tauri 穿透态键盘焦点释放路径
- T-002：global-shortcut 插件 macOS 权限引导细节
- T-003：窗口位置多显示器坐标恢复
- T-004：后续 Apple 签名/公证（获取 Developer 账号后）

---
*由 grill-with-docs 技能在第4轮拷问后冻结。对应完整决策日志见 ADR-0001。*
