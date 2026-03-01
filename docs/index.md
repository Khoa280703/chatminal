# Chatminal Documentation

Last updated: 2026-03-01

Chatminal is a local desktop terminal multiplexer built with Rust + Iced + portable-pty.
This docs set reflects the current implementation in `src/` and the repository snapshot packed by `repomix` into `repomix-output.xml`.

## Contents
- [Project Overview & PDR](./project-overview-pdr.md) - Product scope, requirements, acceptance criteria.
- [System Architecture](./system-architecture.md) - Runtime components, data flow, concurrency model.
- [Code Standards](./code-standards.md) - Structure, coding rules, error handling, test policy.
- [Codebase Summary](./codebase-summary.md) - Evidence-based snapshot of current modules.
- [Deployment Guide](./deployment-guide.md) - Build, run, release workflow.
- [Design Guidelines](./design-guidelines.md) - UI behavior and terminal UX constraints.
- [Project Roadmap](./project-roadmap.md) - Milestones and pending work.
- [Development Roadmap](./development-roadmap.md) - Engineering execution view.
- [Project Changelog](./project-changelog.md) - History of major implementation and docs updates.

## Quick Start
1. Install Rust `1.93+`.
2. Run `cargo run`.
3. Run `cargo test`.
4. Optional config file: `~/.config/chatminal/config.toml`.
