# Implementation Plan: Remote Reconciliation - GitHub/Gitea Sync & Issue Cleanup

**Status**: Updated (4th revision -- 2026-04-27 17:30 CEST, remotes stable)
**Research Doc**: `.docs/research-remote-reconciliation-2026-04-27.md`
**Author**: opencode (glm-5.1)
**Date**: 2026-04-27
**Estimated Effort**: 3-4 hours

## Overview

### Summary
Reconcile the three-way divergence between local main, origin/main, and gitea/main. Both remotes are stable (no movement since v2 fetch). Cherry-pick select content from `task/860-f1-2-exit-codes` (PR #869). Merge origin into local (brings v1.17.0). Push to both remotes. Clean up 71 stale local branches, ~12 stale PRs, ~55 duplicate issues.

### Approach
1. Cherry-pick from task/860-f1-2-exit-codes (select commits only)
2. Merge origin/main INTO local main (1 conflict: Cargo.lock)
3. Test, push to both remotes
4. Clean up branches, PRs, issues

### Key Changes from Previous Plans
- **v1**: Local as authoritative
- **v2**: Origin as authoritative (v1.17.0 arrived)
- **v3**: Added Step 0 for task/860 cherry-picks
- **v4**: Updated issue/PR counts (7 new issues #1015-#1021, 1 new PR #1021, 26 total PRs vs 20 previously tracked). Both remotes stable -- good reconciliation window.

### Scope

**In Scope:**
1. Selective cherry-pick from task/860 (Step 0)
2. Main branch alignment (merge origin, push to both)
3. Stale local branch deletion (71 branches)
4. Stale Gitea PR review/closure (~12 of 26)
5. Duplicate Gitea issue closure (~55 of 439)
6. Protocol documentation in AGENTS.md

**Out of Scope:**
- CI fix (#1005)
- Security findings (#1004)
- Spawner bug (#1020, #1021) -- active, separate
- ADF agent workflow redesign
- Documentation gap filling

**Avoid At All Cost:**
- Force-pushing to any remote
- Closing genuine issues/PRs without verification
- Losing the v1.17.0 release commit

### Simplicity Check

**What if this could be easy?** Both remotes are stable. Merge origin into local (1 Cargo.lock conflict), cherry-pick a few commits from task/860, push to both, batch-close issues via API.

**Senior Engineer Test**: This is cleanup work. Keep it mechanical.

## Architecture

### Reconciliation Flow

```
                    origin/main (faf5e7006) -- STABLE
                    v1.17.0 + cfg-gated tests
                           |
                    merge into local
                           |
local main (91331d4ee) ---+--- push to origin
   + cherry-picks from    |--- push to gitea
     task/860              |
                           v
                    unified main on both remotes
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Selective cherry-pick from task/860 | 38 conflicts on full merge (1791 commits) | Full merge (too risky) |
| Origin-first merge | Origin has v1.17.0, most advanced | Local-first (loses v1.17.0) |
| Merge not rebase | Safety guard blocks force-push | Rebase (blocked) |
| Batch close issues via API | 55 issues is too many to close manually | One-by-one (too slow) |

## Implementation Steps

### Step 0: Selective Merge from task/860-f1-2-exit-codes (PR #869)
**Description**: Cherry-pick valuable unique commits from the branch.
**Estimated**: 20-30 minutes

**Cherry-pick targets**:
1. `exit_codes_integration_test.rs` -- new file
2. Improved `exit_codes.rs` -- doc comments + `search_succeeds_exits_0`
3. Doc comment additions on public types (17 lines in main.rs)
4. `listen --identity` made optional
5. `classify_error` auth heuristic tightening

**Strategy**: Create temp branch, cherry-pick, test, merge to main.

```bash
# Identify commits
git log origin/main..origin/task/860-f1-2-exit-codes --oneline -- crates/terraphim_agent/tests/exit_codes_integration_test.rs
git log origin/main..origin/task/860-f1-2-exit-codes --oneline -- crates/terraphim_agent/tests/exit_codes.rs

# Cherry-pick
git checkout -b temp/860-cherry-picks
git cherry-pick <sha1> <sha2> ...
# Fallback: manual file copy if conflicts
cargo test -p terraphim_agent

# Merge back
git checkout main
git merge temp/860-cherry-picks --no-edit
git branch -d temp/860-cherry-picks
```

### Step 1: Safety Backup
**Estimated**: 2 minutes

```bash
git tag pre-reconciliation/local-main-$(date +%Y%m%d-%H%M) HEAD
git tag pre-reconciliation/origin-main-$(date +%Y%m%d-%H%M) origin/main
git tag pre-reconciliation/gitea-main-$(date +%Y%m%d-%H%M) gitea/main
```

### Step 2: Merge Origin Into Local
**Estimated**: 5 minutes

```bash
git merge origin/main --no-edit
# Resolve Cargo.lock conflict
git checkout --theirs Cargo.lock && cargo generate-lockfile && git add Cargo.lock
git commit --no-edit
```

### Step 3: Test After Merge
**Estimated**: 5 minutes

```bash
cargo build --workspace
cargo test -p terraphim_agent
```

### Step 4: Push to Origin
**Estimated**: 2 minutes

```bash
git push origin main
```

### Step 5: Push to Gitea
**Estimated**: 2 minutes

```bash
git push gitea main
```

### Step 6: Verify Convergence
**Estimated**: 2 minutes

```bash
git diff origin/main gitea/main --stat  # Should be empty
git log --oneline origin/main..HEAD     # Should be 0
git log --oneline gitea/main..HEAD      # Should be 0
```

### Step 7: Prune Stale Local Branches
**Estimated**: 5 minutes

```bash
git branch --merged main | grep -v '^\*' | grep -v '^  main$' | xargs git branch -d
```

### Step 8: Review and Close Stale Gitea PRs
**Estimated**: 15 minutes

**Close (superseded/stale)**:
| PR | Reason |
|----|--------|
| #997 | Superseded by origin commit faf5e7006 |
| #686 | Superseded by PR #1000 |
| #869 | Superseded after Step 0 cherry-picks |
| #830 | 3+ weeks old |
| #780 | 4+ days old |
| #777 | 4+ days old |
| #771 | 4+ days old |
| #757 | 5+ days old |
| #753 | 5+ days old |
| #749 | 5+ days old |
| #705 | 6+ days old |
| #667 | 6+ days old |

**Keep open (active)**:
| PR | Reason |
|----|--------|
| #1021 | Active spawner fix |
| #1000 | Active token budget |
| #999 | Active pr_dispatch refactor |
| #969 | Active F1.1 concepts_matched |
| #958 | Active Phase 2d compliance |
| #956 | Active Phase 2e test guardian |
| #952 | Active Phase 2b spec validator |
| #857 | May overlap with #969 -- review |
| #847 | May be merged already -- review |
| #660, #655, #640, #639, #636 | Previously hidden, need review |

**Note**: Before closing any PR, run `git diff main...<branch> --stat` to verify the branch has no unmerged unique content.

### Step 9: Close Duplicate Gitea Issues
**Estimated**: 30-45 minutes

**Batch closure script**:

```bash
source ~/.profile
close_dup() {
  local idx=$1 keeper=$2 reason=$3
  curl -s -X PATCH "$GITEA_URL/api/v1/repos/terraphim/terraphim-ai/issues/$idx" \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" -d '{"state":"closed"}' > /dev/null
  curl -s -X POST "$GITEA_URL/api/v1/repos/terraphim/terraphim-ai/issues/$idx/comments" \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"body\":\"Closing: $reason. Remote reconciliation 2026-04-27.\"}" > /dev/null
  echo "Closed #$idx"
}
```

**Group A: Already Completed** (5): #932, #933, #926, #927, #903
**Group B: Task 1.4 REPL** (3): #970, #972, #985 -- keep #994
**Group C: Task 1.5 Token Budget** (4): #971, #973, #977 -- keep #996; close #903
**Group D: Task 1.6 Tests** (3): #974, #976, #904 -- keep #995
**Group E: Robot JSON Contract** (4): #975, #980, #981, #992 -- keep #998
**Group F: Documentation Gaps** (4): #966, #1014, #1016, #931 -- keep #988
**Group G: Config Drift** (1): #989 -- keep #993
**Group H: Security** (2): #967, #941 -- keep #1004
**Group I: Flaky Tests** (3): #987, #935, #934 -- keep #997
**Group J: Zlob/Zig** (1): #965 -- keep #984
**Group K: Fleet Health** (1): #930 -- keep #1006
**Group L: ADF Orch** (2): #979, #951 -- keep #963
**Group M: Repo Stewardship** (8): #1009, #1008, #1007, #960, #948, #947, #946, #993 -- keep #898
**Group N: CI Runners** (1): #1015 -- keep #1005
**Group O: Remediation Noise** (3): #1017, #1018, #1019 -- close all
**Group P: Operational/Meta** (3): #983, #982, #1010 -- close all

**Total**: ~55 issues to close

### Step 10: Update AGENTS.md with Sync Protocol
**Estimated**: 10 minutes

```markdown
## Remote Sync Protocol

### Authoritative Remote
- **Primary**: origin (GitHub) - push here first
- **Mirror**: gitea (Gitea) - push after origin succeeds
- Always merge origin into local before pushing

### Agent Push Rules
1. git fetch origin && git merge origin/main --no-edit
2. git push origin main
3. git push gitea main
4. Never force-push to either remote

### Issue Hygiene
- Check for duplicates before creating issues
- One issue per task
- Close issues immediately when work is verified
- Close agent-generated issues without clear acceptance criteria
```

### Step 11: Final Verification & Push
**Estimated**: 5 minutes

```bash
git status
git log --oneline -5
git branch | wc -l
git push origin main
git push gitea main
```

## Rollback Plan

1. All three original heads tagged with timestamps
2. If merge goes wrong: `git reset --hard pre-reconciliation/local-main-*`
3. Closed issues/PRs can be reopened via API
4. Deleted local branches recoverable from remote tracking refs

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Identify exact cherry-pick SHAs from task/860 | Pending | Step 0 |
| Verify Cargo.lock merge resolution | Pending | Step 2 |
| Review previously hidden PRs (#660, #655, #640, #639, #636) | Pending | Step 8 |
| Confirm issue groups before batch close | Pending | Step 9 |
| Get human approval | Pending | User |

## Approval

- [ ] Technical review complete
- [ ] Merge conflict resolution strategy approved
- [ ] Issue/PR closure list approved
- [ ] Human approval received
