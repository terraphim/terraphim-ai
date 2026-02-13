# Implementation Plan: Post-#527 Portfolio Execution (Overall)

**Status**: Draft  
**Research Doc**: `docs/plans/pr-issues-research-2026-02-13.md`  
**Author**: Codex  
**Date**: 2026-02-13  
**Estimated Effort**: 4-6 working days

## Overview

### Summary
This plan defines the overall execution sequence after `PR #527` was merged on 2026-02-13. It prioritizes finishing currently active human PRs (`#516`, `#492`, `#489`), then advancing architecture phases (`#521` -> `#526`), while controlling dependency PR noise.

### Approach
Operate in three lanes with explicit gates:
1. `Stabilization Lane`: land active human PRs in deterministic order.
2. `Architecture Lane`: execute issue chain `#521` -> `#526` with evidence gates.
3. `Dependency Lane`: batch Dependabot PRs in scheduled windows only.

### Scope
**In Scope:**
- Merge-order and gate design for open human PRs after `#527`
- Architecture issue execution order and entry/exit criteria
- CI evidence matrix and command-level verification strategy
- PR/issue operational workflow (`gh`-driven updates and decisions)

**Out of Scope:**
- Implementing architecture refactors
- Broad stale-backlog closure campaign
- New product feature design unrelated to current open PRs/issues

**Avoid At All Cost** (5/25 discipline):
- Parallel refactors across `terraphim_service`, `terraphim_server`, and `terraphim_agent`
- Merging `#489` without narrowing/splitting mixed scope
- Opportunistic dependency merges during human PR stabilization
- Re-introducing CI workflow churn during merge-critical window
- Adding new architecture epics before `#521`/`#522` are complete
- Rewriting merged `#527` outcomes in same cycle
- Expanding acceptance criteria mid-flight
- Mixing branch-protection policy changes with feature merges
- Running full workspace redesign while unresolved conflicts exist
- Deferring gate evidence to "post-merge cleanup"
- Treating queued CI runs as equivalent to passing CI
- Approving PRs with unresolved merge conflicts
- Bundling docs-only and runtime-heavy code in one PR without justification
- Ignoring issue-to-PR traceability on architecture phases
- Assuming non-blocking checks are safe without policy confirmation
- Adding dependency upgrades to unblock unrelated CI failures
- Allowing bot PRs to consume reviewer capacity during phase gates
- Using heroic manual coordination instead of explicit checklist gates
- Delaying rollback strategy definition
- Introducing unrequested optimization work

## Architecture

### Component Diagram
```text
[Open PR/Issue Inventory]
          |
          v
[Gate Controller]
  - mergeability
  - required checks
  - scope hygiene
  - phase prerequisites
          |
          +--> [Stabilization Lane: #516 -> #492 -> #489(split/fix)]
          |
          +--> [Architecture Lane: #521 -> #522 -> #523 -> #524 -> #525 -> #526]
          |
          +--> [Dependency Lane: batched dependabot windows]
                          |
                          v
                  [Evidence + Issue Updates]
```

### Data Flow
```text
GitHub state -> gate evaluation -> remediation action -> CI evidence -> merge/advance decision
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Start from post-merge `#527` baseline | Reflects current repository truth | Re-planning as if `#527` still open |
| Merge order `#516` then `#492` then `#489` | `#516`/`#492` are mergeable; `#489` is conflicting and scope-mixed | Attempting `#489` first |
| Keep architecture phases strictly sequential (`#521`..`#526`) | Existing epic structure already defines dependency chain | Parallel multi-phase execution |
| Treat `#489` as split-first candidate | Reduces conflict and review ambiguity | Forcing a large mixed-scope merge |
| Defer dependency PR wave until stabilization complete | Protects review and CI bandwidth | Continuous background merges |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Immediate all-PR merge blitz | Violates deterministic gating | High regression/conflict risk |
| Disable checks by deleting workflows | Mixes governance with delivery | Hidden quality regressions |
| Architecture + dependency + feature work simultaneously | Too many moving parts | Low signal, high churn |
| Massive stale-issue cleanup now | Not critical path | Execution drift |
| Broad codebase redesign while PRs pending | Not requested and speculative | Timeline slip |

### Simplicity Check
> Minimum code/process needed to move portfolio safely: merge ready PRs, split/conflicting PRs, then run architecture phases in order.

**What if this could be easy?**  
Use one checklist gate per PR/phase, no concurrent major streams, and hard stop on unresolved conflicts/failing required checks.

**Senior Engineer Test**: Pass. This is intentionally procedural, explicit, and non-speculative.

**Nothing Speculative Checklist**:
- [x] No features not requested
- [x] No "future-proof" abstractions
- [x] No optional flexibility layers
- [x] No impossible-case branches
- [x] No premature optimization

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `docs/plans/pr-issues-overall-design-2026-02-13.md` | Phase 2 overall plan from post-`#527` baseline |

### Modified Files
| File | Changes |
|------|---------|
| None | Design-only artifact creation |

### Deleted Files
| File | Reason |
|------|--------|
| None | N/A |

## API Design (Process Contracts)

### Contract: PR Gate
```text
Input:
- pr_number
- mergeability + merge state
- required check states
- changed-file scope summary

Output:
- READY | BLOCKED
- blocker list (owner + action + ETA)
```

### Contract: Phase Gate
```text
Input:
- issue number (#521..#526)
- prior phase evidence
- CI/lint/test evidence bundle

Output:
- ADVANCE | HOLD
- missing-evidence checklist
```

### Error States
```text
BLOCKED_CONFLICT: mergeable=CONFLICTING
BLOCKED_REQUIRED_CHECKS: any required check not successful
BLOCKED_SCOPE_MIX: PR combines unrelated docs/runtime changes requiring split
BLOCKED_PHASE_PREREQ: previous architecture phase incomplete
```

## Test Strategy

### Unit-Level (Gate Checks via Commands)
| Test | Command | Purpose |
|------|---------|---------|
| PR mergeability | `gh pr view <n> --json mergeable,mergeStateStatus` | Validate merge gate |
| Required checks | `gh pr checks <n>` | Validate check gate |
| Scope verification | `gh pr view <n> --json files` | Detect mixed/unbounded scope |

### Integration-Level (Workflow Evidence)
| Test | Evidence | Purpose |
|------|----------|---------|
| Stabilization sequence run | PR timeline + merge commits | Ensure order `#516 -> #492 -> #489` |
| Architecture gate transitions | Issue comments + linked PRs | Verify `#521..#526` sequencing |
| Dependency isolation | Open PR snapshot before/after windows | Confirm controlled batching |

### Verification Commands by Stage
```bash
# PR stabilization checks
gh pr view 516 --json mergeable,mergeStateStatus,statusCheckRollup
gh pr view 492 --json mergeable,mergeStateStatus,statusCheckRollup
gh pr view 489 --json mergeable,mergeStateStatus,files

# Architecture progression checks
gh issue view 521
gh issue view 522
gh issue view 523
gh issue view 524
gh issue view 525
gh issue view 526

# Local quality gates when code changes are made in implementation phase
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
ubs $(git diff --name-only)
```

## Implementation Steps

### Step 1: Close Stabilization PR `#516`
**Description:** Monitor required checks; merge immediately when green.  
**Exit Criteria:** `#516` merged into `main`.

### Step 2: Close Stabilization PR `#492`
**Description:** Keep branch synchronized with `main`, address only required-check failures, merge when green.  
**Exit Criteria:** `#492` merged into `main`.

### Step 3: Normalize PR `#489` (Retitle + Scope Disclosure)
**Description:** Keep single PR, retitle to match actual scope, and require an `Actual Scope` section in PR description listing changed top-level directories before resolving conflicts.  
**Exit Criteria:** accurately titled PR with explicit scope disclosure, mergeable and reviewable.

### Step 4: Execute Architecture P0 Gates (`#521`, `#522`)
**Description:** produce/verify ADR baseline and CI dependency-policy guard before deeper refactors.  
**Exit Criteria:** `#521` and `#522` accepted with objective CI/doc evidence.

### Step 5: Execute Architecture P1-P4 (`#523` -> `#526`)
**Description:** progress phase-by-phase with no skipping and no concurrent phase execution.  
**Exit Criteria:** each phase issue has linked implementation evidence and closure criteria met.

### Step 6: Run Dependency Batch Window
**Description:** process low-risk Dependabot PRs in a controlled window after human/architecture critical path.  
**Exit Criteria:** dependency queue reduced without disrupting architecture lane.

## Rollback Plan

1. If a stabilization PR fails post-merge checks, revert merge commit and reopen targeted fix PR.
2. If `#489` cannot be normalized quickly, park it and continue with architecture P0/P1 to avoid blocking portfolio flow.
3. If architecture phase evidence is insufficient, hold phase advancement and reopen prior-phase remediation.

## Dependencies

### New Dependencies
None in design phase.

### Dependency Updates
No version decisions in this design phase; handled in dependency batch windows.

## Performance Considerations

### Operational Targets
| Metric | Target | Measurement |
|--------|--------|-------------|
| Human PR cycle time (`#516/#492`) | < 1 business day each after green checks | PR timestamps |
| Conflicting PR count in critical path | 0 | `gh pr list` snapshot |
| Architecture phase overlap | 0 simultaneous active phases | issue timeline |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Implement branch protection update removing required `Warden` check while keeping workflow disabled/removed | Pending execution | Repo admin / maintainer |
| Define exact non-breaking dependency batch cadence (weekly vs biweekly) | Pending | Maintainer |
| Operationalize UBS-driven dependency security triage rule in CI/docs | Pending | Maintainer + implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Specification Interview Findings

**Interview Date**: 2026-02-13  
**Dimensions Covered**: Failure Modes & Recovery; Security & Governance; Integration Effects; Operational Concerns; Migration & Compatibility; Scope Management  
**Convergence Status**: Complete (two rounds, decisions stabilized)

### Key Decisions from Interview

#### Concurrency & Race Conditions
- No additional concurrency-specific policy added for this phase; default existing PR/CI serial merge gates remain.

#### Failure Modes
- Required-check flake handling permits maintainer/repo-admin override when checks are unrelated to changed files.
- Override must be evidence-based, not discretionary.

#### Edge Cases
- PR `#489` remains single-PR but must be retitled to match actual code scope.
- A mandatory `Actual Scope` description section is required to prevent docs/code mismatch ambiguity.

#### User Experience
- Reviewers should see explicit scope transparency in PR body before approval.

#### Scale & Performance
- No new performance constraints introduced during specification interview; existing operational targets remain.

#### Security & Privacy
- Dependency security prioritization will be driven using UBS scanning as policy signal.
- Non-security dependency changes remain planned and non-breaking.

#### Integration Effects
- `Warden` is no longer required as a merge check.
- Policy intent is to remove required status and remove/disable the check from merge gating path.

#### Migration & Compatibility
- Dependency lane allows security patches; non-breaking changes are planned in controlled windows.

#### Accessibility
- Not applicable for this phase (portfolio/PR governance scope only).

#### Operational Readiness
- Override authority includes both repo admins and maintainers.
- Evidence expectation: failing CI run reference plus local quality-gate evidence (`fmt/clippy/test`) before override.

### Deferred Items
- Dependency batch cadence (weekly vs biweekly): deferred pending maintainer scheduling preference.
- Precise UBS-to-policy mapping (what counts as security patch candidate): deferred to CI/doc policy update task.

### Interview Summary
The interview converted previously open governance choices into actionable rules for implementation. The most significant changes are: keep `#489` as one PR with truthful scope labeling, remove `Warden` as a required merge check, and permit evidence-backed overrides for flaky unrelated required checks.

Security/dependency policy was narrowed to allow security patches with UBS as the decision tool, while non-breaking dependency updates remain planned in batches. Remaining ambiguity is operational, not architectural: exact dependency cadence and UBS policy encoding need one execution task before Phase 3 starts.
