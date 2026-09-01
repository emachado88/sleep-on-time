//! Persisted settings: `~/.config/sleep-on-time/config.json`.

use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Timer mode duration in minutes. 0 = never set.
    #[serde(default)]
    pub timer_minutes: u32,
    /// Fixed mode time of day (hours 0-23, minutes 0-59). None = never set.
    #[serde(default)]
    pub fixed: Option<FixedTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedTime {
    pub hour: u32,
    pub minute: u32,
}

impl FixedTime {
    pub fn format(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sleep-on-time")
        .join("config.json")
}

pub fn load() -> Config {
    match std::fs::read_to_string(config_path()) {
        Ok(text) => parse_config_text(&text),
        Err(_) => Config::default(),
    }
}

/// Parse config text, tolerating the old Go-era shape (`fixed_time` stored as
/// an RFC3339 datetime) and unknown keys.
fn parse_config_text(text: &str) -> Config {
    #[derive(Deserialize)]
    struct Raw {
        #[serde(default)]
        timer_minutes: u32,
        #[serde(default)]
        fixed: Option<FixedTime>,
        /// Go original wrote the computed datetime, e.g.
        /// "2026-09-01T00:00:00+01:00". We only need the time of day.
        #[serde(default)]
        fixed_time: Option<String>,
    }

    let Ok(raw) = serde_json::from_str::<Raw>(text) else {
        return Config::default();
    };

    let fixed = raw.fixed.or_else(|| {
        raw.fixed_time
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| FixedTime {
                hour: dt.hour(),
                minute: dt.minute(),
            })
    });

    Config {
        timer_minutes: raw.timer_minutes,
        fixed,
    }
}

pub fn save(config: &Config) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(config)
        && let Err(e) = std::fs::write(path, json + "\n")
    {
        eprintln!("failed to save config: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_time_formats_zero_padded() {
        assert_eq!(FixedTime { hour: 9, minute: 5 }.format(), "09:05");
        assert_eq!(
            FixedTime {
                hour: 23,
                minute: 59
            }
            .format(),
            "23:59"
        );
    }

    #[test]
    fn migrates_go_era_config() {
        let cfg = parse_config_text(
            r#"{"timer_minutes": 60, "fixed_time": "2026-09-01T00:05:00+01:00"}"#,
        );
        assert_eq!(cfg.timer_minutes, 60);
        assert_eq!(cfg.fixed, Some(FixedTime { hour: 0, minute: 5 }));
    }

    #[test]
    fn parses_current_config_shape() {
        let cfg =
            parse_config_text(r#"{"timer_minutes": 45, "fixed": {"hour": 22, "minute": 30}}"#);
        assert_eq!(cfg.timer_minutes, 45);
        assert_eq!(
            cfg.fixed,
            Some(FixedTime {
                hour: 22,
                minute: 30
            })
        );
    }

    #[test]
    fn garbage_config_yields_defaults() {
        let cfg = parse_config_text("not json at all");
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn missing_config_yields_defaults() {
        // No filesystem access involved: struct-level defaults.
        assert_eq!(Config::default().timer_minutes, 0);
        assert!(Config::default().fixed.is_none());
    }
}
