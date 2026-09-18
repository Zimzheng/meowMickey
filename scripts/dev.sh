#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"
cargo tauri dev
