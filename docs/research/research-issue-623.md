# Research Document: Issue #623 - Exclude Unused Haystack Providers

**Status**: Approved
**Author**: Claude Code
**Date**: 2026-03-11
**Reviewers**: Engineering Team

## Executive Summary

Issue #623 requests excluding unused haystack providers from workspace builds. Research found that the requested exclusions (haystack_atlassian, haystack_discourse, haystack_grepapp) are ALREADY in the workspace exclude list. The remaining work is to clean up commented-out haystack dependencies in terraphim_middleware/Cargo.toml.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Cleanup reduces confusion and build complexity |
| Leverages strengths? | Yes | Straightforward maintenance task |
| Meets real need? | Yes | Issue #623 explicitly requests this |

**Proceed**: Yes - 3/3 YES

---

## Problem Statement

### Description
Clean up unused haystack provider dependencies and ensure they're excluded from workspace builds.

### Impact
Developers may be confused by commented-out code and unused dependencies in the codebase.

### Success Criteria
1. Unused haystack providers excluded from workspace builds
2. Commented-out haystack deps cleaned up in terraphim_middleware
3. `cargo check --workspace` passes
4. `cargo test --workspace` passes

---

## Current State Analysis

### Existing Implementation

**Root Cargo.toml exclude list** (already contains):
```toml
exclude = [
    # ... other excludes ...
    # Unused haystack providers (kept for future integration)
    "crates/haystack_atlassian",
    "crates/haystack_discourse",
    "crates/haystack_grepapp",
    # ... other excludes ...
]
```

**terraphim_middleware/Cargo.toml** has commented-out deps:
```toml
# grepapp_haystack = { path = "../haystack_grepapp", version = "1.0.0", optional = true }
```

And commented-out features:
```toml
# NOTE: atomic and grepapp features disabled for crates.io publishing
grepapp = []
```

### Code Locations

| Component | Location | Status |
|-----------|----------|--------|
| haystack_atlassian | crates/haystack_atlassian/ | Already excluded |
| haystack_discourse | crates/haystack_discourse/ | Already excluded |
| haystack_grepapp | crates/haystack_grepapp/ | Already excluded |
| haystack_jmap | crates/haystack_jmap/ | **ACTIVE - used by middleware** |
| terraphim_middleware | crates/terraphim_middleware/Cargo.toml | Has commented-out grepapp dep |

### Dependencies

**Active haystack dependencies:**
- `haystack_jmap` - actively used in terraphim_middleware/src/lib.rs
- `haystack_core` - actively used (dependency of haystack_jmap)

**Note**: haystack_jmap should NOT be excluded as it's actively used for email integration.

---

## Constraints

### Technical Constraints
- haystack_jmap must remain in workspace (actively used)
- Directory structure should be preserved for future use
- Commented code should be removed, not just disabled

### Business Constraints
- No breaking changes to active functionality
- Keep excluded crates for future integration

---

## Vital Few

### Essential Constraints
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Preserve haystack_jmap | Actively used for JMAP email | Used in terraphim_middleware |
| Keep directories | Future integration planned | Already excluded but preserved |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Adding haystack_jmap to exclude | It's actively used |
| Removing haystack directories | Keep for future use |

---

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking grepapp feature | Low | Low | grepapp already empty/no-op |

### Open Questions
1. Should we remove the grepapp feature entirely or keep it as placeholder? - Keep as placeholder per issue instructions

---

## Research Findings

### Key Insights
1. **Already Done**: haystack_atlassian, haystack_discourse, haystack_grepapp are already excluded
2. **Active**: haystack_jmap is actively used and should NOT be excluded
3. **Cleanup Needed**: Remove commented-out grepapp_haystack dependency in terraphim_middleware

### Relevant Prior Art
- Root Cargo.toml already has proper exclude structure
- grepapp feature is already a no-op (empty implementation)

---

## Recommendations

### Proceed/No-Proceed
**Proceed** - Cleanup is straightforward and reduces confusion.

### Scope
1. Remove commented-out `grepapp_haystack` dependency from terraphim_middleware/Cargo.toml
2. Keep grepapp feature as placeholder (per issue instructions)
3. Verify workspace builds and tests pass

### Risk Mitigation
- Run full workspace check before committing
- Verify haystack_jmap still builds correctly

---

## Next Steps

1. Create design document with implementation steps
2. Remove commented-out grepapp_haystack dependency
3. Run cargo check --workspace
4. Run cargo test --workspace
5. Commit and PR
