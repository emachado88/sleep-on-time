//! System tray (StatusNotifierItem via ksni): menu, icons, user actions.
//!
//! Single source of truth is `crate::APP`; the `SleepTray` methods are a
//! stateless view ksni calls on its own runtime thread, and menu actions run
//! on detached threads (dialogs must never block the D-Bus service).

use crate::config::{self, FixedTime};
use crate::countdown::{self, Kind};
use crate::dialogs::{self, Prompt};
use crate::icons::ICONS;
use crate::{APP, HANDLE};
use ksni::menu::{MenuItem, StandardItem, SubMenu};
use ksni::{Icon, ToolTip, Tray};

pub struct SleepTray;

impl SleepTray {
    fn pixmap_for(&self, active: bool, dark: bool) -> Icon {
        let icon = if active {
            &ICONS.active
        } else if dark {
            &ICONS.dark
        } else {
            &ICONS.light
        };
        icon.clone()
    }
}

impl Tray for SleepTray {
    // Left click opens the menu: with this, Activate fails with an
    // "ItemIsMenu" UnknownMethod and hosts (Plasma, GNOME ext) open the menu
    // instead of calling `activate`.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "sleep-on-time".into()
    }

    fn title(&self) -> String {
        "Sleep on Time".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let app = APP.lock().unwrap();
        vec![self.pixmap_for(app.countdown.is_some(), app.dark_mode)]
    }

    fn tool_tip(&self) -> ToolTip {
        let app = APP.lock().unwrap();
        let description = match &app.countdown {
            Some(cd) => format!("Sleeping in {}", cd.remaining_compact()),
            None => "Put the computer to sleep on schedule".into(),
        };
        ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: "Sleep on Time".into(),
            description,
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let app = APP.lock().unwrap();

        let timer_current = if app.config.timer_minutes > 0 {
            countdown::format_compact(std::time::Duration::from_secs(
                u64::from(app.config.timer_minutes) * 60,
            ))
        } else {
            "not set".into()
        };
        let fixed_current = app
            .config
            .fixed
            .map(|f: FixedTime| f.format())
            .unwrap_or_else(|| "not set".into());

        let mut items: Vec<MenuItem<Self>> = vec![
            SubMenu {
                label: "Timer".into(),
                submenu: vec![
                    StandardItem {
                        label: format!("Set… (current: {timer_current})"),
                        activate: Box::new(|_| spawn_set_timer()),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: "Activate".into(),
                        enabled: app.config.timer_minutes > 0,
                        activate: Box::new(|_| start(Kind::Timer)),
                        ..Default::default()
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Fixed".into(),
                submenu: vec![
                    StandardItem {
                        label: format!("Set… (current: {fixed_current})"),
                        activate: Box::new(|_| spawn_set_fixed()),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: "Activate".into(),
                        enabled: app.config.fixed.is_some(),
                        activate: Box::new(|_| start(Kind::Fixed)),
                        ..Default::default()
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
        ];

        if let Some(cd) = &app.countdown {
            items.push(
                StandardItem {
                    label: format!("Remaining: {}", cd.remaining_compact()),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(
            StandardItem {
                label: "Cancel".into(),
                enabled: app.countdown.is_some(),
                activate: Box::new(|_| cancel()),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| exit()),
                ..Default::default()
            }
            .into(),
        );
        items
    }

    // Stay alive and re-register if the SNI watcher is not up yet
    // (e.g. autostart before the desktop session is fully ready).
    fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
        true
    }
}

// Menu actions: quick state touches happen inline under the APP lock;
// anything showing a dialog runs on a detached thread.

fn start(kind: Kind) {
    let mut app = APP.lock().unwrap();
    app.start_countdown(kind);
}

fn cancel() {
    APP.lock().unwrap().countdown = None;
}

fn exit() {
    // Menu/activate closures run on the ksni runtime thread: blocking it with
    // `shutdown().wait()` (a block_on) panics. Detach the wait; process exit
    // tears everything down regardless.
    if let Some(handle) = HANDLE.get() {
        let handle = handle.clone();
        std::thread::spawn(move || handle.shutdown().wait());
    }
    config::save(&APP.lock().unwrap().config);
    std::process::exit(0);
}

fn spawn_set_timer() {
    std::thread::spawn(|| {
        let current = APP.lock().unwrap().config.timer_minutes;
        match dialogs::prompt_timer_minutes(current) {
            Prompt::Value(mins) => {
                let mut app = APP.lock().unwrap();
                app.config.timer_minutes = mins;
                config::save(&app.config);
                app.start_countdown(Kind::Timer);
            }
            Prompt::Unavailable(reason) => {
                dialogs::error_dialog("Dialogs unavailable", &reason);
            }
            Prompt::Cancelled => {}
        }
    });
}

fn spawn_set_fixed() {
    std::thread::spawn(|| {
        let current = APP.lock().unwrap().config.fixed;
        match dialogs::prompt_fixed_time(current) {
            Prompt::Value(ft) => {
                let mut app = APP.lock().unwrap();
                app.config.fixed = Some(ft);
                config::save(&app.config);
                app.start_countdown(Kind::Fixed);
            }
            Prompt::Unavailable(reason) => {
                dialogs::error_dialog("Dialogs unavailable", &reason);
            }
            Prompt::Cancelled => {}
        }
    });
}
