//! Tray icon + menu (main thread only). v0.6's menu, rebuilt on tray-icon +
//! muda so no Tauri is linked into the resident process.

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use hourglass_proto::Kind;

use crate::control;
use crate::state::{Ctx, DaemonEvent, TrayUpdate};

/// The tray icon plus the three menu items whose label changes at runtime.
pub struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    screentime: MenuItem,
    pause: MenuItem,
}

/// Build the tray icon and its menu (v0.6's layout, unchanged).
pub fn build() -> Result<Tray, Box<dyn std::error::Error>> {
    let status = MenuItem::with_id("status", "● Running", false, None);
    let screentime = MenuItem::with_id("screentime", "⏱ Screen time: 0m", false, None);
    let settings = MenuItem::with_id("settings", "Settings…", true, None);
    let take_break = MenuItem::with_id("break", "Take a break now", true, None);
    let pause = MenuItem::with_id("pause", "Pause", true, None);
    let quit = MenuItem::with_id("quit", "Quit Hourglass", true, None);

    let menu = Menu::new();
    menu.append_items(&[
        &status,
        &screentime,
        &PredefinedMenuItem::separator(),
        &settings,
        &take_break,
        &pause,
        &PredefinedMenuItem::separator(),
        &quit,
    ])?;

    let icon = load_icon()?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Hourglass")
        .with_icon(icon)
        .build()?;

    Ok(Tray {
        icon: tray_icon,
        status,
        screentime,
        pause,
    })
}

impl Tray {
    /// Apply a label/tooltip change computed on some other thread.
    pub fn apply(&self, u: &TrayUpdate) {
        match u {
            TrayUpdate::Paused(true) => {
                self.pause.set_text("Resume");
                self.status.set_text("‖ Paused");
                let _ = self.icon.set_tooltip(Some("Hourglass — paused"));
            }
            TrayUpdate::Paused(false) => {
                self.pause.set_text("Pause");
                self.status.set_text("● Running");
                let _ = self.icon.set_tooltip(Some("Hourglass"));
            }
            TrayUpdate::ScreenTime(label) => {
                self.screentime.set_text(label);
            }
        }
    }

    /// Wire muda's menu-click channel straight to the actions it names.
    ///
    /// This callback runs on muda's own dispatch thread, not necessarily the
    /// tao loop thread, so `set_event_handler` requires the closure to be
    /// `Send + Sync`. `Ctx` carries a `std::sync::mpsc::Sender`, which is
    /// `Send` but never `Sync` — so `Ctx` itself isn't `Sync` and can't be
    /// captured directly. Wrapping it in a `Mutex` sidesteps that: `Mutex<T>`
    /// is `Sync` whenever `T: Send`, regardless of `T`'s own `Sync`-ness, and
    /// each call here only needs the lock for the instant it takes to act.
    pub fn install_handler(ctx: Ctx) {
        let ctx = std::sync::Mutex::new(ctx);
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let ctx = match ctx.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            crate::state::trace(|| format!("tray menu click {:?}", event.id().0));
            match event.id().0.as_str() {
                "settings" => ctx.emit(DaemonEvent::Show(Kind::Settings)),
                "break" => control::request_break_now(&ctx),
                "pause" => control::toggle_pause(&ctx),
                "quit" => ctx.emit(DaemonEvent::Quit),
                _ => {}
            }
        }));
    }
}

/// Decode the bundled tray icon PNG to RGBA8 for `tray_icon::Icon::from_rgba`.
fn load_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../../ui/src-tauri/icons/32x32.png");
    let mut decoder = png::Decoder::new(&bytes[..]);
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    if info.color_type != png::ColorType::Rgba {
        return Err(format!(
            "tray icon decoded as {:?}, expected Rgba",
            info.color_type
        )
        .into());
    }
    let rgba = buf[..info.buffer_size()].to_vec();
    Ok(Icon::from_rgba(rgba, info.width, info.height)?)
}
