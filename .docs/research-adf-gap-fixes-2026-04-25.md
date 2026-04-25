# Research Document: ADF Gap Fixes -- runtime-guardian rename + meta-learning agent

**Status**: Approved
**Author**: Claude Code (disciplined-research phase)
**Date**: 2026-04-25

---

## Executive Summary

Two structural gaps from the original ADF vision can be addressed in a single config-only implementation. Gap A requires renaming the current infra-health agent to `runtime-guardian` and creating a real fork-sync `upstream-synchronizer`. Gap B requires a new `meta-learning` agent (Mneme persona) that mines the systemd journal for cross-agent patterns and posts a daily fleet-health synthesis. Neither requires Rust code changes.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Fleet blind spots are operational risk; meta-learning was the centrepiece of the original vision |
| Leverages strengths? | Yes | Config-only changes using existing orchestrator infrastructure |
| Meets real need? | Yes | Today's session required human diagnosis of patterns Mneme would catch automatically |

**Proceed**: Yes

---

## Gap A: upstream-synchronizer name/function mismatch

### Current State

The agent named `upstream-synchronizer` in conf.d performs infrastructure health checks:
- Disk usage (alert >80%)
- Docker image accumulation
- Memory (RAM-aware after today's fix)
- GitHub Actions runner status
- Rust `target/` directory sizes
- `git fetch origin` + `cargo outdated`

This is the **`runtime-guardian`** function from the original BigBox architecture (`bigbox-deployment/docs/ARCHITECTURE.md`):
> "Runtime Guardian -- Monitors system performance, resource usage, and optimizes runtime. Memory: 8GB, Priority: MEDIUM"

The original `upstream-synchronizer` function (`cto-executive-system/agents/upstream-synchronizer/agent.py`):
> "Keeps forks in sync with upstream repositories, handles cherry-picks, merge conflict detection."
> Monitored: `https://github.com/go-gitea/gitea.git`

### Gitea Fork State on Bigbox

Location: `/home/alex/projects/terraphim/gitea`

Current remotes:
```
fork      https://github.com/AlexMikhalev/gitea.git
github    https://github.com/kimiko-terraphim/gitea.git
origin    git@github.com:kimiko-terraphim/gitea.git
terraphim https://git.terraphim.cloud/terraphim/gitea.git
```

**Critical finding**: No `upstream` remote pointing to `https://github.com/go-gitea/gitea.git`. The gitea fork has 15 commits ahead of origin/main with custom Robot API code (PageRank, dependency tracking, `gtr` commands). These 15 commits represent significant divergence from upstream go-gitea that has never been tracked for upstream security patches.

The fork is a genuinely custom fork -- not a thin fork -- so tracking go-gitea upstream is about cherry-picking security fixes, not merging feature releases wholesale.

### No Impact on Existing Issues

Zero open Gitea issues mention `@adf:upstream-synchronizer`. Renaming is clean -- no Gitea issues need updating.

### Rename Implications

| Component | Effect of rename |
|-----------|-----------------|
| conf.d agent name | Change `name = "upstream-synchronizer"` → `name = "runtime-guardian"` |
| Worktree paths | Were `/tmp/adf-worktrees/upstream-synchronizer-*`, become `runtime-guardian-*` |
| Journal entries | New PID restart changes all names in logs going forward |
| Gitea mentions | `@adf:upstream-synchronizer` stops working; no open issues use it |
| Template file | Create `scripts/adf-setup/agents/runtime-guardian.toml` (currently no template exists) |

### New upstream-synchronizer Scope

The real upstream-synchronizer should:
1. Add `upstream` remote if not present: `git remote add upstream https://github.com/go-gitea/gitea.git`
2. Fetch upstream: `git fetch upstream`
3. Identify commits on `upstream/main` not in the terraphim fork: `git log HEAD..upstream/main --oneline`
4. Scan for security-relevant commits: `git log upstream/main..HEAD` -- looking for CVE mentions
5. Report divergence count and create a `[Infra] gitea-fork upstream drift` issue if >50 commits behind

Schedule: `30 1 * * *` (1:30am daily -- before overnight agents start, low-frequency because upstream moves slowly).

---

## Gap B: Missing meta-cortex / Mneme observation layer

### What Data Is Actually Available

**1. systemd journal** (richest source -- 160 structured log lines per 24h):
```
agent exit classified agent=upstream-synchronizer exit_code=Some(0)
  exit_class=success confidence=1.0 matched_patterns=[] wall_time_secs=299.5
```
Fields: agent, exit_code, exit_class, confidence, matched_patterns, wall_time_secs.
Access: `sudo journalctl -u adf-orchestrator --since "24 hours ago" --no-pager`

**2. Reports directory** (87 files, growing daily):
```
/opt/ai-dark-factory/reports/
  infra-health-YYYYMMDD-HHMM.md   -- upstream-synchronizer output
  roadmap-YYYYMMDD.md             -- roadmap-planner output
  coordination-YYYYMMDD.md        -- meta-coordinator output
  dev-review-YYYYMMDD.md          -- product-development output
```

**3. Gitea wiki** (30 pages, mostly drift-detector):
- 28 drift-detector session pages
- 2 Learning pages (from implementation-swarm and product-development)
- Other agents write wiki pages but infrequently

**4. Gitea issues** (open, with Theme-IDs):
- Active findings: `gtr list-issues --state open`
- Theme-IDs present: swap-memory-exhaustion, test-guardian-permanently-missing, ci-gate-non-functional

### What Data Is NOT Available

**AgentRunRecord persistence**: `ExitClassifier` creates an `AgentRunRecord` struct in memory and logs it to the journal. There is **no on-disk structured database** of run records. The quickwit feature (which would send records to Quickwit search engine) is an optional feature (`quickwit = ["dep:reqwest"]`) and is NOT in the default feature set. No quickwit sink is configured.

**Consequence**: Mneme cannot query "show me all runs where exit_class=timeout" from a database. It must parse the journal.

### Last 24h Fleet Statistics (from journal)

Total runs: **160** across the overnight window.

| Agent | Runs |
|-------|------|
| meta-coordinator | 12 |
| compliance-watchdog | 11 |
| upstream-synchronizer | 11 |
| log-analyst | 10 |
| documentation-generator | 10 |
| implementation-swarm | 9 |
| spec-validator | 9 |
| odilo-developer | 9 |
| merge-coordinator | 7 |
| product-development | 6 |
| test-guardian | 6 |
| quality-coordinator | 3 |
| repo-steward | 3 |
| compliance-watchdog | 3 |

| Exit class | Count | Note |
|------------|-------|------|
| success | 108 | 68% |
| empty_success | 24 | 15% |
| timeout | 13 | 8% |
| resource_exhaustion | 10 | 6% -- most false positives (old binary) |
| crash | 4 | 3% -- also likely false positives |
| rate_limit | 1 | 1% |

The 28 non-success exits (excluding empty_success) include pre-fix runs from the old binary. With the new binary deployed today, the false-positive rate should drop significantly in tonight's run. Mneme would notice this improvement pattern.

### What Mneme Should Do

Minimum viable form (config-only, no new code):

1. Parse journal for last 24h exit stats (shell awk/grep)
2. Read latest infra-health report from `/opt/ai-dark-factory/reports/`
3. Count open Gitea issues by Theme-ID to detect unresolved recurring themes
4. Compare exit_class distribution vs previous day (read previous Mneme report from reports/)
5. Identify patterns: agents with >3 non-success runs in 24h, confidence=0.5 patterns appearing multiple times, wall_time consistently hitting the max
6. Post a daily wiki page: `Fleet-Health-YYYYMMDD-Mneme`
7. If patterns warrant action, create a single `[ADF] Fleet health alert YYYY-MM-DD` issue

### Scheduling Analysis

Available cron slots not yet used:
- `:20` past each hour (between upstream-sync :15 and product-dev :25)
- `0 11 * * *` -- 11am daily, **after** the 0-10am cron window completes (ideal for Mneme)

Mneme should run `0 11 * * *` -- exactly when the overnight run finishes and all agents have completed their last cycle. This gives it the complete picture of the overnight run.

---

## Constraints

### Technical
1. **Config-only**: No Rust code changes. Both gaps solvable with TOML conf.d changes and shell task scripts.
2. **Journal access requires sudo**: The meta-learning agent must run as a user with `sudo journalctl` access, or the task must use `GITEA_TOKEN` to query a Gitea report. Alternative: parse reports/ directory instead of journal (reports are owned by `alex`).
3. **No upstream remote on gitea fork**: The new upstream-synchronizer task must add it on first run, or we add it manually as a prerequisite.
4. **Rename atomicity**: The orchestrator must be restarted once after the conf.d rename to pick up the new `name`. During the restart window (seconds), no agents run.

### Vital Few

| Constraint | Why vital | Evidence |
|------------|-----------|----------|
| Journal is the only structured run record source | No quickwit, no DB | lib.rs analysis -- records only logged, not stored |
| No open issues use @adf:upstream-synchronizer | Rename is clean | gtr search returned 0 results |
| New binary must be running before tonight's cycle | Old binary produces false positives that Mneme would count as real | Today's deployment |

### Eliminated from Scope

| Eliminated | Why |
|------------|-----|
| Adding Quickwit persistence | Requires feature flag, separate service, config changes -- over-engineering for this need |
| Training ML model on run patterns | Original envisioned ML; current need is pattern detection, not ML |
| Real-time alerting (nightwatch integration) | nightwatch is disabled (active_start_hour=2, active_end_hour=6); once-daily is sufficient |
| Tracking ALL upstream go-gitea commits | Security patches only; full merge would break Robot API custom code |

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `sudo journalctl` not available in ADF cgroup | Medium | Mneme uses reports/ instead of journal | Task uses `/opt/ai-dark-factory/reports/` as primary, journal as secondary |
| go-gitea upstream has diverged significantly (hundreds of commits) | High | First run creates noise | Suppress issue creation if >500 commits behind; just report count |
| Rename breaks agent identity in existing Gitea wiki pages | Low | Historical pages still use old name | Accept; no functional impact |
| New upstream-synchronizer runs git fetch on large go-gitea repo | Medium | First fetch is slow (~500MB) | `git clone --depth=1` then subsequent `git fetch --depth=50` |

---

## Open Questions

1. **Can the meta-learning agent run `sudo journalctl` without password?** -- Check with `ssh bigbox 'sudo -n journalctl --version 2>&1'`. If not, fall back to parsing reports/ directory.
2. **Should runtime-guardian keep schedule `15 0-10 * * *` or shift?** -- No reason to shift; :15 is clean and established.
3. **Should upstream-synchronizer (fork-sync) be `1 */12 * * *` (twice daily) instead of daily?** -- Daily is sufficient; upstream go-gitea releases are infrequent.

---

## Verified Facts

| Fact | Source |
|------|--------|
| Gitea fork at `/home/alex/projects/terraphim/gitea`, no upstream remote | `git remote -v` on bigbox |
| 15 commits ahead of origin/main with custom Robot API code | `git log` on bigbox |
| AgentRunRecords: journal only, no structured DB | `lib.rs:4846-4854`, Cargo.toml features |
| 160 agent runs in last 24h via journalctl | `grep "exit classified" \| wc -l` |
| 87 report files in `/opt/ai-dark-factory/reports/`, latest `infra-health-20260425-1221.md` | `ls -la` on bigbox |
| 30 wiki pages, mostly drift-detector | `gtr wiki-list` |
| No open Gitea issues reference `@adf:upstream-synchronizer` | `gtr list-issues` search |
| `0 11 * * *` schedule slot is unused | Conf.d schedule audit |

---

## Recommendations

**Proceed**: Yes.

**Scope**:
1. Rename current infra-health agent: `upstream-synchronizer` → `runtime-guardian` in conf.d + new template
2. Create new `upstream-synchronizer` (fork-sync): daily at `30 1 * * *`, adds upstream remote, reports divergence count
3. Create `meta-learning` (Mneme): daily at `0 11 * * *`, parses reports/ + journal, posts Fleet-Health wiki page daily

**Answer to open question 1** (sudo journalctl): Verify in design phase before committing to journal parsing. If sudo not available, design around reports/ directory as primary source.

**Implementation order**: Rename first (conf.d + restart) → add fork-sync → add meta-learning. Three separate conf.d edits + one restart.
