<p align="center">
  <img src="assets/app-icon.png" width="128" alt="keepawake app icon">
</p>

<h1 align="center">keepawake</h1>

<p align="center">
  Keep your screen awake on macOS, Windows, and Linux — from the system tray or the terminal.
</p>

<p align="center">
  <a href="https://github.com/cesarferreira/keepawake/releases/latest"><img src="https://img.shields.io/github/v/release/cesarferreira/keepawake" alt="Latest release"></a>
  <a href="https://github.com/cesarferreira/keepawake/actions/workflows/ci.yml"><img src="https://github.com/cesarferreira/keepawake/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
</p>

`keepawake` is a small native utility that prevents your display from going to sleep. Leave it running from the tray, set a timer, follow a daily schedule, or use it as a headless CLI.

## Highlights

- Runs from the system tray by default
- Activates indefinitely, for a set duration, or until a chosen time
- Follows daily schedules, including windows that cross midnight
- Supports headless and quiet operation
- Uses platform-native keep-awake mechanisms

## Install

### Download a release

Download the binary for your platform from the [latest release](https://github.com/cesarferreira/keepawake/releases/latest):

- macOS: Apple silicon or Intel
- Windows: x86_64
- Linux: x86_64

On macOS and Linux, make the downloaded binary executable before running it. For example, with the Apple silicon build:

```sh
chmod +x keepawake-macos-aarch64
./keepawake-macos-aarch64
```

You can rename it to `keepawake` and move it somewhere on your `PATH` for easier access.

### Build from source

Install the stable [Rust toolchain](https://www.rust-lang.org/tools/install), then run:

```sh
cargo build --release
./target/release/keepawake
```

On Windows, the resulting binary is `target\release\keepawake.exe`.

On Linux, the following Ubuntu and Debian packages cover both building from source and running in tray mode:

```sh
sudo apt install libgtk-3-dev libayatana-appindicator3-dev libxdo-dev xdg-utils
```

## Usage

Start `keepawake` with no options to keep the display awake indefinitely and show the tray icon:

```sh
keepawake
```

### Run for a fixed duration

```sh
# Exit after 45 minutes
keepawake --duration 45
```

### Follow a daily schedule

```sh
# Stay awake from 9:00 to 17:00 local time
keepawake --active-window "9am-5pm"

# Overnight windows work too
keepawake --active-window "21:00-06:00"
```

The process stays open outside the active window and automatically resumes at the next start time.

### Run without the tray

```sh
keepawake --no-tray
```

### Suppress output

```sh
keepawake --no-tray --daemon
```

`--daemon` makes the process quiet; it does not detach it from the terminal or install a background service.

## Tray controls

The tray menu shows the current state and gives you quick control without restarting the app:

- **Activate for** — choose until stopped, a preset duration, or a time later today
- **Pause now / Resume now** — temporarily release or restore the keep-awake state
- **Follow daily window** — return to the configured schedule after a manual override
- **Quit** — release the keep-awake state and exit

The tray title and tooltip show the remaining time or the next scheduled start. Tray availability on Linux depends on AppIndicator support in the desktop environment.

## CLI reference

| Option | Description |
| --- | --- |
| `--duration <minutes>` | Exit after the given number of minutes. Omit it to run indefinitely. |
| `--active-window <start-end>` | Stay awake during a local-time window such as `9am-5pm` or `21:00-06:00`. |
| `--interval <seconds>` | Refresh the platform keep-awake state at this interval when required. Defaults to `30`. |
| `--no-tray` | Run without a system tray icon. |
| `--tray` | Explicitly enable the tray, which is already the default. |
| `--daemon` | Suppress standard and warning output without detaching the process. |
| `--debug` | Print state changes and refresh activity. Ignored when `--daemon` is set. |
| `-h`, `--help` | Show command help. |
| `-V`, `--version` | Show the installed version. |

Options can be combined. For example, this quietly follows an office-hours schedule without showing a tray icon:

```sh
keepawake --active-window "09:00-17:00" --no-tray --daemon
```

## Platform support

| Platform | Keep-awake mechanism | Notes |
| --- | --- | --- |
| macOS | `IOPMAssertionCreateWithName` | Falls back to a managed `caffeinate -d` process if the native assertion fails. |
| Windows | `SetThreadExecutionState` | Requests continuous system and display availability. |
| Linux | Periodic `xdg-screensaver reset` | Requires `xdg-screensaver`; tray mode also requires GTK and AppIndicator support. |

The keep-awake state is released when the process exits normally, when you pause it, or when the configured duration ends.

## Development

Run the same checks used by CI:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked
```

### Build the macOS app bundle

The included `Makefile` packages the release binary and icon as `KeepAwake.app`:

```sh
make bundle   # Create KeepAwake.app
make run      # Build and launch it
make install  # Copy it to /Applications
```

### Update the tray icon

`assets/tray.svg` is the editable source and `assets/tray.png` is embedded in the binary. After editing the SVG, regenerate the PNG with:

```sh
rsvg-convert -w 64 -h 64 -o assets/tray.png assets/tray.svg
```
