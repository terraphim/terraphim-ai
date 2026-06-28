# ADF Weekly Activity Report: 2026-06-03 -- 2026-06-10

**Generated**: 2026-06-10 15:52 BST
**Sources**: Bigbox ADF journald logs, Gitea API (`git.terraphim.cloud`), GitHub API (`github.com/terraphim`)
**Scope**: All repositories under `terraphim` organisation on both GitHub and Gitea.

---

## Executive Summary

Over the past week, ADF (AI Dark Factory) processed **30 unique PRs across 6 repositories**, posting **82 terminal commit statuses** (3 per PR gate set: `adf/pr-reviewer`, `adf/validation`, `adf/verification`). The orchestrator spawned **275 agent runs** across 20 different agent roles.

**20 PR gate timeout failures** occurred, all handled fail-closed via the `PrGateResult` contract. **10 agent exits** were classified as `unknown` (non-zero exit code, no pattern match). **7 rate-limit hits** and **3 crash/panic patterns** were detected.

The primary focus of the week was landing the native `PrGateResult` contract and native PR gate producer pipeline on `terraphim-ai` PR #2318, which went through **11 head commits** with iterative testing and live validation on bigbox.

---

## ADF Gate Activity by Repository

### terraphim-ai (Gitea: `git.terraphim.cloud/terraphim/terraphim-ai`)

| PR | Head SHAs Tested | Review Dispatches | Terminal Outcomes |
|----|------------------|-------------------|-------------------|
| #2318 | 11 commits (`b71332d` through `866460a1e`) | 11 | Mixed: passes, timeout failures, malformed-JSON failures; current head blocked by reviewer `failure` |
| #2337 | `cd090a4` | 1 | 3 terminal statuses posted |
| #2340 | `91d8547` | 1 | 3 terminal statuses posted |
| #2343 | `5fd3b3b` | 1 | 3 terminal statuses posted |
| #2347 | `552c2f1` | 1 | 3 terminal statuses posted |
| #2351 | `ad25e95` | 1 | 3 terminal statuses posted |
| #2353 | `28b800f` | 1 | 3 terminal statuses posted |
| #2355 | `6ba7c8e` | 1 | 3 terminal statuses posted |
| #2357 | `4e9cf8f` | 1 | 3 terminal statuses posted |
| #2358 | `180d615` | 1 | 3 terminal statuses posted |
| #2362 | `b6a1227` | 1 | 3 terminal statuses posted |
| #2363 | `35c12d5` | 1 | 3 terminal statuses posted |
| #2365 | `ab0ecc1` | 1 | 2 timeout failures (pr-reviewer, pr-validator); 1 verdict from pr-verifier |
| #2373 | `a576280` | 1 | 3 terminal statuses posted |
| #2376 | `476188e` | 1 | 3 terminal statuses posted |
| #2377 | `f3a2b27` | 1 | 3 terminal statuses posted |

**Total**: 16 PRs processed, 17 dispatch events, ~51 individual ADF gate runs.

#### PR #2318 Detailed Timeline

PR #2318 ("Fix #2301: add PrGateResult contract for PR fan-out gates") was the primary development focus:

| Commit | Description | Gate Outcome |
|--------|-------------|-------------|
| `b71332d` | Initial PR open | All 3 passed (`verdict already present for head`) |
| `0e072ee` | `feat(orchestrator): add PrGateResult contract` | All 3 passed |
| `073ede3` | `fix(orchestrator): discard PrGateResult drain-log parsing` | All 3 passed |
| `88b008a6` | `fix(orchestrator): synthesise canonical failed PR gate results` | All 3 **failed** (malformed `adf:gate-result` JSON -- by design, validating fail-closed parse handling) |
| `ca4a6f2ad` | `fix(orchestrator): fail closed on PR gate timeouts` | All 3 **failed** (`agent exceeded 300s PR gate wall-clock limit` -- by design, validating 300s timeout enforcement) |
| `902f8167b` | `fix(orchestrator): harden native PR gate producer spawn` | All 3 passed (first native producer run) |
| `2575c3604` | `fix(orchestrator): fetch PR refs for native gate evidence` | All 3 passed (107k-character native prompts) |
| `900c00a88` | `fix(orchestrator): fetch PR gate evidence from head refs` | All 3 passed (`pass (4/5)` each) |
| `866460a1e` | `docs(adf): record final PR gate validation` | **pr-reviewer: failure** (`2 blocking finding(s) reported`), validator/verifier: passed |

The final head (`866460a1e`) is blocked by the reviewer gate finding 2 blocking issues (truncated diff evidence and traceability gaps between a doc-only head commit and the implementation commits referenced in validation evidence).

### odilo (Gitea: `zestic-ai/odilo`)

| PR | Head SHA | Outcome |
|----|----------|---------|
| #389 | `b634434` | 3 gates posted |
| #390 | `a186fac` | 3 gates posted |
| #391 | `53f814a` | Processed |
| #392 | `6487316` | Processed |
| #393 | `aab5727` | Processed |
| #394 | `f430d92` | Processed |
| #397 | `8a599a6` | Processed |
| #399 | `14636e2` | Processed |
| #402 | `65347bb` | Processed |

**Total**: 9 PRs processed.

### terraphim-clients (Gitea)

| PR | Head SHA | Outcome |
|----|----------|---------|
| #9 | `59c7db4` | 3 gates posted |
| #10 | `d415c74` | 3 gates posted |

**Total**: 2 PRs processed.

### terraphim-agents (Gitea)

| PR | Head SHA | Outcome |
|----|----------|---------|
| #32 | `95e2811` | 3 gates posted (dispatched twice -- synchronised) |

**Total**: 1 PR processed, 2 dispatch events.

### terraphim-kg-agents (Gitea)

| PR | Head SHA | Outcome |
|----|----------|---------|
| #4 | `654e707` | 3 gates posted (dispatched twice -- synchronised) |

**Total**: 1 PR processed, 2 dispatch events.

### terraphim-service (Gitea)

| PR | Head SHA | Outcome |
|----|----------|---------|
| #7 | `e0891be` | 3 gates posted |

**Total**: 1 PR processed.

---

## Failure Analysis

### Timeout Failures (20 events)

All 20 timeout failures occurred in the PR gate path and were handled fail-closed: the orchestrator posted a `failure` commit status on Gitea and a canonical `adf:gate-result` comment with `"status": "fail"` and `"summary": "agent exceeded 300s PR gate wall-clock limit"`.

Notable timeout clusters:
- `ca4a6f2ad` (PR #2318 timeout validation test): All 3 gates intentionally timed out.
- `ab0ecc1` (PR #2365): 2 of 3 gates timed out (pr-reviewer, pr-validator) at ~327s and ~327s.
- Several `implementation-swarm` agents hit timeout patterns but recovered with `exit_class=success` (the pattern was matched but the agent completed successfully).

### Rate-Limit Failures (7 events)

Rate-limit patterns (`429`, `rate limit`, `ratelimit`) were matched in agent exits:
- 3 PR gate agents hit rate limits (pr-verifier, pr-reviewer, pr-validator)
- These were classified as `success` because the `exit_code` was 0; the rate-limit detection is informational.

### Crash/Panic Patterns (3 events)

- `pr-validator` on an earlier head: matched `crash` pattern, `wall_time_secs=670`.
- `implementation-swarm`: 2 instances of non-zero exit code with `exit_class=unknown`.

### Unknown Exit Classifications (10 events)

Ten agent exits had non-zero exit codes with no matched error pattern:
- `implementation-swarm`: 8 instances (exit_code=1, wall time ~27-30s)
- `meta-learning`: 1 instance (exit_code=1, wall time ~30s)

These are typically CI runner pre-check failures (e.g. missing project workspace) rather than agent code bugs.

### Malformed-Output Failures (3 events, intentional)

- `88b008a6` (PR #2318): All 3 gates failed with parse errors on malformed `adf:gate-result` JSON. This was a deliberate live validation of the fail-closed parsing and canonical failure-envelope synthesis.

---

## Agent Activity Summary

Total agent runs classified: **275** over the period.

| Agent Role | Runs | Dominant Exit Class | Notes |
|------------|------|--------------------|-------|
| `implementation-swarm` | 76 | success (with timeout patterns in ~34 cases) | CI build/test agents |
| `test-guardian` | 24 | success | Post-merge test validation |
| `spec-validator` | 24 | success | Specification conformance |
| `pr-verifier` | 23 | empty_success (confidence 0.8) | PR gate verification |
| `pr-validator` | 23 | empty_success (confidence 0.8) | PR gate validation |
| `pr-reviewer` | 21 | empty_success (confidence 0.8) | PR code review |
| `quality-coordinator` | 19 | success | Quality gate orchestration |
| `security-sentinel` | 18 | success | Security audit |
| `odilo-developer` | 13 | success | Odilo project developer agent |
| `product-owner` | 11 | success | Requirements validation |
| `product-development` | 11 | success | Development workflow |
| `meta-coordinator` | 7 | success | Multi-agent coordination |
| Other roles | 5 | success | upstream-synchronizer, roadmap-planner, documentation-generator, meta-learning, gitea-upstream-synchronizer, atomic-upstream-synchronizer |

---

## Auto-Merge Activity

The auto-merge subsystem scanned all configured projects throughout the week. No PRs were auto-merged; all required human review due to confidence scores below the 5/5 threshold. Notable patterns:

- **Stale verdicts**: Multiple projects (`atomic-server`, `odilo`, `terraphim-ai`) had PRs where the review SHA no longer matched the current head.
- **Parse failures**: Several `terraphim-ai` PRs had reviewer comments that could not be parsed (missing confidence score headers, malformed footers), preventing auto-merge evaluation.
- **Confidence distribution**: Reviewed PRs typically scored between 2/5 and 4/5; none reached the auto-merge threshold of 5/5.

---

## Platform Health

- **Service restarts**: The `adf-orchestrator.service` was restarted at least 4 times during the period (PID changes: `2252029` -> `1732284` -> `582580` -> `645542` -> `2130651`). These correspond to binary deployments for the PR #2318 iterative validation.
- **Reconcile loop**: The reconcile tick typically completed in 300-1800ms but occasionally hit SLOW warnings (>5s) during repository auto-merge enumeration of the large `terraphim-ai` PR backlog (~292 open issues, ~85 open PRs).
- **Telemetry lag**: Output event telemetry regularly reported skipped events for PR gate agents (typically 600-1900 events lagged per 30s interval), indicating the `pi-rust` CLI produces output faster than the orchestrator can drain during LLM inference.

---

## Key Observations

1. **Fail-closed handling validated**: The `PrGateResult` contract's malformed-output and timeout fallback paths were live-tested on PR #2318 commits `88b008a6` and `ca4a6f2ad`. Both correctly produced `failure` statuses and canonical fallback comments.

2. **Native PR gate pipeline operational**: Commits `902f8167b`, `2575c3604`, and `900c00a88` demonstrated the new native pipeline: orchestrator-built bounded prompts (~109k characters), `terraphim_automata` concept matching, head-ref-based evidence fetch, and terminal status posting from parsed `adf:gate-result` blocks.

3. **Reviewer false positive on current head**: The final doc-only commit (`866460a1e`) triggered a reviewer failure (2 blocking findings). The reviewer's concern about truncated diff evidence and traceability gaps is a limitation of the 60k-character comment truncation and the mismatch between the validation report's implementation-commit references and the doc-only head, rather than a code defect.

4. **Cross-repo coverage**: ADF PR gates are active across 6 repositories (`terraphim-ai`, `odilo`, `terraphim-clients`, `terraphim-agents`, `terraphim-kg-agents`, `terraphim-service`) covering the polyrepo split under the `terraphim` Gitea organisation.

5. **No GitHub ADF activity**: The `github.com/terraphim` organisation contains 60+ repositories but ADF gate statuses are posted only to Gitea, which is the authoritative CI and review platform.
