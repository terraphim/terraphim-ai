# Design & Implementation Plan: Fix Opencode Task Delivery in ADF Spawner

## 1. Summary of Target Behavior

After implementation, opencode agents will always receive their task prompt as a positional command-line argument, regardless of task size. This eliminates the stdin delivery hang for large tasks (>32KB) while maintaining backward compatibility for other CLI tools (claude, codex, bash) that use stdin or their own argument patterns.

## 2. Key Invariants and Acceptance Criteria

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| opencode agents with 97KB tasks exit within 30s (not hang) | Integration | `crates/terraphim_spawner/tests/` |
| opencode agents produce output in log files | E2E | Manual verification on bigbox |
| claude agents still use `-p` flag with large tasks | Regression | Existing test suite |
| bash agents still use `-c` with inline script | Regression | Existing test suite |
| No SIGKILL events for opencode agents | E2E | Journal monitoring on bigbox |

## 3. High-Level Design and Boundaries

### Solution Concept
Add a per-CLI-tool configuration flag `supports_stdin: bool` (default: true) to `AgentConfig`. Set it to `false` for opencode. When `supports_stdin=false`, the spawner always passes the task as a positional argument, bypassing the size threshold logic.

### Boundaries
- **Changes inside**: `AgentConfig` struct, `spawn_process` method, `infer_args` logic
- **No changes**: Orchestrator task composition, output capture, health checking, agent definitions in TOML
- **New introduced**: `supports_stdin` field in `AgentConfig`

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_spawner/src/config.rs` | Modify | `AgentConfig` has `use_stdin: bool` | Add `supports_stdin: bool` (default true) | None |
| `crates/terraphim_spawner/src/config.rs` | Modify | `infer_args` only sets `args` | Also set `supports_stdin=false` for opencode | `AgentConfig` |
| `crates/terraphim_spawner/src/lib.rs` | Modify | `spawn_process` uses `use_stdin` param | Check `config.supports_stdin`; if false, always use positional arg | `AgentConfig` |
| `crates/terraphim_spawner/src/lib.rs` | Modify | `spawn_process` closes stdin by default | Only set `Stdio::null()` when not using stdin | `tokio::process` |

## 5. Step-by-Step Implementation Sequence

### Step 1: Add `supports_stdin` to AgentConfig
- **File**: `crates/terraphim_spawner/src/config.rs`
- **Action**: Add `pub supports_stdin: bool` field to `AgentConfig` with default `true`
- **Action**: Set `supports_stdin: false` in `infer_args` when CLI is "opencode"
- **Deployable**: Yes, no behavior change yet (field not read)

### Step 2: Modify spawn_process to respect supports_stdin
- **File**: `crates/terraphim_spawner/src/lib.rs`
- **Action**: Change `spawn_process` to check `config.supports_stdin` before deciding stdin vs positional
- **Logic**: If `supports_stdin == false`, always pass task as positional arg (`cmd.arg(task)`)
- **Deployable**: Yes, this changes behavior for opencode only

### Step 3: Update tests
- **File**: `crates/terraphim_spawner/src/lib.rs` (tests)
- **Action**: Add test verifying opencode config has `supports_stdin=false`
- **Action**: Add test verifying large task is passed as arg when `supports_stdin=false`
- **Deployable**: Yes, test-only change

### Step 4: Build and deploy to bigbox
- **Action**: Build release binary
- **Action**: Deploy to bigbox, restart ADF
- **Action**: Verify with implementation-swarm test
- **Deployable**: Final validation step

## 6. Testing & Verification Strategy

| Test | Type | How |
|------|------|-----|
| `test_opencode_config_no_stdin` | Unit | Assert `infer_args("opencode")` returns config with `supports_stdin=false` |
| `test_opencode_large_task_as_arg` | Integration | Spawn opencode with 97KB task, verify command line contains task, not stdin |
| `test_claude_still_uses_stdin` | Regression | Spawn claude with 97KB task, verify stdin is used (existing behavior) |
| Bigbox E2E | Manual | Run implementation-swarm, verify agents complete within 30s |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| ARG_MAX exceeded for extremely large tasks | ARG_MAX is 2MB; tasks are ~63KB. If ever接近 limit, add file-based fallback | Very low - 32x headroom |
| Special characters in task break shell parsing | `Command::arg()` handles escaping automatically in Rust | None |
| Other tools accidentally affected | `supports_stdin` defaults to `true`; only opencode explicitly set to `false` | Very low |
| Stdin delivery needed for some edge case | Can override per-agent in TOML if needed | Low |

## 8. Open Questions / Decisions for Human Review

1. Should `supports_stdin` be configurable in `terraphim.toml` per-agent, or hardcoded per CLI tool?
2. Should we also disable stdin for `codex` and `claude` if they show similar issues?
