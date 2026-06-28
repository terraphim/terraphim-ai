# Plan: Fix ADF Flows Once and For All

**Status**: Draft for approval
**Author**: Terraphim AI
**Date**: 2026-06-10 16:49 BST
**Merged update**: 2026-06-10 16:58 BST -- folded in operational hardening items from the comparison review
**Sources**: `.docs/adf-weekly-activity-2026-06-03-to-2026-06-10.md`, `long_session.md` (session ses_156a0aff, 2026-06-08 to 2026-06-10), `.docs/research-adf-flow-remediation.md`, `.docs/design-adf-flow-remediation.md`, `.docs/design-adf-pr-gate-result-redesign.md`
**Related issues**: #2301 (PrGateResult, implemented in PR #2318), #2275 (review routing failover), #2285 (remediation loop prerequisites), #2334 (head-ref evidence)

---

## 1. Where we actually are

The week of 2026-06-03 to 2026-06-10 proved that the hard part is done. What remains is consolidation, not invention.

### Done and live-validated (do not redo)

| Capability | Evidence |
|---|---|
| `PrGateResult` canonical contract (`adf:gate-result` JSON block), fail-closed parsing | PR #2318, 31 unit tests, live malformed-JSON validation on `88b008a6` |
| 300s PR gate wall-clock timeout, fail-closed | Live timeout validation on `ca4a6f2ad`; 20 timeouts all handled correctly during the week |
| Native PR gate producer pipeline: orchestrator-owned bounded prompts (~109k chars), `terraphim_automata` concept matching, head-ref evidence fetch | Commits `902f8167b`, `2575c3604`, `900c00a88`; all 3 gates `pass (4/5)` |
| Orchestrator owns comments and terminal statuses (no script-side `gtr comment` / curl) | Deployed as adf v1.20.2 on bigbox |
| Native CI runner reliability (stuck PickTask + orphan-on-skip) | #2185 merged (`445319bdd`) |
| Cross-repo gate coverage (6 Gitea repos) | 30 PRs, 82 terminal statuses posted this week |

### Broken or missing (this plan)

1. **PR #2318 itself is blocked** by a reviewer false positive on the final doc-only commit `866460a1e` (truncated diff evidence + traceability gap). The implementation commits all passed.
2. **Auto-merge still consumes the old prose verdict format** (`Confidence Score: N/5`, `Inline Findings`) while gates now emit `adf:gate-result`. Result: parse failures, stale-verdict vetoes, and **zero auto-merges all week**. The 5/5 confidence threshold is unreachable (observed range 2-4/5).
3. **No bounded review-remediation loop.** Findings are posted but nothing routes them back to implementation. The previous attempt (#2264) was rolled back after the cold-start stampede (~46 dispatches); prerequisites are tracked in #2285 and the kill-switch is still not config-wired.
4. **Duplicate auto-merge failure issue cascade** (~20 duplicate issues per PR per day, issues #2294-#2313) — no idempotency key.
5. **`implementation-swarm` hot-loop**: re-spawned every `eval_interval_secs = 300` instead of hourly; 8 of the week's 10 `unknown` exits are its ~30s pre-check failures.
6. **Branch protection gaps**: all `terraphim/*` sub-repos return 404 on branch protection (no rules), so the auto-merge gate silently skips them.
7. **Evidence quality false positives**: 60k comment truncation, doc-only head commits judged against implementation evidence, merge-base failures on some PRs (e.g. "diff unavailable" finding).
8. **Real gate timeouts under load**: PR #2365 lost 2 of 3 gates at ~327s; telemetry drain lags 600-1900 events per 30s interval during inference.

## 2. Strategy

**Make `PrGateResult` the single source of truth for the whole PR lifecycle**, then build the bounded remediation loop on top of it, persisting loop state in `terraphim_persistence`. Everything else is hygiene executed with existing tools (`gtr`, Gitea API, conf.d TOML). No new services, no new formats, no orchestrator scheduler rewrite.

Infrastructure reuse map:

| Need | Existing Terraphim infrastructure |
|---|---|
| Verdict format | `PrGateResult` / `adf:gate-result` (landed in #2318) — extend consumption, never add a second format |
| Loop state, dedup keys, attempt budgets | `terraphim_persistence` (memory + sqlite profiles already in the orchestrator crate graph) — also satisfies the Q2 goal "persistence used by 2+ agents" |
| Finding fingerprints for no-progress detection | `terraphim_automata` normalised term matching (already a dependency, already used for gate evidence) |
| Model failover on quota exhaustion | `terraphim_router` tier routing (`default_tier = review_tier` haiku fix from #2275 already proven) |
| Issue lifecycle, dedup comments, batch close | `gtr` (gitea-robot) CLI/MCP |
| Build gate | `terraphim-gitea-runner` + `.gitea/workflows/native-ci.yml` (`native-ci / build (push)`) — ADF bash build-runners stay retired |
| Live testing | `adf-ctl` + synthetic webhook to `http://172.18.0.1:9091/webhooks/gitea` (proven in the 2301 session) |
| Agent discipline | Existing `skill_chain` configs (`disciplined-research/design/implementation/verification/validation`, `structural-pr-review`) — no prompt rewrites beyond the remediation dispatch prompt |

## 3. Phases

### Phase 0 — Land PR #2318 and close #2301 (half a day)

The contract is production-validated; the PR is blocked by a known false-positive class.

1. Squash or rebase `task/2301-pr-gate-result-contract` so the head commit is the implementation, not the doc-only validation record (the doc rides along in the same commit). Push; gates re-run on a head whose diff matches the evidence.
2. If the reviewer still blocks on truncated-diff grounds, apply the established force-merge override pattern: GA/native-ci green + per-PR human authorisation + justification in the merge commit. Do not weaken the gate to pass it.
3. Close #2301 with evidence links; comment on #2334.

**Exit criterion**: PR #2318 merged to main; deployed binary and main branch agree.

### Phase 1 — Unify auto-merge on PrGateResult (1 day)

Retire prose verdict parsing as a merge input. Auto-merge already has every signal it needs in commit statuses and gate-result comments.

1. `auto_merge_impl.rs`: evaluate merge eligibility from **commit statuses for the current head SHA** — `native-ci / build (push)`, `adf/pr-reviewer`, `adf/validation`, `adf/verification` all `success` — plus the parsed `PrGateResult` blocks for freshness (`head_sha` match) and `blocking_findings == 0`.
2. Replace the unreachable 5/5 confidence threshold with the gate policy already encoded in `pr_gate_result.rs`: `pass` with no blocking findings merges; `concerns` with no blocking findings merges (policy decision below); `fail` or any blocking finding vetoes.
3. Keep `pr_review::parse_verdict` scoped to the structural reviewer's human report only (as the redesign doc specifies); it must no longer veto or qualify merges on its own.
4. Stale-verdict handling stays: `head_sha` mismatch means re-review, not failure.
5. Split reconciliation responsibilities so PR gate process polling, output extraction, comment posting, and terminal status posting run on a high-priority lane. Auto-merge enumeration, stale PR scans, provider probing, and tracker hygiene must not block terminal status posting for already-exited PR gate agents.
6. Add an operational SLO for the high-priority lane: when a PR gate process exits normally, the corresponding Gitea terminal status should be posted within 30 seconds under normal load. Slow auto-merge scans may delay merge attempts, but must not leave PR gate statuses pending.

**Policy decision needed (Alex)**: does `concerns` (blocking_findings = 0) auto-merge, or hold for human review? Recommendation: auto-merge for sub-repos, hold for `terraphim-ai` initially.

**Exit criterion**: at least one real PR auto-merges with journal evidence; no parse-failure log lines in two reconcile cycles; exited PR gate agents post terminal statuses without being blocked by auto-merge enumeration.

### Phase 2 — Bounded review-remediation loop on terraphim_persistence (2-3 days)

Implements Fix 0b from `design-adf-flow-remediation.md`, with the #2264 stampede lessons as hard requirements.

1. **State machine** (`review_pending → findings_present → remediation_running → re_review_pending → clean | escalated`), persisted per PR lineage in `terraphim_persistence` (sqlite profile, memory cache). Key: `{owner}/{repo}#{pr}`. Fields: attempt count (budget 3), last finding fingerprints, last head SHA, state, updated_at.
2. **Finding fingerprints** via `terraphim_automata` normalisation: `{agent, severity, file, normalised finding text}`. Same fingerprint on two consecutive heads = no-progress = escalate.
3. **Stop conditions** exactly as designed: clean gates, budget exhausted, no-progress, unparseable verdict, hard P0, repeated native-CI failure, merge conflict. On stop: one deduplicated issue (Phase 3 dedup key), state `escalated`, no further dispatch until head changes or human re-trigger.
4. **Stampede guards (from #2285, non-negotiable before enabling)**:
   - Cold-start: on orchestrator start, mark pre-existing findings `escalated`-eligible only via explicit backfill command, never auto-dispatch the backlog.
   - Per-tick dispatch cap (reuse `max_dispatches_per_tick`) applied to remediation specifically; max 1 concurrent remediation per PR, max 2 fleet-wide.
   - **Kill-switch wired into config** (`[remediation] enabled = false` default) and into `adf-ctl` for runtime toggle.
   - Project scoping: remediation only for projects explicitly listed; no cross-project reach.
5. **Remediation dispatch prompt**: route to `implementation-swarm` with the existing `disciplined-implementation` skill chain; include findings, head SHA, branch name, attempt number/budget; require same-branch fixes, no new PR.

**Exit criterion**: integration tests for every stop condition; live proof on one seeded-finding PR: exactly one remediation dispatch, one re-review, loop terminates.

### Phase 3 — Dispatch and tracker hygiene (1 day, mostly config/API)

1. **implementation-swarm cooldown**: `max_ticks = 1` + `grace_period_secs = 3600` in `/opt/ai-dark-factory/conf.d/terraphim.toml`; file-based cooldown guard as fallback if the orchestrator ignores the fields. Also add a cheap pre-check (`gtr ready` non-empty) so no-work runs exit before spawning the LLM — this kills the 8 weekly `unknown` exits at the source.
2. **Issue dedup**: dedup key `ADF-Failure-Key: <owner>/<repo>#<pr>:<head_sha>:<failure_kind>` in issue body; on recurrence, `gtr comment` instead of create. Batch-close #2294-#2313 and successors via `gtr` after deploy.
3. **Branch protection alignment** via Gitea API on all `terraphim/*` repos: require exactly the contexts each repo emits (`native-ci / build (push)` + three `adf/*` contexts where PR agents cover the repo via `extra_projects`). Treat 404 as "no rules yet — create", not as an error to skip. `zestic-ai/odilo` stays out of scope (token lacks admin).
4. **Runner/process hygiene**: kill duplicate `terraphim-gitea-runner` processes after checking for active jobs; confirm deploy procedure (build in dedicated worktree, `--check`, atomic rename, deploy from `gitea` remote) is followed — already documented in the rebuild/redeploy procedure doc.
5. **Deployment/rollback discipline**: every live ADF binary or config change must have named backups for `/usr/local/bin/adf`, `/opt/ai-dark-factory/adf`, and modified `conf.d/*.toml`; install via temporary path plus atomic rename; verify `adf --version`, systemd `MainPID`, restart timestamp, and synthetic webhook proof before declaring success. Rollback must be a documented restore of the previous binary/config backup plus service restart.

**Exit criterion**: 0-1 implementation-swarm spawns per hour over a 3-hour window; no new duplicate failure issues over two gate reconcile cycles; branch protection API returns expected contexts on every `terraphim/*` repo.

### Phase 4 — Evidence quality and timeout tuning (1 day)

Targets the false-positive and real-timeout classes from the weekly report.

1. **Doc-only heads**: in `pr_gate_context.rs`, classify the head commit; when the cumulative PR diff is the review subject, present the full PR diff (base...head) as primary evidence and label the head-commit delta separately, so a doc-only tip no longer contradicts implementation evidence.
2. **Truncation honesty**: when the 60k comment or prompt evidence bound truncates a diff, emit an explicit `evidence_truncated: true` marker into the prompt and instruct gates that truncation alone is not a blocking finding — flag as `concerns` instead. Keep fail-closed for genuinely missing evidence (merge-base failure already falls back through `build_diff_ranges()`).
3. **Timeout calibration**: make the 300s cap per-gate configurable (`pr_gate_timeout_secs` in agent config), keep 300s default, raise only for repos/PRs with measured need (PR #2365 lost gates at ~327s — within noise of the cap). Pair with `terraphim_router` haiku-tier default so quota exhaustion does not masquerade as slowness (#2275 pattern).
4. **Telemetry drain**: raise the output-event drain budget or sample-skip with counters; the lag is cosmetic but hides real signals during incidents.

**Exit criterion**: a doc-only follow-up commit on a real PR passes all three gates; no truncation-induced blocking findings in a week of operation.

### Phase 5 — Live proof and standing observability (half a day + ongoing)

1. **End-to-end proof** on one low-risk PR, the design doc's Step 6: issue → implementation-swarm (disciplined skills) → PR → native-ci → 3 gates → seeded finding → 1 remediation → re-review → auto-merge. Capture journal + Gitea evidence into `.docs/validation-report-adf-flow-fix.md`.
2. **Weekly activity report as a cron'd ADF agent** (reuse the exact queries from this week's report) so regressions surface within a week, not on demand.
3. Track permanent ADF health metrics in the weekly report: webhook accepted to terminal status latency, timeout count, unknown exit classifications, rate-limit hits, crash/panic patterns, slow reconcile ticks, output telemetry lag, duplicate issue suppressions, auto-merge attempts, and auto-merge block reasons.
4. Update memory/handover docs; close out the remediation design docs as implemented.

**Exit criterion**: the north-star measure — 5+ agents running overnight with zero manual interventions and at least one unattended issue→merge cycle.

## 4. What we will NOT do

- No second verdict format, no prose-parsing revival — `PrGateResult` only.
- No re-enabling retired ADF bash build-runners; native-ci stays the build gate.
- No unbounded remediation: kill-switch default-off, budget 3, fleet cap 2.
- No orchestrator scheduler rewrite.
- No forced merges without GA/native-ci green and explicit per-PR human authorisation.

## 5. Proposed issue breakdown (to file with gtr on approval)

| Issue | Phase | Title |
|---|---|---|
| A | 0 | Unblock and merge PR #2318; close #2301 |
| B | 1 | Auto-merge: evaluate PrGateResult statuses, retire prose verdict threshold |
| C | 2 | Bounded remediation loop with terraphim_persistence state + stampede guards (supersedes #2264, depends on #2285 items) |
| D | 3 | implementation-swarm cooldown + ready-check pre-gate |
| E | 3 | Failure-issue dedup key + batch-close #2294-#2313 |
| F | 3 | Branch protection alignment across terraphim/* repos |
| G | 4 | Gate evidence: doc-only head handling + truncation markers + per-gate timeout config |
| H | 5 | End-to-end live proof + weekly report cron agent |
| I | 1/3/5 | Operational hardening: PR gate reconciliation priority lane, deploy/rollback runbook checks, ADF health metrics |

Dependency edges: A → B → C; D, E, F independent; G after B; H last. Sequenced this is roughly **5-6 working days**, with Phases 0-1 deliverable by 2026-06-12 and the full loop before the 2026-06-15 north-star checkpoint.

## 6. Disciplined research validation

### Essential questions check

| Question | Answer | Evidence |
|---|---|---|
| Energising? | Yes | ADF flow breakage creates wasted LLM spend, blocked PRs, duplicate issues, and manual intervention. |
| Leverages strengths? | Yes | The plan reuses Terraphim-specific orchestrator, Gitea runner, persistence, automata, router, and disciplined agent infrastructure. |
| Meets real need? | Yes | The weekly report shows 20 timeout failures, 10 unknown exits, 0 auto-merges, parse failures, and cross-repo gate activity needing consolidation. |

**Proceed**: Yes, 3/3 YES.

### Vital few constraints

| Constraint | Why it is vital | Evidence |
|---|---|---|
| `PrGateResult` is the only merge verdict source | Prevents prose parser drift and false-green or false-red PR gates | PR #2318 live validation and weekly parse-failure evidence |
| Remediation is bounded and kill-switch protected | Prevents a repeat of the #2264 cold-start stampede | #2285 prerequisites and documented fleet caps |
| Native CI remains the build gate | Avoids reviving retired bash build-runners and aligns branch protection with emitted statuses | #2185 runner reliability and `native-ci / build (push)` branch protection context |

### Key assumptions

| Assumption | Basis | Risk if wrong | Mitigation |
|---|---|---|---|
| PR #2318 can be landed without redesigning the gate contract | Final `900c00a88` run posted three `success` statuses from canonical results | If blocked again, Phase 0 stalls | Rebase/squash doc-only head, rerun gates, use explicit human override only with native CI green |
| `terraphim_persistence` is suitable for loop state | Already in orchestrator crate graph and supports memory/sqlite profiles | State integration may reveal schema or lifecycle gaps | Implement behind disabled `[remediation] enabled = false`, with integration tests before enabling |
| Branch protection changes are permitted for `terraphim/*` repos | Existing token has owner-level Terraphim access in prior research | API scope may differ per repo | Treat 403 separately, record excluded repos, do not silently skip 404 create-rule cases |

### Research gate result

The plan satisfies Phase 1 research criteria: it identifies the current failure modes, maps existing infrastructure, names constraints, surfaces policy decisions, and limits scope. No additional research is required before Phase 1 implementation work, but the `concerns` auto-merge policy remains a required human decision before enabling auto-merge changes.

## 7. Disciplined design validation

### Design gate checklist

| Gate | Status | Evidence |
|---|---|---|
| File/component changes identified | Conditional pass | Primary components are named (`auto_merge_impl.rs`, `pr_gate_context.rs`, `terraphim_persistence`, `conf.d/terraphim.toml`, runner/process hygiene), but each issue still needs a file-level implementation plan before coding. |
| Public policy decisions identified | Pass | `PrGateResult` merge policy, `concerns` handling, kill-switch default, remediation caps, and branch protection contexts are explicit. |
| Test strategy present | Pass | Exit criteria include integration tests for stop conditions, seeded-finding live proof, doc-only PR proof, and weekly health reporting. |
| Steps sequenced with dependencies | Pass | Issue dependency edges are explicit: A → B → C; D/E/F independent; G after B; H last. |
| Rollback path present | Pass | Deployment/rollback discipline now requires binary/config backups, atomic install, verification, synthetic webhook proof, and restore procedure. |
| Essentialism applied | Pass | The plan explicitly rejects second verdict formats, retired build-runners, unbounded remediation, scheduler rewrite, and unsafe forced merges. |

### Simplicity check

The simplest viable design is to extend consumption of the already-validated `PrGateResult` contract rather than creating new verdict formats, new services, or new agents. The plan follows that path. The only substantial new mechanism is persisted remediation state, which is justified by the need to avoid duplicate dispatches, repeated findings, and cold-start stampedes.

### Design gate result

The plan is approved for issue filing and Phase 0/Phase 1 implementation preparation. Before Phase 2 remediation-loop coding, create a focused implementation plan with concrete files, data types, persistence schema, and integration tests for the remediation state machine.

## 8. KLS quality evaluation

| Dimension | Score | Justification | Required fix |
|---|---:|---|---|
| Physical | 5/5 | Clear headings, tables, phases, issue breakdown, and approval checklist. | None |
| Empirical | 4/5 | Understandable to an ADF/Terraphim maintainer with concrete evidence and issue references. | Add file-level plans per issue before implementation. |
| Syntactic | 4/5 | Structure is internally consistent; operational hardening item spans phases but is tracked as issue I. | Keep issue I as cross-cutting or split it during issue filing. |
| Semantic | 5/5 | Accurately reflects weekly report, long-session evidence, and existing Terraphim infrastructure. | None |
| Pragmatic | 5/5 | Provides ordered phases, exit criteria, dependencies, policy decisions, and acceptance evidence. | None |
| Social | 3/5 | Draft for approval; human decisions remain for phase ordering, `concerns` policy, kill-switch/caps, and issue filing. | Obtain explicit approval decisions before enabling auto-merge/remediation. |

**Average score**: 4.3/5.

**Minimum score**: 3/5, Social.

**Quality gate decision**: Conditional pass. The plan is strong enough to file issues and start Phase 0/Phase 1 preparation. It must not proceed to enabling auto-merge or remediation until the approval items below are decided.

## Approval

- [ ] Phase ordering accepted
- [ ] `concerns` auto-merge policy decided (Phase 1)
- [ ] Remediation kill-switch / caps accepted (Phase 2)
- [ ] Issue breakdown approved for filing via gtr
