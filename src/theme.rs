//! Dark-mode detection, Linux-first (freedesktop portal, then KDE fallbacks).

use std::process::Command;

pub fn detect_dark_mode() -> bool {
    if let Some(scheme) = portal_color_scheme() {
        // org.freedesktop.appearance color-scheme: 0 no-pref, 1 dark, 2 light.
        return scheme == 1;
    }
    if let Some(dark) = kde_theme_is_dark() {
        return dark;
    }
    std::env::var("GTK_THEME")
        .map(|t| t.to_lowercase().contains("dark"))
        .unwrap_or(false)
}

fn portal_color_scheme() -> Option<u32> {
    let out = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Settings.Read",
            "org.freedesktop.appearance",
            "color-scheme",
        ])
        .output()
        .ok()?;
    parse_gdbus_uint32(&String::from_utf8_lossy(&out.stdout))
}

/// Parse gdbus output like `(<<uint32 1>>,)` into `1`.
fn parse_gdbus_uint32(s: &str) -> Option<u32> {
    let idx = s.find("uint32")?;
    let rest = &s[idx + "uint32".len()..];
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn kde_theme_is_dark() -> Option<bool> {
    for kread in ["kreadconfig6", "kreadconfig5"] {
        if let Ok(out) = Command::new(kread)
            .args(["--group", "KDE", "--key", "Theme"])
            .output()
        {
            let theme = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if theme.contains("dark") {
                return Some(true);
            }
            if !theme.trim().is_empty() {
                return Some(false);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_portal_uint32_output() {
        assert_eq!(parse_gdbus_uint32("(<<uint32 1>>,)"), Some(1));
        assert_eq!(parse_gdbus_uint32("(<@uint32 2>,)"), Some(2));
        assert_eq!(parse_gdbus_uint32("garbage"), None);
        assert_eq!(parse_gdbus_uint32(""), None);
    }
}
