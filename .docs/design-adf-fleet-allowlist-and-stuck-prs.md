# Implementation Plan: Reconcile ADF auto-merge allowlist fix + remediate 10 stuck fleet PRs

**Status**: Draft — reviewed; binary-provenance gate required before execution
**Research Doc**: `.docs/research-adf-fleet-allowlist-and-stuck-prs.md`
**Author**: session agent (Claude Code)
**Date**: 2026-07-01
**Estimated Effort**: 2–5 hours, gated by binary provenance and deploy timing

## Overview

### Summary
Prove which repo/branch produced bigbox's installed `/usr/local/bin/adf`, merge
exactly one allowlist fix in that canonical source, verify it covers all six
known fleet logins, deploy it from a tracked branch with rollback, close only
the superseded PRs, finish attributing the 9 remaining stuck PRs' CI failures,
and apply a fixed decision rule to each: merge, fix-then-merge, or close.

### Approach
Sequential, gated steps. Step 1 is a hard provenance gate because evidence is
conflicting: `AGENTS.md` says the orchestrator binary is built from
`terraphim-agents`, while bigbox shell history shows real `terraphim-ai`
task-branch builds. No merge, close, or deploy action happens until the
installed binary is traced by hash/build artefact.

### Scope

**In Scope:**
- Identifying the exact repo/branch that built the current bigbox binary
- Choosing exactly one surviving allowlist fix from terraphim-ai#3065,
  terraphim-agents#70, and terraphim-agents#69
- Verifying the chosen fix covers all six known blocked fleet logins:
  `implementation-swarm`, `odilo-developer`, `meta-coordinator`,
  `quality-coordinator`, `security-sentinel`, and `test-guardian`
- Deploying the chosen fix to bigbox from a tracked branch
- Closing/redirecting only the PRs superseded by the proven canonical fix
- Attributing CI failure cause for the remaining unattributed stuck PRs
- A merge/fix/close decision for each of the 9 remaining stuck PRs

**Out of Scope:**
- Fixing every attributed CI failure in this pass (attribution ≠ remediation
  for every case — see Decision Rule)
- Reconciling the full `terraphim_orchestrator` divergence between
  terraphim-ai and terraphim-agents beyond this incident's allowlist fix
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
- Assuming either `AGENTS.md` or shell history alone proves binary provenance —
  require hash/build artefact evidence
- Treating `implementation-swarm` as the whole incident — #3024 records six
  blocked fleet logins and the fix must cover all six
- Blindly adopting the current task-branch binary's other off-main changes
  just because they are "already deployed" — each must be reviewed and
  either fast-tracked to main or explicitly rolled back

## Architecture

### Data Flow (of this remediation, not of the orchestrator)
```
Step 1 (prove binary provenance) --gates--> Step 2 (choose one fix)
                                                    |
                                                    v
Step 3 (verify six-login coverage) ----------> Step 4 (merge chosen fix)
                                                    |
                                                    v
Step 5 (deploy chosen fix) ------------------> Step 6 (close superseded fixes)
                                                    |
                                                    v
Step 7 (finish CI taxonomy) -----------------> Step 8 (apply decision rule per PR)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|------------------------|
| Make binary provenance the first gate | Current evidence conflicts; closing/merging the wrong repo would have no production effect | Trusting `AGENTS.md` alone; trusting shell history alone |
| Merge exactly one allowlist fix | Three divergent fixes for one policy will drift and recreate the incident | Merging #3065 and agents#70/#69 independently |
| Require all-six-login acceptance before merge | #3024 impact spans six fleet accounts; #3065 currently only embeds `implementation-swarm` | Treating the largest offender as the only offender |
| Prefer KG-driven allowlist if implementation cost is comparable | Editable by ops without a rebuild; aligns with Terraphim KG conventions | Fleet-config-sourced allowlist as the only source if it requires deploys for login additions |
| Deploy from a tracked branch after merge | Avoids silently continuing a task-branch-deploy pattern that has already lost commits (#3024's `38f06db0`) | Deploying an unmerged task branch directly |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|---------------------|
| Fixing all 9 CI-failing PRs' underlying code issues in this plan | 5 are still unattributed; fixing blind is guessing | Wasted effort on PRs that may need to be closed instead (stale, superseded) |
| Unifying terraphim-ai and terraphim-agents' orchestrator copies | Separate, larger architecture problem flagged in research §Recommendations | Scope explosion — this plan would never ship |
| Automating the merge/close decisions with a script | Only 9 PRs; a human-reviewable table is faster to produce and safer to execute than automation for a one-time cleanup | Script bugs execute against real repos with no review step |
| Deploying the current task branch plus a local patch as a one-off | Reinforces the same branch-based-deploy pattern that caused the lost-commit confusion | Lost-traceability risk; main would still not reflect production |

### Simplicity Check
**What if this could be easy?** It would be: "prove the binary source, merge
one fix there, verify the six known logins, deploy it safely, close only the
superseded fixes, then finish the CI taxonomy and apply a simple decision
rule." That's exactly this plan's shape — eight steps, no new abstractions, and
only a tiny allowlist data change if the surviving PR does not already include
all six logins.

**Nothing Speculative Checklist**:
- [x] No features requested beyond "reconcile the fix and clear the queue"
- [x] No new abstractions — reuses existing Gitea PR/issue primitives
- [x] No flexibility added "just in case" — the decision rule is fixed, not configurable
- [x] No error handling for scenarios that cannot occur
- [x] No speculative unification of the two orchestrator copies

## File Changes

No new files are required if provenance proves `terraphim-ai#3065` is the
canonical fix. If provenance proves `terraphim-agents` is canonical, a small
port may be required from #3065's KG-driven design into the agents repo before
merge.

Operational actions:
- Gitea PR merge of exactly one surviving allowlist fix
- Gitea PR close/comment for superseded allowlist fixes
- bigbox systemd deploy from the selected canonical branch
- Gitea PR merge/close/comment for the 9 remaining stuck PRs

## Test Strategy

### Verification of the Chosen Allowlist Fix
| Test | How | Expected Result |
|------|-----|-----------------|
| Unit tests | `cargo test -p terraphim_orchestrator --lib` in the selected canonical repo | Pass, including author-gate policy coverage |
| Six-login coverage | Unit test or direct policy harness covering `implementation-swarm`, `odilo-developer`, `meta-coordinator`, `quality-coordinator`, `security-sentinel`, `test-guardian` | All six satisfy the author gate |
| Runtime KG source | Check whether `ADF_RECOGNISED_AGENTS_KG` is set or `kg/recognised_agents.md` exists in orchestrator working directory | Runtime uses intended KG source, or embedded fallback is known to include all six |
| Pre-commit hook | `git commit` via repo hook | Pass (formatting + workspace tests) |
| Deploy smoke | After deploy, `sudo journalctl -u adf-orchestrator -f` | Previously-blocked login (`implementation-swarm`) either stops producing rejection lines or starts logging successful auto-merge enqueue |

### Verification of Stuck-PR Decisions
| Test | How | Expected Result |
|------|-----|-----------------|
| Taxonomy completeness | Every one of the 9 PRs has a filled-in row | No "not yet isolated" entries |
| Decision-rule correctness | Each PR action is one of {merge, rebase, fix inline, re-run CI, close} with a cited reason | No PR left in ambiguous state |

## Implementation Steps

### Step 1: Prove binary provenance
**Action:** On bigbox, correlate `/usr/local/bin/adf` metadata (mtime, hash,
build-id if available) with all candidate build artefacts and deployment
records. Specifically:
- Record `sha256sum /usr/local/bin/adf`, mtime, size, and systemd unit path.
- Enumerate candidate `target/release/adf` binaries under `/home/alex/projects`,
  `/data/projects`, and `/opt/ai-dark-factory/build`.
- Compare hash, size, and mtime for each candidate.
- For any matching or near-matching candidate, record repo, branch, HEAD SHA,
  dirty state, and diff from main.
- Inspect deployment scripts/history for the command that copied the binary to
  `/usr/local/bin/adf`.

**Verifies:** which repo/branch is canonical for this incident and which
off-main changes are silently deployed.
**Blocks:** Steps 2–5. No PR should be merged or closed before this is done.
**Estimated:** 30–60 minutes.

### Step 2: Choose the single surviving allowlist fix
**Action:** Based on Step 1:
- If `terraphim-ai` is proven canonical: keep #3065 as the surviving fix.
- If `terraphim-agents` is proven canonical: review #70/#69 in full and either
  merge them as a stack or port #3065's KG-driven design into `terraphim-agents`.
- If no candidate artefact proves provenance: stop and ask for owner decision;
  do not infer from partial evidence.

**Depends on:** Step 1.
**Test/Verification:** One PR is explicitly named as surviving; the other two
are explicitly named as superseded but not yet closed.
**Estimated:** 15–30 minutes.

### Step 3: Verify all-six-login coverage
**Action:** Before any merge, verify the chosen fix recognises every login in
the #3024 impact table:
- `implementation-swarm`
- `odilo-developer`
- `meta-coordinator`
- `quality-coordinator`
- `security-sentinel`
- `test-guardian`

If the surviving fix is KG-driven, the `synonyms::` line must include all six.
If it is fleet-config-driven, tests must prove all six sample fleet names pass
the author gate. If either candidate is missing coverage, update that candidate
before merge rather than accepting a partial incident fix.

**Depends on:** Step 2.
**Test/Verification:** A test or direct policy check shows all six logins clear
the author gate, and unknown human/bot logins remain rejected.
**Estimated:** 10–20 minutes.

### Step 4: Merge the chosen fix
**Action:** Merge the chosen PR into its repo's main branch. If a port is needed,
create and verify that PR first; do not deploy a local-only patch.

**Depends on:** Step 3.
**Test/Verification:** Merge commit appears on the selected repo's main branch;
the relevant crate tests pass on that branch.
**Estimated:** 5–30 minutes depending on whether a port is needed.

### Step 5: Deploy the chosen fix to bigbox
**Action (repo path depends on Step 1):**
```bash
ssh bigbox
cd <canonical-orchestrator-repo>
git fetch origin
git checkout main
git pull origin main
cargo build --release -p terraphim_orchestrator --bin adf
sha256sum /usr/local/bin/adf target/release/adf
sudo cp /usr/local/bin/adf /usr/local/bin/adf.pre-allowlist-fix
sudo systemctl stop adf-orchestrator
sudo install -m 0755 target/release/adf /usr/local/bin/adf
sudo systemctl start adf-orchestrator
sudo systemctl status adf-orchestrator --no-pager
```

**Depends on:** Step 4.
**Test/Verification:** After restart, tail `sudo journalctl -u adf-orchestrator
-f` and confirm a PR previously blocked with "author `implementation-swarm` is
not a recognised agent" either stops recurring or proceeds to auto-merge
enqueue.
**Estimated:** 10–20 minutes build + deploy; owner's call on timing.

### Step 6: Close/redirect superseded allowlist PRs
**Action:** Post a closing comment on each superseded PR explaining the proven
canonical repo and the surviving fix. Close only those superseded by the merged
and deployed fix.

**Depends on:** Step 5.
**Test/Verification:** Superseded PRs show state=closed with a redirect comment;
the surviving PR is merged.
**Estimated:** 10 minutes.

### Step 7: Finish the CI failure taxonomy for the remaining unattributed PRs
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

### Step 8: Apply the Decision Rule to each of the 9 remaining PRs

**Decision Rule** (apply mechanically once each PR's category is known):

| Category | Action |
|---|---|
| Combined CI status = success AND mergeable = true | Merge now |
| Combined CI status = success AND mergeable = false | Rebase (or ask the fleet agent to re-run on latest `main`) — do not merge unresolved conflicts |
| Genuine code bug in the PR's own diff (e.g. clippy failure) | Do not merge; either fix inline (small, obvious fixes only — e.g. terraphim-clients#26's `field_reassign_with_default`) or comment asking the agent/owner to re-run |
| CI-runner infrastructure bug (e.g. `rustup-with-perms`), unrelated to the PR's diff | Re-run the workflow once (infra flakes often clear); if it fails identically twice, file a separate infra ticket and do not block the PR on it |
| ADF-gate-only failure (adf/pr-reviewer, adf/validation, adf/verification) with native-ci green | Check whether it correlates with the bigbox Anthropic provider outage noted in this session's earlier findings; if so, treat as a review-agent availability problem, not a PR defect — do not close or force-merge, wait for provider health to recover and let the gate re-run |
| PR superseded/duplicated by another (e.g. terraphim-clients#18 vs #20, both Refs #2366) | Compare diffs; keep the more complete/correct one, close the other with a comment linking to the survivor |

**Depends on:** Step 7 (needs full taxonomy) and Step 5 (the allowlist fix
must be live before any of these can auto-merge even if CI is green).
**Test/Verification:** Post-execution, `terraphim-clients#18`/`#20` overlap
is resolved to exactly one open or merged PR, not two; every other PR is
either merged, has a specific fix-and-retry action recorded against it, or
is closed with a linked reason.
**Estimated:** 45–60 minutes across 9 PRs, dominated by the two duplicate
`#2366` PRs needing an actual diff comparison.

## Rollback Plan
- **Step 4/5 (merge/deploy of chosen fix):** if the deployed fix turns out wrong
  post-deploy (e.g. still doesn't recognise a login), revert the merge commit
  on the selected repo's main branch, rebuild/deploy the previous binary from
  `/usr/local/bin/adf.pre-allowlist-fix` or the prior known-good source state.
- **Step 6 (close superseded PRs):** if the chosen fix is reverted, re-open the
  runner-up PR or create a new one.
- **Step 8 (per-PR merges):** each merge is a normal Gitea PR merge — revertible
  via `git revert` on the target branch if a merged PR turns out broken.
- No schema/data migrations involved; rollback is git-native throughout.

## Dependencies
No new crate dependencies. This plan is entirely Gitea-API-level PR/issue
operations plus one standard Rust build/deploy to bigbox.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Which repo/branch built the current bigbox binary? | **Blocking; evidence conflict unresolved** | session agent |
| Which allowlist fix survives (#3065 vs #70/#69)? | Pending Step 1 | session agent + Alex |
| Does the surviving fix cover all six fleet logins? | Pending Step 3 | session agent |
| Close/redirect superseded allowlist PRs | Pending Step 6 | session agent |
| Deploy chosen fix to bigbox | Pending human approval and Steps 1–4 | Alex |
| CI taxonomy for remaining unattributed PRs | Pending | session agent |
| Duplicate resolution for terraphim-clients#18 vs #20 (issue #2366) | Pending Step 5 completion | session agent |

## Approval

- [x] Technical review complete
- [ ] Binary provenance resolved by hash/build artefact (Step 1)
- [ ] Surviving allowlist fix selected (Step 2)
- [ ] Six-login coverage verified (Step 3)
- [ ] Human approval received to execute Steps 4–8
