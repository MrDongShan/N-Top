use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, UserAttentionType, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

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

fn show_control_panel(app: &tauri::AppHandle) {
    // 打开控制窗口时浮窗默认进入编辑态，保证面板按钮与实际状态一致
    set_mode(app, OverlayMode::Edit);

    if let Some(window) = app.get_webview_window("control-panel") {
        let _ = app.show();
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        let _ = window.request_user_attention(Some(UserAttentionType::Critical));
        let _ = window.set_focus();
    } else {
        match WebviewWindowBuilder::new(
            app,
            "control-panel",
            tauri::WebviewUrl::App("index.html#/control-panel".into()),
        )
        .title("N-Top 控制面板")
        .inner_size(320.0, 840.0)
        .min_inner_size(320.0, 840.0)
        .resizable(true)
        .decorations(true)
        .always_on_top(true)
        .visible(true)
        .build()
        {
            Ok(window) => {
                let _ = window.set_focus();
            }
            Err(e) => eprintln!("创建控制面板失败: {}", e),
        }
    }
}

fn show_history_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("history") {
        let _ = app.show();
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        let _ = window.request_user_attention(Some(UserAttentionType::Critical));
        let _ = window.set_focus();
    } else {
        let _ = WebviewWindowBuilder::new(
            app,
            "history",
            tauri::WebviewUrl::App("index.html#/history".into()),
        )
        .title("N-Top 历史记录")
        .inner_size(360.0, 480.0)
        .resizable(true)
        .decorations(true)
        .always_on_top(true)
        .visible(true)
        .build();
    }
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
            // 控制窗口关闭后，浮窗回到穿透态
            if window.label() == "control-panel"
                && matches!(event, WindowEvent::CloseRequested { .. })
            {
                set_mode(window.app_handle(), OverlayMode::PassThrough);
            }
        })
        .setup(|app| {
            let settings = load_settings(&app.handle());
            let hotkey = settings.hotkey.clone();

            app.manage(AppState {
                mode: Mutex::new(OverlayMode::PassThrough),
                settings: Mutex::new(settings),
                mouse_in_overlay: Mutex::new(false),
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

            // Build tray menu
            let toggle_item =
                MenuItem::with_id(app, "toggle", "切换编辑/穿透", true, None::<&str>)?;
            let panel_item = MenuItem::with_id(app, "panel", "打开控制面板", true, None::<&str>)?;
            let history_item = MenuItem::with_id(app, "history", "历史记录", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出 N-Top", true, None::<&str>)?;
            let menu =
                Menu::with_items(app, &[&toggle_item, &panel_item, &history_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("N-Top 置顶文本浮层")
                .on_menu_event(move |app_handle, event| match event.id.as_ref() {
                    "toggle" => toggle_mode(app_handle),
                    "panel" => show_control_panel(app_handle),
                    "history" => show_history_window(app_handle),
                    "quit" => {
                        app_handle.exit(0);
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
                        let app_handle = tray.app_handle();
                        show_control_panel(app_handle);
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
            set_dock_visibility,
            get_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
