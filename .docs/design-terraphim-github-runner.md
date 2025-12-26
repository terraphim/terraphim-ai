# Design & Implementation Plan: Terraphim Agent as GitHub Runner

## 1. Summary of Target Behavior

After implementation, the system will:

1. **Receive GitHub webhooks** (PR open/sync, push events) via the existing `github_webhook` server
2. **Spawn a Firecracker VM** from a prewarmed pool within sub-2 seconds
3. **Execute workflow commands** inside the isolated VM using terraphim-agent
4. **Create snapshots** after each successful command execution
5. **Track command history** with success/failure metrics and rollback capability
6. **Update the knowledge graph** with learned patterns:
   - Successful command sequences → success patterns
   - Failed commands → failure lessons with prevention strategies
   - Path optimization → increase weights on successful paths
7. **Report results** back to GitHub PR as comments

### System Flow Diagram

```
GitHub Event → Webhook Handler → VM Allocator → Firecracker VM
                    ↓                              ↓
              Parse Workflow               Terraphim Agent
                    ↓                              ↓
              Queue Commands ──────────→ Execute Command
                                               ↓
                                    ┌──────────┴──────────┐
                                Success                 Failure
                                    ↓                      ↓
                           Take Snapshot          Rollback to Last Good
                                    ↓                      ↓
                           Next Command           Record Failure Lesson
                                    ↓                      ↓
                           Update KG (+)           Update KG (-)
                                    ↓                      ↓
                              Continue...          Report & Retry/Abort
```

---

## 2. Key Invariants and Acceptance Criteria

### Data Consistency Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **INV-1** | Each workflow execution has unique session ID | UUID generation at session start |
| **INV-2** | Snapshots are immutable once created | Copy-on-write storage |
| **INV-3** | Command history is append-only | Versioned writes, no deletes |
| **INV-4** | Knowledge graph updates are atomic | Transaction wrapper |

### Security Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **SEC-1** | Webhooks are verified via HMAC-SHA256 | Existing signature check |
| **SEC-2** | Secrets never persist to snapshots | Inject at runtime, memory-only |
| **SEC-3** | VMs are isolated from host | Firecracker containment |
| **SEC-4** | Each workflow gets fresh VM state | Restore from base snapshot |

### Performance SLOs

| ID | SLO | Measurement |
|----|-----|-------------|
| **PERF-1** | VM allocation < 500ms | Pool hit time |
| **PERF-2** | VM boot < 2 seconds | First command ready time |
| **PERF-3** | Snapshot creation < 1 second | Checkpoint duration |
| **PERF-4** | Rollback < 2 seconds | Restore + verify time |

### Acceptance Criteria

| ID | Criterion | Test Type |
|----|-----------|-----------|
| **AC-1** | PR webhook triggers VM execution and posts result | Integration |
| **AC-2** | Each successful command creates a snapshot | Integration |
| **AC-3** | Failed command triggers rollback to last snapshot | Integration |
| **AC-4** | Command history persists across restarts | Persistence |
| **AC-5** | Repeated failures add lesson to knowledge graph | Integration |
| **AC-6** | Successful patterns increase path weight in KG | Integration |
| **AC-7** | System handles 10 concurrent workflows | Load |

---

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                        GitHub (External)                           │
└────────────────────────────┬───────────────────────────────────────┘
                             │ Webhook POST
                             ▼
┌────────────────────────────────────────────────────────────────────┐
│  github_webhook (Extended)                                         │
│  ├── Signature verification (existing)                             │
│  ├── Event parsing (existing)                                      │
│  └── WorkflowOrchestrator (NEW) ◀────────────────────────────────┐ │
└────────────────────────────┬───────────────────────────────────────┘
                             │
                             ▼
┌────────────────────────────────────────────────────────────────────┐
│  terraphim_github_runner (NEW CRATE)                               │
│  ├── WorkflowParser: Parse GitHub workflow YAML                    │
│  ├── WorkflowExecutor: Coordinate command execution                │
│  ├── SessionManager: Manage agent-VM bindings                      │
│  └── LearningCoordinator: Update knowledge graph from outcomes     │
└───────────┬────────────────────────────────────────────────────────┘
            │                    │
            ▼                    ▼
┌────────────────────┐  ┌────────────────────────────────────────────┐
│ terraphim_firecracker│  │ terraphim_multi_agent                      │
│ (Existing)           │  │ ├── FcctlBridge (snapshots, history)       │
│ ├── VmPoolManager   │  │ ├── CommandHistory (tracking)              │
│ └── Sub2SecondVM    │  │ └── VmExecuteRequest/Response              │
└──────────┬───────────┘  └───────────────┬───────────────────────────┘
           │                              │
           ▼                              ▼
┌────────────────────────────────────────────────────────────────────┐
│  Firecracker VM                                                     │
│  └── terraphim-agent (running inside VM)                           │
│       ├── REPL command execution                                    │
│       └── Result reporting                                          │
└────────────────────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────────────────────┐
│  Learning & Persistence                                             │
│  ├── terraphim_agent_evolution (LessonsEvolution)                  │
│  ├── terraphim_rolegraph (RoleGraph - Knowledge Graph)             │
│  └── terraphim_persistence (State storage)                         │
└────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Changes Required |
|-----------|----------------|------------------|
| **github_webhook** | Receive/verify webhooks, trigger execution | Extend to call WorkflowOrchestrator |
| **terraphim_github_runner** (NEW) | Parse workflows, coordinate execution, learning | New crate |
| **terraphim_firecracker** | VM lifecycle, pooling, prewarming | Minor: expose allocation API |
| **terraphim_multi_agent** | VM session management, history | Extend: learning integration |
| **terraphim_agent_evolution** | Lessons management | Extend: GitHub-specific lessons |
| **terraphim_rolegraph** | Knowledge graph, pattern matching | Extend: path weighting |

### Boundaries and Interfaces

```rust
// Interface: github_webhook → terraphim_github_runner
pub trait WorkflowOrchestrator {
    async fn execute_workflow(&self, event: GitHubEvent) -> WorkflowResult;
}

// Interface: terraphim_github_runner → terraphim_firecracker
pub trait VmAllocator {
    async fn allocate_vm(&self, vm_type: &str) -> Result<VmSession>;
    async fn release_vm(&self, session: VmSession) -> Result<()>;
}

// Interface: terraphim_github_runner → terraphim_multi_agent
pub trait ExecutionTracker {
    async fn execute_in_vm(&self, session: &VmSession, command: &str) -> ExecutionResult;
    async fn create_checkpoint(&self, session: &VmSession) -> Result<SnapshotId>;
    async fn rollback(&self, session: &VmSession, snapshot: SnapshotId) -> Result<()>;
}

// Interface: terraphim_github_runner → Learning
pub trait LearningCoordinator {
    async fn record_success(&self, command: &str, context: &WorkflowContext);
    async fn record_failure(&self, command: &str, error: &str, context: &WorkflowContext);
    async fn suggest_optimizations(&self, workflow: &Workflow) -> Vec<Optimization>;
}
```

---

## 4. File/Module-Level Change Plan

### New Crate: `terraphim_github_runner`

| File | Action | Purpose | Dependencies |
|------|--------|---------|--------------|
| `crates/terraphim_github_runner/Cargo.toml` | Create | Crate manifest | workspace deps |
| `crates/terraphim_github_runner/src/lib.rs` | Create | Crate entry, exports | - |
| `crates/terraphim_github_runner/src/workflow/mod.rs` | Create | Workflow module | - |
| `crates/terraphim_github_runner/src/workflow/parser.rs` | Create | LLM-based workflow understanding | terraphim_service::llm |
| `crates/terraphim_github_runner/src/workflow/executor.rs` | Create | Execute workflow steps | FcctlBridge |
| `crates/terraphim_github_runner/src/session/mod.rs` | Create | Session module | - |
| `crates/terraphim_github_runner/src/session/manager.rs` | Create | Manage agent-VM sessions | terraphim_firecracker |
| `crates/terraphim_github_runner/src/learning/mod.rs` | Create | Learning module | - |
| `crates/terraphim_github_runner/src/learning/coordinator.rs` | Create | Coordinate KG updates | terraphim_agent_evolution |
| `crates/terraphim_github_runner/src/learning/patterns.rs` | Create | Pattern extraction | terraphim_rolegraph |
| `crates/terraphim_github_runner/src/models.rs` | Create | Data types | serde |
| `crates/terraphim_github_runner/src/error.rs` | Create | Error types | thiserror |

### Existing Crate Modifications

#### github_webhook

| File | Action | Before | After |
|------|--------|--------|-------|
| `github_webhook/src/main.rs` | Modify | Execute bash script directly | Call WorkflowOrchestrator |
| `github_webhook/src/orchestrator.rs` | Create | - | Integration with terraphim_github_runner |
| `github_webhook/Cargo.toml` | Modify | Current deps | Add terraphim_github_runner dep |

#### terraphim_firecracker

| File | Action | Before | After |
|------|--------|--------|-------|
| `terraphim_firecracker/src/lib.rs` | Modify | Binary-only | Export manager as library |
| `terraphim_firecracker/src/pool/mod.rs` | Modify | Internal pool API | Public allocation API |

#### terraphim_multi_agent

| File | Action | Before | After |
|------|--------|--------|-------|
| `crates/terraphim_multi_agent/src/vm_execution/fcctl_bridge.rs` | Modify | HTTP/direct modes | Add learning hooks |
| `crates/terraphim_multi_agent/src/history.rs` | Modify | Command tracking only | Add pattern extraction |

#### terraphim_rolegraph

| File | Action | Before | After |
|------|--------|--------|-------|
| `crates/terraphim_rolegraph/src/lib.rs` | Modify | Static edges | Add edge weight updates |
| `crates/terraphim_rolegraph/src/weights.rs` | Create | - | Path weight management |

#### terraphim_agent_evolution

| File | Action | Before | After |
|------|--------|--------|-------|
| `crates/terraphim_agent_evolution/src/lessons.rs` | Modify | Generic lessons | Add GitHub-specific categories |
| `crates/terraphim_agent_evolution/src/github.rs` | Create | - | GitHub workflow lessons |

---

## 5. Step-by-Step Implementation Sequence

### Phase 1: Foundation (Estimated: 2-3 steps)

#### Step 1.1: Create terraphim_github_runner crate skeleton
- **Purpose**: Establish crate structure and basic types
- **Deliverable**: Compiling crate with models and error types
- **Deployable**: Yes (no behavior change)
- **Files**: Cargo.toml, lib.rs, models.rs, error.rs

#### Step 1.2: Export terraphim_firecracker as library
- **Purpose**: Enable VM allocation from external crates
- **Deliverable**: Public API for VmPoolManager
- **Deployable**: Yes (backward compatible)
- **Files**: terraphim_firecracker/src/lib.rs, pool/mod.rs

#### Step 1.3: Add LLM-based workflow understanding
- **Purpose**: Use LLM to parse and translate GitHub Actions workflows into executable commands
- **Deliverable**: WorkflowParser using terraphim_service::llm to understand workflow intent
- **Deployable**: Yes (new feature, no change to existing)
- **Files**: workflow/parser.rs, tests
- **LLM Prompt Strategy**: System prompt defines GitHub Actions context, user prompt is workflow YAML, response is executable command sequence

### Phase 2: Core Execution (Estimated: 3-4 steps)

#### Step 2.1: Implement SessionManager
- **Purpose**: Manage VM allocation lifecycle for workflows
- **Deliverable**: Allocate/release VMs with session tracking
- **Deployable**: Yes (internal component)
- **Files**: session/manager.rs

#### Step 2.2: Implement WorkflowExecutor
- **Purpose**: Execute workflow steps in sequence with snapshots
- **Deliverable**: Step-by-step execution with checkpoint after success
- **Deployable**: Yes (internal component)
- **Files**: workflow/executor.rs
- **Depends on**: Step 2.1, FcctlBridge

#### Step 2.3: Integrate with github_webhook
- **Purpose**: Connect webhook handler to workflow execution
- **Deliverable**: Webhook triggers VM execution
- **Deployable**: Yes (feature flag recommended)
- **Files**: github_webhook/src/orchestrator.rs, main.rs

#### Step 2.4: Add result posting back to GitHub
- **Purpose**: Post execution results as PR comments
- **Deliverable**: Success/failure comments with logs
- **Deployable**: Yes (completes basic flow)
- **Files**: github_webhook/src/main.rs (existing post_pr_comment)

### Phase 3: Learning Integration (Estimated: 3 steps)

#### Step 3.1: Implement LearningCoordinator
- **Purpose**: Coordinate recording successes and failures
- **Deliverable**: Record outcomes with context
- **Deployable**: Yes (learning starts)
- **Files**: learning/coordinator.rs

#### Step 3.2: Add pattern extraction from history
- **Purpose**: Extract success/failure patterns from command history
- **Deliverable**: Pattern analysis with lessons creation
- **Deployable**: Yes (enhances learning)
- **Files**: learning/patterns.rs, history.rs modifications

#### Step 3.3: Knowledge graph weight updates
- **Purpose**: Update edge weights based on execution outcomes
- **Deliverable**: Successful paths get higher weights
- **Deployable**: Yes (improves recommendations)
- **Files**: terraphim_rolegraph/src/weights.rs, lib.rs modifications

### Phase 4: Advanced Features (Estimated: 2-3 steps)

#### Step 4.1: Add rollback-on-failure automation
- **Purpose**: Automatic rollback when command fails
- **Deliverable**: Auto-rollback with notification
- **Deployable**: Yes (improves reliability)
- **Files**: workflow/executor.rs modifications

#### Step 4.2: Add optimization suggestions
- **Purpose**: Suggest workflow improvements from learned patterns
- **Deliverable**: Optional optimization hints in PR comments
- **Deployable**: Yes (new feature)
- **Files**: learning/coordinator.rs modifications

#### Step 4.3: Concurrent workflow support
- **Purpose**: Handle multiple workflows simultaneously
- **Deliverable**: Queue and execute multiple workflows
- **Deployable**: Yes (scalability)
- **Files**: Multiple modifications for concurrency

---

## 6. Testing & Verification Strategy

### Unit Tests

| Acceptance Criteria | Test Location | Description |
|---------------------|---------------|-------------|
| Workflow YAML parsing | `terraphim_github_runner/src/workflow/parser.rs` | Parse various workflow formats |
| Session lifecycle | `terraphim_github_runner/src/session/manager.rs` | Allocate, use, release VMs |
| Pattern extraction | `terraphim_github_runner/src/learning/patterns.rs` | Extract patterns from history |

### Integration Tests

| Acceptance Criteria | Test Location | Description |
|---------------------|---------------|-------------|
| **AC-1** PR webhook execution | `github_webhook/tests/` | End-to-end webhook to result |
| **AC-2** Snapshot on success | `terraphim_github_runner/tests/` | Verify snapshot creation |
| **AC-3** Rollback on failure | `terraphim_github_runner/tests/` | Inject failure, verify rollback |
| **AC-4** History persistence | `terraphim_multi_agent/tests/` | Restart, verify history |

### System Tests

| Acceptance Criteria | Test Location | Description |
|---------------------|---------------|-------------|
| **AC-5** Failure → lesson | `tests/learning_e2e.rs` | Multiple failures create lesson |
| **AC-6** Success → weight | `tests/learning_e2e.rs` | Success increases path weight |
| **AC-7** Concurrent workflows | `tests/concurrent_e2e.rs` | 10 parallel workflow execution |

### Test Data

```yaml
# fixtures/test_workflow.yml
name: Test Workflow
on: [push]
jobs:
  build:
    runs-on: self-hosted
    steps:
      - name: Checkout
        run: git clone $REPO
      - name: Build
        run: cargo build
      - name: Test
        run: cargo test
```

---

## 7. Risk & Complexity Review

### Risks from Phase 1 Research

| Risk | Mitigation in Design | Residual Risk |
|------|---------------------|---------------|
| **R-SNAPSHOT-CORRUPT** | Verify snapshot integrity before restore; keep 3 most recent | Low - data loss if all corrupt |
| **R-VM-LEAK** | Session timeout (30 min); background cleanup task | Low - manual cleanup needed rarely |
| **R-KNOWLEDGE-DRIFT** | Decay old lessons; confidence thresholds | Medium - may need tuning |
| **R-RACE-CONDITIONS** | Per-session locks; workflow queue with bounded concurrency | Low - serialization overhead |
| **R-SLOW-LEARNING** | Curated initial patterns; threshold of 3 failures | Medium - cold start period |
| **R-FALSE-POSITIVES** | Require 3+ occurrences; manual review capability | Low - conservative defaults |
| **R-VM-ESCAPE** | Monitor Firecracker CVEs; automatic updates | Low - Firecracker's track record |
| **R-SECRET-LEAK** | In-memory only; no secret in snapshots | Very Low - enforced by design |

### New Risks from Design

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Workflow YAML complexity** | High | Medium | Support subset; document limitations |
| **Integration complexity** | Medium | Medium | Clear interfaces; incremental delivery |
| **Performance regression** | Low | Medium | Benchmarks in CI; profiling |

### Complexity Assessment

| Area | Complexity | Reason | Simplification |
|------|------------|--------|----------------|
| Workflow parsing | Medium | YAML variety | Support bash-only initially |
| VM integration | Low | Existing code | Expose existing APIs |
| Learning system | Medium | State management | Async queued updates |
| Knowledge graph | Medium | Weight calculations | Simple increment/decay |

---

## 8. Open Questions / Decisions for Human Review

### Decision 1: Workflow Parsing Scope
**Question**: How much GitHub Actions YAML syntax should we support initially?

**Options**:
1. **Minimal**: Only `run:` steps with bash commands
2. **Moderate**: Add `uses:` for common actions (checkout, setup-*)
3. **Full**: Complete GitHub Actions compatibility
4. **LLM-based**: Use LLMs to understand and translate workflows

**DECISION: LLM-based** - Use terraphim's existing LLM integration to parse and understand GitHub Actions workflows, translating them into executable commands. This provides flexibility and natural language understanding.

### Decision 2: Snapshot Strategy
**Question**: When exactly should snapshots be created?

**Options**:
1. **Per-command**: After every successful `run:` step
2. **Per-job**: After each job completes successfully
3. **Per-workflow**: Only at workflow completion

**DECISION: Per-command** - Maximum recoverability with fine-grained rollback points.

### Decision 3: Learning Threshold
**Question**: How many failures before creating a lesson?

**Options**:
1. **Conservative**: 3 identical failures
2. **Aggressive**: 1 failure creates tentative lesson
3. **Statistical**: Based on failure rate percentage

**DECISION: 3 failures** - Conservative approach requiring 3 identical failures before creating a lesson.

### Decision 4: Crate Location
**Question**: Where should `terraphim_github_runner` live?

**Options**:
1. **Workspace crate**: `crates/terraphim_github_runner/`
2. **Separate repo**: New repository linked to github_webhook
3. **In github_webhook**: Extend existing repo

**DECISION: Workspace crate** - Located at `crates/terraphim_github_runner/` for better integration.

### Decision 5: Feature Flag
**Question**: Should the new functionality be behind a feature flag?

**Options**:
1. **Yes**: `--features github-runner`
2. **No**: Always enabled once merged

**DECISION: Yes** - Feature flag `github-runner` for safe rollout.

---

## Summary

This design leverages substantial existing infrastructure:
- **FcctlBridge**: Already has snapshot/history/rollback
- **LessonsEvolution**: Already has failure/success pattern storage
- **RoleGraph**: Already has pattern matching infrastructure

**Primary work is integration**:
1. New crate `terraphim_github_runner` (~1200 LOC estimated)
2. Extensions to existing crates (~300 LOC estimated)
3. Integration with github_webhook (~200 LOC estimated)

**Phased delivery** ensures each step is deployable and testable.

---

**Do you approve this plan as-is, or would you like to adjust any part?**

---

*Design completed: 2025-12-23*
*Phase 2 Disciplined Development*
