use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// App state: edit mode vs pass-through mode
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OverlayMode {
    Edit,
    PassThrough,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct AppSettings {
    pub text: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub bg_alpha: f64,
    pub font_size: u32,
    pub font_color: String,
    pub hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            text: String::from("在这里写你的提醒…"),
            pos_x: 100.0,
            pos_y: 100.0,
            bg_alpha: 0.3,
            font_size: 16,
            font_color: String::from("#ffffff"),
            hotkey: String::from("Alt+Command+E"),
        }
    }
}

pub struct AppState {
    pub mode: Mutex<OverlayMode>,
    pub settings: Mutex<AppSettings>,
}

fn settings_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let dir = app.path().app_data_dir().unwrap_or_else(|_| {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".n-top")
    });
    let _ = std::fs::create_dir_all(&dir);
    dir.join("content.json")
}

fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let path = settings_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(app: &tauri::AppHandle, settings: &AppSettings) {
    let path = settings_path(app);
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, json);
    }
}

/// Switch overlay between Edit and PassThrough mode.
fn set_mode(app: &tauri::AppHandle, mode: OverlayMode) {
    let state = app.state::<AppState>();
    *state.mode.lock().unwrap() = mode;

    if let Some(window) = app.get_webview_window("overlay") {
        match mode {
            OverlayMode::Edit => {
                let _ = window.set_ignore_cursor_events(false);
                let _ = window.set_focus();
                let _ = window.emit("mode-changed", "edit");
            }
            OverlayMode::PassThrough => {
                let _ = window.set_ignore_cursor_events(true);
                let _ = window.emit("mode-changed", "pass-through");
            }
        }
    }
}

/// Toggle between edit and pass-through.
fn toggle_mode(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let current = *state.mode.lock().unwrap();
    let next = match current {
        OverlayMode::Edit => OverlayMode::PassThrough,
        OverlayMode::PassThrough => OverlayMode::Edit,
    };
    set_mode(app, next);
}

/// Show the control panel window (create if not exists).
fn show_control_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("control-panel") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _ = WebviewWindowBuilder::new(
            app,
            "control-panel",
            tauri::WebviewUrl::App("index.html#/control-panel".into()),
        )
        .title("N-Top 控制面板")
        .inner_size(300.0, 360.0)
        .resizable(false)
        .decorations(true)
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
    *state.settings.lock().unwrap() = settings.clone();
    save_settings(&app, &settings);

    // Apply visual settings to overlay
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
        .setup(|app| {
            // Load settings
            let settings = load_settings(&app.handle());
            let hotkey = settings.hotkey.clone();

            app.manage(AppState {
                mode: Mutex::new(OverlayMode::PassThrough),
                settings: Mutex::new(settings),
            });

            // Set overlay window position from saved settings
            if let Some(window) = app.get_webview_window("overlay") {
                let state = app.state::<AppState>();
                let s = state.settings.lock().unwrap();
                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(s.pos_x as i32, s.pos_y as i32),
                ));
                // Start in pass-through mode
                let _ = window.set_ignore_cursor_events(true);
            }

            // Build tray menu
            let toggle_item = MenuItem::with_id(app, "toggle", "切换编辑/穿透", true, None::<&str>)?;
            let panel_item = MenuItem::with_id(app, "panel", "打开控制面板", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出 N-Top", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_item, &panel_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("N-Top 置顶文本浮层")
                .on_menu_event(move |app_handle, event| match event.id.as_ref() {
                    "toggle" => toggle_mode(app_handle),
                    "panel" => show_control_panel(app_handle),
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
                eprintln!("热键注册警告: {} — 请在系统设置中授权辅助功能权限", e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings_cmd,
            toggle_overlay_mode,
            set_overlay_mode,
            get_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
