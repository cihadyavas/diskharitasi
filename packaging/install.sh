#!/bin/sh
# Per-user install (no sudo): ~/.local/bin + application menu entry + icon
set -e
cd "$(dirname "$0")"
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications" "$HOME/.local/share/icons/hicolor/scalable/apps"
install -m 755 diskharitasi "$HOME/.local/bin/diskharitasi"
sed "s|^Exec=diskharitasi|Exec=$HOME/.local/bin/diskharitasi|" diskharitasi.desktop > "$HOME/.local/share/applications/diskharitasi.desktop"
install -m 644 diskharitasi.svg "$HOME/.local/share/icons/hicolor/scalable/apps/diskharitasi.svg"
echo "Installed: $HOME/.local/bin/diskharitasi"
