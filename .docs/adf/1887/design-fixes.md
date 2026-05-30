# Implementation Plan: Fix Flow Working Directory and Review Timeout

**Status**: Approved
**Research Doc**: `.docs/adf/1887/research-fixes.md`
**Author**: ADF Investigation Agent
**Date**: 2026-05-30
**Estimated Effort**: 2 hours

## Overview

### Summary
Fix two critical bugs in the ADF flow execution:
1. `flow.repo_path` is ignored - agents always run in the current working directory
2. Review step timeout is excessive (600s) and prompt contains inline templates causing slowness

### Approach
Minimal surgical fixes:
1. In `cmd_flow`, resolve `flow.repo_path` and use it for both `FlowExecutor::new()` and `ProjectRuntime.working_dir`
2. In `zdp-validate-pipeline.toml`, reduce review timeout to 300s and extract the review output template to a separate file

### Scope

**In Scope:**
- Fix `cmd_flow` working directory initialisation
- Extract review task template to `task_file`
- Reduce review step timeout
- Update flow TOML with corrected paths

**Out of Scope:**
- Spawner changes (works correctly)
- Flow format/schema changes
- New features

**Avoid At All Cost:**
- Rewriting the flow executor (unnecessary complexity)
- Adding configuration options for timeout (YAGNI)
- Changing `repo_path` type from String (breaks existing flows)

## Architecture

### Issue 1: Working Directory Fix

```
cmd_flow()
  BEFORE: cwd = env::current_dir()
          FlowExecutor::new(cwd, ...)
          ProjectRuntime.working_dir = cwd
          
  AFTER:  flow_file_parent = flow_path.parent()
          repo_path = if flow.repo_path.is_absolute() 
                      && exists(flow.repo_path) 
                      => flow.repo_path
                      else => flow_file_parent.join(flow.repo_path)
          FlowExecutor::new(repo_path, ...)
          ProjectRuntime.working_dir = repo_path
```

### Issue 2: Review Timeout and Prompt

```
BEFORE: task = "2000+ character inline template..."
        timeout_secs = 600

AFTER:  task_file = ".terraphim/flows/prompts/review-template.md"
        timeout_secs = 300
```

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Fix `cmd_flow` to use `flow.repo_path` for working directory |
| `.terraphim/flows/zdp-validate-pipeline.toml` | Reduce review timeout, use `task_file` for review template |

### New Files

| File | Purpose |
|------|---------|
| `.terraphim/flows/prompts/review-template.md` | Extracted review output template (reduces prompt size by ~80%) |

## API Design

No new public APIs. Changes are internal to `cmd_flow` and flow configuration.

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_cmd_flow_uses_repo_path` | `adf-ctl.rs` (manual verification) | Verify executor gets correct working dir |
| `test_repo_path_relative_resolution` | Manual | Verify relative repo_path resolves from flow file |

### Integration Tests
| Test | Purpose |
|------|---------|
| Run flow from `/opt/ai-dark-factory` | Verify agents write to `/data/projects/.../.docs/adf/` |
| Review step timing | Verify review completes within 300s |

## Implementation Steps

### Step 1: Fix cmd_flow Working Directory
**Files:** `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`
**Description:** Use `flow.repo_path` instead of `cwd` for executor initialisation
**Dependencies:** None
**Estimated:** 30 minutes

```rust
// BEFORE (lines 988-995):
let project_runtime = ProjectRuntime {
    working_dir: cwd.clone(),
    gitea_owner: Some("terraphim".to_string()),
    gitea_repo: Some("terraphim-ai".to_string()),
};
let executor = FlowExecutor::new(cwd.clone(), flow_state_dir).with_projects(...);

// AFTER:
let repo_path = if std::path::Path::new(&flow.repo_path).is_absolute() {
    std::path::PathBuf::from(&flow.repo_path)
} else {
    flow_path.parent().unwrap_or(&cwd).join(&flow.repo_path)
};
let project_runtime = ProjectRuntime {
    working_dir: repo_path.clone(),
    gitea_owner: Some("terraphim".to_string()),
    gitea_repo: Some("terraphim-ai".to_string()),
};
let executor = FlowExecutor::new(repo_path, flow_state_dir).with_projects(...);
```

### Step 2: Extract Review Template
**Files:** `.terraphim/flows/prompts/review-template.md` (new)
**Description:** Create separate file for review output template
**Dependencies:** None
**Estimated:** 15 minutes

Create `.terraphim/flows/prompts/review-template.md` with the review output structure.

### Step 3: Update Flow Definition
**Files:** `.terraphim/flows/zdp-validate-pipeline.toml`
**Description:** Use `task_file` and reduce timeout
**Dependencies:** Step 2
**Estimated:** 15 minutes

```toml
# Change review step:
[[steps]]
name = "review"
kind = "agent"
cli_tool = "opencode"
model = "kimi-for-coding/k2p6"
provider = "kimi"
timeout_secs = 1200         # Changed from 600 to 20 minutes
on_fail = "continue"
task_file = ".terraphim/flows/prompts/review-template.md"
```

### Step 4: Verify Fix
**Description:** Run flow from non-repo directory and verify agents write to correct location
**Dependencies:** Steps 1-3
**Estimated:** 30 minutes

## Rollback Plan

If issues discovered:
1. Revert `cmd_flow` changes (single function)
2. Restore inline review task in TOML
3. Delete `review-template.md`

## Performance Considerations

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Review prompt size | ~3000 chars | ~500 chars | 6x smaller |
| Review timeout | 600s | 300s | 2x faster failure |
| Working directory correctness | Broken | Fixed | Reliable |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify backward compatibility | Pending | Validation run |

## Approval

- [x] Technical review complete
- [x] Test strategy approved
- [x] Research document approved
