#!/usr/bin/env bash
# Hourglass — user-local installer (no sudo).
#
# Builds the daemon and UI binaries and installs them under ~/.local, adds an
# app-menu launcher, and (by default) a login autostart entry so hourglassd
# starts with your session.
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
EXEC="$BIN_DIR/hourglassd"

# --- toolchain check ---
if ! command -v cargo >/dev/null 2>&1; then
  # rustup installs here but may not be on PATH for non-login shells
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: 'cargo' not found. Install Rust from https://rustup.rs and re-run." >&2
  exit 1
fi

# --- stop any running instances ---
echo ">> Stopping any running instances…"
pkill -x hourglass || true
pkill -x hourglassd || true
pkill -x hourglass-ui || true

# --- build (release profile uses LTO — first build takes a few minutes) ---
echo ">> Building Hourglass (release)… this can take a few minutes."
if ! cargo build --release --manifest-path "$ROOT/Cargo.toml"; then
  cat >&2 <<'EOF'

Build failed. On Debian/Ubuntu you likely need the WebKitGTK build deps:

  sudo apt update
  sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev
EOF
  exit 1
fi

# --- locate the built binaries ---
# The build directory is not always in-tree: a shared build.target-dir
# (see ~/.cargo/config.toml) or CARGO_TARGET_DIR relocates it. Ask cargo where
# it actually wrote things, and fall back to the in-tree default.
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps \
  --manifest-path "$ROOT/Cargo.toml" 2>/dev/null \
  | grep -o '"target_directory":"[^"]*"' | head -1 | cut -d'"' -f4)"
BUILT_DAEMON="${TARGET_DIR:-$ROOT/target}/release/hourglassd"
BUILT_UI="${TARGET_DIR:-$ROOT/target}/release/hourglass-ui"
[ -x "$BUILT_DAEMON" ] || { echo "error: built daemon not found at $BUILT_DAEMON" >&2; exit 1; }
[ -x "$BUILT_UI" ] || { echo "error: built UI not found at $BUILT_UI" >&2; exit 1; }

# --- install binaries + icons ---
echo ">> Installing to $BIN_DIR"
install -Dm755 "$BUILT_DAEMON" "$BIN_DIR/hourglassd"
install -Dm755 "$BUILT_UI" "$BIN_DIR/hourglass-ui"
# Remove stale v0.6 single-binary from earlier version
rm -f "$BIN_DIR/hourglass"
for sz in 32x32 128x128 256x256 512x512; do
  src="$ROOT/ui/src-tauri/icons/${sz}.png"
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
echo "Done. The daemon is installed at $EXEC."
echo "It will run on login (unless --no-autostart was used) and spawn UI windows on demand."
[ "$AUTOSTART" -eq 1 ] && echo "The daemon will also start automatically next time you log in."
echo "Uninstall any time with:   $ROOT/uninstall.sh"
