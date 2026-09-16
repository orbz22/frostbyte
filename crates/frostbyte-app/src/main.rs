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
    menu::{
        CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem,
        SubmenuBuilder,
    },
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
            .args([
                "add",
                REG_RUN_KEY,
                "/v",
                APP_REG_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &cmd,
                "/f",
            ])
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
#[serde(default)]
pub struct AppConfig {
    pub auto_tame: bool,
    pub cool_mode: bool,
    pub auto_cool: bool,
    pub autostart: bool,
    pub close_to_tray: bool,
    pub saturation_threshold: f32,
    pub auto_cool_temp_threshold: f32,
    pub cpu_temp_monitoring: bool,
    pub gpu_temp_monitoring: bool,
    pub gpu_monitoring: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_tame: false,
            cool_mode: false,
            auto_cool: true,
            autostart: false,
            close_to_tray: true,
            saturation_threshold: 80.0,
            auto_cool_temp_threshold: 88.0,
            cpu_temp_monitoring: true,
            gpu_temp_monitoring: true,
            gpu_monitoring: true,
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
    let default_cfg = AppConfig {
        autostart: is_autostart_registered(),
        ..Default::default()
    };
    save_config(&default_cfg);
    default_cfg
}

fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = fs::write(path, content);
    }
}

#[derive(Clone)]
pub struct TrayMenuItems {
    pub cool_on: CheckMenuItem<tauri::Wry>,
    pub cool_off: CheckMenuItem<tauri::Wry>,
    pub tame_on: CheckMenuItem<tauri::Wry>,
    pub tame_off: CheckMenuItem<tauri::Wry>,
}

struct AppState {
    watchdog: Arc<Mutex<Watchdog>>,
    latest_snapshot: Arc<Mutex<Option<SystemSnapshot>>>,
    config: Arc<Mutex<AppConfig>>,
    tray_items: Arc<Mutex<Option<TrayMenuItems>>>,
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Option<SystemSnapshot> {
    state.latest_snapshot.lock().clone()
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.lock().clone()
}

#[tauri::command]
fn update_config(state: State<'_, AppState>, new_config: AppConfig) -> Result<AppConfig, String> {
    if new_config.autostart != is_autostart_registered() {
        set_autostart_registry(new_config.autostart)?;
    }

    {
        let mut watchdog = state.watchdog.lock();
        let _ = watchdog.set_turbo_boost(!new_config.cool_mode);
        watchdog.set_auto_tame(new_config.auto_tame);
        watchdog.set_saturation_threshold(new_config.saturation_threshold);
        watchdog.set_auto_cool(new_config.auto_cool);
        watchdog.set_auto_cool_threshold(new_config.auto_cool_temp_threshold);
        watchdog.set_cpu_temp_monitoring(new_config.cpu_temp_monitoring);
        watchdog.set_gpu_temp_monitoring(new_config.gpu_temp_monitoring);
        watchdog.set_gpu_monitoring(new_config.gpu_monitoring);
    }

    if let Some(items) = state.tray_items.lock().clone() {
        let _ = items.cool_on.set_checked(new_config.cool_mode);
        let _ = items.cool_off.set_checked(!new_config.cool_mode);
        let _ = items.tame_on.set_checked(new_config.auto_tame);
        let _ = items.tame_off.set_checked(!new_config.auto_tame);
    }

    let mut cfg = state.config.lock();
    *cfg = new_config.clone();
    save_config(&cfg);

    Ok(new_config)
}

#[tauri::command]
fn set_auto_cool(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_auto_cool(enabled);

    let mut cfg = state.config.lock();
    cfg.auto_cool = enabled;
    save_config(&cfg);

    enabled
}

#[tauri::command]
fn set_cpu_temp_monitoring(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_cpu_temp_monitoring(enabled);

    let mut cfg = state.config.lock();
    cfg.cpu_temp_monitoring = enabled;
    save_config(&cfg);

    enabled
}

#[tauri::command]
fn set_gpu_temp_monitoring(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_gpu_temp_monitoring(enabled);

    let mut cfg = state.config.lock();
    cfg.gpu_temp_monitoring = enabled;
    save_config(&cfg);

    enabled
}

#[tauri::command]
fn set_gpu_monitoring(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_gpu_monitoring(enabled);

    let mut cfg = state.config.lock();
    cfg.gpu_monitoring = enabled;
    save_config(&cfg);

    enabled
}

#[tauri::command]
fn set_cool_mode(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let mut watchdog = state.watchdog.lock();
    // false to enable cool mode (99%), true to enable boost (100%)
    watchdog
        .set_turbo_boost(!enabled)
        .map_err(|e| e.to_string())?;

    if let Some(items) = state.tray_items.lock().clone() {
        let _ = items.cool_on.set_checked(enabled);
        let _ = items.cool_off.set_checked(!enabled);
    }

    let mut cfg = state.config.lock();
    cfg.cool_mode = enabled;
    save_config(&cfg);

    Ok(enabled)
}

#[tauri::command]
fn set_auto_tame(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_auto_tame(enabled);

    if let Some(items) = state.tray_items.lock().clone() {
        let _ = items.tame_on.set_checked(enabled);
        let _ = items.tame_off.set_checked(!enabled);
    }

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
fn set_close_to_tray(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut cfg = state.config.lock();
    cfg.close_to_tray = enabled;
    save_config(&cfg);
    enabled
}

#[tauri::command]
fn soft_tame_process(
    state: State<'_, AppState>,
    pid: u32,
    cpu_limit: Option<u32>,
) -> Result<(), String> {
    let mut watchdog = state.watchdog.lock();
    watchdog
        .soft_tame_process(pid, cpu_limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn terminate_process(
    state: State<'_, AppState>,
    pid: u32,
    process_name: String,
) -> Result<(), String> {
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
    let mut initial_wd = Watchdog::with_full_settings(
        config.saturation_threshold,
        3,
        2,
        config.auto_tame,
        config.auto_cool,
        config.auto_cool_temp_threshold,
    );
    initial_wd.set_cpu_temp_monitoring(config.cpu_temp_monitoring);
    initial_wd.set_gpu_temp_monitoring(config.gpu_temp_monitoring);
    initial_wd.set_gpu_monitoring(config.gpu_monitoring);
    if config.cool_mode {
        let _ = initial_wd.set_turbo_boost(false);
    }
    // Populate the snapshot immediately with real system data on startup
    let initial_snapshot = initial_wd.tick();

    let watchdog = Arc::new(Mutex::new(initial_wd));
    let latest_snapshot = Arc::new(Mutex::new(Some(initial_snapshot)));
    let config_arc = Arc::new(Mutex::new(config));
    let tray_items_arc: Arc<Mutex<Option<TrayMenuItems>>> = Arc::new(Mutex::new(None));

    let watchdog_clone = Arc::clone(&watchdog);
    let snapshot_clone = Arc::clone(&latest_snapshot);
    let tray_items_clone = Arc::clone(&tray_items_arc);

    tauri::Builder::default()
        .manage(AppState {
            watchdog: Arc::clone(&watchdog),
            latest_snapshot: Arc::clone(&latest_snapshot),
            config: Arc::clone(&config_arc),
            tray_items: Arc::clone(&tray_items_arc),
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.app_handle().state::<AppState>();
                let close_to_tray = state.config.lock().close_to_tray;
                if close_to_tray {
                    // Minimize/hide to the system tray rather than killing the guardian
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    // Full exit requested: clean up any active throttles and exit
                    let mut wd = state.watchdog.lock();
                    let _ = wd.revert_all_tames();
                    window.app_handle().exit(0);
                }
            }
        })
        .setup(move |app| {
            // Check if started with --minimized (e.g. from Windows startup)
            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            let initial_cool = config_arc.lock().cool_mode;
            let initial_tame = config_arc.lock().auto_tame;

            // Build Single Tray Menu with cascading Submenus & real checkmarks (✓)
            let toggle_show =
                MenuItemBuilder::with_id("show", "Show FrostByte Dashboard").build(app)?;
            let sep1 = PredefinedMenuItem::separator(app)?;

            // Submenu 1: Instant Cool (Flyout with ✓ Checkmarks)
            let cool_on = CheckMenuItemBuilder::with_id("cool_on", "Turn ON (99% Cap - Cool)")
                .checked(initial_cool)
                .build(app)?;
            let cool_off =
                CheckMenuItemBuilder::with_id("cool_off", "Turn OFF (100% Boost - Normal)")
                    .checked(!initial_cool)
                    .build(app)?;
            let cool_submenu = SubmenuBuilder::new(app, "❄️ Instant Cool")
                .items(&[&cool_on, &cool_off])
                .build()?;

            // Submenu 2: Auto-Tame (Flyout with ✓ Checkmarks)
            let tame_on = CheckMenuItemBuilder::with_id("tame_on", "Turn ON (10% CPU Hard Cap)")
                .checked(initial_tame)
                .build(app)?;
            let tame_off = CheckMenuItemBuilder::with_id("tame_off", "Turn OFF (Disabled)")
                .checked(!initial_tame)
                .build(app)?;
            let tame_sep = PredefinedMenuItem::separator(app)?;
            let revert_all_item =
                MenuItemBuilder::with_id("revert", "🛡️ Revert All Tamed Processes").build(app)?;
            let tame_submenu = SubmenuBuilder::new(app, "🛡️ Auto-Tame")
                .items(&[&tame_on, &tame_off, &tame_sep, &revert_all_item])
                .build()?;

            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Exit FrostByte").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[
                    &toggle_show,
                    &sep1,
                    &cool_submenu,
                    &tame_submenu,
                    &sep2,
                    &quit,
                ])
                .build()?;

            // Store references to check menu items in state for live synchronization
            *app.state::<AppState>().tray_items.lock() = Some(TrayMenuItems {
                cool_on: cool_on.clone(),
                cool_off: cool_off.clone(),
                tame_on: tame_on.clone(),
                tame_off: tame_off.clone(),
            });

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
                    "cool_on" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.set_turbo_boost(false);
                        let mut cfg = state.config.lock();
                        cfg.cool_mode = true;
                        save_config(&cfg);
                        let maybe_items = state.tray_items.lock().clone();
                        if let Some(items) = maybe_items {
                            let _ = items.cool_on.set_checked(true);
                            let _ = items.cool_off.set_checked(false);
                        }
                    }
                    "cool_off" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.set_turbo_boost(true);
                        let mut cfg = state.config.lock();
                        cfg.cool_mode = false;
                        save_config(&cfg);
                        let maybe_items = state.tray_items.lock().clone();
                        if let Some(items) = maybe_items {
                            let _ = items.cool_on.set_checked(false);
                            let _ = items.cool_off.set_checked(true);
                        }
                    }
                    "tame_on" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        wd.set_auto_tame(true);
                        let mut cfg = state.config.lock();
                        cfg.auto_tame = true;
                        save_config(&cfg);
                        let maybe_items = state.tray_items.lock().clone();
                        if let Some(items) = maybe_items {
                            let _ = items.tame_on.set_checked(true);
                            let _ = items.tame_off.set_checked(false);
                        }
                    }
                    "tame_off" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        wd.set_auto_tame(false);
                        let mut cfg = state.config.lock();
                        cfg.auto_tame = false;
                        save_config(&cfg);
                        let maybe_items = state.tray_items.lock().clone();
                        if let Some(items) = maybe_items {
                            let _ = items.tame_on.set_checked(false);
                            let _ = items.tame_off.set_checked(true);
                        }
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

                    // Live sync tray menu checkmarks when thermal governor intervenes or states change
                    if let Some(items) = tray_items_clone.lock().as_ref() {
                        let _ = items.cool_on.set_checked(snapshot.is_turbo_boost_clamped);
                        let _ = items.cool_off.set_checked(!snapshot.is_turbo_boost_clamped);
                        let _ = items.tame_on.set_checked(snapshot.auto_tame_enabled);
                        let _ = items.tame_off.set_checked(!snapshot.auto_tame_enabled);
                    }

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
            get_config,
            update_config,
            set_cool_mode,
            set_auto_cool,
            set_auto_tame,
            set_cpu_temp_monitoring,
            set_gpu_temp_monitoring,
            set_gpu_monitoring,
            get_autostart,
            set_autostart,
            set_close_to_tray,
            soft_tame_process,
            terminate_process,
            revert_process,
            revert_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
