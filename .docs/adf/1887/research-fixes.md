# Research Document: ADF Flow Executor Improvements

**Status**: Approved
**Author**: ADF Investigation Agent
**Date**: 2026-05-30
**Reviewers**: Disciplined Research / Disciplined Design

## Executive Summary

Two critical issues were identified during the end-to-end validation of ADF's disciplined development pipeline on issue #1887:

1. **Review agent slowness**: The structured PR review step took 6+ minutes for a 199-line change, with the agent still running after 15 minutes. Root cause is excessively long inline task prompts and overly generous timeouts.

2. **Flow `repo_path` ignored**: The `repo_path` field in flow TOML definitions is never used by `cmd_flow`. The executor always uses the current working directory, causing agents to write artefacts to the wrong location when the flow is invoked from a different directory.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Both issues block reliable ADF pipeline execution |
| Leverages strengths? | Yes | We have full control over flow executor and spawner |
| Meets real need? | Yes | Validated during #1887 E2E run - agents wrote to wrong dir |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Issue 1: Review Agent Slowness

**Description**: The `review` step in `zdp-validate-pipeline.toml` has `timeout_secs = 600` (10 minutes). During the #1887 validation, the review agent ran for over 15 minutes without completing, consuming API budget and blocking pipeline progress.

**Impact**: 
- Blocks pipeline execution for extended periods
- Wastes API budget on overly long prompts
- Reduces confidence in ADF automation
- Agents may timeout before producing findings

**Success Criteria**:
- Review step completes within 3 minutes for changes < 500 lines
- Task prompt size < 2000 tokens
- Timeout configured per step complexity, not blanket 10 minutes

### Issue 2: Flow `repo_path` Ignored

**Description**: The `FlowDefinition` struct has a `repo_path` field, but `cmd_flow` in `adf-ctl.rs` never uses it. Instead, it uses `std::env::current_dir()` for both `FlowExecutor::new()` and `ProjectRuntime.working_dir`.

**Impact**:
- Agents write artefacts to wrong directory when flow invoked from non-repo cwd
- Breaks the contract between flow author (who specifies `repo_path`) and executor
- Requires manual workaround (cd to repo before running flow)
- Breaks CI/CD scenarios where working directory differs from repo root

**Success Criteria**:
- `flow.repo_path` is used as the working directory for agent steps
- Relative `repo_path` values resolve correctly from flow file location
- Action steps also execute in `repo_path`

## Current State Analysis

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Flow definition | `crates/terraphim_orchestrator/src/flow/config.rs:58` | `repo_path: String` field |
| Flow executor | `crates/terraphim_orchestrator/src/flow/executor.rs:67-84` | `FlowExecutor::new(working_dir, flow_state_dir)` |
| Flow executor agent | `crates/terraphim_orchestrator/src/flow/executor.rs:249-446` | `execute_agent()` - uses `self.working_dir` |
| Flow executor action | `crates/terraphim_orchestrator/src/flow/executor.rs:114-178` | `execute_action()` - uses `self.working_dir` |
| adf-ctl flow | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:947-1020` | `cmd_flow()` - initializes executor |
| Spawner CWD | `crates/terraphim_spawner/src/lib.rs:657-662` | Priority: ctx > config > spawner default |

### Issue 1: Review Step Configuration

From `zdp-validate-pipeline.toml`:
```toml
[[steps]]
name = "review"
kind = "agent"
cli_tool = "opencode"
model = "kimi-for-coding/k2p6"
timeout_secs = 600
```

The task prompt is 2000+ characters including the full template for `review-findings.md`. The agent must:
1. Read implementation-status.md
2. Read design.md
3. Run `git diff main..task/1887-validation`
4. Analyse test code
5. Write findings with severity

### Issue 2: Working Directory Chain

```
cmd_flow()
  -> cwd = env::current_dir()           # /opt/ai-dark-factory (wrong!)
  -> ProjectRuntime.working_dir = cwd   # /opt/ai-dark-factory
  -> FlowExecutor::new(cwd, ...)        # /opt/ai-dark-factory
  -> execute_agent()
    -> provider.working_dir = self.working_dir  # /opt/ai-dark-factory
    -> spawn_context_for_flow()
      -> ctx.working_dir = runtime.working_dir  # /opt/ai-dark-factory
    -> spawner.spawn_with_fallback()
      -> cmd.current_dir(ctx.working_dir)       # /opt/ai-dark-factory
```

The `flow.repo_path = "/data/projects/terraphim/terraphim-ai"` is never read.

## Constraints

### Technical Constraints
- `flow.repo_path` is a `String`, not a `PathBuf` (legacy design)
- `FlowExecutor::new()` takes ownership of `working_dir: PathBuf`
- Spawner working dir priority is fixed: ctx > config > default
- `cmd_flow` already discovers flow file from cwd upward - cannot easily change cwd

### Business Constraints
- Must not break existing flows that rely on cwd behaviour
- Must maintain backward compatibility with existing flow TOML files
- Changes should be minimal (this is a bug fix, not redesign)

## Vital Few

### Essential Constraints
1. **Working directory must respect `repo_path`**: The flow author's intent must be honoured
2. **Review timeout must scale with change size**: Small changes should not wait 10 minutes
3. **Task prompts should use `task_file`**: Inline templates bloat prompts

### Eliminated from Scope
- Rewriting the spawner's working directory priority chain (works correctly, just fed wrong input)
- Adding async file watching for artefact detection (too complex)
- Changing the flow file format (unnecessary)

## Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_spawner` | Uses SpawnContext.working_dir | Low - already correct |
| `tokio::process::Command` | Uses `.current_dir()` | Low - standard API |
| `toml` crate | Parses flow definition | Low - already used |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Relative `repo_path` breaks when flow file moved | Medium | Medium | Resolve relative to flow file parent dir |
| Existing flows depend on cwd behaviour | Low | High | Only use `repo_path` if it's absolute and exists |
| Shorter timeouts cause premature failures | Low | Medium | Use 300s for review, 600s for implementation |

### Assumptions
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `repo_path` should override cwd when set | Flow TOML has explicit field | Flows break if cwd was intentional | No - this IS the bug |
| Review of <500 lines should take <3 min | Empirical from #1887 | Model API could be slower | Partial - review took 6+ min |

## Research Findings

### Key Insight 1: `repo_path` is Completely Ignored

Searching the codebase:
```bash
grep -r "repo_path" crates/terraphim_orchestrator/src/flow/
```

Only found:
- `config.rs:58` - field definition
- `config.rs` - test fixtures using `/tmp/repo`
- **No usage in `executor.rs` or `adf-ctl.rs`**

This confirms `repo_path` was never wired up.

### Key Insight 2: Review Prompt is 90% Template

The review task prompt in `zdp-validate-pipeline.toml` is ~3000 characters. ~2500 characters are the output template (markdown structure, tables, headers). Only ~500 characters are actual instructions.

This is a known anti-pattern: **Prompts should not contain output templates**. Templates should be in `task_file` references or generated by the agent.

### Key Insight 3: Timeout is Blanket 600s for All Steps

All agent steps use 600s timeout:
- Research: 600s (completed in ~3 min)
- Design: 600s (completed in ~2 min)
- Implementation: 3600s (completed in ~4 min)
- Review: 600s (took 6+ min, likely because of prompt bloat)

There is no correlation between timeout and step complexity.

### Key Insight 4: Spawner Working Directory Logic is Correct

```rust
// crates/terraphim_spawner/src/lib.rs:657-662
let working_dir = ctx
    .working_dir
    .or(config.working_dir.as_ref())
    .unwrap_or(&self.default_working_dir);
cmd.current_dir(working_dir)
```

The spawner correctly prioritises context > config > default. The bug is upstream: `cmd_flow` passes the wrong working directory into the executor.

## Recommendations

### Proceed: Yes

Both issues are well-understood, have clear fixes, and block reliable ADF operation.

### Scope

**In Scope:**
1. Fix `cmd_flow` to use `flow.repo_path` for working directory
2. Reduce review step timeout from 600s to 300s
3. Extract review output template to `task_file` 
4. Update `zdp-validate-pipeline.toml` with corrected paths and timeouts

**Out of Scope:**
- Spawner changes (works correctly)
- Flow format changes (keep `repo_path` as String)
- New features (auto-timeout scaling, etc.)

## Next Steps

1. Create design document with specific file changes
2. Implement fixes
3. Update flow definition
4. Re-run validation to confirm
