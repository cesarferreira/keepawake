# keepawake

Cross-platform CLI to keep your machine awake by periodically pinging native OS APIs. Runs headless or via a system tray with status + Quit.

## Requirements
- Rust toolchain
- Linux: `xdg-screensaver` and (for tray mode) libappindicator/gtk runtime
- macOS: uses `IOPMAssertionCreateWithName` and falls back to `caffeinate -du -t 60`
- Windows: uses `SetThreadExecutionState`

## Quick start
```
# see flags
cargo run -- --help

# default: ping every 30s indefinitely
cargo run --

# custom interval/duration
cargo run -- --interval 5 --duration 10

# quiet headless
cargo run -- --daemon

# tray mode with menu (shows interval/duration/debug + Quit)
cargo run -- --tray --interval 15 --duration 30
```

## Flags
- `--interval <seconds>`: call keep-awake every N seconds (default: 30, min: 1)
- `--duration <minutes>`: stop after N minutes (min: 1). Omit to run indefinitely
- `--daemon`: suppress all output
- `--debug`: print debug pings (suppressed in daemon mode)
- `--tray`: show a system tray icon with status (interval, duration, debug) and a Quit item (uses libappindicator on Linux)

## Tray mode notes
- Icon: small cup with steam; tooltip shows the current cadence (`every <interval>s`, optional duration).
- Menu items: read-only status rows (interval, duration, debug) plus a Quit action.
- On Linux, the icon may be hidden without libappindicator/gtk or if the desktop shell suppresses tray icons.

## Build
```
cargo build --release
```

Run the resulting binary from `target/release/keepawake`. On macOS the tray starts once the event loop is running; on Linux ensure `xdg-screensaver` is present or a warning is printed.
