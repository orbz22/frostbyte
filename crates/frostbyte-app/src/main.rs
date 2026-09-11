// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use frostbyte_core::{SystemSnapshot, Watchdog};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State,
};

struct AppState {
    watchdog: Arc<Mutex<Watchdog>>,
    latest_snapshot: Arc<Mutex<Option<SystemSnapshot>>>,
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
        .map(|_| enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_auto_tame(state: State<'_, AppState>, enabled: bool) -> bool {
    let mut watchdog = state.watchdog.lock();
    watchdog.set_auto_tame(enabled);
    enabled
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
    let watchdog = Arc::new(Mutex::new(Watchdog::with_settings(80.0, 3, 2, false)));
    let latest_snapshot = Arc::new(Mutex::new(None));

    let watchdog_clone = Arc::clone(&watchdog);
    let snapshot_clone = Arc::clone(&latest_snapshot);

    tauri::Builder::default()
        .manage(AppState {
            watchdog: Arc::clone(&watchdog),
            latest_snapshot: Arc::clone(&latest_snapshot),
        })
        .setup(move |app| {
            // Build Tray Menu
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
                    }
                    "boost" => {
                        let state = app.state::<AppState>();
                        let mut wd = state.watchdog.lock();
                        let _ = wd.set_turbo_boost(true);
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

            // Background Telemetry Loop (every 2 seconds)
            let handle_for_loop = app_handle.clone();
            std::thread::spawn(move || loop {
                let snapshot = {
                    let mut wd = watchdog_clone.lock();
                    wd.tick()
                };

                *snapshot_clone.lock() = Some(snapshot.clone());

                // Emit event to frontend if window is open
                let _ = handle_for_loop.emit("snapshot-update", &snapshot);

                std::thread::sleep(Duration::from_secs(2));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_cool_mode,
            set_auto_tame,
            soft_tame_process,
            terminate_process,
            revert_process,
            revert_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
