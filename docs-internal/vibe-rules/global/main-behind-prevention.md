# Main-Behind Prevention (Dual-Remote Sync)

#git #ci #dual-remote #divergence #workflow

`main` must never fall behind on either remote. This repository has two
remotes that MUST stay content-identical:

- **origin** (GitHub: `github.com/terraphim/terraphim-ai`) — primary, push first
- **gitea** (Gitea: `git.terraphim.cloud/terraphim/terraphim-ai`) — mirror, push second

## The Rule

**Severity: P1.** A push or PR that leaves one remote behind violates the
Remote Sync Protocol (see `AGENTS.md` → *Remote Sync Protocol*).

**Flag these patterns:**
- PR opened from a branch whose merge-base is behind `gitea/main` or `origin/main`
- Push to `main` without first `git fetch origin && git merge origin/main --no-edit`
- Pushing to only one remote (origin without gitea, or vice versa)
- `git rev-list --left-right --count origin/main...gitea/main` returning
  non-zero on either side

**Measured motivation:** on 2026-08-17 the remotes were genuinely divergent
(`origin/main` 9 ahead, `gitea/main` 72 ahead) — exactly what this rule
exists to catch. See issue #3252.

## The Fix

Follow the **Mandatory Push Sequence**:

```bash
git fetch origin
git merge origin/main --no-edit
git push origin main
git push gitea main
git diff origin/main gitea/main --stat  # Must be empty
```

If the remotes have already diverged, perform **Divergence Recovery**
(`AGENTS.md` → *Divergence Recovery*) before any new work merges.

## Machine-readable form (for review tooling)

```yaml
name: main-behind-prevention
description: >-
  Never push or open a PR from a local main/branch that is behind either
  remote; origin (GitHub) and gitea must stay content-identical per the
  Dual-Remote Protocol in AGENTS.md.
severity: P1
pattern: |
  - PR opened from a branch whose merge-base is behind gitea/main or origin/main
  - Push to main without first `git fetch origin && git merge origin/main --no-edit`
  - Pushing to only one remote (origin without gitea, or vice versa)
  - `git rev-list --left-right --count origin/main...gitea/main` returning
    non-zero on either side (measured 9/72 divergence on 2026-08-17)
fix: |
  Follow the Mandatory Push Sequence: fetch origin, merge origin/main
  --no-edit, push origin main, push gitea main, then verify
  `git diff origin/main gitea/main --stat` is empty. If remotes have
  diverged, perform Divergence Recovery (AGENTS.md) before any new work
  merges.
source: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3252
```
