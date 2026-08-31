//! Native input dialogs on KDE Plasma: `kdialog` (real KDE/Qt dialogs) with a
//! `zenity` fallback. Includes validation loops so bad input re-prompts
//! instead of closing the dialog.

use std::process::Command;

/// Frontend used for dialogs; detected once at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    KDialog,
    Zenity,
}

pub static BACKEND: LazyLock<Backend> = LazyLock::new(detect);

use std::sync::LazyLock;

fn detect() -> Backend {
    if has("kdialog") {
        Backend::KDialog
    } else if has("zenity") {
        Backend::Zenity
    } else {
        // No dialog tool: let every call surface an error instead of crashing here.
        Backend::Zenity
    }
}

fn has(prog: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {prog} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Outcome of a dialog interaction.
pub enum Prompt<T> {
    /// User submitted a valid value.
    Value(T),
    /// User pressed Cancel / closed the dialog.
    Cancelled,
    /// No dialog backend available (or it failed).
    Unavailable(String),
}

pub fn error_dialog(title: &str, msg: &str) {
    let cmd = match *BACKEND {
        Backend::KDialog => {
            let mut c = Command::new("kdialog");
            c.args(["--error", msg, "--title", title]);
            c
        }
        Backend::Zenity => {
            let mut c = Command::new("zenity");
            c.args(["--error", "--text", msg, "--title", title]);
            c
        }
    };
    spawn_forget(cmd);
}

/// Spawn-and-forget for popups: reaps its own child so the long-lived tray
/// process never accumulates zombies. (Do NOT use a global SIGCHLD=SIG_IGN:
/// it makes `Command::output()` fail with ECHILD, and a SIG_DFL toggle around
/// `output()` lets those dialog children exit as zombies instead.)
fn spawn_forget(mut cmd: Command) {
    match cmd.spawn() {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => eprintln!("failed to spawn {:?}: {e}", cmd.get_program()),
    }
}

/// One text-entry dialog. Returns `Ok(Some(text))` on OK, `Ok(None)` on cancel,
/// `Err(reason)` when no backend is available.
fn entry(title: &str, label: &str, initial: &str) -> Result<Option<String>, String> {
    let (prog, args): (&str, Vec<String>) = match *BACKEND {
        Backend::KDialog => (
            "kdialog",
            vec![
                "--inputbox".into(),
                label.into(),
                initial.into(),
                "--title".into(),
                title.into(),
            ],
        ),
        Backend::Zenity => (
            "zenity",
            vec![
                "--entry".into(),
                format!("--entry-text={initial}"),
                format!("--text={label}"),
                format!("--title={title}"),
            ],
        ),
    };

    match Command::new(prog).args(&args).output() {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(Some(text))
        }
        Ok(_) => Ok(None), // non-zero exit = user cancelled
        Err(e) => Err(format!("failed to run {prog}: {e}")),
    }
}

/// Show a single text-entry dialog and validate until the input is valid.
///
/// * `validate` returns `Ok(value)` to accept, or `Err(message)` to show an
///   error dialog and re-prompt with the bad input.
pub fn validated_entry<T>(
    title: &str,
    label: &str,
    initial: &str,
    validate: impl Fn(&str) -> Result<T, String>,
) -> Prompt<T> {
    let mut current = initial.to_string();
    loop {
        let text = match entry(title, label, &current) {
            Ok(Some(t)) => t,
            Ok(None) => return Prompt::Cancelled,
            Err(reason) => return Prompt::Unavailable(reason),
        };
        match validate(&text) {
            Ok(value) => return Prompt::Value(value),
            Err(msg) => {
                error_dialog("Invalid input", &msg);
                current = text; // keep the bad input visible for correction
            }
        }
    }
}

pub fn parse_minutes(raw: &str) -> Result<u32, String> {
    let n: u32 = raw
        .trim()
        .parse()
        .map_err(|_| format!("\"{raw}\" is not a whole number."))?;
    if n == 0 || n > 999 {
        return Err("Duration must be between 1 and 999 minutes.".into());
    }
    Ok(n)
}

pub fn parse_hour(raw: &str) -> Result<u32, String> {
    let n: u32 = raw
        .trim()
        .parse()
        .map_err(|_| format!("\"{raw}\" is not a whole number."))?;
    if n > 23 {
        return Err("Hour must be between 0 and 23.".into());
    }
    Ok(n)
}

pub fn parse_minute(raw: &str) -> Result<u32, String> {
    let n: u32 = raw
        .trim()
        .parse()
        .map_err(|_| format!("\"{raw}\" is not a whole number."))?;
    if n > 59 {
        return Err("Minute must be between 0 and 59.".into());
    }
    Ok(n)
}

/// Ask for the timer duration (minutes, 1-999), re-prompting on invalid input.
pub fn prompt_timer_minutes(current: u32) -> Prompt<u32> {
    validated_entry(
        "Timer duration",
        "Sleep after how many minutes? (1-999)",
        &current.to_string(),
        parse_minutes,
    )
}

/// Ask for the fixed alarm time as a single `HH:MM` entry, re-prompting on
/// invalid input. Accepts `H`, `H:M`, `H:MM`, `HH:MM` (colon optional for
/// plain 4-digit input, e.g. `0930`).
pub fn prompt_fixed_time(
    current: Option<crate::config::FixedTime>,
) -> Prompt<crate::config::FixedTime> {
    let initial = current
        .map(|f| f.format())
        .unwrap_or_else(|| "23:00".to_string());
    validated_entry(
        "Fixed time",
        "Time to sleep (HH:MM, 24h)",
        &initial,
        parse_hhmm,
    )
}

/// Parse a time-of-day string: `9`, `9:5`, `9:05`, `09:05`, `0930`.
pub fn parse_hhmm(raw: &str) -> Result<crate::config::FixedTime, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("Empty input: enter a time as HH:MM.".into());
    }
    let (h, m) = if let Some((h, m)) = s.split_once(':') {
        (h, m)
    } else if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) {
        (&s[..2], &s[2..]) // "0930" -> "09", "30"
    } else {
        (s, "0") // "9" -> 09:00
    };

    let hour = parse_hour(h)?;
    let minute = parse_minute(m)?;
    Ok(crate::config::FixedTime { hour, minute })
}

pub fn notify(msg: &str) {
    let cmd = match *BACKEND {
        Backend::KDialog => {
            let mut c = Command::new("kdialog");
            c.args(["--passivepopup", msg, "5"]);
            c
        }
        Backend::Zenity => {
            let mut c = Command::new("zenity");
            c.args(["--notification", "--text", msg, "--timeout", "5"]);
            c
        }
    };
    spawn_forget(cmd);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FixedTime;

    #[test]
    fn minutes_accept_valid_range() {
        assert_eq!(parse_minutes("30"), Ok(30));
        assert_eq!(parse_minutes(" 999 "), Ok(999));
        assert!(parse_minutes("0").is_err());
        assert!(parse_minutes("1000").is_err());
        assert!(parse_minutes("-5").is_err());
        assert!(parse_minutes("abc").is_err());
        assert!(parse_minutes("").is_err());
    }

    #[test]
    fn hhmm_parses_common_shapes() {
        assert_eq!(
            parse_hhmm("23:15"),
            Ok(FixedTime {
                hour: 23,
                minute: 15
            })
        );
        assert_eq!(parse_hhmm("9:5"), Ok(FixedTime { hour: 9, minute: 5 }));
        assert_eq!(
            parse_hhmm("0930"),
            Ok(FixedTime {
                hour: 9,
                minute: 30
            })
        );
        assert_eq!(parse_hhmm("7"), Ok(FixedTime { hour: 7, minute: 0 }));
    }

    #[test]
    fn hhmm_rejects_out_of_range() {
        assert!(parse_hhmm("24:00").is_err());
        assert!(parse_hhmm("12:60").is_err());
        assert!(parse_hhmm("-1:00").is_err());
        assert!(parse_hhmm("ab:cd").is_err());
        assert!(parse_hhmm("").is_err());
        assert!(parse_hhmm("1:2:3").is_err());
    }

    // NOTE: no end-to-end test for `validated_entry` — it would open a real
    // dialog window during `cargo test` (zenity/kdialog exist on dev machines)
    // and hang the suite. The re-prompt loop is trivial; parsers are tested above.
}
