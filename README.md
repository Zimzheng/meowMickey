# Mickey Companion

A floating desktop pet that performs idle animations, sneezing, and kneading on a timer. Built with Tauri 2 (Rust + HTML/CSS/JS), runs on macOS and Windows.

## Architecture

- **Backend** (`src-tauri/`): Rust. Owns the timer, `rules.json` I/O, window position persistence, system tray, and the IPC bridge.
- **Frontend** (`ui/`): Vanilla HTML/CSS/JS. Owns sprite rendering and mouse handling. No build step.
- **Spec**: `docs/superpowers/specs/2026-09-18-mickey-tauri-port-design.md`
- **Plan**: `docs/superpowers/plans/2026-09-18-mickey-tauri-port.md`

## Requirements

- Rust 1.75+
- Node-free. No npm, no Vite.
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: MSVC build tools + WebView2 runtime (preinstalled on Win 10/11)

## Development

```bash
./scripts/dev.sh
```

This runs `cargo tauri dev` from `src-tauri/`. Hot-reloads both Rust and `ui/` files.

## Building

```bash
./scripts/build.sh
```

Outputs to `dist/<os>/`:
- macOS: `MickeyCompanion.app` + `启动米奇.command`
- Windows: `MickeyCompanion.exe` (NSIS installer) and/or `.msi` + `启动米奇.bat`

## Tests

```bash
cd src-tauri
cargo test --lib
```

Covers `config`, `window_state`, `timer` modules.

## Configuration

`rules.json` in the project root (or next to the executable in `dist/`):

```json
{
  "sneezeEveryMinutes": 30,
  "kneadEveryMinutes": 5,
  "singleClick": "kneading",
  "doubleClick": "sneezing"
}
```

Reading priority: external (next to exe) → bundled (inside app) → hardcoded default.

Reload at runtime via the tray menu "重新加载规则".

## Notes

- Not code-signed. macOS users may need to right-click → Open the first time.
- Not notarized. Internal use only — see spec §1.2.