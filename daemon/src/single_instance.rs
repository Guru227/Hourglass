//! One daemon per config dir.
//!
//! The port is derived from the config directory path via FNV-1a hash, so two
//! daemons with different XDG_CONFIG_HOME values never collide. This is a
//! pragmatic guard equal to what the old Tauri single-instance plugin
//! guaranteed.

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use hourglass_proto::config_dir;
use hourglass_proto::Kind;

use crate::state::{Ctx, DaemonEvent};

pub enum Instance {
    /// We hold the port: run as the daemon.
    Primary(TcpListener),
    /// Another daemon holds it: tell it to show Settings, then exit.
    Secondary,
}

/// Bind 127.0.0.1:<port> where port = 20000 + (fnv1a(config_dir) % 20000).
pub fn acquire() -> Instance {
    let port = calculate_port();
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => Instance::Primary(listener),
        Err(_) => Instance::Secondary,
    }
}

/// Secondary path: connect and send `show-settings\n`.
pub fn ask_primary_to_show_settings() {
    use std::io::Write;
    let port = calculate_port();
    if let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_millis(500),
    ) {
        let _ = writeln!(stream, "show-settings");
    }
}

/// Primary path: accept connections on a thread; a `show-settings` line
/// emits `DaemonEvent::Show(Kind::Settings)`.
pub fn spawn_listener(listener: TcpListener, ctx: Ctx) {
    thread::Builder::new()
        .name("single-instance".into())
        .spawn(move || {
            for incoming in listener.incoming() {
                if let Ok(stream) = incoming {
                    // Set a 1 second read timeout
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();

                    // Try to read one line
                    if let Ok(n) = reader.read_line(&mut line) {
                        if n > 0 && line.trim() == "show-settings" {
                            ctx.emit(DaemonEvent::Show(Kind::Settings));
                        }
                    }
                }
            }
        })
        .expect("Failed to spawn single-instance thread");
}

/// Calculate the port: 20000 + (fnv1a(config_dir) % 20000).
fn calculate_port() -> u16 {
    let config_dir_path = config_dir();
    let hash = fnv1a_64(config_dir_path.to_string_lossy().as_ref());
    (20000 + (hash % 20000)) as u16
}

/// FNV-1a 64-bit hash.
fn fnv1a_64(data: &str) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in data.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
