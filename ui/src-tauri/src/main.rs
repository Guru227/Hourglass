// Prevent a console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! hourglass-ui — the on-demand window host.
//!
//! One process shows exactly one window (break | nudge | settings), chosen by
//! argv[1] and seeded from the argv[2] JSON blob (an [`InitState`]). It is
//! driven live by [`ToUi`] messages read line-by-line from stdin, reports
//! user actions as [`FromUi`] messages written line-by-line to stdout, and
//! exits the moment its window is gone — there is no tray, no resident
//! event loop of its own, no reason to outlive the window it was spawned for.

use std::io::{BufRead, Write};
use std::sync::Mutex;
use std::time::Duration;

use hourglass_proto::{Config, FromUi, InitState, Kind, NudgeInfo, PhasePayload, Stats, ToUi};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

/// Everything a command or the stdin bridge needs: the live snapshot seeded
/// at launch and kept current by [`ToUi`] messages, the stdout pipe that
/// [`FromUi`] messages go out on, and which window kind this process is.
struct UiState {
    init: Mutex<InitState>,
    out: Mutex<std::io::Stdout>,
    kind: Kind,
}

/// Writes one [`FromUi`] message as a JSON line to stdout and flushes
/// immediately — the daemon reads this pipe line-by-line and must see each
/// action as soon as it happens, not whenever an OS buffer fills.
fn send(state: &UiState, msg: FromUi) {
    match serde_json::to_string(&msg) {
        Ok(line) => {
            let mut out = state.out.lock().unwrap();
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
        Err(e) => eprintln!("hourglass-ui: failed to encode {msg:?}: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Commands — same names as v0.6; the JS invokes them unchanged.
// ---------------------------------------------------------------------------

#[tauri::command]
fn load_config(state: State<UiState>) -> Config {
    state.init.lock().unwrap().config.clone()
}

#[tauri::command]
fn load_stats(state: State<UiState>) -> Stats {
    state.init.lock().unwrap().stats.clone()
}

#[tauri::command]
fn load_phase(state: State<UiState>) -> Option<PhasePayload> {
    state.init.lock().unwrap().phase.clone()
}

#[tauri::command]
fn is_paused(state: State<UiState>) -> bool {
    state.init.lock().unwrap().paused
}

#[tauri::command]
fn load_nudge_info(state: State<UiState>) -> NudgeInfo {
    state.init.lock().unwrap().nudge.clone()
}

#[tauri::command]
fn save_config(state: State<UiState>, config: Config) {
    state.init.lock().unwrap().config = config.clone();
    send(&state, FromUi::SaveConfig { config });
}

#[tauri::command]
fn set_paused(state: State<UiState>, paused: bool) {
    state.init.lock().unwrap().paused = paused;
    send(&state, FromUi::SetPaused { paused });
}

#[tauri::command]
fn break_done(app: AppHandle, state: State<UiState>) {
    send(&state, FromUi::BreakDone);
    app.exit(0);
}

#[tauri::command]
fn open_settings(state: State<UiState>) {
    // This process hosts the break/nudge window; the daemon owns spawning a
    // separate hourglass-ui process for the settings kind. This window stays
    // open — only report the request.
    send(&state, FromUi::OpenSettings);
}

#[tauri::command]
fn close_settings(app: AppHandle, state: State<UiState>) {
    send(&state, FromUi::Closed);
    app.exit(0);
}

#[tauri::command]
fn quit_app(app: AppHandle, state: State<UiState>) {
    send(&state, FromUi::QuitApp);
    app.exit(0);
}

#[tauri::command]
fn nudge_resume(app: AppHandle, state: State<UiState>) {
    send(&state, FromUi::NudgeResume);
    app.exit(0);
}

#[tauri::command]
fn nudge_snooze(app: AppHandle, state: State<UiState>) {
    send(&state, FromUi::NudgeSnooze);
    app.exit(0);
}

// ---------------------------------------------------------------------------
// stdin bridge — daemon → ui
// ---------------------------------------------------------------------------

/// Applies one [`ToUi`] message: updates the seeded snapshot so a page that
/// re-reads it later (e.g. after a reload) sees the latest value, then
/// forwards it to the webview as the event the page already `listen()`s for.
fn apply_to_ui(app: &AppHandle, msg: ToUi) {
    let state = app.state::<UiState>();
    let kind = state.kind;
    match msg {
        ToUi::PhaseChanged(p) => {
            state.init.lock().unwrap().phase = Some(p.clone());
            let _ = app.emit("phase-changed", p);
        }
        ToUi::ConfigUpdated(c) => {
            state.init.lock().unwrap().config = c.clone();
            let _ = app.emit("config-updated", c);
        }
        ToUi::PauseChanged(b) => {
            state.init.lock().unwrap().paused = b;
            let _ = app.emit("pause-changed", b);
        }
        ToUi::StatsUpdated(s) => {
            state.init.lock().unwrap().stats = s.clone();
            let _ = app.emit("stats-updated", s);
        }
        ToUi::NudgeShown(n) => {
            state.init.lock().unwrap().nudge = n.clone();
            let _ = app.emit("nudge-shown", n);
        }
        ToUi::Focus => {
            if let Some(win) = app.get_webview_window(kind.as_str()) {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }
        ToUi::Close => {
            app.exit(0);
        }
    }
}

/// Reads `ToUi` messages from stdin, one JSON object per line, for the life
/// of the process. Runs on its own thread since it blocks on stdin reads.
/// EOF means the daemon's write end closed — i.e. the daemon died — so this
/// process has no reason to keep its window up either.
fn spawn_stdin_bridge(app: AppHandle) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ToUi>(&line) {
                Ok(msg) => apply_to_ui(&app, msg),
                Err(e) => eprintln!("hourglass-ui: bad stdin line: {e}"),
            }
        }
        // stdin closed — the daemon died. Don't linger as an orphan window.
        app.exit(0);
    });
}

// ---------------------------------------------------------------------------
// Window construction — v0.6's per-kind window properties, reproduced
// exactly, applied to a window built at runtime instead of declared in
// tauri.conf.json.
// ---------------------------------------------------------------------------

fn build_window(app: &tauri::App, kind: Kind) -> tauri::Result<tauri::WebviewWindow> {
    let builder = WebviewWindowBuilder::new(app, kind.as_str(), WebviewUrl::App(kind.page().into()))
        .visible(true);

    let builder = match kind {
        Kind::Break => builder
            .title("Hourglass")
            .fullscreen(true)
            .decorations(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .resizable(true)
            .focused(true)
            .shadow(false),
        Kind::Settings => builder
            .title("Hourglass Settings")
            .inner_size(480.0, 660.0)
            .min_inner_size(420.0, 560.0)
            .resizable(true)
            .decorations(true)
            .center()
            .skip_taskbar(false),
        Kind::Nudge => builder
            .title("Hourglass — still paused")
            .inner_size(460.0, 268.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .center()
            .shadow(false),
    };

    builder.build()
}

/// Break and nudge both need to land always-on-top + (break only) fullscreen
/// + sticky-to-every-workspace, and both need it applied *after* the window
/// has actually been mapped — not merely after `show()`. Polls `is_visible()`
/// instead of v0.6's fixed sleep so a slow compositor doesn't lose the race:
/// the very first state call issued before the window is realized is
/// silently dropped under XWayland (observed: fullscreen failed while a
/// later above/sticky call, issued after the window had settled, stuck).
fn settle_and_apply_state(win: tauri::WebviewWindow, kind: Kind) {
    std::thread::spawn(move || {
        for _ in 0..30 {
            if matches!(win.is_visible(), Ok(true)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if matches!(kind, Kind::Break) {
            let _ = win.set_fullscreen(true);
        }
        let _ = win.set_always_on_top(true);
        let _ = win.set_visible_on_all_workspaces(true);
        let _ = win.set_focus();
    });
}

fn main() {
    // Run through XWayland on GNOME/Wayland. Native Wayland forbids clients
    // from self-positioning, forcing always-on-top, or sticking a window to
    // all workspaces; under X11 (XWayland) all three work. Must happen before
    // any GTK init — this process links GTK, so it belongs here, not in the
    // daemon. Respects an explicit user GDK_BACKEND and only kicks in when
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

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: hourglass-ui <break|nudge|settings> <init-json>");
        std::process::exit(2);
    }
    let Some(kind) = Kind::parse(&args[1]) else {
        eprintln!(
            "usage: hourglass-ui <break|nudge|settings> <init-json> (got kind {:?})",
            args[1]
        );
        std::process::exit(2);
    };
    let init: InitState = match serde_json::from_str(&args[2]) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("usage: hourglass-ui <break|nudge|settings> <init-json>: {e}");
            std::process::exit(2);
        }
    };

    tauri::Builder::default()
        .manage(UiState {
            init: Mutex::new(init),
            out: Mutex::new(std::io::stdout()),
            kind,
        })
        .setup(move |app| {
            let win = build_window(app, kind)?;
            if matches!(kind, Kind::Break | Kind::Nudge) {
                settle_and_apply_state(win, kind);
            }
            spawn_stdin_bridge(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // The user closed the window (WM close button, Esc-to-close on
            // settings, etc.). Report it and let the process exit naturally —
            // it does, since this is the last (only) window. No prevent_exit
            // here: unlike v0.6's resident tray app, this process must die
            // with its window.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<UiState>();
                send(&state, FromUi::Closed);
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            load_stats,
            load_phase,
            is_paused,
            load_nudge_info,
            save_config,
            set_paused,
            break_done,
            open_settings,
            close_settings,
            quit_app,
            nudge_resume,
            nudge_snooze,
        ])
        .run(tauri::generate_context!())
        .expect("error while running hourglass-ui");
}
