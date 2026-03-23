# Research Document: Terraphim AI Priority Issues & PRs

**Status**: Draft  
**Author**: Terraphim AI Assistant  
**Date**: 2026-02-04  
**Reviewers**: @AlexMikhalev

## Executive Summary

Terraphim AI has 28 open issues and 20+ open PRs. This research identifies the most critical items that, if addressed, will unblock development, improve stability, and deliver high-value features. The analysis reveals three priority tiers:

1. **Critical Blockers** (3 items): Prevent compilation, auto-updates, and build on clean clones
2. **High-Value Ready PRs** (4 items): Complete features awaiting review/merge
3. **Strategic Features** (4 items): Major capabilities with clear user value

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Core infrastructure fixes unblock all other work; CodeGraph is high-impact |
| Leverages strengths? | Yes | Rust expertise, knowledge graph systems, agent tooling |
| Meets real need? | Yes | Users cannot build from source (#491), updates fail (#462), PRs are staled |

**Proceed**: Yes - at least 2/3 YES

---

## Problem Statement

### Description
The project has accumulated technical debt and feature PRs that are blocking progress:
- **Build system broken**: Clean clone fails due to missing dependency (#491)
- **Auto-updater broken**: Users cannot receive updates (#462)
- **Ready PRs stalled**: 4 major feature PRs ready but not merged
- **Performance issues**: 7 performance-related issues need attention
- **Missing capabilities**: Agent tooling, 1Password parity, CodeGraph

### Impact
- **Contributors**: Cannot build from source, high friction
- **Users**: Auto-update fails, missing features
- **Development**: PR queue growing, context switching overhead
- **Release**: Blocked from shipping stable updates

### Success Criteria
1. Clean clone builds successfully
2. Auto-updater works reliably across platforms
3. Ready PRs merged and released
4. Performance regression issues addressed
5. Strategic features have clear roadmap

---

## Current State Analysis

### Workspace Structure
- **48 crates** in workspace
- **Key binaries**: `terraphim-agent`, `terraphim-server`, `terraphim-cli`, `terraphim-repl`
- **Edition**: 2024 (requires Rust 1.85+)

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Auto-updater | `crates/terraphim_update/` | GitHub Releases integration |
| Test Utils | `crates/terraphim_test_utils/` | Safe env var wrappers |
| 1Password | `crates/terraphim_onepassword_cli/` | Secret management |
| RLM | `crates/terraphim_rlm/` | Recursive LM orchestration |
| Validation | `crates/terraphim_validation/` | Release validation framework |
| Agent | `crates/terraphim_agent/` | CLI/TUI interface |
| Multi-Agent | `crates/terraphim_multi_agent/` | Agent orchestration |

### Critical Issues Breakdown

#### #491: Build Fails on Clean Clone
- **Root Cause**: `terraphim_rlm` depends on `fcctl-core` via local path `../../../firecracker-rust/fcctl-core`
- **Impact**: All new contributors blocked
- **Options**: A) Git submodule, B) Feature flag, C) Exclude from workspace

#### #462: Auto-Update 404 Error
- **Root Cause**: Asset name mismatch (`terraphim_agent-1.5.2-x86_64-unknown-linux-gnu.tar.gz` vs actual release asset)
- **Impact**: Users stuck on old versions
- **Fix**: Normalize asset naming or update lookup logic

#### Compilation Errors (New)
- `terraphim_test_utils`: Unsafe `set_var`/`remove_var` in Rust 1.92+
- Fix already attempted with feature flag, but config not applied

### Open PRs Status

| PR | Issue | Status | Description | Effort |
|----|-------|--------|-------------|--------|
| #516 | - | OPEN | Agent integration tests (cross-mode + KG ranking) | Low |
| #492 | #493 | OPEN | CLI onboarding wizard (6 templates) | Medium |
| #443 | #442 | OPEN | Validation framework - runtime LLM hooks | Medium |
| #413 | #442 | OPEN | Validation framework - base implementation | Low |
| #426 | #480 | OPEN | RLM orchestration crate (6 MCP tools) | High |
| #461 | - | OPEN | GPUI desktop app (DON'T MERGE - 68K lines) | Very High |

---

## Constraints

### Technical Constraints
- **Rust 1.85+ required** (2024 edition)
- **Firecracker dependency** (optional, but RLM crate depends on it)
- **GitHub Releases** for auto-updates (asset naming must match)
- **Cross-platform** support (Linux, macOS, Windows)

### Business Constraints
- **User trust**: Auto-updater must be reliable
- **Contributor experience**: Clean clone must build
- **Release velocity**: Blocked by broken build/updater

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Clean clone build | < 2 min | Fails |
| Auto-update success rate | > 95% | 0% (#462) |
| PR review turnaround | < 1 week | 2-4 weeks |
| Test coverage | > 80% | Unknown |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Build must work on clean clone | Blocks all contributors | #491, repeated reports |
| Auto-updater must function | User trust, security updates | #462, core feature |
| PRs must be reviewed/merged | Development velocity | 20+ open PRs, some 4+ weeks old |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| GPUI migration (#461) | 68K lines, "DON'T MERGE" label, needs separate epic |
| All performance issues (#432, #434-#438) | Important but not blocking, can batch |
| MCP Aggregation phases (#278-281) | Complex, needs design, lower priority |
| npm/PyPI publishing (#315, #318) | Nice-to-have, not blocking core functionality |

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_rlm` on `fcctl-core` | Blocks build | High (#491) |
| `terraphim_settings` on `onepassword` | Optional feature | Low |
| `terraphim_update` on `self_update` | Core updater | Medium |
| `terraphim_test_utils` unsafe code | Blocks compilation | High (new) |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| firecracker-rust | N/A (path) | High | Feature gate, stub |
| self_update | 0.40+ | Medium | None (essential) |
| ast-grep | N/A | Medium | tree-sitter directly |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| RLM feature gating breaks functionality | Medium | Medium | Comprehensive tests |
| Auto-update fix requires release process changes | Medium | Low | Document process |
| GPUI PR creates merge conflicts | High | Medium | Rebase early, often |
| 1Password audit reveals API gaps | Medium | Medium | Feature flag gaps |

### Open Questions

1. **What is the actual GitHub Release asset naming convention?** - Need to inspect releases page
2. **Is firecracker-rust intended to be a submodule or separate repo?** - Check .gitmodules
3. **What is the target architecture for #461 (GPUI)?** - Merge into main or keep parallel?
4. **Are there security concerns with the current 1Password implementation?** - Compare to JS lib

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `fcctl-core` can be feature-gated | PR #426 uses features | RLM non-functional | No |
| Auto-update asset name uses underscores | Issue #462 logs | Fix won't work | No |
| Test utils fix needs build.rs change | Error message | Still broken | No |
| Ready PRs (#492, #443, #516) are complete | PR descriptions | Missing work discovered | Partial |

---

## Research Findings

### Key Insights

1. **Build blocker is a one-line fix**: Feature-gating `fcctl-core` in `terraphim_rlm/Cargo.toml`
2. **Auto-update likely asset naming**: Underscore vs hyphen mismatch in binary names
3. **4 PRs are genuinely ready**: Descriptions show completion, just need review
4. **Test utils already has fix**: Just needs proper feature flag configuration
5. **CodeGraph (#490) is high-value**: Addresses real agent pain point

### Prioritization Matrix

| Item | User Impact | Effort | Strategic Value | Priority |
|------|-------------|--------|-----------------|----------|
| #491 Build fix | High (blocks all) | 1 hour | High | P0 |
| #462 Auto-update | High (users stuck) | 2 hours | High | P0 |
| Test utils fix | High (CI blocked) | 30 min | High | P0 |
| #516 Agent tests | Medium | 1 hour | Medium | P1 |
| #492 Onboarding | High | 2 hours | High | P1 |
| #443 Validation | Medium | 2 hours | Medium | P1 |
| #426 RLM | Medium | 4 hours | High | P2 |
| #503 1Password | Low | 8 hours | Medium | P2 |
| #499 Search output | Medium | 4 hours | Medium | P2 |
| #490 CodeGraph | High | 40 hours | Very High | P3 (epic) |

---

## Recommendations

### Proceed/No-Proceed
**PROCEED** - Critical blockers need immediate attention. Ready PRs deliver value. Strategic features are well-defined.

### Scope Recommendations

**Phase 1 (This Week)**: Unblock Everything
- Fix #491 (build) - feature gate fcctl-core
- Fix test utils compilation
- Fix #462 (auto-update) - normalize asset names
- Merge #516 (agent tests) - ready, low risk

**Phase 2 (Next Week)**: Deliver Value
- Merge #492 (onboarding wizard) - high user value
- Merge #443 (validation framework) - infrastructure
- Merge #413 (if not merged with #443)
- Review #426 (RLM) - high complexity

**Phase 3 (Following Weeks)**: Strategic Features
- #503 1Password audit
- #499 Agent search output format
- #490 CodeGraph (break into milestones)

### Risk Mitigation Recommendations

1. **Test the build fix** on clean clone before merging
2. **Verify auto-update** with actual GitHub release asset names
3. **Batch performance issues** (#432, #434-438) into single PR
4. **Create CodeGraph milestone** with clear deliverables

---

## Next Steps

### Immediate (Today)
1. Create design document for P0 fixes
2. Verify asset naming on GitHub releases page
3. Test feature gate approach for fcctl-core

### Short-term (This Week)
1. Implement P0 fixes
2. Review and merge ready PRs
3. Create CodeGraph epic

### Medium-term (Next 2 Weeks)
1. Address P2 items
2. Batch performance optimizations
3. Plan next release

---

## Appendix

### Reference Materials
- Issue #491: https://github.com/terraphim/terraphim-ai/issues/491
- Issue #462: https://github.com/terraphim/terraphim-ai/issues/462
- PR #516: Integration tests
- PR #492: CLI onboarding wizard
- PR #443: Validation framework (LLM hooks)
- PR #426: RLM orchestration

### Code Locations
```
crates/terraphim_rlm/Cargo.toml          # fcctl-core dependency
crates/terraphim_update/src/downloader.rs # Asset download logic
crates/terraphim_test_utils/Cargo.toml   # Feature flag config
crates/terraphim_onepassword_cli/src/    # 1Password implementation
```

### Dependency Graph (Simplified)
```
terraphim_agent
├── terraphim_update (self_update)
├── terraphim_settings
│   └── terraphim_onepassword_cli (optional)
└── terraphim_rlm (fcctl-core, optional?)
```
