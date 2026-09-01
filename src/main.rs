//! Sleep on Time — system tray app that suspends the machine after a timer
//! or at a fixed time. Rust rewrite of the Go original.

mod config;
mod countdown;
mod dialogs;
mod icons;
mod theme;
mod tray;

use countdown::{Countdown, Kind};
use ksni::blocking::TrayMethods;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Application state. Single source of truth; shared via `APP` and guarded
/// by its mutex. The ksni tray object is a stateless view over this.
pub struct App {
    pub config: config::Config,
    pub countdown: Option<Countdown>,
    pub dark_mode: bool,
}

impl App {
    /// Begin counting down for `kind`, using the stored configuration.
    /// No-op when that mode was never configured.
    pub fn start_countdown(&mut self, kind: Kind) {
        let (deadline, deadline_mono) = match kind {
            Kind::Timer => {
                let mins = self.config.timer_minutes;
                if mins == 0 {
                    return;
                }
                let wait = Duration::from_secs(u64::from(mins) * 60);
                (
                    chrono::Local::now() + chrono::Duration::from_std(wait).expect("timer fits"),
                    std::time::Instant::now() + wait,
                )
            }
            Kind::Fixed => {
                let Some(fixed) = self.config.fixed else {
                    return;
                };
                countdown::next_occurrence(fixed.hour, fixed.minute)
            }
        };
        self.countdown = Some(Countdown {
            kind,
            deadline,
            deadline_mono,
        });
    }
}

pub static APP: Mutex<App> = Mutex::new(App {
    config: config::Config {
        timer_minutes: 0,
        fixed: None,
    },
    countdown: None,
    dark_mode: false,
});

pub static HANDLE: OnceLock<ksni::blocking::Handle<tray::SleepTray>> = OnceLock::new();

/// Outcome of the single-instance check.
enum InstanceGuard {
    /// We own the lock; keep the file alive for the process lifetime.
    Acquired(File),
    /// Another instance holds the lock.
    AlreadyRunning,
    /// Lock file couldn't be created/locked (unusual); caller decides whether
    /// to degrade gracefully.
    Unavailable(std::io::Error),
}

fn lock_file_path() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("sleep-on-time.lock")
}

/// Exclusive flock on a per-user lock file: the second instance detects the
/// held lock and exits. Kernel releases the lock when the owner dies, so
/// there are no stale locks to clean up.
fn acquire_single_instance() -> InstanceGuard {
    let path = lock_file_path();
    let file = match File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => return InstanceGuard::Unavailable(e),
    };
    match file.try_lock() {
        Ok(()) => InstanceGuard::Acquired(file),
        Err(_) => InstanceGuard::AlreadyRunning,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Single instance: hold an exclusive flock for the process lifetime.
    // Kernel-managed, so no stale locks after crashes.
    let _instance_lock = match acquire_single_instance() {
        InstanceGuard::Acquired(file) => Some(file),
        InstanceGuard::AlreadyRunning => {
            eprintln!("sleep-on-time: already running, exiting.");
            return Ok(());
        }
        InstanceGuard::Unavailable(e) => {
            eprintln!("sleep-on-time: single-instance lock unavailable ({e}); continuing.");
            None
        }
    };

    APP.lock().unwrap().config = config::load();
    APP.lock().unwrap().dark_mode = theme::detect_dark_mode();

    let handle = tray::SleepTray.assume_sni_available(true).spawn()?;
    if HANDLE.set(handle).is_err() {
        return Err("tray spawned more than once".into());
    }

    std::thread::spawn(tick_loop);

    // Park the main thread forever; the tray runs on ksni's own thread.
    loop {
        std::thread::park();
    }
}

/// One-second heartbeat: fires the countdown when due and refreshes the
/// dark-mode flag (every 5s, to keep the per-tick cost trivial).
fn tick_loop() {
    let mut last_theme_check = std::time::Instant::now() - Duration::from_secs(10);
    loop {
        std::thread::sleep(Duration::from_secs(1));

        if last_theme_check.elapsed() >= Duration::from_secs(5) {
            last_theme_check = std::time::Instant::now();
            let dark = theme::detect_dark_mode();
            APP.lock().unwrap().dark_mode = dark;
        }

        let (due, active) = {
            let mut app = APP.lock().unwrap();
            let due = app
                .countdown
                .map(|cd| cd.remaining().is_zero())
                .unwrap_or(false);
            if due {
                app.countdown = None;
            }
            (due, app.countdown.is_some())
        };
        // Refresh the tray (tooltip + menu's Remaining entry) every second,
        // while a countdown is running or just ended — the layout is static
        // in idle.
        if (due || active)
            && let Some(handle) = HANDLE.get()
        {
            handle.update(|_| {});
        }
        if due {
            countdown::suspend();
        }
    }
}
