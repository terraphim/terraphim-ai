# Implementation Plan: Reconcile ADF auto-merge allowlist fix + remediate 10 stuck fleet PRs

**Status**: Draft (awaiting human approval — this plan is not to be executed autonomously)
**Research Doc**: `.docs/research-adf-fleet-allowlist-and-stuck-prs.md`
**Author**: session agent (Claude Code)
**Date**: 2026-07-01
**Estimated Effort**: 2–4 hours, gated by Step 1's findings and deploy timing

## Overview

### Summary
Deploy the verified allowlist fix (terraphim-ai#3065) to bigbox, close the two
competing fix PRs opened in the wrong repo (terraphim-agents#70/#69), finish
attributing the 9 remaining stuck PRs' CI failures, and apply a fixed decision
rule to each: merge, fix-then-merge, or close.

### Approach
Sequential, gated steps. Step 1 (identify the active task branch) is short but
informs whether the current binary carries other off-main changes that need to
be preserved or rolled back before #3065 is deployed from `main`.

### Scope

**In Scope:**
- Identifying the terraphim-ai task branch that built the current bigbox binary
- Merging terraphim-ai#3065 and deploying it to bigbox
- Closing/redirecting terraphim-agents#70/#69
- Attributing CI failure cause for the 5 still-unknown stuck PRs
- A merge/fix/close decision for each of the 9 remaining stuck PRs

**Out of Scope:**
- Fixing every attributed CI failure in this pass (attribution ≠ remediation
  for every case — see Decision Rule)
- Reconciling the full `terraphim_orchestrator` divergence between
  terraphim-ai and terraphim-agents beyond closing the two false-premise PRs
- Redesigning the fleet-agent Gitea login convention (`adf-<name>` prefix
  for all agents) — valuable, but a separate proposal
- Any change to the orchestrator's reconcile-tick logging behaviour (the
  "re-log every 30s forever" pattern) — a robustness improvement worth its
  own small ticket, not bundled here

**Avoid At All Cost** (5/25 elimination):
- Rewriting `terraphim_orchestrator`'s auto-merge module from scratch to
  "finally fix it properly" — three fixes already exist; the job is to pick
  one, not add a fourth
- Merging any of the 9 stuck PRs on `mergeable=true` alone without checking
  combined CI status per this plan's Decision Rule (this is exactly the
  mistake that would ship terraphim-clients#26's clippy bug)
- Attempting to fix the CI-runner `rustup-with-perms` infra bug as part of
  this plan — it's an ops issue on the runner host, orthogonal to any PR's
  content, and owning it here would blow the scope
- Blindly adopting the current task-branch binary's other off-main changes
  just because they are "already deployed" — each must be reviewed and
  either fast-tracked to main or explicitly rolled back

## Architecture

### Data Flow (of this remediation, not of the orchestrator)
```
Step 1 (identify active task branch) --gates--> Step 2 (merge #3065)
                                                       |
                                                       v
Step 3 (deploy from main) ----------------------> Step 4 (close agents#70/#69)
                                                       |
                                                       v
Step 5 (finish CI taxonomy) --------------------> Step 6 (apply decision rule per PR)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|------------------------|
| Keep terraphim-ai#3065 as the canonical fix and close agents#70/#69 | Verified that bigbox's binary is built from terraphim-ai; agents#70/#69 were based on a false premise | Merging agents#70/#69 and reconciling three designs; porting #3065 to terraphim-agents |
| Merge #3065 to `main`, then deploy from `main` | Avoids silently continuing a task-branch-deploy pattern that has already lost commits (#3024's `38f06db0`) | Deploying #3065's branch directly without merging to main |
| KG-driven allowlist in the filesystem (`crates/terraphim_orchestrator/kg/recognised_agents.md`) | Editable by ops without a rebuild; no service restart needed for allowlist changes alone | Fleet-config-sourced allowlist (agents#70) still requires a deploy to add a login |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|---------------------|
| Fixing all 9 CI-failing PRs' underlying code issues in this plan | 5 are still unattributed; fixing blind is guessing | Wasted effort on PRs that may need to be closed instead (stale, superseded) |
| Unifying terraphim-ai and terraphim-agents' orchestrator copies | Separate, larger architecture problem flagged in research §Recommendations | Scope explosion — this plan would never ship |
| Automating the merge/close decisions with a script | Only 9 PRs; a human-reviewable table is faster to produce and safer to execute than automation for a one-time cleanup | Script bugs execute against real repos with no review step |
| Deploying the current task branch plus #3065 as a one-off | Reinforces the same branch-based-deploy pattern that caused the lost-commit confusion | Lost-traceability risk; main would still not reflect production |

### Simplicity Check
**What if this could be easy?** It would be: "merge the verified fix, deploy
from main, close the two wrong-repo PRs, then finish the CI taxonomy and apply
a simple decision rule." That's exactly this plan's shape — six steps, no new
abstractions, no code written for this plan itself (all actions are Gitea API
calls and one standard deploy).

**Nothing Speculative Checklist**:
- [x] No features requested beyond "reconcile the fix and clear the queue"
- [x] No new abstractions — reuses existing Gitea PR/issue primitives
- [x] No flexibility added "just in case" — the decision rule is fixed, not configurable
- [x] No error handling for scenarios that cannot occur
- [x] No speculative unification of the two orchestrator copies

## File Changes

No new files in this plan. PR #3065 already contains the necessary code
changes. This plan's actions are operational:
- Gitea PR merge (#3065)
- Gitea PR close with comment (agents#70, agents#69)
- bigbox systemd deploy
- Gitea PR merge/close/comment for the 9 remaining stuck PRs

## Test Strategy

### Verification of #3065
| Test | How | Expected Result |
|------|-----|-----------------|
| Unit tests | `cargo test -p terraphim_orchestrator --lib` | Pass (already verified: 18 tests including `author_is_agent_policy`) |
| Pre-commit hook | `git commit` via repo hook | Pass (formatting + workspace tests) |
| Deploy smoke | After deploy, `sudo journalctl -u adf-orchestrator -f` | Previously-blocked login (`implementation-swarm`) either stops producing rejection lines or starts logging successful auto-merge enqueue |

### Verification of Stuck-PR Decisions
| Test | How | Expected Result |
|------|-----|-----------------|
| Taxonomy completeness | Every one of the 9 PRs has a filled-in row | No "not yet isolated" entries |
| Decision-rule correctness | Each PR action is one of {merge, rebase, fix inline, re-run CI, close} with a cited reason | No PR left in ambiguous state |

## Implementation Steps

### Step 1: Identify the active task branch that built the current binary
**Action:** On bigbox, correlate `/usr/local/bin/adf` metadata (mtime, md5)
with the task-branch checkout paths under `/opt/ai-dark-factory/build/` and the
zsh history entries that clone terraphim-ai task branches. Specifically:
- Check `/opt/ai-dark-factory/build/` for terraphim-ai checkouts and their
  branches.
- Match binary hash to `target/release/adf` in those checkouts.
- Record the branch name and the extra commits it carries vs. `main`.

**Verifies:** which off-main changes are silently deployed.
**Blocks:** Step 3 (deploy from main) if the branch contains needed changes
that must be fast-tracked first.
**Estimated:** 15–30 minutes.

### Step 2: Merge terraphim-ai#3065
**Action:** Merge PR #3065 to `terraphim-ai/main` via Gitea API or web UI.

**Depends on:** nothing (can proceed independently of Step 1).
**Test/Verification:** PR merge commit appears on `main`; `cargo test -p
terraphim_orchestrator --lib` passes on `main`.
**Estimated:** 5 minutes.

### Step 3: Deploy from main to bigbox
**Action:**
```bash
ssh bigbox
cd /home/alex/projects/terraphim/terraphim-ai
git fetch origin
git checkout main
git pull origin main
cargo build --release -p terraphim_orchestrator --bin adf
sudo systemctl stop adf-orchestrator
sudo cp target/release/adf /usr/local/bin/adf
sudo systemctl start adf-orchestrator
```

**Depends on:** Steps 1 and 2.
**Test/Verification:** After restart, tail `sudo journalctl -u adf-orchestrator
-f` and confirm a PR previously blocked with "author `implementation-swarm` is
not a recognised agent" either stops recurring or proceeds to auto-merge
enqueue.
**Estimated:** 10–20 minutes build + deploy; owner's call on timing.

### Step 4: Close/redirect terraphim-agents#70 and #69
**Action:** Post a closing comment on each PR explaining that the deploy source
is verified to be terraphim-ai, the fix has landed in terraphim-ai#3065, and
the agents-repo changes would target the wrong copy. Close both PRs.

**Depends on:** Step 2 (so #3065 exists to link to).
**Test/Verification:** Both PRs show state=closed with a redirect comment.
**Estimated:** 10 minutes.

### Step 5: Finish the CI failure taxonomy for the 5 unattributed PRs
**Depends on:** nothing (can run in parallel with Steps 1–4, but listed after
because it's lower-priority than un-breaking the allowlist).
**Action:** For each of terraphim-agents#53, terraphim-clients#54, #35, #34,
#21, and terraphim-clients#18 (conflicts case), repeat this research session's
method: get the PR's head SHA → combined status → for each `failure` context,
find the matching Actions run/job via
`GET /repos/{owner}/{repo}/actions/runs?limit=100` filtered by
`head_branch` → `GET .../jobs/{id}/logs` → grep for `error:`/`Workflow
failed:`. Record one line per PR: gate, root cause, category (genuine code
bug / infra flake / orchestrator-gate-only).
**Test/Verification:** Every one of the 9 stuck PRs (10 minus #31, already
merged) has a filled-in row in the taxonomy table — no "not yet isolated"
or "empty response" entries left.
**Estimated:** 30–45 minutes (roughly what the first 4 took, this session).

### Step 6: Apply the Decision Rule to each of the 9 remaining PRs

**Decision Rule** (apply mechanically once each PR's category is known):

| Category | Action |
|---|---|
| Combined CI status = success AND mergeable = true | Merge now |
| Combined CI status = success AND mergeable = false | Rebase (or ask the fleet agent to re-run on latest `main`) — do not merge unresolved conflicts |
| Genuine code bug in the PR's own diff (e.g. clippy failure) | Do not merge; either fix inline (small, obvious fixes only — e.g. terraphim-clients#26's `field_reassign_with_default`) or comment asking the agent/owner to re-run |
| CI-runner infrastructure bug (e.g. `rustup-with-perms`), unrelated to the PR's diff | Re-run the workflow once (infra flakes often clear); if it fails identically twice, file a separate infra ticket and do not block the PR on it |
| ADF-gate-only failure (adf/pr-reviewer, adf/validation, adf/verification) with native-ci green | Check whether it correlates with the bigbox Anthropic provider outage noted in this session's earlier findings; if so, treat as a review-agent availability problem, not a PR defect — do not close or force-merge, wait for provider health to recover and let the gate re-run |
| PR superseded/duplicated by another (e.g. terraphim-clients#18 vs #20, both Refs #2366) | Compare diffs; keep the more complete/correct one, close the other with a comment linking to the survivor |

**Depends on:** Step 5 (needs full taxonomy) and Step 3 (the allowlist fix
must be live before any of these can auto-merge even if CI is green).
**Test/Verification:** Post-execution, `terraphim-clients#18`/`#20` overlap
is resolved to exactly one open or merged PR, not two; every other PR is
either merged, has a specific fix-and-retry action recorded against it, or
is closed with a linked reason.
**Estimated:** 45–60 minutes across 9 PRs, dominated by the two duplicate
`#2366` PRs needing an actual diff comparison.

## Rollback Plan
- **Step 2/3 (merge/deploy of #3065):** if the deployed fix turns out wrong
  post-deploy (e.g. still doesn't recognise a login), revert the merge commit
  on `terraphim-ai/main`, rebuild/deploy the previous binary from the prior
  known-good state.
- **Step 4 (close agents#70/#69):** if #3065 is reverted, re-open the
  runner-up PR or create a new one.
- **Step 6 (per-PR merges):** each merge is a normal Gitea PR merge — revertible
  via `git revert` on the target branch if a merged PR turns out broken.
- No schema/data migrations involved; rollback is git-native throughout.

## Dependencies
No new crate dependencies. This plan is entirely Gitea-API-level PR/issue
operations plus one standard Rust build/deploy to bigbox.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Which terraphim-ai task branch built the current bigbox binary? | **Answered in principle (terraphim-ai), but exact branch still to confirm** | session agent |
| Close/redirect terraphim-agents#70/#69 | Pending Step 2 | session agent |
| Deploy #3065 to bigbox | Pending human approval and Step 1/2 | Alex |
| CI taxonomy for 5 PRs | Pending | session agent |
| Duplicate resolution for terraphim-clients#18 vs #20 (issue #2366) | Pending Step 5 completion | session agent |

## Approval

- [x] Technical review complete
- [x] Repo-identity question resolved (terraphim-ai verified)
- [ ] Exact active task branch identified (Step 1)
- [ ] Human approval received to execute Steps 2–6
