# Merged Research & Design Plan: 2026-05-30

**Status**: Draft
**Author**: opencode
**Date**: 2026-05-30
**Type**: Consolidated Research + Design (Disciplined Development Phase 1-2)

---

## Executive Summary

This document consolidates the current state of the terraphim-ai project based on:
- PageRank-prioritised triage of 555 open issues
- 20 open PRs (9 mergeable, 11 need fixes)
- Existing research and design documents

The top priorities are:
1. **Immediate**: Merge the 9 ready PRs to reduce backlog
2. **Short-term**: Fix and merge the 11 blocked PRs
3. **Medium-term**: Implement Fast/Cheap LLM Build-Runner (Epic #1423)
4. **Long-term**: ADF self-healing infrastructure

---

## Part 1: Research (Phase 1)

### 1.1 Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Does this energise us? | Yes | Reducing PR backlog improves velocity |
| Leverages unique strengths? | Yes | ADF infrastructure, terraphim-agent learning |
| Meets validated need? | Yes | 555 open issues indicates backlog problem |

**Proceed**: Yes - 3/3 YES

### 1.2 Current State Analysis

#### Open PRs (20 total)

| Category | Count | PRs |
|----------|-------|-----|
| **Mergeable (Ready)** | 9 | #1891, #1865, #1860, #1858, #1852, #1849, #1828, #1789, #1787 |
| **Blocked (Needs Fix)** | 11 | #1898, #1867, #1859, #1850, #1836, #1832, #1800, #1791, #1788, #1786, #1767 |

#### Top PageRank Issues (from triage)

| Rank | Issue | PageRank | Title |
|------|-------|---------|-------|
| 1 | #1990 | 0.0053 | ADR-001: Fast/Cheap LLM Build-Runner |
| 2 | #2517 | 0.0042 | [ADF self-healing] Step 1: Quieten orchestrator logs |
| 3 | #903 | 0.0041 | Verdict Engine with weighted-score strategy |
| 4 | #2288 | 0.0038 | Tantivy full-text index for session search |
| 5 | #1423 | 0.0038 | Epic: Fast/Cheap LLM Build-Runner |

#### Mergeable PRs Detail

| PR | Title | Key Changes | Risk |
|----|-------|-------------|------|
| #1891 | docs(types): eliminate 93 missing-doc warnings | Rustdoc fixes | Low |
| #1865 | Fix #1862: Local .terraphim/ config first-priority | Config priority fix | Low |
| #1860 | Fix #1854: add missing CHANGELOG.md | Documentation | Low |
| #1858 | Fix #1855: gate slow server-integration tests | Test speedup | Low |
| #1852 | Fix #1845: remove infrastructure/* from workspace | Cleanup | Low |
| #1849 | Fix #1848: add README.md to terraphim_grep | Documentation | Low |
| #1828 | feat(ci): adopt cargo-nextest as workspace test runner | Test runner | Medium |
| #1789 | fix(types): SharedLearning::new() defaults to L0 | Bug fix | Low |
| #1787 | Fix #1779: acknowledge RUSTSEC-2026-0002 | Security | Low |

#### Blocked PRs Detail

| PR | Title | Blockers |
|----|-------|----------|
| #1898 | Fix #1768: .terraphim/skills/ local skill search | Needs fmt/clippy |
| #1867 | docs: rustdoc gaps + CHANGELOG | Needs rebase |
| #1859 | Fix #1853: cargo fmt violations | Format errors |
| #1850 | Fix #1849: terraphim_grep KG roles | Needs fmt |
| #1836 | fix(terraphim_merge_coordinator): license + rustdoc | Needs fmt |
| #1832 | Fix #1831: role config mismatch in tests | Needs investigation |
| #1800 | Fix #1715: preserve required PR dispatch contexts | Needs rebase |
| #1791 | fix(config): redact credentials in Debug | Needs investigation |
| #1788 | feat(adf): integrate .terraphim/skills/ with CLI | Needs #1786 |
| #1786 | Fix #1769: project-scoped ADF agent registry | Needs fmt |
| #1767 | Fix #1719: redact sensitive env_vars in Debug | Needs rebase |

### 1.3 Problem Statement

#### Immediate Problem: PR Backlog
- 20 open PRs, 9 mergeable
- 11 blocked by fmt violations, rebases, or dependency issues
- Low velocity erodes trust in the process

#### Medium-Term Problem: Build-Runner Rigidity (Epic #1423)
- Current build-runner uses hardcoded cargo commands
- No adaptation to project changes
- No learning from previous runs
- Fails on rate limits without intelligent retry

#### Long-Term Problem: ADF Observability (#2517)
- Orchestrator logs too noisy for production debugging
- State transitions buried in verbose output
- Need structured health aggregation

### 1.4 Existing Architecture

```
terraphim-ai/
├── crates/
│   ├── terraphim_orchestrator/   # ADF orchestration engine
│   ├── terraphim_agent/           # CLI agent with learning
│   ├── terraphim_service/         # LLM proxy and routing
│   ├── terraphim_router/          # Cost-aware provider selection
│   ├── terraphim_automata/        # Markdown directive parsing
│   └── terraphim_types/           # Core types
├── desktop/                       # Svelte UI
└── .docs/                        # Research and design docs
```

### 1.5 Constraints

#### Technical Constraints
- Must maintain deterministic fallback for builds
- Must use cheap models only (cost < $0.01/build)
- Must preserve existing LLM proxy multi-provider fallback
- Nextest migration must not break existing test workflows

#### Business Constraints
- No regression in build reliability (82.83% cache hit rate)
- Backward compatibility for PRs without BUILD.md
- Security fixes (RUSTSEC) must be expedited

### 1.6 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Nextest migration breaks CI | Medium | High | Keep cargo test as fallback |
| fmt violations block legitimate fixes | High | Medium | Automated fmt pre-commit |
| Build-runner LLM hallucinates commands | Medium | High | Deterministic fallback |
| Stale learnings cause wrong commands | Medium | Low | Timestamp filtering |

### 1.7 Open Questions

1. **For #1828 (nextest)**: Is there a migration path that preserves existing test output format?
2. **For #2517 (self-healing)**: What is the minimal log surface needed for health aggregation?
3. **For Epic #1423**: Should we parse existing BUILD.md files or create new `build::` directives?

---

## Part 2: Design (Phase 2)

### 2.1 Scope Definition

#### In Scope (Top 5 - Vital Few)
1. **Merge ready PRs** (9 PRs, low risk, high velocity)
2. **Fix fmt-violation PRs** (5 PRs: #1859, #1850, #1836, #1786, #1898)
3. **Nextest migration** (#1828 - has design doc, medium risk)
4. **RUSTSEC fix** (#1787 - security, must priority-slot)
5. **Local skills path** (#1898, #1788 - related, merge together)

#### Out of Scope
- Epic #1423 (Fast/Cheap Build-Runner) - needs separate research
- #2517 (ADF self-healing) - needs separate research
- #2288 (Tantivy session search) - needs separate research

#### Avoid At All Cost
- Large refactors during merge sprint
- Adding new features while cleaning up old PRs
- Parallel nextest + build-runner changes

### 2.2 Implementation Plan: Merge Sprint

#### Step 1: Merge Low-Risk Documentation PRs
**Goal**: Build momentum, verify process

| PR | Action | Estimated |
|----|--------|-----------|
| #1860 | Merge (CHANGELOG) | 5 min |
| #1849 | Merge (README) | 5 min |
| #1852 | Merge (workspace cleanup) | 5 min |

#### Step 2: Merge Security + Types PRs
**Goal**: Address critical fixes

| PR | Action | Estimated |
|----|--------|-----------|
| #1787 | Merge (RUSTSEC-2026-0002) | 10 min |
| #1789 | Merge (SharedLearning default) | 10 min |
| #1891 | Merge (rustdoc fixes) | 15 min |

#### Step 3: Merge Config Priority Fix
**Goal**: Improve local development experience

| PR | Action | Estimated |
|----|--------|-----------|
| #1865 | Merge (local .terraphim/ priority) | 15 min |

#### Step 4: Fix and Merge Test-Related PRs
**Goal**: Improve CI performance

| PR | Action | Blockers | Estimated |
|----|--------|----------|-----------|
| #1858 | Merge (gate slow tests) | None | 15 min |
| #1828 | Needs design review | Nextest migration | 2 hours |

#### Step 5: Fix Format Violations
**Goal**: Unblock 5 PRs

| PR | Action | Fix Command | Estimated |
|----|--------|-------------|-----------|
| #1859 | Apply fmt | `cargo fmt --all` | 5 min |
| #1850 | Apply fmt | `cargo fmt --all` | 5 min |
| #1836 | Apply fmt | `cargo fmt --all` | 5 min |
| #1786 | Apply fmt | `cargo fmt --all` | 5 min |
| #1898 | Apply fmt | `cargo fmt --all` | 5 min |

**Note**: After fmt, these PRs should become mergeable. Re-check and merge.

#### Step 6: Handle Dependency Chain (#1788 ← #1786)
**Goal**: Land skills integration

1. Verify #1786 is merged
2. Check #1788 for conflicts
3. Merge #1788 if clean

### 2.3 Detailed Design: Nextest Migration (#1828)

#### Architecture Decision
```
Current: cargo test --workspace
Target:  cargo nextest run --workspace --profile ci
Fallback: cargo test --workspace (if nextest fails)
```

#### File Changes

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `cargo-nextest` to tools |
| `.github/workflows/ci.yml` | Replace cargo test with nextest |
| `scripts/ci-guard-*.sh` | Update test commands |

#### Test Strategy
1. Run nextest in parallel with cargo test (shadow mode)
2. Compare results for 10 PRs
3. Switch to nextest-only after validation

#### Rollback
```bash
# If nextest breaks:
git revert "Merge #1828"
# Then use cargo test temporarily
```

### 2.4 Detailed Design: Local Skills Path (#1898, #1788)

#### Architecture
```
OpenCode/Claude → terraphim-agent → skill search
                                ↓
                    ~/.terraphim/skills/  (local)
                    .terraphim/skills/    (project)
                    /usr/local/share/...  (system)
```

#### Key Files

| File | Change |
|------|--------|
| `crates/terraphim_agent/src/skills.rs` | Add local path priority |
| `crates/terraphim_agent/src/cli.rs` | Update skill search command |

#### Test Strategy
```bash
# Test local skills are found first
echo "test skill" > .terraphim/skills/test.md
terraphim-agent skills search "test"
# Should find local first
```

### 2.5 Simplified Design: Fast/Cheap Build-Runner (Future)

#### Selected Approach (from research-fast-cheap-build-runner.md)
**Option B**: LLM extracts commands, deterministic validation executes

#### Component Diagram
```
Push Event → build-runner-llm
    ├── Query terraphim-agent learnings
    ├── Parse BUILD.md/CONTRIBUTING.md (haiku)
    ├── Validate against whitelist
    ├── Execute via rch
    └── POST_STATUS
```

#### Vital Few for Build-Runner
1. **Always-on fallback**: If LLM fails, run hardcoded cargo commands
2. **Cost cap**: Max $0.01 per extraction
3. **Command whitelist**: Only cargo, make, npm, etc.

---

## Part 3: Implementation Sequence

### Phase A: Immediate (This Session)

| Step | Action | PRs | Time |
|------|--------|-----|------|
| A1 | Merge momentum PRs | #1860, #1849, #1852 | 15 min |
| A2 | Merge security PR | #1787 | 10 min |
| A3 | Merge types PRs | #1789, #1891 | 25 min |
| A4 | Merge config PR | #1865 | 15 min |
| A5 | Merge test PR | #1858 | 15 min |

**Subtotal**: ~80 minutes

### Phase B: Quick Wins

| Step | Action | PRs | Time |
|------|--------|-----|------|
| B1 | Fix fmt violations | #1859, #1850, #1836, #1786, #1898 | 25 min |
| B2 | Re-check and merge | All above after fmt | 30 min |
| B3 | Handle dependency chain | #1788 (after #1786) | 15 min |

**Subtotal**: ~70 minutes

### Phase C: Medium Effort

| Step | Action | Issue/PR | Time |
|------|--------|----------|------|
| C1 | Nextest design review | #1828 | 2 hours |
| C2 | Nextest implementation | #1828 | 4 hours |
| C3 | Nextest validation | Shadow run | 2 hours |

**Subtotal**: ~8 hours

### Phase D: Future (Separate Sessions)

| Step | Action | Issue | Status |
|------|--------|-------|--------|
| D1 | Research: Build-runner LLM | Epic #1423 | Draft research exists |
| D2 | Research: ADF self-healing | #2517 | Needs research |
| D3 | Design: Tantivy session search | #2288 | Needs research |

---

## Part 4: Verification & Rollback

### Acceptance Criteria

| Phase | Criteria | Verification |
|-------|----------|--------------|
| A | 6 PRs merged | `git log --oneline -6` |
| B | 5 more PRs merged | `git log --oneline -11` |
| C | nextest works in CI | `gh run list` shows green |
| All | Remotes converged | `git diff origin/main gitea/main --stat` empty |

### Rollback Plan

| If... | Then... |
|-------|---------|
| CI fails after merge | Revert commit, investigate |
| nextest breaks tests | Disable nextest, use cargo test |
| Format war breaks files | Re-run fmt, recommit |

---

## Appendix: PR Classification Matrix

| PR | Category | Effort | Risk | Action |
|----|----------|--------|------|--------|
| #1891 | docs | 15 min | Low | Merge now |
| #1865 | fix | 15 min | Low | Merge now |
| #1860 | docs | 5 min | Low | Merge now |
| #1858 | perf | 15 min | Low | Merge now |
| #1852 | cleanup | 5 min | Low | Merge now |
| #1849 | docs | 5 min | Low | Merge now |
| #1828 | feat | 8 hours | Medium | Design review first |
| #1789 | fix | 10 min | Low | Merge now |
| #1787 | security | 10 min | Low | Merge now |
| #1898 | fix | 5 min | Low | Apply fmt, merge |
| #1859 | fix | 5 min | Low | Apply fmt, merge |
| #1850 | fix | 5 min | Low | Apply fmt, merge |
| #1836 | fix | 5 min | Low | Apply fmt, merge |
| #1786 | fix | 5 min | Low | Apply fmt, merge |
| #1867 | docs | 30 min | Low | Rebase, merge |
| #1832 | fix | 1 hour | Medium | Investigate |
| #1800 | fix | 1 hour | Medium | Rebase needed |
| #1791 | fix | 1 hour | Medium | Investigate |
| #1788 | feat | 15 min | Low | After #1786 |
| #1767 | fix | 1 hour | Medium | Rebase needed |

---

**Document Status**: Ready for review
**Next Step**: Human approval to proceed with Phase A
