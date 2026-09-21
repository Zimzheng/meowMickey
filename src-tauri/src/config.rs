use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    pub sneeze_every_minutes: f64,
    pub knead_every_minutes: f64,
    pub single_click: String,
    pub double_click: String,
    #[serde(default = "default_true")]
    pub behavior_enabled: bool,
    #[serde(default = "default_cooldown_seconds")]
    pub action_cooldown_seconds: f64,
    #[serde(default = "default_work_break_minutes")]
    pub work_break_minutes: f64,
    #[serde(default = "default_true")]
    pub night_quiet_enabled: bool,
    #[serde(default = "default_night_start_hour")]
    pub night_start_hour: u8,
    #[serde(default = "default_night_end_hour")]
    pub night_end_hour: u8,
    #[serde(default = "default_night_yawn_minutes")]
    pub night_yawn_minutes: f64,
}

fn default_true() -> bool {
    true
}
fn default_cooldown_seconds() -> f64 {
    12.0
}
fn default_work_break_minutes() -> f64 {
    60.0
}
fn default_night_start_hour() -> u8 {
    23
}
fn default_night_end_hour() -> u8 {
    7
}
fn default_night_yawn_minutes() -> f64 {
    90.0
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Idle,
    Sneezing,
    Kneading,
    RubNose,
    Contented,
    HeadTilt,
    Yawning,
    Stretching,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Idle => "idle",
            Action::Sneezing => "sneezing",
            Action::Kneading => "kneading",
            Action::RubNose => "rubnose",
            Action::Contented => "contented",
            Action::HeadTilt => "headtilt",
            Action::Yawning => "yawning",
            Action::Stretching => "stretching",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Action::Idle),
            "sneezing" => Some(Action::Sneezing),
            "kneading" => Some(Action::Kneading),
            "rubnose" => Some(Action::RubNose),
            "contented" => Some(Action::Contented),
            "headtilt" => Some(Action::HeadTilt),
            "yawning" => Some(Action::Yawning),
            "stretching" => Some(Action::Stretching),
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
            behavior_enabled: true,
            action_cooldown_seconds: default_cooldown_seconds(),
            work_break_minutes: default_work_break_minutes(),
            night_quiet_enabled: true,
            night_start_hour: default_night_start_hour(),
            night_end_hour: default_night_end_hour(),
            night_yawn_minutes: default_night_yawn_minutes(),
        }
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Self>(s).map(Self::normalized)
    }

    pub fn normalized(mut self) -> Self {
        self.sneeze_every_minutes = valid_interval(self.sneeze_every_minutes, 30.0);
        self.knead_every_minutes = valid_interval(self.knead_every_minutes, 5.0);
        self.single_click = Action::from_name(&self.single_click)
            .filter(|action| matches!(action, Action::Sneezing | Action::Kneading))
            .unwrap_or(Action::Kneading)
            .as_str()
            .to_string();
        self.double_click = Action::from_name(&self.double_click)
            .filter(|action| matches!(action, Action::Sneezing | Action::Kneading))
            .unwrap_or(Action::Sneezing)
            .as_str()
            .to_string();
        self.action_cooldown_seconds = valid_seconds(self.action_cooldown_seconds, 12.0);
        self.work_break_minutes = valid_interval(self.work_break_minutes, 60.0);
        self.night_yawn_minutes = valid_interval(self.night_yawn_minutes, 90.0);
        self.night_start_hour = self.night_start_hour.min(23);
        self.night_end_hour = self.night_end_hour.min(23);
        self
    }

    pub fn action_for_single_click(&self) -> Action {
        Action::from_name(&self.single_click).unwrap_or(Action::Kneading)
    }

    pub fn action_for_double_click(&self) -> Action {
        Action::from_name(&self.double_click).unwrap_or(Action::Sneezing)
    }
}

fn valid_interval(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.max(1.0)
    } else {
        fallback
    }
}

fn valid_seconds(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(3.0, 300.0)
    } else {
        fallback
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
            ..Rules::default_rules()
        };
        assert_eq!(r.action_for_single_click(), Action::Kneading);
        assert_eq!(r.action_for_double_click(), Action::Sneezing);
    }

    #[test]
    fn parsing_normalizes_invalid_clicks_and_short_intervals() {
        let json = r#"{"sneezeEveryMinutes": 0, "kneadEveryMinutes": -2, "singleClick": "unknown", "doubleClick": "idle"}"#;
        let r = Rules::from_json(json).unwrap();
        assert_eq!(r.sneeze_every_minutes, 1.0);
        assert_eq!(r.knead_every_minutes, 1.0);
        assert_eq!(r.single_click, "kneading");
        assert_eq!(r.double_click, "sneezing");
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
