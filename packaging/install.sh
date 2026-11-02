#!/bin/sh
# Install Elevenpad for the current user, no root required.
set -e
HERE=$(dirname "$(readlink -f "$0")")
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"

mkdir -p "$BIN" "$APPS" "$ICONS"
install -m 755 "$HERE/elevenpad" "$BIN/elevenpad"
install -m 644 "$HERE/elevenpad.desktop" "$APPS/elevenpad.desktop"
install -m 644 "$HERE/elevenpad.png" "$ICONS/elevenpad.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPS" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed to $BIN/elevenpad"
echo "Make sure $BIN is on PATH, then run: elevenpad"
