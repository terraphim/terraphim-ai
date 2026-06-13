# Handover: ADF digital-twins improvements & per-project status fix

**Date:** 2026-06-13
**Session owner:** opencode (k2p7)

---

## 1. Progress Summary

### Tasks completed this session

1. **Investigated digital-twins auto-merge failures**
   - Root cause identified: orchestrator was posting PR commit statuses to the wrong repo (`terraphim/terraphim-ai` instead of `terraphim/digital-twins`).
   - This meant Gitea never saw the required `adf/pr-reviewer`, `adf/validation`, `adf/verification` checks pass, so merges failed with 405.

2. **Fixed per-project commit-status posting**
   - Updated `crates/terraphim_orchestrator/src/lib.rs`:
     - `post_pending_status` now takes `agent_name` and uses `OutputPoster::tracker_for(project, agent_name)`.
     - `post_terminal_commit_status` now takes `project` and `agent_name` and uses the per-project tracker.
     - Call sites updated in `handle_review_pr` and agent exit handling.
   - Built and deployed new `adf` binary to bigbox (PID 3910650).
   - Pushed branch `task/1715-pr-reviewer-project-source-dispatch-home` to GitHub and Gitea.
   - Created PR #2628: **Fix #1715: use per-project tracker for PR commit statuses**.

3. **Improved digital-twins repo hygiene**
   - Added `.worktrees/` to `.gitignore` (PR #129, merged).
   - Removed 19 tracked `.worktrees/` entries and `.claude/skills/` from `main` (PR #130, merged).

4. **Created branch protection for `main`**
   - Enabled required status checks:
     - `adf/pr-reviewer`
     - `adf/validation`
     - `adf/verification`
   - Set `block_on_outdated_branch: true`.

5. **Cleaned up agent-generated PR backlog**
   - PR #126: rebased, removed recreated `.claude/skills/pr-validator.md`, merged.
   - PR #127: rebased, removed `.worktrees/*` auto-commits, merged.
   - PR #128: rebased, removed `.worktrees/*` auto-commits, merged.

### Current implementation state

- `terraphim/digital-twins`: **0 open issues, 0 open PRs**.
- `terraphim/terraphim-ai` PR #2628 is open but `mergeable=False` (likely behind `main`; needs rebase/merge of `origin/main`).
- New `adf` binary is running on bigbox and using `/tmp/adf-worktrees/` for digital-twins agents.

### What's working

- ADF service active on bigbox (PID 3910650).
- digital-twins branch protection configured with required ADF gate contexts.
- No tracked `.worktrees/` or `.claude/skills/` artefacts remain on digital-twins `main`.
- Status posting logic now resolves the correct per-project Gitea tracker.

### What's blocked / needs follow-up

1. **PR #2628 mergeable=False**
   - Branch `task/1715-pr-reviewer-project-source-dispatch-home` is behind `main`.
   - Next action: fetch `origin/main`, rebase/merge, force-push, then merge PR #2628.

2. **Corrupt worktree index**
   - `/home/alex/projects/terraphim/terraphim-ai-1715-clean` has index/cache-tree corruption (`invalid sha1 pointer in cache-tree`, missing blobs).
   - `git log` works; commit `46900c28e` is already pushed.
   - Do **not** rely on `git status`/`git diff` in this worktree until rebuilt.
   - Recommended: `rm -rf` the worktree and check out fresh if more edits are needed.

3. **Auto-merge still conservative**
   - `min_confidence = 5/5` means most PRs stop at 4/5 and require human merge.
   - Consider lowering to 4/5 for docs-only PRs once status-posting fix is verified.

4. **Fleet-wide reviewer parsing errors**
   - terraphim-ai still has many PRs where reviewer comments fail to parse (head SHA mismatch, missing `adf:gate-result` block).
   - These are unrelated to the digital-twins work but dominate reconcile logs.

---

## 2. Technical Context

### Branch

```
task/1715-pr-reviewer-project-source-dispatch-home
```

### Recent commits

```
46900c28e fix(orchestrator): use per-project tracker for PR commit statuses
6e1eda7e0 fix(agent): make CLI role tests hermetic Refs #1769
6e74f13c0 fix(clippy): satisfy workspace all-target build gate Refs #1769
b80d98de7 feat(orchestrator): add project-scoped agent registry Refs #1769
09b4587df fix(orchestrator): split reconcile tick timeouts Refs #1769
```

### Modified files

```
crates/terraphim_orchestrator/src/lib.rs
```

### digital-twins latest commits

```
b67fc37 Merge pull request 'chore(repo): remove tracked ADF worktrees and deprecated claude skills' (#130)
dc8c8e8 chore(repo): remove tracked ADF worktrees and deprecated claude skills
64732cc Merge branches 'task/127-fix' and 'task/128-fix'
ff82cd4 Merge pull request 'docs(adf): align sdk-coverage-guardian with canonical gate patterns' (#126)
ecabfe5 docs(native-requirements-validator): add Check Status Derivation and tighten field contracts
```

---

## 3. Verification Commands

```bash
# ADF service health
ssh bigbox "sudo systemctl is-active adf-orchestrator"

# digital-twins issue/PR state
gtr list-issues --owner terraphim --repo digital-twins --state open
gtr list-pulls --owner terraphim --repo digital-twins --state open

# Branch protection
curl -s "$GITEA_URL/api/v1/repos/terraphim/digital-twins/branch_protections/main" \
  -H "Authorization: token $GITEA_TOKEN" | python3 -m json.tool

# PR #2628 status
gtr view-pull --owner terraphim --repo terraphim-ai --index 2628
```

---

## 4. Next Steps for Next Session

1. Rebase/merge `origin/main` into `task/1715-pr-reviewer-project-source-dispatch-home` and push.
2. Merge PR #2628 on Gitea.
3. Monitor digital-twins PRs to confirm statuses now post to `terraphim/digital-twins` correctly.
4. Once verified, consider lowering `min_confidence` for docs-only PRs or adding a docs-specific auto-merge policy.
5. Optionally delete the corrupt worktree and re-check out fresh.

## Update: PR #2628 status and canonical source location

PR #2628 was created on `terraphim/terraphim-ai` but is `mergeable=False` because `main` deleted `crates/terraphim_orchestrator/` in commit aa7ba99e (Refs #1910). The orchestrator crate now lives in `terraphim-agents/crates/terraphim_orchestrator`, where the equivalent code already resolves per-project owner/repo via `gitea_owner_repo_for_project(project)` in `pr_handlers_impl.rs`. A comment was added to PR #2628 explaining this.

The binary deployed to bigbox during this session was built from the pre-refactor `terraphim-ai` worktree and includes the fix, so digital-twins is unblocked operationally. Long-term, ADF should be rebuilt/deployed from the `terraphim-agents` repo.
