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
# Keep one signing identity across releases. Never regenerate this key per build.
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ] && [ -f "$ROOT/.local/updater.key" ]; then
  export TAURI_SIGNING_PRIVATE_KEY="$ROOT/.local/updater.key"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
fi
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  echo "Missing TAURI_SIGNING_PRIVATE_KEY; see docs/UPDATES.md" >&2
  exit 1
fi
cargo tauri build

OUT="$ROOT/dist/$PLATFORM"
if [ -d "$OUT" ]; then mv "$OUT" "$OUT.backup.$(date +%Y%m%d-%H%M%S)"; fi
mkdir -p "$OUT"

case "$PLATFORM" in
  macos)
    cp -R "target/release/bundle/macos/米奇.app" "$OUT/"
    # Keep the editable rules beside the signed bundle. Never mutate the .app
    # after Tauri has signed it.
    cp "$ROOT/rules.json" "$OUT/rules.json"
    cp "$ROOT/启动米奇.command" "$OUT/启动米奇.command"
    chmod +x "$OUT/启动米奇.command"
    # Tauri's local bundle can contain only a linker signature. Re-sign the
    # complete copied bundle so CodeResources matches the final app contents.
    # Release distribution can replace '-' with a Developer ID identity.
    codesign --force --deep --sign - "$OUT/米奇.app"
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

python3 "$ROOT/scripts/prepare-update.py" --out "$OUT"

echo ">> output in $OUT"
ls -la "$OUT"
