# keepawake

A small cross-platform CLI to keep your machine awake by periodically poking the native OS APIs.

## Features
- macOS: uses `IOPMAssertionCreateWithName` (falls back to `caffeinate -du -t 60` if assertions fail).
- Windows: calls `SetThreadExecutionState` with continuous system and display requirements.
- Linux: runs `xdg-screensaver reset` in the background.
- Configurable interval and duration, optional daemon mode, and debug logging.

## Usage
```
cargo run -- --help
```

Common examples:
```
# Ping every 30 seconds (default) until stopped manually
cargo run -- --interval 30

# Stay awake for 10 minutes, pinging every 5 seconds
cargo run -- --interval 5 --duration 10

# Quiet background mode
cargo run -- --daemon

# With a tray icon (menu -> Quit)
cargo run -- --tray
```

Options:
- `--interval <seconds>`: call keep-awake every N seconds (default: 30, min: 1).
- `--duration <minutes>`: stop after N minutes (min: 1). Omit to run indefinitely.
- `--daemon`: suppress all output.
- `--debug`: print debug pings (suppressed in daemon mode).
- `--tray`: show a system tray icon with a Quit menu item (uses libappindicator on Linux).

## Building
```
cargo build --release
```

Run the resulting binary from `target/release/keepawake`. On Linux ensure `xdg-screensaver` is available. On macOS a fallback to `caffeinate` is attempted if the assertion API fails.
