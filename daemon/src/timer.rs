//! The break state machine on its own thread.
//!
//! Ported from v0.6's tokio task (main.rs, the `tauri::async_runtime::spawn`
//! break loop) onto `std::thread` + `std::sync::mpsc`: `tokio::select!` on a
//! sleep + a `Notify` becomes `rx.recv_timeout(dur)` on a [`TimerCmd`]
//! channel, and the pause `Notify` becomes a plain blocking `rx.recv()`.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use hourglass_proto::{Kind, PhasePayload, ToUi};

use crate::control;
use crate::state::{Ctx, DaemonEvent, TimerCmd};

/// Runs the work → break → work loop until the process exits. Sleeping the
/// work/break period is `rx.recv_timeout(dur)`; any [`TimerCmd`] interrupts it.
pub fn spawn(ctx: Ctx, rx: Receiver<TimerCmd>) {
    std::thread::Builder::new()
        .name("timer".into())
        .spawn(move || run(ctx, rx))
        .expect("failed to spawn timer thread");
}

fn run(ctx: Ctx, rx: Receiver<TimerCmd>) {
    // 1-based position within the current pomodoro set (always 1 in simple
    // mode) — same bookkeeping as v0.6's `cycle` local.
    let mut cycle: u64 = 1;

    loop {
        if ctx.with_state(|s| s.paused) {
            // Block until resumed. A stray Pause/BreakNow/BreakDone arriving
            // while already paused is meaningless here; ignored.
            match block_until_resume(&rx) {
                ControlFlow::Continue => continue,
                ControlFlow::Exit => return,
            }
        }

        let cfg = ctx.with_state(|s| s.config.clone());
        let is_pomodoro = cfg.mode == "pomodoro";
        let work_secs = if is_pomodoro {
            cfg.pomodoro_work_seconds
        } else {
            cfg.work_seconds
        };
        let cycles_before_long = cfg.pomodoro_cycles.max(1);

        let work_payload = PhasePayload {
            mode: cfg.mode.clone(),
            phase: "work".into(),
            duration_seconds: work_secs,
            started_at_ms: hourglass_proto::now_ms(),
            cycle,
            cycles_before_long_break: cycles_before_long,
        };
        // `phase-changed` is pushed on every transition (work included) so a
        // Settings window can drive a live countdown + session dots without
        // any per-second IPC heartbeat.
        ctx.with_state(|s| s.current_phase = Some(work_payload.clone()));
        ctx.broadcast(ToUi::PhaseChanged(work_payload));

        // Wait out the work period, interruptible by "break now" and "pause".
        // Resume/BreakDone arriving mid-wait (stray signals) must not restart
        // the full duration — a wall-clock deadline plus a re-entrant wait
        // keeps counting down the REMAINING time instead.
        let deadline = Instant::now() + Duration::from_secs(work_secs);
        let mut break_now = false;
        let mut exit = false;
        loop {
            let now = Instant::now();
            if now >= deadline {
                break_now = true;
                break;
            }
            let got = rx.recv_timeout(deadline - now);
            crate::state::trace(|| format!("timer(work) got {got:?}"));
            match got {
                Ok(TimerCmd::BreakNow) => {
                    break_now = true;
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    break_now = true;
                    break;
                }
                // Pause: abandon the current work phase and wait — handled
                // by the paused check at the top of the outer loop.
                Ok(TimerCmd::Pause) => break,
                // Neither is meaningful mid-work; keep waiting for what's
                // left of the work period.
                Ok(TimerCmd::Resume) | Ok(TimerCmd::BreakDone) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    exit = true;
                    break;
                }
            }
        }
        if exit {
            return;
        }
        if !break_now {
            continue;
        }
        // Mirrors v0.6's re-check right after the select!: a Pause set the
        // state flag but may have raced the TimerCmd::Pause message past the
        // timeout above — re-reading the flag closes that window.
        if ctx.with_state(|s| s.paused) {
            continue;
        }

        let (phase_name, break_secs) = if is_pomodoro {
            if cycle % cycles_before_long == 0 {
                ("long_break", cfg.pomodoro_long_break_seconds)
            } else {
                ("short_break", cfg.pomodoro_short_break_seconds)
            }
        } else {
            ("break", cfg.break_seconds)
        };

        if is_pomodoro {
            control::record_pomodoro_completed(&ctx);
        }

        ctx.with_state(|s| s.in_break = true);

        let break_payload = PhasePayload {
            mode: cfg.mode.clone(),
            phase: phase_name.into(),
            duration_seconds: break_secs,
            started_at_ms: hourglass_proto::now_ms(),
            cycle,
            cycles_before_long_break: cycles_before_long,
        };
        // Set the phase snapshot BEFORE asking the main thread to show the
        // overlay: the launcher reads InitState (which embeds this snapshot)
        // when spawning the fresh child, so the break window renders the
        // break immediately instead of waiting on a first PhaseChanged.
        ctx.with_state(|s| s.current_phase = Some(break_payload.clone()));
        ctx.emit(DaemonEvent::Show(Kind::Break));
        // Also broadcast for any already-open Settings window.
        ctx.broadcast(ToUi::PhaseChanged(break_payload));

        // Wait for the break to end. A Pause received here (the overlay's
        // corner pause button sends SetPaused then BreakDone) must not
        // abandon the break — state.paused is already set by set_pause, so
        // simply ignoring the Pause message and continuing to wait for
        // BreakDone is enough; the outer loop's paused check picks it up
        // right after. BreakNow during a break is meaningless; ignored.
        match wait_for_break_done(&rx) {
            ControlFlow::Continue => {}
            ControlFlow::Exit => return,
        }

        ctx.with_state(|s| s.in_break = false);

        if is_pomodoro {
            cycle = if cycle % cycles_before_long == 0 { 1 } else { cycle + 1 };
        }
    }
}

enum ControlFlow {
    Continue,
    Exit,
}

fn block_until_resume(rx: &Receiver<TimerCmd>) -> ControlFlow {
    loop {
        match rx.recv() {
            Ok(TimerCmd::Resume) => return ControlFlow::Continue,
            Ok(_) => continue,
            Err(_) => return ControlFlow::Exit,
        }
    }
}

fn wait_for_break_done(rx: &Receiver<TimerCmd>) -> ControlFlow {
    loop {
        let got = rx.recv();
        crate::state::trace(|| format!("timer(break) got {got:?}"));
        match got {
            Ok(TimerCmd::BreakDone) => return ControlFlow::Continue,
            Ok(_) => continue,
            Err(_) => return ControlFlow::Exit,
        }
    }
}
