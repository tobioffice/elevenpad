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

cp target/release/elevenpad AppDir/usr/bin/
strip AppDir/usr/bin/elevenpad
cp assets/icon.png AppDir/elevenpad.png
cp assets/icon-256.png AppDir/usr/share/icons/hicolor/256x256/apps/elevenpad.png
cp assets/icon.png AppDir/usr/share/icons/hicolor/512x512/apps/elevenpad.png

cat > AppDir/elevenpad.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Elevenpad
GenericName=Text Editor
Comment=Windows 11 Notepad reimplementation for Linux, in Rust
Exec=elevenpad %F
Icon=elevenpad
Terminal=false
Categories=Utility;TextEditor;
MimeType=text/plain;text/markdown;text/x-rust;text/x-python;text/x-csrc;text/x-chdr;text/x-c++src;text/x-c++hdr;text/x-java;text/x-go;application/json;application/x-shellscript;text/xml;text/css;text/html;text/x-yaml;application/toml;
StartupWMClass=elevenpad
EOF
cp AppDir/elevenpad.desktop AppDir/usr/share/applications/

cat > AppDir/AppRun <<'EOF'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
exec "$HERE/usr/bin/elevenpad" "$@"
EOF
chmod +x AppDir/AppRun

TOOL="${APPIMAGETOOL:-/tmp/appimagetool}"
if [ ! -x "$TOOL" ]; then
  curl -sL -o /tmp/appimagetool \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x /tmp/appimagetool
  TOOL=/tmp/appimagetool
fi

ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" AppDir "Elevenpad-$VERSION-x86_64.AppImage"
echo "built Elevenpad-$VERSION-x86_64.AppImage"
