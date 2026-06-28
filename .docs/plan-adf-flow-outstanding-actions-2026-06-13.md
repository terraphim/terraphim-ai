# Plan: ADF Flow Outstanding Actions

**Status**: Draft for approval
**Author**: Terraphim AI
**Date**: 2026-06-13 10:02 BST
**Sources**: `.docs/plan-adf-flow-fix-2026-06-10.md`, implementation audit on 2026-06-13, Gitea PR `#2318`, Gitea issues `#2285`, `#2301`, `#2334`, open duplicate `[ADF] Auto-merge failed` issues
**Purpose**: Close the gap between the validated PR gate foundation and a fully unattended ADF issue-to-merge flow.

---

## 1. Current implementation status

The foundation is in place, but the flow is not fully implemented.

| Area | Status | Evidence |
|---|---|---|
| Canonical PR gate contract | Complete | PR `#2318` merged; issues `#2301` and `#2334` closed. |
| Native PR gate producer path | Complete enough for foundation | `PrGateResult` branch validated and merged; terminal gate statuses were proven in the previous session. |
| Auto-merge on `PrGateResult` | Partial | Branch `origin/task/adf-flow-fix-phase1-automerge` contains `22e64bd4c fix(orchestrator): unify auto-merge on PrGateResult commit statuses`, but full merge/deploy/proof is not confirmed. |
| Auto-merge config/evidence priority | Partial | Branch `origin/task/2301-auto-merge-config` contains `e6259dc3c feat(orchestrator): wire AutoMergeConfig and prioritize gate diff evidence`. |
| Bounded remediation loop | Not implemented | Issue `#2285` remains open; no confirmed persisted state machine, kill-switch, or cold-start guard. |
| Failure issue dedup | Not implemented | Duplicate `[ADF] Auto-merge failed` issues still exist for the same PRs, including repeated `#1970`, `#2410`, `#2414`, `#2417`, `#2433`, `#2443`-`#2446`. |
| `implementation-swarm` cooldown | Not implemented as planned | Config evidence still shows short `grace_period_secs = 30`, not the planned one-hour no-work cooldown. |
| Branch protection alignment | Not proven | No verified report that all `terraphim/*` repos have exact emitted contexts configured. |
| Evidence quality and timeout tuning | Partial | Prioritised diff evidence exists on a branch; no confirmed `evidence_truncated`, doc-only head handling, or `pr_gate_timeout_secs`. |
| End-to-end unattended proof | Not implemented | No confirmed `.docs/validation-report-adf-flow-fix.md`; duplicate issue cascade remains active. |

## 2. Disciplined research validation

### Essential questions check

| Question | Answer | Evidence |
|---|---|---|
| Energising? | Yes | The remaining failures create noisy issues, blocked auto-merge, and manual babysitting. |
| Leverages strengths? | Yes | The work extends existing Terraphim ADF, Gitea, persistence, runner, and automata infrastructure. |
| Meets real need? | Yes | The implementation audit found active duplicate auto-merge issues and an open remediation-loop blocker. |

**Proceed**: Yes, 3/3 YES.

### Problem statement

ADF has a validated canonical PR gate foundation, but it still does not provide the promised closed loop from issue selection through PR review, remediation, re-review, and merge. The outstanding work is concentrated in auto-merge consumption, remediation safety, tracker hygiene, operational cooldowns, evidence quality, and live proof.

### Vital few constraints

| Constraint | Why it is vital | Evidence |
|---|---|---|
| Finish `PrGateResult` consumption before remediation | Remediation depends on reliable fresh gate results | Phase 1 branch exists but is not proven as deployed. |
| Stop duplicate issue creation before enabling more automation | Otherwise remediation and monitoring amplify tracker noise | Open duplicate `[ADF] Auto-merge failed` issues still exist. |
| Keep remediation disabled until stampede guards are proven | Prevents recurrence of the `#2264` cold-start stampede | `#2285` remains open and explicitly blocks redeploy. |

### Assumptions

| Assumption | Basis | Risk if wrong | Mitigation |
|---|---|---|---|
| Branches `task/adf-flow-fix-phase1-automerge` and `task/2301-auto-merge-config` remain viable starting points | Both branches are present on remotes and contain relevant commits. | They may be stale against current main. | Rebase before implementation, run focused orchestrator tests, and avoid cherry-picking blindly. |
| `terraphim_persistence` can hold remediation loop state | Existing plan and crate graph identify it as available infrastructure. | Schema/lifecycle may not fit orchestrator restart semantics. | Build a small state adapter and prove restart behaviour with integration tests. |
| Duplicate issue cascade comes from auto-merge failure handling, not only monitor bots | Open issue titles repeat the same PR numbers. | Multiple emitters may need dedup. | Start with auto-merge dedup, then add a shared ADF alert dedup key if duplicates continue. |

### Out of scope

- No new verdict format.
- No retired ADF bash build-runner revival.
- No unbounded remediation loop.
- No full scheduler rewrite.
- No force-merge without native CI green and explicit human authorisation.

## 3. Disciplined design plan

### Target flow

```text
Gitea issue
-> implementation-swarm
-> branch + PR
-> native-ci / build (push)
-> adf/pr-reviewer + adf/validation + adf/verification
-> PrGateResult evaluation
-> bounded remediation on same branch if needed
-> fresh re-review
-> auto-merge only on fresh clean gates
```

### Phase A: Reconcile and land Phase 1 branches

**Goal**: Make `PrGateResult` the effective auto-merge input on the active main/deployed binary.

Actions:

1. Rebase `origin/task/adf-flow-fix-phase1-automerge` onto current `origin/main`.
2. Reconcile with `origin/task/2301-auto-merge-config`; keep `AutoMergeConfig` only if it remains minimal and directly used.
3. Confirm `auto_merge_impl.rs`/`pr_poller.rs` evaluate current head statuses plus canonical `adf:gate-result` blocks.
4. Keep `pr_review::parse_verdict` only for structural-review human report parsing and telemetry, not merge qualification.
5. Add or update tests for `pass`, `concerns`, `fail`, stale `head_sha`, missing status, malformed gate block, and blocking findings.

Exit criteria:

- Phase 1 code is merged to main.
- Deployed ADF binary includes the merge.
- At least one real or synthetic PR reaches auto-merge eligibility using `PrGateResult`, not prose verdicts.

### Phase B: Stop duplicate failure issue creation

**Goal**: Make ADF issue creation idempotent before adding more automation.

Actions:

1. Implement stable issue-body key: `ADF-Failure-Key: <owner>/<repo>#<pr>:<head_sha>:<failure_kind>`.
2. Search existing open issues by key before creating a new issue.
3. If an issue exists, add a recurrence comment instead of creating a duplicate.
4. Backfill keys where practical for open duplicate `[ADF] Auto-merge failed` issues.
5. Batch-close duplicate issues after the dedup code is deployed, keeping one canonical issue per PR/head/failure kind.

Exit criteria:

- No new duplicate auto-merge failure issues over two gate reconcile cycles.
- Existing duplicate set for `#1970`, `#2410`, `#2414`, `#2417`, `#2433`, `#2443`-`#2446` is closed or consolidated.

### Phase C: Implement remediation safety prerequisites

**Goal**: Close `#2285` without enabling remediation by default.

Actions:

1. Add `[remediation] enabled = false` config default.
2. Add runtime toggle support in `adf-ctl` or equivalent operator path.
3. Persist remediation state per `{owner}/{repo}#{pr}` using `terraphim_persistence`.
4. Store `state`, `attempt_count`, `last_head_sha`, `last_finding_fingerprints`, and `updated_at`.
5. Add cold-start guard: pre-existing findings are not auto-dispatched on restart unless explicitly backfilled.
6. Add project scoping: remediation dispatch only for explicitly listed projects with a resolvable implementation agent.
7. Add fleet caps: max 1 concurrent remediation per PR and max 2 fleet-wide.

Exit criteria:

- Tests cover backlog restart, max=0 disabled, project without agent, cap enforcement, and no re-dispatch for already attempted SHAs.
- Issue `#2285` can be closed with test and config evidence.

### Phase D: Implement bounded remediation loop

**Goal**: Route findings back to implementation without stampedes or branch duplication.

Actions:

1. Implement states: `review_pending`, `findings_present`, `remediation_running`, `re_review_pending`, `clean`, `escalated`.
2. Fingerprint findings using `{agent, severity, file, normalised finding text}` with `terraphim_automata` normalisation where useful.
3. Dispatch remediation to the existing `implementation-swarm` on the same PR branch.
4. Include findings, head SHA, branch name, attempt number, and remaining budget in the prompt.
5. Stop on clean gates, budget exhausted, no-progress fingerprint repeat, unparseable verdict, hard P0, repeated native CI failure, or merge conflict.

Exit criteria:

- One seeded-finding PR produces exactly one remediation dispatch.
- Remediation push triggers fresh native CI and all three ADF gates.
- Loop terminates as `clean` or `escalated` with persisted state.

### Phase E: Fix operational cooldown and branch protection

**Goal**: Remove avoidable ADF noise and align gates with emitted statuses.

Actions:

1. Add a no-work ready-check before spawning `implementation-swarm` LLM work.
2. Enforce one-hour cooldown after no-work exits.
3. Verify whether the orchestrator honours `max_ticks`/`grace_period_secs` for this use case; if not, add an orchestrator-native `next_eligible_at` guard rather than a shell-only workaround.
4. Audit branch protection for every `terraphim/*` repo covered by ADF.
5. Treat 404 as “no rule exists, create it” and 403 as an explicit out-of-scope permission issue.
6. Required contexts must match emitted contexts: `native-ci / build (push)`, `adf/pr-reviewer`, `adf/validation`, `adf/verification` where supported.

Exit criteria:

- `implementation-swarm` spawns 0-1 times per hour when no ready work exists over a 3-hour observation window.
- Branch protection report lists every covered repo and exact required contexts.

### Phase F: Complete evidence quality and timeout tuning

**Goal**: Prevent false positives from doc-only tips, truncation, and near-cap gate durations.

Actions:

1. Add explicit doc-only head classification in PR gate evidence.
2. Present cumulative PR diff as primary evidence and head-commit delta separately.
3. Emit `evidence_truncated: true` when prompt/comment evidence bounds truncate a diff.
4. Instruct gates that truncation alone is `concerns`, not a blocking finding.
5. Add configurable `pr_gate_timeout_secs` with default `300`; raise only per measured repo/PR need.
6. Add counters for telemetry drain lag and skipped output events.

Exit criteria:

- A doc-only follow-up commit on a real or synthetic PR passes all three gates.
- No truncation-only blocking findings for one week.
- Timeout changes are config-driven and tested.

### Phase G: Live proof and observability

**Goal**: Prove the full flow and make regressions visible.

Actions:

1. Run a low-risk end-to-end proof: issue -> implementation -> PR -> native CI -> three ADF gates -> seeded finding -> remediation -> re-review -> auto-merge.
2. Capture evidence in `.docs/validation-report-adf-flow-fix.md`.
3. Add or schedule the weekly ADF report agent using the weekly report query pattern.
4. Track webhook-to-terminal latency, timeout count, unknown exits, rate-limit hits, panic/crash patterns, slow reconcile ticks, output lag, duplicate suppressions, auto-merge attempts, and block reasons.

Exit criteria:

- Validation report exists with Gitea links, journal evidence, and commit SHAs.
- Weekly report can be generated without manual log archaeology.
- At least one unattended issue-to-merge cycle completes.

## 4. Implementation order

| Order | Phase | Dependency | Reason |
|---:|---|---|---|
| 1 | Phase A | None | Auto-merge must consume the canonical gate contract before downstream automation. |
| 2 | Phase B | Phase A preferred, can start in parallel | Duplicate issue noise must stop before remediation adds more events. |
| 3 | Phase C | Phase A, Phase B | Safety prerequisites block remediation. |
| 4 | Phase D | Phase C | Remediation loop must not exist without persistence and kill-switch. |
| 5 | Phase E | None, can run in parallel | Reduces cost and prevents branch protection surprises. |
| 6 | Phase F | Phase A | Evidence fixes depend on the native PR gate path. |
| 7 | Phase G | All prior phases | Full proof only matters after the loop exists. |

## 5. Test strategy

| Area | Required tests |
|---|---|
| `PrGateResult` auto-merge | Unit tests for statuses, freshness, malformed blocks, blocking findings, `concerns` policy. |
| Dedup | Integration test with fake Gitea: repeated same failure creates one issue then recurrence comment. |
| Remediation safety | Restart/backlog test, disabled config test, project scoping test, cap enforcement test, persisted attempt test. |
| Remediation loop | Seeded finding -> one dispatch -> new head -> fresh gates -> clean/escalated. |
| Cooldown | No-ready-work path does not spawn LLM and respects one-hour next eligible time. |
| Evidence | Doc-only head and truncated diff produce non-blocking evidence outcomes. |
| Live proof | Synthetic webhook and one low-risk real PR validation. |

## 6. Quality gate

### KLS evaluation

| Dimension | Score | Justification | Required fix |
|---|---:|---|---|
| Physical | 5/5 | Clear sections, tables, actions, and exit criteria. | None |
| Empirical | 4/5 | Concrete enough for maintainers; exact files still need confirmation during implementation. | Add file-level design per phase before coding. |
| Syntactic | 5/5 | Ordered phases and dependencies are internally consistent. | None |
| Semantic | 4/5 | Matches audit evidence and Gitea state; live bigbox config still needs confirmation. | Verify live config before Phase E. |
| Pragmatic | 5/5 | Gives actionable phases, tests, and exit criteria. | None |
| Social | 3/5 | Draft requires approval and policy decisions. | Approve phase order, `concerns` policy, and remediation caps. |

**Average score**: 4.3/5.

**Quality decision**: Conditional pass. Ready for issue filing and Phase A/B planning. Do not enable remediation until Phase C safety prerequisites pass.

## 7. Proposed Gitea issues

| Issue | Title | Phase | Blocks |
|---|---|---|---|
| A | Land PrGateResult auto-merge branch and deploy active ADF binary | A | B, C, F, G |
| B | Stop duplicate ADF auto-merge failure issues with stable dedup keys | B | C, G |
| C | Implement remediation kill-switch, persistence, cold-start guard, and project scoping | C | D |
| D | Implement bounded same-branch review-remediation loop | D | G |
| E | Add implementation-swarm ready-check and no-work cooldown | E | G |
| F | Align branch protection across covered terraphim repos | E | G |
| G | Harden PR gate evidence for doc-only heads, truncation, and configurable gate timeouts | F | H |
| H | Run and document full ADF issue-to-merge live proof | G | None |

## 8. Approval checklist

- [ ] Phase order accepted.
- [ ] `concerns` auto-merge policy decided.
- [ ] Remediation kill-switch default-off accepted.
- [ ] Remediation fleet caps accepted: max 1 per PR, max 2 fleet-wide.
- [ ] Issue breakdown approved for filing via `gtr`.
