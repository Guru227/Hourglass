#!/usr/bin/env bash
# Hourglass — remove the user-local install (binary, launcher, autostart, icons).
set -euo pipefail

echo ">> Stopping any running instance…"
pkill -x hourglass 2>/dev/null || true

rm -fv "$HOME/.local/bin/hourglass" \
       "$HOME/.local/share/applications/hourglass.desktop" \
       "$HOME/.config/autostart/hourglass.desktop"
for sz in 32x32 128x128 256x256 512x512; do
  rm -fv "$HOME/.local/share/icons/hicolor/$sz/apps/hourglass.png"
done

command -v update-desktop-database >/dev/null 2>&1 && \
  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true

echo ">> Removed. (Your settings at ~/.config/com.guru227.hourglass/ were left intact.)"
