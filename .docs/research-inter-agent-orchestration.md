# Research Document: Inter-Agent Orchestration via Gitea Mentions

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-04-22
**Gitea Issue**: #144
**Reviewers**: Alex

## Executive Summary

Issue #144 describes an epic to evolve the ADF from a cron-driven scheduler into a coordinated agent fleet where agents communicate via Gitea issue comments using `@adf:agent-name` mentions. Research reveals that **most of the infrastructure described in the issue already exists** in the codebase. The core gap is not the mention/detection layer (which is fully implemented) but rather the **structured coordination patterns** that turn mentions into reliable multi-agent workflows: context propagation between agents, result chaining, depth limiting, and human-readable RPC semantics.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Core value proposition of ADF -- agents that collaborate autonomously |
| Leverages strengths? | Yes | KG-first routing, Aho-Corasick matching, existing Gitea API integration |
| Meets real need? | Yes | Reviewer chain (6-agent swarm) already works via mentions; generalising to any agent pair is the next step |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

The ADF orchestrator can already detect `@adf:agent-name` mentions in Gitea comments and spawn agents. However, the system lacks structured coordination patterns: there is no standardised way for one agent to request another agent's output, for agents to pass structured context between each other, or for the orchestrator to enforce depth/cycle limits on mention chains.

### Impact

Without structured coordination:
- Agents cannot reliably delegate subtasks (the reviewer chain is a hardcoded special case)
- No standardised context format between agents
- Risk of infinite mention loops (A mentions B, B mentions A)
- No audit trail of inter-agent decisions in a machine-readable format
- Compound review is a separate code path rather than a specialisation of general mention-driven coordination

### Success Criteria

1. Any agent can mention any other agent via `@adf:agent-name` in its output
2. Mentioned agent receives structured context (task, parent agent, issue, prior decisions)
3. Depth limit enforced (max 3 levels of mention nesting)
4. Loop-risk controls prevent unbounded mention recursion
5. Results are posted back to the same issue as Gitea comments
6. The existing reviewer chain continues to work unchanged
7. Compound review remains unaffected by mention-chain changes

## Current State Analysis

### Existing Implementation

The orchestrator already has a fully-functioning mention detection and dispatch system:

**Mention Polling** (`mention.rs`, 820 lines):
- `MentionCursor` with SQLite persistence -- cursor-based repo-wide polling via `fetch_repo_comments(since, limit)`
- `parse_mention_tokens()` -- regex `@adf:(?:(?P<project>[a-z][a-z0-9-]{1,39})/)?(?P<agent>[a-z][a-z0-9-]{1,39})\b`
- `resolve_mention()` -- multi-project resolution (qualified `@adf:proj/name` and unqualified)
- `AdfCommandParser` -- Aho-Corasick matching for `CompoundReview`, `SpawnAgent`, `SpawnPersona`
- `DetectedMention` -- issue_number, comment_id, raw_mention, agent_name, resolution, comment_body, mentioner, timestamp, project_id
- Rate limiting: `max_dispatches_per_tick` (default 3), `max_concurrent_mention_agents` (default 5)

**Agent Dispatch** (`lib.rs`, `spawn_agent()` at line 1264):
- Full spawn pipeline: project pause gate, disk space guard, budget gate, pre-check gate, model routing (KG/keyword/static), persona composition, skill chain injection, worktree creation, provider construction, spawn with fallback
- `SpawnContext` with env vars: `ADF_PROJECT_ID`, `GITEA_TOKEN`, `ADF_AGENT_NAME`, etc.
- `ManagedAgent` tracks: definition, handle, started_at, restart_count, output_rx, `spawned_by_mention`, worktree_path, routed_model, session_id

**Output Posting** (`output_poster.rs`, 524 lines):
- Per-project `GiteaTracker` with per-agent tokens (13 agent identities)
- `post_agent_output_for_project()` -- posts collapsible markdown (truncated 60KB)
- `post_raw_for_project()` / `post_raw_as_agent_for_project()` -- raw comment posting

**Dispatcher** (`dispatcher.rs`, 512 lines):
- Priority queue: Safety=0, Mention=200, ReviewPr=400, AutoMerge=500, PostMergeGate=600, Core=1000, Growth=2000
- `DispatchTask::MentionDriven { agent_name, issue_number, comment_id, context, project }` already exists
- Round-robin fairness across projects

**Reconciliation Loop** (`lib.rs`, `run()` at line 781):
- Main `tokio::select!` loop handles: scheduler events, drift alerts, tick-based reconciliation
- Tick drains dispatcher queue, polls mentions (poll_modulo gated), polls PR reviews, handles agent exits
- Agent output captured via `output_rx` and posted via OutputPoster

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Mention parsing | `crates/terraphim_orchestrator/src/mention.rs` | Regex + Aho-Corasick mention detection |
| Mention polling | `crates/terraphim_orchestrator/src/lib.rs:2382-2800` | Per-project cursor-based polling |
| Agent dispatch | `crates/terraphim_orchestrator/src/lib.rs:1264-1700` | Full spawn pipeline |
| Output posting | `crates/terraphim_orchestrator/src/output_poster.rs` | Gitea comment posting |
| Dispatcher queue | `crates/terraphim_orchestrator/src/dispatcher.rs` | Priority queue with fairness |
| Gitea API client | `crates/terraphim_tracker/src/gitea.rs` | REST client (issues, comments, PRs, claims) |
| ADF commands | `crates/terraphim_orchestrator/src/adf_commands.rs` | `AdfCommand` enum, `AdfCommandParser` |
| Compound review | `crates/terraphim_orchestrator/src/compound.rs` | 6-agent swarm workflow |
| Handoff context | `crates/terraphim_orchestrator/src/handoff.rs` | `HandoffContext` for inter-agent state |
| Agent run records | `crates/terraphim_orchestrator/src/agent_run_record.rs` | `ExitClass`, run metadata |
| Configuration | `crates/terraphim_orchestrator/src/config.rs` | `OrchestratorConfig`, `AgentDefinition`, `MentionConfig` |
| Nightwatch | `crates/terraphim_orchestrator/src/nightwatch.rs` | Drift detection, correction levels |
| Concurrency | `crates/terraphim_orchestrator/src/concurrency.rs` | Global/project concurrency limits |

### Data Flow (Current)

```
[Gitea comment with @adf:agent-name]
  -> poll_mentions_for_project()
    -> fetch_repo_comments(since=cursor)
    -> parse_mention_tokens(comment.body)
    -> resolve_mention() or AdfCommandParser
    -> Dispatcher::enqueue(MentionDriven { agent_name, issue_number, context, project })
  -> reconcile_tick()
    -> dispatcher.dequeue()
    -> spawn_agent(definition with gitea_issue + mention context appended to task)
    -> agent runs CLI process
    -> output captured via output_rx
    -> OutputPoster::post_agent_output_for_project()
    -> Gitea comment posted under agent's identity
```

### Integration Points

- **Gitea REST API**: `GET /repos/{owner}/{repo}/issues/comments?since=&limit=50` (polling), `POST /repos/{owner}/{repo}/issues/{N}/comments` (posting)
- **gitea-robot CLI**: External binary for claim operations (`/home/alex/go/bin/gitea-robot`)
- **terraphim_persistence**: SQLite for cursor persistence, handoff ledger
- **terraphim_automata**: Aho-Corasick matching for command parsing
- **terraphim_tracker**: Full Gitea REST client with pagination, claim strategies

## Constraints

### Technical Constraints

| Constraint | Source | Impact |
|------------|--------|--------|
| Agents are CLI processes (not gRPC servers) | Architecture | Communication must go through Gitea API, no direct agent-to-agent channels |
| Gitea comment body limit ~1MB | Gitea 1.26.0 | Agent output must be truncated (existing: 60KB) |
| 30-60s polling latency | Current architecture | Mention-driven workflows have inherent latency |
| Single orchestrator process | Architecture | No distributed coordination needed |
| Per-agent Gitea tokens (13 agents) | agent_tokens.json | Each agent posts under its own identity |
| SQLite for persistence | terraphim_persistence | Concurrent writes must be coordinated |

### Business Constraints

| Constraint | Source |
|------------|--------|
| Must not break existing reviewer chain | Production system |
| Must not break compound review workflow | Production system |
| Must not break webhook-driven dispatch | Production system |
| Must work with all CLI tools (opencode, claude, codex) | Multi-provider |

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Mention detection latency | < 60s | ~60s (poll_modulo=2 ticks) |
| Concurrent mention agents | 5 max | 5 max (configurable) |
| Dispatches per tick | 3 max | 3 max (configurable) |
| Agent output posting | < 5s | ~1-2s |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Agents communicate only via Gitea comments | Enables human-readable audit trail, async coordination, no new protocols | Architecture decision in #144 body |
| Orchestrator mediates all Gitea writes | Agents never touch Gitea API directly -- security and consistency | Architecture decision in #144 body |
| Max mention depth of 3 | Prevents infinite loops, controls blast radius | Architecture decision in #144 body |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Webhook-driven mentions (#149) | Deferred -- requires HTTP server changes, polling is sufficient |
| Agent-to-agent gRPC | Agents are CLI processes, not servers |
| Custom RPC protocol over Gitea | Standard markdown comments are sufficient |
| Real-time agent coordination | 60s polling is acceptable for non-safety-critical workflows |
| Database-backed coordination state | SQLite KV store is sufficient for cursor + depth tracking |
| Agent sandboxing improvements | Orthogonal to mention coordination |
| Deep context handoff (full session state) | Shallow handoff (task + decisions + files) is sufficient |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_tracker` | Provides Gitea API client (fetch_repo_comments, post_comment) | Low -- stable, well-tested |
| `terraphim_automata` | Aho-Corasick matching for mention parsing | Low -- already integrated |
| `terraphim_persistence` | SQLite KV store for cursor persistence | Low -- already integrated |
| `terraphim_spawner` | CLI process spawning | Low -- already integrated |
| `terraphim_router` | Model/provider routing | Low -- already integrated |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea REST API | 1.26.0 | Low | N/A (core infrastructure) |
| gitea-robot CLI | current | Low | Direct REST API fallback exists |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Mention replay storm on orchestrator restart | Low (fixed in #186) | High | Cursor-based polling with startup guard |
| Infinite mention loops | Medium | High | Depth counter per dispatch chain + self-mention rejection |
| Agent output too large for Gitea comment | Medium | Low | Existing truncation at 60KB |
| Race condition on concurrent mentions | Low | Medium | Dispatcher serialisation + per-issue dispatch counter |
| Gitea API rate limiting | Low | Medium | Existing rate limiter (max_dispatches_per_tick) |
| Agent mentions itself accidentally | Low | Low | Self-mention filter in dispatch |

### Open Questions

1. **How should agents emit mentions?** -- Currently agents would need to include `@adf:agent-name` in their CLI output. The OutputPoster posts the full output as a comment, so mentions in agent stdout would be parsed on the next poll cycle. This is the natural path but needs confirmation that agent prompts instruct them to use this syntax.
2. **Should mention context be structured JSON or natural language?** -- The `MentionDriven.context` field is currently a String. Structured JSON would be machine-parseable but harder for human readers. Natural language is human-readable but harder to parse programmatically.
3. **Depth tracking scope** -- Per-issue or per-chain? Per-chain requires correlation IDs across comments. Per-issue is simpler but may incorrectly block valid parallel chains on the same issue.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Agent output containing `@adf:` is posted as Gitea comment | OutputPoster posts all agent stdout | Agents could not trigger mentions | Yes (verified in code) |
| Depth counter can be tracked via dispatch metadata | Dispatcher tracks per-task metadata once schema is extended | Chain limits not enforced | No (requires `DispatchTask::MentionDriven` field additions) |
| 60s polling latency is acceptable | Stated in #144 body | Mention-driven workflows feel sluggish | Yes (stakeholder confirmed) |
| Existing mention detection handles multi-project | `resolve_mention()` supports `@adf:proj/name` | Multi-project mentions break | Yes (verified in code) |
| Compound review is a special case of mention-driven workflow | Issue body says mentions generalise it | Compound review would need separate code path | No -- needs validation |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Agents emit mentions in their stdout output | Natural flow, no new code paths needed, delayed by one poll cycle | Chosen -- simplest, reuses existing pipeline |
| Orchestrator injects mention rules into agent prompts | Requires prompt engineering, more control over mention format | Rejected -- agents should be autonomous |
| Structured mention blocks in agent output (JSON/RPC) | Machine-parseable, but requires agent cooperation and parsing | Deferred -- can layer on top later |
| Mention context embedded in comment metadata | Gitea does not support comment metadata | Rejected -- platform limitation |

## Research Findings

### Key Insights

1. **The infrastructure is 80% built.** Mention detection, parsing, resolution, dispatch, and output posting all exist and are production-tested. The gap is in structured coordination patterns (depth tracking, context propagation, loop-risk controls).

2. **The reviewer chain is already mention-driven.** When `quality-coordinator` runs, it posts comments with `@adf:test-guardian`, `@adf:security-sentinel`, etc. These are detected on the next poll cycle and spawn the respective agents. This validates the pattern works in production.

3. **Compound review is NOT mention-driven** -- it uses its own `CompoundReviewWorkflow` with `run_single_agent()`. Generalising it to mentions would require significant refactoring and is not essential for the epic's value.

4. **Depth tracking requires a new primitive.** The current `DetectedMention` has no parent chain concept. Adding a `mention_chain_id` and `depth` field to `DispatchTask::MentionDriven` would enable depth limiting.

5. **Agent prompts need mention instructions.** For agents to reliably emit `@adf:agent-name` mentions, their task prompts need to include the mention syntax and when to use it. The `MetapromptRenderer` already supports template variables.

6. **The `AdfCommandParser` is the extension point.** It uses Aho-Corasick matching on agent names and persona names. New coordination commands (e.g., `@adf:delegate:agent-name`) could be added here.

### Relevant Prior Art

- **Kubernetes Event-Driven Autoscaling (KEDA)**: Polling external event sources with configurable intervals -- similar to mention polling
- **GitHub Actions workflow_dispatch**: Triggering workflows via API calls with structured inputs -- analogous to mention-driven dispatch
- **Mattermost/Slack bot mentions**: `@bot-name command args` pattern -- directly analogous to `@adf:agent-name`

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Depth tracking design | Determine how to track mention chain depth across poll cycles | 2 hours |
| Agent prompt template | Design mention instruction template for agent metaprompts | 2 hours |
| Loop control policy | Evaluate whether ancestry tracking is needed beyond depth limiting | 2 hours |

## Recommendations

### Proceed/No-Proceed

**Proceed.** The infrastructure is mature and the remaining work is well-scoped: depth tracking, loop-risk controls, and agent prompt templates. No new external dependencies needed.

### Scope Recommendations

**Phase 1 (Essential):**
1. Add `mention_chain_id` and `depth` to `DispatchTask::MentionDriven`
2. Enforce max depth of 3 in `spawn_agent()` for mention-driven tasks
3. Add loop-risk controls (self-mention rejection and max depth enforcement)
4. Include mention context in spawned agent's task (parent agent, issue, prior decisions)

**Phase 2 (Valuable):**
5. Standardise mention instruction in agent metaprompts
6. Add `@adf:delegate:agent-name` structured mention syntax
7. Mention chain audit trail in agent run records

**Deferred:**
8. Compound review generalisation (keep as-is for now)
9. Webhook-driven mentions (#149)
10. Structured JSON/RPC mention blocks

### Risk Mitigation Recommendations

1. Add depth counter to `DispatchTask::MentionDriven` before any other changes
2. Add integration test that verifies depth limit is enforced
3. Add self-mention filter (agent cannot mention itself)
4. Add explicit test documenting A->B->A handling semantics under current stateless check
5. Deploy with depth=2 initially, increase to 3 after validation

## Next Steps

If approved:
1. Create child issues for each Phase 1 item
2. Create design document (Phase 2)
3. Implement depth tracking and loop-risk controls
4. Update agent metaprompts with mention instructions
5. Integration testing on bigbox

## Appendix

### Reference Materials

- `.docs/adf-architecture.md` -- Full ADF architecture with ASCII and Mermaid diagrams
- `.docs/design-dark-factory-orchestration.md` -- Original orchestrator implementation plan
- `.docs/design-cursor-mention-polling.md` -- Cursor-based mention polling design
- `.docs/research-mention-replay-storm.md` -- Root cause analysis of Apr 3 2026 incident

### Current Mention Flow (Sequence)

```
poll_mentions_for_project():
  cursor = MentionCursor::load_or_now(project_id)
  comments = tracker.fetch_repo_comments(cursor.last_seen_at, 50)
  for comment in comments:
    if comment.id in cursor.processed_comment_ids: skip
    mentions = parse_mentions(comment, issue_number, agents, personas, project)
    for mention in mentions:
      if dispatches_this_tick >= max_dispatches_per_tick: break
      if should_skip_dispatch(...): continue
      definition = resolve_mention(...)
      definition.task += mention_context_from_comment_body
      definition.gitea_issue = Some(issue_number)
      spawn_agent(definition)  // spawned_by_mention = true
      dispatches_this_tick += 1
    cursor.advance_to(comment.created_at)
    cursor.processed_comment_ids.insert(comment.id)
  cursor.save()
```

### DispatchTask::MentionDriven (Current)

```rust
MentionDriven {
    agent_name: String,
    issue_number: u64,
    comment_id: u64,
    context: String,     // Free-form text from comment body
    project: String,     // Project ID for multi-project
}
```

### Key Config: MentionConfig

```rust
pub struct MentionConfig {
    pub poll_modulo: u64,                    // Default: 2 (every 2 ticks)
    pub max_dispatches_per_tick: u32,         // Default: 3
    pub max_concurrent_mention_agents: u32,   // Default: 5
}
```
