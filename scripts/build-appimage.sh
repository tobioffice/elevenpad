#!/usr/bin/env bash
# Build the AppImage from a release build. Requires appimagetool on PATH or in /tmp.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
cargo build --release
python3 scripts/make-icon.py

rm -rf AppDir
mkdir -p AppDir/usr/bin AppDir/usr/share/applications \
         AppDir/usr/share/icons/hicolor/256x256/apps \
         AppDir/usr/share/icons/hicolor/512x512/apps

cp target/release/win11-notepad AppDir/usr/bin/
strip AppDir/usr/bin/win11-notepad
cp assets/icon.png AppDir/win11-notepad.png
cp assets/icon-256.png AppDir/usr/share/icons/hicolor/256x256/apps/win11-notepad.png
cp assets/icon.png AppDir/usr/share/icons/hicolor/512x512/apps/win11-notepad.png

cat > AppDir/win11-notepad.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Notepad
GenericName=Text Editor
Comment=Windows 11 Notepad, recreated in Rust
Exec=win11-notepad %F
Icon=win11-notepad
Terminal=false
Categories=Utility;TextEditor;
MimeType=text/plain;
StartupWMClass=win11-notepad
EOF
cp AppDir/win11-notepad.desktop AppDir/usr/share/applications/

cat > AppDir/AppRun <<'EOF'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
exec "$HERE/usr/bin/win11-notepad" "$@"
EOF
chmod +x AppDir/AppRun

TOOL="${APPIMAGETOOL:-/tmp/appimagetool}"
if [ ! -x "$TOOL" ]; then
  curl -sL -o /tmp/appimagetool \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x /tmp/appimagetool
  TOOL=/tmp/appimagetool
fi

ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" AppDir "Notepad-$VERSION-x86_64.AppImage"
echo "built Notepad-$VERSION-x86_64.AppImage"
