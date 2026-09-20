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

pub fn default_physical_position(
    monitor_x: i32,
    monitor_y: i32,
    monitor_width: u32,
    _monitor_height: u32,
    scale_factor: f64,
    sprite_scale: f64,
) -> WindowPosition {
    WindowPosition {
        x: monitor_x as f64 + monitor_width as f64
            - (SPRITE_WIDTH * sprite_scale + MARGIN_RIGHT) * scale_factor,
        y: monitor_y as f64 + MARGIN_BOTTOM * scale_factor,
        scale: sprite_scale,
    }
}

pub fn clamp_to_monitor(
    position: WindowPosition,
    monitor_x: i32,
    monitor_y: i32,
    monitor_width: u32,
    monitor_height: u32,
    scale_factor: f64,
) -> WindowPosition {
    let width = SPRITE_WIDTH * position.scale * scale_factor;
    let height = SPRITE_HEIGHT * position.scale * scale_factor;
    let min_x = monitor_x as f64;
    let min_y = monitor_y as f64;
    let max_x = (min_x + monitor_width as f64 - width).max(min_x);
    let max_y = (min_y + monitor_height as f64 - height).max(min_y);
    WindowPosition {
        x: position.x.clamp(min_x, max_x),
        y: position.y.clamp(min_y, max_y),
        ..position
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

    #[test]
    fn physical_default_accounts_for_retina_scale_and_monitor_origin() {
        let p = default_physical_position(100, 200, 2880, 1800, 2.0, 1.0);
        assert_eq!(p.x, 100.0 + 2880.0 - (192.0 + 35.0) * 2.0);
        assert_eq!(p.y, 200.0 + 55.0 * 2.0);
    }

    #[test]
    fn saved_position_is_clamped_back_onto_the_monitor() {
        let p = clamp_to_monitor(
            WindowPosition { x: 5000.0, y: -500.0, scale: 2.0 },
            0, 0, 1920, 1080, 1.0,
        );
        assert_eq!(p.x, 1920.0 - 384.0);
        assert_eq!(p.y, 0.0);
    }
}
