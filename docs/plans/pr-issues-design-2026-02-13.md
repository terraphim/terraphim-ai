# Implementation Plan: PR/Issue Execution with PR #527 Merged First

**Status**: Draft
**Research Doc**: `docs/plans/pr-issues-research-2026-02-13.md`
**Author**: Codex
**Date**: 2026-02-13
**Estimated Effort**: 5-7 working days (planning + execution)

## Overview

### Summary
This plan defines a merge-and-delivery sequence that starts by merging PR #527 first, then resumes architecture phases (#521-#526) with explicit gates. The objective is to reduce branch conflict risk, restore CI signal quality, and preserve architecture roadmap momentum.

### Approach
Use three controlled lanes with hard gates:
1. `Feature Lane` (immediate): unblock and merge PR #527 first.
2. `Architecture Lane` (next): execute P0/P1/P2/P3/P4 in order (`#521` -> `#526`).
3. `Dependency Lane` (background): batch dependabot updates in scheduled windows, not continuously.

### Scope
**In Scope:**
- Merge-first execution plan for PR #527
- Gate definitions (entry/exit criteria) for post-#527 architecture phases
- File-level change plan for process/docs/CI artifacts
- Test and evidence strategy per phase
- Issue/PR update protocol using `gh`

**Out of Scope:**
- Implementing code refactors in server/service crates
- Rewriting TinyClaw design
- Closing historical stale issues en masse in this same execution window

**Avoid At All Cost** (5/25 discipline):
- Parallel deep refactors in `terraphim_server` and `terraphim_service` before #527 lands
- Merging dependency PRs opportunistically during #527 stabilization
- Introducing new architecture epics before #520 phase chain advances
- Coupling #527 merge criteria to unrelated stale backlog cleanup
- Running broad “big-bang” CI matrix changes during merge-critical window

## Architecture

### Component Diagram
```
[PR #527 Readiness]
      |
      v
[Merge Gate Controller]
  - mergeability
  - required checks
  - branch sync
  - scope hygiene
      |
      +--> if pass --> [Merge #527]
      |
      +--> if fail --> [Targeted remediation loop]

After merge #527:
[Phase Gate Controller]
  P0 (#521/#522) -> P1 (#523) -> P2 (#524) -> P3 (#525) -> P4 (#526)
      |
      v
[Evidence Store in docs/ + PR notes + issue updates]
```

### Data Flow
```
Issue/PR state -> Gate evaluation -> Required actions -> CI evidence -> Merge/advance decision
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Merge #527 first | User-directed priority; reduces long-lived branch drift | Deferring #527 behind architecture P0/P1 |
| Gate-based readiness checklist for #527 | Prevents subjective merge calls and rework | Ad-hoc merge judgment |
| Freeze architecture-sensitive merges while stabilizing #527 | Minimizes conflicts and CI noise | Continue normal parallel merges |
| Batch dependency PR handling | Protects review bandwidth and CI clarity | Continuous unbatched dependabot merges |
| Sequential architecture phases after #527 | Aligns with existing #520 contract and acceptance criteria | Multi-phase concurrent execution |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Start Phase 0 architecture before #527 merge | Conflicts with explicit user priority | Higher merge conflict probability |
| Re-scope #527 to include architecture cleanup | Scope creep | Delayed merge and unclear ownership |
| Add new CI pipelines immediately | Not required for first merge objective | Pipeline instability during critical merge |
| Mass close stale issues now | Not on critical path to #527 merge | Loss of focus and review churn |
| Full dependency upgrade wave now | Competes with merge-critical validation | Breakage surface expansion |

### Simplicity Check
**What if this could be easy?**
Easy path: treat #527 as a bounded merge candidate, fix only blockers that affect mergeability/checks/scope, merge it, then execute architecture phases using already-defined issue contracts.

**Senior Engineer Test:** passes; the plan is intentionally procedural and minimizes concurrent moving parts.

**Nothing Speculative Checklist**:
- [x] No features beyond requested planning scope
- [x] No speculative abstractions
- [x] No "just in case" flexibility layers
- [x] No impossible-case error branches
- [x] No premature optimization tasks

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `docs/plans/pr-issues-design-2026-02-13.md` | Phase 2 design plan for merge-first execution |

### Modified Files
| File | Changes |
|------|---------|
| `docs/plans/pr-issues-research-2026-02-13.md` | No changes required in design phase |

### Deleted Files
| File | Reason |
|------|--------|
| None | N/A |

## API/Interface Design (Process-Level)

### Gate Contract: PR #527 Merge Readiness
```text
Input:
- PR metadata (merge state, changed files, reviews)
- CI check statuses
- branch divergence state

Output:
- READY_TO_MERGE | BLOCKED
- Blocker list with owner + remediation action
```

### Gate Contract: Architecture Phase Advancement
```text
Input:
- Prior phase completion evidence
- CI evidence bundle
- dependency diff/compile-time notes (where applicable)

Output:
- ADVANCE_TO_NEXT_PHASE | HOLD
- Missing evidence checklist
```

### Error States
```text
BLOCKED_MERGEABILITY: PR merge state DIRTY or CONFLICTING
BLOCKED_CI: required checks failing
BLOCKED_SCOPE: PR contains non-target artifacts requiring split/revert strategy
BLOCKED_PHASE_EVIDENCE: missing ADR/baseline/check artifacts for phase gate
```

## Test Strategy

### Unit-Level (Process Validation via Commands)
| Test | Command | Purpose |
|------|---------|---------|
| PR merge state check | `gh pr view 527 --json mergeStateStatus` | Ensure clean mergeability |
| CI status check | `gh pr checks 527` | Ensure required checks pass |
| PR scope check | `gh pr view 527 --json files` | Validate file set is intentional |

### Integration-Level (Workflow Validation)
| Test | Evidence | Purpose |
|------|----------|---------|
| Merge-first workflow dry run | Checklist completion log | Verify end-to-end #527 readiness path |
| Phase gate transition P0->P1 | Issue update + CI artifacts | Verify architecture gating discipline |
| Dependency lane isolation | PR queue snapshot | Verify bot PRs do not interrupt critical lane |

### Verification Commands by Stage
```bash
# Stage A: #527 merge readiness
gh pr view 527 --json mergeStateStatus,statusCheckRollup,files
gh pr checks 527
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
ubs $(git diff --name-only)

# Stage B: post-merge architecture phase gates
gh issue view 521
gh issue view 522
cargo tree -d --workspace -e no-dev
```

## Implementation Steps

### Step 1: #527 Triage and Scope Hygiene
**Targets:** PR #527 metadata + file list
**Description:** classify blockers into mergeability, CI, and scope categories; decide exact remediation actions.
**Exit Criteria:** blocker list with clear owner/action; no ambiguous blocker remains.

### Step 2: #527 Branch Synchronization
**Targets:** branch `claude/tinyclaw-terraphim-plan-lIt3V`
**Description:** rebase/merge latest `main`, resolve DIRTY state, re-run required checks.
**Exit Criteria:** `mergeStateStatus=CLEAN` (or equivalent mergeable), checks pending/pass.

### Step 3: #527 CI Stabilization
**Targets:** failing workflows (`Quick Rust Validation`, `Warden review`)
**Description:** fix only failures directly preventing merge; avoid opportunistic refactors.
**Exit Criteria:** required checks green; no new failing required checks.

### Step 4: #527 Review and Merge
**Targets:** PR review state + merge action
**Description:** request review, address comments, merge with conventional commit strategy.
**Exit Criteria:** PR #527 merged into `main`; issue #519 updated with merge note and follow-up list.

### Step 5: Resume Architecture Phases in Order
**Targets:** #521 -> #522 -> #523 -> #524 -> #525 -> #526
**Description:** execute each phase with explicit gate evidence before opening next phase PR.
**Exit Criteria:** each issue has linked PR/evidence; no phase skipping.

### Step 6: Dependency PR Windowing
**Targets:** dependabot PR queue
**Description:** batch low-risk dependency updates into planned windows after architecture gate checkpoints.
**Exit Criteria:** reduced review noise and preserved critical-lane CI signal.

## Acceptance Criteria

1. PR #527 merges first, before architecture phase implementation PRs.
2. #527 merge occurs with passing required checks and resolved mergeability.
3. Post-#527 architecture execution follows #520 phase order without skips.
4. Each phase has attached objective evidence (CI + dependency/compile notes where relevant).
5. Dependency updates are processed in controlled batches, not continuously during critical merges.

## Execution Checklist (Operator View)

- [ ] Confirm #527 blocker list (mergeability/CI/scope)
- [ ] Resolve DIRTY merge state on #527
- [ ] Pass required #527 checks
- [ ] Merge #527
- [ ] Update #519 with merge summary + residual tasks
- [ ] Start #521 ADR/baseline deliverables
- [ ] Complete #522 CI dependency guard
- [ ] Advance #523, #524, #525, #526 in sequence
