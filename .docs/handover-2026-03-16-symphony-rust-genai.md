# Handover: Symphony Orchestration for rust-genai Fork Sync

**Date**: 2026-03-16
**Session duration**: ~2 hours
**Status**: COMPLETE -- all 10 issues closed, verified

## Progress Summary

### Completed

1. **Created Gitea repo** `terraphim/rust-genai` at `git.terraphim.cloud` and pushed fork main
2. **Created 10 issues** on Gitea with full descriptions covering upstream merge, adapter updates, new features, tests, and validation
3. **Added 16 dependency edges** via `gitea-robot add-dep` forming a proper DAG
4. **Created WORKFLOW-rust-genai.md** adapted from the proven tlaplus-ts WORKFLOW for Rust (cargo build/test/clippy/fmt quality gate, 600s timeout, `-X theirs` merge strategy)
5. **Deployed and ran Symphony** on bigbox -- all 10 issues dispatched, agents completed, merged to main, issues closed
6. **Verified final result** -- fresh clone passes all quality gates (107 tests pass, 0 failures, clean clippy, clean fmt)

### Dispatch Waves Observed

| Wave | Issues | Time |
|------|--------|------|
| 1 | #1 (merge), #3 (Cerebras), #5 (AuthData), #10 (Zai URL) -- concurrent | ~5 min |
| 2 | #4 (OpenRouter) | ~3 min |
| 3 | #2 (Bedrock) -- required 1 retry | ~8 min |
| 4 | #6 (Kimi), #8 (tests) -- concurrent | ~5 min |
| 5 | #7 (SSE) | ~3 min |
| 6 | #8 (manual close due to hook bug), #9 (final validation) | ~5 min |

Total orchestration: ~45 minutes including retries.

## Technical Context

### Repositories

| Repo | Location | State |
|------|----------|-------|
| terraphim-ai (primary) | `/Users/alex/projects/terraphim/terraphim-ai` | main, WORKFLOW file uncommitted |
| rust-genai (local) | `/Users/alex/projects/terraphim/rust-genai` | main, local behind gitea/main |
| rust-genai (Gitea) | `git.terraphim.cloud/terraphim/rust-genai` | main at `b76e5a0`, fully synced |
| rust-genai (GitHub) | `github.com/terraphim/rust-genai` | main at `1c62bfb`, NOT synced |

### Uncommitted Files in terraphim-ai

- `crates/terraphim_symphony/examples/WORKFLOW-rust-genai.md` -- the Symphony WORKFLOW for rust-genai

### Key Artefact

**WORKFLOW-rust-genai.md** (`crates/terraphim_symphony/examples/WORKFLOW-rust-genai.md`):
- Tracker: Gitea `terraphim/rust-genai`
- Agent: claude-code, max 2 concurrent, 50 turns
- Quality gate: `cargo build && cargo test && cargo clippy -- -D warnings && cargo fmt -- --check`
- Hook timeout: 600000ms (10 min for Rust compilation)
- Merge strategy: `-X theirs` to auto-resolve CLAUDE.md conflicts
- CLAUDE.md tracked in git per user preference

## Issues Encountered and Fixed

### 1. Symphony positional argument
**Problem**: Used `symphony run --workflow <path>` but correct syntax is `symphony <path>` (positional)
**Fix**: Removed `run --workflow` flag

### 2. before_run branch already exists
**Problem**: On retry, `git checkout -b "$BRANCH"` fails because local branch exists from previous run
**Fix**: Added `git checkout main && git branch -D "$BRANCH"` before branch creation

### 3. CLAUDE.md merge conflicts blocking merge-to-main
**Problem**: Each agent writes a different CLAUDE.md; when merging branch to main, git conflicts on CLAUDE.md
**Fix**: Used `git merge -X theirs` in both before_run fallback and after_run merge step

### 4. Untracked CLAUDE.md blocking branch checkout
**Problem**: after_create wrote CLAUDE.md but didn't commit it; before_run's `git checkout` refused because local changes would be overwritten
**Fix**: Added `git add CLAUDE.md && git commit` at end of after_create

### 5. Issue #8 merge-to-main loop
**Problem**: Quality gate passed but curl to close issue timed out, triggering fallback "merge failed" message; issue stayed open; Symphony re-dispatched forever
**Fix**: Manual merge-and-close via Gitea API. Root cause: the `||` error handling in after_run treats any failure in the `git merge && git push && curl close && curl comment` chain as merge failure

## Pending / Follow-up

1. **Push Gitea main to GitHub origin**: `cd ~/projects/terraphim/rust-genai && git pull gitea main && git push origin main` -- the GitHub fork is still at the pre-sync state
2. **Commit WORKFLOW-rust-genai.md**: The WORKFLOW file in terraphim-ai is uncommitted
3. **Fix after_run hook robustness**: The curl timeout issue (#5 above) should be addressed -- separate the merge+push from the issue-close curl, and handle curl failures independently
4. **Clean up symphony branches**: Remote branches `symphony/issue-{2,6,7,8,9}` remain on Gitea after merge; could be cleaned up
5. **CLAUDE.md on main**: Contains agent-specific instructions rather than project-level learnings. Could be replaced with proper project CLAUDE.md

## Key Learnings for MEMORY.md

Already recorded in MEMORY.md:
- Symphony positional argument syntax
- Branch cleanup in before_run for retries
- `-X theirs` merge strategy for CLAUDE.md conflicts
- after_create must commit tracked files
- Rust quality gate with 600s timeout
- after_run curl timeout can cause infinite retry loops

## Environment State

- **bigbox**: Symphony stopped, tmux session `symphony-genai` killed
- **Gitea**: All 10 issues closed, main at `b76e5a0`
- **Local rust-genai**: main at `1c62bfb`, gitea/main at `b76e5a0` (needs pull)
- **GITEA_TOKEN**: `5d663368d955953ddf900ff33420fcabebfbfb4b` (also in MEMORY.md context)
