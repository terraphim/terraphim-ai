# Design & Implementation Plan: ADF PR Gate Reconciliation (Gitea #1122)

**Date**: 2026-05-01
**Issue**: terraphim/terraphim-ai#1122
**Research Doc**: `.docs/research-adf-pr-merge-progress-2026-05-01.md`

## 1. Summary of Target Behaviour

A canonical PR gate reconciler runs periodically (every N reconcile ticks), reads actual commit statuses and branch protection requirements from Gitea, classifies each open PR head into a deterministic state, and takes action: enqueue missing agents, open deduplicated remediation issues, or clear the PR for auto-merge evaluation.

## 2. Key Invariants and Acceptance Criteria

| # | Criterion | Acceptance Test |
|---|-----------|-----------------|
| AC-1 | For every open PR head, report required context state | Unit test: `reconcile_pr_gate` with missing contexts |
| AC-2 | Missing required contexts enqueue the correct agent | Integration: reconciler returns `EnqueueMissingChecks` |
| AC-3 | `adf/build` posted on PR head SHA | Unit test: `CommitStatusSummary` parsing |
| AC-4 | `adf/pr-reviewer` posted on PR head SHA | Unit test: same |
| AC-5 | Status-post failures become `FactoryFault` | Unit test: API error path |
| AC-6 | Green contexts proceed to auto-merge | Unit test: `ReadyForPolicy` decision |
| AC-7 | Low-confidence PRs create one remediation issue | Unit test: `AwaitingHumanReview` + dedup |
| AC-8 | No duplicate remediation issues | Unit test: `remediation_key` dedup |
| AC-9 | PR #1099 reaches deterministic final state | Manual acceptance fixture |
| AC-10 | All `PrGateDecision` variants have unit tests | Coverage check |

## 3. High-Level Design and Boundaries

### New Module: `pr_gate.rs`

Pure-function module (no I/O, no runtime state) following the `pr_review.rs` pattern:

```
PrGateSnapshot -> reconcile_pr_gate() -> PrGateDecision
```

### New Tracker API Methods

Two read methods added to `terraphim_tracker/src/gitea.rs`:

```
list_commit_statuses(sha) -> Vec<CommitStatus>
get_branch_protection(branch) -> BranchProtection
```

### New Reconcile Tick Step

Between Step 17 (drain dispatch) and Step 18 (poll reviews), insert:

```
Step 17.5: PR gate reconciliation (every N ticks)
```

### Boundaries

| Change | Inside Component | New Component | Side Effects |
|--------|-----------------|---------------|--------------|
| Pure types + reconcile logic | - | `pr_gate.rs` | None |
| Status read API | `terraphim_tracker` | - | Network |
| Branch protection read API | `terraphim_tracker` | - | Network |
| Reconciler tick step | `lib.rs` | - | Enqueue tasks |
| Remediation issue creation | `lib.rs` | - | Gitea API |

## 4. File/Module-Level Change Plan

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/terraphim_orchestrator/src/pr_gate.rs` | **Create** | Pure types + `reconcile_pr_gate()` |
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | Add `pub mod pr_gate;`, add reconciler step in tick |
| `crates/terraphim_tracker/src/gitea.rs` | Modify | Add `list_commit_statuses()`, `get_branch_protection()` |
| `crates/terraphim_tracker/src/lib.rs` | Modify | Re-export new types if needed |
| `crates/terraphim_orchestrator/src/config.rs` | Modify | Add `gate_reconcile_interval_ticks: u32` (default 10) |
| `crates/terraphim_orchestrator/src/pr_poller.rs` | Modify | Use reconciler decision before polling comments |

## 5. Step-by-Step Implementation Sequence

### Phase 1: Pure Reconciler Types (`pr_gate.rs`)

**Purpose**: Define the canonical gate state machine. No I/O.

Create `crates/terraphim_orchestrator/src/pr_gate.rs` with:

```rust
/// Snapshot of everything the reconciler needs to classify a PR head.
pub struct PrGateSnapshot {
    pub pr_number: u64,
    pub head_sha: String,
    pub base_branch: String,
    pub required_contexts: Vec<String>,          // from branch protection
    pub head_statuses: Vec<CommitStatusSummary>,  // from commit status API
    pub has_reviewer_comment: bool,               // from comment poll
    pub reviewer_verdict: Option<ReviewVerdict>,   // parsed if present
}

/// One commit status entry for a SHA.
pub struct CommitStatusSummary {
    pub context: String,
    pub state: CommitStatusState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitStatusState {
    Pending,
    Success,
    Failure,
    Error,
}

/// Deterministic classification of a PR head's gate state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrGateDecision {
    /// All required contexts green; proceed to auto-merge policy.
    ReadyForPolicy,
    /// Required contexts not yet posted; enqueue the responsible agents.
    EnqueueMissingChecks { missing: Vec<String> },
    /// Required contexts posted but still pending; wait.
    AwaitingChecks { pending: Vec<String> },
    /// At least one required context failed; open remediation issue.
    BlockedByFailedChecks { failed: Vec<(String, String)> },
    /// Reviewer posted but confidence/criteria below threshold.
    AwaitingHumanReview { reason: String },
    /// Status API or branch protection API failure; service fault.
    FactoryFault { error: String },
}

/// Reconcile the PR gate state from a snapshot. Pure function.
pub fn reconcile_pr_gate(snapshot: &PrGateSnapshot) -> PrGateDecision

/// Compute which required contexts are absent from the head SHA.
pub fn missing_required_contexts(
    required: &[String],
    statuses: &[CommitStatusSummary],
) -> Vec<String>

/// Check if all required contexts have terminal success state.
pub fn terminal_required_contexts_green(
    required: &[String],
    statuses: &[CommitStatusSummary],
) -> bool

/// Deterministic dedup key for remediation issues.
/// Format: "project:pr:sha:context" or "project:pr:sha:human-review"
pub fn remediation_key(project: &str, pr_number: u64, head_sha: &str, decision: &PrGateDecision) -> String
```

**Deployable**: Yes. Module compiles independently, no callers yet.

**Tests**: Unit test every `PrGateDecision` variant with crafted snapshots.

### Phase 2: Tracker API Additions (`terraphim_tracker`)

**Purpose**: Read commit statuses and branch protection from Gitea.

Add to `crates/terraphim_tracker/src/gitea.rs`:

```rust
/// GET /repos/{owner}/{repo}/commits/{sha}/statuses
pub async fn list_commit_statuses(&self, sha: &str) -> Result<Vec<CommitStatusEntry>>

/// GET /repos/{owner}/{repo}/branch_protections/{branch}
pub async fn get_branch_protection(&self, branch: &str) -> Result<BranchProtection>

pub struct CommitStatusEntry {
    pub context: String,
    pub state: String,  // "pending", "success", "failure", "error"
    pub description: Option<String>,
    pub target_url: Option<String>,
}

pub struct BranchProtection {
    pub enable_status_check: bool,
    pub status_check_contexts: Vec<String>,
}
```

**Deployable**: Yes. New methods, no breaking changes.

**Tests**: Integration test against real Gitea API (or mock server).

### Phase 3: Reconciler Integration into Tick Loop

**Purpose**: Wire the reconciler into the main reconcile tick.

Modify `crates/terraphim_orchestrator/src/lib.rs`:

1. Add `pub mod pr_gate;`
2. Add config field `gate_reconcile_interval_ticks: u32` (default: 10)
3. Between Step 17 and Step 18, add Step 17.5:

```
if tick_counter % gate_reconcile_interval_ticks == 0 {
    reconcile_pr_gates().await;
}
```

4. `reconcile_pr_gates()` for each project:
   a. Read branch protection for `main` -> required contexts
   b. List open PRs
   c. For each open PR: read commit statuses for head SHA
   d. Build `PrGateSnapshot`
   e. Call `reconcile_pr_gate()`
   f. Act on `PrGateDecision`:
      - `ReadyForPolicy`: let Step 18 (poll reviews) handle it
      - `EnqueueMissingChecks`: dispatch the missing agents with PR env
      - `AwaitingChecks`: log and skip (will be rechecked next interval)
      - `BlockedByFailedChecks`: open/update dedup remediation issue
      - `AwaitingHumanReview`: open/update dedup remediation issue
      - `FactoryFault`: open/update dedup remediation issue + log error

**Deployable**: Yes. Feature-gated if needed.

**Tests**: Integration test with PR #1099 fixture data.

### Phase 4: PR-Head Build Status

**Purpose**: Ensure `adf/build` is produced for PR head SHAs.

This is already partially handled by `post_pending_status()` at `lib.rs:2436`. The gap is:
- `pending` is posted, but if the `build-runner` agent never runs or crashes, no terminal status is posted
- The reconciler (Phase 3) will detect this via `EnqueueMissingChecks` or `AwaitingChecks` with timeout

**Change**: In the reconciler, if `adf/build` has been `pending` for > 30 minutes, transition to `FactoryFault` and open remediation issue.

**Deployable**: Yes, as part of Phase 3 reconciler logic.

### Phase 5: PR Reviewer Terminal Status

**Purpose**: Every `pr-reviewer` completion posts terminal `adf/pr-reviewer` on the PR head SHA.

**Change**: In `output_poster.rs` or a new hook in `lib.rs`, after the pr-reviewer agent exits:
1. Parse the agent's exit code
2. If exit 0: post `adf/pr-reviewer` = `success` on head SHA
3. If exit != 0: post `adf/pr-reviewer` = `failure` on head SHA
4. If status post fails: log `FactoryFault`

This replaces the fragile bash/curl approach with orchestrator-managed status posting.

**Deployable**: Yes. Status post is idempotent.

### Phase 6: Service Reliability

**Purpose**: Ensure `adf-orchestrator.service` does not remain inactive.

**Change**: Add a health-check step to the reconcile tick:
- Every N ticks, verify the orchestrator can reach Gitea API
- If unreachable for 3 consecutive intervals, emit `FactoryFault`
- Document systemd policy: `Restart=always`, `RestartSec=10`

**Deployable**: Yes. Documentation + config change only.

## 6. Testing and Verification Strategy

| Acceptance Criterion | Test Type | Location |
|---------------------|-----------|----------|
| AC-1: Report required context state | Unit | `pr_gate.rs` tests |
| AC-2: Missing contexts enqueue agent | Unit + Integration | `pr_gate.rs` + `lib.rs` |
| AC-3: `adf/build` posted | Unit | Tracker API test |
| AC-4: `adf/pr-reviewer` posted | Unit | Tracker API test |
| AC-5: FactoryFault on API failure | Unit | `pr_gate.rs` |
| AC-6: Green -> ReadyForPolicy | Unit | `pr_gate.rs` |
| AC-7: Low confidence -> remediation | Unit | `pr_gate.rs` |
| AC-8: No duplicate issues | Unit | `remediation_key` dedup test |
| AC-9: PR #1099 final state | Manual | Acceptance fixture |
| AC-10: All variants tested | Coverage | `cargo test -p terraphim_orchestrator` |

### Test Fixture: PR #1099

```rust
#[test]
fn pr_1099_missing_both_required_contexts() {
    let snapshot = PrGateSnapshot {
        pr_number: 1099,
        head_sha: "9c287d68".into(),
        base_branch: "main".into(),
        required_contexts: vec!["adf/build".into(), "adf/pr-reviewer".into()],
        head_statuses: vec![],  // No statuses posted
        has_reviewer_comment: true,
        reviewer_verdict: Some(ReviewVerdict {
            confidence: 3,  // Below threshold
            ..default_verdict()
        }),
    };
    let decision = reconcile_pr_gate(&snapshot);
    assert_eq!(decision, PrGateDecision::EnqueueMissingChecks {
        missing: vec!["adf/build".into(), "adf/pr-reviewer".into()],
    });
}
```

## 7. Risk and Complexity Review

| Risk | Mitigation | Residual |
|------|------------|----------|
| Gitea API rate limits | Reconcile every 10 ticks (not every tick) | None |
| Duplicate status posts | Gitea allows multiple statuses per context (uses latest) | None |
| Reconciler step delays tick loop | Pure function is fast; I/O is async | None |
| Existing agents post statuses via curl | Reconciler reads actual state, doesn't assume | None |
| Phase 5 changes agent output flow | Status post is additive, non-breaking | Low |

## 8. Implementation Order and Dependencies

```
Phase 1 (pr_gate.rs)     -- no dependencies, pure types
    |
    v
Phase 2 (tracker API)    -- no dependencies, new methods
    |
    v
Phase 3 (tick integration) -- depends on Phase 1 + 2
    |
    +---> Phase 4 (build status timeout) -- sub-case of Phase 3
    |
    +---> Phase 5 (reviewer terminal status) -- depends on Phase 2
    |
    +---> Phase 6 (service reliability) -- independent
```

Phases 1 and 2 can be implemented in parallel. Phase 3 depends on both. Phases 4-6 can be parallelised after Phase 3.

## 9. Open Questions / Decisions for Human Review

1. **Reconcile interval**: Default every 10 ticks (~10 minutes at 1 tick/min)? Or configurable?
2. **Stale pending timeout**: 30 minutes? Longer for build-runner?
3. **Should Phase 5 replace the bash/curl status posting entirely?** Or run both as belt-and-suspenders?
4. **Should remediation issues auto-close** when the PR gate unblocks?
5. **Should `AutoMergeCriteria` become configurable** as part of this work or a separate issue?
