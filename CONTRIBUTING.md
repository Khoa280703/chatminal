# Contributing to Chatminal

Thank you for your interest in contributing to Chatminal! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Development Setup](#development-setup)
- [Building the Project](#building-the-project)
- [Running Tests](#running-tests)
- [Code Style](#code-style)
- [Commit Messages](#commit-messages)
- [Submitting Pull Requests](#submitting-pull-requests)

## Development Setup

### Prerequisites

- **Rust**: Stable version (>= 1.93)
- **Platform**: macOS or Linux
- **Git**: For version control

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/chatminal/chatminal.git
cd chatminal

# Build the project
make check
```

On first build, vendor dependencies will be hydrated automatically. If needed:

```bash
make bootstrap-terminal-deps
```

## Building the Project

```bash
# Run the desktop app
make window

# Check the entire workspace
make check

# Check just the desktop app
make check-desktop
```

For more build commands, run `make help`.

## Running Tests

```bash
# Run all tests
make test

# Run tests for specific crate
cargo test -p runtime
cargo test --manifest-path crates/store/Cargo.toml

# Run tests for the entire workspace
cargo test --workspace --lib --bins --tests
```

## Code Style

- Follow existing code patterns in the codebase
- Use descriptive variable and function names
- Keep functions focused and small
- Add comments for complex logic
- Follow Rust naming conventions (snake_case for functions/variables)

See [Code Standards](./docs/code-standards.md) for detailed guidelines.

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]
[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `chore`: Maintenance tasks

### Examples

```
feat(runtime): add session restore functionality
fix(desktop): resolve window focus issue on macOS
docs: update README with installation instructions
refactor(terminal): simplify escape sequence parsing
test(store): add tests for session persistence
chore: update dependencies
```

## Submitting Pull Requests

1. **Fork the repository** and create a new branch:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make your changes** and ensure tests pass:
   ```bash
   make check
   make test
   ```

3. **Commit your changes** with clear messages:
   ```bash
   git commit -m "feat: add new feature"
   ```

4. **Push to your fork**:
   ```bash
   git push origin feat/your-feature-name
   ```

5. **Open a Pull Request** with:
   - Clear title following conventional commit format
   - Description of what changes were made
   - Links to any related issues
   - Testing instructions if applicable

## Questions?

- For general questions, open an issue
- For design discussions, reference existing issues or create a new one
- Check existing documentation in the `./docs` directory

## Thank You

Your contributions make Chatminal better for everyone!
