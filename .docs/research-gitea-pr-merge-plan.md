# Research Document: Gitea PR Merge Plan

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-04-05
**Reviewers**: Human

## Executive Summary

13 open PRs exist on Gitea (`git.terraphim.cloud/terraphim/terraphim-ai`). Analysis reveals: 1 PR already merged (#157), 3 PRs are duplicates/superseded (#345, #349, #327), 4 PRs are mergeable with minor conflicts, and 5 PRs need rebase before merging. The merge order matters significantly because multiple PRs touch overlapping files (Cargo.toml, Cargo.lock, terraphim_automata, terraphim_orchestrator).

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Clearing PR backlog unblocks all future development |
| Leverages strengths? | Yes | Dependency analysis is a structured engineering task |
| Meets real need? | Yes | 13 open PRs block development velocity; agent branches are accumulating |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
13 open Gitea PRs need to be evaluated, deduplicated, conflict-resolved, and merged into `main` in a safe, sequenced order. Several PRs share commits or overlap in touched files, creating merge-order dependencies.

### Impact
- PRs accumulate and diverge from main, making future merges harder
- Agent-generated branches create noise (`.opencode/reviews/`, empty `curl` files, stale reports)
- Security fix (#353, CVE) is blocked by conflicts

### Success Criteria
1. All mergeable PRs merged into `main`
2. Duplicate/superseded PRs closed with explanation
3. `cargo build --workspace` and `cargo clippy` pass after each merge
4. `main` branch pushed to both `origin` (GitHub) and `gitea` remotes

## Current State Analysis

### PR Inventory (13 total)

| PR # | Title | Commits | Mergeable? | Action |
|------|-------|---------|------------|--------|
| #353 | Fix #341: rustls-webpki CVE (RUSTSEC-2026-0049) | 8 | No | REBASE & MERGE (security critical) |
| #349 | Fix #344: License compliance - AGPL-3.0 | 6 | No | CLOSE (superseded by #353) |
| #345 | Fix #343: License compliance remediation | 7 | No | CLOSE (superseded by #349/#353) |
| #327 | Fix #326: Merge warp-drive-theme | 43 | No | CLOSE (superseded by #250) |
| #283 | fix(ci): checkout clean: true | 1 | No* | REBASE & MERGE (trivial) |
| #250 | feat(orchestrator): ADF warp-drive-theme | 20 | Yes | MERGE (Cargo.lock conflict, mechanical) |
| #245 | feat: consolidate agent registry | 1 | No | REBASE & MERGE (clean after #239) |
| #239 | feat: KG-boosted file search / workspace deps | 1 | Yes | MERGE (12 Cargo.toml conflicts, mechanical) |
| #185 | feat: SharedLearning, UUID, automata mentions | 8 | No | REBASE & MERGE (supersedes #184) |
| #184 | Fix #173: Replace regex with automata | 7 | No | CLOSE (superseded by #185) |
| #157 | Fix #155: PreCheckStrategy enum | 0 | Yes | CLOSE (already merged) |
| #156 | feat: offline mode default for TUI | 1 | Yes | MERGE (main.rs conflict, local already merged) |
| #154 | MeSH ontology benchmark (Phase 3-4) | 2 | No | REBASE & MERGE (additive) |

*PR #283 shows not mergeable but only touches `.github/workflows/ci-native.yml` (2 lines) -- likely needs simple rebase.

### File Overlap Matrix (Critical for Merge Order)

Key overlapping files across PRs:

| File | PRs That Touch It | Merge Implication |
|------|-------------------|-------------------|
| `Cargo.lock` | #353, #345, #349, #250, #239, #185, #184, #245 | Every merge updates Cargo.lock; must resolve sequentially |
| `Cargo.toml` (workspace) | #353, #239 | #239 converts 48 crates to workspace deps; merge BEFORE others |
| `crates/terraphim_automata/` | #185, #184 | #185 supersedes #184; close #184 |
| `crates/terraphim_orchestrator/` | #250, #185, #184 | #250 adds ADF features; #185/#184 add mention dispatch |
| `crates/terraphim_agent/` | #185, #184, #156 | #156 adds tui_backend; #185 adds shared_learning |
| `crates/terraphim_tracker/src/gitea.rs` | #250, #154 | Both extend Gitea API; merge #250 first |

### Agent-Generated Noise

Several branches contain auto-generated files that should be cleaned before merging:
- `.opencode/reviews/` files (PRs #353, #349, #345, #327)
- Empty `curl` file (PR #353)
- Empty `export` file (PR #327)
- Stale report files (PRs #327, #345)
- `.md` root file with 1 line (PR #353)

## Constraints

### Technical Constraints
- Cargo.lock conflicts are mechanical but must be resolved by running `cargo build` after each merge
- PR #239 changes 48 Cargo.toml files -- must merge before other PRs that touch Cargo.toml to avoid cascading conflicts
- PR #185 supersedes #184 (same base commits + 1 additional); merging #184 then #185 would create unnecessary conflicts

### Business Constraints
- Security CVE (#353) should merge ASAP
- Warp-drive-theme (#250) is the core ADF feature branch -- high business value

### Non-Functional Requirements
| Requirement | Target | Notes |
|-------------|--------|-------|
| Build passes | `cargo build --workspace` | After every merge |
| Lint passes | `cargo clippy` | After every merge |
| No dead code | `cargo fmt` clean | After every merge |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Merge #239 before others touching Cargo.toml | 48 crates converted to workspace deps; cascading conflicts otherwise | 12 Cargo.toml conflicts detected |
| Close superseded PRs before merging | #345, #349, #327, #184, #157 are dead branches | Shared commits, already-merged code |
| Verify build after each merge | Must not break main | Standard engineering practice |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Cleaning agent noise from all branches | Not blocking; can be done post-merge |
| Running full test suite after each merge | Tests are already validated per-PR |
| Rebased onto latest main for all branches | Only rebase branches we actually merge |

## Dependencies

### Internal Dependencies (Merge Order)
```
#239 (workspace deps) -- MUST merge first
  |
  +-- #283 (CI fix, trivial rebase)
  +-- #245 (registry consolidation, rebase after #239)
  +-- #154 (MeSH benchmarks, rebase after #239)
  +-- #156 (offline TUI, rebase after #239)
  +-- #250 (ADF warp-drive, merge after #239)
  +-- #353 (CVE fix, rebase after #239)
  +-- #185 (SharedLearning + automata, rebase after #239 and #250)
```

### External Dependencies
None beyond the codebase itself.

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cargo.toml conflicts cascade if wrong order | High | High | Merge #239 first |
| PR #353 contains agent noise files | Medium | Low | Cherry-pick only relevant commits or clean before merge |
| PR #185 diverges significantly from main | Medium | Medium | Rebase onto latest main after #239 and #250 |
| Cargo.lock conflicts on every merge | High | Low | Run `cargo build` to regenerate |

### Open Questions
1. Should agent-generated noise files (`.opencode/reviews/`, empty `curl`) be stripped before merge? -- **Recommendation: yes, but can be post-merge cleanup**
2. PR #353 includes license compliance changes mixed with security fix -- should these be separated? -- **Recommendation: merge as-is, it's all improvements**
3. Should merged PRs be pushed to GitHub `origin` after each merge or batched? -- **Recommendation: push to gitea after each, batch push to origin at end**

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| #239's workspace dep conversion is correct | PR was reviewed | Cargo.toml errors break build | No -- need to verify build |
| #185 supersedes #184 | Same 7 base commits + 1 additional | Missing unique changes in #184 | Yes |
| #349 is superseded by #353 | #349's 6 commits are subset of #353's 8 | Missing license work | Partially -- #349 has AGPL additions |
| #327 is superseded by #250 | #327 is "merge warp-drive-theme" task | #327 has ADR docs not in #250 | No -- need to verify ADR inclusion |
| PR #157 is already merged | 0 commits ahead of main | PR left open by mistake | Yes (git merge-tree verified) |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Merge all PRs via Gitea merge API | Simple, tracked | Rejected -- many need rebase first |
| Rebase all branches onto main, force-push, then merge | Clean history | Chosen -- gives clean linear history |
| Cherry-pick specific commits | Surgical, avoids noise | Rejected for all except possibly #353 |

## Research Findings

### Key Insights
1. **5 of 13 PRs should be closed** (#157 already merged, #184 superseded by #185, #345/#349 superseded by #353, #327 superseded by #250)
2. **PR #239 is the critical path** -- it touches 48 Cargo.toml files and must merge first
3. **The "not mergeable" status is mostly stale** -- most PRs just need a rebase onto current main
4. **Agent-generated noise** is pervasive in recent branches -- a codebase cleanup pass is warranted post-merge
5. **PR #353 is security-critical** (CVE) but shares commits with #349/#345, complicating isolation

### Recommended Merge Order

**Wave 1: Close dead PRs (no code changes)**
- Close #157 (already merged)
- Close #184 (superseded by #185)
- Close #345 (superseded by #353)
- Close #349 (superseded by #353)
- Close #327 (superseded by #250)

**Wave 2: Merge clean, foundational PRs**
1. #239 (workspace deps) -- resolve 12 Cargo.toml conflicts
2. #283 (CI fix) -- trivial rebase, 2-line change
3. #156 (offline TUI default) -- rebase, resolve main.rs conflict
4. #245 (registry consolidation) -- rebase after #239

**Wave 3: Merge feature PRs**
5. #250 (ADF warp-drive-theme) -- resolve Cargo.lock conflict
6. #154 (MeSH benchmarks) -- rebase after #239
7. #185 (SharedLearning + automata) -- rebase after #239 + #250

**Wave 4: Merge security fix**
8. #353 (CVE rustls-webpki) -- rebase after all above, clean agent noise

## Next Steps

Await human approval of this research document, then proceed to Phase 2 (Design) to create the detailed implementation plan.
