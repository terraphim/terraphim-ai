# Implementation Plan: PR #1788 Remaining Slices (2-8)

**Status**: Draft
**Research Doc**: `.docs/research-pr-1788-slices-2-8.md`
**Author**: OpenCode
**Date**: 2026-05-31
**Estimated Effort**: 4-6 focused sessions

## Overview

### Summary

This plan decomposes the remaining seven slices from PR #1788 into independently implementable units, grouped by theme and dependency. Slices 2-3 (observability/security) are combined because Slice 3 depends on Slice 2. Slices 4-8 are sequenced after 2-3 based on priority and risk.

### Approach

Extract each slice as a focused branch from current `main`. Do not rebase the full #1788 branch. Each slice gets its own verification and validation before merge.

### Scope

**In Scope:**

| Slice | Feature | Priority |
|-------|---------|----------|
| Slice 2 | Output capture buffer with redaction | High |
| Slice 3 | Timeout output posting to Gitea | High (blocked on Slice 2) |
| Slice 5 | Webhook group alias dispatch | Medium |
| Slice 6 | Worktree fail-closed behaviour | Medium |
| Slice 7 | Provider probe timeout reduction | Medium |

**Out of Scope:**

| Slice | Feature | Disposition |
|-------|---------|-------------|
| Slice 4 | Agent registry | Defer until wired into lookups |
| Slice 8 | TLA specs + generated docs | Review separately; never commit generated learnings |

**Avoid At All Cost:**
- Bundling any two slices into one PR.
- Merging Slice 3 before Slice 2 has redaction.
- Removing content-based provider probe classification.
- Committing `.terraphim/learnings/*.md`.

## Architecture

### Component Diagram

```text
┌─────────────────────────────────────────────────────────────┐
│                     Slice 2: Output Capture                  │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐   │
│  │ Child stdout │───▶│ OutputCapture│───▶│ Broadcast    │   │
│  │ Child stderr │───▶│              │───▶│ MPSC         │   │
│  └──────────────┘    │ + VecDeque   │    │ + VecDeque   │   │
│                      │   (redacted) │    │   (redacted) │   │
│                      └─────────────┘    └──────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Slice 3: Timeout Reporting                  │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐   │
│  │poll_wall_    │───▶│ captured_   │───▶│ output_poster│   │
│  │  timeouts()  │    │   events()  │    │ (redacted)   │   │
│  └──────────────┘    └─────────────┘    └──────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Slice 5: Webhook Aliases                   │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐   │
│  │ Gitea webhook│───▶│ group_alias │───▶│ dispatch_tx  │   │
│  │   mention    │    │  _members() │    │ (capped)     │   │
│  └──────────────┘    └─────────────┘    └──────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                Slice 6: Worktree Fail-Closed                 │
│  ┌──────────────┐    ┌─────────────┐                        │
│  │ Agent spawn  │───▶│ create_     │───▶ Err (don't run)   │
│  │   request    │    │ agent_worktree                     │
│  └──────────────┘    └─────────────┘                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Slice 7: Provider Probe Timeout                 │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐   │
│  │ KG router    │───▶│ probe_single│───▶│ CircuitBreaker│  │
│  │   rules      │    │ (15s timeout│    │ (provider:model)│ │
│  └──────────────┘    │ + content   │    └──────────────┘   │
│                      │   classification)                  │
│                      └─────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow: Slices 2-3 Combined

```text
Input: Child stdout/stderr line
  -> OutputCapture::capture_stdout/stderr
  -> parse line (mention detection)
  -> redact line (pattern-based)
  -> store in VecDeque<OutputEvent> (MAX_CAPTURED_EVENTS)
  -> broadcast to live subscribers
  -> send to mpsc event channel

Input: Wall-clock timeout detected
  -> AgentOrchestrator::poll_wall_timeouts()
  -> handle.captured_events() (already redacted)
  -> filter_map to lines
  -> append timeout_summary()
  -> post_agent_output_for_project() with redaction marker verification
  -> Gitea issue comment
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Combine Slices 2-3 | Slice 3 depends on Slice 2; redaction must be designed before posting | Separate PRs with interface churn |
| Redact at capture time | Defense in depth: buffer never holds secrets | Redact only at posting time (risky if other callers read buffer) |
| Use VecDeque instead of Vec | O(1) eviction instead of O(n) remove(0) | Keep Vec with remove(0) (performance regression) |
| Keep content-based probe classification | Prevents misclassifying token-less responses as healthy | Remove classification (reintroduces known regression) |
| Cap group alias expansion | Prevents mention-spam DoS | Unlimited expansion (risky) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Merge all slices at once | Repeats #1788 bundling mistake | High regression risk, hard rollback |
| Agent registry without wiring | Adds code with no functional benefit | Maintenance burden, confusion |
| Post raw output without redaction | Security risk | Credential leakage in Gitea issues |
| Remove probe content classification | Reintroduces known regression | Wrong routing decisions |
| Commit generated learnings | Not source code | Repository bloat, noise |

### Simplicity Check

**Senior Engineer Test**: A senior engineer would insist on redaction before any remote posting, would reject the O(n) Vec eviction, and would defer the registry until it's actually used. This plan does all three.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later" (registry deferred)
- [x] No flexibility "just in case"
- [x] Error handling covers realistic scenarios only
- [x] No premature optimisation

## File Changes

### Slice 2-3: Output Capture + Timeout Reporting

#### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_spawner/src/output.rs` | Replace `Vec` with `VecDeque`, add redaction, add `captured_events()` |
| `crates/terraphim_orchestrator/src/lib.rs` | Update `poll_wall_timeouts()` to use captured output with redaction verification |
| `crates/terraphim_orchestrator/src/output_poster.rs` | Add redaction marker check before posting |

#### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_spawner/src/redaction.rs` | Redaction policy and pattern matching |

### Slice 5: Webhook Group Alias Dispatch

#### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/webhook.rs` | Add `group_alias_members()`, cap expansion, config field |
| `crates/terraphim_orchestrator/src/config.rs` | Add `max_group_alias_members` to orchestrator config |

### Slice 6: Worktree Fail-Closed

#### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/worktree_guard.rs` or related file | Verify exact location and apply fail-closed change |

### Slice 7: Provider Probe Timeout

#### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/provider_probe.rs` | Reduce timeout from 120s to 15s, keep content classification, keep CLI-specific breakers |

### Slice 4: Agent Registry (Deferred)

No file changes unless explicitly re-scoped.

### Slice 8: TLA/Docs (Deferred)

No file changes unless explicitly re-scoped.

## API Design

### Slice 2: Redaction Module

```rust
// crates/terraphim_spawner/src/redaction.rs

/// Patterns that indicate sensitive content in agent output.
pub const DEFAULT_REDACTION_PATTERNS: &[&str] = &[
    r"(?i)(api[_-]?key\s*[:=]\s*)[^\s]+",
    r"(?i)(token\s*[:=]\s*)[^\s]+",
    r"(?i)(secret\s*[:=]\s*)[^\s]+",
    r"(?i)(password\s*[:=]\s*)[^\s]+",
    r"(?i)(sk-[a-zA-Z0-9]{20,})",
    r"(?i)(ghp_[a-zA-Z0-9]{36})",
];

/// Redact sensitive patterns from a string, replacing matches with `***REDACTED***`.
pub fn redact(input: &str) -> String {
    // Implementation: apply all patterns sequentially
}

/// Verify that a string contains no apparent secrets (for pre-posting check).
pub fn verify_redacted(input: &str) -> bool {
    // Implementation: true if no patterns match
}
```

### Slice 2: OutputCapture Changes

```rust
// In OutputCapture struct:
captured_events: Arc<Mutex<VecDeque<OutputEvent>>>,

// In capture_stdout/capture_stderr:
let redacted_line = redaction::redact(&line);
// Store redacted_line in OutputEvent instead of raw line
```

### Slice 3: Timeout Reporting

```rust
// In poll_wall_timeouts():
let output_lines: Vec<String> = managed
    .handle
    .captured_events()
    .into_iter()
    .filter_map(|event| match event {
        OutputEvent::Stdout { line, .. } => Some(line),
        OutputEvent::Stderr { line, .. } => Some(format!("[stderr] {line}")),
        _ => None,
    })
    .collect();

// Verify all lines are redacted before posting
assert!(output_lines.iter().all(|l| redaction::verify_redacted(l)));

// Post with explicit redaction marker
timeout_lines.push(format!("[timeout] agent exceeded limit after {} (limit {})",
    format_runtime_duration(elapsed_secs),
    format_runtime_duration(runtime_limit_secs)));
```

### Slice 5: Webhook Alias Config

```rust
// In OrchestratorConfig:
pub max_group_alias_members: Option<usize>, // default: 10

// In webhook.rs:
const DEFAULT_MAX_GROUP_ALIAS_MEMBERS: usize = 10;

fn group_alias_members(alias: &str, agent_names: &[String], max: usize) -> Vec<&str> {
    let prefix = format!("{}-", alias);
    agent_names
        .iter()
        .filter_map(|name| name.strip_prefix(&prefix).map(|_| name.as_str()))
        .take(max)
        .collect()
}
```

### Slice 7: Provider Probe

```rust
// In provider_probe.rs:
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

// Keep CLI-specific circuit breaker keys:
let key = format!("{}:{}:{}", result.cli_tool, result.provider, result.model);

// Keep content classification:
let token_bearing = has_token_bearing_output(&stdout_str);
if output.status.success() && token_bearing { /* healthy */ }
else if output.status.success() && !token_bearing { /* unhealthy */ }
```

## Test Strategy

### Slice 2-3: Output Capture + Timeout Reporting

#### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_redact_api_key` | `redaction.rs` | Verify API key pattern is redacted |
| `test_redact_token` | `redaction.rs` | Verify token pattern is redacted |
| `test_redact_no_false_positives` | `redaction.rs` | Verify safe text is not redacted |
| `test_verify_redacted_detects_leak` | `redaction.rs` | Verify pre-posting check catches leaks |
| `test_capture_buffer_bounded` | `output.rs` | Verify VecDeque eviction at MAX_CAPTURED_EVENTS |
| `test_capture_buffer_redacts` | `output.rs` | Verify stored events are redacted |

#### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_timeout_posting_redacted` | `orchestrator_tests.rs` | Verify timeout output is redacted before posting |
| `test_timeout_posting_includes_summary` | `orchestrator_tests.rs` | Verify timeout summary is appended |

### Slice 5: Webhook Alias Dispatch

#### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_group_alias_members_respects_cap` | `webhook.rs` | Verify max members is enforced |
| `test_group_alias_members_no_false_matches` | `webhook.rs` | Verify `implementation-swarmish` does not match `implementation-swarm` |

### Slice 7: Provider Probe

#### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_probe_timeout_is_15s` | `provider_probe.rs` | Verify timeout duration |
| `test_probe_content_classification_healthy` | `provider_probe.rs` | Verify token-bearing output is healthy |
| `test_probe_content_classification_unhealthy` | `provider_probe.rs` | Verify token-less exit-0 is unhealthy |

## Implementation Steps

### Phase A: Slices 2-3 (Output Capture + Timeout Reporting)

#### Step 1: Redaction Module

**Files:** `crates/terraphim_spawner/src/redaction.rs`
**Description:** Create redaction module with pattern-based secret scrubbing.
**Tests:** Unit tests for redact and verify_redacted.
**Estimated:** 1 hour.

#### Step 2: OutputCapture VecDeque + Redaction

**Files:** `crates/terraphim_spawner/src/output.rs`
**Description:** Replace `Vec` with `VecDeque`, apply redaction before storage, add `captured_events()`.
**Tests:** Buffer bounds, redaction, event retrieval.
**Dependencies:** Step 1.
**Estimated:** 1.5 hours.

#### Step 3: Timeout Reporting with Redaction Verification

**Files:** `crates/terraphim_orchestrator/src/lib.rs`, `src/output_poster.rs`
**Description:** Update `poll_wall_timeouts()` to use captured events, verify redaction before posting.
**Tests:** Integration tests for timeout posting.
**Dependencies:** Step 2.
**Estimated:** 2 hours.

#### Step 4: Verification

**Files:** All changed files.
**Description:** UBS, tests, clippy, fmt, coverage.
**Tests:** Full test suite for changed crates.
**Dependencies:** Steps 1-3.
**Estimated:** 1.5 hours.

### Phase B: Slice 5 (Webhook Alias Dispatch)

#### Step 5: Config + Alias Expansion

**Files:** `crates/terraphim_orchestrator/src/config.rs`, `src/webhook.rs`
**Description:** Add `max_group_alias_members` config, cap expansion, tests.
**Tests:** Unit tests for alias members and cap.
**Estimated:** 1.5 hours.

### Phase C: Slice 6 (Worktree Fail-Closed)

#### Step 6: Verify and Apply

**Files:** TBD after locating exact change.
**Description:** Find the fail-closed change, verify it, apply to focused branch.
**Tests:** Worktree creation failure path.
**Estimated:** 1 hour.

### Phase D: Slice 7 (Provider Probe Timeout)

#### Step 7: Timeout Reduction Only

**Files:** `crates/terraphim_orchestrator/src/provider_probe.rs`
**Description:** Reduce timeout to 15s, keep content classification, keep CLI-specific breakers.
**Tests:** Unit tests for timeout and classification.
**Estimated:** 1 hour.

## Rollback Plan

For each slice:

1. Revert the merge commit for that slice only.
2. Push revert to both `origin` and `gitea`.
3. Comment on the original issue/PR explaining the revert.

Because each slice is focused and touches distinct areas, rollback should be low-risk and isolated.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

### Slice 2: Output Capture

| Metric | Target | Measurement |
|--------|--------|-------------|
| Buffer eviction | O(1) per event | VecDeque vs Vec remove(0) |
| Redaction overhead | < 1ms per line | Benchmark with regex |
| Memory per agent | < 5MB (4096 events * ~1KB avg) | Memory profiling |

### Slice 7: Provider Probe

| Metric | Target | Measurement |
|--------|--------|-------------|
| Probe latency | < 15s | Timeout constant |
| False positive rate | < 1% | Monitor circuit breaker flapping |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Exact location of worktree fail-closed change | Pending | Implementer |
| Redaction pattern completeness review | Pending | Security reviewer |
| Probe timeout baseline measurement | Pending | Ops |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Redaction policy approved
- [ ] Human approval received
