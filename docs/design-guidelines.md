# Design Guidelines

Last updated: 2026-04-06

## UX Direction

- Terminal-first native experience
- Prioritize fidelity and stability of terminal behavior
- Do not add panels or UI elements unless needed for terminal core

## Interaction

- Session switching must preserve correct pane state per session
- Reconnect must maintain reasonable history preview before live stream
- Input/output paths must not block the render loop

## Visual Constraints

- Minimal interface, easy to read
- Focus on information density (sessions + active pane + status)

## Principles

- **YAGNI** (You Aren't Gonna Need It): Don't add features until needed
- **KISS** (Keep It Simple, Stupid): Simple solutions are better
- **DRY** (Don't Repeat Yourself): Avoid code duplication
