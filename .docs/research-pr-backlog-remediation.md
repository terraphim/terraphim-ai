# Research Document: Systematic PR Backlog Remediation

**Status**: Draft
**Author**: Root (orchestrator agent)
**Date**: 2026-05-31
**Reviewers**: TBD

## Executive Summary

All 19 open PRs in terraphim-ai are non-mergeable due to branch drift from main. Triage reveals 5 PRs are already superseded by code on main, but 14 represent real unfixed issues requiring fresh implementation. This document analyses the root causes and establishes a prioritised remediation plan.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Security vulnerabilities and core feature gaps directly impact production |
| Leverages strengths? | Yes | We have full codebase access and CI infrastructure |
| Meets real need? | Yes | Issues span P0 security, core features, CI quality, and test reliability |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
The repository has accumulated 19 stale, non-mergeable PRs. These represent:
- **Security vulnerabilities** (Redis exposed on 0.0.0.0)
- **Core feature gaps** (context rot detection, Thesaurus matching, RLM hardening)
- **CI quality regressions** (no rustdoc warning gate)
- **Test reliability issues** (parallel test interference, wrong role names)
- **Configuration problems** (strict permissions missing, robot envelope incomplete)

### Impact
- **Security**: Redis binding exposes cache to network; circuit breaker incorrectly penalises config errors
- **Functionality**: Robot mode lacks Thesaurus matching; context rot silently degrades agents
- **Quality**: No CI gate preventing rustdoc decay; intermittent test failures
- **Velocity**: 19 blocked PRs create cognitive overhead and hide real work

### Success Criteria
1. All 19 stale PRs closed with documented rationale
2. Fresh focused PRs created for 14 real issues
3. Security issues fixed within 48 hours
4. Core features landed within 1 week
5. CI quality gates operational within 1 week

## Current State Analysis

### Repository Health
- **Main branch**: `1051ff255fe6dfb3022bfbfe41448e7c7f3d2fe1` (current)
- **Open PRs**: 19, all `mergeable: false`
- **Merge base drift**: All PRs branch from commits predating substantial refactors
- **Key refactor**: `NormalizedTerm` removed, `meta_coordinator` wired, temp path fixes applied

### Existing Implementation (What IS on Main)
| Feature | Status | Evidence |
|---------|--------|----------|
| `NormalizedTerm` fix | DONE | Struct removed/refactored |
| `meta_coordinator` wiring | DONE | `lib.rs:57` has `pub mod meta_coordinator;` |
| Unique temp path for MCP tests | DONE | `mcp_tool_index.rs:281-287` |
| `warn_if_world_readable()` | DONE | `config.rs:1320` with tests |

### Missing Implementation (What is NOT on Main)
| Feature | Issue | Evidence |
|---------|-------|----------|
| Redis 127.0.0.1 binding | #1313 | `docker-compose.yml:55` shows `6379:6379` |
| Context rot detection | #1443 | No `context_rot` references in orchestrator |
| Thesaurus matching in robot mode | #851 | No `Thesaurus_matched` field population |
| RLM executor hardening | #1488 | No `LocalExecutor`/`DockerExecutor` in codebase |
| Rustdoc CI gate | #1362 | No `RUSTDOCFLAGS=-D warnings` in workflows |
| RetryBound enforcement | #251 | No `RetryBound` logic in Symphony |
| C1 probe exemption | #446 | No `is_environment_error()` predicate |

## Constraints

### Technical Constraints
- **Rust toolchain**: Must compile with `cargo check --workspace --all-features`
- **CI**: GitHub Actions on self-hosted runners (`bigbox`)
- **Docker**: Compose v3.8, services must bind to 127.0.0.1
- **Gitea**: PR workflow requires `gitea-robot` for status updates

### Business Constraints
- **Timeline**: Security issues must land before next release
- **Scope**: Each fix must be a focused PR (< 20 files ideally)
- **Compatibility**: No breaking changes to existing APIs without ADR

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| CI pipeline time | < 30 min | ~25 min |
| Test flakiness | < 1% | Unknown (parallel test issues suggest higher) |
| Doc coverage | 100% public items | ~85% (rustdoc warnings present) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
1. **Security first**: Redis binding and circuit breaker fixes must land before any feature work
2. **Small PRs**: Each fix must be independently reviewable and mergeable
3. **No rebase attempts**: All work starts fresh from current main to avoid merge conflicts

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Rebase existing PRs | Branches are too stale; merge conflicts guaranteed |
| Bulk close all PRs without triage | Would lose track of real issues |
| Single mega-PR with all fixes | Violates small PR constraint; review impossible |
| Rewrite Symphony from scratch | Out of scope; RetryBound fix is surgical |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_orchestrator` | Most fixes touch this crate | High - core system |
| `terraphim_agent` | Robot mode, test fixes | Medium |
| `terraphim_symphony` | RetryBound fix | Low - isolated module |
| CI workflows | Quality gates | Medium |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Docker Compose | 3.8 | Low | None (standard) |
| GitHub Actions | v4 | Low | None |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Security fix breaks local dev | Medium | Medium | Test with `docker-compose up` before PR |
| Context rot detection has false positives | Medium | High | Start with conservative thresholds |
| RLM hardening requires API changes | Medium | Medium | Review API contract first |
| CI rustdoc gate fails on existing warnings | High | Medium | Run `cargo doc` first to establish baseline |

### Open Questions
1. What is the current rustdoc warning count on main? - **Answer**: Need to run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace`
2. Does `test_full_feature_matrix` still exist? - **Answer**: Search shows it may have been renamed/removed
3. Is the Symphony `on_retry_timer` code path still active? - **Answer**: Need to verify module existence

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| All 14 issues are still relevant on main | grep searches returned no matches for fixes | Some issues may have been fixed through unrelated refactors | Partially verified |
| Redis binding fix is backward-compatible | Only changes compose port binding | Could break remote Redis access in some setups | No |
| Small focused PRs will be accepted | Team preference from AGENTS.md | Could be asked to combine | No |

## Research Findings

### Key Insights
1. **Systematic drift**: The backlog is not accidental; it's a process failure. No PR has been merged from this batch, indicating a workflow breakdown around mid-May 2026.
2. **Refactor velocity**: Main moved fast in late May, invalidating many branches. The `NormalizedTerm` removal, `meta_coordinator` wiring, and temp path fixes show substantial refactoring.
3. **Security exposure**: Redis on 0.0.0.0 is a live vulnerability in the docker-compose.yml on main.

### Relevant Prior Art
- **PR #1788**: Successfully split into 7 focused PRs (#1921-#1925) - this pattern works
- **PR #1920**: Small focused fix merged successfully - validates the approach

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Run `cargo doc` with `-D warnings` | Establish baseline warning count | 10 minutes |
| Check Symphony `on_retry_timer` existence | Verify #251 is still applicable | 5 minutes |
| Verify `test_full_feature_matrix` location | Find test for #1358 fix | 5 minutes |

## Recommendations

### Proceed/No-Proceed
**PROCEED** - All three essential questions answered YES. The security issues alone justify immediate action.

### Scope Recommendations
- **Phase 1**: Security fixes (#1319 Redis binding)
- **Phase 2**: Core features (#1524 context rot, #1380 Thesaurus, #1491 RLM)
- **Phase 3**: CI/quality (#1365 rustdoc gate, #1367 test fix, #1514 permissions)
- **Phase 4**: Medium priority (#1600, #1316, #1349, #1599, #1615, #1604, #1308)

### Risk Mitigation Recommendations
- Create each fix as independent branch from main
- Run full CI on each PR before requesting review
- Document each fix with references to original issue and stale PR

## Next Steps

1. Create implementation plan (Phase 2) with detailed file changes and test strategies
2. Get human approval on prioritisation
3. Begin Phase 1 implementation immediately (security fixes)

## Appendix

### Reference Materials
- `.docs/pr-backlog-correction-plan.md` - Initial triage results
- Issue #1926 - Meta tracking issue

### Code Snippets
```yaml
# Current docker-compose.yml Redis binding (vulnerable)
redis:
  ports:
    - "6379:6379"
```

```rust
// Current lib.rs has meta_coordinator (fixed)
pub mod meta_coordinator;
```

```rust
// Current mcp_tool_index.rs has unique temp path (fixed)
let unique = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .subsec_nanos();
let index_path = temp_dir.join(format!("test-mcp-index-{unique}.json"));
```
