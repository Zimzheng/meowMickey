#!/usr/bin/env bash
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
cd "$(dirname "$0")/../src-tauri"
cargo tauri dev
