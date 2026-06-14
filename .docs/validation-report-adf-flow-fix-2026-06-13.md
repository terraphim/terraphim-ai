# ADF Flow Fix — Live Validation Report

**Date**: 2026-06-13
**Issue**: terraphim/terraphim-ai#2596
**Environment**: bigbox (`/opt/ai-dark-factory`, `adf` rebuilt from `task/2596-adf-issue-dedup`)

---

## Executive summary

| Action | Result |
|---|---|
| Orchestrator stability | **PASS** — crash loop fixed; service active |
| `min_confidence = 4` | **PASS** — config + binary; journald shows `threshold 4/5` |
| Branch protection (4 contexts) | **PASS** — verified on terraphim-ai main |
| Issue dedup (code) | **DEPLOYED** — binary on bigbox; PR #43 merge pending CI |
| Duplicate issue cleanup | **PASS** — 65 closed; ~14 distinct PR alerts remain |
| Auto-merge live proof | **PASS** — terraphim-ai PR #2402 merged autonomously |
| implementation-swarm cadence | **PARTIAL** — 10 spawns/hour fleet-wide (polyrepo); pre-check skips: 0 |

---

## Action 6 — End-to-end auto-merge proof

### Primary evidence: terraphim-ai PR #2402

| Check | Evidence |
|---|---|
| PR | [#2402](https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/2402) |
| Head SHA (pre-merge) | `ae90a628e223710393f7ba3f2e2be7423d3732fa` |
| Merge time | `2026-06-13T12:40:39+02:00` |
| Merge SHA (log) | `782cd08061fbf6b13c35d32d2d2698ab0706ed6a` |

**Journald (bigbox)**:
```
2026-06-13T10:40:26.977367Z INFO terraphim_orchestrator::auto_merge_impl: enqueuing AutoMerge for PR that cleared every gate project=terraphim-ai pr=2402 head=ae90a628...
2026-06-13T10:40:39.646806Z INFO terraphim_orchestrator::auto_merge_impl: pr_auto_merged pr_number=2402 project=terraphim-ai merge_sha=782cd08061fbf6b13c35d32d2d2698ab0706ed6a
```

**Required commit statuses on head** (`ae90a628...`):

| Context | State |
|---|---|
| `native-ci / build (push)` | success |
| `adf/pr-reviewer` | success |
| `adf/validation` | success (4/5 confidence in description) |
| `adf/verification` | success |

**Threshold policy**: New binary logs `confidence 3/5 below auto-merge threshold 4/5` for sub-threshold PRs; PR #2402 cleared all gates including confidence ≥ 4.

### Secondary merges (same reconcile tick, polyrepo)

| Project | PR | merge_sha (log) |
|---|---|---|
| terraphim-core | #4 | `2b2d49d0...` |
| terraphim-service | #4 | `0d892478...` |
| terraphim-clients | #13 | `4442bec1...` |

Demonstrates fleet-wide auto-merge path operational after `min_confidence` and binary upgrade.

---

## Action 2 — min_confidence verification

```bash
# bigbox
adf --check orchestrator.toml | rg -A4 'AUTO-MERGE'
# → min_confidence 4

journalctl -u adf-orchestrator.service --since '2026-06-13 12:40:00' \
  | rg 'threshold 4/5'
```

**Finding**: Config alone was insufficient; stale `adf` binary (pre-#2285) ignored `[auto_merge]`. Rebuild from `task/2596-adf-issue-dedup` required.

---

## Action 4 — implementation-swarm cadence

**Window**: 2026-06-13 14:00–15:00 UTC (1 hour post-rebuild)

| Metric | Observed | Target |
|---|---|---|
| `spawning agent agent=implementation-swarm` | **10** | ≤3 per terraphim-ai |
| `skipping spawn: pre-check found nothing actionable` | **0** | >0 when backlog empty |

**Spawn minute distribution** (fleet-wide):
```
14:00×1  14:18×1  14:20×1  14:35×1  14:40×3  14:45×1  14:50×1  14:55×1
```

**Analysis**:
- `*/20` schedule is active (spawns cluster at :00, :20, :40).
- **Polyrepo amplification**: `implementation-swarm` is declared per `extra_projects` repo; each project fires on the same cron → ~3× burst at :40.
- Pre-check shell script returns ready issues (backlog never empty) → no skip logs.

**Follow-up** (out of scope for this session):
- Scope pre-check / schedule to `project = "terraphim-ai"` only, or stagger per-repo cron.
- Close stale ready issues to let pre-check exit empty.

---

## Action 5 — duplicate cleanup

- Closed **65** `[ADF] Auto-merge failed` duplicates (kept newest per PR).
- **#1971** remains open (dependency on #2596).

---

## PR #43 — dedup merge status

| Field | Value |
|---|---|
| PR | [terraphim-agents#43](https://git.terraphim.cloud/terraphim/terraphim-agents/pulls/43) |
| CI | `native-ci / build (push)` **failed** on `4d28fec` |
| Root cause | `evaluate_verdict_diff_over_cap_below_threshold_is_human_review` used `diff_loc=999` but default `max_diff_loc` is now 10_000 |
| Fix | Commit `89e7aab` — test uses 15_000 LoC with explicit cap 10_000 |

Re-push triggers CI; merge when green.

---

## Commands for re-validation

```bash
# Auto-merge activity
ssh bigbox "sudo journalctl -u adf-orchestrator.service --since '2 hours ago' --no-pager" \
  | rg 'enqueuing AutoMerge|pr_auto_merged'

# Confidence threshold
ssh bigbox "sudo journalctl -u adf-orchestrator.service --since '1 hour ago' --no-pager" \
  | rg 'threshold 4/5'

# Swarm cadence (1 hour)
ssh bigbox "sudo journalctl -u adf-orchestrator.service --since '1 hour ago' --no-pager" \
  | rg 'spawning agent agent=implementation-swarm' | wc -l
```

---

## Sign-off

**ADF flow reliability restored** for auto-merge and orchestrator uptime. Remaining gaps: PR #43 merge, swarm cadence polyrepo amplification, pre-check effectiveness when backlog is large.