// Prevent a console window from opening alongside the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;

/// User-editable settings. Mirrors the original Java `defualtConfig.xml`,
/// stored as JSON in the OS config dir (e.g. ~/.config/Hourglass/config.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    /// Seconds of work between breaks (Java: buttonDisableDuration).
    work_seconds: u64,
    /// Seconds the "I'm done" button stays disabled = forced break length
    /// (Java: timerDuration).
    break_seconds: u64,
    msg_heading: String,
    msg_body: String,
    quit_button_msg: String,
    terminate_button_msg: String,
    dark_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work_seconds: 900, // 15 minutes, as in the Java defaults
            break_seconds: 30,
            msg_heading: "Up you go!".into(),
            msg_body: "Time to stretch".into(),
            quit_button_msg: "I'm Done stretching!".into(),
            terminate_button_msg: "Stop the timer".into(),
            dark_mode: true,
        }
    }
}

/// Shared state: the live config and a notifier the break loop waits on.
struct AppState {
    config: Mutex<Config>,
    resume: Arc<Notify>,
}

fn config_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir.join("config.json")
}

/// Read config from disk, creating it with defaults on first run.
fn read_or_init_config(app: &AppHandle) -> Config {
    let path = config_path(app);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
            let c = Config::default();
            let _ = fs::write(&path, serde_json::to_string_pretty(&c).unwrap());
            c
        }),
        Err(_) => {
            let c = Config::default();
            let _ = fs::write(&path, serde_json::to_string_pretty(&c).unwrap());
            c
        }
    }
}

#[tauri::command]
fn load_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<AppState>, config: Config) {
    let _ = fs::write(
        config_path(&app),
        serde_json::to_string_pretty(&config).unwrap(),
    );
    *state.config.lock().unwrap() = config;
}

/// Called when the user clicks "I'm done": hide the overlay and let the
/// work-period loop resume.
#[tauri::command]
fn break_done(app: AppHandle, state: State<AppState>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    state.resume.notify_one();
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let config = read_or_init_config(&handle);
            let resume = Arc::new(Notify::new());

            app.manage(AppState {
                config: Mutex::new(config),
                resume: resume.clone(),
            });

            // The break loop: replaces the Java TimerThread/PopupThread duo.
            let loop_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let work = loop_handle
                        .state::<AppState>()
                        .config
                        .lock()
                        .unwrap()
                        .work_seconds;

                    tokio::time::sleep(Duration::from_secs(work)).await;

                    if let Some(win) = loop_handle.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.set_always_on_top(true);
                    }
                    let _ = loop_handle.emit("break-start", ());

                    // Wait until the user dismisses the break.
                    resume.notified().await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            break_done,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hourglass");
}
