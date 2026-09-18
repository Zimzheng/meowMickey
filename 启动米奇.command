#!/bin/zsh
APP_DIR="${0:A:h}"
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
exec "$APP_DIR/MickeyCompanion.app/Contents/MacOS/MickeyCompanion"
