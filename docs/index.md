# Chatminal Documentation

Last updated: 2026-03-01

Chatminal is a local desktop terminal workspace built with Rust + Iced + portable-pty, with terminal state parsing/render snapshot fed by `wezterm-term`.
This documentation set reflects current code in `src/` and the latest `repomix-output.xml` snapshot.

## Start Here
- [Repository README](../README.md) - Setup, quick start, config, troubleshooting.
- [Project Overview & PDR](./project-overview-pdr.md) - Scope, requirements, acceptance criteria.

## Core Technical Docs
- [System Architecture](./system-architecture.md) - Runtime components, PTY/data flow, concurrency.
- [Codebase Summary](./codebase-summary.md) - Verified repository snapshot and module map.
- [Code Standards](./code-standards.md) - Implementation rules and sync standards.

## Product and Planning Docs
- [Project Roadmap](./project-roadmap.md) - Milestones, progress, upcoming tracks.
- [Development Roadmap](./development-roadmap.md) - Engineering-phase execution view.
- [Project Changelog](./project-changelog.md) - Timeline of notable implementation/docs changes.

## Supplemental Docs
- [Deployment Guide](./deployment-guide.md) - Build/test/release workflow.
- [Design Guidelines](./design-guidelines.md) - UI behavior and rendering constraints.
