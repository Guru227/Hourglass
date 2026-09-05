//! Pause watchdog thread.
//!
//! Ported from v0.6's tokio task (main.rs, the "Pause watchdog" spawn) onto a
//! plain `std::thread` with `thread::sleep`. Polling a stored epoch (rather
//! than sleeping until a computed deadline) means the nudge is correct
//! across suspend/resume and picks up a deadline that `set_pause` /
//! `save_config` moved underneath it.

use std::thread;
use std::time::Duration;

use hourglass_proto::{Kind, ToUi};

use crate::control;
use crate::state::{Ctx, DaemonEvent};

const TICK_SECS: u64 = 10;

/// Every 10 s while paused: if `nudge_due_at_ms` has passed, re-arm one
/// snooze out and ask the main loop to show the nudge window.
pub fn spawn(ctx: Ctx) {
    thread::Builder::new()
        .name("nudge-watchdog".into())
        .spawn(move || run(ctx))
        .expect("failed to spawn nudge-watchdog thread");
}

fn run(ctx: Ctx) {
    loop {
        thread::sleep(Duration::from_secs(TICK_SECS));

        let due_now = ctx.with_state(|s| {
            s.paused && matches!(s.nudge_due_at_ms, Some(due) if hourglass_proto::now_ms() >= due)
        });

        if due_now {
            // Re-arm before showing: a nudge the user never answers comes
            // back one snooze later instead of firing once and going silent.
            control::arm_snooze(&ctx);
            ctx.emit(DaemonEvent::Show(Kind::Nudge));
            // The launcher passes fresh NudgeInfo in InitState when it spawns
            // a new child; if a nudge child is already open it sends Focus
            // instead, which never re-reads InitState — so also broadcast
            // the live numbers here to cover that case.
            ctx.broadcast(ToUi::NudgeShown(control::nudge_info(&ctx)));
        }
    }
}
