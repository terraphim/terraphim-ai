# Handover: 2026-06-03 -- Echo / implementation-swarm-A: #2049 shared-learning promote guard

**Branch**: task/2049-shared-learning-promote-guard
**Base**: origin/main (GitHub monorepo, 58 crates, commit 98fa93b32)
**Commit**: c3be79ead
**GitHub PR**: https://github.com/terraphim/terraphim-ai/pull/897
**Gitea Issue**: #2049
**Gitea Comment**: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/2049#issuecomment-36299

## Session Summary

Implemented fix for issue #2049: `learn shared promote <id> --to l2` from L0 silently no-opped and reported false success. Root cause: `SharedLearningStore::promote_to_l2` called `SharedLearning::promote_to_l2()` without checking the current trust level. `SharedLearning::promote_to_l2()` only promotes from L1 (guards with `if trust_level == L1`) — when called on L0, it was a silent no-op, but the store returned `Ok(())` and the CLI printed "Promoted".

## What Was Done

### Fix (3 files changed, 113 insertions, 6 deletions)

**`crates/terraphim_agent/src/shared_learning/store.rs`**
- `promote_to_l1`: added guard — returns `StoreError::InvalidInput` if `trust_level != TrustLevel::L0`
- `promote_to_l2`: added guard — returns `StoreError::InvalidInput("Learning {id} is at {level}; promote to L1 first using --to l1")` if `trust_level != TrustLevel::L1`

**`crates/terraphim_agent/src/main.rs`**
- `TrustLevel::L1` CLI branch: now calls `store.promote_to_l1(&id).await` and prints success, instead of returning a static error with wrong message ("learnings start at L1")

**`crates/terraphim_agent/tests/shared_learning_cli_tests.rs`**
- Fixed 5 pre-existing broken tests that called `promote_to_l2` from L0 (masked the root-cause bug)
- Fixed `shared_import_creates_l1_entries` assertion from `TrustLevel::L1` to `TrustLevel::L0` (import does not promote)
- Added 3 regression tests: `promote_l0_directly_to_l2_fails`, `promote_l0_to_l1_via_store_succeeds`, `promote_l1_to_l1_again_fails`

### Results

- `cargo test -p terraphim_agent --test shared_learning_cli_tests --features shared-learning`: 12/12 PASS
- `cargo clippy -p terraphim_agent --features shared-learning -- -D warnings`: CLEAN

## Critical Lessons Learned

- **gitea/main vs origin/main divergence**: This worktree uses `gitea/main` (polyrepo, 15 crates). Issues referencing `terraphim_types`, `terraphim_agent`, etc. require `origin/main` (GitHub, 58 crates). Always branch from `origin/main` for issues that reference these crates.
- **TrustLevel Display**: `TrustLevel::L0` displays as `"Extracted"` (not `"L0"`). When asserting on error messages that include `{trust_level}`, check for `"Extracted"` or `"L0"` (both), not just `"L0"`.
- **Store guard location**: Add guards to STORE methods (not struct methods). The struct's `promote_to_l2()` is also called in the auto-promote path (`record_effective`), which already checks `trust_level == TrustLevel::L1` before calling it. Guarding the struct method would break nothing, but guarding the store method is the right boundary.
- **Wiki REST API returns 405**: Skip wiki creation, use Gitea issue comment for session record.

## What Is Next

- The GitHub PR #897 is open for review and merge
- Gitea issue #2049 remains open (close on merge per workflow)
- Related issue #2046 ("5 SharedLearningStore tests fail") is effectively resolved by this PR — the 5 tests are now fixed. Consider closing #2046 after #2049 merges.
- The `terraphim_orchestrator/src/lib.rs` format drift (issue #2044) is pre-existing and out of scope for this PR

## State

Branch is pushed. PR is created. Issue has handover comment. Working tree is clean. No uncommitted work.
