// Prevent a console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;

/// User-editable settings, stored as JSON in the OS config dir
/// (e.g. ~/.config/com.guru227.hourglass/config.json). `#[serde(default)]`
/// lets older/partial config files load — missing fields fall back to default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    /// Seconds of work between breaks.
    work_seconds: u64,
    /// Forced break length = seconds the "I'm done" button stays disabled.
    break_seconds: u64,
    msg_heading: String,
    msg_body: String,
    quit_button_msg: String,
    terminate_button_msg: String,
    /// "crt" | "dark" | "light"
    theme: String,
    /// "both" | "factoids" | "quotes"
    content_mode: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work_seconds: 900,
            break_seconds: 30,
            msg_heading: "Up you go!".into(),
            msg_body: "Time to stretch".into(),
            quit_button_msg: "I'm Done stretching!".into(),
            terminate_button_msg: "Stop the timer".into(),
            theme: "crt".into(),
            content_mode: "both".into(),
        }
    }
}

/// Shared state for the break loop and commands.
struct AppState {
    config: Mutex<Config>,
    resume: Arc<Notify>,       // break was dismissed
    break_now: Arc<Notify>,    // user asked for an immediate break
    resume_pause: Arc<Notify>, // unpause signal
    paused: Arc<AtomicBool>,
    in_break: Arc<AtomicBool>,
}

fn config_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir.join("config.json")
}

fn read_or_init_config(app: &AppHandle) -> Config {
    let path = config_path(app);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => {
            let c = Config::default();
            let _ = fs::write(&path, serde_json::to_string_pretty(&c).unwrap());
            c
        }
    }
}

/// Show the break overlay as a genuine, sticky, always-on-top fullscreen window.
/// Setting the monitor size/position explicitly is the reliable path on Wayland,
/// where `fullscreen` alone can degrade to a tiny window at the origin.
async fn show_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        // Show (map) first, then let the window realize before applying state.
        // The very first state call after show() otherwise loses a race and is
        // dropped — observed: fullscreen failed while later above/sticky stuck.
        let _ = win.show();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = win.set_fullscreen(true);
        let _ = win.set_always_on_top(true);
        let _ = win.set_visible_on_all_workspaces(true);
        let _ = win.set_focus();
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
    let _ = app.emit("config-updated", ()); // live-apply in the overlay
}

#[tauri::command]
fn break_done(app: AppHandle, state: State<AppState>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    state.in_break.store(false, Ordering::SeqCst);
    state.resume.notify_one();
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn close_settings(app: AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings_i = MenuItemBuilder::with_id("settings", "Settings…").build(app)?;
    let break_i = MenuItemBuilder::with_id("break", "Take a break now").build(app)?;
    let pause_i = MenuItemBuilder::with_id("pause", "Pause / Resume").build(app)?;
    let quit_i = MenuItemBuilder::with_id("quit", "Quit Hourglass").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&settings_i)
        .item(&break_i)
        .item(&pause_i)
        .item(&quit_i)
        .build()?;

    let mut builder = TrayIconBuilder::with_id("tray")
        .tooltip("Hourglass")
        .menu(&menu);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id().as_ref() {
                "settings" => {
                    if let Some(win) = app.get_webview_window("settings") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                "break" => {
                    if !state.in_break.load(Ordering::SeqCst) {
                        state.break_now.notify_one();
                    }
                }
                "pause" => {
                    let now = !state.paused.load(Ordering::SeqCst);
                    state.paused.store(now, Ordering::SeqCst);
                    if !now {
                        state.resume_pause.notify_one();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    // Run through XWayland on GNOME/Wayland. Native Wayland forbids clients from
    // self-positioning, forcing always-on-top, or sticking a window to all
    // workspaces; under X11 (XWayland) all three work. Must happen before any
    // GTK init. Respects an explicit user GDK_BACKEND and only kicks in when
    // XWayland (DISPLAY) is actually available.
    #[cfg(target_os = "linux")]
    {
        let on_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|v| v.contains("wayland"))
                .unwrap_or(false);
        let xwayland_present = std::env::var("DISPLAY").is_ok();
        if on_wayland && xwayland_present && std::env::var("GDK_BACKEND").is_err() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let config = read_or_init_config(&handle);

            let resume = Arc::new(Notify::new());
            let break_now = Arc::new(Notify::new());
            let resume_pause = Arc::new(Notify::new());
            let paused = Arc::new(AtomicBool::new(false));
            let in_break = Arc::new(AtomicBool::new(false));

            app.manage(AppState {
                config: Mutex::new(config),
                resume: resume.clone(),
                break_now: break_now.clone(),
                resume_pause: resume_pause.clone(),
                paused: paused.clone(),
                in_break: in_break.clone(),
            });

            build_tray(&handle)?;

            // The break loop: sleep the work period (interruptible by "break now"
            // and "pause"), show the overlay, wait until dismissed, repeat.
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    if paused.load(Ordering::SeqCst) {
                        resume_pause.notified().await;
                        continue;
                    }
                    let work = h.state::<AppState>().config.lock().unwrap().work_seconds;

                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(work)) => {}
                        _ = break_now.notified() => {}
                    }

                    if paused.load(Ordering::SeqCst) {
                        continue;
                    }

                    in_break.store(true, Ordering::SeqCst);
                    show_overlay(&h).await;
                    let _ = h.emit("break-start", ());
                    resume.notified().await;
                    in_break.store(false, Ordering::SeqCst);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            break_done,
            open_settings,
            close_settings,
            quit_app
        ])
        .build(tauri::generate_context!())
        .expect("error while building Hourglass")
        .run(|_app, event| {
            // Keep Hourglass alive in the tray even with no visible windows.
            // Explicit app.exit() (tray "Quit" / "Stop the timer") still exits.
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
