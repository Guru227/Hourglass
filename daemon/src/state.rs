//! Shared daemon state + the two channels that tie the threads together.
//!
//! Threading model (see main.rs): the tao event loop owns the main thread and
//! is the ONLY thread that touches the tray (macOS requirement) or spawns /
//! writes to / kills `hourglass-ui` children. Every other thread — timer,
//! idle tracker, nudge watchdog, per-child stdout readers, single-instance
//! listener — mutates [`State`] under its mutex and then asks the main loop
//! for any visible effect by sending a [`DaemonEvent`] through the
//! [`tao::event_loop::EventLoopProxy`] held in [`Ctx`].
//!
//! Rule: never hold the `State` mutex while sending on the proxy or on the
//! timer channel. Copy out what you need, drop the guard, then send.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

/// `HOURGLASS_TRACE=1` prints every event, IPC line and timer command to
/// stderr — the only window into a headless daemon whose effects are all
/// cross-thread and cross-process.
pub fn trace(msg: impl FnOnce() -> String) {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("HOURGLASS_TRACE").is_some()) {
        eprintln!("[hourglassd] {}", msg());
    }
}

use hourglass_proto::{Config, Kind, PhasePayload, Stats, ToUi};
use tao::event_loop::EventLoopProxy;

/// Everything mutable, behind one mutex (v0.6's `AppState` minus the tokio
/// `Notify`s — those became [`TimerCmd`] — and minus the tray item handles,
/// which now live in `tray.rs` on the main thread).
pub struct State {
    pub config: Config,
    pub stats: Stats,
    pub paused: bool,
    pub in_break: bool,
    /// Last phase pushed, so a Settings window opened mid-session gets an
    /// instant snapshot instead of waiting for the next transition.
    pub current_phase: Option<PhasePayload>,
    /// Pause-nudge bookkeeping. Both are wall-clock epoch millis rather than
    /// sleeps, so a deadline keeps counting across a machine suspend.
    /// `paused_since_ms` is the anchor the deadline is re-derived from when the
    /// interval is edited mid-pause; `nudge_due_at_ms` is None whenever no
    /// nudge is armed — not paused, or the interval is configured to 0.
    pub paused_since_ms: Option<u64>,
    pub nudge_due_at_ms: Option<u64>,
}

impl State {
    pub fn new(config: Config, stats: Stats) -> Self {
        State {
            config,
            stats,
            paused: false,
            in_break: false,
            current_phase: None,
            paused_since_ms: None,
            nudge_due_at_ms: None,
        }
    }
}

pub type Shared = Arc<Mutex<State>>;

/// Commands that interrupt the timer thread's `recv_timeout` sleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerCmd {
    /// Tray "Take a break now": end the work phase immediately.
    BreakNow,
    /// Pause flipped on: abandon the current work phase and wait.
    Pause,
    /// Pause flipped off: start a fresh work phase.
    Resume,
    /// The break overlay was dismissed (button, corner pause, window closed,
    /// or the UI process died): leave the break phase.
    BreakDone,
}

/// Tray label changes, applied on the main thread only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUpdate {
    Paused(bool),
    ScreenTime(String),
}

/// Effects only the main thread may perform.
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    /// Spawn `hourglass-ui <kind>` (or focus it if already running).
    Show(Kind),
    /// Ask the child of this kind to close (sends [`ToUi::Close`], then reaps).
    Hide(Kind),
    /// Raise an already-running child of this kind. (`Show` already focuses
    /// a live child, so nothing emits this today; kept for the tray/CLI.)
    #[allow(dead_code)]
    Focus(Kind),
    /// Write one message to every live child's stdin.
    Broadcast(ToUi),
    Tray(TrayUpdate),
    /// Close every child and exit the process.
    Quit,
}

/// Cloneable handle every thread gets: the state, the timer's command
/// channel, and the way to reach the main loop.
#[derive(Clone)]
pub struct Ctx {
    pub state: Shared,
    pub timer: Sender<TimerCmd>,
    pub proxy: EventLoopProxy<DaemonEvent>,
}

impl Ctx {
    /// Fire-and-forget to the main loop. A closed loop means we are exiting,
    /// so the error is deliberately dropped.
    pub fn emit(&self, ev: DaemonEvent) {
        let _ = self.proxy.send_event(ev);
    }

    pub fn broadcast(&self, msg: ToUi) {
        self.emit(DaemonEvent::Broadcast(msg));
    }

    pub fn timer(&self, cmd: TimerCmd) {
        let _ = self.timer.send(cmd);
    }

    /// Run `f` with the state locked. Never call `emit`/`timer` from inside.
    pub fn with_state<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    }
}
