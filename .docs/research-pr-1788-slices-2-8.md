# Research Document: PR #1788 Remaining Slices (2-8)

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-31
**Reviewers**: Human maintainer
**Source PR**: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1788
**Last reviewed commit**: `b0b3d93`

## Executive Summary

PR #1788 contained seven additional feature groups beyond the local skills integration (Slice 1, already merged). These fall into three natural themes: observability/security (output capture + timeout reporting), routing/dispatch (agent registry + webhook aliases), and operational tuning (worktree safety, probe timeout, documentation). This research document analyses each slice to determine whether it should proceed to implementation, be deferred, or be explicitly rejected.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes (Slices 2-3, 6), Partially (4-5, 7), No (8) | Output capture and worktree safety are high-leverage operational improvements. Registry and webhook changes are architectural enablers but add complexity. |
| Leverages strengths? | Yes | The codebase already has `OutputCapture`, `AgentOrchestrator`, and webhook infrastructure. |
| Meets real need? | Yes | Timeout diagnostics and worktree safety are real operational pain points observed in the PR branch. |

**Proceed**: Yes, but with strict slice-by-slice prioritisation. Slice 2 (output capture) is the enabler for Slice 3 (timeout reporting) and must be designed with redaction first. Slices 4-5 are architectural and should follow only after observability is stable. Slice 8 (TLA/docs) is lowest priority.

## Problem Statement

### Description

PR #1788 bundled multiple unrelated orchestrator improvements. After extracting Slice 1 (local skills), the remaining changes need individual evaluation before any are merged. Each slice has distinct risk profiles, dependencies, and acceptance criteria.

### Impact

Without focused review, these changes could introduce:
- **Data exposure** (Slice 3: timeout output posted to Gitea without redaction)
- **Performance regressions** (Slice 7: probe timeout reduction may mask real failures)
- **Architectural instability** (Slice 4: registry without clear migration path)
- **Broken dispatch semantics** (Slice 5: webhook aliases need collision handling)

### Success Criteria

- Each slice is evaluated independently and either merged, rejected, or deferred with explicit rationale.
- No slice is silently dropped.
- Slices with security implications (2, 3) are reviewed before any with routing implications (4, 5).
- All merged slices pass the same verification gates as Slice 1 (tests, clippy, fmt, UBS, coverage).

## Current State Analysis

### Slice 2: Output Capture Buffer

**Existing Implementation:**

`OutputCapture` in `crates/terraphim_spawner/src/output.rs` already captures stdout/stderr via `tokio::io::BufReader` and emits `OutputEvent` variants via broadcast and mpsc channels. The #1788 branch adds an `Arc<Mutex<Vec<OutputEvent>>>` buffer with a 4096-event cap and `Vec::remove(0)` eviction.

**Code Location:**

| Component | Location | Purpose |
|-----------|----------|---------|
| Output capture | `crates/terraphim_spawner/src/output.rs` | Reads child stdout/stderr and emits events |
| Captured buffer | #1788 branch only | Stores last 4096 events for timeout reporting |

**Data Flow:**

```text
Child stdout/stderr -> BufReader -> line parsing
    -> broadcast::Sender (live streaming)
    -> mpsc::Sender (event routing)
    -> Arc<Mutex<Vec<OutputEvent>>> (captured buffer, #1788 addition)
```

**Issues Found:**

1. `Vec::remove(0)` on every eviction is O(n) and creates unnecessary churn.
2. `Arc<Mutex<...>>` held across async boundaries in tokio::spawn tasks.
3. No redaction applied before events are stored.
4. Buffer is unbounded in memory per event (only count is capped).

### Slice 3: Timeout Output Posting

**Existing Implementation:**

The #1788 branch modifies `AgentOrchestrator::poll_wall_timeouts()` to collect captured output lines, append a timeout summary, and post them to the agent's associated Gitea issue via `output_poster::post_agent_output_for_project()`.

**Code Location:**

| Component | Location | Purpose |
|-----------|----------|---------|
| Timeout polling | `crates/terraphim_orchestrator/src/lib.rs` | Checks wall-clock timeouts and respawns |
| Output poster | `crates/terraphim_orchestrator/src/output_poster.rs` | Posts agent output to Gitea issues |

**Data Flow:**

```text
agent stdout/stderr -> OutputCapture -> captured_events
    -> wall-clock timeout detected
    -> filter_map to lines + timeout_summary()
    -> post_agent_output_for_project(project, agent, issue, lines, None)
    -> Gitea issue comment
```

**Issues Found:**

1. Raw agent output can contain secrets, tokens, or operational context.
2. No redaction policy is applied before posting.
3. `post_agent_output_for_project` takes `&[String]` with no transformation hook.
4. Timeout summary uses `max_cpu_seconds` as wall-clock limit, which is misleading terminology.

### Slice 4: Agent Registry

**Existing Implementation:**

The #1788 branch adds `crates/terraphim_orchestrator/src/agent_registry.rs` with `AgentKey`, `AgentScope`, `RegisteredAgent`, and `AgentSource`. The registry is described as read-only over `OrchestratorConfig` but the PR does not replace existing config lookups with registry lookups.

**Code Location:**

| Component | Location | Purpose |
|-----------|----------|---------|
| Agent registry | `crates/terraphim_orchestrator/src/agent_registry.rs` (new) | Project-scoped agent index |

**Issues Found:**

1. Registry is added but not wired into existing lookup paths.
2. No migration path from config-based to registry-based lookups.
3. `AgentSource::ConfigMerged` is the only variant; extensibility is speculative.
4. Increases code surface without immediate functional benefit.

### Slice 5: Webhook Group Alias Dispatch

**Existing Implementation:**

The #1788 branch adds `group_alias_members()` to `webhook.rs` which expands `@adf:<prefix>` mentions into dispatches for all agents matching `<prefix>-*`. For example, `@adf:implementation-swarm` dispatches to `implementation-swarm-A` and `implementation-swarm-B`.

**Code Location:**

| Component | Location | Purpose |
|-----------|----------|---------|
| Webhook handler | `crates/terraphim_orchestrator/src/webhook.rs` | Parses Gitea webhooks and dispatches agents |

**Issues Found:**

1. Prefix matching with `-` delimiter is fragile (e.g. `implementation-swarmish` would not match, which is correct, but the logic depends on exact prefix + dash).
2. No collision detection if an actual agent name matches the alias.
3. Group alias dispatches use full comment body as context, unlike individual mentions.
4. No rate limiting or cardinality cap on group expansion.

### Slice 6: Worktree Fail-Closed

**Existing Implementation:**

The #1788 branch modifies worktree creation to fail closed (return error instead of proceeding without a worktree) when ADF worktree creation fails.

**Code Location:**

| Component | Location | Purpose |
|-----------|----------|---------|
| Worktree guard | `crates/terraphim_orchestrator/src/worktree_guard.rs` | Manages per-agent Git worktrees |

**Data Flow:**

```text
Agent spawn requested
    -> create_agent_worktree()
    -> if fail: return Err (fail-closed) instead of proceeding without isolation
```

**Issues Found:**

1. No diff was found in the worktree_guard.rs file when comparing #1788 to main. The change may be in a different file or the branch structure may differ.
2. Need to verify exact location and nature of the fail-closed change.

### Slice 7: Provider Probe Timeout

**Existing Implementation:**

The #1788 branch reduces probe timeout from 120s to 15s and removes content-based success classification (`has_token_bearing_output`). It also removes CLI-specific circuit breaker keys, collapsing them to provider:model only.

**Code Location:**

| Component | Location | Purpose |
|-----------|----------|---------|
| Provider probe | `crates/terraphim_orchestrator/src/provider_probe.rs` | Health-checks LLM providers |

**Issues Found:**

1. 15s may be insufficient for slow providers or cold starts.
2. Removing content-based classification means `exit 0` alone is considered healthy, which could misclassify token-less responses as healthy.
3. Removing CLI-specific breakers means a broken CLI integration poisons the provider for all CLIs.
4. The content-based classification was added for a specific regression (opencode + zai-coding-plan); removing it may reintroduce that issue.

### Slice 8: TLA and Generated Docs

**Existing Implementation:**

The #1788 branch adds TLA+ specification files and generated `.terraphim/learnings/*.md` files.

**Issues Found:**

1. Generated learnings must not be committed (already established in Slice 1).
2. TLA specs are valuable but unrelated to local skills and should be reviewed separately.
3. `.docs/` additions from #1788 are mixed with operational docs and may conflict with existing documentation structure.

## Constraints

### Technical Constraints

- Rust workspace with `cargo fmt`, `cargo clippy`, and `cargo test` gates.
- `terraphim_symphony` is excluded from workspace.
- Output capture must not block the async runtime.
- Gitea API interactions must not leak secrets in issue comments.
- Circuit breaker state is in-memory only (no persistence).

### Business Constraints

- Each slice must be independently reviewable and revertable.
- No slice may introduce breaking changes to existing `adf --local` behaviour.
- Security-sensitive slices (2, 3) require explicit security review.

### Non-Functional Requirements

| Requirement | Target | Slice Relevance |
|-------------|--------|-----------------|
| Memory per agent | < 10MB | Slice 2: captured buffer |
| Probe latency | < 30s p99 | Slice 7: timeout reduction |
| Webhook dispatch latency | < 1s | Slice 5: alias expansion |
| Issue comment size | < 64KB | Slice 3: timeout output posting |

## Vital Few (Essentialism)

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Output redaction before remote posting | Prevents credential leakage in Gitea issues | Slice 3 currently posts raw output |
| Probe timeout must not mask real failures | Provider health affects all agent routing | Slice 7 removes content classification |
| Group alias expansion must be bounded | Prevents mention-spam DoS | Slice 5 has no cardinality cap |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Committing generated `.terraphim/learnings/` | Established as non-source in Slice 1 |
| Agent registry without wiring | Adds surface area with no immediate functional benefit; defer until lookup migration is designed |
| TLA specs in operational PR | Formal specs should be reviewed by someone with TLA expertise separately |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `OutputCapture` | Slice 2 extends this | Medium: async locking, memory |
| `AgentOrchestrator` | Slices 3, 4, 6, 7 modify this | High: core runtime |
| `WebhookState` | Slice 5 extends this | Medium: dispatch semantics |
| `ProviderHealthMap` | Slice 7 modifies this | Medium: routing correctness |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea API | REST v1 | Medium: rate limits | N/A |
| Tokio | 1.x | Low: well-tested | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Slice 3 leaks secrets in issue comments | High | High | Block Slice 3 until Slice 2 has redaction |
| Slice 7 misclassifies unhealthy providers | Medium | High | Keep content classification; only reduce timeout |
| Slice 5 group alias expands to 100+ agents | Low | Medium | Add max_members cap |
| Slice 4 registry is added but never used | High | Low | Defer or reject |

### Open Questions

1. What is the exact redaction policy for agent output? (API keys, tokens, passwords, URLs with credentials?)
2. Is 15s sufficient for all providers in cold-start scenarios?
3. Should group aliases be defined in `adf.toml` or inferred from agent naming conventions?
4. Where exactly is the worktree fail-closed change in #1788?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Output capture buffer should be bounded | #1788 implements 4096-event cap | Unbounded memory growth | Yes |
| Redaction should happen before storage, not before posting | Defense in depth | If posting path bypasses redaction, leak still occurs | No -- needs design decision |
| Provider probe timeout can be reduced | #1788 changes 120s -> 15s | False positives on slow providers | No -- needs measurement |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Merge all remaining slices as one PR | Fast but repeats #1788's mistake | Rejected: same bundling problem |
| Merge slices 2-3 together, defer 4-8 | Output capture and timeout reporting are coupled | Accepted as the next combined unit |
| Reject everything except Slice 6 (worktree) | Safest but loses valuable observability | Rejected: timeout diagnostics are needed |
| Merge Slice 7 (probe timeout) independently | Low risk if content classification kept | Accepted after 2-3 |

## Research Findings

### Key Insights

1. **Slices 2 and 3 are coupled**: Slice 3 (timeout posting) depends on Slice 2 (output capture). Slice 2 must be designed with redaction from the start, or Slice 3 becomes a security risk.
2. **Slice 7 has a specific regression history**: The content-based classification (`has_token_bearing_output`) was added because `exit 0` alone misclassified opencode + zai-coding-plan. Removing it may reintroduce that bug.
3. **Slice 4 (registry) is premature**: The registry module is well-structured but not wired into any caller. It should either be fully integrated or deferred.
4. **Slice 5 (webhook aliases) needs policy**: Prefix-based alias expansion is useful but needs cardinality limits and explicit alias definitions.
5. **Slice 6 needs location verification**: The worktree fail-closed change was not found in the expected file during diff inspection.

### Relevant Prior Art

- Slice 1 already established the pattern of: research -> design -> verification -> validation -> merge.
- `AgentConfig::infer_api_keys()` already documents which CLIs use OAuth vs API keys, relevant for output redaction policy.
- `output_poster.rs` already has `post_agent_output_for_project()` which takes raw string slices.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Redaction policy design | Define what patterns to redact and where (storage vs posting) | 1 hour |
| Probe timeout measurement | Confirm 15s is sufficient for all providers in practice | 30 minutes |
| Worktree fail-closed location | Find exact file/line of the change | 15 minutes |

## Recommendations

### Proceed/No-Proceed

**Proceed with slices 2-3 as a combined unit**, then evaluate 5-7 individually. **Defer or reject slice 4 and 8** unless explicit justification emerges.

### Scope Recommendations

**Next (slices 2-3 combined):**

- Redesign output capture with `VecDeque` instead of `Vec::remove(0)`.
- Apply redaction before storage (defense in depth).
- Post only redacted timeout summaries to Gitea.
- Add tests for redaction, buffer bounds, and timeout posting.

**After 2-3:**

- Slice 5: Add cardinality cap and explicit alias config.
- Slice 7: Reduce timeout but keep content classification.
- Slice 6: Verify change location and merge if safe.

**Defer indefinitely:**

- Slice 4: Merge only when registry is wired into lookups.
- Slice 8: Review TLA separately; never commit generated learnings.

### Risk Mitigation Recommendations

- Run `security-audit` skill on Slice 2-3 before implementation.
- Measure probe latency with current 120s timeout to establish baseline before reducing.
- Add `max_group_alias_members` config field before enabling Slice 5.

## Next Steps

If approved:

1. Create combined research/design for Slices 2-3 (output capture + timeout reporting).
2. Implement with redaction-first design.
3. Verify with UBS, tests, coverage.
4. Merge and evaluate Slices 5-7 individually.
5. Create explicit rejection or deferral comments for Slices 4 and 8.
