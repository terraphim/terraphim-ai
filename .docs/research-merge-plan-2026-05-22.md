# Research Document: Open PR Merge Plan

## 1. Problem Restatement and Scope

The repository has diverged across GitHub and Gitea. GitHub has two stale open pull requests, while Gitea has a much larger active PR queue. Local `main` matches `origin/main`, but `gitea/main` is ahead with an already-merged KG-router fallback fix. A safe merge plan must restore remote convergence, classify PRs by readiness, and avoid merging stale, duplicate, or failing work.

In scope:
- Evaluate open GitHub PRs via `gh`.
- Evaluate open Gitea PRs via `gtr`.
- Account for `origin/main` versus `gitea/main` divergence.
- Prioritise PRs by merge readiness, evidence, duplication, and current operational value.
- Define cleanup for stale or duplicate PRs.

Out of scope:
- Implementing PR fixes.
- Merging PRs without explicit approval.
- Rewriting PR history or force-pushing.
- Closing PRs automatically.
- Solving every historical failing PR in the backlog.

## 2. User & Business Outcomes

Expected outcomes:
- Maintainers get a clear sequence for reducing PR backlog safely.
- GitHub and Gitea `main` are brought back into a predictable relationship before further merges.
- High-value ADF stability work lands before lower-priority or stale work.
- Duplicate PRs are identified so reviewers do not waste effort on redundant branches.
- Failing PRs are not merged based only on `mergeable=true` metadata.

## 3. System Elements and Dependencies

| Element | Location | Role | Dependency/Concern |
| --- | --- | --- | --- |
| Local repository | `/home/alex/projects/terraphim/terraphim-ai` | Working checkout used for evaluation | Current branch is `main`; untracked local files must be preserved |
| GitHub remote | `origin` / `gh pr` | Public or primary upstream PR surface | `origin/main` is behind `gitea/main` |
| Gitea remote | `gitea` / `gtr` | Authoritative task/PR workflow | 50 open PRs, including current ADF work |
| ADF statuses | Gitea commit statuses | Build/review evidence for Gitea PRs | Some statuses are stale, failed, or missing |
| Branch protection | Gitea main protection | Merge gate enforcement | Recent logs show some branch protection API lookups fail for other projects; terraphim-ai remains the target here |
| ADF build-runner | `adf/build` | Workspace validation gate | Step failures must block merge unless intentionally waived |
| PR duplicate relation | PR `#1782` and `#1786` | Same head SHA for issue `#1769` | One should be retained, the duplicate closed after context is preserved |
| Remote convergence | `origin/main` and `gitea/main` | Release/source-of-truth consistency | Gitea has PR `#1794` already merged; GitHub lacks those commits |

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
| --- | --- | --- |
| Use `gh` for GitHub and `gtr` for Gitea | User explicitly requested tool split | Avoid direct API as primary source except when `gtr` output needs post-processing |
| Do not destroy local untracked files | Workspace contains unrelated `.codex`, `.docs/design`, `.docs/research`, `.terraphim` files | No reset/clean; all operations must preserve them |
| Gitea is task-management source of truth | Project instructions require Gitea workflow | Merge plan should privilege Gitea PR state over stale GitHub PRs |
| No force push | Remote sync rules prohibit destructive history changes | Convergence must use normal merge/push or PR workflow |
| `adf/build` is a hard quality signal | It validates workspace build, clippy, and tests | PRs with failed `adf/build` need repair before merge |
| Duplicate PRs inflate review load | `#1782` and `#1786` point to same SHA | Select one canonical PR and close/comment the duplicate |
| GitHub PRs are conflicting | `gh` reports both open GitHub PRs as `CONFLICTING` | Do not merge GitHub PRs as-is; close or supersede after confirming no unique work |

## 5. Risks, Unknowns, and Assumptions

Risks:
- Merging into local `main` from stale `origin/main` could omit already-merged Gitea work.
- Closing stale PRs without checking unique commits could lose useful context.
- `mergeable=true` on Gitea does not mean quality gates passed.
- PRs with old green checks may no longer pass against current `main`.
- PR `#1788` depends on ADF config/project-source behaviour that overlaps with `#1786` and `#1794`.

Unknowns:
- Whether GitHub should mirror Gitea immediately or only after a selected merge batch.
- Whether all old Gitea PRs are still desired or should be bulk-closed as obsolete.
- Whether `#1788` has hidden dependency on `#1786` despite being independently mergeable.
- Whether `#1791`, `#1789`, and `#1787` have simple clippy/build failures or deeper design issues.

Assumptions:
- Gitea `main` is currently ahead because PR `#1794` was intentionally merged.
- Gitea PR `#1786` is the canonical ADF agent-registry PR, and `#1782` is duplicate/superseded because it has the same head SHA.
- GitHub PRs `#881` and `#882` are stale because their current mergeability is `CONFLICTING` and matching/superseding work exists in Gitea.
- PRs with failed `adf/build` should not be merged until fixed or explicitly waived.

## 6. Context Complexity vs. Simplicity Opportunities

Complexity sources:
- Two remotes with different open PR sets.
- Gitea has a large historical backlog with mixed quality and stale checks.
- ADF status contexts include both legacy audit-style statuses and current build/review gates.
- Multiple PRs target similar security/config themes.

Simplicity opportunities:
- First converge remotes, then merge one small batch at a time.
- Treat recent ADF PRs separately from the historical backlog.
- Use strict buckets: ready, duplicate, needs-fix, stale/conflicting, investigate.
- Prefer PRs with current `adf/build success` and no duplication.

## 7. Questions for Human Reviewer

1. Should `gitea/main` be pushed to `origin/main` before any further merge work?
2. Should Gitea PR `#1786` be canonical and `#1782` closed as duplicate?
3. Should GitHub PRs `#881` and `#882` be closed as stale/superseded?
4. Is it acceptable to prioritise ADF operational fixes over older feature/test PRs?
5. Should PRs with failed `adf/build` be automatically labelled or commented before being revisited?
6. Should historical PRs older than 7 days be bulk-triaged into a stale backlog rather than evaluated one by one?
