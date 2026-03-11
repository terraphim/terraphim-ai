# Implementation Plan: PR #652 Merge Conflict Resolution

**Status**: Draft
**Research Doc**: [.docs/research-pr-652-agent-workflows-e2e.md](research-pr-652-agent-workflows-e2e.md)
**Author**: Claude Code (Terraphim AI)
**Date**: 2026-03-10
**Estimated Effort**: 30 minutes

## Overview

### Summary
Resolve merge conflicts in PR #652 (agent-workflows E2E) with main branch. Two files have conflicts that need manual resolution while preserving both clippy compliance and E2E functionality.

### Approach
Minimal conflict resolution strategy:
1. Accept our doc comment formatting (fixes clippy warnings)
2. Accept their implementation logic (E2E functionality)
3. Verify no regressions

### Scope

**In Scope:**
- Resolve `terraphim_server/src/workflows/multi_agent_handlers.rs` conflict
- Resolve `examples/agent-workflows/3-parallelization/app.js` conflict
- Verify clippy passes
- Verify merge is clean

**Out of Scope:**
- Code refactoring beyond conflict resolution
- Adding new tests
- Modifying other files

**Avoid At All Cost:**
- Rewriting the ServiceTargetResolver logic
- Consolidating helper functions (separate PR)
- Changing test timeouts or configuration

## Architecture

### Conflict Resolution Strategy

```
main branch (ours)          PR #652 (theirs)            Result
-------------------         -----------------           ------
Proper doc comments    +    Helper functions      =    Both
(clippy-compliant)          (poor formatting)          (best of both)

Original parse logic   +    parseLlmResponse      =    Their implementation
                            (real LLM parsing)         (E2E functionality)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Keep our doc formatting | Fixes clippy `doc_lazy_continuation` warnings | Taking their formatting would break CI |
| Keep their parseLlmResponse | Adds real LLM response parsing | Keeping our version would lose E2E functionality |
| No additional refactoring | Keep PR focused, reduce risk | Refactoring helpers would expand scope |

### Simplicity Check

**What if this could be easy?**
The simplest approach is to merge the two versions manually, taking the best from each. No redesign, no refactoring - just clean conflict resolution.

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `terraphim_server/src/workflows/multi_agent_handlers.rs` | Merge doc comments from main with helper functions from PR |
| `examples/agent-workflows/3-parallelization/app.js` | Take PR's parseLlmResponse implementation |

## Implementation Steps

### Step 1: Pre-Merge Verification
**Files:** None (verification only)
**Description:** Verify current state before making changes
**Estimated:** 2 minutes

```bash
# Verify we're on main and up to date
git checkout main
git pull upstream main

# Check PR branch state
git fetch upstream feat/agent-workflows-e2e

# Verify conflicts exist
git merge-tree $(git merge-base upstream/main upstream/feat/agent-workflows-e2e) upstream/main upstream/feat/agent-workflows-e2e
```

**Verification Gate:**
- [ ] Main is up to date
- [ ] Conflicts confirmed in 2 files only

### Step 2: Resolve handlers.rs Conflict
**Files:** `terraphim_server/src/workflows/multi_agent_handlers.rs`
**Description:** Merge helper functions with proper doc formatting
**Dependencies:** Step 1
**Estimated:** 10 minutes

**Conflict Details:**
- Lines 67-70: Our version has blank line after "4. Hardcoded fallback"
- Lines 71-73: Our version has blank line after "Get a string value..."
- Lines 94-99: Our version has full doc comment for resolve_llm_config

**Resolution:**
```rust
// Keep our doc comment formatting (with blank lines)
/// 4. Hardcoded fallback (lowest priority)
///
/// Get a string value from Role.extra, checking both flat and nested paths.
///
/// Due to `#[serde(flatten)]` on `Role.extra`...

// Keep their helper functions
fn get_role_extra_str<'a>(...) -> Option<&'a str> { ... }
fn get_role_extra_f64(...) -> Option<f64> { ... }

// Keep our doc comment for resolve_llm_config
/// Resolve LLM configuration from multiple sources with priority order:
///
/// 1. Request-level config...
fn resolve_llm_config(...) -> LlmConfig { ... }
```

**Verification Gate:**
- [ ] File compiles: `cargo check -p terraphim_server`
- [ ] Clippy passes: `cargo clippy -p terraphim_server --all-targets -- -D warnings`

### Step 3: Resolve app.js Conflict
**Files:** `examples/agent-workflows/3-parallelization/app.js`
**Description:** Take PR's parseLlmResponse implementation
**Dependencies:** Step 2
**Estimated:** 10 minutes

**Conflict Details:**
- Around line 502: Different implementations of extractPerspectiveAnalysis
- PR adds parseLlmResponse method with markdown parsing
- PR adds fallback handling for missing LLM results

**Resolution:**
Accept their implementation entirely - it adds:
- `parseLlmResponse(perspective, text)` method
- Fallback when no LLM result available
- Better markdown parsing for key points, insights, recommendations

**Verification Gate:**
- [ ] JavaScript syntax valid
- [ ] No obvious logic errors

### Step 4: Post-Merge Verification
**Files:** All
**Description:** Verify merge is clean and all checks pass
**Dependencies:** Steps 2-3
**Estimated:** 5 minutes

```bash
# Full workspace check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib

# Verify no remaining conflicts
grep -r "<<<<<<<" --include="*.rs" --include="*.js" . || echo "No conflicts found"
```

**Verification Gate:**
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] No conflict markers remaining

### Step 5: Commit and Push
**Files:** All modified
**Description:** Commit resolved conflicts and push
**Dependencies:** Step 4
**Estimated:** 3 minutes

```bash
git add -A
git commit -m "resolve merge conflicts with main

- Keep clippy-compliant doc comments in multi_agent_handlers.rs
- Keep helper functions (get_role_extra_str, get_role_extra_f64)
- Accept parseLlmResponse implementation in app.js
- Preserve E2E functionality while maintaining code quality"

git push upstream feat/agent-workflows-e2e
```

**Verification Gate:**
- [ ] Push succeeds
- [ ] CI triggers

### Step 6: CI Verification
**Files:** None (monitoring)
**Description:** Monitor CI for any issues
**Dependencies:** Step 5
**Estimated:** 5 minutes (waiting)

```bash
gh pr checks 652 --repo terraphim/terraphim-ai --watch
```

**Verification Gate:**
- [ ] Rust Format Check passes
- [ ] Rust Clippy passes
- [ ] Rust Compilation Check passes
- [ ] Rust Unit Tests pass

## Rollback Plan

If issues discovered:
1. `git reset --hard HEAD~1` (undo merge commit)
2. `git push upstream feat/agent-workflows-e2e --force-with-lease`
3. Re-analyze conflicts

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Conflict resolution | Pending | Claude |
| CI verification | Pending | GitHub Actions |

## Approval

- [ ] Research document reviewed
- [ ] Implementation plan approved
- [ ] Ready to execute

## Execution Commands

Quick reference for execution:

```bash
# Step 1: Pre-merge verification
git checkout main && git pull upstream main
git fetch upstream feat/agent-workflows-e2e

# Step 2-3: Resolve conflicts (manual editing)
# Edit terraphim_server/src/workflows/multi_agent_handlers.rs
# Edit examples/agent-workflows/3-parallelization/app.js

# Step 4: Verification
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib

# Step 5: Commit and push
git add -A
git commit -m "resolve merge conflicts with main"
git push upstream feat/agent-workflows-e2e

# Step 6: Monitor CI
gh pr checks 652 --repo terraphim/terraphim-ai --watch
```
