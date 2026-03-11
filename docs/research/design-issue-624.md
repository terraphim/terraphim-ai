# Implementation Plan: Issue #624 - Remove terraphim_repl, Consolidate CLIs

**Status**: Draft
**Research Doc**: docs/research/research-issue-624.md
**Author**: Claude Code
**Date**: 2026-03-11
**Estimated Effort**: 1 hour

## Overview

### Summary
Remove the terraphim_repl crate directory that is already excluded from the workspace. The REPL functionality is superseded by terraphim_agent with `--features repl-full`. Verify terraphim_agent and terraphim_cli still build correctly after removal.

### Approach
Delete the entire terraphim_repl directory and verify no documentation references remain. Run workspace builds and tests to confirm no breakage.

### Scope
**In Scope:**
- Delete crates/terraphim_repl/ directory
- Search for and update any documentation references
- Verify workspace builds without it

**Out of Scope:**
- Changes to terraphim_agent (already provides REPL)
- Changes to terraphim_cli
- Nested terraphim_settings cleanup (none found)

**Avoid At All Cost:**
- Partial deletion (must delete entire directory)
- Skipping verification steps
- Changes to working REPL functionality in terraphim_agent

## Architecture

No architectural changes. This removes a superseded crate.

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Delete entire directory | Already excluded, no longer needed | Partial cleanup |
| Verify terraphim_agent REPL | Ensure replacement works | Assuming it works |

### Simplicity Check
**What if this could be easy?** Delete directory, run tests, verify builds. This is straightforward cleanup.

## File Changes

### Modified Files
None.

### Deleted Files/Directories
| File/Directory | Reason |
|----------------|--------|
| `crates/terraphim_repl/` | Superseded by terraphim_agent --features repl-full |
| `crates/terraphim_repl/Cargo.toml` | Part of removed crate |
| `crates/terraphim_repl/src/main.rs` | Part of removed crate |
| `crates/terraphim_repl/README.md` | Part of removed crate |
| `crates/terraphim_repl/CHANGELOG.md` | Part of removed crate |
| `crates/terraphim_repl/assets/` | Part of removed crate |
| `crates/terraphim_repl/tests/` | Part of removed crate |

## Test Strategy

### Verification Steps
| Step | Command | Expected Result |
|------|---------|-----------------|
| 1 | `cargo check --workspace` | Clean build, no errors |
| 2 | `cargo test --workspace` | All tests pass |
| 3 | `cargo build -p terraphim_agent --features repl-full` | REPL agent builds successfully |
| 4 | `cargo build -p terraphim_cli` | CLI builds successfully |
| 5 | Search docs for references | No broken references |

## Implementation Steps

### Step 1: Check for Documentation References
**Command:** `grep -r "terraphim_repl" --include="*.md" --include="*.toml" .`
**Purpose:** Find any references that need updating before deletion
**Estimated:** 5 minutes

### Step 2: Delete terraphim_repl Directory
**Command:** `rm -rf crates/terraphim_repl/`
**Purpose:** Remove the superseded crate entirely
**Estimated:** 1 minute

### Step 3: Verify Workspace Check
**Command:** `cargo check --workspace`
**Expected:** Clean build with no errors
**Estimated:** 2 minutes

### Step 4: Run Workspace Tests
**Command:** `cargo test --workspace`
**Expected:** All tests pass
**Estimated:** 10 minutes

### Step 5: Verify terraphim_agent REPL Builds
**Command:** `cargo build -p terraphim_agent --features repl-full`
**Expected:** Successful build
**Estimated:** 3 minutes

### Step 6: Verify terraphim_cli Builds
**Command:** `cargo build -p terraphim_cli`
**Expected:** Successful build
**Estimated:** 3 minutes

## Rollback Plan

If issues discovered:
1. Restore terraphim_repl directory from git: `git checkout HEAD -- crates/terraphim_repl/`
2. Verify workspace builds
3. Investigate and fix any issues before re-attempting

## Dependencies

No dependency changes. The crate was already excluded from the workspace.

## Open Items

None.

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
