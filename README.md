# keepawake

Cross-platform CLI to keep your machine awake by periodically pinging native OS APIs. Shows a system tray icon with status + Quit by default, or runs headless with `--no-tray`.

## Requirements
- Rust toolchain
- Linux: `xdg-screensaver` and libappindicator/gtk runtime (tray is default; use `--no-tray` to skip)
- macOS: uses `IOPMAssertionCreateWithName` and falls back to `caffeinate -du -t 60`
- Windows: uses `SetThreadExecutionState`

## Quick start
```
# see flags
cargo run -- --help

# default: tray icon pinging every 30s indefinitely
cargo run --

# headless (no tray icon)
cargo run -- --no-tray

# custom interval/duration
cargo run -- --interval 5 --duration 10

# quiet headless
cargo run -- --no-tray --daemon
```

## Flags
- `--interval <seconds>`: call keep-awake every N seconds (default: 30, min: 1)
- `--duration <minutes>`: stop after N minutes (min: 1). Omit to run indefinitely
- `--daemon`: suppress all output
- `--debug`: print debug pings (suppressed in daemon mode)
- `--tray`: show a system tray icon with status (interval, duration, debug) and a Quit item (uses libappindicator on Linux). Enabled by default.
- `--no-tray`: disable the system tray icon and run headless

## Tray mode notes
- Icon: larger steaming cup rendered from `assets/tray.svg` / `assets/tray-animated.svg` (128px target); tooltip shows the current cadence (`every <interval>s`, optional duration).
- Menu items: read-only status rows (interval, duration, debug) plus a Quit action.
- Icon steam animates every ~2 seconds (using the animated SVG as reference).
- On Linux, the icon may be hidden without libappindicator/gtk or if the desktop shell suppresses tray icons.

### Customizing the tray icon
- `assets/tray.svg` / `assets/tray-animated.svg` in the repo match the rendered icon (24x24 viewBox, stroked cup + steam). The app rasterizes that shape to 128px and animates the steam.
- If you change the SVG geometry, update `build_icon_frames` in `src/tray.rs` to match your steam offsets. If SVG parsing fails, a simple fallback dot icon is used and a warning is printed (non-daemon).

## Build
```
cargo build --release
```

Run the resulting binary from `target/release/keepawake`. On macOS the tray starts once the event loop is running; on Linux ensure `xdg-screensaver` is present or a warning is printed.
