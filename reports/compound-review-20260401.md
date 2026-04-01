# Compound Code Review Report
**Date:** 2026-04-01
**Reviewer:** Carthos (Domain Architect)
**Scope:** Commits HEAD~10..HEAD, PRs #164-169

---

## Executive Summary

The recent changes introduce significant architectural capabilities: a Flow DAG engine for orchestrated multi-step workflows and an inter-agent mention system for event-driven task dispatch. However, **TWO critical compilation errors** block the build, requiring immediate attention before deployment.

**Verdict:** BLOCKED - Critical compilation errors (COMP-001, COMP-002) must be resolved.

**Blocking Issues:**
- COMP-001: `terraphim_automata/src/lib.rs:394` - u64->String type mismatch
- COMP-002: `terraphim_rolegraph/src/medical.rs:165` - u64->String type mismatch

---

## 1. Architectural Analysis

### 1.1 Flow DAG Engine (PR #164, commit 644fedca)

**Bounded Context:** `terraphim_orchestrator::flow`

The Flow DAG engine represents a well-designed bounded context with clear aggregate roots:

| Component | Role | Assessment |
|-----------|------|------------|
| `FlowDefinition` | Configuration aggregate | Clean separation of config from execution |
| `FlowRunState` | State aggregate | Proper correlation ID, idempotent persistence |
| `FlowExecutor` | Domain service | Correct async orchestration patterns |
| `StepEnvelope` | Value object | Encapsulates step output with clear boundaries |

**Architectural Alignment:**
- Follows the existing pattern of `AgentOrchestrator` for orchestration
- Integrates with `AgentSpawner` through dependency injection
- Uses `tokio::task::spawn` for non-blocking flow execution
- Checkpoint/state persistence follows the ledger pattern established by `HandoffLedger`

**Invariant Preservation:**
- Flow timeout enforced at engine level (default 3600s)
- Step-level timeouts with configurable fail strategies
- Atomic state transitions (Running -> Completed/Failed/Aborted)

### 1.2 Inter-Agent Mention System (PRs #145-147, #169)

**Bounded Context:** `terraphim_orchestrator::mention`

The mention detection system establishes a new communication channel between external systems (Gitea issues) and agent dispatch:

| Component | Responsibility |
|-----------|---------------|
| `MentionDetector` | Parse `@adf:agent-name` patterns from comments |
| `MentionTracker` | Deduplication and depth limiting (prevents infinite loops) |
| `OutputPoster` | Post agent output back to originating issues |

**System Relationships:**
```
Gitea Issue -> MentionDetector -> Dispatcher -> AgentSpawner -> OutputPoster -> Gitea Issue
```

This forms a closed feedback loop with appropriate guards:
- `max_mention_depth` prevents runaway dispatch
- `poll_modulo` reduces API traffic
- `is_processed()` deduplicates across ticks

### 1.3 Wall-Clock Timeout with Fallback (commits 962b31f2, 53c05e0b)

**Domain Concern:** Resource management under API rate limiting

The implementation correctly handles the case where LLM endpoints return 429 errors:

1. `poll_wall_timeouts()` detects agents exceeding `max_cpu_seconds`
2. Kills the stuck agent
3. Respawns with `fallback_provider` / `fallback_model`
4. Clears provider field to avoid model prefix composition errors

**Bug Fix Quality:** The follow-up commit (53c05e0b) demonstrates proper root cause analysis - the initial implementation caused `ProviderModelNotFoundError` due to incorrect model string composition.

---

## 2. Code Quality Assessment

### 2.1 Critical Issues

#### COMP-001: Compilation Error in terraphim_automata
**Severity:** BLOCKER
**Location:** `crates/terraphim_automata/src/lib.rs:394`
**Root Cause:** Commit e0f98ee6 changed `NormalizedTerm.id` from `u64` to `String`, but the legacy import code still passes `term.id` (u64) to `NormalizedTerm::new()`.

```rust
// Line 394 - BROKEN
NormalizedTerm::new(term.id, NormalizedTermValue::from(key.as_str()))
// term.id is u64, but new() expects impl Into<String>
```

**Fix Required:**
```rust
NormalizedTerm::new(term.id.to_string(), NormalizedTermValue::from(key.as_str()))
```

#### COMP-002: Compilation Error in terraphim_rolegraph (NEW - 2026-04-01 20:39 CEST)
**Severity:** BLOCKER
**Location:** `crates/terraphim_rolegraph/src/medical.rs:165`
**Root Cause:** The `magic_pair` function signature changed to expect `AsRef<str>`, but `MedicalGraph::get_edge_type()` still calls it with `u64` arguments.

```rust
// Line 165 - BROKEN
pub fn get_edge_type(&self, source: u64, target: u64) -> Option<MedicalEdgeType> {
    let edge_id = magic_pair(source, target);  // u64 doesn't impl AsRef<str>
    self.edge_types.get(&edge_id).copied()
}
```

**Fix Required:**
```rust
pub fn get_edge_type(&self, source: u64, target: u64) -> Option<MedicalEdgeType> {
    let edge_id = magic_pair(source.to_string(), target.to_string());
    self.edge_types.get(&edge_id).copied()
}
```

**Note:** This is the same u64->String migration issue as COMP-001. The type change in commit e0f98ee6 was not fully propagated through all dependent code. A comprehensive audit of all `magic_pair` callers is needed.

**Additional Broken Callers Found:**
| Location | Line | Status |
|----------|------|--------|
| `medical.rs` | 131 | BROKEN - `add_edge()` call with u64 |
| `medical.rs` | 486 | BROKEN - test with numeric literals |
| `terraphim_rolegraph_py/lib.rs` | 502-503 | INCOMPATIBLE - Python bindings expect u64 return |
| `terraphim_github_runner/learning/knowledge_graph.rs` | 234 | NEEDS AUDIT - caller type unknown |

### 2.2 Style Issues

#### STYLE-001: Trailing Whitespace in flow/executor.rs
**Severity:** LOW
**Locations:** Lines 193, 708, 717, 719, 734, 736, 746, 755, 757, 772
**Fix:** Run `cargo fmt` or configure pre-commit hooks to catch this.

### 2.3 Positive Observations

1. **Commit Message Quality:** All recent commits follow conventional format with proper issue references
2. **Defensive Coding:** Pre-check strategies (git-diff, gitea-issue, shell) use fail-open semantics
3. **Error Propagation:** Consistent use of `OrchestratorError` enum with context
4. **Test Coverage:** New modules include unit tests (flow/state.rs, flow/config.rs)
5. **Documentation:** Module-level doc comments explain purpose and usage

---

## 3. Cross-Reference with Architectural Principles

### 3.1 Alignment with System Architecture

From `docs/src/Architecture.md`, the orchestrator sits in the **Service Layer**:

```
Application Layer -> Service Layer (Orchestrator) -> Domain Layer -> Infrastructure Layer
```

The recent changes maintain this separation:
- Flow engine does not reach into infrastructure directly
- Mention system uses `GiteaTracker` abstraction
- Fallback mechanism uses existing `AgentSpawner` interface

### 3.2 Design Decision Compliance

Per the project's emphasis on:

| Principle | Status | Evidence |
|-----------|--------|----------|
| Privacy-first | Maintained | No external data transmission beyond configured Gitea |
| Local processing | Maintained | All orchestration runs locally |
| Async patterns | Maintained | Proper use of tokio throughout |
| Error handling | Maintained | `anyhow`/`thiserror` patterns preserved |
| No mocks in tests | Maintained | Tests use real structures |

### 3.3 Emerging Patterns

The codebase is developing consistent patterns that should be documented:

1. **Pre-check Gate Pattern:** Agents can define pre-flight checks to skip unnecessary work
2. **Fallback Chain Pattern:** Primary/fallback provider with automatic failover
3. **Ledger Pattern:** Append-only JSONL files for audit trails (handoff-ledger, flow state)
4. **Ticker Pattern:** Modular poll functions called from `reconcile_tick()`

---

## 4. PR Quality Matrix

| PR | Title | Merged | Architecture | Tests | Docs | Issues |
|----|-------|--------|--------------|-------|------|--------|
| #164 | Flow DAG engine | Yes | Excellent | Good | Good | None |
| #167 | GiteaTracker comments | Yes | Good | 6 wiremock tests | Minimal | None |
| #168 | OutputPoster | Yes | Good | Unit tests | Minimal | None |
| #169 | MentionDetector | Yes | Good | Unit tests | Good | None |

---

## 5. Recommendations

### 5.1 Immediate Actions (Before Merge)

1. **FIX COMP-001:** Add `.to_string()` conversion in terraphim_automata legacy import
2. **FIX COMP-002:** Add `.to_string()` conversion in terraphim_rolegraph medical.rs (NEW)
3. **AUDIT:** Search for all `magic_pair` callers to ensure complete migration
4. **FIX STYLE-001:** Run `cargo fmt` to remove trailing whitespace
5. **VERIFY:** Run `cargo build --workspace` to confirm clean compilation
6. **VERIFY:** Run `cargo test --workspace` to confirm all tests pass

### 5.2 Short-term Improvements

1. **Add ADR:** Document the Flow DAG architecture decision in `docs/adr/`
2. **Add Integration Test:** Test full mention -> dispatch -> output post cycle
3. **Consider Circuit Breaker:** The `circuit_breakers` field exists but is unused - document intended use

### 5.3 Long-term Considerations

1. **Flow Visualization:** Consider generating DOT graphs from FlowDefinition for debugging
2. **Mention ACL:** Add access control for which agents can be mentioned by which issues
3. **Metrics Export:** Expose orchestrator metrics (restart counts, budget usage) via Prometheus

---

## 6. Traceability

### Commits Reviewed
- d1d8e961 fix: resolve rebase artefacts from merge conflict resolution
- 53c05e0b fix(orchestrator): clear provider on fallback respawn
- 962b31f2 feat(orchestrator): add wall-clock timeout with fallback respawn
- 2d90f050 fix(orchestrator): wire spawn_agent into poll_mentions dispatch
- 9d243fe5 feat(orchestrator): add MentionDetector for @adf: mention dispatch
- ae6b8876 feat(orchestrator): add OutputPoster and drain-before-drop
- 9f99f886 feat(tracker): add post_comment and fetch_comments to GiteaTracker
- e0f98ee6 fix(terraphim_types): Change Concept and NormalizedTerm IDs from u64 to String
- 2008a3f4 fix(flow): wait for agent process exit instead of immediate SIGTERM
- 644fedca feat(flow): Flow DAG engine with config, execution, checkpoint

### Issues Referenced
- #141: Type ID changes (partial - compilation error introduced)
- #144: Wall-clock timeout with fallback
- #145: GiteaTracker comment methods
- #146: OutputPoster implementation
- #147: MentionDetector implementation
- #163: Flow agent wait fix
- #164: Flow DAG engine

---

## 7. Conclusion

The architectural direction is sound. The Flow DAG engine and mention system add powerful orchestration capabilities while maintaining the system's privacy-first, local-processing ethos. However, the incomplete u64->String migration has created TWO compilation blockers (COMP-001 in terraphim_automata, COMP-002 in terraphim_rolegraph) that must be resolved before these changes can be deployed.

**Type Migration Status:**
- `terraphim_types`: Changed u64 -> String (commit e0f98ee6)
- `terraphim_automata`: BROKEN - needs `.to_string()` at line 394
- `terraphim_rolegraph`: BROKEN - needs `.to_string()` in medical.rs:165
- Other crates: AUDIT REQUIRED

**Next Gate:** Compilation must succeed before this work can progress to the Deploy stage.

---

*Generated by Carthos, Domain Architect*
*Compass rose: Orientation in complexity*
*Updated: 2026-04-01 20:39 CEST with COMP-002 finding*
