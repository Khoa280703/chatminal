# Phase 3.2: Delete third_party/terminal-engine-reference

**Context:** [plan.md](./plan.md) | Tier 3 Low | Independent, anytime

## Overview

- **Priority:** P3
- **Status:** completed
- **Effort:** 15min
- **Description:** Delete ~3GB WezTerm reference snapshot. No runtime code depends on it; workspace Cargo.toml already excludes it. README already documents it as reference-only.

## Key Insights

- `third_party/terminal-engine-reference/` is 3.0GB
- Workspace `Cargo.toml:71` has `exclude = ["third_party/terminal-engine-reference"]` — not built
- `README.md:9,11` already notes it's reference-only and no longer a supported workspace
- Contains WezTerm source code used during initial fork; all needed code has been copied/adapted into chatminal crates
- `split_and_insert` references exist inside this directory (from WezTerm original) — irrelevant after deletion

## Related Code Files

**Delete:**
- `third_party/terminal-engine-reference/` (entire directory, ~3GB)

**Modify:**
- `Cargo.toml:71` — remove `exclude` line (nothing left to exclude)
- `README.md` — remove references to `third_party/terminal-engine-reference`

## Implementation Steps

1. **Delete the directory:**
   ```bash
   rm -rf third_party/terminal-engine-reference
   ```

2. **Remove or clean `third_party/` if empty:**
   ```bash
   rmdir third_party 2>/dev/null || true
   ```

3. **Update `Cargo.toml:71`:**
   ```toml
   # Before:
   exclude = ["third_party/terminal-engine-reference"]
   # After: remove the line entirely
   ```

4. **Update `README.md`:**
   - Remove lines 9 and 11 referencing `third_party/terminal-engine-reference`
   - Or replace with: "WezTerm reference source has been removed; all needed code is in chatminal crates."

5. **Update `.gitignore` if it references third_party** (check first)

## Todo List

- [x] Delete third_party/terminal-engine-reference directory
- [x] Remove Cargo.toml exclude line
- [x] Update README.md references
- [x] Check and update .gitignore if needed
- [x] Run verification

## Success Criteria

- `third_party/` directory gone (or empty)
- `cargo check --workspace` passes
- No references to `terminal-engine-reference` in workspace config
- Git commit shows ~3GB reduction

## Risk Assessment

- **Low risk:** Already excluded from workspace build
- **Watch:** Ensure no build script or CI references files inside third_party
- **Note:** Git history retains the files; repo size only shrinks after `git gc` or fresh clone with shallow history

## Verification

```bash
cargo check --workspace
# Confirm no references:
grep -rn "terminal-engine-reference" --include="*.toml" --include="*.md" .
ls third_party/ 2>/dev/null && echo "WARN: third_party still exists" || echo "OK: third_party gone"
```
