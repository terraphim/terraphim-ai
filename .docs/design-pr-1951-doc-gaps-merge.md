# Implementation Plan: Merge PR #1951 Doc Fixes into Main

**Status**: Draft
**Research Doc**: `.docs/research-pr-1951-doc-gaps-merge.md`
**Author**: Opencode (GLM-5.1)
**Date**: 2026-06-01
**Estimated Effort**: 2-3 hours

## Overview

### Summary
Cherry-pick 19 doc-comment commits from PR #1951 onto a fresh branch from main, resolving conflicts and verifying zero doc warnings.

### Approach
Sequential cherry-pick of each `docs(...)` commit, conflict resolution, validation.

### Scope
**In Scope:**
- All 19 doc-comment commits from `gitea/task/doc-gaps-2026-06-01`
- Conflict resolution for 3-7 files
- Verification: `cargo doc`, `cargo test`, `cargo clippy`, `cargo fmt`

**Out of Scope:**
- Non-doc code changes from PR #1951 (guard.rs removal, config refactoring, etc.)
- Release tagging (PR #1968 handled separately)

**Avoid At All Cost:**
- Full merge of PR #1951 (brings undesired code changes)
- Rewriting main history

## File Changes

### Modified Files (Expected)
| File | Nature | Conflict Likelihood |
|------|--------|-------------------|
| `crates/terraphim_persistence/src/lib.rs` | Doc additions | High |
| `crates/terraphim_rolegraph/src/lib.rs` | Doc additions | High |
| `crates/terraphim_agent/src/service.rs` | Doc additions | Low |
| `crates/terraphim_agent/src/robot/*.rs` | Doc additions | Low |
| `crates/terraphim_agent_evolution/src/*.rs` | Doc additions | Medium |
| `crates/terraphim_orchestrator/src/*.rs` | Doc additions | Medium |
| `crates/terraphim_rlm/src/*.rs` | Doc additions | Low |
| `crates/terraphim_service/src/*.rs` | Doc additions | Low |
| `crates/terraphim_grep/src/*.rs` | Doc additions | Low |
| `crates/terraphim_middleware/src/*.rs` | Doc additions | Low |
| `crates/terraphim_workspace/src/lib.rs` | Doc additions | Low |
| `crates/terraphim_sessions/src/model.rs` | Doc additions | Low |
| `crates/terraphim_settings/src/lib.rs` | Doc additions | Low |
| `crates/terraphim_validation/src/*.rs` | Doc additions | Low |
| `crates/terraphim_config/src/*.rs` | Doc additions | Medium |
| `crates/terraphim_types/src/lib.rs` | Doc additions | High |
| `crates/terraphim_agent_messaging/src/*.rs` | Doc additions | Low |
| `crates/terraphim_agent_registry/src/*.rs` | Doc additions | Low |
| `crates/terraphim_agent_supervisor/src/*.rs` | Doc additions | Low |

## Implementation Steps

### Step 0: Merge PR #1968 (Release)
**Description:** Merge the release PR first to mark v2026.05.31
**Command:** `gtr merge-pull --owner terraphim --repo terraphim-ai --index 1968`
**Estimated:** 2 minutes

### Step 1: Create Feature Branch
**Description:** Fresh branch from main
**Commands:**
```bash
git checkout main
git pull origin main
git checkout -b task/1951-doc-gaps-cherry-pick
```
**Estimated:** 1 minute

### Step 2: Cherry-Pick Doc Commits (Sequential)
**Description:** Cherry-pick each of the 19 doc commits in chronological order
**Commits (oldest first):**
```
27cdc798d docs(terraphim_sessions): add missing doc comments to model structs
b09660d40 docs(terraphim_workspace): add missing doc comments to public API
05b7ff8e9 docs(terraphim_rlm): add missing doc comments to MCP tool structs
c2b0de56a docs(terraphim_grep): add missing doc comments to public API
efdd110c7 docs(terraphim_config): add missing doc comments to public API
4b9a2f75b docs(terraphim_persistence): fix missing docs and intra-doc link warnings
310e7c72e docs(terraphim_middleware): add missing doc comments to public API
b2d5ef411 docs(terraphim_rolegraph): fix missing docs and intra-doc link warnings
6ede7c70b docs(terraphim_agent_supervisor): add missing doc comments
5ae10ea43 docs(terraphim_service): add missing doc comments to public API
3c7ea033a docs(terraphim_agent_messaging): add missing doc comments
42ef3fa69 docs(terraphim_agent_registry): add missing doc comments
32f95470a docs(terraphim_agent_evolution): add missing doc comments phase 1
fa07272e5 docs(terraphim_agent): add missing doc comments to service and robot modules
ef1a4a921 docs(terraphim_settings): fix missing doc comments
b6aedd304 docs(terraphim_validation): add missing doc comments to public API
8afceba91 docs(terraphim_orchestrator): add missing doc comments phase 1
b84d0bd1f docs(terraphim_orchestrator): add missing doc comments phase 2
6b3792098 docs: correct CHANGELOG entry for PR #1943
```

**Conflict Resolution Strategy:**
- For each conflict: keep main's code, add the doc comments from the cherry-pick
- Skip commits that fail with irreconcilable conflicts and note for manual follow-up
- Run `cargo check` after each cherry-pick to catch issues early

**Estimated:** 60-90 minutes

### Step 3: Verify Doc Warnings
**Description:** Run cargo doc to confirm zero missing-doc warnings
**Command:** `cargo doc --workspace 2>&1 | grep -c 'warning: missing documentation'`
**Expected:** 0
**Estimated:** 10 minutes

### Step 4: Run Quality Gates
**Description:** Full test + lint suite
**Commands:**
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
**Estimated:** 30-60 minutes (on bigbox)

### Step 5: Commit, Push, PR
**Description:** Push branch and create PR
**Commands:**
```bash
git push origin task/1951-doc-gaps-cherry-pick
gtr create-pull --owner terraphim --repo terraphim-ai \
  --title "docs: cherry-pick PR #1951 doc-comment additions across 17 crates" \
  --head task/1951-doc-gaps-cherry-pick \
  --body "Cherry-picks the 19 doc-comment commits from closed PR #1951..."
```
**Estimated:** 5 minutes

## Rollback Plan
- Delete branch if cherry-picks cause unresolvable conflicts
- Fall back to manual doc-comment additions per crate
- Original branch preserved at `gitea/task/doc-gaps-2026-06-01`

## Approval
- [ ] Research document reviewed
- [ ] Implementation plan approved
- [ ] Human confirms non-doc changes should be excluded
