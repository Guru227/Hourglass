//! Idle detection + the screen-time tracker thread.
//!
//! On Linux, we use GNOME/Mutter's IdleMonitor D-Bus interface for idle time,
//! with a fallback to FreeDesktop ScreenSaver. The X11 MIT-SCREEN-SAVER
//! extension is not available under Mutter's XWayland (verified: Mutter does
//! not implement it). We use libdbus's blocking API rather than zbus to avoid
//! pulling in an async runtime.
//!
//! On macOS and Windows, the `user-idle` crate measures idle time natively.

use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use hourglass_proto::{save_stats, today_str, format_hm, Stats, ToUi};

use crate::state::Ctx;

// Linux: dbus-based idle detection
#[cfg(target_os = "linux")]
use dbus::blocking::Connection;

/// Seconds since the last keyboard/mouse input, or None if unknown (caller
/// fails open = "active").
pub fn idle_seconds() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        linux_idle_seconds()
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        user_idle::UserIdle::get_time().ok().map(|t| t.as_seconds())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Every 5 s: count the tick as active screen time when idle < threshold,
/// roll the day over at midnight, persist on minute change, broadcast
/// `stats-updated`, and update the tray label when its text changes.
pub fn spawn(ctx: Ctx) {
    thread::Builder::new()
        .name("idle-tracker".into())
        .spawn(move || {
            const POLL_SECS: u64 = 5;
            let mut last_minute_bucket: u64 = 0;
            let mut last_label: Option<String> = None;

            loop {
                thread::sleep(Duration::from_secs(POLL_SECS));

                // Read the idle threshold
                let idle_threshold = ctx.with_state(|state| state.config.idle_threshold_seconds);

                // Check if active (fail open to true on error)
                let is_active = idle_seconds()
                    .map(|idle_secs| idle_secs < idle_threshold)
                    .unwrap_or(true);

                // Update the shared stats IN PLACE (the daemon's copy is what
                // init_state hands to a fresh Settings window and what
                // record_pomodoro_completed bumps), then copy out a snapshot.
                let (stats_changed, date_rolled, stats) = ctx.with_state(|state| {
                    let date_rolled = state.stats.date != today_str();
                    if date_rolled {
                        state.stats = Stats::default();
                    }
                    if is_active {
                        state.stats.screen_active_seconds += POLL_SECS;
                    }
                    (date_rolled || is_active, date_rolled, state.stats.clone())
                });

                // Persist only when the minute bucket moves or the day rolls:
                // v0.6 rewrote stats.json every 5 s, which was pure churn.
                let current_minute_bucket = stats.screen_active_seconds / 60;
                let minute_changed = current_minute_bucket != last_minute_bucket;

                if minute_changed || date_rolled {
                    save_stats(&stats);
                    last_minute_bucket = current_minute_bucket;
                }

                // Broadcast stats if something changed
                if stats_changed {
                    ctx.broadcast(ToUi::StatsUpdated(stats.clone()));
                }

                // Update tray label if text changed
                let new_label = format!("⏱ Screen time: {}", format_hm(stats.screen_active_seconds));
                if last_label.as_ref() != Some(&new_label) {
                    use crate::state::DaemonEvent;
                    use crate::state::TrayUpdate;
                    ctx.emit(DaemonEvent::Tray(TrayUpdate::ScreenTime(new_label.clone())));
                    last_label = Some(new_label);
                }
            }
        })
        .expect("Failed to spawn idle-tracker thread");
}

#[cfg(target_os = "linux")]
fn linux_idle_seconds() -> Option<u64> {
    // Try GNOME/Mutter IdleMonitor first
    if let Some(idle_ms) = try_gnome_idle() {
        return Some(idle_ms / 1000);
    }

    // Fall back to FreeDesktop ScreenSaver
    if let Some(idle_secs) = try_freedesktop_idle() {
        return Some(idle_secs);
    }

    None
}

#[cfg(target_os = "linux")]
fn try_gnome_idle() -> Option<u64> {
    thread_local! {
        static CONN: OnceLock<Arc<Mutex<Option<Connection>>>> = OnceLock::new();
    }

    CONN.with(|cell| {
        let conn_lock = cell.get_or_init(|| Arc::new(Mutex::new(None)));
        let mut conn_guard = conn_lock.lock().unwrap();

        // Try to get or create the connection
        if conn_guard.is_none() {
            match Connection::new_session() {
                Ok(c) => *conn_guard = Some(c),
                Err(_) => return None,
            }
        }

        let conn = conn_guard.as_ref()?;

        // Create a proxy and make the method call
        let proxy = conn.with_proxy(
            "org.gnome.Mutter.IdleMonitor",
            "/org/gnome/Mutter/IdleMonitor/Core",
            Duration::from_millis(500),
        );

        match proxy.method_call::<(u64,), (), &str, &str>(
            "org.gnome.Mutter.IdleMonitor",
            "GetIdletime",
            (),
        ) {
            Ok((idle_ms,)) => Some(idle_ms),
            Err(_) => {
                // Drop the connection on error so we reconnect next time
                *conn_guard = None;
                None
            }
        }
    })
}

#[cfg(target_os = "linux")]
fn try_freedesktop_idle() -> Option<u64> {
    let conn = Connection::new_session().ok()?;
    let proxy = conn.with_proxy(
        "org.freedesktop.ScreenSaver",
        "/org/freedesktop/ScreenSaver",
        Duration::from_millis(500),
    );

    match proxy.method_call::<(u32,), (), &str, &str>(
        "org.freedesktop.ScreenSaver",
        "GetSessionIdleTime",
        (),
    ) {
        Ok((idle_secs,)) => Some(idle_secs as u64),
        Err(_) => None,
    }
}
