# Implementation Plan: ADF Closed-Loop Flow Remediation

**Status**: Draft
**Research Doc**: `.docs/research-adf-flow-remediation.md`
**Author**: AI Agent
**Date**: 2026-06-09
**Estimated Effort**: 1-2 days

## Overview

### Summary
Fix ADF as a complete delivery loop, not as isolated status checks. The target flow is: Gitea issue -> disciplined research -> disciplined design -> implementation -> PR -> native CI -> ADF review/validation/verification -> remediation -> re-review -> merge.

### Approach
Make verdict posting deterministic in the orchestrator, preserve native Gitea CI as the build gate, and add a bounded review-remediation loop with clear stop conditions. Do not re-enable retired ADF bash build-runner agents.

### Scope

**In Scope:**
- Fix #2301: Orchestrator-side parseable verdict comments from review-agent output.
- Full ADF PR loop: review -> remediation -> re-review -> auto-merge.
- Explicit review-cycle stop conditions.
- Stop `implementation-swarm` no-work hot-loop.
- Align branch protection with actual contexts: `native-ci / build (push)`, `adf/pr-reviewer`, `adf/validation`, `adf/verification`.
- Clean duplicate auto-merge failure issues after the loop is fixed.
- Native runner hygiene: keep `terraphim-gitea-runner` active, remove duplicate stale runner processes.

**Out of Scope:**
- Fix `zestic-ai/odilo` branch protection; token lacks admin write.
- Re-enable retired ADF bash build-runner agents.
- Force-merge PRs without passing native CI and ADF verdict gates.
- Rewrite the whole orchestrator scheduler.

**Avoid At All Cost:**
- Treating agent exit code as a review verdict.
- Requiring `adf/build` where the real build context is `native-ci / build (push)`.
- Creating duplicate remediation or auto-merge failure issues per reconcile tick.
- Infinite review-remediation loops.

## Architecture

### Target ADF Flow

```text
Gitea ready issue
  -> implementation-swarm claims exactly one issue
  -> disciplined-research
  -> disciplined-design
  -> implementation on task branch
  -> PR opened
  -> native-ci / build (push)
  -> pr-reviewer + pr-validator + pr-verifier
  -> orchestrator posts parseable verdict comments and statuses
  -> if findings: remediation dispatch to implementation agent
  -> agent fixes review comments on same branch
  -> PR head SHA changes
  -> native-ci and ADF review agents re-run
  -> stop when clean, exhausted, no progress, or escalated
  -> auto-merge only on clean fresh gates
```

### Required Status Contexts

| Context | Source | Meaning |
|---------|--------|---------|
| `native-ci / build (push)` | `terraphim-gitea-runner` running `.gitea/workflows/native-ci.yml` | Deterministic fmt/clippy/build/test gate |
| `adf/pr-reviewer` | `pr-reviewer` | Structural review verdict |
| `adf/validation` | `pr-validator` | Requirements and acceptance validation verdict |
| `adf/verification` | `pr-verifier` | Implementation and test verification verdict |

Do not require `adf/build` unless the repository branch protection and runner intentionally emit that context. Current `terraphim-ai/main` requires `native-ci / build (push)`.

## Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Orchestrator posts verdict comments | Models do not reliably run `gtr comment`; parseable comments are required for auto-merge/remediation | Relying on agent prompt compliance |
| Native runner remains build path | `terraphim-gitea-runner` is already doing real cargo work; ADF bash build-runner was retired | Re-enabling disabled bash build-runners |
| Bounded remediation loop | Prevents infinite review cycles and duplicate issue spam | Unbounded re-review on every reconcile tick |
| Status based on parsed verdict | Exit code only proves process success, not review approval | Green status on `empty_success` |

## Agent Skill Coverage Proof

This plan depends on live ADF agents carrying the disciplined-engineering skills. Verified from `/opt/ai-dark-factory/conf.d/terraphim.toml` on bigbox.

| Lifecycle Stage | Required Skill | Live Agent | Config Evidence | Covered? |
|-----------------|----------------|------------|-----------------|----------|
| Issue selection and implementation planning | `disciplined-research` | `implementation-swarm` | lines 1736-1752: `skill_chain = ["disciplined-research", ...]` | Yes |
| Design before code | `disciplined-design` | `implementation-swarm` | lines 1745-1748 include `disciplined-design` | Yes |
| Code implementation | `disciplined-implementation` | `implementation-swarm` | line 1748 includes `disciplined-implementation` | Yes |
| Implementation self-check before PR | `disciplined-verification`, `testing`, `rust-mastery` | `implementation-swarm` | lines 1749-1751 include `disciplined-verification`, `rust-mastery`, `testing` | Yes |
| Structural PR review | `structural-pr-review` | `pr-reviewer` | line 911: `skill_chain = ["structural-pr-review"]`; lines 954-970 require the structural review template | Yes |
| Requirements/user-purpose validation | `disciplined-validation` | `pr-validator` | line 1885: `skill_chain = ["disciplined-validation"]`; line 1935 prompt applies the skill | Yes |
| Design-to-code/test verification | `disciplined-verification` | `pr-verifier` | line 2074: `skill_chain = ["disciplined-verification"]`; line 2124 prompt applies the skill | Yes |
| Sub-project coverage | Same PR agents via `extra_projects` | `pr-reviewer`, `pr-validator`, `pr-verifier` | lines 903, 1877, 2066 list `terraphim-core`, `terraphim-config-persistence`, `terraphim-service`, `terraphim-agents`, `terraphim-kg-agents`, `terraphim-clients` | Yes |
| Remediation loop | `disciplined-implementation` plus current findings | `implementation-swarm` | must be routed by new remediation dispatch to the existing PR branch | Covered by agent, dispatch wiring to implement |

### Required Dispatch Contract

The remediation loop must route findings back to `implementation-swarm`, not to a generic agent. The remediation prompt must require:

- Read the original issue and current PR.
- Read the latest parseable verdict comments from `pr-reviewer`, `pr-validator`, and `pr-verifier`.
- Apply `disciplined-implementation` only to the listed findings.
- Preserve the existing PR branch; do not create a new PR.
- Run local verification before pushing.
- Include the remediation attempt number and remaining budget.

This preserves the left-to-right V-model discipline: research/design/implementation are covered by `implementation-swarm`; validation and verification are covered by PR agents; structural review is covered by `pr-reviewer`; remediation returns to implementation and then repeats the PR-side gates.

## Fix 0: Verdict Comment Posting (#2301)

### Problem
Review agents (`pr-reviewer`, `pr-validator`, `pr-verifier`) can produce useful review output, but the system relies on the agent/model to run `gtr comment` correctly. When it does not, `auto_merge_impl` sees no fresh parseable verdict and reports `NoReviewerComment`, `StaleReview`, or `HumanReviewNeeded`.

### Design
Move verdict publication into the orchestrator:
- Read the review-agent drain log after completion.
- Extract assistant-visible review text.
- Normalise it into the existing `pr_review::parse_verdict` format.
- Post a Gitea PR comment containing `Confidence Score: N/5`, `Inline Findings`, and `Last reviewed commit: <head-sha>`.
- Set commit status from the parsed verdict, not the process exit code alone.

### Stop Condition
If no parseable verdict can be extracted after one agent run, mark the relevant ADF status `failure`, create or update a single deduplicated issue keyed by `repo/pr/head_sha/agent`, and do not retry that same agent on the same head SHA until the PR head changes or a human explicitly re-triggers it.

## Fix 0b: Bounded Review-Remediation Loop

### Loop States

| State | Meaning | Next Step |
|-------|---------|-----------|
| `review_pending` | PR head has no fresh verdicts | Dispatch PR agents |
| `review_failed` | Agent failed or verdict unparseable | Dedup issue, wait for new head/manual trigger |
| `findings_present` | Parseable verdict contains actionable findings | Dispatch remediation |
| `remediation_running` | Implementation agent fixing review comments | Wait for push |
| `re_review_pending` | New head SHA after remediation | Dispatch PR agents again |
| `clean` | Native CI and all ADF verdicts are fresh and passing | Auto-merge |
| `escalated` | Stop condition reached | Human review required |

### Explicit Stop Conditions

The review cycle stops when any of these is true:

1. **Clean gate**: `native-ci / build (push)`, `adf/pr-reviewer`, `adf/validation`, and `adf/verification` are all `success` for the current head SHA and verdicts contain no P0/P1 blocking findings.
2. **Attempt budget exhausted**: `max_remediation_attempts = 3` for the same PR lineage. A lineage is the original PR plus all remediation pushes derived from it.
3. **No-progress loop**: remediation produces a new head SHA but the same finding fingerprint appears twice consecutively. Fingerprint = `{agent, severity, file, line if available, normalised finding text}`.
4. **Unparseable verdict**: a review agent completes but the orchestrator cannot extract a parseable verdict for the same head SHA. One dedup issue is created/updated and retries pause.
5. **Hard P0 escalation**: any P0/security/data-loss finding marked non-remediable by the reviewer, or repeated P0 after one remediation attempt.
6. **Native CI failure**: native CI fails twice on the same head SHA without code changes. Do not keep re-running ADF review agents; dispatch implementation once or escalate.
7. **Branch conflict/merge conflict**: auto-merge returns merge conflict or protected-branch failure after gates are green. Create/update one issue and stop.

### Remediation Dispatch Rules

- Dispatch remediation only for fresh verdicts on the current head SHA.
- Remediation must work on the existing PR branch, not create a duplicate branch or PR.
- Remediation prompt must include findings, file references, current head SHA, and stop-condition budget.
- After remediation push, old verdicts are stale by definition and PR agents must re-review.
- Never dispatch more than one remediation agent per PR at the same time.

## Fix 1: implementation-swarm Cooldown

### Problem
`implementation-swarm` is configured with `schedule = "0 * * * *"` but has spawned every 5 minutes when it exits quickly with no work.

### Design
Add `max_ticks = 1` and a 1-hour cooldown if supported by the orchestrator config. If not supported, add a file-based guard at the top of the task script.

```toml
[[agents]]
name = "implementation-swarm"
schedule = "0 * * * *"
max_ticks = 1
grace_period_secs = 3600
```

Fallback guard:

```bash
COOLDOWN_FILE="/tmp/adf-implementation-swarm-cooldown"
COOLDOWN_SECS=3600
if [ -f "$COOLDOWN_FILE" ]; then
  LAST=$(stat -c %Y "$COOLDOWN_FILE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  if [ $((NOW - LAST)) -lt "$COOLDOWN_SECS" ]; then
    echo "implementation-swarm: cooldown active"
    exit 0
  fi
fi
touch "$COOLDOWN_FILE"
```

## Fix 2: Branch Protection Alignment

### Design
For each `terraphim/*` repository, configure required contexts that the repo actually emits. For `terraphim-ai`, use:

```json
{
  "branch_name": "main",
  "enable_status_check": true,
  "status_check_contexts": [
    "native-ci / build (push)",
    "adf/pr-reviewer",
    "adf/validation",
    "adf/verification"
  ]
}
```

For sub-projects, verify emitted native CI context first, then apply the same contexts only if ADF PR agents cover that project via `extra_projects`.

## Fix 3: Duplicate Issue Deduplication

### Design
Auto-merge and review failure issue creation must be idempotent. Dedup key:

```text
ADF-Failure-Key: <owner>/<repo>#<pr>:<head_sha>:<failure_kind>
```

If an open issue with that key exists, comment with recurrence details instead of creating a new issue. Batch-close existing duplicate `[ADF] Auto-merge failed` issues after the new dedup logic is deployed.

## Fix 4: Native Runner Hygiene

### Current State
`terraphim-gitea-runner` is running and doing real work. Evidence from journal logs:
- `terraphim-agents@d1271f24...` completed `native-ci` in 45,241ms.
- `terraphim-ai@259f0dfc...` completed `native-ci` in 164,530ms.

Actions run #17864 showed 0s because it was cancelled, not because the successful native runner path skips cargo execution.

### Design
- Keep native runner as the only build path.
- Kill duplicate stale runner processes after confirming they have no active job.
- Keep ADF bash build-runner agents disabled.
- Monitor runner journal for `Executing step` and `Workflow 'native-ci' completed successfully`.

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/reconcile_impl.rs` | Read drain output, classify captured text, post parseable verdict comments/statuses |
| `crates/terraphim_orchestrator/src/pr_review.rs` | Reuse parser; add tests for orchestrator-posted footer and finding fingerprints |
| `crates/terraphim_orchestrator/src/auto_merge_impl.rs` | Add dedup keys, remediation state transitions, stop-condition enforcement |
| `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | Ensure PR agents and remediation dispatch use current head SHA and branch metadata |
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | Add `implementation-swarm` cooldown config or task guard |

### No-Change Files

| File | Reason |
|------|--------|
| 6 sub-project `conf.d/*.toml` | ADF bash build-runners stay disabled |
| `.gitea/workflows/native-ci.yml` | Native CI is already present and running |
| `orchestrator.toml` | No scheduler core change needed |

## Test Strategy

### Unit Tests

| Test | Purpose |
|------|---------|
| `parse_orchestrator_posted_verdict_footer` | Ensures posted comments remain parseable |
| `finding_fingerprint_stable_for_same_finding` | Enables no-progress stop condition |
| `dedup_key_includes_repo_pr_head_failure_kind` | Prevents duplicate issue spam |
| `remediation_stops_after_three_attempts` | Enforces hard loop budget |
| `unparseable_verdict_pauses_same_head_retry` | Prevents repeated failed review loops |

### Integration Tests

| Test | Purpose |
|------|---------|
| Review output -> verdict comment -> parse -> status | Proves #2301 fix |
| Findings -> remediation dispatch -> new head -> re-review | Proves closed-loop behaviour |
| Same finding twice -> escalation | Proves no-progress stop condition |
| Native CI failure -> no repeated review spam | Proves CI failure stop condition |
| Clean verdicts + native CI success -> auto-merge | Proves end-to-end success |

### Live Verification

Use a low-risk PR similar to #2268:

1. Confirm `native-ci / build (push)` succeeds with real journal evidence.
2. Confirm `adf/pr-reviewer`, `adf/validation`, `adf/verification` statuses are posted for the current head SHA.
3. Confirm each ADF status has a parseable verdict comment containing `Last reviewed commit: <head>`.
4. If findings are present, confirm exactly one remediation dispatch occurs.
5. Confirm re-review occurs after the remediation push.
6. Confirm the loop stops cleanly on success or escalates once after stop condition.

## Implementation Steps

### Step 0: Implement verdict-comment posting (#2301)

**Files:** `reconcile_impl.rs`, `pr_review.rs`, possibly `output_poster.rs`

**Description:** After PR agents finish, orchestrator reads drain logs and posts parseable verdict comments. Status reflects parsed verdict, not exit code.

**Verification:** Integration test proves captured output becomes parseable Gitea comment.

### Step 1: Implement bounded remediation loop

**Files:** `auto_merge_impl.rs`, `pr_handlers_impl.rs`

**Description:** Add remediation state transitions and stop conditions. Prevent concurrent remediation on same PR. Stop after clean gates, 3 attempts, no-progress fingerprint, unparseable verdict, hard P0, repeated CI failure, or merge conflict.

**Verification:** Tests cover all stop conditions.

### Step 2: Stop implementation-swarm hot-loop

**Files:** `/opt/ai-dark-factory/conf.d/terraphim.toml`

**Description:** Add supported cooldown config or task-level file guard.

**Verification:** 15-minute observation shows 0-1 spawn, not 3+.

### Step 3: Align branch protection

**Files:** None; Gitea API.

**Description:** Ensure required contexts match emitted contexts per repo.

**Verification:** Branch protection API returns expected contexts.

### Step 4: Native runner hygiene

**Files:** Operational only.

**Description:** Keep one active native runner per intended repo set; kill stale duplicates after checking for active jobs.

**Verification:** Journal shows real native-ci steps and no runner contention.

### Step 5: Deduplicate and clean historic issues

**Files:** None; Gitea API.

**Description:** Batch-close duplicate auto-merge failure issues with a comment pointing to the canonical dedup issue.

**Verification:** No new duplicate failure issues over two reconcile cycles.

### Step 6: End-to-end live proof

**Description:** Use a PR like #2268 to prove native CI + ADF review + remediation loop + re-review + merge eligibility.

**Stop Condition for Proof:** If the proof hits any stop condition, do not manually force it through. Record the stop reason and create/update one canonical issue.

## Rollback Plan

1. Disable new remediation dispatch while keeping verdict posting if verdict posting is stable.
2. Revert implementation-swarm cooldown only if it prevents legitimate hourly work.
3. Revert branch protection context changes if they block all merges due to missing statuses.
4. Do not re-enable ADF bash build-runner as rollback; native runner remains the build path.

## Approval

- [ ] Research corrections accepted
- [ ] Stop conditions accepted
- [ ] Test strategy accepted
- [ ] Human approval received before implementation
