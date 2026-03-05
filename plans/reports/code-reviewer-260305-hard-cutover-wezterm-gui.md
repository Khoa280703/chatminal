## Code Review Summary

### Scope
- Files:
  - `apps/chatminal-app/src/main.rs`
  - `apps/chatminal-app/src/config.rs`
  - `apps/chatminal-app/src/input/mod.rs`
  - `apps/chatminal-app/src/input/terminal_input_event.rs`
  - `apps/chatminal-app/src/input/terminal_input_shortcut_filter.rs`
  - `apps/chatminal-app/src/input/ime_commit_deduper.rs`
  - `Makefile`
  - `README.md`
- LOC: 1461
- Focus: specific batch review (hard-cutover command/runtime WezTerm GUI)
- Scout findings:
  - Command surface claims rollback path, but runtime path appears unified.
  - Input pipeline mode parse is global (all commands), not command-scoped.
  - IME/input building blocks are test-gated in `input/mod.rs` (latent risk for embedded GUI enablement).

### Overall Assessment
Batch is mostly stable and compiles/tests clean for current proxy-first runtime. Main risk is operational: rollback command surface is not behaviorally independent from default path.

### Critical Issues
- None.

### High Priority
1. `window-proxy` rollback path is not independent; both commands resolve to same launcher/runtime.
- Evidence:
  - `apps/chatminal-app/src/main.rs:86` and `apps/chatminal-app/src/main.rs:89` both return `run_window_wezterm_gui(&config, &args)`.
  - `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:94` always builds `proxy-wezterm-session` payload for WezTerm start args.
  - `Makefile:23` and `README.md:45` describe `window-proxy` as rollback path, but `Makefile:70` routes to command that currently aliases default behavior.
- Repro confirmation:
  - `window-wezterm-gui` and `window-wezterm-gui-proxy` both emit identical launcher argv: `start -- ... proxy-wezterm-session <session_id>`.
- Impact:
  - During incident response, operator expects rollback path but gets same runtime behavior; rollback drill gives false confidence.
- Recommended fix:
  - Either implement distinct runtime branch for `window-wezterm-gui-proxy` (true fallback), or mark it explicitly as temporary alias in command/help/docs to avoid operational ambiguity.

### Medium Priority
1. Invalid `CHATMINAL_INPUT_PIPELINE_MODE` breaks all commands, including non-input operations and preflight checks.
- Evidence:
  - `apps/chatminal-app/src/main.rs:85` loads full `AppConfig` before dispatch.
  - `apps/chatminal-app/src/config.rs:19` always resolves pipeline mode.
  - `apps/chatminal-app/src/config.rs:137` returns hard error on invalid value.
- Impact:
  - A stale typo in env can fail commands like `workspace` and `sessions`, and indirectly fail `make window` preflight (`workspace` probe) though those commands do not use key translation.
- Recommended fix:
  - Parse `CHATMINAL_INPUT_PIPELINE_MODE` lazily only for commands that consume key mapping (`attach-wezterm` / future embedded window), or downgrade invalid value to warn+default for non-input commands.

### Low Priority
1. Runtime support messaging drift between docs and code.
- Evidence:
  - `README.md:18` says Linux/macOS only.
  - `apps/chatminal-app/src/config.rs:32` and `apps/chatminal-app/src/config.rs:45` include explicit Windows named-pipe endpoint logic.
- Impact:
  - Minor operator confusion for platform support scope.
- Recommended fix:
  - Clarify whether Windows support is active or preparatory; align README wording.

### Edge Cases Found by Scout
- Rollback command/path ambiguity under stress: incident runbook chooses `window-proxy`, but effective runtime does not change.
- Env typo blast radius: non-input commands fail due global pipeline-mode parse.
- Future cutover risk: `input/mod.rs:1` and `input/mod.rs:3` gate IME modules to `#[cfg(test)]`; if embedded GUI path is wired without revisiting gates/exports, production integration can regress unexpectedly.

### Positive Observations
- `cargo check --manifest-path apps/chatminal-app/Cargo.toml` passed.
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml` passed (65/65).
- Input mapping/dedupe modules have focused unit tests (AltGr normalization, IME dedupe window, attach exit key, legacy parity).

### Recommended Actions
1. Decide and enforce contract for `window-wezterm-gui-proxy`: real fallback vs explicit alias.
2. Reduce env misconfiguration blast radius by scoping `CHATMINAL_INPUT_PIPELINE_MODE` parsing to relevant commands.
3. Add regression smoke that asserts expected behavioral difference (or explicit equality) between `window-wezterm-gui` and `window-wezterm-gui-proxy`.

### Metrics
- Type Coverage: N/A (not measured in this batch; Rust compile/type-check passed)
- Test Coverage: N/A (coverage percentage not collected)
- Linting Issues: N/A (lint not run in this batch)
- Build/Test Validation:
  - `cargo check --manifest-path apps/chatminal-app/Cargo.toml` ✅
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` ✅ (65 passed)

### Unresolved Questions
1. `window-wezterm-gui-proxy` hiện chủ đích là alias tạm hay phải là fallback runtime độc lập ngay trong batch này?
2. Với `CHATMINAL_INPUT_PIPELINE_MODE`, team muốn fail-fast toàn cục hay chỉ áp dụng cho command thực sự dùng key translation?
