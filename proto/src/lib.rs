//! hourglass-proto — the contract between the resident daemon (`hourglassd`)
//! and the on-demand window host (`hourglass-ui`).
//!
//! Pure data + a few path/time helpers. Nothing here may pull in GTK, Tauri,
//! tokio or any platform library: the daemon links this to stay tiny, the UI
//! links it to speak the same wire format.
//!
//! Wire format: one JSON object per line.
//!   daemon → ui (child's stdin):  [`ToUi`]   — `{"type":"phase-changed","payload":{…}}`
//!   ui → daemon (child's stdout): [`FromUi`] — `{"type":"set_paused","args":{"paused":true}}`
//! `ToUi` type strings are exactly the Tauri event names the pages already
//! `listen()` for; `FromUi` type strings are exactly the `invoke()` command
//! names the pages already call. That is what lets the JS stay untouched.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Bundle identifier — also the config directory name. Must stay identical to
/// the Tauri `identifier` so v0.6 users keep their config/stats files.
pub const APP_ID: &str = "com.guru227.hourglass";

// ---------------------------------------------------------------------------
// Persisted user data
// ---------------------------------------------------------------------------

/// User-editable settings, stored as JSON in the OS config dir
/// (e.g. ~/.config/com.guru227.hourglass/config.json). `#[serde(default)]`
/// lets older/partial config files load — missing fields fall back to default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Seconds of work between breaks (simple mode).
    pub work_seconds: u64,
    /// Forced break length = seconds the "I'm done" button stays disabled (simple mode).
    pub break_seconds: u64,
    pub msg_heading: String,
    pub msg_body: String,
    pub quit_button_msg: String,
    /// "crt" | "dark" | "light"
    pub theme: String,
    /// "both" | "factoids" | "quotes"
    pub content_mode: String,
    /// "simple" | "pomodoro"
    pub mode: String,
    pub pomodoro_work_seconds: u64,
    pub pomodoro_short_break_seconds: u64,
    pub pomodoro_long_break_seconds: u64,
    /// Work cycles between long breaks.
    pub pomodoro_cycles: u64,
    /// Seconds of no keyboard/mouse input before the screen-time tracker
    /// treats the user as away.
    pub idle_threshold_seconds: u64,
    /// Seconds a pause may run before Hourglass nudges you to switch the timer
    /// back on. 0 disables the nudge entirely.
    pub pause_nudge_after_seconds: u64,
    /// How far the nudge's "Snooze" button pushes the next nudge out. Doubles
    /// as the re-fire interval for a nudge the user never answers.
    pub pause_snooze_seconds: u64,
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
            pause_nudge_after_seconds: 7200,
            pause_snooze_seconds: 1800,
        }
    }
}

/// Daily tallies — screen time (idle-detected) + completed pomodoros.
/// Stored as JSON alongside config.json; rolled over whenever the local date
/// on disk no longer matches today.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    /// Local YYYY-MM-DD this tally belongs to.
    pub date: String,
    pub screen_active_seconds: u64,
    pub pomodoros_completed: u64,
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

// ---------------------------------------------------------------------------
// Live snapshots pushed to windows
// ---------------------------------------------------------------------------

/// A phase transition. The overlay reacts to non-"work" phases; the settings
/// window uses every phase to drive its live countdown + session dots.
/// `started_at_ms` + `duration_seconds` let the page tick its own countdown
/// locally rather than the daemon pushing a heartbeat every second.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhasePayload {
    pub mode: String,
    /// "work" | "break" | "short_break" | "long_break"
    pub phase: String,
    pub duration_seconds: u64,
    pub started_at_ms: u64,
    /// 1-based position within the current pomodoro set (always 1 in simple mode).
    pub cycle: u64,
    pub cycles_before_long_break: u64,
}

/// What the pause-nudge window renders: how long this pause has run, what
/// "Snooze" is worth right now, and the theme to paint in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeInfo {
    pub paused_seconds: u64,
    pub snooze_seconds: u64,
    pub theme: String,
}

// ---------------------------------------------------------------------------
// Window kinds + launch snapshot
// ---------------------------------------------------------------------------

/// The three windows the UI host can be asked to show. One `hourglass-ui`
/// process hosts exactly one of these; the daemon runs at most one per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Break,
    Nudge,
    Settings,
}

impl Kind {
    /// argv[1] of `hourglass-ui`, and the Tauri window label.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Break => "break",
            Kind::Nudge => "nudge",
            Kind::Settings => "settings",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "break" => Some(Kind::Break),
            "nudge" => Some(Kind::Nudge),
            "settings" => Some(Kind::Settings),
            _ => None,
        }
    }

    /// The page each kind loads from the frontend dist.
    pub fn page(self) -> &'static str {
        match self {
            Kind::Break => "index.html",
            Kind::Nudge => "nudge.html",
            Kind::Settings => "settings.html",
        }
    }
}

/// Everything a freshly spawned window needs to render its first frame,
/// passed as argv[2] (one JSON blob) so no round-trip is needed before paint.
/// The UI seeds a mutex from it; `load_config` / `load_phase` / `is_paused` /
/// `load_stats` / `load_nudge_info` answer from that mutex, which the stdin
/// reader keeps current from later [`ToUi`] messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitState {
    pub kind: Kind,
    pub config: Config,
    pub paused: bool,
    pub phase: Option<PhasePayload>,
    pub stats: Stats,
    pub nudge: NudgeInfo,
}

// ---------------------------------------------------------------------------
// Wire messages
// ---------------------------------------------------------------------------

/// daemon → ui. The `type` string doubles as the webview event name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum ToUi {
    PhaseChanged(PhasePayload),
    /// Carries the full new config so the UI's snapshot is overwritten before
    /// the page's `config-updated` handler calls `load_config` again.
    ConfigUpdated(Config),
    PauseChanged(bool),
    StatsUpdated(Stats),
    NudgeShown(NudgeInfo),
    /// Raise + focus the window (second launch, tray "Settings…" while open).
    Focus,
    /// Close the window and exit the process (daemon-initiated hide/quit).
    Close,
}

/// ui → daemon. The `type` string is the `invoke()` command name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "args", rename_all = "snake_case")]
pub enum FromUi {
    BreakDone,
    SetPaused { paused: bool },
    SaveConfig { config: Config },
    NudgeResume,
    NudgeSnooze,
    OpenSettings,
    QuitApp,
    /// The window was closed by the user (WM close, Esc, close_settings…);
    /// the process is about to exit. Break kind ⇒ daemon treats as break_done.
    Closed,
}

// ---------------------------------------------------------------------------
// Paths + time helpers (identical semantics to v0.6)
// ---------------------------------------------------------------------------

/// The per-user config directory. Resolves exactly where Tauri 2's
/// `app_config_dir()` did for this identifier: Linux `$XDG_CONFIG_HOME/<id>`
/// (default `~/.config/<id>`), macOS `~/Library/Application Support/<id>`,
/// Windows `%APPDATA%\<id>`. Created on first call.
pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn stats_path() -> PathBuf {
    config_dir().join("stats.json")
}

pub fn read_or_init_config() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => {
            let c = Config::default();
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&c).unwrap());
            c
        }
    }
}

pub fn save_config(config: &Config) {
    let _ = std::fs::write(config_path(), serde_json::to_string_pretty(config).unwrap());
}

/// Reads stats.json, rolling over to a fresh day if the stored date has
/// passed — so a stale yesterday's tally is never shown as "today".
pub fn read_or_init_stats() -> Stats {
    let path = stats_path();
    let mut s: Stats = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Stats::default(),
    };
    if s.date != today_str() {
        s = Stats::default();
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&s).unwrap());
    s
}

pub fn save_stats(stats: &Stats) {
    let _ = std::fs::write(stats_path(), serde_json::to_string_pretty(stats).unwrap());
}

pub fn today_str() -> String {
    chrono::Local::now().date_naive().to_string()
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn format_hm(total_seconds: u64) -> String {
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_ui_type_strings_match_js_event_names() {
        let s = serde_json::to_string(&ToUi::PauseChanged(true)).unwrap();
        assert_eq!(s, r#"{"type":"pause-changed","payload":true}"#);
        let s = serde_json::to_string(&ToUi::Focus).unwrap();
        assert_eq!(s, r#"{"type":"focus"}"#);
    }

    #[test]
    fn from_ui_type_strings_match_invoke_names() {
        let m: FromUi = serde_json::from_str(r#"{"type":"set_paused","args":{"paused":true}}"#).unwrap();
        assert!(matches!(m, FromUi::SetPaused { paused: true }));
        let m: FromUi = serde_json::from_str(r#"{"type":"break_done"}"#).unwrap();
        assert!(matches!(m, FromUi::BreakDone));
    }

    #[test]
    fn config_dir_ends_with_app_id() {
        assert!(config_dir().ends_with(APP_ID));
    }
}
