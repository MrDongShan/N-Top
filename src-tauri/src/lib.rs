use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::{Effect, EffectState, EffectsBuilder},
    Emitter, Manager, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// 控制面板窗口标签
const CONTROL_PANEL_LABEL: &str = "control-panel";
/// 历史记录窗口标签
const HISTORY_LABEL: &str = "history";
/// 托盘图标 id，用来查图标在菜单栏上的位置
const TRAY_ID: &str = "ntop-tray";

/// 控制面板窗口尺寸（逻辑点），与 styles.css 里 .cp 的铺满布局一一对应
const PANEL_WIDTH: f64 = 320.0;
/// 面板内容一屏放下（headless 实测：正文 613 + 头部/常驻区 155，再留一点呼吸位）
const PANEL_HEIGHT: f64 = 786.0;
/// 面板圆角：必须与 styles.css 的 --glass-radius 一致，否则两层圆角会错位
const PANEL_RADIUS: f64 = 20.0;
/// 面板顶部与菜单栏图标之间的间距
const PANEL_TRAY_GAP: f64 = 6.0;
/// 失焦后先等这么久再判断，让焦点事件落地
const PANEL_AUTOHIDE_DELAY: Duration = Duration::from_millis(180);
/// 鼠标停在浮窗上时，最多再多等这么久
const PANEL_AUTOHIDE_MAX_WAIT: Duration = Duration::from_secs(10);
/// 前端声明的「正在交互」保护时长（系统取色器这类原生弹窗会抢走焦点）
const PANEL_KEEP_ALIVE: Duration = Duration::from_millis(2500);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OverlayMode {
    Edit,
    PassThrough,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
#[serde(default)]
pub struct AppSettings {
    pub text: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub bg_alpha: f64,
    pub bg_color: String,
    pub font_size: u32,
    pub font_color: String,
    pub hotkey: String,
    pub text_stroke: bool,
    pub show_in_dock: bool,
    pub text_flash_interval_seconds: u32,
    pub show_first_paragraph: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            text: String::from("在这里写你的提醒…"),
            pos_x: 100.0,
            pos_y: 100.0,
            bg_alpha: 0.3,
            bg_color: String::from("#fffaf0"),
            font_size: 16,
            font_color: String::from("#000000"),
            hotkey: String::from("Alt+Command+E"),
            text_stroke: true,
            show_in_dock: false,
            text_flash_interval_seconds: 300,
            show_first_paragraph: false,
        }
    }
}

pub struct AppState {
    pub mode: Mutex<OverlayMode>,
    pub settings: Mutex<AppSettings>,
    pub mouse_in_overlay: Mutex<bool>,
    /// 控制面板「请勿自动收起」的截止时间
    pub panel_keep_alive: Mutex<Option<Instant>>,
    /// 面板是否处于展开状态（判断点击是「开」还是「关」以它为准）
    pub panel_open: Mutex<bool>,
    /// 每次开/关都自增，用来让更早排队的自动收起失效
    pub panel_generation: Mutex<u64>,
}

fn settings_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let dir = app.path().app_data_dir().unwrap_or_else(|_| {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".n-top")
    });
    let _ = std::fs::create_dir_all(&dir);
    dir.join("content.json")
}

fn history_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    let dir = app.path().app_data_dir().unwrap_or_else(|_| {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".n-top")
    });
    let history = dir.join("history");
    let _ = std::fs::create_dir_all(&history);
    history
}

fn save_history_snapshot(app: &tauri::AppHandle, settings: &AppSettings) {
    let dir = history_dir(app);

    // Check if the latest snapshot has identical text — skip if so
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<_> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        files.sort();
        files.reverse();
        if let Some(latest) = files.first() {
            let latest_path = dir.join(latest);
            if let Ok(latest_json) = std::fs::read_to_string(&latest_path) {
                if let Ok(latest_settings) = serde_json::from_str::<AppSettings>(&latest_json) {
                    if latest_settings.text == settings.text {
                        return;
                    }
                }
            }
        }
    }

    let now = chrono::Local::now();
    let filename = format!("{}.json", now.format("%Y-%m-%d_%H-%M-%S"));
    let path = dir.join(&filename);
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, json);
    }
}

fn cleanup_old_history(app: &tauri::AppHandle) {
    let dir = history_dir(app);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Keep only today's files
            if !name.starts_with(&today) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let path = settings_path(app);
    let backup_path = path.with_extension("json.bak");

    if let Ok(contents) = std::fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&contents) {
            return settings;
        }
    }

    if let Ok(contents) = std::fs::read_to_string(&backup_path) {
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&contents) {
            return settings;
        }
    }

    AppSettings::default()
}

fn save_settings(app: &tauri::AppHandle, settings: &AppSettings) {
    let path = settings_path(app);
    let backup_path = path.with_extension("json.bak");
    let temp_path = path.with_extension("json.tmp");

    let Ok(json) = serde_json::to_string_pretty(settings) else {
        return;
    };

    if std::fs::write(&temp_path, json).is_err() {
        return;
    }

    if path.exists() {
        let _ = std::fs::copy(&path, &backup_path);
    }

    let _ = std::fs::rename(&temp_path, &path);
}

fn save_overlay_position(app: &tauri::AppHandle, position: tauri::PhysicalPosition<i32>) {
    // 显示器热插拔/系统切换瞬间 Moved 事件可能上报屏幕外的脏坐标，直接丢弃，
    // 否则下次启动会把浮窗恢复到屏幕外。
    if let Some(window) = app.get_webview_window("overlay") {
        if let Ok(size) = window.outer_size() {
            if !is_window_visible_on_any_monitor(app, position, size) {
                return;
            }
        }
    }

    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.pos_x = f64::from(position.x);
    settings.pos_y = f64::from(position.y);
    save_settings(app, &settings);
}

/// 窗口在任意一块显示器上是否有足够可见区域（可被看到并拖回）
fn is_window_visible_on_any_monitor(
    app: &tauri::AppHandle,
    pos: tauri::PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
) -> bool {
    let Ok(monitors) = app.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        return true;
    }

    monitors.iter().any(|m| {
        let (mx, my) = (m.position().x as f64, m.position().y as f64);
        let (mw, mh) = (m.size().width as f64, m.size().height as f64);
        let (wx, wy) = (pos.x as f64, pos.y as f64);
        let (ww, wh) = (size.width as f64, size.height as f64);
        let overlap_w = (wx + ww).min(mx + mw) - wx.max(mx);
        let overlap_h = (wy + wh).min(my + mh) - wy.max(my);
        overlap_w >= 60.0 && overlap_h >= 40.0
    })
}

/// Get current mouse cursor position via core-graphics
fn get_mouse_position() -> Option<(f64, f64)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let point = event.location();
    Some((point.x, point.y))
}

/// Check if a point is inside the overlay window bounds
fn is_point_in_overlay(app: &tauri::AppHandle, px: f64, py: f64) -> bool {
    let Some(window) = app.get_webview_window("overlay") else {
        return false;
    };
    let Ok(pos) = window.outer_position() else {
        return false;
    };
    let Ok(size) = window.outer_size() else {
        return false;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    // CGEvent returns logical pixels; Tauri returns physical pixels.
    // Convert mouse coords to physical pixels to match window coords.
    let px = px * scale;
    let py = py * scale;
    let x = pos.x as f64;
    let y = pos.y as f64;
    let w = size.width as f64;
    let h = size.height as f64;
    px >= x && px <= x + w && py >= y && py <= y + h
}

/// Start a background thread that polls mouse position when in pass-through mode
fn start_mouse_poll(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(50));

        let state = app.state::<AppState>();
        let mode = *state.mode.lock().unwrap();

        if mode != OverlayMode::PassThrough {
            // In edit mode, always visible
            let mut in_overlay = state.mouse_in_overlay.lock().unwrap();
            if *in_overlay {
                *in_overlay = false;
                let _ = app.emit("mouse-hover", false);
            }
            continue;
        }

        // Pass-through mode: check if mouse is over the overlay
        let Some((mx, my)) = get_mouse_position() else {
            continue;
        };
        let in_overlay = is_point_in_overlay(&app, mx, my);

        let mut current = state.mouse_in_overlay.lock().unwrap();
        if *current != in_overlay {
            *current = in_overlay;
            let _ = app.emit("mouse-hover", in_overlay);
        }
    });
}

fn set_mode(app: &tauri::AppHandle, mode: OverlayMode) {
    let state = app.state::<AppState>();
    *state.mode.lock().unwrap() = mode;

    if let Some(window) = app.get_webview_window("overlay") {
        match mode {
            OverlayMode::Edit => {
                let _ = window.set_ignore_cursor_events(false);
                let _ = window.set_focus();
                let _ = app.emit("mode-changed", "edit");
            }
            OverlayMode::PassThrough => {
                let _ = window.set_ignore_cursor_events(true);
                let _ = app.emit("mode-changed", "pass-through");
            }
        }
    }
}

fn toggle_mode(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let current = *state.mode.lock().unwrap();
    let next = match current {
        OverlayMode::Edit => OverlayMode::PassThrough,
        OverlayMode::PassThrough => OverlayMode::Edit,
    };
    set_mode(app, next);
}

/// 托盘图标在菜单栏上的位置和尺寸（物理像素，按图标所在显示器的缩放换算）
fn tray_rect(app: &tauri::AppHandle) -> Option<(f64, f64, f64, f64)> {
    let tray = app.tray_by_id(TRAY_ID)?;
    let rect = tray.rect().ok().flatten()?;
    let pos = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<f64>(1.0);
    Some((pos.x, pos.y, size.width, size.height))
}

/// 点 (x, y) 落在哪个显示器上。
/// 不能用 AppHandle::monitor_from_point：macOS 上它按 CGDisplayBounds（点）判断，
/// 而 Monitor::position/size 又是「点 × 该屏缩放」，两套坐标在缩放不同的
/// 第二块屏上对不上，会直接返回 None。
fn monitor_containing(app: &tauri::AppHandle, x: f64, y: f64) -> Option<tauri::Monitor> {
    let monitors = app.available_monitors().ok()?;
    monitors.into_iter().find(|monitor| {
        let pos = monitor.position();
        let size = monitor.size();
        let (min_x, min_y) = (pos.x as f64, pos.y as f64);
        let (max_x, max_y) = (min_x + size.width as f64, min_y + size.height as f64);
        x >= min_x && x < max_x && y >= min_y && y < max_y
    })
}

/// 懒创建控制面板：无边框透明浮窗，背景交给 NSVisualEffectView 做毛玻璃，
/// 圆角、投影由窗口本身提供，CSS 只叠白色薄雾和内描边
fn ensure_control_panel(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window(CONTROL_PANEL_LABEL) {
        return Some(window);
    }

    match WebviewWindowBuilder::new(
        app,
        CONTROL_PANEL_LABEL,
        tauri::WebviewUrl::App("index.html#/control-panel".into()),
    )
    .title("N-Top")
    .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    .transparent(true)
    .effects(
        EffectsBuilder::new()
            .effect(Effect::Popover)
            .state(EffectState::Active)
            .radius(PANEL_RADIUS)
            .build(),
    )
    // 面板固定浅色：系统的深色模式也不该把毛玻璃染黑
    .theme(Some(tauri::Theme::Light))
    // 投影也跟着圆角走，直接由窗口画，不用再留透明边
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    // 面板要能出现在任何桌面/显示器上，否则切到另一个显示器的桌面后
    // 点开也看不到（窗口还留在原来的桌面上）
    .visible_on_all_workspaces(true)
    .visible(false)
    .build()
    {
        Ok(window) => Some(window),
        Err(e) => {
            eprintln!("创建控制面板失败: {}", e);
            None
        }
    }
}

/// 把面板贴到菜单栏图标正下方，并夹在图标所在显示器的可视范围内。
///
/// 托盘 rect 和 Monitor::position/size 都是「逻辑点 × 该屏缩放」的物理像素，
/// 所以先在这一套坐标里算，最后除以图标所在显示器的缩放换回逻辑点再设位置。
/// 直接把这套物理值交给 set_position 会被 tao 用「窗口当前所在显示器」的缩放
/// 再解释一次，两台显示器缩放不同时就会算到屏幕外面，表现为点好几次才出来。
fn position_control_panel(app: &tauri::AppHandle, window: &tauri::WebviewWindow) {
    let Some((tray_x, tray_y, tray_w, tray_h)) = tray_rect(app) else {
        return;
    };

    let icon_center_x = tray_x + tray_w / 2.0;
    let icon_bottom = tray_y + tray_h;

    let monitor = monitor_containing(app, icon_center_x, tray_y + tray_h / 2.0);
    let scale = monitor
        .as_ref()
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0)
        .max(1.0);

    // 面板尺寸（逻辑点）换算到同一套物理像素
    let win_w = PANEL_WIDTH * scale;
    let win_h = PANEL_HEIGHT * scale;

    let mut x = icon_center_x - win_w / 2.0;
    let mut y = icon_bottom + PANEL_TRAY_GAP * scale;

    // 托盘图标贴着屏幕右边缘，靠边的屏幕要夹回来
    if let Some(monitor) = &monitor {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        let (min_x, min_y) = (m_pos.x as f64, m_pos.y as f64);
        let max_x = min_x + m_size.width as f64 - win_w;
        let max_y = min_y + m_size.height as f64 - win_h;
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));
    }

    let _ = window.set_position(tauri::LogicalPosition::new(x / scale, y / scale));
}

fn open_control_panel(app: &tauri::AppHandle) {
    let Some(window) = ensure_control_panel(app) else {
        return;
    };
    let state = app.state::<AppState>();

    // 窗口只属于它第一次显示时所在的那个桌面，切到别的桌面后 show 出来还在原处
    let _ = window.set_visible_on_all_workspaces(true);

    // 每次都按当前托盘图标的位置重算，并强行走一遍 hide → show，
    // 让窗口重新排到最前面、位置改动立刻生效
    position_control_panel(app, &window);
    let _ = window.hide();
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    // 只把焦点交给面板，不动浮窗的编辑/穿透状态
    let _ = window.set_focus();

    *state.panel_open.lock().unwrap() = true;
    *state.panel_generation.lock().unwrap() += 1;
}

fn hide_control_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(CONTROL_PANEL_LABEL) {
        let _ = window.hide();
    }

    let state = app.state::<AppState>();
    *state.panel_keep_alive.lock().unwrap() = None;
    *state.panel_open.lock().unwrap() = false;
    *state.panel_generation.lock().unwrap() += 1;
}

fn toggle_control_panel(app: &tauri::AppHandle) {
    // 以自己的状态为准，而不是窗口的 is_visible：窗口在别的桌面上时
    // is_visible 同样是 true，会把「打开」误判成「收起」，点起来就像没反应
    let state = app.state::<AppState>();
    let open = *state.panel_open.lock().unwrap()
        && app
            .get_webview_window(CONTROL_PANEL_LABEL)
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);

    if open {
        hide_control_panel(app);
    } else {
        open_control_panel(app);
    }
}

/// 历史记录弹窗：复用同一个窗口实例，标题栏自带的关闭按钮关掉后下次重建
fn show_history_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(HISTORY_LABEL) {
        let _ = app.show();
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        let _ = window.set_focus();
        return;
    }

    if let Err(e) = WebviewWindowBuilder::new(
        app,
        HISTORY_LABEL,
        tauri::WebviewUrl::App("index.html#/history".into()),
    )
    .title("N-Top 历史记录")
    .inner_size(360.0, 480.0)
    .resizable(true)
    // 正文用同一套毛玻璃观感，标题栏保持系统绘制
    .transparent(true)
    .effects(
        EffectsBuilder::new()
            .effect(Effect::Popover)
            .state(EffectState::FollowsWindowActiveState)
            .build(),
    )
    .theme(Some(tauri::Theme::Light))
    .always_on_top(true)
    .visible(true)
    .build()
    {
        eprintln!("创建历史记录窗口失败: {}", e);
    }
}

fn keep_alive_active(app: &tauri::AppHandle) -> bool {
    let state = app.state::<AppState>();
    let guard = state.panel_keep_alive.lock().unwrap();
    guard.is_some_and(|deadline| Instant::now() < deadline)
}

/// 面板失焦后判断要不要自动收起。
/// 光标在浮窗上、或浮窗正在编辑（本应用窗口仍持有焦点）时保持展开。
fn spawn_control_panel_autohide(app: tauri::AppHandle) {
    let generation = *app.state::<AppState>().panel_generation.lock().unwrap();

    std::thread::spawn(move || {
        let started = Instant::now();

        loop {
            std::thread::sleep(PANEL_AUTOHIDE_DELAY);

            // 这期间面板被重新打开过，本次失焦作废，别把刚打开的面板收掉
            if *app.state::<AppState>().panel_generation.lock().unwrap() != generation {
                return;
            }

            let Some(window) = app.get_webview_window(CONTROL_PANEL_LABEL) else {
                return;
            };
            if !window.is_visible().unwrap_or(false) {
                return;
            }

            let app_focused = app
                .webview_windows()
                .values()
                .any(|w| w.is_focused().unwrap_or(false));
            if app_focused {
                return;
            }

            if keep_alive_active(&app) {
                continue;
            }

            let hovering_overlay = get_mouse_position()
                .is_some_and(|(mx, my)| is_point_in_overlay(&app, mx, my));
            if hovering_overlay && started.elapsed() < PANEL_AUTOHIDE_MAX_WAIT {
                continue;
            }

            hide_control_panel(&app);
            return;
        }
    });
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> AppSettings {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    settings
}

#[tauri::command]
fn save_settings_cmd(app: tauri::AppHandle, settings: AppSettings) {
    let state = app.state::<AppState>();
    let old_hotkey = state.settings.lock().unwrap().hotkey.clone();
    let hotkey_changed = old_hotkey != settings.hotkey;

    *state.settings.lock().unwrap() = settings.clone();
    save_settings(&app, &settings);
    save_history_snapshot(&app, &settings);

    // Re-register global hotkey if it changed
    if hotkey_changed {
        // Unregister all existing shortcuts
        let _ = app.global_shortcut().unregister_all();
        // Register new hotkey
        if let Err(e) = register_hotkey(&app, &settings.hotkey) {
            eprintln!("热键重新注册失败: {}", e);
        }
    }

    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("settings-updated", &settings);
    }
}

#[tauri::command]
fn toggle_overlay_mode(app: tauri::AppHandle) {
    toggle_mode(&app);
}

#[tauri::command]
fn set_overlay_mode(app: tauri::AppHandle, mode: String) {
    let m = if mode == "edit" {
        OverlayMode::Edit
    } else {
        OverlayMode::PassThrough
    };
    set_mode(&app, m);
}

#[tauri::command]
fn get_mode(app: tauri::AppHandle) -> String {
    let state = app.state::<AppState>();
    let m = *state.mode.lock().unwrap();
    if m == OverlayMode::Edit {
        "edit".to_string()
    } else {
        "pass-through".to_string()
    }
}

#[tauri::command]
fn is_mouse_in_overlay(app: tauri::AppHandle) -> bool {
    let Some((mx, my)) = get_mouse_position() else {
        return false;
    };
    is_point_in_overlay(&app, mx, my)
}

/// 由面板自己的 ✕ / Esc 触发收起
#[tauri::command]
fn hide_control_panel_cmd(app: tauri::AppHandle) {
    hide_control_panel(&app);
}

/// 面板里的「历史记录」，开在独立弹窗里
#[tauri::command]
fn open_history_window_cmd(app: tauri::AppHandle) {
    show_history_window(&app);
}

/// 面板里的「退出 N-Top」
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// 面板正在交互（如打开系统取色器）时，请自动收起逻辑再等一会儿
#[tauri::command]
fn keep_control_panel_open(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    *state.panel_keep_alive.lock().unwrap() = Some(Instant::now() + PANEL_KEEP_ALIVE);
}

#[derive(serde::Serialize)]
struct HistoryEntry {
    filename: String,
    timestamp: String,
    text: String,
}

#[tauri::command]
fn get_history(app: tauri::AppHandle) -> Vec<HistoryEntry> {
    let dir = history_dir(&app);
    cleanup_old_history(&app);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut entries: Vec<HistoryEntry> = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(&today) || !name.ends_with(".json") {
                continue;
            }
            // Extract timestamp from filename: YYYY-MM-DD_HH-MM-SS.json
            let ts = name.trim_end_matches(".json").to_string();
            // Parse display: "YYYY:MM:DD HH:MM:SS" → "HH:MM:SS"
            let time_part = ts.split('_').nth(1).unwrap_or("").replace("-", ":");
            let display = format!("{} {}", ts.split('_').next().unwrap_or(""), time_part);

            let text = std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
                .map(|s| s.text)
                .unwrap_or_default();

            entries.push(HistoryEntry {
                filename: name,
                timestamp: display,
                text,
            });
        }
    }

    // Sort by filename descending (newest first)
    entries.sort_by(|a, b| b.filename.cmp(&a.filename));
    entries
}

#[tauri::command]
fn set_dock_visibility(app: tauri::AppHandle, visible: bool) {
    use tauri::ActivationPolicy;
    let policy = if visible {
        ActivationPolicy::Regular
    } else {
        ActivationPolicy::Accessory
    };
    let _ = app.set_activation_policy(policy);

    // Persist to settings
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.show_in_dock = visible;
    save_settings(&app, &settings);
}

fn register_hotkey(app: &tauri::AppHandle, hotkey_str: &str) -> Result<(), String> {
    let shortcut: Shortcut = hotkey_str
        .parse()
        .map_err(|e| format!("解析热键失败: {}", e))?;

    app.global_shortcut()
        .on_shortcut(shortcut, move |app_handle, _sc, event| {
            if event.state == ShortcutState::Pressed {
                toggle_mode(app_handle);
            }
        })
        .map_err(|e| format!("注册热键失败: {}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("N-Top")
                .build(),
        )
        .on_window_event(|window, event| {
            // 失焦（点到别的 App，或点到浮窗上编辑）后延迟判断是否收起
            if window.label() == CONTROL_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                spawn_control_panel_autohide(window.app_handle().clone());
            }
        })
        .setup(|app| {
            let settings = load_settings(&app.handle());
            let hotkey = settings.hotkey.clone();

            app.manage(AppState {
                mode: Mutex::new(OverlayMode::PassThrough),
                settings: Mutex::new(settings),
                mouse_in_overlay: Mutex::new(false),
                panel_keep_alive: Mutex::new(None),
                panel_open: Mutex::new(false),
                panel_generation: Mutex::new(0),
            });

            // Set overlay window position from saved settings
            if let Some(window) = app.get_webview_window("overlay") {
                let state = app.state::<AppState>();
                let s = state.settings.lock().unwrap();
                let mut pos = tauri::PhysicalPosition::new(s.pos_x as i32, s.pos_y as i32);
                let size = window.outer_size().unwrap_or_default();
                if !is_window_visible_on_any_monitor(&app.handle(), pos, size) {
                    eprintln!(
                        "浮窗保存的位置 ({}, {}) 不在任何屏幕内，重置到主屏幕左上角",
                        pos.x, pos.y
                    );
                    pos = tauri::PhysicalPosition::new(100, 100);
                }
                let _ = window.set_position(tauri::Position::Physical(pos));
                let _ = window.set_ignore_cursor_events(true);

                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Moved(position) = event {
                        save_overlay_position(&app_handle, *position);
                    }
                });
            }

            // 不挂菜单：点图标就是开 / 关控制面板
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("N-Top 置顶文本浮层")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left | MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = tray.app_handle();
                        toggle_control_panel(app_handle);
                    }
                })
                .build(app)?;

            // Register global hotkey
            if let Err(e) = register_hotkey(&app.handle(), &hotkey) {
                eprintln!("热键注册警告: {} - 请在系统设置中授权辅助功能权限", e);
            }

            // Apply dock visibility from saved settings
            {
                use tauri::ActivationPolicy;
                let s = app.state::<AppState>();
                let show_dock = s.settings.lock().unwrap().show_in_dock;
                let policy = if show_dock {
                    ActivationPolicy::Regular
                } else {
                    ActivationPolicy::Accessory
                };
                let _ = app.handle().set_activation_policy(policy);
            }

            // Start mouse hover polling for pass-through transparency
            start_mouse_poll(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings_cmd,
            toggle_overlay_mode,
            set_overlay_mode,
            get_mode,
            is_mouse_in_overlay,
            hide_control_panel_cmd,
            keep_control_panel_open,
            open_history_window_cmd,
            quit_app,
            set_dock_visibility,
            get_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
