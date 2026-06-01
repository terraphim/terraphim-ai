# Implementation Plan: Merge Remaining PR Backlog

**Status**: Draft (Pending Human Approval)
**Author**: Root (orchestrator agent)
**Date**: 2026-06-01
**Based on**: Research Document `.docs/research-remaining-prs.md`

## Overview

### Summary
Close 6 stale PRs that are already implemented on main, then investigate and fix 3 remaining real issues.

### Approach
Surgical cleanup: close stale PRs with documentation, verify remaining issues, create fresh focused PRs.

### Scope
**In Scope:**
- Close 6 stale PRs with explanatory comments
- Investigate 3 remaining issues
- Create fresh PRs for confirmed issues

**Out of Scope:**
- Rebase existing branches (all stale)
- Bulk changes (one issue per PR)
- New feature development

**Avoid At All Cost:**
- Re-creating stale branches
- Combining multiple issues into one PR
- Implementing without verifying issue still exists

## Phase 1: Close Stale PRs (Immediate)

### PRs to Close

| PR | Issue | Close Reason | Evidence |
|----|-------|--------------|----------|
| #1615 | #821 | `applied_by` parameter already in `SharedLearningStore` trait and all implementations | `terraphim_types/src/shared_learning.rs:139-140` |
| #1604 | #1577 | `MatrixConfig` with `params`, `max_parallel`, `fail_strategy` already exists | `terraphim_orchestrator/src/flow/config.rs:28-40` |
| #1600 | #842 | `ResponseMeta.query` and `ResponseMeta.role` fields with builders already exist | `terraphim_agent/src/robot/schema.rs:74-123` |
| #1491 | #1488 | All RLM executors (`Local`, `Docker`, `Firecracker`, `SSH`) already implemented | `terraphim_rlm/src/executor/*.rs` |
| #1599 | #1572 | Context rot detection superseded by commit `e84f9214` | CHANGELOG + code |
| #1365 | #1362 | Rustdoc CI gate superseded by commit `d1f2c767` | CHANGELOG + CI workflow |

### Close Comment Template
```
Closing as superseded. [Feature] has been [implemented/merged] on main.

[Specific evidence of implementation]

See CHANGELOG for details.

Refs #[issue_number]
```

## Phase 2: Investigate Remaining Issues

### Issue #1367 / #1358: Test Role Names

**Current State**: 55 references to `"test_agent"` string literal in test code
**Question**: Does this cause test failures or is it just inconsistent naming?

**Investigation Steps**:
1. Run tests that reference `"test_agent"` to see if they fail
2. Check if any test expects `"Terraphim Engineer"` role specifically
3. Determine scope: rename all test agents or only specific ones

**Expected Outcome**: Either close (no failures) or create fresh PR (rename needed)

### Issue #1514 / #1299: Strict Permissions Flag

**Current State**: `PermissionCheck` exists in `commands/validator.rs` but no `--strict-permissions` CLI flag
**Question**: Is the flag missing or implemented under a different name?

**Investigation Steps**:
1. Search for `strict` and `permission` in `adf-ctl.rs` CLI definitions
2. Check if permission checking is enabled by default or behind a feature flag
3. Review issue #1297 for original requirements

**Expected Outcome**: Either close (already implemented) or create fresh PR (add flag)

### Issue #1308 / #1297: Spec Gaps

**Current State**: Title is vague - "close persistent spec gaps"
**Question**: What specific spec gaps remain unfixed?

**Investigation Steps**:
1. Read issue #1297 description for specific gaps
2. Cross-reference with current spec documents in `docs/specifications/`
3. Verify which gaps are already addressed by PR #1963 (ADF behaviour specs)

**Expected Outcome**: Either close (gaps addressed) or create issue with specific remaining gaps

## Phase 3: Fresh Implementation (If Needed)

### Test Role Names Fix (if confirmed)

**Files to modify**:
- Search results from `grep -r '"test_agent"' crates/ --include="*.rs"`

**Changes**:
```rust
// BEFORE
let agent = Agent::new("test_agent".to_string());

// AFTER  
let agent = Agent::new("test_user".to_string());
```

**Test strategy**:
1. Run `cargo test` to verify no failures
2. Run `cargo test --workspace` for full verification

**Estimated Effort**: 30 minutes

### Strict Permissions Flag (if confirmed)

**Files to modify**:
- `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` - Add CLI flag
- `crates/terraphim_orchestrator/src/config.rs` - Add config field
- `crates/terraphim_agent/src/commands/validator.rs` - Wire up flag

**Changes**:
```rust
// In adf-ctl.rs CLI definition
#[arg(long, help = "Enforce strict file permissions (0600) on config files")]
strict_permissions: bool,
```

**Test strategy**:
1. Unit test: flag parsing
2. Integration test: permission check with flag enabled

**Estimated Effort**: 2 hours

## Rollback Plan

If issues discovered during investigation:
1. Document findings in issue comments
2. Close PR if issue already fixed
3. Create new focused issue if new problem found

## Implementation Steps

### Step 1: Close Stale PRs
**Files**: None (Gitea operations only)
**Description**: Post close comments on 6 stale PRs
**Estimated**: 15 minutes

### Step 2: Investigate Test Role Names
**Files**: TBD based on grep results
**Description**: Run tests and determine if rename needed
**Estimated**: 30 minutes

### Step 3: Investigate Permissions Flag
**Files**: `adf-ctl.rs`, `config.rs`
**Description**: Search for existing flag implementation
**Estimated**: 30 minutes

### Step 4: Investigate Spec Gaps
**Files**: `docs/specifications/`, issue #1297
**Description**: Read issue and cross-reference with existing specs
**Estimated**: 30 minutes

### Step 5: Create Fresh PRs (if needed)
**Files**: TBD from investigation
**Description**: Implement confirmed fixes
**Estimated**: 1-3 hours per fix

## Approval

- [ ] Close 6 stale PRs
- [ ] Investigate 3 remaining issues
- [ ] Create fresh PRs for confirmed issues
- [ ] Human approval received

## Next Steps After Approval

1. Execute Phase 1 (close stale PRs)
2. Execute Phase 2 (investigate remaining issues)
3. Execute Phase 3 (create fresh PRs if needed)
