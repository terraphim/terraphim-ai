# Design & Implementation Plan: Fix parse_chained_command mid-chain failure (#906)

**Status**: Draft
**Research Doc**: `.docs/research-0906-parse-chained-command-mid-chain.md`
**Author**: opencode
**Date**: 2026-04-25
**Supersedes**: `.docs/design-0866-parse-chained-command.md`
**Estimated Effort**: 1 hour

## Overview

### Summary

Fix `parse_chained_command` to not return provably incorrect answers for `&&` chains. The previous fix (#866, commit `54e6d4b4`) changed from returning `parts[0]` to `parts.last()` for non-zero exit — which is wrong in the opposite direction for `&&` chains where mid-chain commands never executed.

### Approach

**Operator-specific logic with honest heuristics:**
- `&&` non-zero: return `parts[0]` (first subcommand — definitely ran, may or may not be the one that failed)
- `||` non-zero: return `parts.last()` (last attempted — all failed, so last one definitely ran and failed)
- `;` non-zero: return `parts.last()` (all ran, can't disambiguate, last is conventional)
- Any operator, exit=0: return `parts[0]` (chain succeeded)
- No operator: return `(command.trim(), None)`

Add a code comment documenting the `&&` limitation clearly.

### Scope

**In Scope:**
- Fix `parse_chained_command` logic in `capture.rs:1080`
- Update `test_parse_chained_command` tests
- Add code comment documenting limitation

**Out of Scope:**
- Per-step exit code tracking (shell instrumentation)
- Function signature change
- `CapturedLearning` struct changes
- Handling `cmd1&&cmd2` (no spaces) — rare in practice
- Mixed operator chains (`cmd1 && cmd2 || cmd3`)

**Avoid At All Cost:**
- `ChainAnalysis` enum or structured return type (over-engineering for P2)
- `terraphim-automata` for 3 static operators (wrong abstraction level)
- Any new dependency

## Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Return `parts[0]` for `&&` non-zero | First command definitely executed; never provably wrong | `parts.last()` (may never have run), return all (signature change), return full chain (changes caller semantics) |
| Keep `(String, Option<String>)` return type | No caller changes needed | `ChainAnalysis` enum (over-engineered) |
| Document limitation in code comment | Future developers see the tradeoff | Silent heuristic (how we got here) |
| `||` non-zero returns `parts.last()` | Correct: all commands ran, last one is final attempt | Return first (misleading for fallback chains) |

### Simplicity Check

**What if this could be easy?** The simplest fix is a 3-line change in the `get_failing` closure: for `&&`, use `parts[0]` instead of `parts.last()`. The only complication is that the current code uses a single closure for all operators. We need operator-specific logic.

**Minimum code change**: Split the single closure into operator-specific branches. ~15 lines changed total.

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/learnings/capture.rs:1080-1110` | Replace `get_failing` closure with operator-specific logic |
| `crates/terraphim_agent/src/learnings/capture.rs:1901-1935` | Update test expectations for `&&` mid-chain case |

No new files. No deleted files.

## API Design

### Function Signature (unchanged)

```rust
fn parse_chained_command(command: &str, exit_code: i32) -> (String, Option<String>)
```

Returns `(actual_command, Some(full_chain))` if chain detected, `(command, None)` otherwise.

### Behaviour Change

| Operator | Exit Code | Before (current, broken) | After (this fix) |
|----------|-----------|--------------------------|-------------------|
| `&&` | non-zero | `parts.last()` | `parts[0]` |
| `&&` | 0 | `parts[0]` | `parts[0]` (unchanged) |
| `\|\|` | non-zero | `parts.last()` | `parts.last()` (unchanged) |
| `\|\|` | 0 | `parts[0]` | `parts[0]` (unchanged) |
| `;` | non-zero | `parts.last()` | `parts.last()` (unchanged) |
| `;` | 0 | `parts[0]` | `parts[0]` (unchanged) |
| none | any | `(command, None)` | `(command, None)` (unchanged) |

**Only one line of logic changes**: `&&` non-zero goes from `parts.last()` to `parts[0]`.

## Implementation Steps

### Step 1: Replace `parse_chained_command` function body

**File:** `capture.rs:1080-1110`
**Change:** Replace the `get_failing` closure with operator-specific logic

**Before:**
```rust
fn parse_chained_command(command: &str, exit_code: i32) -> (String, Option<String>) {
    let get_failing = |parts: &[&str]| -> String {
        if exit_code != 0 {
            parts.last().unwrap().trim().to_string()
        } else {
            parts[0].trim().to_string()
        }
    };

    let chain_operators = [" && ", " || ", "; "];

    for op in &chain_operators {
        if command.contains(op) {
            let parts: Vec<&str> = command.split(op).collect();
            if parts.len() > 1 {
                let failing = get_failing(&parts);
                return (failing, Some(command.to_string()));
            }
        }
    }

    (command.trim().to_string(), None)
}
```

**After:**
```rust
fn parse_chained_command(command: &str, exit_code: i32) -> (String, Option<String>) {
    // Operator-specific heuristics for identifying the failing subcommand.
    //
    // && chains: short-circuit on first failure. With a single exit code we
    // cannot tell WHICH subcommand failed, only that some prefix succeeded
    // then one returned non-zero. We return the first subcommand because it
    // definitely executed (the last may never have run).
    //
    // || chains: short-circuit on first success. Non-zero exit means ALL
    // commands ran and all failed, so the last is the final attempt.
    //
    // ; chains: all commands run regardless. We return the last as a
    // convention (cannot disambiguate without per-step exit codes).

    if command.contains(" && ") {
        let parts: Vec<&str> = command.split(" && ").collect();
        if parts.len() > 1 {
            let candidate = if exit_code != 0 {
                parts[0].trim().to_string()
            } else {
                parts[0].trim().to_string()
            };
            return (candidate, Some(command.to_string()));
        }
    }

    if command.contains(" || ") {
        let parts: Vec<&str> = command.split(" || ").collect();
        if parts.len() > 1 {
            let candidate = if exit_code != 0 {
                parts.last().unwrap().trim().to_string()
            } else {
                parts[0].trim().to_string()
            };
            return (candidate, Some(command.to_string()));
        }
    }

    if command.contains("; ") {
        let parts: Vec<&str> = command.split("; ").collect();
        if parts.len() > 1 {
            let candidate = if exit_code != 0 {
                parts.last().unwrap().trim().to_string()
            } else {
                parts[0].trim().to_string()
            };
            return (candidate, Some(command.to_string()));
        }
    }

    (command.trim().to_string(), None)
}
```

**Deployable:** Function compiles, tests will need updating.

### Step 2: Update test_parse_chained_command

**File:** `capture.rs:1901-1935`
**Change:** Fix the `&&` non-zero test expectation from `parts.last()` to `parts[0]`

**Before (current test):**
```rust
fn test_parse_chained_command() {
    // Test && chain with non-zero exit (last subcommand failed)
    let (cmd, chain) = parse_chained_command("cargo build && cargo test", 1);
    assert_eq!(cmd, "cargo test");
    // ...

    // Test && chain with zero exit (success, first subcommand is meaningful)
    let (cmd2, chain2) = parse_chained_command("cargo build && cargo test", 0);
    assert_eq!(cmd2, "cargo build");
    // ...
}
```

**After:**
```rust
fn test_parse_chained_command() {
    // && chain, non-zero exit: first subcommand (conservative, definitely executed)
    let (cmd, chain) = parse_chained_command("cargo build && cargo test", 1);
    assert_eq!(cmd, "cargo build");
    assert_eq!(chain, Some("cargo build && cargo test".to_string()));

    // && chain, zero exit: first subcommand (chain succeeded)
    let (cmd2, chain2) = parse_chained_command("cargo build && cargo test", 0);
    assert_eq!(cmd2, "cargo build");
    assert_eq!(chain2, Some("cargo build && cargo test".to_string()));

    // || chain, non-zero exit: last subcommand (all failed, last attempted)
    let (cmd3, chain3) = parse_chained_command("cmd_a || cmd_b || cmd_c", 1);
    assert_eq!(cmd3, "cmd_c");
    assert_eq!(chain3, Some("cmd_a || cmd_b || cmd_c".to_string()));

    // ; chain, non-zero exit: last subcommand (cannot disambiguate)
    let (cmd4, chain4) = parse_chained_command("cmd_a; cmd_b; cmd_c", 1);
    assert_eq!(cmd4, "cmd_c");
    assert_eq!(chain4, Some("cmd_a; cmd_b; cmd_c".to_string()));

    // Single command, no chain
    let (cmd5, chain5) = parse_chained_command("git status", 0);
    assert_eq!(cmd5, "git status");
    assert_eq!(chain5, None);

    // Single command with failure
    let (cmd6, chain6) = parse_chained_command("git status", 1);
    assert_eq!(cmd6, "git status");
    assert_eq!(chain6, None);

    // && three-part chain: first subcommand returned regardless of which
    // actually failed (we cannot determine that from exit code alone)
    let (cmd7, chain7) = parse_chained_command("cmd1 && cmd2 && cmd3", 1);
    assert_eq!(cmd7, "cmd1");
    assert_eq!(chain7, Some("cmd1 && cmd2 && cmd3".to_string()));

    // || chain, zero exit (short-circuited on success): first subcommand
    let (cmd8, chain8) = parse_chained_command("cmd_a || cmd_b || cmd_c", 0);
    assert_eq!(cmd8, "cmd_a");
    assert_eq!(chain8, Some("cmd_a || cmd_b || cmd_c".to_string()));
}
```

**Deployable:** All tests pass.

### Step 3: Run tests and verify

```bash
cargo test -p terraphim_agent parse_chained_command
cargo test -p terraphim_agent capture_failed_command
cargo clippy -p terraphim_agent -- -D warnings
```

## Test Strategy

| Test Case | Input | Exit | Expected `cmd` | Expected `chain` |
|-----------|-------|------|----------------|-------------------|
| `&&` 2-part failure | `"cargo build && cargo test"` | 1 | `"cargo build"` | `Some("cargo build && cargo test")` |
| `&&` 2-part success | `"cargo build && cargo test"` | 0 | `"cargo build"` | `Some("cargo build && cargo test")` |
| `&&` 3-part failure | `"cmd1 && cmd2 && cmd3"` | 1 | `"cmd1"` | `Some("cmd1 && cmd2 && cmd3")` |
| `\|\|` all fail | `"cmd_a \|\| cmd_b \|\| cmd_c"` | 1 | `"cmd_c"` | `Some(...)` |
| `\|\|` success | `"cmd_a \|\| cmd_b \|\| cmd_c"` | 0 | `"cmd_a"` | `Some(...)` |
| `;` failure | `"cmd_a; cmd_b; cmd_c"` | 1 | `"cmd_c"` | `Some(...)` |
| Single success | `"git status"` | 0 | `"git status"` | `None` |
| Single failure | `"git status"` | 1 | `"git status"` | `None` |

### Regression Tests

Existing tests for `capture_failed_command`, `test_correct_learning`, and the importance/repetition tests should pass unchanged — they use single commands only.

## Risk Review

| Risk (from Phase 1) | Mitigation | Residual |
|---------------------|------------|----------|
| Returning first for `&&` may not be the actual failure | Documented in code comment; `full_chain` preserves context | Low — first is always plausible, unlike last which is often impossible |
| Callers rely on `parts.last()` behaviour | Check callers — only `capture_failed_command` at line 950 uses result; downstream storage is fine | None |
| `&&` and `||` have same non-zero result for `parts[0]` | Wait — no. `&&` returns first, `||` returns last. Different operators, different logic | None |

## Approval

- [ ] Research reviewed
- [ ] Design reviewed
- [ ] Human approval received
