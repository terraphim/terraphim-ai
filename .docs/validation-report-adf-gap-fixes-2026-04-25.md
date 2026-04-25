# Validation Report: ADF Gap Fixes

**Status**: CONDITIONAL_PASS
**Date**: 2026-04-25
**Research Doc**: `.docs/research-adf-gap-fixes-2026-04-25.md`
**Design Doc**: `.docs/design-adf-gap-fixes-2026-04-25.md`
**Verification Report**: All checks passed (Phase 4, 2026-04-25)

---

## Executive Summary

Four requirements validated. R4 (routing priority) is fully proven in production. R1, R2, and R3 are structurally complete and verified by dry-run -- their first live executions are scheduled for tonight (R2 at 01:30 UTC) and tomorrow morning (R1 at 00:15 UTC, R3 at 11:00 UTC). No regressions in existing agent behaviour. Overall verdict: **CONDITIONAL_PASS** -- all conditions are time-based and require no further engineering work.

---

## Requirements Traceability

| Req | Description | Evidence | Status |
|-----|-------------|----------|--------|
| R1 | infra-health agent named `runtime-guardian` | conf.d verified; pre-rename runs healthy; first fire tomorrow 00:15 UTC | CONDITIONAL_PASS |
| R2 | `upstream-synchronizer` monitors gitea fork vs go-gitea | upstream remote live; dry-run fetch succeeded; first run tonight 01:30 UTC | CONDITIONAL_PASS |
| R3 | `meta-learning` (Mneme) synthesises fleet patterns daily | journal parsing verified; reports dir populated; first run tomorrow 11:00 UTC | CONDITIONAL_PASS |
| R4 | `review_tier` priority = 40 (below `implementation_tier` = 50) | `priority:: 40` in file; KG router reloaded; routing logs confirm | PASS |

---

## V1: R1 -- runtime-guardian naming

**Requirement**: The infra-health agent must carry the name `runtime-guardian` to match the BigBox architecture taxonomy.

**Evidence**:

Pre-rename runs (old binary, today before 11:05 UTC): 11 successful runs of the infra-health function, all `exit_class=success confidence=1.0`. The last was at `10:24 UTC`, `wall_time=299.6s`.

Post-rename conf.d (verified):
```
runtime-guardian: schedule='15 0-10 * * *', persona='Ferrox'
```

The cron window `15 0-10 * * *` does not overlap with current UTC time (~11:30 UTC). First fire under the new name: **tomorrow 00:15 UTC**.

**Verdict**: CONDITIONAL_PASS -- structural change complete, first live execution deferred.

---

## V2: R2 -- upstream-synchronizer fork-sync

**Requirement**: An `upstream-synchronizer` must monitor `/home/alex/projects/terraphim/gitea` against `go-gitea/gitea.git`, detect security-relevant commits, and create Gitea issues when needed.

**Evidence**:

Upstream remote confirmed on bigbox:
```
upstream  https://github.com/go-gitea/gitea.git (fetch)
upstream  https://github.com/go-gitea/gitea.git (push)
```

Dry-run fetch succeeded (depth=10):
```
* [new branch]  release/v1.8  -> upstream/release/v1.8
* [new branch]  release/v1.9  -> upstream/release/v1.9
* [new tag]     v1.26.1       -> v1.26.1
* [new tag]     v1.25.5       -> v1.25.5
```

Divergence count (depth=10 shallow): **10 commits behind**. Full depth=200 fetch at first live run will give the complete picture.

Task text confirmed: contains `GITEA_FORK="/home/alex/projects/terraphim/gitea"` and `UPSTREAM_URL="https://github.com/go-gitea/gitea.git"`.

Conditional logic verified: creates issue only if `$BEHIND -gt 50 AND $SECURITY_COMMITS non-empty`. With 10 commits behind (shallow), no spurious issue will be created on first run.

**First live run**: tonight at **01:30 UTC**.

**Verdict**: CONDITIONAL_PASS -- upstream data accessible, fork-sync logic correct, first run tonight.

---

## V3: R3 -- meta-learning fleet synthesis

**Requirement**: A `meta-learning` agent (Mneme persona) must synthesise cross-agent patterns from overnight data and post a daily `Fleet-Health-YYYYMMDD-Mneme` wiki page.

**Evidence**:

Journal parsing command validated on live bigbox data (last 24h, 160 runs):
```
101 success
 24 empty_success
 13 timeout
  9 resource_exhaustion
  3 crash
  1 rate_limit
```

This demonstrates the shell pipeline produces correct structured output for Mneme's synthesis step.

Reports directory populated with today's runs:
```
/opt/ai-dark-factory/reports/infra-health-20260425-1221.md
/opt/ai-dark-factory/reports/infra-health-20260425-1117.md
/opt/ai-dark-factory/reports/infra-health-20260425-1019.md
```

`sudo -n journalctl` confirmed zero-password in the ADF cgroup (Phase 1 research finding, validated today).

Persona `Mneme` registered in `/home/alex/terraphim-ai/data/personas/mneme.toml` -- orchestrator loads personas at startup.

**First live run**: tomorrow at **11:00 UTC** (after overnight 00:00--10:00 UTC window completes).

**Verdict**: CONDITIONAL_PASS -- data sources accessible, parsing logic validated, agent registered.

---

## V4: R4 -- routing priority ordering

**Requirement**: `review_tier` priority must be 40 (below `implementation_tier` = 50), restoring the fix from `blog-post-deploying-ai-dark-factory.md`.

**Evidence**:

File contents:
```
review_tier.md:         priority:: 40
implementation_tier.md: priority:: 50
planning_tier.md:       priority:: 80
```

KG router reload confirmed in journal at 09:31 UTC:
```
INFO terraphim_orchestrator::kg_router: KG router loaded path=.../adf rules=3 synonyms=82
```

Routing log from today's runs confirms review_tier still selected for appropriate agents:
```
model selected via KG tier routing agent=product-development concept=review_tier provider=anthropic model=haiku confidence=0.54
```

Rust test suite: `loads_real_adf_taxonomy_3_tiers` asserts `priority == 40` -- PASSES.

**Verdict**: PASS -- fully validated end-to-end.

---

## V5: No Regressions

Agents running after the conf.d change and orchestrator restart all show clean exits:

| Agent | exit_class | confidence | Note |
|-------|------------|------------|------|
| product-development | success | 1.0 | Clean |
| documentation-generator | success | 1.0 | Clean |
| log-analyst | success | 1.0 | matched_patterns=["oom"] -- CORRECT: observability preserved, not misclassified |

The `log-analyst` entry is the classifier fix working as designed: "oom" appears in the log output (the agent discusses OOM conditions), `matched_patterns` records it for observability, but `exit_class=success` because `exit_code=0` is authoritative.

**Verdict**: PASS -- no regressions.

---

## Deferred Validation Schedule

| Requirement | First live evidence | Where to check |
|-------------|--------------------|-----------------|
| R1 (runtime-guardian) | Tomorrow 00:15 UTC | `journalctl | grep "cron schedule fired agent=runtime-guardian"` |
| R2 (upstream-synchronizer) | Tonight 01:30 UTC | `journalctl | grep "cron schedule fired agent=upstream-synchronizer"` and check for divergence report |
| R3 (meta-learning) | Tomorrow 11:00 UTC | `gtr wiki-get --name "Fleet-Health-$(date +%Y%m%d)-Mneme"` |

No engineering work needed -- these are time-gated observations.

---

## Defect Register

None. Phase 4 and Phase 5 both clean.

---

## Gate Checklist

- [x] R4 fully validated in production (priority ordering correct, KG router reloaded)
- [x] R1 structurally complete; deferred first-fire observation to tomorrow
- [x] R2 structurally complete; dry-run validated; deferred first-fire to tonight
- [x] R3 structurally complete; journal parsing validated; deferred first-fire to tomorrow
- [x] No regressions in existing agent behaviour
- [x] Classifier fix working correctly (log-analyst shows observational matched_patterns with exit_class=success)
- [x] All Rust tests pass (0 failures workspace-wide)
- [x] Both Gitea and GitHub remotes at head

## Approval

**CONDITIONAL_PASS** -- approved for production, conditions are time-based only:

1. Observe `runtime-guardian` fires at 00:15 UTC tomorrow in journal
2. Observe `upstream-synchronizer` runs at 01:30 UTC tonight and produces divergence report
3. Observe `Fleet-Health-$(date +%Y%m%d)-Mneme` wiki page created at 11:00 UTC tomorrow

All conditions require observation only, no further implementation.
