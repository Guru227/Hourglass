//! hourglass-ui child management (main thread only).
//!
//! One `hourglass-ui` process per window kind, spawned with an [`InitState`]
//! JSON blob in argv, fed [`ToUi`] lines on stdin, read for [`FromUi`] lines
//! on stdout by a per-child reader thread. Nothing here is resident between
//! windows — when the map is empty, no WebKit exists anywhere.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child as StdChild, ChildStdin, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use hourglass_proto::{FromUi, Kind, ToUi};

use crate::control;
use crate::state::Ctx;

struct Child {
    process: StdChild,
    stdin: ChildStdin,
}

/// Every `hourglass-ui` process currently running, keyed by which window it
/// hosts. The daemon runs at most one per [`Kind`].
pub struct Children {
    map: HashMap<Kind, Child>,
}

impl Children {
    pub fn new() -> Self {
        Children {
            map: HashMap::new(),
        }
    }

    /// Path to the sibling `hourglass-ui` binary, next to this daemon's own
    /// executable. Overridable via `HOURGLASS_UI_BIN` (used by tests).
    fn ui_binary_path() -> PathBuf {
        if let Ok(p) = std::env::var("HOURGLASS_UI_BIN") {
            return PathBuf::from(p);
        }
        let dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let mut path = dir.join("hourglass-ui");
        if cfg!(target_os = "windows") {
            path.set_extension("exe");
        }
        path
    }

    /// Drops any entry whose process has already exited. The reader thread
    /// still calls `control::on_ui_exited` on EOF regardless — this just
    /// keeps `show`'s "is one already running" check honest.
    fn reap(&mut self) {
        let dead: Vec<Kind> = self
            .map
            .iter_mut()
            .filter_map(|(k, c)| match c.process.try_wait() {
                Ok(Some(_)) => Some(*k),
                _ => None,
            })
            .collect();
        for k in dead {
            self.map.remove(&k);
        }
    }

    /// Focus a live child of this kind, or spawn a fresh `hourglass-ui`.
    pub fn show(&mut self, ctx: &Ctx, kind: Kind) {
        self.reap();

        if self.map.contains_key(&kind) {
            self.focus(kind);
            if kind == Kind::Nudge {
                self.write(kind, &ToUi::NudgeShown(control::nudge_info(ctx)));
            }
            return;
        }

        let init = control::init_state(ctx, kind);
        let payload = match serde_json::to_string(&init) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ui_launcher: failed to serialize InitState for {kind:?}: {e}");
                return;
            }
        };

        let bin = Self::ui_binary_path();
        let mut process = match Command::new(&bin)
            .arg(kind.as_str())
            .arg(payload)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("ui_launcher: failed to spawn {}: {e}", bin.display());
                return;
            }
        };

        let stdin = match process.stdin.take() {
            Some(s) => s,
            None => {
                eprintln!("ui_launcher: spawned {kind:?} child had no stdin pipe");
                let _ = process.kill();
                return;
            }
        };
        let stdout = process.stdout.take();

        if let Some(stdout) = stdout {
            let reader_ctx = ctx.clone();
            let spawned = thread::Builder::new()
                .name(format!("ui-{}-reader", kind.as_str()))
                .spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        if line.trim().is_empty() {
                            continue;
                        }
                        crate::state::trace(|| format!("from {kind:?}: {line}"));
                        match serde_json::from_str::<FromUi>(&line) {
                            Ok(msg) => control::handle_from_ui(&reader_ctx, kind, msg),
                            Err(e) => eprintln!(
                                "ui_launcher: unparsable line from {kind:?} child: {e} ({line:?})"
                            ),
                        }
                    }
                    crate::state::trace(|| format!("{kind:?} child stdout EOF"));
                    control::on_ui_exited(&reader_ctx, kind);
                });
            if let Err(e) = spawned {
                eprintln!("ui_launcher: failed to spawn reader thread for {kind:?}: {e}");
            }
        }

        self.map.insert(kind, Child { process, stdin });
    }

    /// Write one line to a live child's stdin. A write failure means the
    /// child died without going through the reader's EOF path yet (e.g. it
    /// was killed) — reap it here too.
    fn write(&mut self, kind: Kind, msg: &ToUi) {
        let Some(child) = self.map.get_mut(&kind) else {
            return;
        };
        let mut line = match serde_json::to_string(msg) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ui_launcher: failed to serialize {msg:?}: {e}");
                return;
            }
        };
        line.push('\n');
        let ok = child
            .stdin
            .write_all(line.as_bytes())
            .and_then(|_| child.stdin.flush())
            .is_ok();
        if !ok {
            self.map.remove(&kind);
        }
    }

    /// Ask a child to close, then give it up to 2s to exit on its own before
    /// killing it.
    pub fn hide(&mut self, kind: Kind) {
        if !self.map.contains_key(&kind) {
            return;
        }
        self.write(kind, &ToUi::Close);
        if let Some(child) = self.map.get_mut(&kind) {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match child.process.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            let _ = child.process.kill();
                            let _ = child.process.wait();
                            break;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
            self.map.remove(&kind);
        }
    }

    pub fn focus(&mut self, kind: Kind) {
        self.write(kind, &ToUi::Focus);
    }

    /// Write one message to every live child's stdin.
    pub fn broadcast(&mut self, msg: &ToUi) {
        let kinds: Vec<Kind> = self.map.keys().copied().collect();
        for kind in kinds {
            self.write(kind, msg);
        }
    }

    pub fn close_all(&mut self) {
        let kinds: Vec<Kind> = self.map.keys().copied().collect();
        for kind in kinds {
            self.hide(kind);
        }
    }
}
