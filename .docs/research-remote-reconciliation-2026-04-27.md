# Research Document: Remote Reconciliation - GitHub/Gitea Sync & Issue Cleanup

**Status**: Updated (4th fetch, 2026-04-27 17:30 CEST)
**Author**: opencode (glm-5.1)
**Date**: 2026-04-27
**Reviewers**: Alex

## Executive Summary

Three-way divergence between local main (`91331d4ee`), origin/main (`faf5e7006`), and gitea/main (`8ab6e3c16`). Origin and gitea are both stable (no movement since last check). Gitea now has 439 open issues (7 new since v3: #1015-#1021, all agent-generated). 26 open PRs (new PR #1021 for spawner fix). 71 stale local branches. Origin has unique content not on local/gitea: v1.17.0 release + flaky test fix. Local has unique content not on origin/gitea: #905 robot search fix + exit codes tests.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Multiple agents stepping on each other blocks all forward progress |
| Leverages strengths? | Yes | We built the Gitea PageRank system specifically for this |
| Meets real need? | Yes | Without reconciliation, every new session starts with confusion |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
Three-way divergence between local main, origin/main, and gitea/main. Autonomous agents continue generating issues and PRs. The core problems are:
1. Different commit SHAs for identical content (rebases/cherry-picks) across remotes
2. 439 open Gitea issues, many stale/duplicate, with 7 more generated since last check
3. 26 open Gitea PRs, many stale
4. 71 merged local branches polluting `git branch`
5. Agents continue generating noise (remediation issues #1017-#1019 for a single #107 issue)

### Impact
- Any new session starts confused about "what's the current state?"
- Agents waste cycles re-discovering already-completed work
- CI is non-functional (#1005), so divergence goes undetected
- Issue noise ratio is extreme: ~50 genuine issues vs ~390 agent noise

### Success Criteria
1. All three main heads converge to a single commit (or known-good state)
2. Stale branches pruned (local + remote)
3. Gitea issues triaged: duplicates closed, stale closed, active prioritised
4. A documented sync protocol for agents going forward

## Current State Analysis

### Git Heads (stable since v2 fetch, 2026-04-27)

| Head | SHA | State |
|------|-----|-------|
| local main | `91331d4ee` | 2 ahead of gitea; 11 behind origin |
| origin/main | `faf5e7006` | 12 ahead of gitea; has v1.17.0 release |
| gitea/main | `8ab6e3c16` | 9 ahead of origin (ADF Phase 2 merge) |

**Stability**: Both origin and gitea/main have NOT moved since the v2 fetch. No new commits on either remote's main branch.

### Merge Bases (unchanged)
- local vs origin: `e2212c396` (#936 spec accuracy)
- local vs gitea: `8ab6e3c16` (gitea/main itself)
- origin vs gitea: `e2212c396` (#936 spec accuracy)

### Content Divergence (unchanged)

**origin/main vs local** (10 files, 442+/18-): Cargo.toml v1.17.0, CHANGELOG, cfg-gated tests, release plan docs, session file

**gitea/main vs local** (6 files, 376-): #905 research/design docs, test KG fixture, exit_codes.rs, auto_route.rs

**origin/main vs gitea/main** (16 files, 818 lines): All of the above combined

### Predicted Merge Conflict
- **1 conflict only**: Cargo.lock (trivial, auto-generated)

### Branch Inventory (unchanged)

| Category | Count |
|----------|-------|
| Stale local branches (merged to main) | 71 |
| New remote branches | 1 (`gitea/task/1020-fix-spawner-task-body`) |

### Open Gitea PRs (26 total, 6 new since v3)

| PR | Branch | Title | Status |
|----|--------|-------|--------|
| #1021 | task/1020-fix-spawner-task-body | fix(orchestrator): spawn agents with TOML task body | **NEW** - Active |
| #1000 | task/672-token-budget-management | Fix #672: wire token budget flags | Active |
| #999 | worktree-agent-ad4db636 | refactor(adf): per-project pr_dispatch | Active |
| #997 | task/987-fix-flaky-performance-test | Fix #987: raise flaky threshold | Superseded by origin commit |
| #969 | task/851-f1-1-concepts-matched | fix(agent): F1.1 populate concepts_matched | Active |
| #958 | task/955-adf-phase-2d-compliance-watchdog | Phase 2d compliance watchdog | Active |
| #956 | worktree-agent-a2799aaa | Phase 2e test guardian | Agent-generated |
| #952 | task/950-pr-spec-validator-phase-2b | Phase 2b spec validator | Active |
| #869 | task/860-f1-2-exit-codes | F1.2 exit code contract | Superseded (cherry-pick value first) |
| #857 | task/851-populate-concepts-matched-wildcard-fallback | Fix #851 concepts_matched | May overlap with #969 |
| #847 | task/838-robot-mode-query-field | fix(agent): query/role in robot JSON | May be merged |
| #830 | task/747-repo-steward-template | Fix #747: repo-steward template | Stale (3+ weeks) |
| #780-#667 | various stale branches | Various old fixes | Stale (5+ days) |
| #660 | task/252-supervisor-tla-fixes | Fix #252 #255: supervisor | **Previously hidden** |
| #655 | task/173-automata-mention-detection | Aho-Corasick mention scanner | **Previously hidden** |
| #640 | task/638-spec-validator-reconcile | Fix #638 spec-validator reconcile | **Previously hidden** |
| #639 | task/638-spec-validator-reconcile | Fix #638 evaluate_rejects_p0 | **Previously hidden** |
| #636 | cherry-pick/fix-ci-zlob | Cherry-pick: fix ci zlob | **Previously hidden** |

### Gitea Issues

**Total**: 1021 issues, 439 open (repo reports 413, likely caching discrepancy)

**New since v3**: 7 issues
| # | Title | Category |
|---|-------|----------|
| #1021 | fix(orchestrator): spawn agents with TOML task body | Genuine bug fix |
| #1020 | bug(orchestrator): spawner runs cli_tool with runtime task_string | Genuine bug report |
| #1019 | [Remediation] security-sentinel FAIL on #107 | Agent noise |
| #1018 | [Remediation] spec-validator FAIL on #107 | Agent noise |
| #1017 | [Remediation] test-guardian FAIL on #107 | Agent noise |
| #1016 | [Repo Stewardship][Usefulness] documentation-debt | Agent noise (dup of #988, #966, #931) |
| #1015 | [Repo Stewardship][Stability] ci-runners-stopped | Agent noise (dup of #1005) |

**Issue flood summary**: Issues #964-#1021 (58 issues), nearly all auto-generated by agents in ~72 hours.

**Updated duplicate groups**:

| Topic | Keep | Close (duplicates) | New additions |
|-------|------|---------------------|---------------|
| Task 1.4 REPL | #994 | #970, #972, #985, #932 | |
| Task 1.5 token budget | #996 | #971, #973, #977, #933, #903 | |
| Task 1.6 tests | #995 | #974, #976, #904 | |
| Robot search JSON | #998 | #975, #980, #981, #992 | |
| Documentation gaps | #988 | #966, #1014, #1016, #931 | +2 new |
| Config drift | #993 | #989 | |
| Security | #1004 | #967, #941 | |
| Flaky tests | #997 | #987, #935, #934 | |
| Zlob/Zig build | #984 | #965 | |
| Fleet health | #1006 | #930 | |
| ADF orchestrator | #963 | #979, #951 | |
| Repo Stewardship | #898 | #1009, #1008, #1007, #960, #948, #947, #946 | |
| CI runners | #1005 | #1015 | +1 new |
| Remediation noise | close all | #1017, #1018, #1019 | +3 new |
| Operational/meta | close all | #983, #982, #1010 | |

**Total to close**: ~55 issues + ~12 stale PRs

## Constraints

### Technical Constraints (unchanged)
1. **Safety guard blocks force-push**: Cannot `git push --force` to any remote
2. **Pre-commit hooks are heavy**: Every commit triggers clippy + build + test + UBS
3. **CI is down**: #1005 - all 5 GitHub Actions runners stopped
4. **Protected branches**: Gitea has branch protection on main
5. **Only 1 merge conflict predicted**: Cargo.lock (trivial)

### Process Constraints (unchanged)
1. **Must push to both remotes**: Project policy requires origin + gitea sync
2. **Agents push concurrently**: Multiple autonomous agents working simultaneously
3. **No `tea` CLI**: Use gitea-robot MCP tools or curl API for Gitea interactions

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Main branch convergence | Without it, every session starts confused | 3-way divergence, stable but unresolved |
| Duplicate issue cleanup | 439 open issues makes triage impossible | 58 agent-generated issues in 72 hours |
| Stale branch/PR cleanup | 26 PRs + 71 dead branches obscure active work | PRs from weeks ago still open |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Fixing CI (#1005) | Infrastructure access required |
| ADF agent workflow redesign | Orthogonal to reconciliation |
| Security findings (#1004, #967) | Separate concern |
| Spawner bug (#1020, #1021) | Active, separate from reconciliation |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Agent pushes during reconciliation | High | High | Reconcile quickly |
| Cargo.lock merge conflict | High | Low | Auto-resolve |
| Agents generate more issues during cleanup | High | Low | Batch-close faster than they generate |
| Closing wrong issues as duplicates | Medium | Low | Verify per group |
| Stale PRs have unmerged useful work | Medium | Medium | Review each PR diff |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Origin is most advanced | Has v1.17.0 + cfg-gated tests | We lose content | Yes |
| Local #905/#892 already on origin in different SHAs | Same logical changes, different history | We lose bug fixes | Partially |
| Most issues >#964 are agent noise | Titles follow agent patterns | We close genuine issue | Per-group review |
| Remediation issues (#1017-1019) are noise | 3 agents reporting FAIL on same #107 | We miss real bug | Low risk |

## Research Findings

### Key Insights

1. **Both remotes are stable** -- no main branch movement since v2 fetch. Good window for reconciliation.

2. **Agent noise is accelerating**: 7 new issues in the time since v3 check. Issues #1017-#1019 are three separate agents reporting FAIL on the same #107 -- classic agent noise pattern.

3. **Genuine new issues exist**: #1020 and #1021 (spawner task body bug) appear to be real bugs with an active PR. Keep these.

4. **26 open PRs** (up from 20 in v3, but some were previously hidden by pagination). At least 12 are stale (>5 days).

5. **The reconciliation window is good**: No remote movement means no new divergence to worry about.

### Recommended Strategy (unchanged from v3)

**Phase A**: Cherry-pick from task/860 (Step 0), merge origin into local, push to both
**Phase B**: Delete 71 stale local branches
**Phase C**: Close ~12 stale PRs, keep ~14 active
**Phase D**: Close ~55 duplicate/stale issues
**Phase E**: Update AGENTS.md

## Appendix

### Three-Way Divergence Map (unchanged)

```
e2212c396 (merge base - #936 spec accuracy)
    |
    +-- origin/main (faf5e7006) -- 12 commits, includes v1.17.0
    +-- gitea/main (8ab6e3c16) -- 9 commits, ADF Phase 2 merge
    +-- local main (91331d4ee) -- gitea + 2 commits (#905 fix + KG fixture)
```

### Updated Duplicate Issue Groups

**Group A: Already Completed** (5): #932, #933, #926, #927, #903
**Group B: Task 1.4 REPL** (3): #970, #972, #985
**Group C: Task 1.5 Token Budget** (4): #971, #973, #977, #903
**Group D: Task 1.6 Tests** (3): #974, #976, #904
**Group E: Robot JSON Contract** (4): #975, #980, #981, #992
**Group F: Documentation Gaps** (4): #966, #1014, #1016, #931
**Group G: Config Drift** (1): #989
**Group H: Security** (2): #967, #941
**Group I: Flaky Tests** (3): #987, #935, #934
**Group J: Zlob/Zig** (1): #965
**Group K: Fleet Health** (1): #930
**Group L: ADF Orch** (2): #979, #951
**Group M: Repo Stewardship** (8): #1009, #1008, #1007, #960, #948, #947, #946, #993
**Group N: CI Runners** (1): #1015
**Group O: Remediation Noise** (3): #1017, #1018, #1019
**Group P: Operational/Meta** (3): #983, #982, #1010

**Total**: ~55 issues to close

### Branch Analysis: task/860-f1-2-exit-codes (PR #869)

**Status**: 1791 commits ahead, 38 merge conflicts, `mergeable: false`

**Unique value to cherry-pick**:
1. `exit_codes_integration_test.rs` -- typed exit code contract tests
2. Improved `exit_codes.rs` -- better doc comments, `search_succeeds_exits_0` test
3. 17 doc comment additions on public types in `main.rs`
4. `listen --identity` made `Option<String>`
5. `classify_error` auth heuristic tightening

**Recommendation**: Cherry-pick select commits (Step 0), don't attempt full merge.
