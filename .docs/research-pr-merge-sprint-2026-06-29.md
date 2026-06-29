# Research Document: PR Merge Sprint 2026-06-29

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-06-29

## Executive Summary

The terraphim-ai repository has accumulated ~100 open Gitea PRs (#2902-#3034), ~253 ready (unblocked) issues, and 3034 total issues (493 open). Both remotes (origin/gitea) are currently in sync. The `cargo check --workspace` passes on main, but there are minor fmt/clippy warnings (2 fmt files, 3 clippy warnings). The goal is to merge as many PRs as possible and create a fix plan for outstanding issues.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | PR backlog is a daily operational burden; merge sprint directly improves velocity |
| Leverages strengths? | Yes | Automated CI + rust tooling expertise are core capabilities |
| Meets real need? | Yes | 116 open PRs not draining; auto-merge stalled ~2.5d; ADF agents creating duplicate noise |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
The repository has accumulated a massive PR backlog (100+ PRs), with auto-merge severely degraded by CI configuration gaps (missing fmt gate, missing clippy gate, missing compile gate). Several PRs have merge conflicts indicating concurrent work on overlapping code. The ADF orchestrator is generating duplicate issues and PRs compounding the noise.

### Impact
- 116 open PRs not being merged; main only advances via direct push
- Ad-hoc "stash/merge" workflow without proper CI gating
- Agent-generated duplicate issues cluttering the backlog (249/373 ready issues are monitor noise)
- Developer velocity severely degraded

### Success Criteria
1. All mergeable PRs merged to main
2. fmt/clippy/compile gates passing on main
3. Both remotes synchronised
4. Reduction in open PR count from ~100 to <20

## Current State Analysis

### Existing Implementation

**Build Status (main)**:
- `cargo check --workspace`: PASSES (1m 03s)
- `cargo fmt --all -- --check`: FAILS (2 files: `mcp_tools.rs`, `tinyclaw/main.rs`)
- `cargo clippy --workspace`: PASSES with warnings (3 clippy warnings in `mcp_tools.rs`)

**Remote Sync**: origin/main and gitea/main are identical (no divergent commits)

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| CI PR workflow | `.gitea/workflows/ci-pr.yml` | PR merge gate |
| RLM mcp_tools | `crates/terraphim_rlm/src/mcp_tools.rs` | Has clippy + fmt issues |
| Tinyclaw main | `crates/terraphim_tinyclaw/src/main.rs` | Has fmt issue |
| Orchestrator | `crates/terraphim_orchestrator/` | Missing meta_coordinator module |

### PR Categories by Size

**Tier 1 - Quick Merge (<50 lines, single concern, green lights)**:
- #3018: fmt gate (2 files, formatting only)
- #3012: clippy mcp_tools fix (3 lines)
- #2966: remove assert!(true) (1 line)
- #2968: remove dangling meta_coordinator (1 line)
- #2924: remove dead assert (1 line)

**Tier 2 - Merge with Confidence (single concern, tests pass)**:
- #3031: auto-merge gate log enhancement
- #3026: auto-merge agent allowlist
- #3032: Gitea health check fix
- #3002: worktree disk dedup
- #3001: flaky repro profile
- #3000: per-test timeout
- #2993: OnceLock redaction
- #2963: flaky port fix
- #2932: remove 1P vault ref

**Tier 3 - Merge with Review (tests added, moderate changes)**:
- #2985: github_runner tests (49 tests)
- #2976: tinyclaw core tests
- #2962: validation unit tests
- #2957: gitea_runner tests
- #2977: weather_report tests
- #2946: spawner validator tests
- #2926: workspace archive tests
- #2925: RLM strictness tests
- #2903: DSM unit tests

**Tier 4 - Needs Conflict Resolution**:
- #2970: reconcile_tick timeout (mergeable=False)
- #2969: clippy const assertions (mergeable=False)

**Tier 5 - CI Infrastructure** (foundational, merge early):
- #2955: cargo audit CI gate
- #2954: CI rust-clippy + rust-compile jobs
- #2942: workspace test execution gate
- #2939: compile gate
- #2960: host-runners workspace

**Tier 6 - Feature PRs** (requires review):
- #3034: rlm run subcommand (+94 lines)
- #3033: clippy cleanup + 1P tty guard
- #2984: PrFile struct (+deserialization)
- #2979: relocate stranded specs
- #2974: remove orphaned agent source
- #2931: AGENTS.md crate ownership docs

### Newly Fetched Branches (not yet PR'd?)
| Branch | Commits | Status |
|--------|---------|--------|
| `task/docs-stale-spec-annotations-2026-06-25` | 1 commit | Only on gitea, not origin |
| `task/fix-clippy-errors-blocking-ci` | 0 diff vs main | Empty? |
| `task/fix-gitea-runner-poll-timeout` | 0 diff vs main | Empty? |
| `task/kg-driven-runner-allowlist` | 0 diff vs main | Empty? |
| `task/shimaguru-1910-decomposition-restore` | 1 commit | Only on gitea |

## Constraints

### Technical Constraints
- Must not break `cargo check --workspace`
- Must pass `cargo fmt --all -- --check`
- Must pass `cargo clippy --workspace`
- Must not introduce merge conflicts with existing work
- Both remotes (origin + gitea) must stay in sync

### Business Constraints
- PR merges must reference closing issue numbers
- Git history must remain linear (rebase workflow)
- No force-push to either remote

## Vital Few (Essential Constraints)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Fix fmt on main first | Blocks all CI gates | 2 files unformatted, #3018 fixes this |
| Fix clippy on main | Blocks compile gate in CI | 3 warnings in mcp_tools.rs |
| Merge CI gate PRs early | Prevents future regressions | Current lack of gates caused the backlog |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Merge conflicts cascade | Medium | High | Merge oldest PRs first, fmt-fix PRs before others |
| CI gate PRs break existing PRs | Low | High | Test each gate in isolation before merging |
| Auto-merge stalls during sprint | Medium | Medium | Manual merge of Tier 1 PRs |
| Both remotes diverge | Low | High | Push to origin first, verify sync before gitea push |

### Open Questions
1. Why are branches `task/fix-clippy-errors-blocking-ci`, `task/fix-gitea-runner-poll-timeout`, `task/kg-driven-runner-allowlist` showing 0 diff vs main? (Agent committed before pushing?)
2. Should we close stale ADF-generated duplicate issues en masse? (Yes - target ~200 duplicates)

## Recommendations

### Proceed/No-Proceed
**PROCEED** with merge sprint.

### Merge Order (Dependency-Aware)
1. Fix fmt on main (or merge #3018 first, then #3012)
2. Merge CI gate PRs: #2955, #2954, #2942, #2939, #2960
3. Merge Tier 1 quick wins: #2966, #2968, #2924
4. Merge Tier 2 confidence PRs: #3031, #3026, #3032, #3002, etc.
5. Merge Tier 3 test PRs in ascending PR number order
6. Resolve conflicts on #2970, #2969
7. Merge Tier 6 feature PRs
8. Close merge-conflict-failed ADF PRs with comment

## Appendix

### Key Issue Reference
- #3596: Step H post-merge gate reverts (P1 INFRA, 0.0205 PageRank)
- #3648: Alert emitters spawn duplicate issues (P1 INFRA, 0.0244 PageRank - highest)
- #3790: PR-merge pipeline stalled ~2.5d (116 open PRs)
- #4007: MSRV mismatch (rust-version vs clippy.toml, 0.0103 PageRank)
- #4310: clippy -D warnings fails on mcp_tools.rs
