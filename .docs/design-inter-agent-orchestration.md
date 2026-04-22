# Implementation Plan: Inter-Agent Orchestration via Gitea Mentions

**Status**: Draft
**Research Doc**: `.docs/research-inter-agent-orchestration.md`
**Author**: Terraphim AI
**Date**: 2026-04-22
**Gitea Issue**: #144
**Estimated Effort**: 2-3 days

## Overview

### Summary

Add structured coordination primitives to the existing mention-driven dispatch system: chain depth tracking, loop-risk controls, structured context propagation, and standardised mention emission. The infrastructure (mention detection, parsing, resolution, dispatch, output posting) already exists. This plan adds the missing coordination layer on top.

### Approach

Layer new coordination primitives onto the existing `DispatchTask::MentionDriven` path. Extend `MentionDriven` with chain metadata, add depth/cycle checks in the dispatch pipeline, and standardise mention context format for spawned agents.

### Scope

**In Scope (Top 5):**
1. Mention chain depth tracking and enforcement (max 3)
2. Loop-risk control via self-mention rejection and bounded depth
3. Structured mention context propagation between agents
4. Agent metaprompt mention instructions
5. Mention chain audit trail in agent run records

**Out of Scope:**
- Compound review generalisation (keep as-is)
- Webhook-driven mentions (#149, deferred)
- Structured JSON/RPC mention blocks
- New CLI commands for mention chain inspection
- Multi-project coordination beyond existing support

**Avoid At All Cost** (5/25 rule):
- Custom agent-to-agent communication protocol
- Database-backed coordination state (SQLite KV is enough)
- Real-time coordination (polling is fine)
- Breaking the reviewer chain
- Breaking compound review
- Changing the `@adf:` mention syntax
- Adding new external dependencies
- Agent sandboxing changes
- Custom serialisation format for context
- Distributed consensus for chain tracking

## Architecture

### Component Diagram

```
                    EXISTING (no changes)
+------------------+  +-------------------+  +------------------+
| Mention Polling  |  | Gitea API Client  |  | Output Poster    |
| (mention.rs)     |  | (terraphim_tracker|  | (output_poster)  |
|                  |  |  /gitea.rs)       |  |                  |
+--------+---------+  +---------+---------+  +--------+---------+
         |                      |                      |
         v                      v                      v
+------------------------------------------------------------------+
|                    NEW: Mention Chain Layer                        |
|                                                                   |
|  +-------------------------+  +-------------------------------+  |
|  | MentionChainTracker     |  | MentionContextBuilder         |  |
|  | - reject self-mention   |  | - parent_agent               |  |
|  | - enforce_depth_limit   |  | - issue_number               |  |
|  | - validate chain fields |  | - prior_decisions            |  |
|  | - stateless checks      |  | - files_touched              |  |
|  +-------------------------+  | - correlation_id             |  |
|               |               +-------------------------------+  |
|               |                              |                   |
+---------------|------------------------------|-------------------+
                |                              |
                v                              v
+------------------------------------------------------------------+
|                    EXISTING (extended)                             |
|  DispatchTask::MentionDriven  (add chain_id, depth, parent)      |
|  spawn_agent()                (add depth/self-mention gate)      |
|  MetapromptRenderer           (add mention instructions)         |
|  AgentRunRecord               (add chain metadata)               |
+------------------------------------------------------------------+
```

### Data Flow (Enhanced)

```
[Gitea comment with @adf:agent-name]
  -> poll_mentions_for_project()
    -> fetch_repo_comments(since=cursor)
    -> parse_mention_tokens(comment.body)
    -> resolve_mention()
    -> NEW: MentionChainTracker::check(depth, parent, agent_name)
      -> If depth >= MAX_MENTION_DEPTH: skip + post warning
-> If self-mention detected: skip + post warning
      -> If self-mention: skip
    -> NEW: MentionContextBuilder::build(parent, issue, comment, depth)
      -> Returns structured context string for agent task
    -> Dispatcher::enqueue(MentionDriven { ..., chain_id, depth, parent_agent })
  -> reconcile_tick()
    -> dispatcher.dequeue()
    -> spawn_agent(definition)
      -> NEW: depth gate check (redundant safety)
      -> Append structured mention context to task
      -> Append mention instructions to metaprompt
    -> agent runs, output captured
    -> OutputPoster posts as agent identity
    -> NEW: if agent output contains @adf:, chain continues next poll
    -> NEW: record chain metadata in AgentRunRecord
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Chain ID = ULID of initial mention dispatch | Globally unique, time-sortable, no coordination needed | UUID (no ordering), sequential counter (requires coordination) |
| Depth tracked in DispatchTask fields | Dispatcher already has all needed metadata | Separate HashMap (extra state management), per-issue depth (too coarse) |
| Loop control via self-mention + depth limit | Stateless and low overhead; avoids unbounded recursion without ancestry tracking | Full chain history (more complexity for current scope) |
| Context as structured markdown block | Human-readable in Gitea, machine-parseable with regex | JSON (hard to read in Gitea UI), pure text (hard to parse) |
| Mention instructions in metaprompt | Agents need explicit instructions to emit mentions | Hardcoded output parsing (fragile), post-processing (missed mentions) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full chain history tracking | Max depth 3 makes this unnecessary complexity | Memory growth, state management burden |
| Structured JSON context blocks | Breaks human readability in Gitea comments | Reduces auditability |
| Per-chain SQLite table | In-memory tracking is sufficient (chains live < 5 min) | Adds I/O, schema migration |
| Agent-to-agent context handoff files | Gitea comments already serve this purpose | File management complexity |
| Custom mention syntax (`@adf:delegate:...`) | Current `@adf:agent-name` is sufficient | Breaking change, additional parsing |

### Simplicity Check

**What if this could be easy?**

The simplest possible design: add 3 fields to `MentionDriven` (chain_id, depth, parent_agent), add a 10-line depth/cycle check before enqueue, add a 5-line context builder, and add a sentence to agent metaprompts. That's it. No new files, no new structs beyond what's needed for tracking.

**Senior Engineer Test**: "You're adding depth tracking and loop-risk controls to an existing mention dispatcher. That's a gate check and a few fields. This is the right scope."

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose | Est. Lines |
|------|---------|------------|
| `crates/terraphim_orchestrator/src/mention_chain.rs` | `MentionChainTracker`, depth/cycle checks, context builder | 200 |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Add `mod mention_chain;`, import `MentionChainTracker`, add depth/cycle gate in `poll_mentions_for_project()`, add context builder call, extend `spawn_agent()` with mention instructions |
| `crates/terraphim_orchestrator/src/dispatcher.rs` | Add `chain_id`, `depth`, `parent_agent` fields to `DispatchTask::MentionDriven` |
| `crates/terraphim_orchestrator/src/config.rs` | Add `max_mention_depth` (default: 3) to `MentionConfig` |
| `crates/terraphim_orchestrator/src/agent_run_record.rs` | Add `mention_chain_id`, `mention_depth` fields to `AgentRunRecord` |
| `crates/terraphim_orchestrator/src/persona.rs` | Add mention instruction snippet to `MetapromptRenderer` |

### Deleted Files

None.

## API Design

### Public Types

```rust
// mention_chain.rs

/// Maximum mention chain depth (configurable via MentionConfig)
pub const DEFAULT_MAX_MENTION_DEPTH: u32 = 3;

/// Tracks mention chain state for depth limiting and loop-risk controls.
/// Stateless -- all chain metadata lives in DispatchTask fields.
pub struct MentionChainTracker;

impl MentionChainTracker {
    /// Check if a mention dispatch should proceed.
    ///
    /// Returns Ok(()) if the dispatch is safe, Err(MentionChainError) if blocked.
    ///
    /// # Arguments
    /// * `depth` - Current depth in the mention chain (0 = initial mention)
    /// * `parent_agent` - Name of the agent that triggered this mention
    /// * `target_agent` - Name of the agent being mentioned
    /// * `max_depth` - Maximum allowed depth from config
    pub fn check(
        depth: u32,
        parent_agent: &str,
        target_agent: &str,
        max_depth: u32,
    ) -> Result<(), MentionChainError> {
        // 1. Self-mention check
        if parent_agent == target_agent {
            return Err(MentionChainError::SelfMention { agent: target_agent.to_string() });
        }
        // 2. Depth check
        if depth >= max_depth {
            return Err(MentionChainError::DepthExceeded {
                depth,
                max_depth,
                agent: target_agent.to_string(),
            });
        }
        // 3. No ancestry-based cycle detection in this stateless check.
        // Loop risk is controlled by:
        // - self-mention rejection (parent_agent == target_agent)
        // - bounded chain depth (depth < max_depth)
        Ok(())
    }

    /// Build structured mention context for the spawned agent's task.
    ///
    /// Produces a markdown block that the agent can use to understand
    /// why it was mentioned and what context to carry forward.
    pub fn build_context(args: MentionContextArgs) -> String;
}

/// Arguments for building mention context
pub struct MentionContextArgs {
    pub parent_agent: String,
    pub issue_number: u64,
    pub comment_body: String,
    pub depth: u32,
    pub chain_id: String,
}

/// Errors from mention chain validation
#[derive(Debug, thiserror::Error)]
pub enum MentionChainError {
    #[error("agent '{agent}' cannot mention itself")]
    SelfMention { agent: String },

    #[error("mention chain depth {depth} exceeds max {max_depth} for agent '{agent}'")]
    DepthExceeded { depth: u32, max_depth: u32, agent: String },

}
```

### Modified Types

```rust
// dispatcher.rs -- extended MentionDriven variant

pub enum DispatchTask {
    // ... existing variants unchanged ...

    MentionDriven {
        agent_name: String,
        issue_number: u64,
        comment_id: u64,
        context: String,
        project: String,
        // NEW FIELDS:
        chain_id: String,       // ULID of the initial mention in this chain
        depth: u32,             // 0 = direct mention, 1 = mention-of-mention, etc.
        parent_agent: String,   // Name of the agent that triggered this mention
    },
}
```

```rust
// config.rs -- extended MentionConfig

pub struct MentionConfig {
    pub poll_modulo: u64,
    pub max_dispatches_per_tick: u32,
    pub max_concurrent_mention_agents: u32,
    // NEW FIELD:
    #[serde(default = "default_max_mention_depth")]
    pub max_mention_depth: u32,  // Default: 3
}
```

```rust
// agent_run_record.rs -- extended AgentRunRecord

pub struct AgentRunRecord {
    // ... existing fields ...
    // NEW FIELDS:
    pub mention_chain_id: Option<String>,
    pub mention_depth: Option<u32>,
    pub mention_parent_agent: Option<String>,
}
```

### Mention Context Format

The context builder produces a markdown block appended to the agent's task:

```markdown
---
**Mention Context** (chain: `{chain_id}`, depth: {depth})
Triggered by: `@adf:{parent_agent}` on issue #{issue_number}
---

{extracted_relevant_text_from_comment_body}

---
When your work is complete, you may mention another agent using `@adf:agent-name` in your output.
Maximum mention chain depth remaining: {max_depth - depth - 1}
---
```

### Mention Instructions for Agent Metaprompts

A standard snippet appended to all agent tasks when spawned via mention:

```
## Inter-Agent Coordination

You are part of an agent fleet coordinated via Gitea issue comments.
To request another agent's assistance, include `@adf:agent-name` in your output.
Available agents: {agent_list}
Your output will be posted as a Gitea comment. Any `@adf:` mentions will be detected
and the mentioned agent will be spawned with context from your output.
Mention chain depth limit: {remaining_depth} more level(s) available.
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_self_mention_rejected` | `mention_chain.rs` | Agent cannot mention itself |
| `test_depth_limit_enforced` | `mention_chain.rs` | Dispatch blocked at max depth |
| `test_depth_zero_allowed` | `mention_chain.rs` | Direct mention always allowed |
| `test_depth_one_allowed` | `mention_chain.rs` | First nested mention allowed |
| `test_depth_two_allowed` | `mention_chain.rs` | Second nested mention allowed (depth < 3) |
| `test_depth_three_blocked` | `mention_chain.rs` | Third nested mention blocked (depth >= 3) |
| `test_cycle_detection_ab_a` | `mention_chain.rs` | Documents current behaviour: A->B->A is not ancestry-blocked by `check()` |
| `test_different_agents_allowed` | `mention_chain.rs` | A->B->C allowed when depth < max |
| `test_build_context_includes_chain_id` | `mention_chain.rs` | Context has chain metadata |
| `test_build_context_includes_remaining_depth` | `mention_chain.rs` | Context shows remaining depth |
| `test_config_default_mention_depth` | `config.rs` | Default max_mention_depth = 3 |
| `test_dispatch_mention_driven_has_chain_fields` | `dispatcher.rs` | MentionDriven includes chain_id/depth/parent |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_mention_chain_depth_tracked_across_polls` | `mention_chain.rs` or tests/ | Simulate A mentions B, B's output mentions C, verify depth increments |
| `test_mention_chain_stops_at_max_depth` | tests/ | Chain stops at depth 3, warning posted to Gitea |
| `test_existing_reviewer_chain_unchanged` | tests/ | Reviewer chain (QC -> 4 reviewers -> MC) still works |
| `test_agent_run_record_includes_chain` | tests/ | AgentRunRecord has chain_id and depth after mention spawn |

### Tests NOT Needed

- End-to-end Gitea API tests (covered by existing tracker tests)
- Performance benchmarks (depth check is O(1))
- Property tests (input space is tiny: depth u32 + two agent names)

## Implementation Steps

### Step 1: Types and Error Definitions
**Files:** `src/mention_chain.rs` (new), `src/dispatcher.rs`, `src/config.rs`, `src/error.rs`
**Description:** Create `MentionChainTracker`, `MentionChainError`, `MentionContextArgs`. Add `chain_id`, `depth`, `parent_agent` to `DispatchTask::MentionDriven`. Add `max_mention_depth` to `MentionConfig`.
**Tests:** `test_self_mention_rejected`, `test_depth_limit_enforced`, `test_depth_zero_allowed`, `test_depth_one_allowed`, `test_depth_two_allowed`, `test_depth_three_blocked`, `test_cycle_detection_ab_a`, `test_config_default_mention_depth`
**Dependencies:** None
**Estimated:** 3 hours

### Step 2: Context Builder
**Files:** `src/mention_chain.rs`
**Description:** Implement `MentionChainTracker::build_context()` that produces the markdown mention context block. Include parent agent, issue number, depth, remaining depth, chain ID.
**Tests:** `test_build_context_includes_chain_id`, `test_build_context_includes_remaining_depth`
**Dependencies:** Step 1
**Estimated:** 2 hours

### Step 3: Dispatch Pipeline Integration
**Files:** `src/lib.rs`
**Description:** In `poll_mentions_for_project()`:
1. When a mention is detected and resolved, call `MentionChainTracker::check(depth, parent, target, max_depth)` before enqueuing
2. On `MentionChainError`, post a warning comment to the issue and skip dispatch
3. Determine chain_id: if the comment was posted by an agent (check `CommentUser.login` against agent names), use the parent's chain_id and increment depth. Otherwise, generate a new ULID chain_id with depth=0.
4. Call `MentionChainTracker::build_context()` and append to the agent's task
5. Enqueue `MentionDriven { ..., chain_id, depth, parent_agent }`
**Tests:** `test_mention_chain_depth_tracked_across_polls`, `test_mention_chain_stops_at_max_depth`
**Dependencies:** Steps 1, 2
**Estimated:** 4 hours

### Step 4: Agent Run Records
**Files:** `src/agent_run_record.rs`
**Description:** Add `mention_chain_id`, `mention_depth`, `mention_parent_agent` fields to `AgentRunRecord`. Populate from `DispatchTask::MentionDriven` fields during agent spawn.
**Tests:** `test_agent_run_record_includes_chain`
**Dependencies:** Step 1
**Estimated:** 1 hour

### Step 5: Metaprompt Mention Instructions
**Files:** `src/persona.rs`, `src/lib.rs`
**Description:** Add mention instruction snippet to `MetapromptRenderer`. When spawning via mention, append the snippet with available agent names and remaining depth. Include agent list from config.
**Tests:** Verify in integration test that spawned agent task includes mention instructions
**Dependencies:** Step 3
**Estimated:** 2 hours

### Step 6: Backward Compatibility Verification
**Files:** tests/
**Description:** Verify existing reviewer chain, compound review, and cron-driven agents work unchanged. Run full test suite.
**Tests:** `test_existing_reviewer_chain_unchanged`
**Dependencies:** Steps 1-5
**Estimated:** 2 hours

## Rollback Plan

If issues discovered:
1. Set `max_mention_depth = 0` in config -- disables all mention dispatch (including direct mentions)
2. The depth check is a gate, not a mutation -- removing it restores original behaviour
3. New fields in `DispatchTask::MentionDriven` default to `chain_id: String::new()`, `depth: 0`, `parent_agent: String::new()` -- backward compatible with any serialised dispatch tasks

Feature flag: `max_mention_depth` config value (0 = all mention dispatch disabled, 3 = default)

## Migration

### Config Migration

```toml
# NEW field in [mentions] section
[mentions]
max_mention_depth = 3    # New: max mention chain nesting depth
# Existing fields unchanged:
poll_modulo = 2
max_dispatches_per_tick = 3
max_concurrent_mention_agents = 5
```

### No Data Migration

All chain state is ephemeral (lives in DispatchTask during dispatch). No SQLite schema changes needed.

## Dependencies

### New Dependencies

None. Uses existing `ulid` crate (already in Cargo.toml for correlation IDs).

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Depth check latency | < 0.01ms | String comparison (O(1)) |
| Context build latency | < 0.1ms | String formatting |
| Added memory per dispatch | < 200 bytes | 3 String fields in DispatchTask |

No benchmarks needed. All operations are O(1) string comparisons.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Determine how to detect if a comment was posted by an agent vs human | Pending | Implementation (check CommentUser.login against agent names from config) |
| Verify compound review is not affected | Pending | Integration test |
| Verify webhook-driven dispatch is not affected | Pending | Integration test |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
