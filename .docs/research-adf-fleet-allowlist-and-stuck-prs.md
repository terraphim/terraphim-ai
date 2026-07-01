# Research Document: ADF auto-merge allowlist bug and 10 stuck fleet PRs

**Status**: Draft — reviewed 2026-07-01 after deploy-source evidence conflict
**Author**: session agent (Claude Code)
**Date**: 2026-07-01
**Reviewers**: Alex Mikhalev

## Executive Summary

The ADF auto-merge author-allowlist bug has three layers, not one:
1. **Root cause**: `author_is_agent()` in the orchestrator only recognised
   `claude-code`, `root`, and `adf-*`-prefixed logins, rejecting six real fleet
   agents.
2. **Deploy-source question**: Evidence is conflicting. Project instructions
   say the orchestrator binary is built from `terraphim-agents`, while
   bigbox's `~/.zsh_history` shows real `terraphim-ai` task-branch builds of
   `adf`. Neither evidence source alone proves which checkout produced the
   currently-installed `/usr/local/bin/adf` because no candidate build has yet
   been hash-matched to the installed binary. Therefore **repo identity remains
   a blocking provenance question**, not a settled fact.
3. **Remaining work**: 10 PRs are still stuck. One (`terraphim-clients#31`) was
   merged in this session. The other 9 fail CI for at least three independent
   reasons (orchestrator-gate/provider issue, CI-runner rustup proxy bug,
   genuine per-PR clippy failure) that need individual attribution before any
   merge decision.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Directly unblocks North Star priority #2 (ADF stabilisation, 5+ agents reliable overnight, target already past due) |
| Leverages strengths? | Yes | Repo/Gitea archaeology and cross-repo diffing is exactly the kind of investigation this session is suited for |
| Meets real need? | Yes | ~97,000 blocked auto-merge attempts in 48h per terraphim-ai#3024; 11 real PRs sitting stuck for 2+ weeks |

**Proceed**: Yes (3/3 YES).

## Problem Statement

### Description
Two related problems:
1. The ADF orchestrator's auto-merge author gate only recognises
   `claude-code`, `root`, and `adf-*`-prefixed logins. Six real fleet agents
   (`implementation-swarm`, `odilo-developer`, `meta-coordinator`,
   `quality-coordinator`, `security-sentinel`, `test-guardian`) don't match,
   so every PR they open requires manual merge forever, and the orchestrator
   re-logs the rejection every ~30s reconcile tick.
2. 10 of the 11 PRs this bug surfaced are *also* failing their own CI/ADF
   gates for reasons unrelated to the allowlist bug, so fixing the allowlist
   alone would not make them auto-mergeable.

### Impact
- 11 real PRs (bug fixes, feature work, CI unblocks) sitting open 2–4 weeks.
- Orchestrator log spam (one warning per blocked PR per reconcile tick,
  continuously, for potentially all 11 PRs since creation).
- Confusion risk: two competing agent-generated fixes for the same root
  cause already exist (terraphim-agents#70, #69) plus this session's
  independent fix (terraphim-ai#3065) — three fixes, two repos, none merged.

### Success Criteria
- The allowlist fix lands in the repo proven to produce the installed bigbox
  binary, is deployed from a tracked branch, and is confirmed via a subsequent
  auto-merge log line showing a previously-blocked login now clearing the gate.
- No duplicate/competing PRs left open for the same issue.
- Each of the 9 CI-failing PRs has a known, attributed failure cause (even if
  not yet fixed) rather than being lumped under "the allowlist bug."

## Current State Analysis

### Existing Implementation

`crates/terraphim_orchestrator/src/pr_review.rs::author_is_agent()` — pure
function, hardcoded match on `claude-code | root | adf-*`. Called from two
sites: `pr_review::evaluate()` (used in `auto_merge_impl.rs`'s
`poll_pending_reviews`) and `pr_poller::evaluate_pr_gates()`.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Allowlist policy candidate A | `terraphim-ai` repo, `crates/terraphim_orchestrator/src/pr_review.rs` | PR #3065 implements a KG-driven allowlist; bigbox history shows at least one `adf` build from this repo. **Gap found during review:** its current KG synonyms list only `implementation-swarm`, not all six blocked fleet logins. |
| Allowlist policy candidate B | `terraphim-agents` repo, `crates/terraphim_orchestrator/src/pr_review.rs` | Project deployment instructions say this is the orchestrator binary source; materially ahead in some areas (`blocker_kind`, `max_remediation_attempts`, `From<&AutoMergeConfig>`) |
| Competing fix #1 | `terraphim-agents#70` "Fix terraphim-ai#3024: source auto-merge agent-author allowlist from fleet config" | Open; should not be closed until binary provenance proves this repo is not the deploy source |
| Competing fix #2 | `terraphim-agents#69` "Fix terraphim-ai#3028: include allowlist hint in auto-merge author-rejection" | Open; same provenance gate as #70 |
| Issue tracking | `terraphim-ai#3024`, `terraphim-ai#3028` | Filed 2026-06-29 by ADF itself, ~48h before ADF's own remediation agent produced #70/#69; #3024's body records that an *earlier* attempted fix (commit `38f06db0` on a branch that no longer exists) was silently lost |
| Orchestrator config (bigbox) | `/opt/ai-dark-factory/orchestrator.toml` + `conf.d/*.toml` | `max_diff_loc = 10000` matches behaviour seen in the deployed binary, but does not by itself identify the source checkout |

### Data Flow
Orchestrator binary (`/usr/local/bin/adf`, source checkout still to be proven by
hash/build metadata) polls each configured project's Gitea PRs every
reconcile tick → `evaluate_pr_gates`/`evaluate` → rejects any PR whose author
isn't in the 3-entry hardcoded allowlist → re-logs the same rejection
indefinitely, no backoff.

### Integration Points
Gitea REST API (PR list, PR comments, commit statuses, merge). ADF fleet
agents open PRs under their own service-account logins
(`implementation-swarm`, etc.) rather than a shared `claude-code`/`adf-*`
identity — this is itself worth a design question: should fleet agents use a
uniform `adf-<agent-name>` login convention going forward instead of raw
agent names, to make the prefix rule sufficient without a per-agent
allowlist?

## Constraints

### Technical Constraints
- `pr_review.rs` module has an explicit "zero I/O" contract (see its
  top-of-file doc comment) — any KG/file-based allowlist loading must live in
  a separate module (respected in PR #3065 via `agent_allowlist_kg.rs`).
- The deploy-source evidence conflicts: project instructions name
  `terraphim-agents`, while bigbox shell history shows `terraphim-ai`
  task-branch builds. A safe deploy must first identify or reproduce the exact
  binary provenance, then build from the agreed canonical repo.

### Business Constraints
- North Star: ADF stabilisation target was 2026-06-15; it's now 2026-07-01,
  16 days past due, and this specific bug has been silently discarding fleet
  agent work for at least that long (`38f06db0` commit loss mentioned in
  #3024 suggests this bug — or attempts to fix it — predate 2026-06-29).

### Non-Functional Requirements
Not applicable in the traditional latency/throughput sense; the relevant
"non-functional" property here is *auto-merge availability* — currently 0%
for 6 of the fleet's most active agents.

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Fix must land in the repo proven to build `/usr/local/bin/adf` and then be deployed | A merged fix in a non-deployed repo has zero production effect | Evidence currently conflicts; hash-level provenance is required |
| Must not duplicate ADF's own in-flight remediation | Three independent fixes for one bug (agents#70, agents#69, ai#3065) is worse than one, and merging more than one risks conflicting `AutoMergeCriteria` shapes | agents#70/#69 predate this session's fix by 2 days; #3065 uses a different KG-driven design |
| Surviving allowlist fix must cover all six blocked fleet logins | Covering only `implementation-swarm` leaves five known fleet agents blocked | terraphim-ai#3024 impact table lists `implementation-swarm`, `odilo-developer`, `meta-coordinator`, `quality-coordinator`, `security-sentinel`, `test-guardian` |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Fixing all 9 remaining PRs' CI failures in this session | Each requires per-repo, per-PR code changes across 4 unfamiliar repos not checked out locally; out of scope for a research pass |
| Redesigning the fleet-agent login convention (`adf-<name>` uniform prefix) | Valuable idea surfaced during research but is a separate, larger design decision affecting how every fleet agent authenticates to Gitea |
| Auditing every other file that diverged between terraphim-ai's and terraphim-agents' orchestrator copies | Only `pr_review.rs`'s allowlist-relevant diff was pulled; a full reconciliation is its own project |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim-ai repo's orchestrator copy | Candidate deploy source: bigbox zsh history shows `adf` builds from this repo | High until hash provenance is established |
| terraphim-agents repo's orchestrator copy | Candidate deploy source: project deployment instructions say this builds the orchestrator binary | High until hash provenance is established |
| ADF's own remediation agent (produced #70/#69) | Already attempted this exact fix with a different design | Medium — must not close or merge until provenance decides which repo is canonical |
| `terraphim-gitea-runner` CI infrastructure | One observed failure (`unknown proxy name: 'rustup-with-perms'`) is a runner-host bug, not a code bug | Medium — unknown how many other "failing" PRs are runner flakes vs real breakage |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|----------|
| Plan chooses the wrong repo because it trusts shell history or AGENTS.md alone | Medium | High (fix has no production effect) | Reproduce or hash-match the installed binary before merging/closing any competing fix PR |
| Surviving fix only allowlists `implementation-swarm` and leaves five known fleet agents blocked | Medium | High (incident partially persists) | Candidate-fix acceptance gate must verify all six fleet logins clear `author_is_agent` |
| The current bigbox binary was built from an unmerged task branch; merging a fix to main without accounting for off-main deployed changes regresses production behaviour | Medium | Medium-high | Diff current deployed source branch against main before deploying from main |
| terraphim-agents#70/#69 or terraphim-ai#3065 are left open after canonical fix lands, creating future conflicting `AutoMergeCriteria` shapes | Medium | Low-medium (rework, not breakage) | Close superseded PRs only after canonical repo and surviving design are decided |
| Blind-merging the other 9 PRs once CI is "green" ships unrelated regressions | Low if this document's guidance is followed | High | Never merge on `mergeable=true` alone; require green combined CI status, verified per PR |

### Open Questions
1. Which repo/branch actually produced the currently-installed `/usr/local/bin/adf`, proven by matching binary hash, build-id, or deployment artefact path?
2. If the source is a task branch, what commits are deployed off-main and must be preserved before redeploying from main?
3. Is `terraphim-agents` supposed to be a live fork of `terraphim_orchestrator`, or is this itself the architectural bug (an accidental duplication from the polyrepo split that should be collapsed back to one source)?
4. Are terraphim-agents#70 and #69 mutually exclusive or stackable (i.e. does #69 depend on #70's allowlist-source refactor)?
5. Was the `38f06db0` "lost commit" mentioned in issue #3024 ever real, or is that itself a symptom of a broken ADF remediation-tracking mechanism worth its own investigation?

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| terraphim-ai may be the deploy source for `/usr/local/bin/adf` | Bigbox `~/.zsh_history` shows literal `cargo build -p terraphim_orchestrator --release --bin adf` executed inside a terraphim-ai task-branch checkout | If wrong, effort goes toward the wrong repo | No — direct build history exists, but installed-binary hash provenance is not yet proved |
| terraphim-agents may be the deploy source for `/usr/local/bin/adf` | Project deployment instructions state the orchestrator binary is built from terraphim-agents | If wrong, agents#70/#69 target a non-deployed copy | No — documented policy exists, but installed-binary hash provenance is not yet proved |
| The 9 CI failures are independent, not systemic | Spot-checked 4 of 9 job logs, found 3 different causes (allowlist-adjacent ADF-gate failures, a runner rustup bug, a genuine clippy lint) | If wrong, a shared fix could unblock more PRs at once than this document assumes | Partially — 4/9 checked |

## Research Findings

### Key Insights
1. This bug already has an ADF-native paper trail (issues + two competing
   fix PRs) that predates this session's independent discovery — the
   session should reconcile with, not duplicate, that work.
2. PR #3065's KG-driven design is operationally attractive, but its current
   default `recognised_agents.md` is incomplete for #3024: it lists
   `implementation-swarm` but not `odilo-developer`, `meta-coordinator`,
   `quality-coordinator`, `security-sentinel`, or `test-guardian`.
3. `crates/terraphim_orchestrator` is duplicated across two Gitea repos with
   diverging implementations. This is now part of the incident, not merely a
   follow-up, because the competing fixes are in different copies.
4. The deployed binary on bigbox may have been built from a task branch, not
   from `main`. This means other changes (e.g. `max_diff_loc: 10_000`,
   `blocker_kind` classification) may be silently live in production without
   having landed on the branch selected for redeploy.
5. CI failures on the 9 non-#31 stuck PRs are **not** one problem: at least
   one ADF-gate-only failure pattern (adf/pr-reviewer, adf/validation —
   plausibly downstream of the Anthropic provider outage found earlier this
   session), one CI-runner infrastructure bug (broken `rustup-with-perms`
   proxy), and one genuine per-PR code defect (clippy `field_reassign_with_default`
   in terraphim-clients#26).

### CI Failure Taxonomy (partial — 4/9 PRs job-log-verified)

| PR | Failing gate | Root cause found |
|---|---|---|
| terraphim-clients#26 | native-ci | Genuine code bug: `clippy::field_reassign_with_default` in `crates/terraphim_agent/src/shared_learning/wiki_sync.rs:536-537` |
| terraphim-core#9 | native-ci | CI runner infra bug: `error: unknown proxy name: 'rustup-with-perms'` — broken rustup toolchain proxy on the runner host, unrelated to PR content |
| terraphim-config-persistence#7 | adf/pr-reviewer, adf/validation, adf/verification (native-ci itself passes) | Not yet isolated; tests pass locally in the log tail, so failure is in the ADF review/validation agent step itself, not the build |
| terraphim-agents#53 | native-ci | Job log fetch returned empty via the jobs-list endpoint used; needs the same per-job drill-down as the other PRs |
| terraphim-clients#54, #35, #34, #21, #18, terraphim-clients#20 (conflicts) | mixed adf/* and native-ci | Not yet drilled into — remaining PRs still need attribution or conflict/duplicate resolution |

### Relevant Prior Art
- terraphim-ai#3024 / #3028: same-bug issues filed by ADF, with a detailed
  48h impact table this document's estimate should defer to.
- terraphim-ai#3026 (closed unmerged): earlier attempt at fleet-config-sourced
  allowlist.
- terraphim-ai#3031 (closed unmerged): earlier attempt at allowlist-hint
  messaging.
- terraphim-agents#42 (merged): "classify auto-merge blockers by kind" —
  introduces `blocker_kind`, but in the non-deployed repo copy.
- terraphim-agents#48 (merged): "unify auto-merge on PrGateResult commit
  statuses" — another orchestrator change absent from terraphim-ai's copy.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|-------------------|
| Prove the exact repo/branch that built the current bigbox binary | Avoid landing/deploying a fix in the wrong repo; know what else is silently deployed off-main | 30–60 min (hash-match installed binary against candidate builds, inspect deployment scripts/history) |
| Verify candidate allowlist coverage for all six fleet logins | Ensure the chosen fix actually closes #3024 rather than only unblocking `implementation-swarm` | 10–15 min |
| Job-log drill-down for remaining unattributed PRs | Complete the CI failure taxonomy before any merge attempt | 30–45 min |
| Diff full terraphim-ai vs terraphim-agents `terraphim_orchestrator` trees | Quantify total divergence, scope a reconciliation | 1–2 hours, separate task |

## Recommendations

### Proceed/No-Proceed
Proceed to design, but keep repo identity as the first blocking gate. The
design's focus is:
1. Prove which repo/branch produced the installed binary and choose the
   surviving fix accordingly.
2. Verify the surviving fix covers all six known fleet agent logins.
3. Finish CI failure attribution for the 9 remaining stuck PRs.
4. Apply a mechanical merge/fix/close decision rule.

### Scope Recommendations
- Design phase should produce: (a) a binary-provenance gate, (b) a branch plan
  for either #3065 or agents#70/#69 depending on that gate, (c) a per-PR
  remediation plan for the 9 CI-failing PRs *only after* their failure causes
  are fully attributed.
- Do not expand scope to the terraphim-ai/terraphim-agents duplication
  itself in this pass — flag it as a follow-up architecture issue.

### Risk Mitigation Recommendations
- Do not merge #3065 or close agents#70/#69 until the binary provenance gate
  is complete.
- After merging the chosen fix, explicitly build from the selected canonical
  branch and deploy to bigbox; do not assume merging is enough.
- Do not merge any of the 9 remaining PRs until each has a filled-in taxonomy
  row and passes the Decision Rule.

## Next Steps

If approved:
1. **Binary-provenance spike** (30–60 min): prove which repo/branch built the
   current bigbox `/usr/local/bin/adf` by hash-matching or deployment artefact
   trace, and record off-main commits if any.
2. **Choose one fix and verify coverage**: if `terraphim-ai` is proven
   canonical, use #3065 only after adding/verifying all six fleet logins; if
   `terraphim-agents` is proven canonical, review #70/#69 and either merge them
   or port #3065's KG design there with the same six-login coverage.
3. **Deploy the chosen fix** (owner's call on deploy timing): build from the
   selected canonical branch, copy to `/usr/local/bin/adf`, restart
   `adf-orchestrator`.
4. **Close/redirect superseded PRs**: only after the chosen fix is merged and
   deployed.
5. **Finish the CI failure taxonomy** for the remaining unattributed PRs.
6. **File a follow-up issue** for the terraphim-ai/terraphim-agents
   `terraphim_orchestrator` duplication as a standalone architecture problem.

## Appendix

### Reference Materials
- terraphim-ai#3024, #3028, #3026, #3031 (Gitea issues/PRs)
- terraphim-agents#70, #69, #48, #42 (Gitea PRs)
- Session's own PR: terraphim-ai#3065
- Bigbox zsh history entry (line 2361): `git clone --depth 1 -b task/2301-pr-gate-result-contract … terraphim-ai.git` followed by `cargo build -p terraphim_orchestrator --release --bin adf`

### Code Snippets
See §CI Failure Taxonomy for the two concrete job-log excerpts (clippy
lint, rustup proxy error) captured during this research pass.
