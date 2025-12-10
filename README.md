# Claude Tray

A system tray application for monitoring Claude AI usage.

## Table of Contents

- [Description](#description)
- [Installation](#installation)
  - [Prerequisites](#prerequisites)
  - [Install from .deb Package](#install-from-deb-package)
  - [Build from Source](#build-from-source)
  - [Running](#running)
- [Systemd Service](#systemd-service)
  - [Starting the Service](#starting-the-service)
  - [Managing the Service](#managing-the-service)
  - [Auto-start on Login](#auto-start-on-login)
- [Troubleshooting](#troubleshooting)
  - [Service Not Starting](#service-not-starting)
  - [Display Issues](#display-issues)
  - [Uninstalling](#uninstalling)
- [Configuration](#configuration)
- [Building .deb Package](#building-deb-package)
- [Tested OS](#tested-os)
- [Contributing](#contributing)
- [License](#license)

## Description

Claude Tray is a Linux/Debian system tray utility that integrates with Claude AI's OAuth API. It provides quick access to Claude AI services and displays usage statistics directly from your system tray.

Features:
- System tray integration for quick access
- Claude AI authentication via OAuth
- Usage monitoring (5-hour and 7-day limits)
- Quick browser access to Claude AI
- Systemd service integration for automatic startup

## Installation

### Prerequisites

- Claude CLI installed and configured

### Install from .deb Package

After building or downloading the `.deb` package:

```bash
sudo dpkg -i claude-tray_*.deb
```

Then enable and start the service:

```bash
systemctl --user enable --now claude-tray
```

### Build from Source

```bash
git clone https://github.com/jrdx0/claude-tray.git
cd claude-tray
cargo build --release
```

The compiled binary will be available at `target/release/claude-tray`.

### Build .deb Package

To build the `.deb` package, it will require to have the repository locally and `cargo-deb` installed:

```bash
git clone https://github.com/jrdx0/claude-tray.git
cd claude-tray
```

```bash
cargo install cargo-deb
```

And run cargo deb inside the repository directory:

```bash
cargo deb
```

The `.deb` package will be available at `target/debian/`. To install it:

```bash
sudo dpkg -i target/debian/claude-tray_*.deb
```

### Running

**Standalone mode:**
```bash
./target/release/claude-tray
```

**As a systemd service (recommended):**
```bash
systemctl --user enable --now claude-tray
```

## Systemd Service

The service runs as a **user service**, meaning each user manages their own instance.

### Starting the Service

**Enable and start the service:**
```bash
systemctl --user enable --now claude-tray
```

**Check service status:**
```bash
systemctl --user status claude-tray
```

**View logs:**
```bash
journalctl --user -u claude-tray -f
```

### Managing the Service

**Stop the service:**
```bash
systemctl --user stop claude-tray
```

**Restart the service:**
```bash
systemctl --user restart claude-tray
```

**Disable the service (stop autostart):**
```bash
systemctl --user disable claude-tray
```

### Auto-start on Login

The service is configured to start automatically when you log in (via `WantedBy=default.target`).

If you want the service to start even before login (linger), run:
```bash
loginctl enable-linger $USER
```

## Troubleshooting

### Service Not Starting

1. Check logs: `journalctl --user -u claude-tray -n 50`
2. Verify binary exists: `ls -l /usr/bin/claude-tray`
3. Check permissions: `systemctl --user status claude-tray`

### Display Issues

If the tray icon doesn't appear, ensure:
- You're running a desktop environment with system tray support
- `DISPLAY` environment variable is set correctly
- X11 authorization is working (`echo $XAUTHORITY`)

### Uninstalling

```bash
# Stop and disable service first
systemctl --user stop claude-tray
systemctl --user disable claude-tray

# Remove package
sudo dpkg -r claude-tray

# Purge (removes config files)
sudo dpkg -P claude-tray
```

## Configuration

User configuration is stored in: `~/.config/claude-tray/`

This directory is preserved during package upgrades and must be manually removed if desired.

## Tested OS

This application has been tested on:
- Pop!_OS (Linux 6.17.4)

Note: This application requires a system tray implementation compatible with the StatusNotifier/AppIndicator protocol (commonly available on Linux desktop environments) and x-terminal-emulator for terminal-based interactions.

## Contributing

Contributions are welcome! Here's how to get started with development:

### Development Mode

1. Clone the repository:
```bash
git clone https://github.com/jrdx0/claude-tray.git
cd claude-tray
```

2. Run in development mode:
```bash
cargo run
```

### Code Formatting and Linting

Before submitting changes, ensure your code is properly formatted and passes linting:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy
```

## License

MIT
