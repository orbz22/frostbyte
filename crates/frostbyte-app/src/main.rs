// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use frostbyte_core::{SystemSnapshot, Watchdog};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const REG_RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const APP_REG_NAME: &str = "FrostByte";

fn is_autostart_registered() -> bool {
    let output = Command::new("reg")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["query", REG_RUN_KEY, "/v", APP_REG_NAME])
        .output();

    if let Ok(output) = output {
        output.status.success()
    } else {
        false
    }
}

fn set_autostart_registry(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let cmd = format!("\"{}\" --minimized", exe_path.display());
        let output = Command::new("reg")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["add", REG_RUN_KEY, "/v", APP_REG_NAME, "/t", "REG_SZ", "/d", &cmd, "/f"])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    } else {
        let _ = Command::new("reg")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["delete", REG_RUN_KEY, "/v", APP_REG_NAME, "/f"])
            .output();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub auto_tame: bool,
    pub cool_mode: bool,
    pub autostart: bool,
    pub saturation_threshold: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_tame: false,
            cool_mode: false,
            autostart: false,
            saturation_threshold: 80.0,
        }
    }
}

fn get_config_path() -> PathBuf {
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        let dir = PathBuf::from(local_appdata).join("FrostByte");
        let _ = fs::create_dir_all(&dir);
        dir.join("config.json")
    } else {
        PathBuf::from("frostbyte_config.json")
    }
}

fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(mut config) = serde_json::from_str::<AppConfig>(&content) {
            // Re-sync with actual registry state
            config.autostart = is_autostart_registered();
            return config;
        }
    }
    let mut default_cfg = AppConfig::default();
    default_cfg.autostart = is_autostart_registered();
    save_config(&default_cfg);
    default_cfg
}

fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = fs::write(path, content);
    }
}

struct AppState {
    watchdog: Arc<Mutex<Watchdog>>,
    latest_snapshot: Arc<Mutex<Option<SystemSnapshot>>>,
    config: Arc<Mutex<AppConfig>>,
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Option<SystemSnapshot> {
    state.latest_snapshot.lock().clone()
}

#[tauri::command]
fn set_cool_mode(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let mut watchdog = state.watchdog.lock();
    // false to enable cool mode (99%), true to enable boost (100%)
    watchdog
        .set_turbo_boost(!enabled)
        .map_err(|e| e.to_string())?;

    let mut cfg = state.config.lock();
    cfg.cool_mode = enabled;
    save_config(&cfg);

    Ok(enabled)
}

#[tauri::command]
fn set_auto_tame(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_auto_tame(enabled);

    let mut cfg = state.config.lock();
    cfg.auto_tame = enabled;
    save_config(&cfg);

    enabled
}

#[tauri::command]
fn get_autostart() -> bool {
    is_autostart_registered()
}

#[tauri::command]
fn set_autostart(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    set_autostart_registry(enabled)?;

    let mut cfg = state.config.lock();
    cfg.autostart = enabled;
    save_config(&cfg);

    Ok(enabled)
}

#[tauri::command]
fn soft_tame_process(state: State<'_, AppState>, pid: u32, cpu_limit: Option<u32>) -> Result<(), String> {
    let mut watchdog = state.watchdog.lock();
    watchdog
        .soft_tame_process(pid, cpu_limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn terminate_process(state: State<'_, AppState>, pid: u32, process_name: String) -> Result<(), String> {
    let mut watchdog = state.watchdog.lock();
    watchdog
        .terminate_process(pid, &process_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn revert_process(state: State<'_, AppState>, pid: u32) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.revert_tame(pid)
}

#[tauri::command]
fn revert_all(state: State<'_, AppState>) -> usize {
    let mut watchdog = state.watchdog.lock();
    watchdog.revert_all_tames()
}

fn main() {
    let config = load_config();
    let mut initial_wd = Watchdog::with_settings(config.saturation_threshold, 3, 2, config.auto_tame);
    if config.cool_mode {
        let _ = initial_wd.set_turbo_boost(false);
    }
    // Populate the snapshot immediately with real system data on startup
    let initial_snapshot = initial_wd.tick();

    let watchdog = Arc::new(Mutex::new(initial_wd));
    let latest_snapshot = Arc::new(Mutex::new(Some(initial_snapshot)));
    let config_arc = Arc::new(Mutex::new(config));

    let watchdog_clone = Arc::clone(&watchdog);
    let snapshot_clone = Arc::clone(&latest_snapshot);

    tauri::Builder::default()
        .manage(AppState {
            watchdog: Arc::clone(&watchdog),
            latest_snapshot: Arc::clone(&latest_snapshot),
            config: Arc::clone(&config_arc),
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Clicking 'X' minimizes/hides to the system tray rather than killing the guardian
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            // Check if started with --minimized (e.g. from Windows startup)
            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Build Single Tray Menu & Icon
            let toggle_show = MenuItemBuilder::with_id("show", "Show FrostByte Dashboard").build(app)?;
            let sep1 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let cool_mode = MenuItemBuilder::with_id("cool", "❄️ Instant Cool Down (99% Boost Clamp)").build(app)?;
            let boost_mode = MenuItemBuilder::with_id("boost", "⚡ Restore Performance (100% Boost)").build(app)?;
            let revert_all_item = MenuItemBuilder::with_id("revert", "🛡️ Revert All Tamed Processes").build(app)?;
            let sep2 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Exit FrostByte").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&toggle_show, &sep1, &cool_mode, &boost_mode, &revert_all_item, &sep2, &quit])
                .build()?;

            let app_handle = app.handle().clone();

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("FrostByte Thermal Guardian")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "cool" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.set_turbo_boost(false);
                        let mut cfg = state.config.lock();
                        cfg.cool_mode = true;
                        save_config(&cfg);
                    }
                    "boost" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.set_turbo_boost(true);
                        let mut cfg = state.config.lock();
                        cfg.cool_mode = false;
                        save_config(&cfg);
                    }
                    "revert" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.revert_all_tames();
                    }
                    "quit" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.revert_all_tames();
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
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Background Telemetry Loop (every 1.5 seconds)
            let handle_for_loop = app_handle.clone();
            std::thread::spawn(move || {
                // Short initial delay so the second tick has an accurate CPU delta
                std::thread::sleep(Duration::from_millis(600));

                loop {
                    let snapshot = {
                        let mut wd = watchdog_clone.lock();
                        wd.tick()
                    };

                    *snapshot_clone.lock() = Some(snapshot.clone());

                    // Emit event to frontend if window is open
                    let _ = handle_for_loop.emit("snapshot-update", &snapshot);

                    std::thread::sleep(Duration::from_millis(1500));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_cool_mode,
            set_auto_tame,
            get_autostart,
            set_autostart,
            soft_tame_process,
            terminate_process,
            revert_process,
            revert_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
