// Prevent a console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, Wry};
use tokio::sync::Notify;

/// User-editable settings, stored as JSON in the OS config dir
/// (e.g. ~/.config/com.guru227.hourglass/config.json). `#[serde(default)]`
/// lets older/partial config files load — missing fields fall back to default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    /// Seconds of work between breaks (simple mode).
    work_seconds: u64,
    /// Forced break length = seconds the "I'm done" button stays disabled (simple mode).
    break_seconds: u64,
    msg_heading: String,
    msg_body: String,
    quit_button_msg: String,
    /// "crt" | "dark" | "light"
    theme: String,
    /// "both" | "factoids" | "quotes"
    content_mode: String,
    /// "simple" | "pomodoro"
    mode: String,
    pomodoro_work_seconds: u64,
    pomodoro_short_break_seconds: u64,
    pomodoro_long_break_seconds: u64,
    /// Work cycles between long breaks.
    pomodoro_cycles: u64,
    /// Seconds of no keyboard/mouse input before the screen-time tracker
    /// treats the user as away.
    idle_threshold_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work_seconds: 900,
            break_seconds: 30,
            msg_heading: "Up you go!".into(),
            msg_body: "Time to stretch".into(),
            quit_button_msg: "I'm Done stretching!".into(),
            theme: "crt".into(),
            content_mode: "both".into(),
            mode: "simple".into(),
            pomodoro_work_seconds: 1500,
            pomodoro_short_break_seconds: 300,
            pomodoro_long_break_seconds: 900,
            pomodoro_cycles: 4,
            idle_threshold_seconds: 60,
        }
    }
}

/// Daily tallies — screen time (X11 idle-detected) + completed pomodoros.
/// Stored as JSON alongside config.json; rolled over whenever the local date
/// on disk no longer matches today.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Stats {
    /// Local YYYY-MM-DD this tally belongs to.
    date: String,
    screen_active_seconds: u64,
    pomodoros_completed: u64,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            date: today_str(),
            screen_active_seconds: 0,
            pomodoros_completed: 0,
        }
    }
}

/// A phase transition — pushed to both windows. The overlay reacts to
/// non-"work" phases (shows itself); the settings window uses every phase to
/// drive its live countdown + session dots. `started_at_ms` + `duration_seconds`
/// let the frontend tick its own countdown locally rather than us pushing a
/// heartbeat over IPC every second.
#[derive(Debug, Clone, Serialize)]
struct PhasePayload {
    mode: String,
    /// "work" | "break" | "short_break" | "long_break"
    phase: String,
    duration_seconds: u64,
    started_at_ms: u64,
    /// 1-based position within the current pomodoro set (always 1 in simple mode).
    cycle: u64,
    cycles_before_long_break: u64,
}

/// Shared state for the break loop and commands.
struct AppState {
    config: Mutex<Config>,
    stats: Mutex<Stats>,
    resume: Arc<Notify>,       // break was dismissed
    break_now: Arc<Notify>,    // user asked for an immediate break
    resume_pause: Arc<Notify>, // unpause signal
    paused: Arc<AtomicBool>,
    in_break: Arc<AtomicBool>,
    // Tray menu handles, filled in after the tray is built, so pause state /
    // screen time can be reflected live in the menu.
    pause_item: Mutex<Option<MenuItem<Wry>>>,
    status_item: Mutex<Option<MenuItem<Wry>>>,
    screentime_item: Mutex<Option<MenuItem<Wry>>>,
    // Last-emitted phase-changed payload, so a settings window opened
    // mid-session gets an instant snapshot instead of waiting for the next
    // transition (which may be up to a full work session away).
    current_phase: Mutex<Option<PhasePayload>>,
}

/// Emits `phase-changed` and remembers it as the current snapshot for
/// `load_phase` to serve to a freshly-opened settings window.
fn emit_phase(app: &AppHandle, payload: PhasePayload) {
    let state = app.state::<AppState>();
    *state.current_phase.lock().unwrap() = Some(payload.clone());
    let _ = app.emit("phase-changed", payload);
}

fn today_str() -> String {
    chrono::Local::now().date_naive().to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn format_hm(total_seconds: u64) -> String {
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
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

fn stats_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir.join("stats.json")
}

/// Reads stats.json, rolling over to a fresh day if the stored date has
/// passed — so a stale yesterday's tally is never shown as "today".
fn read_or_init_stats(app: &AppHandle) -> Stats {
    let path = stats_path(app);
    let mut s: Stats = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Stats::default(),
    };
    if s.date != today_str() {
        s = Stats::default();
    }
    let _ = fs::write(&path, serde_json::to_string_pretty(&s).unwrap());
    s
}

fn save_stats(app: &AppHandle, stats: &Stats) {
    let _ = fs::write(stats_path(app), serde_json::to_string_pretty(stats).unwrap());
}

/// Idle time (seconds since last keyboard/mouse input) via GNOME/Mutter's
/// IdleMonitor D-Bus interface. This is the mechanism GNOME actually
/// implements on Wayland/XWayland sessions — the X11 MIT-SCREEN-SAVER
/// extension (what the `user-idle` crate's x11 backend relies on) is NOT
/// implemented by Mutter's XWayland (verified: `Xlib: extension
/// "MIT-SCREEN-SAVER" missing on display ":0"`). Shells out to `gdbus`
/// rather than a hand-rolled zbus proxy — a 5s-interval poll comfortably
/// tolerates one process spawn, and `gdbus` ships with the GTK stack this
/// app already depends on. GNOME/Mutter-specific: returns None (→ caller
/// fails open) on any other desktop environment.
fn gnome_idle_seconds() -> Option<u64> {
    let output = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Mutter.IdleMonitor",
            "--object-path",
            "/org/gnome/Mutter/IdleMonitor/Core",
            "--method",
            "org.gnome.Mutter.IdleMonitor.GetIdletime",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // Output looks like "(uint64 1234,)\n" — pull the digits out.
    let text = String::from_utf8_lossy(&output.stdout);
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().ok().map(|ms| ms / 1000)
}

/// Bumps the completed-pomodoro tally, persists, and pushes it live. Rolls
/// over first in case the day changed mid-session.
fn record_pomodoro_completed(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut stats = state.stats.lock().unwrap();
    if stats.date != today_str() {
        *stats = Stats::default();
    }
    stats.pomodoros_completed += 1;
    save_stats(app, &stats);
    let _ = app.emit("stats-updated", stats.clone());
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
fn load_stats(state: State<AppState>) -> Stats {
    state.stats.lock().unwrap().clone()
}

#[tauri::command]
fn load_phase(state: State<AppState>) -> Option<PhasePayload> {
    state.current_phase.lock().unwrap().clone()
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

/// Single place that flips pause state and reflects it everywhere: the break
/// loop, the tray (menu label + status line + tooltip), and a `pause-changed`
/// event the settings window listens to.
fn set_pause(app: &AppHandle, paused: bool) {
    let state = app.state::<AppState>();
    state.paused.store(paused, Ordering::SeqCst);
    if !paused {
        state.resume_pause.notify_one();
    }
    if let Some(item) = state.pause_item.lock().unwrap().as_ref() {
        let _ = item.set_text(if paused { "Resume" } else { "Pause" });
    }
    if let Some(item) = state.status_item.lock().unwrap().as_ref() {
        let _ = item.set_text(if paused { "‖ Paused" } else { "● Running" });
    }
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_tooltip(Some(if paused {
            "Hourglass — paused"
        } else {
            "Hourglass"
        }));
    }
    let _ = app.emit("pause-changed", paused);
}

#[tauri::command]
fn is_paused(state: State<AppState>) -> bool {
    state.paused.load(Ordering::SeqCst)
}

#[tauri::command]
fn set_paused(app: AppHandle, paused: bool) {
    set_pause(&app, paused);
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Non-clickable status lines at the top reflect paused/running + today's
    // screen time at a glance.
    let status_i = MenuItemBuilder::with_id("status", "● Running")
        .enabled(false)
        .build(app)?;
    let screentime_i = MenuItemBuilder::with_id("screentime", "⏱ Screen time: 0m")
        .enabled(false)
        .build(app)?;
    let settings_i = MenuItemBuilder::with_id("settings", "Settings…").build(app)?;
    let break_i = MenuItemBuilder::with_id("break", "Take a break now").build(app)?;
    let pause_i = MenuItemBuilder::with_id("pause", "Pause").build(app)?;
    let quit_i = MenuItemBuilder::with_id("quit", "Quit Hourglass").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&status_i)
        .item(&screentime_i)
        .separator()
        .item(&settings_i)
        .item(&break_i)
        .item(&pause_i)
        .separator()
        .item(&quit_i)
        .build()?;

    // Keep handles so set_pause() / the screen-time poller can update the
    // labels live.
    {
        let state = app.state::<AppState>();
        *state.pause_item.lock().unwrap() = Some(pause_i.clone());
        *state.status_item.lock().unwrap() = Some(status_i.clone());
        *state.screentime_item.lock().unwrap() = Some(screentime_i.clone());
    }

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
                    set_pause(app, now);
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
        // Must be the first plugin: a second launch (autostart + manual click)
        // is funnelled into the already-running instance instead of spawning a
        // duplicate tray icon / overlay.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .setup(|app| {
            let handle = app.handle().clone();
            let config = read_or_init_config(&handle);
            let stats = read_or_init_stats(&handle);

            let resume = Arc::new(Notify::new());
            let break_now = Arc::new(Notify::new());
            let resume_pause = Arc::new(Notify::new());
            let paused = Arc::new(AtomicBool::new(false));
            let in_break = Arc::new(AtomicBool::new(false));

            app.manage(AppState {
                config: Mutex::new(config),
                stats: Mutex::new(stats),
                resume: resume.clone(),
                break_now: break_now.clone(),
                resume_pause: resume_pause.clone(),
                paused: paused.clone(),
                in_break: in_break.clone(),
                pause_item: Mutex::new(None),
                status_item: Mutex::new(None),
                screentime_item: Mutex::new(None),
                current_phase: Mutex::new(None),
            });

            build_tray(&handle)?;

            // Screen-time tracker: independent of the break loop (runs whether
            // or not break reminders are paused) — polls X11 idle time every
            // 5s via XScreenSaverQueryInfo and counts the tick as active screen
            // time when the user has touched keyboard/mouse more recently than
            // idle_threshold_seconds. Rolls the tally over at local midnight.
            {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    const POLL_SECS: u64 = 5;
                    loop {
                        tokio::time::sleep(Duration::from_secs(POLL_SECS)).await;
                        let idle_threshold =
                            h.state::<AppState>().config.lock().unwrap().idle_threshold_seconds;
                        // Blocking process spawn — off the async worker thread.
                        // Fail open on idle-read errors (e.g. non-GNOME session,
                        // or D-Bus not up yet at startup) — assume active rather
                        // than silently under-counting screen time all session.
                        let is_active = tokio::task::spawn_blocking(gnome_idle_seconds)
                            .await
                            .ok()
                            .flatten()
                            .map(|idle_secs| idle_secs < idle_threshold)
                            .unwrap_or(true);

                        let state = h.state::<AppState>();
                        let mut stats = state.stats.lock().unwrap();
                        if stats.date != today_str() {
                            *stats = Stats::default();
                        }
                        if is_active {
                            stats.screen_active_seconds += POLL_SECS;
                        }
                        save_stats(&h, &stats);
                        let _ = h.emit("stats-updated", stats.clone());
                        let screentime_item = state.screentime_item.lock().unwrap();
                        if let Some(item) = screentime_item.as_ref() {
                            let _ = item.set_text(format!(
                                "⏱ Screen time: {}",
                                format_hm(stats.screen_active_seconds)
                            ));
                        }
                    }
                });
            }

            // The break loop: sleep the work period (interruptible by "break now"
            // and "pause"), show the overlay, wait until dismissed, repeat.
            // In "pomodoro" mode the work/break durations + cadence come from the
            // pomodoro_* config fields, a long break replaces the short break
            // every `pomodoro_cycles`th cycle, and each completed work phase
            // bumps the day's pomodoro tally. `phase-changed` is pushed on every
            // transition (work included) so the settings window can drive a live
            // countdown + session dots without any per-second IPC heartbeat.
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut cycle: u64 = 1;
                loop {
                    if paused.load(Ordering::SeqCst) {
                        resume_pause.notified().await;
                        continue;
                    }

                    let cfg = h.state::<AppState>().config.lock().unwrap().clone();
                    let is_pomodoro = cfg.mode == "pomodoro";
                    let work_secs = if is_pomodoro {
                        cfg.pomodoro_work_seconds
                    } else {
                        cfg.work_seconds
                    };
                    let cycles_before_long = cfg.pomodoro_cycles.max(1);

                    emit_phase(
                        &h,
                        PhasePayload {
                            mode: cfg.mode.clone(),
                            phase: "work".into(),
                            duration_seconds: work_secs,
                            started_at_ms: now_ms(),
                            cycle,
                            cycles_before_long_break: cycles_before_long,
                        },
                    );

                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(work_secs)) => {}
                        _ = break_now.notified() => {}
                    }

                    if paused.load(Ordering::SeqCst) {
                        continue;
                    }

                    let (phase, break_secs) = if is_pomodoro {
                        if cycle % cycles_before_long == 0 {
                            ("long_break", cfg.pomodoro_long_break_seconds)
                        } else {
                            ("short_break", cfg.pomodoro_short_break_seconds)
                        }
                    } else {
                        ("break", cfg.break_seconds)
                    };

                    if is_pomodoro {
                        record_pomodoro_completed(&h);
                    }

                    in_break.store(true, Ordering::SeqCst);
                    show_overlay(&h).await;
                    emit_phase(
                        &h,
                        PhasePayload {
                            mode: cfg.mode.clone(),
                            phase: phase.into(),
                            duration_seconds: break_secs,
                            started_at_ms: now_ms(),
                            cycle,
                            cycles_before_long_break: cycles_before_long,
                        },
                    );
                    resume.notified().await;
                    in_break.store(false, Ordering::SeqCst);

                    if is_pomodoro {
                        cycle = if cycle % cycles_before_long == 0 { 1 } else { cycle + 1 };
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            load_stats,
            load_phase,
            break_done,
            open_settings,
            close_settings,
            quit_app,
            is_paused,
            set_paused
        ])
        .build(tauri::generate_context!())
        .expect("error while building Hourglass")
        .run(|_app, event| {
            // Stay resident in the tray when a window is closed/hidden (code:
            // None), but let an explicit app.exit() (tray "Quit") actually quit.
            // An unconditional prevent_exit() would swallow app.exit() too.
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
