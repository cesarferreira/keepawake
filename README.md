# keepawake

Cross-platform CLI that manages native OS keep-awake state. Shows a system tray icon with status + Quit by default, or runs headless with `--no-tray`.

## Requirements
- Rust toolchain
- Linux: `xdg-screensaver` and libappindicator/gtk runtime (tray is default; use `--no-tray` to skip)
- macOS: uses `IOPMAssertionCreateWithName` and falls back to a managed `caffeinate -d` process
- Windows: uses `SetThreadExecutionState`

## Quick start
```
# see flags
cargo run -- --help

# default: tray icon, keeping awake indefinitely
cargo run --

# headless (no tray icon)
cargo run -- --no-tray

# daily window (run between 9:00 and 17:00 local time)
cargo run -- --active-window "9am-5pm"

# custom interval/duration
cargo run -- --interval 5 --duration 10

# quiet headless
cargo run -- --no-tray --daemon
```

## Flags
- `--interval <seconds>`: refresh interval on platforms that require periodic keep-awake signals (default: 30, min: 1)
- `--duration <minutes>`: stop after N minutes (min: 1). Omit to run indefinitely
- `--active-window <start-end>`: daily window to stay awake, e.g. `9am-5pm` or `21:00-06:00`
- `--daemon`: suppress all output
- `--debug`: print debug pings (suppressed in daemon mode)
- `--tray`: show a system tray icon with status (interval, duration, debug) and a Quit item (uses libappindicator on Linux). Enabled by default.
- `--no-tray`: disable the system tray icon and run headless

## Tray mode notes
- Icon: static wake-spark mug sourced from `assets/tray.svg` and embedded as a pre-rendered 64px PNG; macOS renders it as a template icon. Tooltip/title reflect the current remaining time (e.g. `14min left`, `3h50 left`).
- Menu items: status rows (interval, daily window, debug), an `Activate for` submenu (until stopped or quick durations), a pause/resume toggle, and Quit. If a daily window is configured the menu also offers “Follow daily window”.
- On Linux, the icon may be hidden without libappindicator/gtk or if the desktop shell suppresses tray icons.

### Customizing the tray icon
- `assets/tray.svg` is the editable 24x24 source; `assets/tray.png` is the embedded 64px runtime asset.
- After changing the SVG, regenerate the PNG with `rsvg-convert -w 64 -h 64 -o assets/tray.png assets/tray.svg`.

## Build
```
cargo build --release
```

Run the resulting binary from `target/release/keepawake`. On macOS the tray starts once the event loop is running; on Linux ensure `xdg-screensaver` is present or a warning is printed.
