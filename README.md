# N-Top

macOS 置顶文本浮层工具 — 在所有应用窗口之上显示可编辑的文本提醒，支持鼠标穿透、背景透明、全局热键。

## 功能

- **置顶浮层**：始终显示在所有应用之上，全屏 App 也不遮挡
- **自由文本**：多行文本框，随意编辑
- **鼠标穿透**：穿透态下鼠标点击直接作用到下方应用，不影响操作
- **仅背景透明**：背景透明度 0–100% 可调，文字始终清晰
- **菜单栏下拉面板**：点菜单栏图标，面板贴在图标下方弹出；再点一次图标、按 Esc、点 ✕ 或点到别的 App 都会收起，鼠标移到浮层上编辑时保持展开
- **面板内建操作**：编辑/穿透切换、透明度、字号、颜色、热键、历史记录、退出 N-Top。打开面板只把焦点交给面板，不会改变浮层的编辑/穿透状态
- **历史记录弹窗**：面板里的「历史记录」按钮打开独立窗口（旧版复用的那个），可展开全文、复制、手动刷新；窗口关掉后下次打开重建
- **全局热键**：默认 ⌥⌘E 切换编辑/穿透，可自定义
- **自动持久化**：文本内容、窗口位置、外观设置自动保存

## 开发

### 前置要求

- Node.js ≥ 18
- Rust（通过 rustup 安装）
- Xcode Command Line Tools

### 运行

```bash
npm install
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/`。

## 技术栈

- **Tauri v2**（Rust + Webview）
- **前端**：TypeScript + Vite
- **插件**：tray-icon、global-shortcut、updater

## 设计文档

- [ADR-0001 — Grilling 决策日志](docs/adr/ADR-0001-grilling-log.md)
- [ADR-0002 — 设计冻结](docs/adr/ADR-0002-design-freeze.md)
- [术语表](docs/glossary/GLOSSARY.md)
