use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

pub const SPRITE_WIDTH: f64 = 192.0;
#[allow(dead_code)] // reserved for future bottom-edge offset calculation
pub const SPRITE_HEIGHT: f64 = 208.0;
pub const MARGIN_RIGHT: f64 = 35.0;
pub const MARGIN_BOTTOM: f64 = 55.0;
pub const MIN_SCALE: f64 = 0.5;
pub const MAX_SCALE: f64 = 2.0;
pub const DEFAULT_SCALE: f64 = 1.0;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WindowPosition {
    pub x: f64,
    pub y: f64,
    #[serde(default = "default_scale")]
    pub scale: f64,
}

fn default_scale() -> f64 {
    DEFAULT_SCALE
}

pub fn default_position(screen_width: f64, _screen_height: f64) -> WindowPosition {
    WindowPosition {
        x: screen_width - SPRITE_WIDTH - MARGIN_RIGHT,
        y: MARGIN_BOTTOM,
        scale: DEFAULT_SCALE,
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
        assert_eq!(p.scale, 1.0);
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let pos = WindowPosition {
            x: 100.5,
            y: 200.25,
            scale: 1.5,
        };
        save(dir.path(), pos).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, pos);
    }

    #[test]
    fn save_creates_dir_if_missing() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a/b/c");
        save(
            &nested,
            WindowPosition {
                x: 1.0,
                y: 2.0,
                scale: 0.75,
            },
        )
        .unwrap();
        assert!(nested.join("window.json").exists());
    }

    #[test]
    fn load_returns_none_on_malformed_json() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("window.json"), "{ broken").unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn load_defaults_scale_when_missing_for_backward_compat() {
        let dir = tempdir().unwrap();
        // Old-format window.json without a scale field
        fs::write(
            dir.path().join("window.json"),
            r#"{"x": 100.0, "y": 200.0}"#,
        )
        .unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.x, 100.0);
        assert_eq!(loaded.y, 200.0);
        assert_eq!(loaded.scale, 1.0); // serde default
    }
}
