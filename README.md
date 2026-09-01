# Sleep on Time

A simple system tray application that puts your computer to sleep after a timer or at a fixed time.

Native Linux rewrite in Rust: the tray is a pure [StatusNotifierItem](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/) (via [ksni](https://crates.io/crates/ksni)) — no GUI toolkit — and dialogs are real desktop dialogs (`kdialog` on KDE Plasma, `zenity` fallback elsewhere).

## Features

- System tray icon with active/inactive states, rendered from embedded SVGs
- Automatic dark/light mode detection (freedesktop portal, KDE `kreadconfig` fallback)
- Remaining time in the tooltip and a disabled menu entry while counting down
- **Timer mode**: set minutes (1–999), auto-activates after setting
- **Fixed mode**: set `HH:MM`, schedules the next occurrence (today or tomorrow), auto-activates
- Invalid input is rejected with an error dialog and the prompt repeats until valid (Cancel always exits)
- Persists settings in `~/.config/sleep-on-time/config.json` (reads the old Go config, including its `fixed_time` datetime format)
- Cross-platform suspend: `systemctl suspend` with `loginctl` fallback on Linux

## Dependencies

Runtime: `kdialog` (KDE, optional but recommended) or `zenity` (fallback) for dialogs, `gdbus` (for theme detection; optional).

Build: Rust 1.85+ (edition 2024).

## Build

```bash
cargo build --release
```

### Install (Linux)

```bash
./install-linux.sh
```

Builds the release binary, installs it to `/usr/local/bin`, installs the desktop file and icons, and refreshes the icon caches.

## Usage

Run the executable. The app appears in your system tray (left-click toggles nothing; use the right-click menu):

- **Timer > Set…** — set the duration in minutes (1–999); activates immediately
- **Timer > Activate** — start the countdown with the stored duration
- **Fixed > Set…** — set the time of day (`HH:MM`, also accepts `9`, `9:5`, `0930`); activates immediately
- **Fixed > Activate** — start the countdown to the next occurrence of that time
- **Remaining: …** — informational entry shown while counting down
- **Cancel** — stop the active countdown
- **Exit** — quit

## Input validation

Both prompts re-prompt on invalid input: wrong types, out-of-range values (minutes must be 1–999, hours 0–23, minutes 0–59) show an error dialog and reopen with the previous entry. Cancel or closing the dialog aborts without changing anything.

## Configuration

`~/.config/sleep-on-time/config.json`:

```json
{
  "timer_minutes": 60,
  "fixed": { "hour": 23, "minute": 0 }
}
```

## Icons

SVG sources live in `assets/` (`icon-light`, `icon-dark`, `icon-active`) and are embedded in the binary at compile time — the installed app needs no external icon files. The system-installed copies are only for the desktop menu entry.
