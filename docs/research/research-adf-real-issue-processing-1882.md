# Research Document: ADF Real Issue Processing + k=3 Project Template (#1882)

**Status**: Approved
**Author**: Opencode Session
**Date**: 2026-05-28
**Issues**: #1875 (ADF direct dispatch), #1882 (project template + k=3)

## Executive Summary

The local ADF configuration posts dispatch receipts as Gitea comments instead of producing meaningful issue-specific artefacts. Simultaneously, issue #1882 defines a project template with k=3 boosting at the planning phase. The solution is to replace the placeholder `adf-e2e-stage` script with real stage logic that produces structured artefacts, and express the ZDP pipeline as a `FlowDefinition` using the orchestrator's existing flow engine with matrix fan-out for k=3 parallel proposals.

## Problem Statement

### Problem 1: ADF local dispatch is noise, not work
The 12 local agents in `.terraphim/adf.toml` all call `adf-e2e-stage`, which resolves the issue number, formats a template comment, and posts it via `gtr comment`. No agent reads the issue, classifies it, or produces a research/design/implementation artefact. The proof-of-dispatch comments are indistinguishable from real work in Gitea.

### Problem 2: No k=3 planning pipeline
Issue #1882 defines a project template with k=3 parallel proposals at the research/design phase, but no dispatch mechanism exists. The `terraphim_multi_agent` crate has an `AgentPool` but no fan-out primitive. The `terraphim_orchestrator` has `FlowDefinition` with matrix fan-out, but it is not wired to local agents.

### Impact
- Running ADF on real issues produces 24+ comments of pure noise per cycle
- No way to run k=3 planning boosting locally
- Local ADF config drifts from bigbox config because local is only used for dispatch proofs

## Current State Analysis

### Existing Implementation

| Component | Location | State |
|-----------|----------|-------|
| Local ADF config | `.terraphim/adf.toml` | 12 agents, all using `adf-e2e-stage` |
| Local stage script | `.terraphim/bin/adf-e2e-stage` | Template comment poster, no real work |
| Flow engine | `crates/terraphim_orchestrator/src/flow/` | Full DAG executor with matrix fan-out |
| Flow config types | `flow/config.rs` | `FlowDefinition`, `FlowStepDef`, `MatrixConfig`, `StepKind` |
| Flow executor | `flow/executor.rs` | `FlowExecutor::run()`, `execute_matrix_step()` with sequential sub-steps |
| Flow state | `flow/state.rs` | `FlowRunState`, checkpoint/resume |
| Agent spawner | `crates/terraphim_spawner/` | `SpawnRequest`, `AgentSpawner`, CLI tool spawning |
| Router | `crates/terraphim_router/` | `RoutingEngine`, `RoutingContext.strategy_override` |
| Multi-agent pool | `crates/terraphim_multi_agent/` | `AgentPool`, `LoadBalancingStrategy`, no fan-out |
| Persistence | `crates/terraphim_persistence/` | `DeviceStorage`, `Persistable` trait, no Proposal/Verdict types |
| KG orchestration | `crates/terraphim_kg_orchestration/` | `ExecutionCoordinator`, `MockAutomata` placeholder |
| LSP | `crates/terraphim_lsp/` | Placeholder only, no implementation |
| Compound review | `crates/terraphim_orchestrator/src/compound.rs` | 6 parallel review agents with `JoinSet` |

### Key Insight: Flow Engine Already Supports k=3

The flow engine's `MatrixConfig` provides exactly what k=3 needs:

```toml
[[steps]]
name = "research-proposals"
kind = "agent"
cli_tool = "opencode"
task = "disciplined-research for {{matrix.issue}}"

[steps.matrix]
max_parallel = 3
fail_strategy = "continue"

[[steps.matrix.params]]
issue = "1882"
model = "opus"
provider = "anthropic"

[[steps.matrix.params]]
issue = "1882"
model = "k2p6"
provider = "kimi"

[[steps.matrix.params]]
issue = "1882"
model = "gpt-5.4"
provider = "openai"
```

The matrix step expands into N sub-executions with template substitution. Each gets a different model. The executor collects all envelopes. A downstream gate or agent step can judge the proposals.

### Data Flow (Current vs Proposed)

**Current (noise):**
```
adf-ctl --local trigger <agent> --direct
  -> adf-e2e-stage
  -> pick issue via gtr ready
  -> format template comment
  -> gtr comment (noise)
```

**Proposed (real work):**
```
adf-ctl --local trigger disciplined-research-agent --direct --context "issue=1882"
  -> adf-issue-stage research
  -> read issue body from Gitea API
  -> classify: valid/stale/duplicate/blocked/needs-rescope
  -> write artefact to .docs/adf/1882/research.md
  -> gtr comment with classification + artefact path (only meaningful output)
```

**Proposed (k=3 via FlowDefinition):**
```
FlowDefinition "zdp-research-1882":
  step 1: matrix fan-out (3 agents, 3 models)
    -> .docs/adf/1882/research-proposal-{1,2,3}.md
  step 2: gate (check proposals exist)
  step 3: judge agent (reads proposals, synthesises)
    -> .docs/adf/1882/research-synthesis.md
  step 4: checkpoint (human review gate)
```

## Constraints

### Technical Constraints
- Flow executor runs matrix sub-steps **sequentially** (max_parallel is advisory, not enforced)
- `AgentSpawner` spawns CLI tools (opencode, claude, codex), not in-process agents
- Local `adf-ctl --local trigger` dispatches one agent at a time
- `terraphim_lsp` is unimplemented; drift_check cannot use LSP rules yet
- `terraphim_multi_agent` has no fan-out or voting primitive
- `terraphim_persistence` has no Proposal/Verdict types

### Business Constraints
- Must work on local machine (no bigbox dependency for local ADF)
- Must not post noise to real Gitea issues
- k=3 cost: 3 planning calls + 1 judge per task (subscription models only)
- Matrix sub-steps are sequential today; parallel execution requires code change

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Artefact production | Every stage produces a file | Template comments only |
| Issue classification | Research classifies in <30s | Not implemented |
| k=3 dispatch | 3 parallel proposals | Not implemented |
| Noise ratio | 0 meaningless comments | 100% meaningless comments |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Artefacts over comments | Comments are noise; files are work | #1336 E2E produced 16 comments, 0 artefacts |
| Flow engine for k=3 | Matrix fan-out already exists in flow/ | `FlowDefinition` + `MatrixConfig` in flow/config.rs |
| No proof mode | ADF must always do real work or fail | User directive: "There is no need for proof mode" |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| LSP-based drift_check | terraphim_lsp is placeholder; defer until implemented |
| terraphim_multi_agent fan-out | Flow engine already provides this; avoid duplicate |
| Proposal/Verdict persistence types | File-based artefacts are sufficient; no schema migration needed |
| Parallel matrix execution | Sequential is sufficient for k=3; parallel is future optimisation |
| KG orchestration integration | Uses MockAutomata; not production-ready |
| Cost guard for pay-per-use | Issue #1882 specifies subscription-gated only |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Flow executor not wired to local adf-ctl | Medium | High | Need adapter layer or new `adf-ctl flow` command |
| CLI tools (opencode) cannot produce structured artefacts autonomously | High | High | Stage script produces artefacts; CLI tool is the spawner |
| Matrix sub-steps run sequentially | Low | Low | k=3 sequential is acceptable for planning phase |
| Gitea API rate limits on issue reads | Low | Medium | Cache issue body in local file |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `adf-ctl --local trigger` can pass context to stage script | It passes task string to cli_tool | Stage script cannot receive issue context | Yes (existing behaviour) |
| Flow executor can be invoked locally | `FlowExecutor::new()` takes working_dir | Cannot run flows outside bigbox | Partially (needs binary integration) |
| File artefacts in `.docs/adf/` are sufficient | No structured persistence needed | Artefacts get lost or corrupted | Yes (git tracks them) |
| `gtr comment` is the right channel for meaningful output | Existing Gitea integration | Comments still look like noise | Partially (structured format helps) |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| ADF stages as bash scripts calling CLI tools | Simple, composable, works with adf-ctl | Chosen: matches existing architecture |
| ADF stages as Rust code in orchestrator | Tighter integration, type safety | Rejected: requires recompilation per stage change |
| ADF stages as FlowDefinition | Powerful DAG, matrix fan-out | Chosen for k=3; coexists with script stages |

## Recommendations

### Proceed: Yes

1. Replace `adf-e2e-stage` with `adf-issue-stage` that produces real artefacts
2. Add a ZDP `FlowDefinition` for k=3 planning (research + design phases)
3. Wire flow execution into `adf-ctl` or add `adf-flow-runner` script
4. Remove proof-mode comments entirely

### Scope Recommendations

**In scope (this PR):**
- Replace `adf-e2e-stage` with `adf-issue-stage` (real artefact production)
- Update `.terraphim/adf.toml` agents to use new script
- Add `.terraphim/flows/zdp-research.toml` for k=3 research
- Add `.terraphim/flows/zdp-design.toml` for k=3 design
- Add `.terraphim/boosting.toml` (from #1882)
- Stage artefact contracts: research, design, implementation, review, corrections

**Deferred (follow-up issues):**
- Parallel matrix execution in flow executor
- LSP-based drift_check rules
- Proposal/Verdict typed persistence
- `terraphim_lsp` implementation
- KG orchestration integration
- CI workflow (`drift_check + lsp + tests before merge`)

## Next Steps

1. Create disciplined design document with file changes and API signatures
2. Implement `adf-issue-stage` script
3. Create flow definitions for k=3 planning
4. Update `adf.toml` to use new stage script
5. Validate on a real issue (e.g. #1336 rescope or #1882)
