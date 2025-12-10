# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`claude-tray` is a Rust project targeting the 2024 edition. The project is in early development stages.

## Build and Development Commands

### Building
```bash
cargo build          # Debug build
cargo build --release  # Release build
```

### Running
```bash
cargo run           # Run debug build
cargo run --release # Run release build
```

### Testing
```bash
cargo test          # Run all tests
cargo test <test_name>  # Run specific test
```

### Linting and Formatting
```bash
cargo clippy        # Run linter
cargo fmt           # Format code
cargo fmt -- --check  # Check formatting without changing files
```

### Cleaning
```bash
cargo clean         # Remove target directory
```

## Architecture

The codebase is currently minimal with a single entry point in `src/main.rs`.
