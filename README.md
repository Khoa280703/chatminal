# Chatminal

> A modern terminal emulator. Simple, fast, and powerful.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.93+-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux-blue.svg)](https://github.com/chatminal/chatminal)

## 🌟 Features

- **🚀 Native Performance** - Built with Rust for blazing-fast speed
- **💻 Cross-Platform** - Runs on macOS and Linux
- **💾 Session Management** - Save and restore your terminal sessions seamlessly
- **🎨 Profile Support** - Create custom profiles for different workflows
- **✨ Modern UI** - Clean, minimal interface that gets out of your way
- **⚡ GPU Accelerated** - Smooth rendering with hardware acceleration

## 📦 Installation

### Quick Install

```bash
curl -fsSL https://chatminal.com/install | bash
```

Default behavior installs the latest stable release.
Current installer targets:

- macOS `aarch64`
- macOS `x86_64`
- Linux `x86_64`

Optional:

```bash
curl -fsSL https://chatminal.com/install | CHATMINAL_VERSION=v0.1.2 bash
```

### Homebrew

```bash
brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal
brew install --cask chatminal
```

### From Source

```bash
# Clone the repository
git clone https://github.com/chatminal/chatminal.git
cd chatminal

# Build and run
make window
```

### Requirements

- **Rust**: Stable version (>= 1.93)
- **Platform**: macOS or Linux

### Install Locations

- **Linux app files**: `~/.local/share/chatminal/<version>`
- **Linux launcher**: `~/.local/bin/chatminal`
- **macOS app**: `~/Applications/Chatminal.app`
- **macOS launcher**: `~/.local/bin/chatminal`

> **Note**: On first build, vendor dependencies will be hydrated automatically.

## 🚀 Quick Start

After building, the desktop window will open automatically:

1. Create a new session with your default shell
2. Configure profiles in settings
3. Start using Chatminal as your daily terminal

## 🛠️ Build Commands

```bash
# Run the desktop app
make window

# Check the build
make check

# Run tests
make test

# Clean build artifacts
make clean

# Show all available commands
make help
```

## ⚙️ Configuration

Chatminal can be configured via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CHATMINAL_DATA_DIR` | Directory for app data | Platform-specific |
| `CHATMINAL_DEFAULT_SHELL` | Shell to spawn | `$SHELL` |
| `CHATMINAL_DEFAULT_COLS` | Default terminal width | `120` |
| `CHATMINAL_DEFAULT_ROWS` | Default terminal height | `32` |

See [`.env.example`](.env.example) for all available configuration options.

## 📚 Documentation

- [**Getting Started**](./docs/index.md) - Introduction and overview
- [**System Architecture**](./docs/system-architecture.md) - How Chatminal works
- [**Code Standards**](./docs/code-standards.md) - Contributing guidelines
- [**Deployment Guide**](./docs/deployment-guide.md) - Building for production
- [**Roadmap**](./docs/project-roadmap.md) - Future plans
- [**Changelog**](./docs/project-changelog.md) - Release history

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](./CONTRIBUTING.md) for details on:

- Setting up your development environment
- Building and testing
- Submitting pull requests
- Code style and conventions

### Code of Conduct

Please note that this project is released with a [Code of Conduct](./CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## 📄 License

This project is licensed under the [MIT License](./LICENSE).

## 🙏 Acknowledgments

Chatminal is built on the foundation of [WezTerm](https://wezfurlong.org/wezterm/), the wonderful terminal multiplexer by Wez Furlong.

---

<div align="center">

**Star ⭐ this repo if you find it helpful!**

</div>
