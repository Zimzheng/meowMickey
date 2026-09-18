#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
rm -rf "$ROOT/src-tauri/target"
rm -rf "$ROOT/dist"
echo "cleaned target/ and dist/"
