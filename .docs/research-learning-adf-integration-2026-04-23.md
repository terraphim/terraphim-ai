# Research Document: Wire Learning System into ADF Orchestrator

**Status**: Draft
**Date**: 2026-04-23

## Executive Summary

The learning system (local capture, shared learning store, suggestion approval, cross-agent injection) is fully implemented and tested behind feature gates but completely disconnected from the ADF orchestrator. Agents spawn with zero memory of prior runs. This research identifies the minimal wiring needed to close the loop: inject learnings at spawn, record outcomes at exit, and periodically consolidate.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Agents repeat identical failures across runs -- visible in ADF logs |
| Leverages strengths? | Yes | All learning subsystems already built, tested, feature-gated |
| Meets real need? | Yes | Gitea issues #578 (priority 38), #668 (priority ~5), #179 (epic) all request this |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

`AgentOrchestrator` in `lib.rs` (~7174 lines) has zero references to `terraphim_orchestrator::learning`, `terraphim_agent_evolution::LessonsEvolution`, or `terraphim_agent::shared_learning`. The `spawn_agent()` method builds prompts from persona metaprompts + skill chains + pre-flight findings, but never consults any learning store. The `poll_agent_exits()` method classifies exits via `ExitClassifier` and records `AgentRunRecord`, but never feeds outcomes back.

### Impact

Every ADF agent run starts from scratch. Security-sentinel discovers the same vulnerability pattern repeatedly. Compilation errors that were previously diagnosed recur. No feedback loop exists.

### Success Criteria

1. Agents receive relevant prior learnings in their prompt at spawn time
2. Agent exit outcomes flow back as learning validation evidence
3. Cross-agent shared learnings are queryable from the orchestrator
4. All existing tests continue to pass; no performance regression at spawn

## Current State Analysis

### Existing Implementation (Implemented, Not Wired)

| Component | Crate | Location | Status |
|-----------|-------|----------|--------|
| Learning Capture CLI | terraphim_agent | `src/learnings/` | Working, no feature gate |
| SharedLearningStore (agent) | terraphim_agent | `src/shared_learning/store.rs` | Working behind `shared-learning` feature |
| BM25 dedup + trust promotion | terraphim_agent | `src/shared_learning/store.rs` | Working |
| Gitea Wiki Sync | terraphim_agent | `src/shared_learning/wiki_sync.rs` | Working |
| Suggestion Approval | terraphim_agent | `src/learnings/suggest.rs` | Working behind `shared-learning` |
| LearningInjector | terraphim_agent | `src/shared_learning/injector.rs` | Working behind `cross-agent-injection` |
| SharedLearningStore (orchestrator) | terraphim_orchestrator | `src/learning.rs` | Full impl with DeviceStorage, never called |
| LessonsEvolution | terraphim_agent_evolution | `src/lessons.rs` | Full impl with Persistable, never wired to orchestrator |
| AgentRunRecord + ExitClassifier | terraphim_orchestrator | `src/agent_run_record.rs` | Working, classifies every exit |
| LearningCoordinator | terraphim_github_runner | `src/learning/coordinator.rs` | Independent system, not wired to ADF |

### Two Learning Systems (Key Insight)

There are **two independent** learning systems in the codebase:

1. **terraphim_orchestrator::learning** (`src/learning.rs`) -- `SharedLearningStore` with `LearningPersistence` trait, `DeviceStorageLearningPersistence`, `NewLearning`/`Learning` domain types, trust levels L0-L3, context file generation, JSONL import. Designed for the orchestrator's pre-spawn context injection pattern. **Never instantiated in AgentOrchestrator.**

2. **terraphim_agent_evolution::LessonsEvolution** -- `Lesson` with categories (Technical/Process/Domain/Failure/SuccessPattern), `Evidence` with outcomes, `find_applicable_lessons()`, `validate_lesson()`, versioned persistence. Designed for per-agent across-runs compounding. **Never wired to orchestrator.**

3. **terraphim_agent::shared_learning** -- `SharedLearningStore` with BM25, markdown backend, suggestion approval, Gitea wiki sync, cross-agent injector. CLI-only today. **Not imported by orchestrator.**

### Spawn Path (injection point)

`spawn_agent()` at `lib.rs:1267`:
1. Resolve persona metaprompt (~line 1500)
2. Inject pre-flight findings (~line 1530)
3. Load skill chain content (~line 1540)
4. Build `SpawnRequest` with composed task (~line 1608)
5. Set up worktree, spawn process

**Injection point**: After step 3 (skill chain) and before step 4 (SpawnRequest), append a `## Prior Lessons` section.

### Exit Path (feedback point)

`poll_agent_exits()` at `lib.rs:4582`:
1. Wait for child process exit
2. Classify exit via `ExitClassifier` (~line 4692)
3. Build `AgentRunRecord` (~line 4729)
4. Feed provider health circuit breaker (~line 4760)
5. Handle error signatures (~line 4772)
6. Handle restart logic, worktree cleanup

**Feedback point**: After step 3, call `record_effective()` or `record_applied()` on the learning store.

## Constraints

### Technical Constraints
- **No new dependencies**: All crates already exist and are workspace members
- **Feature gates**: Must respect `shared-learning` and `cross-agent-injection` feature gates
- **No mocks in tests**: Use real stores against tempdir, as per AGENTS.md
- **No dead code**: Must not introduce `#![allow(dead_code)]`
- **No performance regression**: Learning lookup at spawn must be < 1ms (in-memory cache)
- **Token budget**: Lessons section must not exceed configurable max tokens (default 1500)

### Business Constraints
- Must close existing Gitea issues (#578, #668, #180) not create new ones
- Single PR preferred (~400 LOC scope)
- Must not break existing `learn` CLI commands

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Learning lookup latency | < 1ms | N/A (not called) |
| Token overhead | < 1500 tokens | N/A |
| Test coverage | All new code tested | N/A |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Spawn-time injection | Without this, agents have zero memory | Every ADF run starts from scratch |
| Exit-time feedback | Without this, learnings never improve | No validation evidence flows back |
| Use existing `learning.rs` | Avoids duplicating what's already built | `DeviceStorageLearningPersistence` already persists to `terraphim_persistence` |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| `terraphim_agent_evolution::LessonsEvolution` wiring | Requires `terraphim_agent_evolution` as dependency of orchestrator (not currently). The `learning.rs` module already provides the same capability with DeviceStorage. Wiring both would create two competing systems. Pick one. |
| GitHub Runner LearningCoordinator wiring | Separate crate, separate deployment. Not in ADF orchestrator scope. |
| Nightly failure clustering (#279) | Blocked on this work but is a follow-up, not a prerequisite |
| Flow DAG for learning extraction (#367) | Deferred phase, not needed for MVP wiring |
| `terraphim_agent::shared_learning` direct import from orchestrator | Orchestrator should not depend on agent crate. Use `learning.rs` instead. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_orchestrator::learning` | Already in same crate, just needs instantiation | Low |
| `terraphim_persistence::DeviceStorage` | Already used by `learning.rs` | Low |
| `ExitClassifier` / `ExitClass` | Already used in `poll_agent_exits()` | Low |
| `OrchestratorConfig` | Needs new fields for learning config | Low |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| None | N/A | N/A | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Learning section bloats prompt past token limit | Medium | Medium | Truncate to `max_lesson_tokens` config, drop lowest-trust first |
| DeviceStorage init fails at startup | Low | High | Fail-open: log warning, continue without learnings |
| `learning.rs` types vs `shared_learning` types divergence | Medium | Low | Use `learning.rs` types only; they're designed for orchestrator |
| Test hermeticity: DeviceStorage singleton | Medium | Medium | Use `InMemoryLearningPersistence` in tests |

### Open Questions

1. Should we use `learning.rs` (already in orchestrator crate) or add `terraphim_agent_evolution` dependency? -- **Recommend `learning.rs`** (simpler, no new crate dep, types already designed for orchestrator)
2. Should context file go to `/tmp/adf-context-{agent}.md` (as `write_context_file()` does) or inline into the prompt? -- **Recommend inline** (no filesystem dependency, already have skill chain injection pattern)
3. How many learnings to inject by default? -- **Recommend top 10, truncated to 1500 tokens**

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `learning.rs` is sufficient for orchestrator needs | It has insert, query_relevant, record_applied/effective, generate_context, archive_stale, JSONL import, trust promotion | May need to port features from `shared_learning` | Partially |
| `InMemoryLearningPersistence` is suitable for tests | It implements the full `LearningPersistence` trait | Test coverage gaps | Yes |
| Agents read their full stdin prompt | Current spawn path already sends persona + skills via stdin | Long prompts may be truncated by CLI tools | Yes (existing pattern) |
| No feature gate needed for orchestrator-side wiring | `learning.rs` is always compiled in `terraphim_orchestrator` | May need cfg gate for minimal builds | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Wire `terraphim_agent_evolution` into orchestrator (#578 approach) | Adds crate dep, uses `Lesson`/`Evidence` types, `find_applicable_lessons()` | Rejected -- adds dependency, two learning systems is confusing |
| Use `learning.rs` from orchestrator crate (#278 approach) | No new deps, `Learning`/`NewLearning`/`TrustLevel` types, `query_relevant()`, `generate_context()` | **Chosen** -- already in crate, designed for this purpose |
| Use `shared_learning::SharedLearningStore` from agent crate | Requires agent crate dep in orchestrator | Rejected -- circular dep risk, agent crate is heavy |

## Research Findings

### Key Insights

1. **`learning.rs` is a complete subsystem** with `InMemoryLearningPersistence` (tests) and `DeviceStorageLearningPersistence` (production), trust levels L0-L3, auto-promotion, context generation, JSONL import. It just needs one `SharedLearningStore` field on `AgentOrchestrator`.

2. **The injection pattern already exists** for skill chains (line 1540-1552): load content, format it, append to `composed_task`. Learning injection follows the exact same pattern.

3. **The feedback pattern already exists** for provider health (line 4760-4770): match on `ExitClass`, call appropriate method. Learning feedback follows the exact same pattern.

4. **Open PRs are unrelated**: PR #841 (session connectors), #835 (extract validation hang), #834 (export-kg). No conflicts.

5. **Active branches are stale**: Most remote branches are from previous sprints. No active work on learning-into-ADF.

6. **Related Gitea issues**: #578 (wire evolution, priority 38, unblocked), #668 (cross-run injection, blocked on #578), #278 (AgentRunRecord, unblocked), #180 (SharedLearning store, unblocked, already done), #179 (epic). **#578 is highest priority unblocked issue in this domain**.

7. **`terraphim_agent_evolution` is a heavier dependency** than needed. It brings in LLM adapter types, workflow patterns, memory system, task tracking. `learning.rs` is purpose-built for orchestrator learning with DeviceStorage persistence.

### Relevant Prior Art

- Skill chain injection pattern (`lib.rs:1143-1200`): loads from disk, formats, appends to prompt
- Pre-flight findings injection (`lib.rs:1530-1538`): same pattern, conditional injection
- Provider health feedback (`lib.rs:4760-4770`): match exit class, record outcome

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify `DeviceStorageLearningPersistence` loads without error | Ensure no runtime surprises | 30 min |

## Recommendations

### Proceed/No-Proceed

**Proceed**. All infrastructure exists. The work is pure wiring: add a field, call methods at two existing hook points.

### Scope Recommendations

1. **Use `learning.rs` from `terraphim_orchestrator`** (not `terraphim_agent_evolution`). It's already in the crate, has the right types, and doesn't add dependencies.
2. **Wire into `spawn_agent()` and `poll_agent_exits()`** at the existing injection/feedback points.
3. **Close #578** by wiring learning (which is what it asks for), noting we use `learning.rs` rather than `terraphim_agent_evolution`.
4. **Close #668** by adding `load_prior_context()` equivalent via `query_relevant()`.
5. **Update #180** -- Phase 1 (SharedLearning store) is already done in `learning.rs`. Mark as complete.

### Risk Mitigation Recommendations

- Start with `InMemoryLearningPersistence` default, add `DeviceStorage` opt-in via config
- Feature-gate behind existing pattern (no new feature flag needed since `learning.rs` is always compiled)
- Truncate learning section aggressively to avoid token budget issues

## Next Steps

If approved:
1. Load disciplined-design skill
2. Create design document specifying exact code changes
3. Create Gitea issue for the wiring work (or reuse #578)
4. Implement in single PR

## Appendix

### Key Code Locations

| File | Line(s) | Purpose |
|------|---------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | 1267-1620 | `spawn_agent()` -- prompt composition |
| `crates/terraphim_orchestrator/src/lib.rs` | 1540-1552 | Skill chain injection (pattern to follow) |
| `crates/terraphim_orchestrator/src/lib.rs` | 4582-4800 | `poll_agent_exits()` -- exit classification |
| `crates/terraphim_orchestrator/src/lib.rs` | 4729-4758 | AgentRunRecord construction |
| `crates/terraphim_orchestrator/src/learning.rs` | 1-500 | SharedLearningStore, LearningPersistence trait |
| `crates/terraphim_orchestrator/src/learning.rs` | 350-420 | `generate_context()` -- markdown context generation |
| `crates/terraphim_orchestrator/src/learning.rs` | 422-440 | `write_context_file()` -- /tmp file approach |
| `crates/terraphim_orchestrator/src/learning.rs` | 442-465 | `import_jsonl()` -- JSONL import |
| `crates/terraphim_orchestrator/src/config.rs` | N/A | Needs new learning config fields |
| `crates/terraphim_orchestrator/Cargo.toml` | N/A | Already has terraphim_persistence dep |

### Open PRs (No Conflicts)

| PR | Title | Status |
|----|-------|--------|
| #841 | OpenCode and Codex JSONL session connectors | Open, checks not started |
| #835 | Fix #776: extract functionality validation hangs | Open, lint failure |
| #834 | Learn export-kg and NormalizedTerm action metadata | Open, checks not started |

### Related Gitea Issues by Priority

| Issue | Title | Priority | Blocked? |
|-------|-------|----------|----------|
| #578 | Wire terraphim_agent_evolution into ADF orchestrator | 38 | No |
| #278 | Structured AgentRunRecord with ExitClass taxonomy | ~5 | No |
| #668 | Cross-run lesson injection via load_prior_context | ~5 | Yes (on #578) |
| #180 | SharedLearning store with trust-gated promotion | 36 | No (Phase 1 done) |
| #179 | Epic: ADF shared learnings | 5 | No |
| #242 | Phase 1: SQLite shared learning store | 37 | No (done in learning.rs) |
