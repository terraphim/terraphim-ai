# Implementation Plan: Issue #623 - Exclude Unused Haystack Providers

**Status**: Draft
**Research Doc**: docs/research/research-issue-623.md
**Author**: Claude Code
**Date**: 2026-03-11
**Estimated Effort**: 30 minutes

## Overview

### Summary
Clean up commented-out haystack provider dependencies in terraphim_middleware/Cargo.toml. The haystack providers (atlassian, discourse, grepapp) are already excluded from the workspace; this completes the cleanup by removing the dead code.

### Approach
Single-file edit to remove the commented-out grepapp_haystack dependency line while preserving the placeholder feature flag per issue instructions.

### Scope
**In Scope:**
- Remove commented-out grepapp_haystack dependency from terraphim_middleware/Cargo.toml

**Out of Scope:**
- Removing haystack directories (kept for future use)
- Modifying .release-plz.toml (grepapp not present)
- Changes to active haystack_jmap (actively used)

**Avoid At All Cost:**
- Removing the grepapp feature placeholder (issue says to keep it)
- Any changes beyond the single commented line

## Architecture

No architectural changes. This is a cleanup task removing dead code.

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Keep grepapp feature placeholder | Per issue instructions | Removing feature entirely |
| Remove only grepapp_haystack line | Minimal change principle | Major Cargo.toml restructure |

### Simplicity Check
**What if this could be easy?** This IS easy - delete one commented line.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_middleware/Cargo.toml` | Remove commented grepapp_haystack dependency line |

### Deleted Files
None.

## Test Strategy

### Verification Steps
| Step | Command | Expected Result |
|------|---------|-----------------|
| 1 | `cargo check --workspace` | Clean build, no errors |
| 2 | `cargo test --workspace` | All tests pass |
| 3 | `cargo build -p terraphim_middleware` | Middleware builds successfully |
| 4 | `cargo build -p terraphim_middleware --features grepapp` | Placeholder feature works |

## Implementation Steps

### Step 1: Remove Commented Dependency
**Files:** `crates/terraphim_middleware/Cargo.toml`
**Description:** Remove the commented-out grepapp_haystack dependency line
**Changes:**
- Delete line 24-25: `# NOTE: grepapp_haystack commented out for crates.io publishing (not published yet)` and `# grepapp_haystack = { path = "../haystack_grepapp", version = "1.0.0", optional = true }`

**Estimated:** 5 minutes

### Step 2: Verify Build
**Command:** `cargo check --workspace`
**Expected:** Clean build with no errors

**Estimated:** 2 minutes

### Step 3: Run Tests
**Command:** `cargo test --workspace`
**Expected:** All tests pass

**Estimated:** 5 minutes

### Step 4: Verify Middleware Builds
**Command:** `cargo build -p terraphim_middleware --features grepapp`
**Expected:** Builds successfully (placeholder feature still works)

**Estimated:** 3 minutes

## Rollback Plan

If issues discovered:
1. Revert the single line deletion in terraphim_middleware/Cargo.toml
2. Rebuild and verify

## Dependencies

No dependency changes.

## Open Items

None.

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
