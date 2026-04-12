# Research Document: PR #790 Cleanup - Decomposition into Atomic PRs

**Status**: Draft
**Author**: AI Research Agent
**Date**: 2026-04-12
**Reviewers**: Alex

## Executive Summary

PR #790 (`fix/autoupdate-gnu-musl-fallback`) contains only **4 commits** ahead of main (the other 24 commits are already merged via PRs #781, #782, #783). However, those 4 commits still mix **5 distinct concerns** across 20 files (+3,620/-183 lines). This research identifies the exact decomposition boundaries and dependency ordering needed to split into clean, atomic PRs.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | A clean git history reduces review burden and CI failures |
| Leverages strengths? | Yes | Atomic PRs map to existing Gitea issues |
| Meets real need? | Yes | Current PR mixes fixes + features + docs, blocking review |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
PR #790 branches from `origin/main` at commit `ea1e915b` and adds 4 commits that interleave tracker fixes, autoupdate fallback, telemetry events, CI workflow fixes, and documentation. The PR title suggests it is only about autoupdate GNU/MUSL fallback, but it contains much more.

### Impact
- Reviewers cannot assess safety of any single change
- CI failures in one concern block merging unrelated fixes
- Rollback of one concern requires reverting all concerns

### Success Criteria
- Each PR touches files from exactly one concern area
- Each PR is independently mergeable
- No PR depends on another PR to compile

## Current State Analysis

### Existing Implementation
The branch `fix/autoupdate-gnu-musl-fallback` has its merge-base at `ea1e915b` (current `origin/main` tip). Only 4 commits are ahead:

| # | Commit | Message | Files |
|---|--------|---------|-------|
| 1 | `7aa18ec4` | fix(tracker): restore Gitea paging and claim verification | 4 files |
| 2 | `3efe03c8` | fix(tracker): resolve Gitea tracker listener regressions | 9 files |
| 3 | `1b22da3c` | docs(agent): add test ranking knowledge graph documentation | 2 files |
| 4 | `21dc532c` | fix(tracker): make listener retries durable and unblock CI | 5 files |

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Autoupdate | `crates/terraphim_update/src/lib.rs` | GNU/MUSL fallback download logic |
| Release CI | `.github/workflows/release-comprehensive.yml` | Cache skip + SHA fallback |
| Tracker | `crates/terraphim_tracker/src/gitea.rs` | Paging, claim verification, issue CRUD |
| Listener | `crates/terraphim_agent/src/listener.rs` | New 1560-line listener module |
| Telemetry Events | `crates/terraphim_orchestrator/src/control_plane/events.rs` | New 697-line event types |
| Orchestrator Wiring | `crates/terraphim_orchestrator/src/{mod,lib,dual_mode,output_poster}.rs` | Minor wiring changes |
| Agent Main | `crates/terraphim_agent/src/main.rs` | Listener registration |
| Agent Cargo | `crates/terraphim_agent/Cargo.toml` | New deps for listener |
| Hooks | `crates/terraphim_hooks/src/validation.rs` | Minor validation fix |
| Symphony | `crates/terraphim_symphony/bin/symphony.rs` | Unknown coupling |
| Benchmarking | `crates/terraphim_validation/src/bin/performance_benchmark.rs` | Schema fix |
| Benchmarking CI | `.github/workflows/performance-benchmarking.yml` | Baseline JSON schema |
| Docs | `crates/terraphim_agent/docs/src/kg/test_ranking_kg.md` | KG test ranking docs |
| Workspace | `Cargo.toml`, `Cargo.lock` | Version/deps updates |

### Data Flow
```
Commits are linear: 7aa18ec4 -> 3efe03c8 -> 1b22da3c -> 21dc532c
Each commit touches multiple concern areas, creating interleaving.
```

## Concern Decomposition

### Commit 1: `7aa18ec4` - fix(tracker): restore Gitea paging and claim verification
| File | Concern |
|------|---------|
| `crates/terraphim_tracker/src/gitea.rs` | **B. Tracker** - paging, claim, CRUD |
| `crates/terraphim_agent/src/listener.rs` | **C. Listener** - new 1560-line module |
| `crates/terraphim_hooks/src/validation.rs` | **F. Misc** - validation fix |
| `crates/terraphim_symphony/bin/symphony.rs` | **F. Misc** - symphony changes |

### Commit 2: `3efe03c8` - fix(tracker): resolve Gitea tracker listener regressions
| File | Concern |
|------|---------|
| `Cargo.lock`, `Cargo.toml` | **D. Workspace** - deps/version bumps |
| `crates/terraphim_agent/Cargo.toml` | **C. Listener** - new deps |
| `crates/terraphim_agent/src/main.rs` | **C. Listener** - registration |
| `crates/terraphim_orchestrator/src/control_plane/mod.rs` | **E. Events** - module registration |
| `crates/terraphim_orchestrator/src/dual_mode.rs` | **E. Events** - minor wiring |
| `crates/terraphim_orchestrator/src/lib.rs` | **E. Events** - minor wiring |
| `crates/terraphim_orchestrator/src/output_poster.rs` | **E. Events** - minor wiring |
| `crates/terraphim_tracker/tests/gitea_create_issue_test.rs` | **B. Tracker** - test fix |
| `crates/terraphim_update/Cargo.toml` | **A. Update** - version bump |

### Commit 3: `1b22da3c` - docs(agent): add test ranking knowledge graph documentation
| File | Concern |
|------|---------|
| `crates/terraphim_agent/docs/src/kg/test_ranking_kg.md` | **G. Docs** |
| `crates/terraphim_orchestrator/src/control_plane/events.rs` | **E. Events** - 697-line new file |

### Commit 4: `21dc532c` - fix(tracker): make listener retries durable and unblock CI
| File | Concern |
|------|---------|
| `.github/workflows/performance-benchmarking.yml` | **F. Misc** - benchmark baseline |
| `.github/workflows/release-comprehensive.yml` | **A. Update** - cache skip + SHA |
| `crates/terraphim_agent/src/listener.rs` | **C. Listener** - retry fixes |
| `crates/terraphim_update/src/lib.rs` | **A. Update** - GNU/MUSL fallback |
| `crates/terraphim_validation/src/bin/performance_benchmark.rs` | **F. Misc** - schema fix |

## Constraints

### Technical Constraints
- Commits are linear, so cherry-picking or interactive rebase is required
- `listener.rs` (1560 lines) is introduced in commit 1 and modified in commit 4
- `events.rs` (697 lines) is introduced in commit 3 but `mod.rs` references it in commit 2
- Workspace `Cargo.lock` changes span commits 2 only

### Dependency Analysis

### Verified Dependency Graph

```
A. Update (terraphim_update)        - standalone
B. Tracker (terraphim_tracker)      - standalone
C. Listener (terraphim_agent)       - depends on B (tracker types) + E (events)
D. Workspace (Cargo.toml/lock)      - depends on C (agent Cargo.toml)
E. Events (orchestrator/events.rs)  - standalone
F. Symphony                         - depends on B (tracker API changes)
G. Hooks validation                 - standalone
H. Benchmark CI + bin               - standalone
I. Docs (test_ranking_kg.md)        - standalone
```

**Key finding**: Listener (C) depends on Events (E), so they MUST be in the same PR.
Symphony (F) depends on Tracker (B), so they should be in the same PR.

### Simplified Merge Groups

| Group | Concerns | Reasoning |
|-------|----------|-----------|
| **PR-1: Tracker + Symphony** | B + F | Symphony adapter wraps new tracker API |
| **PR-2: Update** | A | Standalone GNU/MUSL fallback |
| **PR-3: Events + Listener** | C + E + D | Listener imports from events; Cargo changes follow |
| **PR-4: Misc** | G + H + I | Standalone fixes |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `listener.rs` changes in 2 commits make cherry-pick fragile | High | Medium | Squash listener changes together |
| `events.rs` + `mod.rs` split across commits 2-3 | High | High | Must be in same PR or reorder |
| `Cargo.lock` conflicts between concerns | Medium | Low | Regenerate per branch |
| `symphony.rs` coupling to tracker/listener unknown | Medium | Medium | Verify compilation per PR |

### Open Questions
1. Does `listener.rs` depend on `events.rs`? - Need to check imports
2. Does `symphony.rs` change relate to tracker or is it independent?
3. Are the `terraphim_hooks/validation.rs` changes related to tracker or standalone?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Each concern compiles independently | Modular crate structure | Compilation errors per PR | No |
| ~~listener.rs does not import events.rs~~ | ~~Different crate boundaries~~ | ~~Cross-PR dependency~~ | **DISPROVED** |
| symphony.rs changes are standalone | Unrelated to tracker | Merge conflict | Yes (adapter for tracker API changes) |

### VERIFIED: listener.rs DEPENDS on events.rs

`listener.rs` imports from `terraphim_orchestrator::control_plane`:
- `normalize_polled_command` (from `events.rs`)
- `NormalizedAgentEvent` (from `events.rs`)
- `EventOrigin`, `CommandKind` (from `events.rs`)
- `terraphim_orchestrator::adf_commands::AdfCommandParser`

This means: **Listener + Events must be in the SAME PR**. They cannot be split.

### VERIFIED: symphony.rs depends on tracker API changes

`symphony.rs` introduces a `LinearTrackerAdapter` that wraps `terraphim_tracker::LinearTracker` and maps between tracker types and symphony types. This is a direct dependency on the tracker API changes (specifically `IssueTracker` trait, `LinearConfig`). **Symphony must be in the same PR as tracker or after it.**

### VERIFIED: hooks/validation.rs is standalone

The `validation.rs` change is purely a test timing adjustment (increased latency threshold from 1ms to 5ms, added cache warmup). Completely independent.

## Recommended Split Strategy

### Strategy: Fresh Branches with Squashed Cherry-Picks

Given the interleaved nature of the 4 commits and verified cross-concern dependencies, the cleanest approach is:

1. **Start fresh** from `origin/main` for each PR
2. **Cherry-pick individual file diffs** using `git checkout <commit> -- <paths>`
3. **Squash** into single meaningful commits per concern

### Target PRs (4 PRs, ordered by merge sequence)

#### PR-1: `fix(tracker): restore Gitea paging, claim verification, and symphony adapter`
| Files | From Commit(s) |
|-------|----------------|
| `crates/terraphim_tracker/src/gitea.rs` | 7aa18ec4 |
| `crates/terraphim_tracker/tests/gitea_create_issue_test.rs` | 3efe03c8 |
| `crates/terraphim_symphony/bin/symphony.rs` | 7aa18ec4 |

**Size**: ~1100 lines changed
**Depends on**: Nothing
**Refs**: #791 (Gitea tracker regressions)

#### PR-2: `fix(update): GNU/MUSL fallback for autoupdate and release pipeline`
| Files | From Commit(s) |
|-------|----------------|
| `crates/terraphim_update/src/lib.rs` | 21dc532c |
| `crates/terraphim_update/Cargo.toml` | 3efe03c8 |
| `.github/workflows/release-comprehensive.yml` | 21dc532c |

**Size**: ~200 lines changed
**Depends on**: Nothing
**Refs**: #791 (autoupdate failure)

#### PR-3: `feat(agent): add event-driven listener with durable retries`
| Files | From Commit(s) |
|-------|----------------|
| `crates/terraphim_orchestrator/src/control_plane/events.rs` | 1b22da3c |
| `crates/terraphim_orchestrator/src/control_plane/mod.rs` | 3efe03c8 |
| `crates/terraphim_orchestrator/src/lib.rs` | 3efe03c8 |
| `crates/terraphim_orchestrator/src/dual_mode.rs` | 3efe03c8 |
| `crates/terraphim_orchestrator/src/output_poster.rs` | 3efe03c8 |
| `crates/terraphim_agent/src/listener.rs` | 7aa18ec4 + 21dc532c (squashed) |
| `crates/terraphim_agent/src/main.rs` | 3efe03c8 |
| `crates/terraphim_agent/Cargo.toml` | 3efe03c8 |
| `Cargo.toml` | 3efe03c8 |
| `Cargo.lock` | 3efe03c8 |

**Size**: ~2400 lines changed
**Depends on**: PR-1 (tracker types used by listener)
**Refs**: #523 (telemetry), new issue for listener

#### PR-4: `fix(misc): validation latency, benchmark baseline, KG docs`
| Files | From Commit(s) |
|-------|----------------|
| `crates/terraphim_hooks/src/validation.rs` | 7aa18ec4 |
| `crates/terraphim_validation/src/bin/performance_benchmark.rs` | 21dc532c |
| `.github/workflows/performance-benchmarking.yml` | 21dc532c |
| `crates/terraphim_agent/docs/src/kg/test_ranking_kg.md` | 1b22da3c |

**Size**: ~100 lines changed
**Depends on**: Nothing

### Execution Order

```
PR-1 (tracker+symphony) ─┐
PR-2 (update)            ─┼─> PR-3 (events+listener) ─> PR-4 (misc)
                          │
              (either can go first)
```

## Next Steps

If approved:
1. Proceed to Phase 2 (Design) with exact `git checkout` commands per PR
2. Create Gitea issues for each target PR
3. Execute the split using the design document
