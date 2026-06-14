#!/usr/bin/env bash
# Hourglass — user-local installer (no sudo).
#
# Builds the release binary and installs it under ~/.local, adds an app-menu
# launcher, and (by default) a login autostart entry so Hourglass starts with
# your session and lives in the system tray.
#
#   ./install.sh                 install + run on login
#   ./install.sh --no-autostart  install without the login autostart entry
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

AUTOSTART=1
for arg in "$@"; do
  case "$arg" in
    --no-autostart) AUTOSTART=0 ;;
    -h|--help) sed -n '2,11p' "$0"; exit 0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor"
AUTOSTART_DIR="$HOME/.config/autostart"
EXEC="$BIN_DIR/hourglass"

# --- toolchain check ---
if ! command -v cargo >/dev/null 2>&1; then
  # rustup installs here but may not be on PATH for non-login shells
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: 'cargo' not found. Install Rust from https://rustup.rs and re-run." >&2
  exit 1
fi

# --- build (release profile uses LTO — first build takes a few minutes) ---
echo ">> Building Hourglass (release)… this can take a few minutes."
if ! cargo build --release --manifest-path "$ROOT/src-tauri/Cargo.toml"; then
  cat >&2 <<'EOF'

Build failed. On Debian/Ubuntu you likely need the WebKitGTK build deps:

  sudo apt update
  sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
EOF
  exit 1
fi

BUILT="$ROOT/src-tauri/target/release/hourglass"
[ -x "$BUILT" ] || { echo "error: built binary not found at $BUILT" >&2; exit 1; }

# --- install binary + icons ---
echo ">> Installing to $BIN_DIR"
install -Dm755 "$BUILT" "$EXEC"
for sz in 32x32 128x128 256x256 512x512; do
  src="$ROOT/src-tauri/icons/${sz}.png"
  [ -f "$src" ] && install -Dm644 "$src" "$ICON_DIR/$sz/apps/hourglass.png"
done

# --- desktop entry (shared by app-menu launcher + autostart) ---
write_desktop() {
  cat > "$1" <<EOF
[Desktop Entry]
Type=Application
Name=Hourglass
GenericName=Break reminder
Comment=Retro fullscreen break-reminder timer
Exec=$EXEC
Icon=hourglass
Terminal=false
Categories=Utility;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF
}

mkdir -p "$APP_DIR"
write_desktop "$APP_DIR/hourglass.desktop"
echo ">> App-menu launcher: $APP_DIR/hourglass.desktop"

if [ "$AUTOSTART" -eq 1 ]; then
  mkdir -p "$AUTOSTART_DIR"
  write_desktop "$AUTOSTART_DIR/hourglass.desktop"
  echo ">> Autostart on login: $AUTOSTART_DIR/hourglass.desktop"
else
  echo ">> Skipped autostart (--no-autostart)."
fi

# --- best-effort cache refresh (harmless if missing) ---
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" 2>/dev/null || true
command -v gtk-update-icon-cache    >/dev/null 2>&1 && gtk-update-icon-cache "$ICON_DIR" 2>/dev/null || true

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo ">> Note: $BIN_DIR is not on your PATH (only matters for launching 'hourglass' from a terminal)." ;;
esac

echo ""
echo "Done. Launch it now with:  $EXEC"
[ "$AUTOSTART" -eq 1 ] && echo "It will also start automatically next time you log in."
echo "Uninstall any time with:   $ROOT/uninstall.sh"
