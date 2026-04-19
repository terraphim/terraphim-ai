# Implementation Plan: Agent CLI Dispatch for "implement" Subcommand

**Status**: Draft
**Author**: Claude
**Date**: 2026-04-16
**Research**: Phase 1 completed (via code analysis)

## Overview

### Summary
Add capability to dispatch "implement" subcommand to an external AI coding agent CLI (e.g., opencode) instead of the built-in terraphim-agent. This enables integration with specialized coding agents for implementation tasks.

### Feature Status
| Component | Status |
|-----------|--------|
| `DispatchConfig::agent_cli` | Implemented |
| `DispatchConfig::agent_model` | Implemented |
| `ShellDispatchConfig::agent_cli` | Implemented |
| `ShellDispatchConfig::agent_model` | Implemented |
| `execute_agent_dispatch()` | Implemented |
| Listener dispatch logic | Implemented |
| **Test compilation** | **BROKEN - missing fields** |

## Issue Identified

### Compilation Error
```
error[E0063]: missing fields `agent_cli` and `agent_model` in initializer of `shell_dispatch::ShellDispatchConfig`
   --> crates/terraphim_agent/src/shell_dispatch.rs:647:9
```

### Root Cause
`test_config()` helper at line 646-654 does not include the new `agent_cli` and `agent_model` fields.

## Implementation Steps

### Step 1: Fix Test Compilation
**File:** `crates/terraphim_agent/src/shell_dispatch.rs`
**Line:** 646-654
**Change:** Add missing fields to `test_config()`

```rust
fn test_config(binary: &str) -> ShellDispatchConfig {
    ShellDispatchConfig {
        agent_binary: PathBuf::from(binary),
        max_output_bytes: MAX_OUTPUT_BYTES,
        timeout: Duration::from_secs(5),
        extra_allowed: vec![],
        working_dir: None,
        guard: std::sync::Arc::new(crate::guard_patterns::CommandGuard::new()),
        agent_cli: None,           // ADD
        agent_model: None,         // ADD
    }
}
```

**Verification:** `cargo test -p terraphim_agent --no-run` passes

---

### Step 2: Add Unit Tests for `execute_agent_dispatch`
**File:** `crates/terraphim_agent/src/shell_dispatch.rs`
**Location:** After existing dispatch tests (around line 700)

**Tests to add:**
```rust
#[tokio::test]
async fn test_execute_agent_dispatch_not_configured() {
    let mut config = test_config("/bin/echo");
    config.agent_cli = None;  // Explicitly not configured
    let result = execute_agent_dispatch(&config, "test").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("agent_cli not configured"));
}

#[tokio::test]
async fn test_execute_agent_dispatch_success() {
    let mut config = test_config("/bin/echo");
    config.agent_cli = Some(PathBuf::from("/bin/echo"));
    config.agent_model = Some("kimi-for-coding/k2p5".to_string());

    let result = execute_agent_dispatch(&config, "test message").await;
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.subcommand, "implement");
    assert!(result.exit_code == 0);
    assert!(result.stdout.contains("test message"));
}

#[tokio::test]
async fn test_execute_agent_dispatch_with_args() {
    let mut config = test_config("/bin/sh");
    config.agent_cli = Some(PathBuf::from("/bin/sh"));
    config.agent_model = Some("test-model".to_string());

    let result = execute_agent_dispatch(&config, "echo hello").await;
    assert!(result.is_ok());
}
```

**Verification:** `cargo test -p terraphim_agent shell_dispatch::` passes

---

### Step 3: Verify Listener Integration
**File:** `crates/terraphim_agent/src/listener.rs`
**Lines:** 1607-1647

**Verification checklist:**
- [ ] `agent_cli.is_some()` check triggers correctly
- [ ] Context message format is correct
- [ ] Error handling posts to tracker correctly
- [ ] Success result posts formatted reply correctly

**Manual test (if Gitea connection available):
```bash
# Send test event with "implement" subcommand
```

---

### Step 4: Update Documentation
**File:** `crates/terraphim_agent/src/listener.rs` (inline docs)
**Change:** Update `DispatchConfig` doc comment to clarify usage

Current:
```rust
/// Path to an AI coding agent CLI (e.g. opencode) for "implement" dispatch.
```

Verify this is sufficient or add example config.

---

## Simplicity Check

**What if this could be easy?**
The implementation is already complete - just needs the test fix and verification. This is a minimal, focused change.

## Eliminated Options

| Option | Why Rejected |
|--------|--------------|
| Add "analyze" subcommand dispatch | Not in scope - only "implement" requested |
| Generic external agent spawning | Over-engineering - hardcoded "implement" is sufficient |
| Configurable subcommand -> agent mapping | YAGNI - only "implement" needs this |

## Testing Strategy

### Unit Tests (Step 2)
- Test `execute_agent_dispatch` with various configurations
- Test error path when `agent_cli` not configured
- Test success path with mock binary

### Integration Tests
- Existing `shell_dispatch` tests cover basic dispatch
- Listener dispatch logic covered by existing integration tests

### Verification Commands
```bash
# Compile check
cargo check -p terraphim_agent

# Run tests
cargo test -p terraphim_agent shell_dispatch

# Full test suite
cargo test -p terraphim_agent
```

## Rollback Plan

If issues arise:
1. Revert `test_config()` change (remove `agent_cli`/`agent_model`)
2. Revert listener dispatch logic change
3. Revert `ShellDispatchConfig` field additions
4. Revert `DispatchConfig` field additions

**No feature flag needed** - this is an opt-in feature via config.

## Dependencies

| Dependency | Version | Risk |
|------------|---------|------|
| tokio::process::Command | stable | None |
| tokio::time::timeout | stable | None |

No new dependencies required.

## Performance Considerations

- Spawns external process (same as existing dispatch)
- Timeout handling already implemented
- No performance impact on existing paths

## Open Items

| Item | Status |
|------|--------|
| None | - |

## Approval Gate

- [ ] Compilation error fixed
- [ ] Unit tests added and passing
- [ ] `cargo clippy` passes
- [ ] `cargo fmt` applied
