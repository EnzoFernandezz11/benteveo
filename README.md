# Benteveo

<p align="center">
  <img src="resources/benteveo-pixel.png" width="180" alt="Pixel-art Benteveo bird icon">
</p>

A local-first focus app for Linux. It combines continuous noise, a Pomodoro timer, and a task list in a lightweight, dark, keyboard-friendly terminal-inspired window.

No accounts, Internet connection, or telemetry required.

## Features

| Area | Included |
| --- | --- |
| Noise | White, pink, and brown noise; adjustable volume; keeps playing while you move between tabs. |
| Pomodoro | Classic (25/5), Deep Work (90/10), and Custom (1–180 min) profiles; start, pause, resume, skip, and reset controls. |
| Tasks | Create, edit, complete, reorder, delete, and clear completed tasks. |
| Data | Automatic local, atomic, versioned storage with recovery and backup for corrupted JSON files. |

Completed Pomodoro stages send a desktop notification when supported by the Linux environment. If the notification server or an audio device is unavailable, the app keeps working and shows a non-blocking in-app message.

Benteveo also adds a small pixel-art status icon to the system tray, with **Open Benteveo** and **Quit** actions. KDE and other desktops with StatusNotifier support show it natively. On stock GNOME, enable an AppIndicator/KStatusNotifierItem extension first; the desktop environment, not the app, decides its exact position on the panel.

## Quick start

With stable Rust installed:

```bash
git clone <REPOSITORY-URL> benteveo
cd benteveo
cargo run --release
```

The optimized binary is created at `target/release/benteveo`:

```bash
./target/release/benteveo
```

## Linux requirements

In addition to stable Rust, you need development libraries for windowing, audio, and D-Bus. On Debian/Ubuntu:

```bash
sudo apt install build-essential pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev libdbus-1-dev
```

Rust is best installed with [rustup](https://rustup.rs/):

```bash
rustup toolchain install stable
```

## Install for your user

After building a release binary:

```bash
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/256x256/apps
install -m 755 target/release/benteveo ~/.local/bin/benteveo
install -m 644 resources/benteveo.desktop ~/.local/share/applications/benteveo.desktop
install -m 644 resources/benteveo-pixel.png ~/.local/share/icons/hicolor/256x256/apps/benteveo-pixel.png
```

You can then open **Benteveo** from your application launcher or run `benteveo` in a terminal. The pixel-art great kiskadee icon above is the icon used by the Linux launcher.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+1` | Open Noise |
| `Ctrl+2` | Open Pomodoro |
| `Ctrl+3` | Open Tasks |
| `Ctrl+Q` | Quit the app |
| `Enter` | Add the typed task in the Tasks tab |

## Local data

Configuration is automatically stored in the standard Linux configuration directory—usually `~/.config/benteveo/config.json`. It includes preferences, Pomodoro sessions, and tasks. If the file is invalid, Benteveo keeps it as `config.json.bak` and restores safe defaults.

## Development and quality

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --release
```

The repository includes a GitHub Actions workflow that runs these checks on Linux.

## License

[MIT](LICENSE).
