# Disciplined Research: Current PR and Issue Portfolio

- Date: 2026-02-13
- Repository: terraphim/terraphim-ai
- Inputs reviewed: open PRs, open issues, architecture epics/issues (#520-#526), active feature issue (#519), and referenced architecture docs
- Method: portfolio triage focused on dependency, sequencing, and delivery risk

## 1. Problem Statement

The repository currently has a large mixed backlog of open pull requests and issues spanning architecture refactor phases, feature work (TinyClaw), and dependency update churn. Without an explicit integration plan, there is high risk of merge conflicts, context switching, and stalled architecture outcomes.

## 2. Current-State Inventory (as of 2026-02-13)

### Pull Requests
- Open PRs: 26
- Human-authored PRs: 8
- Dependabot PRs: 18
- PRs updated within 7 days: 9
- PRs stale over 30 days: 0

### Issues
- Open issues: 85
- Architecture-labeled open issues: 12
- Issues updated within 7 days: 8
- Issues stale over 60 days: 59
- Bug-labeled open issues: 2

### High-Signal Active Workstreams
1. Architecture modernization epic: #520 with phased breakdown #521-#526
2. TinyClaw implementation: #519, paired with active PR #527
3. Integration testing for agent mode consistency: PR #516
4. Continuous dependency update stream (18 Dependabot PRs)

## 3. Key Findings

### F1. Architecture sequencing is defined but not yet execution-hardened
Issue #520 provides clear phase order (P0..P4) and measurable acceptance criteria. This is positive, but no evidence yet that all guardrails are active in CI (notably policy enforcement from #522 and baseline measurement from #521).

### F2. Active feature branch has merge friction
PR #527 (`claude/tinyclaw-terraphim-plan-lIt3V`) is currently in `DIRTY` merge state, signaling rebase/merge-conflict pressure if architecture phases land first or in parallel without coordination.

### F3. Integration testing PR exists but appears disconnected from architecture lane
PR #516 adds meaningful cross-mode tests, but there is no explicit visible linkage to architecture phase acceptance gates (#523/#524) despite overlapping concerns (route parity, mode consistency, service decomposition).

### F4. Portfolio mix creates throughput drag
A high ratio of bot PRs (18/26) competes with human feature/architecture work for review bandwidth. Unbatched dependency updates increase noise and can mask high-value structural changes.

### F5. Backlog aging suggests weak closure loop
59 issues are stale for over 60 days. This creates planning ambiguity and raises the chance of duplicated effort or reviving outdated direction.

## 4. Dependency and Constraint Map

### Confirmed Dependencies
1. #521 (ADRs/baseline) and #522 (CI dependency policy) are prerequisites for reliable measurement of #523-#526 outcomes.
2. #523 (bootstrap + route unification) should precede #524 (service decomposition) to reduce refactor churn and test drift.
3. #525 (dependency isolation) impacts build profiles and should gate broad rollout of new runtime-heavy features from #519.
4. #526 (CLI product-line consolidation) should incorporate outcomes from #516 (cross-mode consistency tests) as regression contracts.

### Operational Constraints
- Any plan must preserve release flow while reducing dependency drift.
- Existing docs (`docs/architecture-review-report.md`, `docs/architecture-improvement-plan.md`) already define expected architecture direction; new work should align, not fork.
- Merge conflict risk is elevated for PR #527 unless branch synchronization cadence is enforced.

## 5. Risk Register

1. Risk: Architecture phase slippage due to parallel feature pressure from #519/#527.
- Likelihood: High
- Impact: High
- Mitigation: Freeze large feature merges into architecture-sensitive crates until P0/P1 merge.

2. Risk: CI policy gap allows dependency drift despite issue #522.
- Likelihood: Medium
- Impact: High
- Mitigation: Implement and enforce dependency-policy check before Phase 1+ architecture merges.

3. Risk: Test suite does not track structural architecture acceptance criteria.
- Likelihood: Medium
- Impact: Medium
- Mitigation: Map PR #516 tests to specific phase acceptance criteria and add missing tests where needed.

4. Risk: Review bandwidth dilution from bot PR volume.
- Likelihood: High
- Impact: Medium
- Mitigation: Batch/queue dependency PRs and prioritize architecture/feature-critical PRs first.

5. Risk: Stale issue backlog obscures current priorities.
- Likelihood: High
- Impact: Medium
- Mitigation: Run stale triage pass and close/archive non-actionable legacy items.

## 6. Unknowns Requiring Design Decisions

1. PR integration strategy for #527 relative to #523/#524 (rebase-after-each-phase vs temporary freeze).
2. Dependency PR policy during architecture phases (auto-merge low risk vs deferred batch windows).
3. Acceptance gate definition tying issue milestones to objective CI evidence.
4. Ownership model for architecture phases vs TinyClaw delivery to avoid context fragmentation.

## 7. Research Conclusion

The repository has a workable strategic architecture plan already defined (issues #520-#526 + architecture docs), but execution risk is primarily coordination and sequencing, not missing technical direction. The most leverage comes from enforcing phase prerequisites (P0), reducing merge entropy, and binding tests to phase acceptance criteria.

## 8. Recommended Next Step (Phase 2 Input)

Proceed to disciplined design with a portfolio execution design that defines:
1. Lane-based sequencing (Architecture lane, Feature lane, Dependency lane)
2. PR/issue gating rules by phase
3. Evidence matrix (what CI/test evidence is required for each phase)
4. Week-by-week integration cadence with merge conflict controls

