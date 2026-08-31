//! Countdown state: deadline ticking, suspension trigger.

use crate::dialogs;
use chrono::{Duration, Local, Timelike};
use std::process::Command;
use std::time::{Duration as StdDuration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Timer,
    Fixed,
}

/// Active countdown. A single `Option` is the source of truth:
/// `Some` = countdown running, `None` = idle.
#[derive(Clone, Copy, Debug)]
pub struct Countdown {
    pub kind: Kind,
    /// System time when the countdown fires.
    pub deadline: chrono::DateTime<Local>,
    /// Monotonic instant matching `deadline` (immune to wall-clock jumps).
    pub deadline_mono: Instant,
}

impl Countdown {
    pub fn remaining(&self) -> StdDuration {
        self.deadline_mono.saturating_duration_since(Instant::now())
    }

    /// "1h30m45s", "45m20s", "20s", "0s".
    pub fn remaining_compact(&self) -> String {
        format_compact(self.remaining())
    }
}

pub fn format_compact(d: StdDuration) -> String {
    let total = d.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let mut out = String::new();
    if h > 0 {
        out.push_str(&format!("{h}h"));
    }
    if m > 0 {
        out.push_str(&format!("{m}m"));
    }
    if s > 0 || out.is_empty() {
        out.push_str(&format!("{s}s"));
    }
    out
}

/// Next wall-clock occurrence of `target` (today if still ahead, else tomorrow).
pub fn next_occurrence(hour: u32, minute: u32) -> (chrono::DateTime<Local>, Instant) {
    let mut target = Local::now()
        .with_hour(hour)
        .and_then(|t| t.with_minute(minute))
        .and_then(|t| t.with_second(0))
        .expect("validated hour/minute produce a valid datetime");
    if target <= Local::now() {
        target += Duration::hours(24);
    }
    let wait = (target - Local::now())
        .to_std()
        .unwrap_or(StdDuration::ZERO);
    (target, Instant::now() + wait)
}

/// Suspend the machine (Linux: systemctl -> loginctl fallback).
pub fn suspend() {
    let systemctl = Command::new("systemctl").arg("suspend").status();
    if !systemctl.map(|s| s.success()).unwrap_or(false) {
        let loginctl = Command::new("loginctl").arg("suspend").status();
        if let Err(e) = loginctl {
            eprintln!("failed to suspend via systemctl and loginctl: {e}");
            dialogs::notify("Sleep on Time: failed to suspend the system");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_format_drops_zero_components() {
        assert_eq!(format_compact(StdDuration::from_secs(5415)), "1h30m15s");
        assert_eq!(format_compact(StdDuration::from_secs(3000)), "50m");
        assert_eq!(format_compact(StdDuration::from_secs(45)), "45s");
        assert_eq!(format_compact(StdDuration::ZERO), "0s");
    }

    #[test]
    fn next_occurrence_is_future_and_matches_hm() {
        let (dt, _mono) = next_occurrence(8, 30);
        assert!(dt > Local::now());
        assert_eq!(dt.hour(), 8);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }
}
