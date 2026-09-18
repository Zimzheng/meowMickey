# Mickey Companion Tauri Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Swift/AppKit macOS desktop pet `MickeyCompanion` to a Tauri 2 (Rust + HTML/CSS/JS) project that builds and runs on macOS and Windows from one codebase, with a local packaging script that produces `.app` and `.exe` outputs in `dist/`.

**Architecture:** Tauri 2.x desktop app. Rust backend owns the timer, rules.json I/O, window position, system tray. Vanilla HTML/CSS/JS frontend owns sprite rendering and mouse interaction. They communicate via Tauri events (`emit`) and `invoke` commands. No frontend framework, no Tauri plugins.

**Tech Stack:** Tauri 2, Rust 1.75+, serde, serde_json, tokio, notify, vanilla HTML/CSS/JS. macOS only; Windows build is verified post-handoff.

**Spec:** `docs/superpowers/specs/2026-09-18-mickey-tauri-port-design.md` — the plan argues from the spec, so read both before starting.

---

## Global Constraints

These apply to every task below; tasks don't repeat them.

- **Project root:** `/Users/zimzhengbaidu/Developer/MickeyCompanion/`. Every path in this plan is relative to it unless stated.
- **Tauri version:** 2.x (latest stable). Cargo dependency: `tauri = "2"`, `tauri-build = "2"`.
- **No Tauri plugins.** No `tauri-plugin-*` dependencies. Use Rust std + system APIs.
- **Frontend stack:** vanilla HTML/CSS/JS. No `package.json`, no npm, no Vite, no framework.
- **Sprite PNGs:** exactly 1152×208 (6 frames × 192 wide × 208 tall). Source assets live in the original Codex worktree — copy, don't recreate.
- **`rules.json` schema (camelCase JSON):**
  ```json
  { "sneezeEveryMinutes": 30, "kneadEveryMinutes": 5, "singleClick": "kneading", "doubleClick": "sneezing" }
  ```
  Rust uses `#[serde(rename_all = "camelCase")]` so the Rust struct field `sneeze_every_minutes` matches `sneezeEveryMinutes` in JSON.
- **Bundle ID / product name:** `local.codex.mickey-companion` / `MickeyCompanion` (lowercase wsl path matches; display name in Chinese for menu UI only).
- **App data dir on macOS:** `~/Library/Application Support/local.codex.mickey-companion/`. Resolve via `app.path().app_data_dir()`.
- **Frame timings (seconds per frame):**
  - `idle`: `[0.28, 0.11, 0.11, 0.14, 0.14, 0.32]`
  - `sneezing`: `[0.18, 0.15, 0.12, 0.14, 0.17, 0.25]`
  - `kneading`: `[0.18, 0.18, 0.18, 0.18, 0.18, 0.24]`
- **Drag threshold:** 3 px (movement exceeding this on mouseup is treated as drag, not click).
- **Click timing:** 250 ms gap to distinguish single from double click.
- **Commit style:** short, imperative subject line. Use `chore:`, `feat:`, `test:`, `fix:` prefixes.

---

## Task 0: Project Bootstrap

**Files:**
- Create: `.gitignore`
- Create: `README.md` (placeholder; final version in Task 13)
- Create: empty directories: `src-tauri/`, `src-tauri/src/`, `src-tauri/resources/`, `src-tauri/icons/`, `src-tauri/capabilities/`, `ui/`, `scripts/`, `dist/`

**Interfaces:** None yet — this is pure scaffolding.

- [ ] **Step 1: Verify working directory and create dirs**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
mkdir -p src-tauri/src src-tauri/resources src-tauri/icons src-tauri/capabilities ui scripts dist
ls -la
```
Expected: all listed directories exist. `docs/` already exists from the spec phase.

- [ ] **Step 2: Write `.gitignore`**

Create `.gitignore` with this exact content:

```gitignore
# Rust
src-tauri/target/
**/*.rs.bk

# Tauri build output
src-tauri/gen/

# Distribution outputs
dist/

# OS
.DS_Store
Thumbs.db

# Editor
*.swp
*.swo
.vscode/
.idea/

# Logs
*.log
```

- [ ] **Step 3: Write README placeholder**

Create `README.md`:

```markdown
# Mickey Companion

A floating desktop pet. Tauri 2 (Rust + HTML/CSS/JS).

See `docs/superpowers/specs/2026-09-18-mickey-tauri-port-design.md` for the design.

## Quick Start

```bash
./scripts/dev.sh
```

## Build

```bash
./scripts/build.sh
```

Outputs land in `dist/<os>/`.
```

- [ ] **Step 4: Initialize git and commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git init
git add .gitignore README.md docs/
git commit -m "chore: bootstrap project, add spec"
```

Expected: git repo initialized, two commits worth of tracked files (docs/ from earlier, plus this commit).

---

## Task 1: Cargo + Tauri Scaffolding

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`

**Interfaces:** Produces a Cargo workspace root under `src-tauri/`. Subsequent Rust tasks add modules under `src-tauri/src/`. Verify with `cargo check`.

- [ ] **Step 1: Write `src-tauri/Cargo.toml`**

```toml
[package]
name = "mickey-companion"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[lib]
name = "mickey_companion_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["time", "sync", "macros", "rt"] }
notify = "6"

[dev-dependencies]
tempfile = "3"
```

Note: we use both `lib` and a binary. The lib will hold modules with unit tests; `main.rs` is a thin entry.

- [ ] **Step 2: Write `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: Write `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "MickeyCompanion",
  "version": "1.0.0",
  "identifier": "local.codex.mickey-companion",
  "build": {
    "frontendDist": "../ui"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "米奇桌宠",
        "width": 192,
        "height": 208,
        "resizable": false,
        "maximizable": false,
        "minimizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "visibleOnAllWorkspaces": true,
        "focus": false,
        "shadow": false
      }
    ],
    "security": {
      "csp": "default-src 'self' tauri:; img-src 'self' tauri: data: asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["app", "msi", "nsis"],
    "icon": [
      "icons/icon.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "resources": [
      "resources/rules.json",
      "resources/idle.png",
      "resources/kneading.png",
      "resources/sneezing.png"
    ],
    "category": "Entertainment",
    "shortDescription": "Floating desktop cat",
    "longDescription": "A transparent floating sprite that performs idle animations, sneezing, and kneading on a timer."
  }
}
```

- [ ] **Step 4: Write `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capabilities for the main floating window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-set-position",
    "core:window:allow-set-size",
    "core:window:allow-start-dragging",
    "core:event:default",
    "core:event:allow-emit",
    "core:event:allow-listen",
    "core:tray:default",
    "core:menu:default",
    "core:app:default"
  ]
}
```

- [ ] **Step 5: Add a placeholder `src-tauri/src/lib.rs`**

Create `src-tauri/src/lib.rs`:

```rust
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert_eq!(version(), "0.1.0");
    }
}
```

This exists so `cargo check` has something to compile. `main.rs` is added in Task 6.

- [ ] **Step 6: Verify `cargo check` succeeds**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo check 2>&1 | tail -30
```

Expected: ends with `Finished ... profile [unoptimized + debuginfo] target(s)`. First run downloads crates; later runs are fast. If errors mention missing icon files, that's fine — Tauri's bundler config references them but `cargo check` doesn't validate icon presence yet. Real error would be a parse/typo issue in the configs.

- [ ] **Step 7: Run the placeholder test**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib version_is_set 2>&1 | tail -15
```

Expected: `1 passed`.

- [ ] **Step 8: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add src-tauri/
git commit -m "feat: scaffold Tauri 2 project with cargo + tauri.conf"
```

---

## Task 2: Asset Migration

**Files:**
- Create: `src-tauri/resources/idle.png`
- Create: `src-tauri/resources/kneading.png`
- Create: `src-tauri/resources/sneezing.png`
- Create: `src-tauri/resources/rules.json`
- Create: `rules.json` (at project root)
- Create: `src-tauri/icons/icon.png` (placeholder; replace in Task 11 if user provides)
- Create: `src-tauri/icons/icon.icns` (placeholder)
- Create: `src-tauri/icons/icon.ico` (placeholder)
- Create: `src-tauri/icons/tray.png` (placeholder)

**Interfaces:** Produces the asset tree that `tauri.conf.json` references and the runtime reads. Verify with `file` and `sips` (macOS).

- [ ] **Step 1: Copy the three sprite PNGs from the original Codex worktree**

```bash
SRC="/Users/zimzhengbaidu/Documents/Codex/2026-09-17/hatch-pet-users-zimzhengbaidu-codex-skills-2/work/mickey/overlay"
DST="/Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri/resources"
cp "$SRC/../outputs/mickey-blue/MickeyCompanion.app/Contents/Resources/idle.png" "$DST/idle.png"
cp "$SRC/../outputs/mickey-blue/MickeyCompanion.app/Contents/Resources/kneading.png" "$DST/kneading.png"
cp "$SRC/../outputs/mickey-blue/MickeyCompanion.app/Contents/Resources/sneezing.png" "$DST/sneezing.png"
ls -la "$DST"
```

Expected: three PNGs, each ~200–320 KB. The `outputs/mickey-blue/MickeyCompanion.app/Contents/Resources/` path was confirmed during exploration; the `overlay` source folder has only `Info.plist` and source, not the sprites.

If that path doesn't exist, fall back to:
```bash
find /Users/zimzhengbaidu/Documents/Codex -name "idle.png" -path "*Resources*" 2>/dev/null
```
…and use whatever path returns valid PNGs.

- [ ] **Step 2: Verify PNG dimensions match 1152×208**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri/resources
for f in idle.png kneading.png sneezing.png; do
  sips -g pixelWidth -g pixelHeight "$f" | tail -2
done
```

Expected for each: `pixelWidth: 1152` and `pixelHeight: 208`. If any fails, stop — the rest of the plan assumes this exact size.

- [ ] **Step 3: Create `src-tauri/resources/rules.json` (bundled fallback)**

Write to `src-tauri/resources/rules.json`:

```json
{
  "sneezeEveryMinutes": 30,
  "kneadEveryMinutes": 5,
  "singleClick": "kneading",
  "doubleClick": "sneezing"
}
```

- [ ] **Step 4: Create project-root `rules.json`**

Same content, written to `/Users/zimzhengbaidu/Developer/MickeyCompanion/rules.json`. Used as the editable copy and copied into the bundle output in Task 11.

- [ ] **Step 5: Generate placeholder icons**

For the bundle to build without errors, `tauri.conf.json` references `icons/icon.png`, `icons/icon.icns`, `icons/icon.ico`. The simplest way to get all three is `cargo tauri icon` after we have a source PNG. Since we don't yet have a user-provided source, use this one-liner to create a 1024×1024 placeholder:

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
# Reuse idle.png as a placeholder source (any 1024-ish square works for tauri icon)
# If we don't have a 1024 source, cargo tauri icon will resize what we give it.
cp src-tauri/resources/idle.png src-tauri/icons/icon-source.png
```

Then in Task 11 we'll run `cargo tauri icon src-tauri/icons/icon-source.png` to generate the full icon set. For now, just confirm the directory exists:

```bash
mkdir -p src-tauri/icons
ls src-tauri/icons/
```

The actual `icon.png`, `icon.icns`, `icon.ico` files are produced by `cargo tauri icon` in Task 11. **Do not commit empty icons to git yet** — they'll be added with the Task 11 commit.

- [ ] **Step 6: Commit resources**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add src-tauri/resources/ rules.json
git commit -m "feat: add sprite PNGs and rules.json"
```

---

## Task 3: config.rs (TDD)

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs` (re-export `config` module)

**Interfaces:**
- Consumes: nothing (pure module)
- Produces:
  ```rust
  pub struct Rules {
      pub sneeze_every_minutes: f64,
      pub knead_every_minutes: f64,
      pub single_click: String,
      pub double_click: String,
  }
  impl Rules {
      pub fn default_rules() -> Self;          // hardcoded defaults
      pub fn from_json(s: &str) -> Result<Self, serde_json::Error>;
      pub fn action_for_single_click(&self) -> Action;
      pub fn action_for_double_click(&self) -> Action;
  }
  pub enum Action { Idle, Sneezing, Kneading }
  impl Action {
      pub fn as_str(&self) -> &'static str;
      pub fn from_name(s: &str) -> Option<Self>;
  }
  pub fn load_rules_from_paths(external: &Path, bundled: &Path) -> Rules;
  ```

The three-tier load priority: external file → bundled file → defaults. Test with `tempfile`.

- [ ] **Step 1: Write the failing test file `src-tauri/src/config.rs`**

Create `src-tauri/src/config.rs` with tests first, no implementation:

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    pub sneeze_every_minutes: f64,
    pub knead_every_minutes: f64,
    pub single_click: String,
    pub double_click: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Idle,
    Sneezing,
    Kneading,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Idle => "idle",
            Action::Sneezing => "sneezing",
            Action::Kneading => "kneading",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Action::Idle),
            "sneezing" => Some(Action::Sneezing),
            "kneading" => Some(Action::Kneading),
            _ => None,
        }
    }
}

impl Rules {
    pub fn default_rules() -> Self {
        Self {
            sneeze_every_minutes: 30.0,
            knead_every_minutes: 5.0,
            single_click: "kneading".to_string(),
            double_click: "sneezing".to_string(),
        }
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn action_for_single_click(&self) -> Action {
        Action::from_name(&self.single_click).unwrap_or(Action::Kneading)
    }

    pub fn action_for_double_click(&self) -> Action {
        Action::from_name(&self.double_click).unwrap_or(Action::Sneezing)
    }
}

pub fn load_rules_from_paths(external: &Path, bundled: &Path) -> Rules {
    if let Ok(data) = std::fs::read_to_string(external) {
        if let Ok(rules) = Rules::from_json(&data) {
            return rules;
        }
    }
    if let Ok(data) = std::fs::read_to_string(bundled) {
        if let Ok(rules) = Rules::from_json(&data) {
            return rules;
        }
    }
    Rules::default_rules()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn default_rules_have_expected_values() {
        let r = Rules::default_rules();
        assert_eq!(r.sneeze_every_minutes, 30.0);
        assert_eq!(r.knead_every_minutes, 5.0);
        assert_eq!(r.single_click, "kneading");
        assert_eq!(r.double_click, "sneezing");
    }

    #[test]
    fn from_json_parses_camel_case() {
        let json = r#"{"sneezeEveryMinutes": 10, "kneadEveryMinutes": 2, "singleClick": "sneezing", "doubleClick": "kneading"}"#;
        let r = Rules::from_json(json).unwrap();
        assert_eq!(r.sneeze_every_minutes, 10.0);
        assert_eq!(r.knead_every_minutes, 2.0);
        assert_eq!(r.single_click, "sneezing");
    }

    #[test]
    fn from_json_rejects_invalid() {
        assert!(Rules::from_json("not json").is_err());
        assert!(Rules::from_json("{}").is_err()); // missing fields
    }

    #[test]
    fn action_strings_roundtrip() {
        assert_eq!(Action::Idle.as_str(), "idle");
        assert_eq!(Action::from_name("sneezing"), Some(Action::Sneezing));
        assert_eq!(Action::from_name("nope"), None);
    }

    #[test]
    fn click_helpers_fall_back_to_defaults_on_unknown_action() {
        let r = Rules {
            sneeze_every_minutes: 30.0,
            knead_every_minutes: 5.0,
            single_click: "unknown".to_string(),
            double_click: "unknown".to_string(),
        };
        assert_eq!(r.action_for_single_click(), Action::Kneading);
        assert_eq!(r.action_for_double_click(), Action::Sneezing);
    }

    #[test]
    fn load_prefers_external_over_bundled() {
        let dir = tempdir().unwrap();
        let external = dir.path().join("rules.json");
        let bundled = dir.path().join("bundled.json");
        let mut f = std::fs::File::create(&external).unwrap();
        writeln!(
            f,
            r#"{{"sneezeEveryMinutes": 1, "kneadEveryMinutes": 1, "singleClick": "sneezing", "doubleClick": "kneading"}}"#
        )
        .unwrap();
        let mut f = std::fs::File::create(&bundled).unwrap();
        writeln!(
            f,
            r#"{{"sneezeEveryMinutes": 99, "kneadEveryMinutes": 99, "singleClick": "kneading", "doubleClick": "sneezing"}}"#
        )
        .unwrap();
        let r = load_rules_from_paths(&external, &bundled);
        assert_eq!(r.sneeze_every_minutes, 1.0);
        assert_eq!(r.action_for_single_click(), Action::Sneezing);
    }

    #[test]
    fn load_falls_back_to_bundled_when_external_missing() {
        let dir = tempdir().unwrap();
        let external = dir.path().join("does-not-exist.json");
        let bundled = dir.path().join("bundled.json");
        let mut f = std::fs::File::create(&bundled).unwrap();
        writeln!(
            f,
            r#"{{"sneezeEveryMinutes": 7, "kneadEveryMinutes": 3, "singleClick": "kneading", "doubleClick": "sneezing"}}"#
        )
        .unwrap();
        let r = load_rules_from_paths(&external, &bundled);
        assert_eq!(r.sneeze_every_minutes, 7.0);
        assert_eq!(r.knead_every_minutes, 3.0);
    }

    #[test]
    fn load_falls_back_to_defaults_when_both_missing() {
        let dir = tempdir().unwrap();
        let external = dir.path().join("missing-a.json");
        let bundled = dir.path().join("missing-b.json");
        let r = load_rules_from_paths(&external, &bundled);
        assert_eq!(r, Rules::default_rules());
    }

    #[test]
    fn load_falls_back_when_external_is_malformed() {
        let dir = tempdir().unwrap();
        let external = dir.path().join("rules.json");
        let bundled = dir.path().join("bundled.json");
        std::fs::write(&external, "garbage { not json").unwrap();
        std::fs::write(
            &bundled,
            r#"{"sneezeEveryMinutes": 4, "kneadEveryMinutes": 4, "singleClick": "sneezing", "doubleClick": "kneading"}"#,
        )
        .unwrap();
        let r = load_rules_from_paths(&external, &bundled);
        assert_eq!(r.sneeze_every_minutes, 4.0);
    }
}
```

- [ ] **Step 2: Re-export from `lib.rs`**

Edit `src-tauri/src/lib.rs` — add `pub mod config;` at the top, after the version function:

```rust
pub mod config;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert_eq!(version(), "0.1.0");
    }
}
```

- [ ] **Step 3: Run tests — all should pass (we wrote impl and tests together)**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib config:: 2>&1 | tail -25
```

Expected: 9 passed (`default_rules_have_expected_values`, `from_json_parses_camel_case`, `from_json_rejects_invalid`, `action_strings_roundtrip`, `click_helpers_fall_back_to_defaults_on_unknown_action`, `load_prefers_external_over_bundled`, `load_falls_back_to_bundled_when_external_missing`, `load_falls_back_to_defaults_when_both_missing`, `load_falls_back_when_external_is_malformed`).

- [ ] **Step 4: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat: add config module with three-tier rules loading"
```

---

## Task 4: window_state.rs (TDD)

**Files:**
- Create: `src-tauri/src/window_state.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces:
  ```rust
  pub struct WindowPosition { pub x: f64, pub y: f64 }
  pub fn load(dir: &Path) -> Option<WindowPosition>;   // reads {dir}/window.json
  pub fn save(dir: &Path, pos: WindowPosition) -> std::io::Result<()>;
  pub fn default_position(screen_width: f64, screen_height: f64) -> WindowPosition;
  ```

The save path is `{dir}/window.json`. `dir` will be `app_data_dir()` in main.rs.

- [ ] **Step 1: Write `src-tauri/src/window_state.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WindowPosition {
    pub x: f64,
    pub y: f64,
}

const SPRITE_WIDTH: f64 = 192.0;
const SPRITE_HEIGHT: f64 = 208.0;
const MARGIN_RIGHT: f64 = 35.0;
const MARGIN_BOTTOM: f64 = 55.0;

pub fn default_position(screen_width: f64, screen_height: f64) -> WindowPosition {
    WindowPosition {
        x: screen_width - SPRITE_WIDTH - MARGIN_RIGHT,
        y: MARGIN_BOTTOM,
    }
}

pub fn load(dir: &Path) -> Option<WindowPosition> {
    let path = dir.join("window.json");
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save(dir: &Path, pos: WindowPosition) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join("window.json");
    let json = serde_json::to_string_pretty(&pos)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_position_is_bottom_right_with_margins() {
        let p = default_position(1920.0, 1080.0);
        assert_eq!(p.x, 1920.0 - 192.0 - 35.0);
        assert_eq!(p.y, 55.0);
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let pos = WindowPosition { x: 100.5, y: 200.25 };
        save(dir.path(), pos).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, pos);
    }

    #[test]
    fn save_creates_dir_if_missing() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a/b/c");
        save(&nested, WindowPosition { x: 1.0, y: 2.0 }).unwrap();
        assert!(nested.join("window.json").exists());
    }

    #[test]
    fn load_returns_none_on_malformed_json() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("window.json"), "{ broken").unwrap();
        assert!(load(dir.path()).is_none());
    }
}
```

- [ ] **Step 2: Add `pub mod window_state;` to `lib.rs`**

```rust
pub mod config;
pub mod window_state;

pub fn version() -> &'static str { ... }
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib window_state:: 2>&1 | tail -20
```

Expected: 5 passed.

- [ ] **Step 4: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add src-tauri/src/window_state.rs src-tauri/src/lib.rs
git commit -m "feat: add window_state module for position persistence"
```

---

## Task 5: timer.rs (TDD)

**Files:**
- Create: `src-tauri/src/timer.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `config::Rules`
- Produces:
  ```rust
  pub trait Clock: Send + Sync {
      fn now(&self) -> std::time::Duration;  // monotonic seconds since some origin
  }
  pub struct SystemClock;
  impl Clock for SystemClock { ... }
  pub struct ManualClock { now: std::sync::Mutex<std::time::Duration> }
  impl ManualClock { pub fn new(start: Duration) -> Self; pub fn advance(&self, secs: f64); }
  impl Clock for ManualClock { ... }
  pub struct Scheduler {
      rules: Rules,
      next_sneeze_at: Duration,
      next_knead_at: Duration,
      paused: bool,
  }
  impl Scheduler {
      pub fn new(rules: Rules, clock: &dyn Clock) -> Self;
      pub fn tick(&mut self, clock: &dyn Clock) -> Option<Action>;  // Some(action) when due
      pub fn trigger(&mut self, action: Action, clock: &dyn Clock);  // manual; resets timer
      pub fn paused(&self) -> bool;
      pub fn set_paused(&mut self, paused: bool);
      pub fn reload_rules(&mut self, rules: Rules, clock: &dyn Clock);
  }
  ```

The scheduler is pure logic over an injectable clock. The async `tokio` runtime driver lives in `main.rs` and uses `SystemClock`.

- [ ] **Step 1: Write `src-tauri/src/timer.rs`**

```rust
use crate::config::{Action, Rules};
use std::sync::Mutex;
use std::time::Duration;

pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        // Use monotonic time. std::time::Instant is monotonic but can't be exported as a Duration from epoch;
        // for production use we need the elapsed-since-start time. Using SystemTime is not monotonic.
        // The pragmatic choice: track an epoch inside Scheduler using SystemClock::now as a Duration
        // from UNIX_EPOCH is fine because we only compare within a single run.
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
    }
}

pub struct ManualClock {
    now: Mutex<Duration>,
}

impl ManualClock {
    pub fn new(start: Duration) -> Self {
        Self { now: Mutex::new(start) }
    }
    pub fn advance(&self, secs: f64) {
        let mut n = self.now.lock().unwrap();
        *n += Duration::from_secs_f64(secs);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}

pub struct Scheduler {
    rules: Rules,
    next_sneeze_at: Duration,
    next_knead_at: Duration,
    paused: bool,
}

impl Scheduler {
    pub fn new(rules: Rules, clock: &dyn Clock) -> Self {
        let now = clock.now();
        let sneeze_in = rules.sneeze_every_minutes.max(1.0) * 60.0;
        let knead_in = rules.knead_every_minutes.max(1.0) * 60.0;
        Self {
            rules,
            next_sneeze_at: now + Duration::from_secs_f64(sneeze_in),
            next_knead_at: now + Duration::from_secs_f64(knead_in),
            paused: false,
        }
    }

    pub fn tick(&mut self, clock: &dyn Clock) -> Option<Action> {
        if self.paused {
            return None;
        }
        let now = clock.now();
        if now >= self.next_sneeze_at {
            self.next_sneeze_at = now
                + Duration::from_secs_f64(self.rules.sneeze_every_minutes.max(1.0) * 60.0);
            Some(Action::Sneezing)
        } else if now >= self.next_knead_at {
            self.next_knead_at = now
                + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
            Some(Action::Kneading)
        } else {
            None
        }
    }

    pub fn trigger(&mut self, action: Action, clock: &dyn Clock) {
        if action == Action::Idle {
            return;
        }
        let now = clock.now();
        match action {
            Action::Sneezing => {
                self.next_sneeze_at = now
                    + Duration::from_secs_f64(self.rules.sneeze_every_minutes.max(1.0) * 60.0);
            }
            Action::Kneading => {
                self.next_knead_at = now
                    + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
            }
            Action::Idle => {}
        }
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn reload_rules(&mut self, rules: Rules, clock: &dyn Clock) {
        self.rules = rules;
        let now = clock.now();
        // After reloading, push next events out by at least one interval so the user
        // gets a fresh window after editing the rules.
        self.next_sneeze_at = now
            + Duration::from_secs_f64(self.rules.sneeze_every_minutes.max(1.0) * 60.0);
        self.next_knead_at = now
            + Duration::from_secs_f64(self.rules.knead_every_minutes.max(1.0) * 60.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rules;

    fn rules(min_sneeze: f64, min_knead: f64) -> Rules {
        Rules {
            sneeze_every_minutes: min_sneeze,
            knead_every_minutes: min_knead,
            single_click: "kneading".to_string(),
            double_click: "sneezing".to_string(),
        }
    }

    #[test]
    fn new_scheduler_does_not_fire_immediately() {
        let clock = ManualClock::new(Duration::from_secs(1_000_000));
        let s = Scheduler::new(rules(30.0, 5.0), &clock);
        let mut s = s;
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn fires_sneeze_after_sneeze_interval() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        // After exactly 60s (1 minute) we should get a sneeze.
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(Action::Sneezing));
        // Immediately after, no event (timer was reset).
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn fires_knead_after_knead_interval() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 1.0), &clock);
        clock.advance(60.0);
        assert_eq!(s.tick(&clock), Some(Action::Kneading));
    }

    #[test]
    fn sneeze_takes_priority_over_knead_when_both_due() {
        let clock = ManualClock::new(Duration::from_secs(0));
        // Both intervals are 1 minute.
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        clock.advance(120.0); // both are well overdue
        assert_eq!(s.tick(&clock), Some(Action::Sneezing));
    }

    #[test]
    fn paused_blocks_all_ticks() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(1.0, 1.0), &clock);
        s.set_paused(true);
        clock.advance(3600.0);
        assert!(s.tick(&clock).is_none());
        s.set_paused(false);
        clock.advance(3600.0);
        // Should now fire
        assert!(s.tick(&clock).is_some());
    }

    #[test]
    fn manual_trigger_pushes_next_event_out() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 5.0), &clock);
        // Manually trigger sneeze at t=10s; next sneeze should be t=10+30*60.
        clock.advance(10.0);
        s.trigger(Action::Sneezing, &clock);
        // At t=11s, no sneeze
        clock.advance(1.0);
        assert!(s.tick(&clock).is_none());
        // Jump to 30*60+10 = 1810s total
        clock.advance(30.0 * 60.0);
        assert_eq!(s.tick(&clock), Some(Action::Sneezing));
    }

    #[test]
    fn trigger_idle_is_a_noop() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 5.0), &clock);
        clock.advance(10.0);
        s.trigger(Action::Idle, &clock);
        // Schedule unchanged: at 11s no event
        clock.advance(1.0);
        assert!(s.tick(&clock).is_none());
    }

    #[test]
    fn reload_rules_resets_schedule() {
        let clock = ManualClock::new(Duration::from_secs(0));
        let mut s = Scheduler::new(rules(30.0, 5.0), &clock);
        clock.advance(60.0);
        // Without reload, knead should fire at t=60s (5 min interval set in test setup is wrong — actually 5 min from t=0 → t=300s)
        // Reload with 1-min knead interval.
        s.reload_rules(rules(30.0, 1.0), &clock);
        clock.advance(60.0); // total t=120s
        assert_eq!(s.tick(&clock), Some(Action::Kneading));
    }

    #[test]
    fn minimum_interval_is_one_minute() {
        let clock = ManualClock::new(Duration::from_secs(0));
        // 0 minutes should be treated as 1 minute floor, matching the spec.
        let mut s = Scheduler::new(rules(0.0, 0.0), &clock);
        clock.advance(60.0); // exactly 1 minute
        // Should fire (whichever has priority)
        assert!(s.tick(&clock).is_some());
    }
}
```

- [ ] **Step 2: Add `pub mod timer;` to `lib.rs`**

```rust
pub mod config;
pub mod timer;
pub mod window_state;

pub fn version() -> &'static str { ... }
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib timer:: 2>&1 | tail -25
```

Expected: 9 passed.

- [ ] **Step 4: Run the entire test suite to confirm nothing broke**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib 2>&1 | tail -10
```

Expected: 23+ passed (3 from earlier tasks + 9 config + 5 window_state + 9 timer; includes the `version_is_set` test).

- [ ] **Step 5: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add src-tauri/src/timer.rs src-tauri/src/lib.rs
git commit -m "feat: add timer scheduler with injectable clock"
```

---

## Task 6: main.rs Glue (Window + Tray + Commands)

**Files:**
- Modify: `src-tauri/src/lib.rs` (add `pub mod app;` and expose `run()`)
- Create: `src-tauri/src/app.rs`
- Create: `src-tauri/src/main.rs`

**Interfaces:**
- `app::run() -> tauri::Result<()>` — entry point used by `main.rs`.
- Tauri commands registered:
  - `get_rules() -> Rules`
  - `start_drag() -> Option<WindowPosition>`
  - `update_drag(x: f64, y: f64) -> ()`
  - `end_drag() -> ()`
  - `trigger_action(action: String) -> ()`
  - `show_context_menu() -> ()`
  - `quit() -> ()`
- Tauri events emitted (to window "main"):
  - `trigger` payload `{action: String}`
  - `rules_changed` payload `Rules`
  - `paused` payload `{paused: bool}`
- Tray menu items: `打喷嚏`, `踩奶`, `重新加载规则`, `打开规则文件`, `暂停/继续定时动作`, `退出米奇`.

The state shared across commands: `Arc<Mutex<AppState>>` where `AppState` holds the `Scheduler` and a `paused` flag. Tray callbacks and commands both lock and modify state.

- [ ] **Step 1: Add `pub mod app;` to `lib.rs` and create `app.rs` skeleton**

In `src-tauri/src/lib.rs`, change to:

```rust
pub mod app;
pub mod config;
pub mod timer;
pub mod window_state;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_is_set() { assert_eq!(version(), "0.1.0"); }
}
```

Create `src-tauri/src/app.rs`:

```rust
use crate::config::{load_rules_from_paths, Action, Rules};
use crate::timer::{Clock, Scheduler, SystemClock};
use crate::window_state::{self, WindowPosition};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WindowEvent};

const SPRITE_W: f64 = 192.0;
const SPRITE_H: f64 = 208.0;
const DRAG_THRESHOLD: f64 = 3.0;

pub struct AppState {
    pub scheduler: Scheduler,
    pub data_dir: PathBuf,
    pub external_rules_path: PathBuf,
    pub bundled_rules_path: PathBuf,
}

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Clone, Serialize)]
struct TriggerPayload {
    action: String,
}

#[derive(Clone, Serialize)]
struct PausedPayload {
    paused: bool,
}

pub fn run() -> tauri::Result<()> {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Resolve resource paths. Bundled resources live under app_data_dir on most platforms,
            // but Tauri exposes a dedicated resource_dir. Try both.
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("resolve app_data_dir");
            std::fs::create_dir_all(&data_dir).ok();

            let external_rules = locate_external_rules(&app_handle);
            let bundled_rules = app_handle
                .path()
                .resolve("rules.json", tauri::path::BaseDirectory::Resource)
                .unwrap_or_else(|_| data_dir.join("rules.json"));

            let rules = load_rules_from_paths(&external_rules, &bundled_rules);

            let scheduler = Scheduler::new(rules.clone(), clock.as_ref());

            let state = Arc::new(Mutex::new(AppState {
                scheduler,
                data_dir: data_dir.clone(),
                external_rules_path: external_rules,
                bundled_rules_path: bundled_rules,
            }));

            app.manage(state.clone());

            // Restore window position.
            if let Some(window) = app_handle.get_webview_window("main") {
                let pos = window_state::load(&data_dir)
                    .unwrap_or_else(|| {
                        // Default: bottom-right of primary monitor.
                        if let Some(monitor) = window.primary_monitor().ok().flatten() {
                            let sf = monitor.size();
                            WindowPosition {
                                x: sf.width as f64 - SPRITE_W - 35.0,
                                y: 55.0,
                            }
                        } else {
                            WindowPosition { x: 100.0, y: 100.0 }
                        }
                    });
                let _ = window.set_position(PhysicalPosition::new(pos.x, pos.y));
            }

            // Build tray menu.
            let menu = build_tray_menu(&app_handle)?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app_handle.default_window_icon().cloned().unwrap())
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    handle_menu_event(app, event);
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(menu) = tray.app_handle().tray_by_id("main-tray").and_then(|t| t.menu().cloned()) {
                            menu.popup(tray.app_handle().clone());
                        }
                    }
                })
                .build(app)?;

            // Spawn the rules.json file watcher. When the external file changes,
            // reload rules and emit `rules_changed` so the frontend can refresh.
            let app_handle_for_watcher = app_handle.clone();
            let state_for_watcher = state.clone();
            let watch_path = {
                let st = state_for_watcher.lock().unwrap();
                st.external_rules_path.clone()
            };
            let watch_dir = watch_path.parent().map(|p| p.to_path_buf());
            if let Some(dir) = watch_dir {
                tauri::async_runtime::spawn(async move {
                    use notify::{RecursiveMode, Watcher};
                    let app = app_handle_for_watcher.clone();
                    let state = state_for_watcher.clone();
                    let watch_path_str = watch_path.to_string_lossy().to_string();
                    let res = tauri::async_runtime::spawn_blocking(move || -> notify::Result<()> {
                        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
                        let mut watcher = notify::recommended_watcher(tx)?;
                        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
                        for ev in rx {
                            if let Ok(event) = ev {
                                let touched = event.paths.iter().any(|p| {
                                    p.to_string_lossy() == watch_path_str
                                });
                                if touched {
                                    let rules = {
                                        let st = state.lock().unwrap();
                                        load_rules_from_paths(&st.external_rules_path, &st.bundled_rules_path)
                                    };
                                    let _ = app.emit("rules_changed", rules);
                                }
                            }
                        }
                        Ok(())
                    }).await;
                    let _ = res;
                });
            }

            // Spawn the tick loop.
            let app_handle_for_tick = app_handle.clone();
            let state_for_tick = state.clone();
            let clock_for_tick = clock.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(33));
                loop {
                    interval.tick().await;
                    let mut st = state_for_tick.lock().unwrap();
                    if let Some(action) = st.scheduler.tick(clock_for_tick.as_ref()) {
                        drop(st);
                        let _ = app_handle_for_tick.emit(
                            "trigger",
                            TriggerPayload { action: action.as_str().to_string() },
                        );
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of quit so tray stays useful.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_rules,
            start_drag,
            update_drag,
            end_drag,
            trigger_action,
            show_context_menu,
            quit,
        ])
        .run(tauri::generate_context!())
}

fn locate_external_rules(_app: &AppHandle) -> PathBuf {
    // The external rules.json lives next to the executable (same convention as the Swift version).
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe.parent().map(|p| p.join("rules.json")).unwrap_or(PathBuf::from("rules.json"))
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let sneeze = MenuItem::with_id(app, "sneeze", "打喷嚏", true, None::<&str>)?;
    let knead = MenuItem::with_id(app, "knead", "踩奶", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let reload = MenuItem::with_id(app, "reload", "重新加载规则", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "打开规则文件", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "暂停／继续定时动作", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出米奇", true, None::<&str>)?;
    Menu::with_items(app, &[&sneeze, &knead, &sep1, &reload, &open, &pause, &sep2, &quit])
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    let state: tauri::State<SharedState> = app.state();
    match event.id().as_ref() {
        "sneeze" => {
            emit_trigger(app, "sneezing");
            let mut st = state.lock().unwrap();
            st.scheduler
                .trigger(crate::config::Action::Sneezing, &SystemClock);
        }
        "knead" => {
            emit_trigger(app, "kneading");
            let mut st = state.lock().unwrap();
            st.scheduler
                .trigger(crate::config::Action::Kneading, &SystemClock);
        }
        "reload" => {
            reload_rules_into(app);
        }
        "open" => {
            let st = state.lock().unwrap();
            let path = &st.external_rules_path;
            // Best-effort: open with the OS default handler.
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(path).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .args(&["/c", "start", "", path.to_str().unwrap_or("")])
                .spawn();
            #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
        "pause" => {
            let mut st = state.lock().unwrap();
            let new_paused = !st.scheduler.paused();
            st.scheduler.set_paused(new_paused);
            drop(st);
            let _ = app.emit("paused", PausedPayload { paused: new_paused });
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

fn emit_trigger(app: &AppHandle, action: &str) {
    let _ = app.emit("trigger", TriggerPayload { action: action.to_string() });
}

fn reload_rules_into(app: &AppHandle) {
    let state: tauri::State<SharedState> = app.state();
    let mut st = state.lock().unwrap();
    let rules = load_rules_from_paths(&st.external_rules_path, &st.bundled_rules_path);
    st.scheduler.reload_rules(rules.clone(), &SystemClock);
    drop(st);
    let _ = app.emit("rules_changed", rules);
}

// ---------- Commands ----------

#[tauri::command]
fn get_rules(state: tauri::State<SharedState>) -> Rules {
    let st = state.lock().unwrap();
    // We don't store the parsed Rules on AppState (only inside the Scheduler),
    // so reload from disk to return the current truth.
    let rules = load_rules_from_paths(&st.external_rules_path, &st.bundled_rules_path);
    rules
}

#[derive(Serialize, Clone, Copy)]
struct DragOrigin {
    x: f64,
    y: f64,
}

// Per-drag state stored in a OnceLock-equivalent (we use a Mutex<Option<...>> on a global).
use std::sync::OnceLock;
static DRAG_ORIGIN: OnceLock<Mutex<Option<DragOrigin>>> = OnceLock::new();

fn drag_cell() -> &'static Mutex<Option<DragOrigin>> {
    DRAG_ORIGIN.get_or_init(|| Mutex::new(None))
}

#[tauri::command]
fn start_drag(window: tauri::Window) -> DragOrigin {
    let pos = window.outer_position().unwrap_or_default();
    let origin = DragOrigin {
        x: pos.x as f64,
        y: pos.y as f64,
    };
    *drag_cell().lock().unwrap() = Some(origin);
    origin
}

#[tauri::command]
fn update_drag(window: tauri::Window, x: f64, y: f64) {
    let Some(origin) = *drag_cell().lock().unwrap() else { return };
    let _ = window.set_position(PhysicalPosition::new(
        origin.x + x,
        origin.y + y,
    ));
}

#[tauri::command]
fn end_drag(state: tauri::State<SharedState>, window: tauri::Window) {
    *drag_cell().lock().unwrap() = None;
    if let Ok(pos) = window.outer_position() {
        let st = state.lock().unwrap();
        let _ = window_state::save(&st.data_dir, WindowPosition {
            x: pos.x as f64,
            y: pos.y as f64,
        });
    }
}

#[tauri::command]
fn trigger_action(app: AppHandle, state: tauri::State<SharedState>, action: String) {
    let parsed = match Action::from_name(&action) {
        Some(a) => a,
        None => return,
    };
    if parsed == Action::Idle {
        return;
    }
    emit_trigger(&app, &action);
    let mut st = state.lock().unwrap();
    st.scheduler.trigger(parsed, &SystemClock);
}

#[tauri::command]
fn show_context_menu(app: AppHandle) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Some(menu) = tray.menu().cloned() {
            menu.popup(app);
        }
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}
```

- [ ] **Step 2: Create `src-tauri/src/main.rs`**

```rust
// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mickey_companion_lib::app::run().expect("error while running Mickey Companion");
}
```

- [ ] **Step 3: Verify `cargo check` succeeds**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo check 2>&1 | tail -30
```

Expected: ends with `Finished ...`. If errors mention missing tray icon or similar, that's a config issue — fix the `tauri.conf.json` icon path. If errors are about Tauri API changes (e.g., `tray_by_id` API drift), adjust method names; the surface is small and self-evident from compiler errors.

- [ ] **Step 4: Run all unit tests to ensure nothing broke**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib 2>&1 | tail -10
```

Expected: 23+ passed (same as Task 5).

- [ ] **Step 5: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add src-tauri/src/app.rs src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat: wire main.rs with window, tray, commands, tick loop"
```

---

## Task 7: Frontend HTML + CSS Skeleton

**Files:**
- Create: `ui/index.html`
- Create: `ui/styles.css`

**Interfaces:** A bare transparent window with a single `#sprite` div. Verifies the Rust→Webview pipeline is wired and transparent window settings work.

- [ ] **Step 1: Write `ui/index.html`**

```html
<!DOCTYPE html>
<html lang="zh">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>米奇桌宠</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div id="sprite"></div>
  <script type="module" src="main.js"></script>
</body>
</html>
```

- [ ] **Step 2: Write `ui/styles.css`**

```css
:root {
  --sprite-w: 192px;
  --sprite-h: 208px;
}

html, body {
  margin: 0;
  padding: 0;
  width: var(--sprite-w);
  height: var(--sprite-h);
  background: transparent;
  overflow: hidden;
  cursor: default;
  -webkit-user-select: none;
  user-select: none;
  -webkit-app-region: no-drag;
}

#sprite {
  width: var(--sprite-w);
  height: var(--sprite-h);
  background-repeat: no-repeat;
  background-position: 0 0;
  /* background-image is set from JS once the sprite loads */
}
```

- [ ] **Step 3: Verify by running dev mode**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo tauri dev 2>&1 | tail -30
```

Expected: window opens. You'll see a 192×208 transparent rectangle in the bottom-right. No sprite yet (no JS). Quit with Ctrl+C in the terminal.

If you see "Failed to load resource" errors in the terminal, that's expected for `main.js` (not written yet) — confirm the HTML renders by visually inspecting the window.

- [ ] **Step 4: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add ui/index.html ui/styles.css
git commit -m "feat: add frontend HTML and CSS skeleton"
```

---

## Task 8: sprite.js

**Files:**
- Create: `ui/sprite.js`

**Interfaces:** Exports `class SpriteSheet` and helper `loadSpriteSheet(action)` that returns the asset URL usable as a CSS `background-image`. Sprite frames have durations per frame:

```js
import { invoke } from '/__TAURI__/index.js';  // not used here; placeholder for Task 10
// (we don't actually use invoke here — we use convertFileSrc)

const SPRITE_W = 192;
const SPRITE_H = 208;
const FRAME_COUNT = 6;
const FRAME_DURATIONS = {
  idle:     [0.28, 0.11, 0.11, 0.14, 0.14, 0.32],
  sneezing: [0.18, 0.15, 0.12, 0.14, 0.17, 0.25],
  kneading: [0.18, 0.18, 0.18, 0.18, 0.18, 0.24],
};
```

```js
const tauriWindow = window.__TAURI__;
const convertFileSrc = tauriWindow?.core?.convertFileSrc || tauriWindow?.convertFileSrc;

function assetUrl(action) {
  if (convertFileSrc) {
    // Tauri 2 API: resolves a bundled resource to a usable URL.
    return convertFileSrc(`${action}.png`);
  }
  // Fallback for plain browser dev (won't actually load PNG, but won't crash).
  return `${action}.png`;
}

export class SpriteSheet {
  constructor(element) {
    this.element = element;
    this.action = 'idle';
    this.frameIndex = 0;
    this.timerId = null;
    this.urls = {
      idle: assetUrl('idle'),
      sneezing: assetUrl('sneezing'),
      kneading: assetUrl('kneading'),
    };
    // Preload images so switching actions is instant.
    this.images = {};
    for (const [name, url] of Object.entries(this.urls)) {
      const img = new Image();
      img.src = url;
      this.images[name] = img;
    }
  }

  setAction(action) {
    if (this.action === action) return;
    this.action = action;
    this.frameIndex = 0;
    this.element.style.backgroundImage = `url("${this.urls[action]}")`;
    this.render();
    this.scheduleNext();
  }

  render() {
    this.element.style.backgroundPosition = `-${this.frameIndex * SPRITE_W}px 0`;
  }

  scheduleNext() {
    if (this.timerId !== null) {
      clearTimeout(this.timerId);
      this.timerId = null;
    }
    const durations = FRAME_DURATIONS[this.action];
    const dur = durations[this.frameIndex] ?? 0.2;
    this.timerId = setTimeout(() => this.advance(), dur * 1000);
  }

  advance() {
    this.frameIndex += 1;
    if (this.frameIndex >= FRAME_COUNT) {
      // Idle loops forever; sneeze/knead return to idle after their sequence.
      if (this.action === 'idle') {
        this.frameIndex = 0;
      } else {
        this.setAction('idle');
        return;
      }
    }
    this.render();
    this.scheduleNext();
  }
}
```

- [ ] **Step 1: Write `ui/sprite.js`**

Place the above content (combined into one file) into `ui/sprite.js`. Strip the import line — Tauri 2's global `window.__TAURI__` is exposed automatically; no import needed.

Final `ui/sprite.js`:

```js
const SPRITE_W = 192;
const SPRITE_H = 208;
const FRAME_COUNT = 6;
const FRAME_DURATIONS = {
  idle:     [0.28, 0.11, 0.11, 0.14, 0.14, 0.32],
  sneezing: [0.18, 0.15, 0.12, 0.14, 0.17, 0.25],
  kneading: [0.18, 0.18, 0.18, 0.18, 0.18, 0.24],
};

const tauri = window.__TAURI__;
const convertFileSrc = tauri?.core?.convertFileSrc || tauri?.convertFileSrc;

function assetUrl(action) {
  if (convertFileSrc) return convertFileSrc(`${action}.png`);
  return `${action}.png`;
}

export class SpriteSheet {
  constructor(element) {
    this.element = element;
    this.action = 'idle';
    this.frameIndex = 0;
    this.timerId = null;
    this.urls = {
      idle: assetUrl('idle'),
      sneezing: assetUrl('sneezing'),
      kneading: assetUrl('kneading'),
    };
    this.preload();
    this.setAction('idle');
  }

  preload() {
    for (const url of Object.values(this.urls)) {
      const img = new Image();
      img.src = url;
    }
  }

  setAction(action) {
    if (!FRAME_DURATIONS[action]) return;
    if (this.timerId !== null) {
      clearTimeout(this.timerId);
      this.timerId = null;
    }
    this.action = action;
    this.frameIndex = 0;
    this.element.style.backgroundImage = `url("${this.urls[action]}")`;
    this.render();
    this.scheduleNext();
  }

  render() {
    this.element.style.backgroundPosition = `-${this.frameIndex * SPRITE_W}px 0`;
  }

  scheduleNext() {
    const durations = FRAME_DURATIONS[this.action];
    const dur = durations[this.frameIndex] ?? 0.2;
    this.timerId = setTimeout(() => this.advance(), dur * 1000);
  }

  advance() {
    this.timerId = null;
    this.frameIndex += 1;
    if (this.frameIndex >= FRAME_COUNT) {
      if (this.action === 'idle') {
        this.frameIndex = 0;
      } else {
        this.setAction('idle');
        return;
      }
    }
    this.render();
    this.scheduleNext();
  }
}
```

- [ ] **Step 2: Wire a minimal `main.js` placeholder so the sprite actually shows**

Create `ui/main.js` (will be expanded in Task 10):

```js
import { SpriteSheet } from './sprite.js';

const sprite = new SpriteSheet(document.getElementById('sprite'));
window.__mickey = { sprite };
```

- [ ] **Step 3: Verify in dev mode**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo tauri dev 2>&1 | tail -10
```

Expected: window opens, idle sprite animates (you'll see the cat breathing). Drag works as a default (browser drag of the div) — full mouse handling comes in Task 9. Quit with Ctrl+C.

If sprite doesn't appear, check the browser console (right-click the webview in dev mode → Inspect): likely a path issue with `convertFileSrc`. Tauri 2 paths: `convertFileSrc('idle.png')` resolves to `http://asset.localhost/...` based on `tauri.conf.json` `assetProtocol` config — if not set, the URL may be `tauri://localhost/...` instead. Confirm with the actual URL the browser sees. If the asset doesn't load, see Task 10 note on `assetProtocol` config.

- [ ] **Step 4: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add ui/sprite.js ui/main.js
git commit -m "feat: add sprite animation engine"
```

---

## Task 9: mouse.js

**Files:**
- Create: `ui/mouse.js`

**Interfaces:** Exports `installMouseHandling(element, onSingleClick, onDoubleClick, onRightClick)`. Internally uses Tauri `invoke` to drive window dragging.

- [ ] **Step 1: Write `ui/mouse.js`**

```js
const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const DRAG_THRESHOLD = 3;
const CLICK_DELAY_MS = 250;

export function installMouseHandling(element, { onSingleClick, onDoubleClick, onRightClick }) {
  let dragStartClient = null;
  let dragOrigin = null;
  let moved = false;
  let pendingSingleClick = null;

  element.addEventListener('mousedown', (event) => {
    if (event.button !== 0) return; // left button only for drag
    dragStartClient = { x: event.clientX, y: event.clientY };
    moved = false;
    if (invoke) {
      invoke('start_drag').then((origin) => {
        dragOrigin = origin;
      }).catch(() => { dragOrigin = null; });
    } else {
      dragOrigin = { x: 0, y: 0 };
    }
  });

  element.addEventListener('mousemove', (event) => {
    if (!dragStartClient) return;
    const dx = event.clientX - dragStartClient.x;
    const dy = event.clientY - dragStartClient.y;
    if (!moved && Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD) {
      moved = true;
    }
    if (moved && dragOrigin && invoke) {
      invoke('update_drag', { x: dx, y: dy }).catch(() => {});
    }
  });

  element.addEventListener('mouseup', (event) => {
    if (event.button !== 0) return;
    const wasDragging = moved;
    const wasClick = !moved;
    dragStartClient = null;
    dragOrigin = null;
    moved = false;

    if (invoke) {
      invoke('end_drag').catch(() => {});
    }

    if (wasDragging) return;

    if (event.detail >= 2) {
      // Double-click (browser detected via detail)
      if (pendingSingleClick) {
        clearTimeout(pendingSingleClick);
        pendingSingleClick = null;
      }
      onDoubleClick && onDoubleClick();
      return;
    }

    // Single click with manual double-click window
    if (pendingSingleClick) {
      clearTimeout(pendingSingleClick);
    }
    pendingSingleClick = setTimeout(() => {
      pendingSingleClick = null;
      onSingleClick && onSingleClick();
    }, CLICK_DELAY_MS);
  });

  element.addEventListener('contextmenu', (event) => {
    event.preventDefault();
    onRightClick && onRightClick();
  });
}
```

- [ ] **Step 2: Update `ui/main.js` to use mouse.js (still no events from Rust yet)**

Replace `ui/main.js`:

```js
import { SpriteSheet } from './sprite.js';
import { installMouseHandling } from './mouse.js';

const spriteElement = document.getElementById('sprite');
const sprite = new SpriteSheet(spriteElement);

installMouseHandling(spriteElement, {
  onSingleClick: () => sprite.setAction('kneading'),
  onDoubleClick: () => sprite.setAction('sneezing'),
  onRightClick: () => {
    const tauri = window.__TAURI__;
    const invoke = tauri?.core?.invoke || tauri?.invoke;
    if (invoke) invoke('show_context_menu').catch(() => {});
  },
});

window.__mickey = { sprite };
```

- [ ] **Step 3: Verify in dev mode**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo tauri dev 2>&1 | tail -10
```

Expected:
- Idle sprite animates
- Drag the sprite around the screen — window follows the cursor
- Single-click → kneading animation plays
- Double-click → sneezing animation plays
- Right-click → tray menu pops up
- Close the window via the tray "退出米奇" — app exits

If any of these don't work, check the JS console. Most likely failure modes:
- `invoke is undefined`: Tauri 2's global might be `window.__TAURI__.core.invoke`. Confirm by inspecting `window.__TAURI__` in dev tools.
- Drag doesn't move window: `start_drag` may have returned an undefined origin; check that `tauri::Window::outer_position` returns positive numbers.

- [ ] **Step 4: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add ui/mouse.js ui/main.js
git commit -m "feat: add mouse handling (drag, single/double click, right-click)"
```

---

## Task 10: main.js Wiring (Rust Events)

**Files:**
- Modify: `ui/main.js`

**Interfaces:** Subscribe to `trigger`, `rules_changed`, `paused` events from Rust. Fetch initial rules via `get_rules` on load.

- [ ] **Step 1: Replace `ui/main.js` with the full event-driven entry**

```js
import { SpriteSheet } from './sprite.js';
import { installMouseHandling } from './mouse.js';

const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const listen = tauri?.event?.listen || tauri?.listen;

const spriteElement = document.getElementById('sprite');
const sprite = new SpriteSheet(spriteElement);

let rules = {
  sneezeEveryMinutes: 30,
  kneadEveryMinutes: 5,
  singleClick: 'kneading',
  doubleClick: 'sneezing',
};

async function loadInitialRules() {
  if (!invoke) return;
  try {
    rules = await invoke('get_rules');
  } catch (e) {
    console.warn('get_rules failed; using defaults', e);
  }
}

function triggerAction(action) {
  if (!action || action === 'idle') return;
  sprite.setAction(action);
  if (invoke) {
    invoke('trigger_action', { action }).catch((e) => {
      console.warn('trigger_action failed', e);
    });
  }
}

installMouseHandling(spriteElement, {
  onSingleClick: () => triggerAction(rules.singleClick),
  onDoubleClick: () => triggerAction(rules.doubleClick),
  onRightClick: () => {
    if (invoke) invoke('show_context_menu').catch(() => {});
  },
});

(async function init() {
  await loadInitialRules();

  if (listen) {
    await listen('trigger', (event) => {
      const action = event.payload?.action;
      if (action) sprite.setAction(action);
    });
    await listen('rules_changed', (event) => {
      if (event.payload) rules = event.payload;
    });
    await listen('paused', (event) => {
      // Optional: show a "paused" indicator. For parity with Swift version we just keep
      // listening and let Rust stop emitting trigger events when paused.
      console.log('paused:', event.payload);
    });
  }
})();

window.__mickey = { sprite, get rules() { return rules; } };
```

- [ ] **Step 2: Verify in dev mode**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo tauri dev 2>&1 | tail -10
```

Expected:
- Single click triggers kneading, double click triggers sneezing — Rust acknowledges via `trigger_action` (no visible feedback but no errors in console)
- Tray menu "打喷嚏" triggers sneeze animation
- Tray menu "踩奶" triggers knead animation
- Edit `rules.json` (in the bundle output's `Resources/` folder, or external rules path), then tray "重新加载规则" — `rules_changed` event fires and rules update in JS
- Tray "暂停/继续定时动作" — `paused` event fires; further ticks don't trigger animations
- Close via tray menu "退出米奇" — app exits

If the Tauri 2 API globals aren't `window.__TAURI__.core.invoke` but instead something else (e.g., `window.__TAURI_INTERNALS__`), check the actual Tauri 2 docs. As of late 2025 the path is `window.__TAURI__.core.invoke` / `event.listen`; the older `window.__TAURI__.invoke` may still work in some 2.x versions.

- [ ] **Step 3: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add ui/main.js
git commit -m "feat: wire Rust events into frontend (trigger, rules_changed, paused)"
```

---

## Task 11: Build Scripts + Icon Generation

**Files:**
- Create: `scripts/dev.sh`
- Create: `scripts/build.sh`
- Create: `scripts/clean.sh`
- Create: `src-tauri/icons/icon.png` (generated)
- Create: `src-tauri/icons/icon.icns` (generated, macOS only — wrap in #[cfg])
- Create: `src-tauri/icons/icon.ico` (generated)
- Modify: `src-tauri/tauri.conf.json` (add `assetProtocol` if needed)

**Interfaces:** Three shell scripts that wrap `cargo tauri` commands.

- [ ] **Step 1: Generate icons**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
cargo install tauri-cli --version "^2" --locked 2>&1 | tail -3
cargo tauri icon src-tauri/icons/icon-source.png 2>&1 | tail -10
ls src-tauri/icons/
```

Expected: install of `tauri-cli` completes; `cargo tauri icon` produces `icon.png`, `icon.icns`, `icon.ico`, plus a bunch of platform-specific sizes in `icons/`. If `icon-source.png` is rejected (must be at least 1024×1024), generate one programmatically:

```bash
# Quick 1024x1024 placeholder:
python3 -c "
from PIL import Image
img = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
# Draw something simple — replace with your real icon later.
img.save('src-tauri/icons/icon-source.png')
"
```

(If you don't have Pillow, just open `icon-source.png` in Preview and resize to 1024×1024 manually.)

- [ ] **Step 2: Write `scripts/dev.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"
cargo tauri dev
```

Make executable: `chmod +x scripts/dev.sh`.

- [ ] **Step 3: Write `scripts/build.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

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
    cp "$ROOT/rules.json" "$OUT/MickeyCompanion.app/Contents/Resources/rules.json"
    cp "$ROOT/启动米奇.command" "$OUT/启动米奇.command"
    chmod +x "$OUT/启动米奇.command"
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
```

Make executable: `chmod +x scripts/build.sh`.

- [ ] **Step 4: Write `scripts/clean.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
rm -rf "$ROOT/src-tauri/target"
rm -rf "$ROOT/dist"
echo "cleaned target/ and dist/"
```

Make executable: `chmod +x scripts/clean.sh`.

- [ ] **Step 5: Write `启动米奇.command` and `启动米奇.bat` for distribution**

`启动米奇.command` (write to project root, replace existing):

```bash
#!/bin/zsh
APP_DIR="${0:A:h}"
exec "$APP_DIR/MickeyCompanion.app/Contents/MacOS/MickeyCompanion"
```

`启动米奇.bat` (write to project root):

```bat
@echo off
cd /d "%~dp0"
start "" MickeyCompanion.exe
```

- [ ] **Step 6: Run the build script and verify `.app` output**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
./scripts/build.sh 2>&1 | tail -30
```

Expected:
- Last lines show `>> output in /Users/zimzhengbaidu/Developer/MickeyCompanion/dist/macos/`
- `dist/macos/MickeyCompanion.app/Contents/MacOS/MickeyCompanion` exists
- `dist/macos/MickeyCompanion.app/Contents/Resources/{idle,kneading,sneezing}.png` exist
- `dist/macos/MickeyCompanion.app/Contents/Resources/rules.json` exists
- `dist/macos/启动米奇.command` exists and is executable

- [ ] **Step 7: Smoke-test the built `.app`**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
open dist/macos/MickeyCompanion.app
sleep 3
# Verify the process is running:
pgrep -lf MickeyCompanion
```

Expected: a MickeyCompanion process is listed. Quit it from the tray menu (right-click 🐾 → 退出米奇) or kill with `pkill MickeyCompanion` if needed.

- [ ] **Step 8: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add scripts/ 启动米奇.command 启动米奇.bat src-tauri/icons/
git commit -m "feat: add build scripts and generated icon set"
```

---

## Task 12: Manual Verification Against §8.3 Checklist

**Files:** None (verification task).

Run through every item in spec §8.3 and record the result.

- [ ] **Step 1: Start the built `.app`**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
./dist/macos/启动米奇.command
```

Wait 2 seconds. Confirm the sprite appears bottom-right of the main screen.

- [ ] **Step 2: Walk through the checklist**

Tick each as you confirm:

- [ ] 启动：图标出现在右下角
- [ ] 单击：踩奶动画播放
- [ ] 双击：喷嚏动画播放
- [ ] 拖动：图标跟随鼠标，松手后位置保留
- [ ] 重启后位置恢复（退出 → 启动）
- [ ] 编辑 `dist/macos/MickeyCompanion.app/Contents/Resources/rules.json` 的 `sneezeEveryMinutes` 为 `1`，菜单"重新加载规则" — 等 1 分钟自动喷嚏
- [ ] 菜单"暂停/继续定时动作" — 定时动作停止/恢复
- [ ] 关闭/打开窗口 — 应用不退出（托盘仍然可用）

If any item fails, **stop and debug before committing this task**. Use the systematic-debugging skill: gather evidence (logs, screenshots, JS console output), form a hypothesis, test it. Don't shotgun-fix.

- [ ] **Step 3: Capture build artifact size**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
du -sh dist/macos/MickeyCompanion.app
du -h dist/macos/MickeyCompanion.app/Contents/MacOS/MickeyCompanion
```

Record these in your handoff notes. A typical Tauri release .app is 5–10 MB total.

- [ ] **Step 4: No commit needed — verification only**

If any failures were fixed, commit those fixes as part of their respective module tasks.

---

## Task 13: Documentation

**Files:**
- Modify: `README.md`
- Create: `使用说明.md`

**Interfaces:** End-user docs (Chinese) + developer docs (English).

- [ ] **Step 1: Write `README.md` (developer guide)**

```markdown
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
```

- [ ] **Step 2: Write `使用说明.md` (Chinese user manual)**

```markdown
# 米奇悬浮桌宠（Tauri 版）

透明悬浮的桌宠，每 5 分钟踩奶一次、每 30 分钟打喷嚏一次。

## 启动

- macOS：双击 `启动米奇.command`
- Windows：双击 `启动米奇.bat`

## 交互

- 单击米奇：踩奶
- 双击米奇：打喷嚏
- 拖动米奇：移动窗口位置（位置会自动保存）
- 右键米奇：弹出菜单
- 菜单栏 🐾 图标的菜单：
  - 打喷嚏 — 立即播放
  - 踩奶 — 立即播放
  - 重新加载规则 — 重读 `rules.json`
  - 打开规则文件 — 用系统默认编辑器打开
  - 暂停／继续定时动作
  - 退出米奇

## 配置

编辑旁边的 `rules.json`，保存后在菜单栏选择"重新加载规则"：

```json
{
  "sneezeEveryMinutes": 30,
  "kneadEveryMinutes": 5,
  "singleClick": "kneading",
  "doubleClick": "sneezing"
}
```

- `sneezeEveryMinutes` / `kneadEveryMinutes`：间隔分钟数，最小 1
- `singleClick` / `doubleClick`：`sneezing` 或 `kneading`
```

- [ ] **Step 3: Commit**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git add README.md 使用说明.md
git commit -m "docs: add developer README and Chinese user manual"
```

---

## Task 14: Final Cleanup

**Files:**
- Possibly modify: any files with stray debug code, dead imports, or `unwrap`s in non-test code.

- [ ] **Step 1: Run `cargo clippy` and fix warnings**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo clippy --lib -- -D warnings 2>&1 | tail -40
```

Fix any warnings introduced by your code. Pre-existing Tauri framework warnings can be silenced with `#[allow(...)]` if they're spurious.

- [ ] **Step 2: Final test pass**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion/src-tauri
cargo test --lib 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 3: Final build pass**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
./scripts/build.sh 2>&1 | tail -10
```

Expected: clean build, artifacts in `dist/macos/`.

- [ ] **Step 4: Final commit (if any cleanup)**

```bash
cd /Users/zimzhengbaidu/Developer/MickeyCompanion
git status
# If anything changed:
git add -A
git commit -m "chore: clippy fixes and cleanup"
```

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-18-mickey-tauri-port.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration with isolated context.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

(Reminder: macOS-only verification is in scope. Windows build is exercised by you on a Windows machine using the same scripts.)
