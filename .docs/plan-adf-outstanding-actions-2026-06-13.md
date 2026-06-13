# Outstanding Actions: ADF Flow Fix

**Status**: In progress (2026-06-13 session, bigbox actions 2/4/5 deployed)
**Author**: Terraphim AI
**Date**: 2026-06-13 10:31 BST
**Supersedes**: `.docs/plan-adf-flow-fix-2026-06-10.md` (original plan — preserved for reference)
**Method**: Disciplined research + disciplined design applied to the original plan

---

## Disciplined Research Findings

### Architecture Correction (Critical, Verify Before Coding)

The original plan assumed `terraphim_orchestrator` lives inside `terraphim-ai`. Current evidence indicates the active orchestrator development line is in `terraphim-agents`, but this must be verified in the target working tree before implementation.

- `terraphim_orchestrator` is the core of **`terraphim-agents`** (a separate Gitea repo at `git.terraphim.cloud/terraphim/terraphim-agents`), locally at `/Users/alex/projects/terraphim/terraphim-agents-2301`
- Branch `task/2465-auto-merge-blocker-kind` is the active development branch (recent commits 2026-06-11 and 2026-06-12)
- Branch `task/2301-auto-merge-config` on `terraphim-ai` contains 1,059 new files / 475,092 insertions — this is the **import/extraction of the orchestrator INTO terraphim-ai** (Gitea #1910 god-file decomposition), not a small config change
- All Phase 2 remediation loop code (RemediationState, terraphim_persistence, integration tests) is already merged in `terraphim-agents` main

### Orchestrator Consolidation Decision

Bringing `terraphim_orchestrator` back into `terraphim-ai` is a repo-consolidation move, not a prerequisite for fixing ADF flow reliability.

Likely intended benefits:

- Make `terraphim-ai` the single source of truth for ADF orchestration, native CI, deployment artefacts, docs, and branch protection.
- Reduce drift between `terraphim-ai` and `terraphim-agents`.
- Simplify end-to-end testing of ADF against the main Terraphim codebase.
- Centralise Gitea issue/PR automation and release governance.

Decision for this plan:

- **Do not merge the 475k-line orchestrator import as part of the ADF flow fix.**
- Fix the active orchestrator where it currently runs (`terraphim-agents`) and deploy that binary/config path.
- Treat repo consolidation as a separate architecture decision record and implementation plan after ADF flow reliability is restored.
- Any cherry-pick from `task/2301-auto-merge-config` must be a narrow, reviewed change with a clear runtime target; do not use that branch wholesale as the implementation base.

### Verification Required Before Implementation

| Claim | Verification command/evidence | Status |
|---|---|---|
| Active orchestrator code is in `terraphim-agents` | In `/Users/alex/projects/terraphim/terraphim-agents-2301`, fetch remotes and verify `crates/terraphim_orchestrator` exists on the target branch. | **Verified** |
| `task/2465-auto-merge-blocker-kind` contains active auto-merge work | Verify branch exists locally/remotely and inspect recent commits before editing. | **Verified** (commits 3d4eb19, cb9f7e8, 47767b0) |
| Phase 2 remediation loop is already merged | Search the verified branch for `RemediationState`, persistence-backed remediation state, and integration tests. | **Verified** (`pr_poller::RemediationState`) |
| `task/2301-auto-merge-config` is an orchestrator import, not a small config branch | Confirm diff size and target repo before using it as an implementation base. | **Verified** (deferred; 475k-line import on terraphim-ai) |
| Orchestrator consolidation is not needed for this fix | Confirm active deployed ADF binary is built from `terraphim-agents` and can be patched/deployed independently. | **Verified** (PR #43 on terraphim-agents) |

### What Is Already Done (not in original plan's "done" list)

These items are reported as already done, but must be treated as pending verification until the target `terraphim-agents` branch is fetched and inspected.

| Item | Evidence |
|---|---|
| `AutoMergeBlockerKind` classification — structured blocker types replacing opaque reason strings | Commit `3d4eb19` in terraphim-agents (2026-06-11) |
| `adf:gate-result` comments accepted for auto-merge verdict evaluation | Commit `cb9f7e8` in terraphim-agents (2026-06-11) |
| `max_diff_loc` raised 500 → 10,000 LoC; exposed in `[auto_merge]` config | Commit `47767b0` in terraphim-agents (2026-06-12) |
| PrGateMeta rename from PrVerdictMeta | `pr_handlers_impl.rs` line 284 uses `PrGateMeta` |

### What Is Actually Broken (corrected from original plan)

1. **Duplicate issue crisis** — Gitea issue #2596 is open: 249 of 373 "ready" issues are alert-emitter duplicates. Agent coordination (`gtr ready`, `triage`, PageRank) is 67% corrupted by noise. This is the #1 blocker for reliable overnight agents.

2. **`min_confidence` threshold is 5/5** — auto-merge requires `min_confidence = 5` but real PR scores are 2–4/5. No PR will ever auto-merge at the current threshold. The fix is a config change.

3. **Branch protection only requires 2 of 4 gate contexts** — `terraphim-ai` and `terraphim-agents` both require only `native-ci / build (push)` + `adf/pr-reviewer`. The `adf/validation` and `adf/verification` contexts are not yet enforced.

4. **implementation-swarm cadence** — documented target is `*/20 * * * *` (every 20 min); the hot-loop runs every 5 min (eval_interval_secs=300). Bigbox live config needs updating.

5. **`task/2301-auto-merge-config` PR is not merged** — 475k-line orchestrator import into terraphim-ai is pending; deferred this sprint.

6. **`concerns` policy was unresolved** — now decided (see Decisions below).

### What Is NOT a Gap (original plan was wrong)

- `pr_gate_context.rs` as a named file: not needed — evidence building is distributed across `pr_gate.rs`, `pr_review.rs`, `pr_poller.rs`
- "Truncation markers" for per-gate evidence: not applicable — gate evaluation uses live Gitea API calls (instant), not streaming output. Truncation in issue body posting already appends `"... (truncated)"`
- "Per-gate configurable timeout": 300s agent execution timeout (`DEFAULT_PR_GATE_TIMEOUT_SECS`) is the right granularity. No additional configurability needed.
- Phase 2 remediation loop stranded: it IS the `terraphim-agents` main, not stranded.

---

## Decisions (recorded 2026-06-13)

1. **`min_confidence` threshold**: **4/5**. `concerns` with zero blocking findings also merges. PRs at 3/5 or below are held for remediation or human review.
2. **#1910 orchestrator extraction**: **Deferred**. Cherry-pick only auto-merge unification changes; keep `terraphim-agents` as orchestrator home for this sprint. `task/2301-auto-merge-config` is not merged this sprint.
3. **Orchestrator repo consolidation**: **Separate ADR required**. Consolidation may be valuable, but it is out of scope for the ADF flow reliability fix.
4. **Bigbox SSH**: Available in current session — Actions 2 and 4 can be executed directly.

---

## Outstanding Actions

### Action 1 — Fix duplicate issue creation (#2596) [P0, ~2 hours, code change in terraphim-agents]

**Problem**: Alert emitters in `auto_merge_impl.rs` call `create_issue` on every failed reconcile tick for the same PR, producing 20+ duplicates daily.

**Design**: Before creating a failure issue, search open issues for a stable dedup key (`remediation_key()` if available, otherwise `ADF-Failure-Key: <owner>/<repo>#<pr>:<head_sha>:<failure_kind>` in the issue body). If found, post a `gtr comment` update instead of creating a new issue.

**Files**: `crates/terraphim_orchestrator/src/auto_merge_impl.rs` in `terraphim-agents` (approx lines 1330–1400 — issue creation path)

**Existing infrastructure to reuse**: `remediation_key()` in `pr_gate.rs`; `list_issues` Gitea API; `gtr comment` via gitea-robot

**Verification**: After deploy, run `gtr triage --owner terraphim --repo terraphim-ai` and confirm ready issues are no longer dominated by duplicates. No new duplicate issues created over 2 reconcile cycles.

### Action 2 — Set `min_confidence = 4` in live orchestrator config [P0, 30 minutes, bigbox config]

**Problem**: `min_confidence = 5` means `AutoMergeCriteria` never approves a real PR (observed range 2–4/5). Zero auto-merges all week.

**Design**: First verify that the active auto-merge code path still consults `min_confidence`. If it does, `ssh bigbox` and set `min_confidence = 4` in `/opt/ai-dark-factory/orchestrator.toml` under `[auto_merge]`. Also confirm `concerns` with `blocking_findings = 0` is treated as mergeable per `pr_gate_result.rs` status policy.

**Verification**: Use journald or Quickwit to verify the active `AutoMergeCriteria` at startup and watch for an `AutoMerge` decision after restarting orchestrator. If the `PrGateResult` path no longer consults confidence, record this action as not applicable and do not make a cosmetic config-only change.

### Action 3 — Add `adf/validation` + `adf/verification` to branch protection [P1, 1 hour, Gitea API]

**Problem**: Only 2 of 4 required gate contexts are enforced. `adf/validation` and `adf/verification` are posted but not required for merge.

**Design**: First verify each repo reliably emits all four contexts on current PR heads. Only then use the Gitea API to update branch protection on repos with complete `extra_projects` coverage. Reuse pattern from `scripts/adf-setup/polyrepo-ci/cutover-to-native.sh`.

**Repos to update**: `terraphim-ai`, `terraphim-agents`, `terraphim-service`, `terraphim-clients`, `terraphim-kg-agents`, `terraphim-config-persistence`, `terraphim-core`.

```bash
for REPO in terraphim-ai terraphim-agents terraphim-service terraphim-clients terraphim-kg-agents terraphim-config-persistence terraphim-core; do
  curl -fsS -X PATCH \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" \
    "${GITEA_URL}/api/v1/repos/terraphim/${REPO}/branch_protections/main" \
    -d '{"status_check_contexts":["native-ci / build (push)","adf/pr-reviewer","adf/validation","adf/verification"]}'
done
```

**Verification**: `curl -s -H "Authorization: token $GITEA_TOKEN" "$GITEA_URL/api/v1/repos/terraphim/terraphim-ai/branch_protections/main"` → returns all 4 contexts.

**Safety rule**: Do not require `adf/validation` or `adf/verification` on a repo until a recent commit on that repo has posted those statuses successfully. Missing-context branch protection blocks all merges.

### Action 4 — Fix implementation-swarm spawn cadence [P1, 30 minutes, bigbox config]

**Problem**: Spawning every 5 min (eval_interval_secs=300) instead of every 20 min. 8 of 10 weekly "unknown" exits are pre-check no-work runs burning LLM credits.

**Design (two-part)**:
1. `ssh bigbox` and set `schedule = "*/20 * * * *"` in live conf.d for implementation-swarm (per `scripts/adf-setup/docs/cron-cadence.md`).
2. Add a `gtr ready` pre-check before the LLM execution path: if no ready issues exist, exit without spawning an LLM.

**Config path on bigbox**:
```bash
# Locate the implementation-swarm block with rg or editor search.
rg -n 'implementation.swarm|implementation-swarm' /opt/ai-dark-factory/conf.d/terraphim.toml
# Edit schedule field, then:
adf --check /opt/ai-dark-factory/conf.d/terraphim.toml && sudo systemctl restart adf-orchestrator
```

**Verification**: Count `implementation-swarm` dispatches over 1 hour: should be ≤3 (one per 20-min slot), not 12+.

### Action 5 — Batch-close duplicate failure issues [P1, 1 hour, Gitea API]

**Problem**: 50+ open `[ADF]` issues, most duplicates for the same PR failure. Blocks `gtr ready` / `gtr triage` accuracy (249/373 ready issues are noise).

**Design**: Run only after Action 1 is deployed, unless the duplicate emitter is temporarily disabled. Use a `gtr`-driven script:
1. List all open issues matching `[ADF]` prefix.
2. Group by PR number from the `remediation_key` pattern in the title.
3. For each group with >1 issue, keep the newest, close the rest with comment "Superseded by #NEWEST (dedup batch 2026-06-13)".

**Infrastructure**: `gtr list-issues`, `gtr close-issue`, `gtr comment` — all available now.

**Verification**: `gtr triage --owner terraphim --repo terraphim-ai` shows ≤10 ADF-emitted issues, all distinct and actionable.

### Action 6 — End-to-end live proof [P2, half a day]

**Design**: Open a low-complexity PR on `terraphim-agents` or `terraphim-ai`. Confirm:
1. `native-ci / build (push)` posts green status
2. `adf/pr-reviewer`, `adf/validation`, `adf/verification` all post `success` for the head SHA
3. All three have matching `adf:gate-result` comments with `head_sha` matching the current head
4. Auto-merge fires when confidence ≥ 4/5 and zero blocking findings
5. If findings present, one remediation dispatch occurs; re-review follows

**Capture to**: `.docs/validation-report-adf-flow-fix-2026-06-13.md`

---

## Sequencing

| Order | Action | Effort | Depends on |
|---|---|---|---|
| 0 | Verify architecture claims and active repo/branch | 30 min | — |
| 1 | Action 1: fix dedup in auto_merge_impl.rs (code) | 2 h | Verified repo/branch |
| 2 | Action 5: batch-close duplicate issues | 1 h | Action 1 deployed, or emitter temporarily disabled |
| 3 | Action 2: set min_confidence = 4 if active path uses it | 30 min | SSH access + code-path verification |
| 4 | Action 4: implementation-swarm cadence (bigbox config) | 30 min | SSH access |
| 5 | Action 3: branch protection alignment | 1 h | Per-repo status emission proof |
| 6 | Action 6: live proof | 4 h | Actions 1–4 done; Action 3 done where safe |

Total estimated effort: **~9 hours** across code + config + Gitea API.

---

## Verification Commands

```bash
# After Action 1+2 (dedup code + threshold):
ssh bigbox "sudo journalctl -u adf-orchestrator.service --since '1 hour ago' --no-pager" \
  | rg 'AutoMerge|auto.merge|create_issue'

# After Action 3 (branch protection):
curl -s -H "Authorization: token $GITEA_TOKEN" \
  "${GITEA_URL}/api/v1/repos/terraphim/terraphim-ai/branch_protections/main" \
  | jq -r '.status_check_contexts[]'

# After Action 4 (implementation-swarm):
ssh bigbox "sudo journalctl -u adf-orchestrator.service --since '60 min ago' --no-pager" \
  | rg 'implementation-swarm.*dispatch' --count
# Expect ≤3

# After Action 5 (batch-close):
gtr triage --owner terraphim --repo terraphim-ai
# Expect: no [ADF] cluster dominating the top

# After Action 6 (live proof):
curl -s -H "Authorization: token $GITEA_TOKEN" \
  "${GITEA_URL}/api/v1/repos/terraphim/terraphim-ai/commits/{HEAD_SHA}/statuses" \
  | python3 -c "import sys,json; [print(s['context'], s['state']) for s in json.load(sys.stdin)]"
```

---

## Verification table (2026-06-13 bigbox session)

| Action | Evidence | Status |
|---|---|---|
| 0 Architecture | `terraphim-agents` + `task/2465-auto-merge-blocker-kind` verified | Done |
| 1 Issue dedup | PR #43; binary on bigbox built from `task/2596-adf-issue-dedup` | Deployed (merge PR pending) |
| 2 min_confidence=4 | `adf --check` → `min_confidence 4`; journald `threshold 4/5` after binary rebuild | Done |
| 3 Branch protection | terraphim-ai: 4 status contexts | Done |
| 4 Swarm cadence | `schedule */20`, `pre_check` after `project`; orchestrator active | Config done; 60m dispatch count pending |
| 5 Batch-close dupes | 65 closed; 1 skipped (#1971 dep on #2596) | Done (partial) |
| 6 Live proof | PR #2402 auto-merged; `.docs/validation-report-adf-flow-fix-2026-06-13.md` | Done |
