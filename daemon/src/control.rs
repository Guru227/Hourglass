//! The daemon's command surface — every state transition that v0.6 exposed
//! as a `#[tauri::command]` or a tray click, as plain functions over [`Ctx`].
//!
//! Called from: per-child stdout reader threads ([`handle_from_ui`]), the
//! tray dispatcher on the main thread, the nudge watchdog, and the timer.
//! None of these functions touch the tray or a child directly; they mutate
//! state and emit [`crate::state::DaemonEvent`]s.
//!
//! Rule (state.rs): never hold the state mutex while calling `ctx.emit` /
//! `ctx.broadcast` / `ctx.timer`. Every function here copies what it needs
//! out of `with_state`, lets the guard drop, then sends.

use hourglass_proto::{Config, FromUi, InitState, Kind, NudgeInfo, Stats, ToUi};

use crate::state::{Ctx, DaemonEvent, State, TimerCmd, TrayUpdate};

/// Snapshot for the nudge window, computed with the state lock already held
/// (so [`init_state`] can build one alongside the rest of its snapshot
/// without a nested lock). [`nudge_info`] is the public, self-locking form.
fn nudge_info_of(s: &State) -> NudgeInfo {
    let paused_seconds = s
        .paused_since_ms
        .map(|since| hourglass_proto::now_ms().saturating_sub(since) / 1000)
        .unwrap_or(0);
    NudgeInfo {
        paused_seconds,
        snooze_seconds: s.config.pause_snooze_seconds.max(60),
        theme: s.config.theme.clone(),
    }
}

/// Single place that flips pause state and reflects it everywhere: the timer
/// thread, the tray (menu label + status line + tooltip), every open window
/// (`pause-changed`), and the nudge deadline (armed on pause, cleared and the
/// nudge window hidden on resume).
pub fn set_pause(ctx: &Ctx, paused: bool) {
    crate::state::trace(|| format!("set_pause({paused})"));
    ctx.with_state(|s| {
        s.paused = paused;
        // Every pause path (tray, settings, the overlay's corner button)
        // funnels through here, so arming/disarming the nudge here covers
        // all of them.
        if paused {
            let after = s.config.pause_nudge_after_seconds;
            let now = hourglass_proto::now_ms();
            s.paused_since_ms = Some(now);
            s.nudge_due_at_ms = if after > 0 { Some(now + after * 1000) } else { None };
        } else {
            s.paused_since_ms = None;
            s.nudge_due_at_ms = None;
        }
    });

    if paused {
        ctx.timer(TimerCmd::Pause);
    } else {
        ctx.emit(DaemonEvent::Hide(Kind::Nudge));
        ctx.timer(TimerCmd::Resume);
    }

    ctx.emit(DaemonEvent::Tray(TrayUpdate::Paused(paused)));
    ctx.broadcast(ToUi::PauseChanged(paused));
}

pub fn toggle_pause(ctx: &Ctx) {
    let currently_paused = ctx.with_state(|s| s.paused);
    set_pause(ctx, !currently_paused);
}

/// Persist + apply a config from the Settings window. If a pause is already
/// running, re-anchor its nudge deadline off `paused_since_ms` with the *new*
/// interval — otherwise shortening the interval mid-pause wouldn't take
/// effect until the pause after this one. Broadcasts `config-updated`
/// carrying the full config.
pub fn save_config(ctx: &Ctx, config: Config) {
    hourglass_proto::save_config(&config);

    ctx.with_state(|s| {
        s.config = config;
        if s.paused {
            let since = s.paused_since_ms;
            let after = s.config.pause_nudge_after_seconds;
            s.nudge_due_at_ms = match since {
                Some(since) if after > 0 => Some(since + after * 1000),
                _ => None,
            };
        }
    });

    let config = ctx.with_state(|s| s.config.clone());
    ctx.broadcast(ToUi::ConfigUpdated(config));
}

/// The break overlay was dismissed: leave the break, hide the overlay child.
pub fn break_done(ctx: &Ctx) {
    ctx.with_state(|s| s.in_break = false);
    ctx.timer(TimerCmd::BreakDone);
    ctx.emit(DaemonEvent::Hide(Kind::Break));
}

/// Tray "Take a break now" — ignored while already in a break.
pub fn request_break_now(ctx: &Ctx) {
    let in_break = ctx.with_state(|s| s.in_break);
    if !in_break {
        ctx.timer(TimerCmd::BreakNow);
    }
}

/// Nudge → "Switch on timer". `set_pause(false)` hides the nudge and clears
/// the deadline itself, so this is a thin alias kept for a self-describing
/// call site in the nudge window.
pub fn nudge_resume(ctx: &Ctx) {
    set_pause(ctx, false);
}

/// Nudge → "Snooze": stay paused, push the next nudge out, hide the window.
pub fn nudge_snooze(ctx: &Ctx) {
    arm_snooze(ctx);
    ctx.emit(DaemonEvent::Hide(Kind::Nudge));
}

/// Arms the next nudge `pause_snooze_seconds` (min 60) out from now. Used by
/// the Snooze button and by the watchdog when it fires (so an ignored nudge
/// comes back instead of going quiet after one appearance).
pub fn arm_snooze(ctx: &Ctx) {
    ctx.with_state(|s| {
        let snooze = s.config.pause_snooze_seconds.max(60);
        s.nudge_due_at_ms = Some(hourglass_proto::now_ms() + snooze * 1000);
    });
}

/// Snapshot for the nudge window. `paused_seconds` is 0 when not paused,
/// which the window renders as a generic line rather than "0m".
pub fn nudge_info(ctx: &Ctx) -> NudgeInfo {
    ctx.with_state(|s| nudge_info_of(s))
}

/// Bumps the completed-pomodoro tally, persists, and pushes `stats-updated`.
/// Rolls the day over first in case the date changed mid-session.
pub fn record_pomodoro_completed(ctx: &Ctx) {
    let stats = ctx.with_state(|s| {
        if s.stats.date != hourglass_proto::today_str() {
            s.stats = Stats::default();
        }
        s.stats.pomodoros_completed += 1;
        hourglass_proto::save_stats(&s.stats);
        s.stats.clone()
    });
    ctx.broadcast(ToUi::StatsUpdated(stats));
}

/// Everything a new `hourglass-ui <kind>` needs for its first paint.
pub fn init_state(ctx: &Ctx, kind: Kind) -> InitState {
    ctx.with_state(|s| InitState {
        kind,
        config: s.config.clone(),
        paused: s.paused,
        phase: s.current_phase.clone(),
        stats: s.stats.clone(),
        nudge: nudge_info_of(s),
    })
}

/// Dispatch one line received from a child's stdout.
pub fn handle_from_ui(ctx: &Ctx, kind: Kind, msg: FromUi) {
    match msg {
        FromUi::BreakDone => break_done(ctx),
        FromUi::SetPaused { paused } => set_pause(ctx, paused),
        FromUi::SaveConfig { config } => save_config(ctx, config),
        FromUi::NudgeResume => nudge_resume(ctx),
        FromUi::NudgeSnooze => nudge_snooze(ctx),
        FromUi::OpenSettings => ctx.emit(DaemonEvent::Show(Kind::Settings)),
        FromUi::QuitApp => ctx.emit(DaemonEvent::Quit),
        // The user closed the overlay some other way (WM close, Esc). A
        // Break child counts this as break_done so the timer never wedges;
        // Settings/Nudge need nothing here — the launcher reaps the child.
        FromUi::Closed => {
            if kind == Kind::Break {
                let in_break = ctx.with_state(|s| s.in_break);
                if in_break {
                    break_done(ctx);
                }
            }
        }
    }
}

/// A child's stdout hit EOF (process exited or crashed). A Break child that
/// vanished mid-break counts as `break_done` so the timer never wedges — the
/// same treatment as an explicit `Closed`, for a UI that crashed outright.
pub fn on_ui_exited(ctx: &Ctx, kind: Kind) {
    if kind == Kind::Break {
        let in_break = ctx.with_state(|s| s.in_break);
        if in_break {
            break_done(ctx);
        }
    }
}
