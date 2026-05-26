# Design & Implementation Plan: Open PR Merge Plan

## 1. Summary of Target Behavior

The merge process should restore remote convergence, then reduce the PR backlog in a safe order. GitHub PRs are handled through `gh`; Gitea PRs are handled through `gtr`. No PR is merged solely because it is mechanically mergeable. `adf/build` success, duplication, freshness, and relationship to current ADF stability work determine the merge sequence.

## 2. Key Invariants and Acceptance Criteria

Invariants:
- Preserve all untracked local files.
- Never force-push.
- Do not merge PRs with failed `adf/build` unless explicitly approved as an exception.
- Do not merge duplicate PRs with identical head SHAs.
- Keep `origin/main` and `gitea/main` converged after approved merge batches.

Acceptance criteria:
- `main`, `origin/main`, and `gitea/main` state is known before merging.
- GitHub PRs are classified through `gh`.
- Gitea PRs are classified through `gtr`.
- Each recent Gitea PR is assigned to one bucket: ready, duplicate, needs fix, stale, or investigate.
- The merge sequence has explicit stop/go gates.
- Every merge batch ends with verification and remote convergence check.

## 3. High-Level Design and Boundaries

The plan has four boundaries:

| Boundary | Responsibility | Out of Boundary |
| --- | --- | --- |
| Remote convergence | Decide how to reconcile `origin/main` and `gitea/main` | Force-push or history rewrite |
| GitHub PR cleanup | Close or supersede stale GitHub PRs after confirmation | Revive conflicting PRs without new work |
| Recent Gitea PR merge lane | Merge current high-signal ADF PRs with green gates | Historical backlog cleanup |
| Historical backlog triage | Categorise old PRs and create follow-up issues/comments | Immediate mass merge |

## 4. File/Module-Level Change Plan

No application code changes are part of this merge plan. Operational changes are limited to repository/PR state.

| Target | Action | Before | After | Dependencies |
| --- | --- | --- | --- | --- |
| `origin/main` | Sync decision | Behind `gitea/main` | Either converged to `gitea/main` or explicitly deferred | Human approval |
| `gitea/main` | Reference main | Ahead by PR `#1794` | Remains source of latest ADF fallback fix | Verify diff before pushing to GitHub |
| GitHub PR `#881` | Close/supersede candidate | Conflicting, old CI failures | Closed or commented as superseded | Confirm no unique work not in Gitea PRs |
| GitHub PR `#882` | Close/supersede candidate | Conflicting, old CI failures | Closed or commented as superseded | Confirm relation to Gitea `#1758` |
| Gitea PR `#1786` | Merge candidate | `adf/build` success, mergeable | Merge after convergence decision | Duplicate `#1782` handling |
| Gitea PR `#1782` | Duplicate cleanup | Same head SHA as `#1786` | Close/comment duplicate | Preserve relevant comments/context |
| Gitea PR `#1788` | Merge candidate | `adf/build` success, mergeable | Merge after `#1786` or after dependency check | Confirm no hidden dependency ordering |
| Gitea PRs `#1791`, `#1789`, `#1787` | Needs-fix lane | Mergeable but `adf/build` failed | Comment failure and return to agents | Build failure details |
| Historical Gitea PRs | Triage lane | Mixed stale/failing states | Categorised; no immediate merge | Separate backlog sweep |

## 5. Step-by-Step Implementation Sequence

1. Confirm repository state: run `git status --short --branch`, `git rev-parse origin/main`, and `git rev-parse gitea/main`. Purpose: prevent accidental merges from a stale base. Deployable state: yes.
2. Decide remote convergence: if approved, merge `gitea/main` into local `main`, push to `origin`, then verify `git diff origin/main gitea/main --stat` is empty. Purpose: make both remotes agree before new merges. Deployable state: yes.
3. Handle GitHub stale PRs: use `gh pr view` for `#881` and `#882`, confirm they are superseded, then comment/close if approved. Purpose: remove conflicting duplicate review surfaces. Deployable state: yes.
4. Select canonical ADF registry PR: retain Gitea `#1786`; comment on and close `#1782` as duplicate if approved. Purpose: avoid merging the same SHA twice. Deployable state: yes.
5. Merge Gitea `#1786`: use `gtr merge-pull` only after confirming `adf/build success` remains current against its head. Purpose: land project-scoped agent registry. Deployable state: yes.
6. Re-fetch and re-evaluate `#1788`: confirm it still merges cleanly and `adf/build` is still valid after `#1786`. Purpose: land local skills integration after the registry foundation. Deployable state: yes.
7. Queue needs-fix PRs: comment on `#1791`, `#1789`, and `#1787` with their `adf/build` failure summary and do not merge. Purpose: keep failed PRs visible but blocked. Deployable state: yes.
8. Historical backlog pass: create a separate issue or report grouping old PRs into stale, failed-build, conflict, and ready-for-rebase buckets. Purpose: prevent old PRs from blocking the current ADF merge lane. Deployable state: yes.
9. End-of-batch verification: fetch both remotes, compare `origin/main` and `gitea/main`, run relevant status checks, and summarise merged/closed/deferred PRs. Purpose: leave the repository in a known state. Deployable state: yes.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Verification Location |
| --- | --- | --- |
| Remote state is known | Git verification | `git rev-parse main origin/main gitea/main` |
| GitHub PRs classified via `gh` | CLI verification | `gh pr list --repo terraphim/terraphim-ai --state open` |
| Gitea PRs classified via `gtr` | CLI verification | `gtr list-pulls --owner terraphim --repo terraphim-ai --state open` |
| No failed build PR merged | Status verification | `gtr` PR data plus commit statuses shown in Gitea |
| Duplicate PR not merged twice | SHA comparison | `#1782` and `#1786` head SHA comparison |
| Remotes converge after merge batch | Git verification | `git diff origin/main gitea/main --stat` empty |
| Local unrelated files preserved | Worktree verification | `git status --short --branch` still shows only expected untracked files |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
| --- | --- | --- |
| GitHub and Gitea divergence causes accidental regression | Converge remotes before additional merges | GitHub branch protection may require PR instead of direct push |
| `#1788` depends on unmerged `#1786` semantics | Merge `#1786` first, then re-check `#1788` | Rebase may still be needed |
| Failed PRs get merged because Gitea says `mergeable=true` | Treat `adf/build` failure as blocking | Manual override remains possible |
| Duplicate `#1782` contains useful discussion | Comment with canonical PR link before closing | Some context may remain split |
| Old PR backlog remains large | Separate stale/backlog triage from current merge lane | Requires follow-up session |
| Direct push to GitHub main is blocked | Use PR or configured remote sync process | Adds delay |

## 8. Open Questions / Decisions for Human Review

1. Approve syncing `gitea/main` to `origin/main` before any further merges?
2. Approve making `#1786` canonical and closing `#1782` as duplicate?
3. Approve closing GitHub `#881` and `#882` as stale/superseded after a final unique-commit check?
4. Should `#1788` be merged immediately after `#1786` if its build status remains green?
5. Should failed recent PRs `#1791`, `#1789`, and `#1787` be reassigned to agents with explicit build-failure comments?
