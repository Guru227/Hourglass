//! hourglassd — the resident half of Hourglass.
//!
//! Owns the tao event loop on the main thread: the tray icon and every
//! `hourglass-ui` child process may only be touched from here. Every other
//! thread (timer, idle tracker, nudge watchdog, single-instance listener,
//! per-child stdout readers) mutates `State` behind `Ctx` and asks this loop
//! for any visible effect via `DaemonEvent`. See state.rs for the full
//! threading-model rationale.

// Prevent a console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod control;
mod idle;
mod nudge;
mod single_instance;
mod state;
mod timer;
mod tray;
mod ui_launcher;

use std::sync::{mpsc, Arc, Mutex};

use hourglass_proto::{read_or_init_config, read_or_init_stats};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use single_instance::Instance;
use state::{Ctx, DaemonEvent, State};
use tray::Tray;

fn main() {
    // One daemon per config dir: a second launch hands off to the primary
    // and exits rather than spawning a duplicate tray icon.
    let listener = match single_instance::acquire() {
        Instance::Secondary => {
            single_instance::ask_primary_to_show_settings();
            return;
        }
        Instance::Primary(listener) => listener,
    };

    let config = read_or_init_config();
    let stats = read_or_init_stats();
    let state = Arc::new(Mutex::new(State::new(config, stats)));

    let event_loop = EventLoopBuilder::<DaemonEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let (timer_tx, timer_rx) = mpsc::channel();
    let ctx = Ctx {
        state,
        timer: timer_tx,
        proxy,
    };

    timer::spawn(ctx.clone(), timer_rx);
    idle::spawn(ctx.clone());
    nudge::spawn(ctx.clone());
    single_instance::spawn_listener(listener, ctx.clone());

    // Tray creation must happen on the main thread (macOS requirement) and
    // after the event loop above, since tao's `EventLoop::new` initializes
    // GTK on Linux and tray-icon relies on that already having happened.
    let tray = tray::build().expect("failed to build tray icon");
    Tray::install_handler(ctx.clone());

    let mut children = ui_launcher::Children::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(ev) => match {
                state::trace(|| format!("event {ev:?}"));
                ev
            } {
                DaemonEvent::Show(kind) => children.show(&ctx, kind),
                DaemonEvent::Hide(kind) => children.hide(kind),
                DaemonEvent::Focus(kind) => children.focus(kind),
                DaemonEvent::Broadcast(msg) => children.broadcast(&msg),
                DaemonEvent::Tray(update) => tray.apply(&update),
                DaemonEvent::Quit => {
                    children.close_all();
                    *control_flow = ControlFlow::Exit;
                }
            },
            Event::LoopDestroyed => children.close_all(),
            _ => {}
        }
    })
}
