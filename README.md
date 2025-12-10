# Claude Tray

A system tray application for monitoring Claude AI usage.

## Description

Claude Tray is a Linux/Debian system tray utility that integrates with Claude AI's OAuth API. It provides quick access to Claude AI services and displays usage statistics directly from your system tray.

Features:
- System tray integration for quick access
- Claude AI authentication via OAuth
- Usage monitoring (5-hour and 7-day limits)
- Quick browser access to Claude AI

## Installation

### Prerequisites

- Claude CLI installed and configured

### Build from source

```bash
git clone https://github.com/jrdx0/claude-tray.git
cd claude-tray
cargo build --release
```

The compiled binary will be available at `target/release/claude-tray`.

### Running

```bash
./target/release/claude-tray
```

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

### Code formatting and linting

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
