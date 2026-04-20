# Research Document: PR Backlog Prioritization & Merge Strategy

**Status**: Draft
**Author**: AI Analysis
**Date**: 2026-04-09
**Reviewers**: [Human Approval Required]

## Executive Summary

Analysis of 17 open PRs (2 GitHub, 15 Gitea) reveals a fragmented backlog with duplicate fixes, security remediations at varying stages, and feature work with unclear dependencies. The optimal approach is a 6-phase merge strategy prioritizing security/compliance first, consolidating duplicates, then proceeding to infrastructure and features.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Does this energize us? | YES | Critical security and compliance items blocking progress |
| Does it leverage strengths? | YES | Gitea PageRank workflow for prioritization |
| Does it meet a real need? | YES | 17 PRs backlogged, some contain security fixes |

**Proceed**: YES (3/3 YES)

## Problem Statement

### Description
17 open PRs across two remotes (GitHub and Gitea) with:
- Duplicate/overlapping fixes for the same issues
- Security remediations at varying stages of completion
- Feature PRs with unclear dependency chains
- Unclear which PRs are actually still open vs already merged

### Impact
- Development bottleneck due to PR backlog
- Security vulnerabilities may remain unfixed if PRs are not merged in correct order
- Wasted CI/resources on duplicate PRs
- Risk of merge conflicts if not sequenced properly

### Success Criteria
- All security/compliance PRs merged first
- Duplicate PRs consolidated or closed
- Clear dependency graph established
- All remaining PRs merged in priority order

## Current State Analysis

### Existing Implementation
Git repository with dual-remote setup:
- `gitea` remote: interim work-in-progress
- `origin` remote: release-quality only

### Remote Configuration
```
gitea   https://git.terraphim.cloud/terraphim/terraphim-ai.git (fetch/push)
origin  https://github.com/terraphim/terraphim-ai.git (fetch only shown)
```

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Security PRs | `task/486-*`, `task/440-*` branches | RUSTSEC-2026-0049 remediation |
| License fixes | `task/493-*`, `task/496-*`, `task/503-*` branches | License field additions |
| ValidationService | `feat/kg-command-validation` branch | Command validation infrastructure |

### Data Flow
PR lifecycle:
```
Branch Creation -> Commit -> Push to gitea -> PR Created -> Code Review -> Merge (gitea) -> Sync to origin (release)
```

## Constraints

### Technical Constraints
- Dual-remote architecture requires PRs to be release-quality before GitHub merge
- Security PRs require `security-sentinel` verification pass
- Tauri being removed - moved to terraphim-ai-desktop repository

### Business Constraints
- RUSTSEC-2026-0049 is a known vulnerability that should be remediated
- License compliance required for CI gates (cargo deny)
- Some PRs duplicate work already done in other branches

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Security fixes | 100% merged before feature work | Partial |
| Duplicate resolution | < 3 duplicate PRs remaining | 6+ duplicates |
| Merge queue | < 10 open PRs | 17 open |
| Tauri removal | Complete | Pending |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
1. **Security first**: RUSTSEC-2026-0049 must be remediated before release
2. **No duplicates**: Consolidate license field fixes to single PR
3. **Dependency order**: Infrastructure before features

### Eliminated from Scope
- Investigating why duplicate PRs were created (post-mortem for later)
- Refactoring existing ADF swarm architecture
- Adding new features not already in PR backlog

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| ValidationService (#520) | Blocks #516 extension work | Medium |
| Tauri v2 migration (#491) | Required for desktop builds | High |
| Token tracking (#519) | Building block for cost controls | Medium |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| RUSTSEC-2026-0049 | N/A | High - active CVE | Upgrade rustls-webpki |
| Tauri API v2 | Breaking | High | Stay on v1 (blocks desktop) |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Duplicate PR merge conflicts | High | Medium | Close duplicates first |
| RUSTSEC fix incomplete | Medium | Critical | Require security-sentinel pass |
| Tauri v2 breaks desktop | Medium | High | Require platform testing |

### Open Questions
1. Are PRs #508, #512 actually merged? (commit history suggests yes)
2. Does #516 unblock #520 deployment readiness?
3. What are phase completion criteria for #405 (Phase 7)?
4. Has desktop platform testing for #491 been performed?

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| RUSTSEC fix in #486 is complete | Multiple branches reference same CVE | Incomplete fix remains | No |
| License PRs are duplicates | Similar titles, same manifests | Merge conflicts | Partial |
| GitHub #776 is duplicate of Gitea #520 | Both ValidationService | Wasted review effort | Yes |

## Research Findings

### Key Insights
1. **Duplicate泛滥**: 6+ PRs address overlapping concerns (license fields, RUSTSEC remediation)
2. **Security debt**: Multiple RUSTSEC-related branches suggest incomplete or contested fixes
3. **Phase confusion**: #405 labeled "Phase 7" but unclear what phases 1-6 are
4. **State inconsistency**: Gitea UI shows PRs as "open" but commit history suggests some are merged

### Relevant Prior Art
- Gitea PageRank workflow for issue prioritization
- ZDP phased development (Discovery, Define, Design, Develop, Deploy, Drive)
- Dual-remote git workflow (interim vs release)

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify #486 RUSTSEC fix completeness | Confirm no remaining vulnerabilities | 2 hours |
| Tauri v2 API migration assessment | Confirm desktop compatibility | 4 hours |
| PR state reconciliation | Sync Gitea UI with actual branch state | 1 hour |

## Recommendations

### Proceed/No-Proceed
**PROCEED** with 6-phase merge strategy, cleaning duplicates first.

### Scope Recommendations
1. Close duplicate PRs before attempting any merges
2. Merge security/compliance PRs in dependency order
3. Defer Tauri v2 migration until platform testing confirmed
4. Close GitHub PRs that duplicate Gitea work

### Risk Mitigation Recommendations
1. Require `cargo audit` and `cargo deny` passes before compliance PR merge
2. Require security-sentinel verification for RUSTSEC fix
3. Perform Tauri v2 migration in staging environment first

## Next Steps (Upon Approval)

1. Reconcile Gitea PR state with actual branch state
2. Close duplicate PRs (#496, #503, GitHub #776)
3. Execute 6-phase merge strategy in order
4. Monitor CI/CD for failures

## Appendix

### PR Categorization Matrix

| Type | PRs | Count | Effort | Impact |
|------|-----|-------|--------|--------|
| **Security** | #486, #412 | 2 | Low | Critical |
| **Compliance** | #493, #496, #503 | 3 | Low | High |
| **Bugfix** | #475, #477, #508, #512 | 4 | Low | Medium |
| **Feature** | #405, #519, #520 | 3 | Medium | High |
| **Infrastructure** | #491 | 1 | Medium | High |

### Recommended Merge Sequence

| Phase | PRs | Action |
|-------|-----|--------|
| 1 - Cleanup | Duplicates | Close #496, #503, GitHub #776 |
| 2 - Security | #486, #412 | Merge first (critical) |
| 3 - Compliance | #493 | Consolidate license fixes |
| 4 - Bugfixes | #475, #477, #508, #512 | Merge low-risk |
| 5 - Infrastructure | #491 | Tauri v2 with testing |
| 6 - Features | #520, #405, #519 | Final merge wave |

### PRs to Close Immediately

| PR | Reason |
|----|--------|
| GitHub #776 | Duplicate of Gitea #520 |
| GitHub #775 | Bench fix appears already merged at main |
| #496 | Duplicate of #493 |
| #503 | Duplicate of #493 |
