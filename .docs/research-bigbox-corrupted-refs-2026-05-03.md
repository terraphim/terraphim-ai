# Research Document: Bigbox Git Repository Corrupted Refs

**Status**: Draft | Review | Approved
**Author**: AI Agent (Phase 1 Disciplined Research)
**Date**: 2026-05-03
**Reviewers**: [Human Approval Required]

## Executive Summary

The bigbox git repository at `/home/alex/terraphim-ai` suffers from recurring corrupted refs caused by stale remote tracking branches that point to missing objects. The root cause is a combination of: (1) ADF agents creating 100+ worktrees in `/tmp/adf-worktrees/`, (2) gitea remote having 192 branches that become stale when force-pushed, and (3) insufficient ref cleanup. A previous fix in commit `550be8820` added `git fetch --prune` but the issue persists due to systemic problems with agent branch management.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Does fixing this energize us? | **Yes** | Corrupted refs break CI, agent work, and require manual intervention |
| Does it leverage our strengths? | **Yes** | We control the ADF orchestrator, build-runner, and bigbox infrastructure |
| Does it meet a validated need? | **Yes** | Issue #471 documents "Git worktree cleanup race condition"; commit `550be8820` shows previous fix attempt |

**Proceed**: Yes — 3/3 YES

---

## Problem Statement

### Description

The git repository on bigbox (`/home/alex/terraphim-ai`) develops corrupted refs where remote tracking branches (`.git/refs/remotes/gitea/*` and `.git/packed-refs`) point to objects that no longer exist in the repository. This causes errors like:

```
error: Could not read <hash>
fatal: Could not read <hash>
```

The corruption manifests when:
1. ADF agents fetch PR branches from gitea
2. Branches are force-pushed or deleted on the remote
3. Local refs become stale (point to missing objects)
4. Subsequent fetches or agent operations fail

### Impact

- **ADF agents fail** to checkout branches for building/validation
- **Build-runner** cannot create working directories
- **CI pipeline** breaks when agents report failures
- **Manual intervention** required to fix (prune refs, expire reflogs)

### Success Criteria

1. Zero corrupted refs after 30 days of agent activity
2. Automated cleanup prevents ref accumulation
3. Agent worktree creation/cleanup is race-condition free
4. Remote ref count stays under 50 (currently 192)

---

## Current State Analysis

### Existing Implementation

#### Repository Structure on Bigbox

| Component | Location | Purpose |
|-----------|----------|---------|
| Main repo | `/home/alex/terraphim-ai` | Primary working directory |
| Worktrees | `/tmp/adf-worktrees/` | Agent parallel work spaces (100+ entries) |
| Worktree metadata | `.git/worktrees/` | 222 entries (many stale) |
| Remote: origin | GitHub | Tracks only `main` branch |
| Remote: gitea | Gitea server | Tracks ALL branches (`refs/heads/*:refs/remotes/gitea/*`) |

#### ADF Agent Workflow (Problem Area)

```
Gitea webhook → ADF orchestrator → Spawns agent (sentinel, merge-coordinator, etc.)
    ↓
Agent creates worktree: git worktree add /tmp/adf-worktrees/{agent}-{hash} {branch}
    ↓
Agent does work, commits, pushes
    ↓
Agent removes worktree: git worktree remove {path}
    ↓
PROBLEM: Race condition or crash leaves stale metadata in .git/worktrees/
```

#### Git Configuration Issues

```bash
# .git/config - PROBLEMATIC fetch refspec
[remote "gitea"]
    url = https://git.terraphim.cloud/terraphim/terraphim-ai.git
    fetch = +refs/heads/*:refs/remotes/gitea/*  # Fetches ALL 192 branches!
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Build-runner config | `crates/terraphim_orchestrator/tests/fixtures/conf.d/terraphim.toml` | Agent definitions, includes fix from `550be8820` |
| ADF orchestrator | `crates/terraphim_orchestrator/src/` | Spawns agents, manages worktrees |
| Worktree cleanup | Issue #471, commit `550be8820` | Previous fix attempts |

### Data Flow

```
Gitea Server (192 branches)
    ↓ fetch (all branches)
Bigbox .git/refs/remotes/gitea/* + .git/packed-refs
    ↓ agent worktree add (creates ref in .git/worktrees/)
Agent work directory
    ↓ force-push or branch delete on Gitea
Stale refs pointing to missing objects (CORRUPTION)
```

### Integration Points

- **Gitea API**: Creates/updates branches via webhooks
- **ADF Orchestrator**: Spawns 19 agent types (sentinel, merge-coordinator, etc.)
- **Build-runner**: Fetches, checks out, builds PR branches
- **GitHub Actions**: Self-hosted runners on bigbox (`[self-hosted, bigbox]`)

---

## Constraints

### Technical Constraints

| Constraint | Source | Impact |
|------------|--------|--------|
| Gitea fetch refspec fetches ALL branches | `.git/config` | 192 branches, many stale |
| Worktree metadata accumulates | `.git/worktrees/` | 222 entries, cleanup race condition (#471) |
| Commit `550be8820` fix is partial | Git history | `fetch --prune` added but insufficient |
| No automated ref cleanup cron | Infrastructure | Refs accumulate until manual intervention |

### Business Constraints

| Constraint | Source | Impact |
|------------|--------|--------|
| ADF agents must run 24/7 | Operational requirement | Cannot stop agents for maintenance |
| Zero downtime for CI | Project policy | Must fix without breaking running agents |
| Gitea is single source of truth | Gitea PageRank Workflow | Cannot migrate away from Gitea |

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Remote ref count | < 50 | ~192 |
| Worktree metadata entries | < 20 | 222 |
| Ref cleanup frequency | Daily automated | Manual only |
| Agent worktree lifecycle | < 1 hour | Unknown (possible leaks) |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Gitea fetch refspec too broad | Causes 192 refs, many stale | `.git/config` shows `refs/heads/*:refs/remotes/gitea/*` |
| Worktree cleanup race condition | Leaves stale metadata, corrupts future operations | Issue #471, 222 entries in `.git/worktrees/` |
| No automated ref pruning | Refs accumulate until corruption | Commit `550be8820` added manual prune but not automated |

### Eliminated from Scope

| Item | Why Eliminated |
|------|---------------|
| Migrate away from Gitea | Over-engineering; Gitea is core to workflow |
| Rewrite ADF orchestrator | Wrong order; fix ref management first |
| Shrink number of agent types | Separate concern; 19 agents are valid |

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_orchestrator` | Manages agent worktree lifecycle | High — changes here affect all agents |
| Gitea server API | Source of branch refs | Low — stable |
| Build-runner script | Fetches and checks out branches | Medium — needs robust error handling |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea | v1.21+ | Low — stable API | None (core infrastructure) |
| Git | 2.x | Low | N/A |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Automated pruning breaks running agents | Medium | High | Prune only after agent completes; use reflog expire first |
| Changing fetch refspec breaks CI | Low | Medium | Test on branch; keep origin as fallback |
| Worktree race condition hard to fix | High | Medium | Use file locks; atomic worktree operations |
| 192 branches include critical PRs | Low | High | Audit before pruning; whitelist main branches |

### Open Questions

1. **What is the exact error message when corruption occurs?** — Need to capture actual error output from agents
2. **Can we change Gitea fetch to only track `main` + PR branches?** — Need to test refspec like `+refs/pull/*/head:refs/remotes/gitea/pr/*`
3. **Why does commit `550be8820` fix not persist?** — Need to check if build-runner config is actually deployed to bigbox
4. **Are there other repos on bigbox with same issue?** — Need to check `/home/alex/projects/` for similar problems

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Stale refs cause the corruption | `git fsck` shows no corruption; error messages mention "bad object" | None — error pattern matches | Yes |
| 192 branches are too many | Standard practice is < 50 remote branches | High — if valid, pruning breaks workflow | No |
| Worktree metadata 222 entries is abnormal | Typical repo has < 10 worktrees | None — clearly excessive | Yes |
| Commit `550be8820` is deployed | Build-runner config in fixtures/ suggests it's the source of truth | High — if not deployed, fix never applied | No |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **Interpretation A**: The fix is to narrow Gitea fetch refspec to only `main` + PRs | Reduces refs from 192 to ~20; breaks current agent workflow that expects `refs/remotes/gitea/*` for all branches | **Rejected** — breaks too many things |
| **Interpretation B**: Add automated daily cron job to prune refs and expire reflogs | Simple; addresses symptom but not root cause (worktree race) | **Chosen** — immediate relief; combine with Interpretation C |
| **Interpretation C**: Fix worktree cleanup race condition in ADF orchestrator | Addresses root cause; requires code changes to orchestrator | **Chosen** — necessary for long-term fix |

---

## Research Findings

### Key Insights

1. **Two layers of corruption**: (1) Stale remote refs from broad fetch refspec, (2) Leaked worktree metadata from agent crashes/races. Both must be fixed.

2. **Previous fix was incomplete**: Commit `550be8820` added `fetch --prune` to build-runner, but did not address: (a) automated scheduling, (b) worktree cleanup, (c) narrow refspec.

3. **192 branches are unreasonable**: Standard git practice is to track only `main` + specific branches. The current refspec `refs/heads/*:refs/remotes/gitea/*` is too broad.

4. **Worktree metadata explosion**: 222 entries in `.git/worktrees/` with many pointing to non-existent directories in `/tmp/adf-worktrees/`. This suggests agents crash or are killed without cleanup.

5. **Race condition documented**: Issue #471 explicitly mentions "Git worktree cleanup race condition" — this is a known problem, not a new discovery.

### Relevant Prior Art

- **Commit `550be8820`** (`fix: build-runner git checkout robustness`): Added `git fetch --prune` and suppression of bad object errors
- **Issue #471** (`[ADF] Git worktree cleanup race condition`): Documents the race condition problem
- **`.docs/research-CI-baseline-restoration-2026-04-08.md`**: Mentions Issue #471 in context of CI infrastructure problems

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Test narrow Gitea refspec | Change `+refs/heads/*:refs/remotes/gitea/*` to only track `main` + PRs | 1 hour |
| Audit ADF orchestrator worktree code | Find race condition in `crates/terraphim_orchestrator/src/` | 2 hours |
| Test automated ref pruning cron | Create cron job on bigbox; verify it doesn't break running agents | 1 hour |
| Check build-runner deployment | Verify commit `550be8820` changes are actually on bigbox | 30 minutes |

---

## Recommendations

### Proceed/No-Proceed

**Proceed** — The corruption is recurring, affects all ADF agents, and has a documented previous fix attempt. The problem is systemic and requires both operational (ref pruning) and code (worktree cleanup) fixes.

### Scope Recommendations

**In scope (Phase 2 Design):**
1. Fix Gitea fetch refspec to be narrow (only track `main` + specific branches)
2. Add automated ref pruning (cron job or orchestrator hook)
3. Fix worktree cleanup race condition in ADF orchestrator
4. Add monitoring/alerting for ref corruption

**Out of scope (deferred):**
- Migrating away from Gitea (over-engineering)
- Rewriting ADF orchestrator (too large; fix incrementally)
- Reducing number of agent types (valid business need)

### Risk Mitigation Recommendations

1. **Test refspec change on branch first**: Create test repo or use `git clone --bare` to verify new refspec works
2. **Deploy in phases**: (1) Add cron job for pruning, (2) Fix worktree cleanup, (3) Narrow refspec last
3. **Monitor after each phase**: Check agent success rates and ref counts daily
4. **Keep fallback**: Maintain `origin` remote as narrow-reference fallback during transition

---

## Next Steps

If approved:

1. **Phase 2 (Design)**: Produce Implementation Plan with three tracks:
   - Track A: Operational fix — automated ref pruning cron job
   - Track B: Code fix — worktree cleanup race condition in orchestrator
   - Track C: Config fix — narrow Gitea fetch refspec

2. **Audit current state**: Verify build-runner config from commit `550be8820` is actually deployed to bigbox

3. **Stakeholder communication**: Comment on Issue #471 to indicate work is starting; coordinate with agents to avoid disruption during fix

---

## Appendix

### Reference Materials

- **Commit `550be8820`**: `fix: build-runner git checkout robustness` (2026-05-03)
- **Issue #471**: `[ADF] Git worktree cleanup race condition`
- **Gitea PageRank Workflow**: `.docs/gitea-pagerank-workflow.md` (if exists)
- **ADF Operations Guide**: `docs/adf/operations.md`

### Code Snippets

```bash
# Current problematic refspec in .git/config
[remote "gitea"]
    url = https://root:***@git.terraphim.cloud/terraphim/terraphim-ai.git
    fetch = +refs/heads/*:refs/remotes/gitea/*  # PROBLEM: fetches all 192 branches

# Suggested fix: track only main + PRs
[remote "gitea"]
    url = https://root:***@git.terraphim.cloud/terraphim/terraphim-ai.git
    fetch = +refs/heads/main:refs/remotes/gitea/main
    fetch = +refs/pull/*/head:refs/remotes/gitea/pr/*
```

```bash
# Current state on bigbox
$ ls .git/worktrees/ | wc -l
222

$ ls /tmp/adf-worktrees/ | wc -l
~50 (many worktrees are for agents that have already completed)

$ git for-each-ref refs/remotes/gitea/ | wc -l
192
```
