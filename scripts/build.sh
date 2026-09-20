#!/usr/bin/env bash
set -euo pipefail

[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/src-tauri"

OS="$(uname -s)"
case "$OS" in
  Darwin) PLATFORM=macos ;;
  Linux)  PLATFORM=linux ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM=windows ;;
  *) echo "unknown OS: $OS"; exit 1 ;;
esac

echo ">> building for $PLATFORM"
cargo tauri build

OUT="$ROOT/dist/$PLATFORM"
rm -rf "$OUT"
mkdir -p "$OUT"

case "$PLATFORM" in
  macos)
    cp -R "target/release/bundle/macos/MickeyCompanion.app" "$OUT/"
    # Keep the editable rules beside the signed bundle. Never mutate the .app
    # after Tauri has signed it.
    cp "$ROOT/rules.json" "$OUT/rules.json"
    cp "$ROOT/启动米奇.command" "$OUT/启动米奇.command"
    chmod +x "$OUT/启动米奇.command"
    # Tauri's local bundle can contain only a linker signature. Re-sign the
    # complete copied bundle so CodeResources matches the final app contents.
    # Release distribution can replace '-' with a Developer ID identity.
    codesign --force --deep --sign - "$OUT/MickeyCompanion.app"
    ;;
  windows)
    if [ -d "target/release/bundle/msi" ]; then
      cp "target/release/bundle/msi/"*.msi "$OUT/" || true
    fi
    if [ -d "target/release/bundle/nsis" ]; then
      cp "target/release/bundle/nsis/"*.exe "$OUT/" || true
    fi
    cp "$ROOT/rules.json" "$OUT/rules.json"
    cp "$ROOT/启动米奇.bat" "$OUT/启动米奇.bat"
    ;;
  linux)
    if [ -d "target/release/bundle/deb" ]; then
      cp "target/release/bundle/deb/"*.deb "$OUT/" || true
    fi
    if [ -d "target/release/bundle/appimage" ]; then
      cp "target/release/bundle/appimage/"*.AppImage "$OUT/" || true
    fi
    ;;
esac

echo ">> output in $OUT"
ls -la "$OUT"
