# Chatminal

A modern terminal emulator for everyone. Simple, fast, and powerful.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Native Performance**: Built with Rust for speed and reliability
- **Cross-Platform**: Runs on macOS and Linux
- **Session Management**: Save and restore your terminal sessions
- **Profile Support**: Create custom profiles for different workflows
- **Modern UI**: Clean, minimal interface that gets out of your way
- **GPU Accelerated**: Smooth rendering with hardware acceleration

## Quick Start

### Installation

**From Source (macOS/Linux):**

```bash
# Clone the repository
git clone https://github.com/chatminal/chatminal.git
cd chatminal

# Build and run
make window
```

**Build Requirements:**
- Rust stable (>= 1.93)
- On first build, vendor dependencies will be hydrated automatically

### First Run

After building, the desktop window will open automatically. You can:

1. Create a new session with your default shell
2. Configure profiles in settings
3. Start using Chatminal as your daily terminal

## Configuration

Chatminal respects these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CHATMINAL_DATA_DIR` | Directory for app data | `~/.local/share/chatminal` |
| `CHATMINAL_DEFAULT_SHELL` | Default shell to spawn | `/bin/bash` |
| `CHATMINAL_DEFAULT_COLS` | Default terminal columns | `120` |
| `CHATMINAL_DEFAULT_ROWS` | Default terminal rows | `32` |

See `.env.example` for all available configuration options.

## Documentation

- [Project Overview](./docs/index.md) - Introduction and getting started
- [System Architecture](./docs/system-architecture.md) - How Chatminal works
- [Code Standards](./docs/code-standards.md) - Contributing guidelines
- [Deployment Guide](./docs/deployment-guide.md) - Building for production
- [Development Roadmap](./docs/project-roadmap.md) - Future plans
- [Changelog](./docs/project-changelog.md) - Release history

## Contributing

We welcome contributions! Please see our [Contributing Guide](./CONTRIBUTING.md) for details on:

- Setting up your development environment
- Building and testing
- Submitting pull requests
- Code style and conventions

## Building from Source

```bash
# Run the desktop app
make window

# Check the build
make check

# Run tests
make test

# Clean build artifacts
make clean
```

For more build commands, run `make help`.

## License

This project is licensed under the [MIT License](./LICENSE).

## Acknowledgments

Chatminal is built on the foundation of [WezTerm](https://wezfurlong.org/wezterm/), the wonderful terminal multiplexer by Wez Furlong.
