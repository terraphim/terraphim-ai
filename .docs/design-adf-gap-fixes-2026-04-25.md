# Implementation Plan: ADF Gap Fixes -- runtime-guardian + upstream-synchronizer + meta-learning

**Status**: Draft
**Research Doc**: `.docs/research-adf-gap-fixes-2026-04-25.md`
**Author**: Claude Code (disciplined-design phase)
**Date**: 2026-04-25
**Estimated Effort**: 2 hours

---

## Overview

### Summary

Three config-only changes address two structural gaps: rename the infra-health agent to `runtime-guardian` (restoring correct naming), create a real `upstream-synchronizer` that monitors gitea fork drift from upstream go-gitea, and create a `meta-learning` agent (Mneme persona) that synthesises fleet-wide patterns from the systemd journal daily.

No Rust code changes. No new dependencies. One orchestrator restart.

### Approach

All changes are TOML conf.d edits + new template files. Deploy as: template commit → Python script edits live conf.d → single orchestrator restart → manual `git remote add upstream` on bigbox.

### Scope

**In Scope:**
- Rename `upstream-synchronizer` → `runtime-guardian` in conf.d and as a new template file
- New `upstream-synchronizer` agent: gitea fork sync (1:30am daily)
- New `meta-learning` agent: fleet pattern synthesis (11am daily)
- One-off prerequisite: add `upstream` remote to gitea fork on bigbox

**Out of Scope:**
- Quickwit integration for run record persistence
- ML-based pattern detection (shell grep/awk is sufficient)
- Alerting via Telegram or Slack
- Tracking ALL go-gitea commits (security-relevant only)

**Avoid At All Cost:**
- Adding a structured run-record database (overkill; journal provides sufficient data)
- Making meta-learning call other agents (creates circular dispatch dependency)
- Running upstream-synchronizer more than once daily (go-gitea releases are infrequent)

---

## Architecture

### Data Flow

```
Overnight agents (0-10am)
    │
    ├── write reports → /opt/ai-dark-factory/reports/infra-health-*, roadmap-*
    ├── create Gitea issues → Theme-ID: * (open issues)
    └── journal → systemd journal (agent exit classified ...)
         │
         └── 11:00am daily: meta-learning (Mneme)
              ├── sudo journalctl → exit_class stats (160 lines/day)
              ├── gtr list-issues → open Theme-IDs
              ├── cat reports/infra-health-* → latest infra state
              ├── LLM synthesis (sonnet)
              ├── gtr wiki-create → Fleet-Health-YYYYMMDD-Mneme
              └── gtr create-issue (only if P0/P1 pattern found)

01:30am daily: upstream-synchronizer (fork-sync)
    ├── git remote add upstream (if absent)
    ├── git fetch upstream --depth=100
    ├── git log HEAD..upstream/main → count + security scan
    └── gtr create-issue (only if >50 behind AND security commits)

15-past-each-hour: runtime-guardian (infra health -- unchanged function)
    ├── disk / docker / memory / runners / target/ sizes
    └── gtr create-issue (max 2/run, idempotent)
```

### Key Design Decisions

| Decision | Rationale | Alternative Rejected |
|----------|-----------|----------------------|
| Journal via `sudo -n journalctl` in meta-learning task | Confirmed zero-password; structured log data | Quickwit: requires feature compile + service |
| meta-learning at `0 11 * * *` not hourly | Needs complete overnight picture; once is enough | Hourly: redundant, wastes API calls |
| upstream-synchronizer `--depth=100` fetch | go-gitea moves slowly; deep history not needed | Full clone: 500MB+ on first run |
| Conduit persona for fork-sync | DevOps pipeline role matches; "connective tissue" | Ferrox: already used for 4 agents |
| Mneme/sonnet for meta-learning | Pattern synthesis requires reasoning; haiku too shallow | opus: too expensive for daily fleet report |

### Eliminated Options

| Option Rejected | Why | Risk of Including |
|-----------------|-----|-------------------|
| Real-time exit-class alerting | Nightwatch disabled; once-daily is right cadence | Noisy alerts on transient failures |
| Meta-learning creates sub-agent tasks | Circular dependency, harder to debug | Infinite dispatch loops |
| Track all go-gitea commits | Custom Robot API code means full merge is dangerous | Accidental overwrite of custom code |

### Simplicity Check

**What if this could be easy?**

- `runtime-guardian`: change one word (`name`) in conf.d. Done.
- `upstream-synchronizer`: 30 lines of shell + 1 LLM call. The agent is a DevOps script with a report at the end.
- `meta-learning`: 50 lines of shell to extract journal stats, pass to sonnet for synthesis, write wiki page. No database. No ML. Just grep + LLM.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimisation

---

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `scripts/adf-setup/agents/runtime-guardian.toml` | Template for infra-health agent (renamed from upstream-synchronizer) |
| `scripts/adf-setup/agents/upstream-synchronizer.toml` | Template for new fork-sync agent |
| `scripts/adf-setup/agents/meta-learning.toml` | Template for Mneme fleet-pattern agent |

### Modified Files

| File | Changes |
|------|---------|
| `/opt/ai-dark-factory/conf.d/terraphim.toml` (bigbox) | `name = "upstream-synchronizer"` → `"runtime-guardian"`; two new `[[agents]]` blocks appended |

### No Deleted Files

The old `upstream-synchronizer` conf.d entry becomes `runtime-guardian` in-place. No files deleted.

---

## Exact Changes

### Change 1: conf.d rename (runtime-guardian)

In `/opt/ai-dark-factory/conf.d/terraphim.toml`, find:
```toml
name = "upstream-synchronizer"
```
Replace with:
```toml
name = "runtime-guardian"
```
Everything else in that `[[agents]]` block stays identical.

**Python script** (`/tmp/fix_rename_us.py`):
```python
text = open('/opt/ai-dark-factory/conf.d/terraphim.toml').read()
idx = text.find('name = "upstream-synchronizer"')
if idx == -1:
    print("ERROR: not found"); exit(1)
text = text[:idx] + 'name = "runtime-guardian"' + text[idx + len('name = "upstream-synchronizer"'):]
open('/tmp/terraphim_renamed.toml', 'w').write(text)
# Verify
assert 'name = "runtime-guardian"' in text
assert 'name = "upstream-synchronizer"' not in text or text.count('name = "upstream-synchronizer"') == 0
print("OK: renamed to runtime-guardian")
```

---

### Change 2: New upstream-synchronizer conf.d block

Append to `/opt/ai-dark-factory/conf.d/terraphim.toml` BEFORE the first `[[flows]]` section (or at end of `[[agents]]` list):

```toml
[[agents]]
name = "upstream-synchronizer"
layer = "Core"
schedule = "30 1 * * *"
cli_tool = "/home/alex/.local/bin/claude"
model = "haiku"
fallback_model = "kimi-for-coding/k2p5"
fallback_provider = "/home/alex/.bun/bin/opencode"
persona = "Conduit"
skill_chain = [
    "devops",
    "git-safety-guard",
]
max_cpu_seconds = 600
grace_period_secs = 30
capabilities = [
    "upstream-sync",
    "fork-management",
    "security-patch-detection",
]
project = "terraphim"
task = "..."
```

(Full task text specified in the template file section below.)

---

### Change 3: New meta-learning conf.d block

Append after the upstream-synchronizer block:

```toml
[[agents]]
name = "meta-learning"
layer = "Core"
schedule = "0 11 * * *"
cli_tool = "/home/alex/.local/bin/claude"
model = "sonnet"
fallback_model = "kimi-for-coding/k2p5"
fallback_provider = "/home/alex/.bun/bin/opencode"
persona = "Mneme"
skill_chain = [
    "disciplined-research",
    "disciplined-verification",
]
max_cpu_seconds = 1200
grace_period_secs = 30
capabilities = [
    "meta-learning",
    "pattern-synthesis",
    "fleet-health",
    "cross-agent-analysis",
]
project = "terraphim"
task = "..."
```

(Full task text specified in template file section below.)

---

## Template Files (Complete Content)

### `scripts/adf-setup/agents/runtime-guardian.toml`

```toml
# runtime-guardian agent template
#
# Infrastructure health watchdog: disk, Docker, memory, GitHub Actions runners,
# Rust target/ directory sizes, upstream git divergence, cargo outdated.
#
# Renamed from upstream-synchronizer (which was a misnomer -- this agent
# performs runtime/infra monitoring, not fork synchronisation).
#
# Layer: Core (hourly at :15, midnight to 10am)
# Persona: Ferrox (Rust Engineer -- meticulous, zero-waste)
# Subscription-only models only (C1 constraint).

[[agents]]
name = "runtime-guardian"
layer = "Core"
cli_tool = "/home/alex/.bun/bin/opencode"
fallback_provider = "/home/alex/.bun/bin/opencode"
fallback_model = "kimi-for-coding/k2p5"
persona = "Ferrox"
terraphim_role = "DevOps Engineer"
skill_chain = [
    "disciplined-verification",
    "devops",
    "git-safety-guard",
]
schedule = "15 0-10 * * *"
max_cpu_seconds = 7200
grace_period_secs = 30
capabilities = [
    "infrastructure",
    "dependency-management",
    "health-check",
    "devops",
]
project = "terraphim"
task = '''
source ~/.profile

## Session Start -- Read Before Working

Before doing ANY work, check for learnings from previous agent runs:

1. List wiki pages for relevant learnings:
   gtr wiki-list --owner terraphim --repo terraphim-ai | grep -i "Learning-"

2. Read any learning pages matching your current task:
   gtr wiki-get --owner terraphim --repo terraphim-ai --name "Learning-<relevant>"

3. Check terraphim-agent learnings for known mistakes:
   ~/.cargo/bin/terraphim-agent learn query "infrastructure health disk docker memory"

4. Apply any relevant learnings to avoid repeating past mistakes.

---

export GITEA_TOKEN=5d663368d955953ddf900ff33420fcabebfbfb4b
export GITEA_URL=https://git.terraphim.cloud

You are the infrastructure health and runtime monitoring agent for the terraphim-ai AI Dark Factory.

## Part 1: Infrastructure Health Check

1. Disk usage (alert if > 80%):
   df -h / | tail -1

2. Docker image accumulation (main space consumer on this server):
   docker images --format '{{.Repository}}:{{.Tag}} {{.Size}}' | head -20
   docker system df

3. Memory usage (RAM-aware -- do NOT flag swap alone on this 128 GiB machine):
   free -h
   AVAIL_GiB=$(free -g | awk '/^Mem:/{print $7}')
   SWAP_FREE_MiB=$(free -m | awk '/^Swap:/{print $4}')
   if [ "${AVAIL_GiB:-0}" -lt 20 ] && [ "${SWAP_FREE_MiB:-999}" -lt 200 ]; then
     echo "MEMORY CRITICAL: swap exhausted AND available RAM below 20 GiB"
   elif [ "${AVAIL_GiB:-0}" -lt 10 ]; then
     echo "MEMORY WARNING: available RAM below 10 GiB"
   else
     echo "Memory OK: ${AVAIL_GiB} GiB available (swap state is informational only)"
   fi

4. Running services:
   systemctl is-active adf-orchestrator
   docker ps --format '{{.Names}} {{.Status}}' | head -10

5. GitHub Actions runner status:
   ls -d /home/alex/actions-runner-*/run.sh 2>/dev/null | while read r; do
     dir=$(dirname "$r")
     name=$(basename "$dir")
     echo "$name: $(pgrep -f "$dir/bin/Runner.Listener" > /dev/null && echo RUNNING || echo STOPPED)"
   done

6. Rust target directory sizes (can grow to 60G+):
   du -sh /home/alex/terraphim-ai/target/ 2>/dev/null
   du -sh /home/alex/projects/*/target/ 2>/dev/null | sort -rh | head -5

## Part 2: Dependency Sync

1. cd /home/alex/terraphim-ai && git fetch origin
2. Check for upstream commits not yet on main:
   git log HEAD..origin/main --oneline 2>/dev/null | head -10
3. Check for outdated dependencies:
   cargo outdated --root-deps-only 2>/dev/null | head -20

## Part 3: Create Issues for Problems Found

For each critical finding (disk > 85%, service down, stale deps with CVEs):

1. Check for existing issues first:
   gtr list-issues --owner terraphim --repo terraphim-ai --limit 30

2. If no existing issue covers it, create one (max 2 per run):
   gtr create-issue --owner terraphim --repo terraphim-ai \
     --title "[Infra] <short description>" \
     --body "## Problem
[What was found]

## Impact
[What breaks if not fixed]

## Fix
[Specific commands or steps to resolve]"

Rules:
- Max 2 issues per run
- Do NOT create duplicates -- always search first
- Only create issues for CRITICAL findings (service down, disk > 85%, CVEs)
- Informational findings (memory OK, runners running) go in the report only

## Part 4: Write Report

Write to /opt/ai-dark-factory/reports/infra-health-$(date +%Y%m%d-%H%M).md with:
- Disk/Memory/Docker/Runner status summary
- Any issues created this run
- Recommendations

## Session Handover

gtr wiki-create --owner terraphim --repo terraphim-ai \
  --title "Learning-$(date +%Y%m%d)-runtime-guardian" \
  --content "## Session Summary
**Agent**: runtime-guardian
**Outcome**: SUCCESS/FAIL
### Findings
- ...
### What to check next time
- ..." \
  --message "Session learning from runtime-guardian"
'''
```

---

### `scripts/adf-setup/agents/upstream-synchronizer.toml`

```toml
# upstream-synchronizer agent template
#
# Monitors divergence between the terraphim gitea fork and upstream go-gitea.
# Runs once daily at 1:30am. Adds the upstream remote if absent.
# Only creates a Gitea issue if the fork is >50 commits behind AND
# security-relevant commits (CVE, security, fix(sec)) are found upstream.
#
# Fork location: /home/alex/projects/terraphim/gitea
# Upstream: https://github.com/go-gitea/gitea.git
#
# Layer: Core (daily at 1:30am)
# Persona: Conduit (DevOps Engineer -- connective tissue, pipeline-minded)
# Subscription-only models only (C1 constraint).

[[agents]]
name = "upstream-synchronizer"
layer = "Core"
schedule = "30 1 * * *"
cli_tool = "/home/alex/.local/bin/claude"
model = "haiku"
fallback_model = "kimi-for-coding/k2p5"
fallback_provider = "/home/alex/.bun/bin/opencode"
persona = "Conduit"
skill_chain = [
    "devops",
    "git-safety-guard",
]
max_cpu_seconds = 600
grace_period_secs = 30
capabilities = [
    "upstream-sync",
    "fork-management",
    "security-patch-detection",
]
project = "terraphim"
task = '''
source ~/.profile

export GITEA_TOKEN=5d663368d955953ddf900ff33420fcabebfbfb4b
export GITEA_URL=https://git.terraphim.cloud

GITEA_FORK="/home/alex/projects/terraphim/gitea"
UPSTREAM_URL="https://github.com/go-gitea/gitea.git"

echo "=== Gitea Fork Upstream Sync Check ==="
echo "Fork: $GITEA_FORK"
echo "Upstream: $UPSTREAM_URL"
echo "Time: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# 1. Ensure fork directory exists
if [ ! -d "$GITEA_FORK/.git" ]; then
  echo "ERROR: Gitea fork not found at $GITEA_FORK"
  exit 1
fi

cd "$GITEA_FORK"

# 2. Add upstream remote if absent
if ! git remote | grep -q "^upstream$"; then
  git remote add upstream "$UPSTREAM_URL"
  echo "Added upstream remote: $UPSTREAM_URL"
else
  echo "Upstream remote already configured"
fi

# 3. Fetch upstream (shallow -- last 200 commits is sufficient for go-gitea)
echo ""
echo "Fetching upstream..."
git fetch upstream --depth=200 2>&1 | tail -5 || {
  echo "ERROR: git fetch upstream failed -- check network or URL"
  exit 1
}

# 4. Count divergence
BEHIND=$(git log HEAD..upstream/main --oneline 2>/dev/null | wc -l | tr -d ' ')
AHEAD=$(git log upstream/main..HEAD --oneline 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "=== Divergence Summary ==="
echo "Fork is $AHEAD commits ahead of upstream/main (our custom Robot API additions)"
echo "Fork is $BEHIND commits behind upstream/main (unmerged upstream changes)"

# 5. Show our custom commits (ahead)
echo ""
echo "Our custom commits (top 10):"
git log upstream/main..HEAD --oneline 2>/dev/null | head -10

# 6. If behind, scan for security-relevant upstream commits
SECURITY_COMMITS=""
if [ "$BEHIND" -gt 0 ]; then
  echo ""
  echo "=== Upstream commits we are missing (top 20) ==="
  git log HEAD..upstream/main --oneline 2>/dev/null | head -20

  SECURITY_COMMITS=$(git log HEAD..upstream/main --oneline 2>/dev/null \
    | grep -iE "CVE|security|vuln|fix.*(sec|auth|xss|sqli|csrf|injection|bypass|escalat)" \
    | head -10)

  if [ -n "$SECURITY_COMMITS" ]; then
    echo ""
    echo "=== SECURITY-RELEVANT UPSTREAM COMMITS ==="
    echo "$SECURITY_COMMITS"
  else
    echo ""
    echo "No security-relevant commits in upstream gap"
  fi
fi

# 7. Create Gitea issue only if significantly behind AND security commits found
if [ "$BEHIND" -gt 50 ] && [ -n "$SECURITY_COMMITS" ]; then
  EXISTING=$(gtr list-issues --owner terraphim --repo terraphim-ai --state open 2>/dev/null \
    | python3 -c 'import json,sys; issues=json.load(sys.stdin); [print(i["number"]) for i in issues if "Theme-ID: gitea-upstream-drift" in i.get("body","")]' \
    | head -1)

  if [ -n "$EXISTING" ]; then
    gtr comment --owner terraphim --repo terraphim-ai --index "$EXISTING" \
      --body "Upstream drift update $(date -u +%Y-%m-%dT%H:%M:%SZ): fork is now $BEHIND commits behind.

Security-relevant upstream commits:
$SECURITY_COMMITS"
  else
    gtr create-issue --owner terraphim --repo terraphim-ai \
      --title "[Infra] gitea fork $BEHIND commits behind upstream go-gitea" \
      --body "## Problem

The terraphim gitea fork (\`/home/alex/projects/terraphim/gitea\`) is $BEHIND commits behind \`go-gitea/gitea\` main branch.

## Security-Relevant Commits Missing

$SECURITY_COMMITS

## Context

The fork is $AHEAD commits ahead (our custom Robot API additions). These custom commits must be preserved when cherry-picking upstream security fixes.

## Recommended Action

Review each security commit above and cherry-pick if applicable:
\`\`\`bash
cd /home/alex/projects/terraphim/gitea
git cherry-pick <commit-sha>
# Resolve conflicts if any (our Robot API additions are in routers/api/v1/robot/)
\`\`\`

Theme-ID: gitea-upstream-drift"
  fi
elif [ "$BEHIND" -gt 0 ]; then
  echo "Fork is $BEHIND commits behind but no security commits found -- informational only"
else
  echo "Fork is up to date with upstream go-gitea -- no action needed"
fi

echo ""
echo "=== Sync check complete ==="
'''
```

---

### `scripts/adf-setup/agents/meta-learning.toml`

```toml
# meta-learning agent template (Mneme)
#
# Fleet-pattern synthesis agent. Reads the systemd journal for the last
# 24 hours of agent exits, identifies recurring patterns, reads the latest
# infra-health report, counts open Gitea Theme-IDs, and writes a daily
# Fleet-Health wiki page. Only creates a Gitea alert issue for P0/P1
# patterns (e.g. an agent failing every run, new crash pattern).
#
# Runs AFTER the overnight cron window closes (11am = after 10am last run).
# This gives it the complete picture of one full overnight cycle.
#
# Persona: Mneme -- The memory of the fleet. Eldest and wisest.
#   Observes, correlates, advises. Never acts -- only synthesises.
#
# Layer: Core (daily at 11am)
# Subscription-only models only (C1 constraint).

[[agents]]
name = "meta-learning"
layer = "Core"
schedule = "0 11 * * *"
cli_tool = "/home/alex/.local/bin/claude"
model = "sonnet"
fallback_model = "kimi-for-coding/k2p5"
fallback_provider = "/home/alex/.bun/bin/opencode"
persona = "Mneme"
skill_chain = [
    "disciplined-research",
    "disciplined-verification",
]
max_cpu_seconds = 1200
grace_period_secs = 30
capabilities = [
    "meta-learning",
    "pattern-synthesis",
    "fleet-health",
    "cross-agent-analysis",
]
project = "terraphim"
task = '''
source ~/.profile

export GITEA_TOKEN=5d663368d955953ddf900ff33420fcabebfbfb4b
export GITEA_URL=https://git.terraphim.cloud
TODAY=$(date +%Y%m%d)
YESTERDAY=$(date -d 'yesterday' +%Y%m%d 2>/dev/null || date -v-1d +%Y%m%d 2>/dev/null)

echo "=== Mneme Fleet Health Synthesis: $TODAY ==="
echo "Observing, correlating, advising."
echo ""

## Step 1: Parse systemd journal for last 24h agent exit stats

echo "--- Journal stats (last 24h) ---"
JOURNAL_RAW=$(sudo -n journalctl -u adf-orchestrator --since "24 hours ago" --no-pager 2>/dev/null \
  | grep "exit classified")

TOTAL_RUNS=$(echo "$JOURNAL_RAW" | grep -c "exit_class=" || echo 0)
echo "Total agent runs: $TOTAL_RUNS"

# Runs by agent
echo ""
echo "Runs per agent:"
echo "$JOURNAL_RAW" | grep -oP 'agent=\K[^ ]+' | sort | uniq -c | sort -rn | head -20

# Exits by class
echo ""
echo "Exit class distribution:"
echo "$JOURNAL_RAW" | grep -oP 'exit_class=\K[^ ]+' | sort | uniq -c | sort -rn

# Non-success exits (excluding empty_success -- that is normal)
FAILURES=$(echo "$JOURNAL_RAW" | grep -v "exit_class=success\b" | grep -v "exit_class=empty_success")
FAILURE_COUNT=$(echo "$FAILURES" | grep -c "exit_class=" 2>/dev/null || echo 0)
echo ""
echo "Non-success exits: $FAILURE_COUNT"
echo "$FAILURES" | grep -oP 'agent=\K[^ ]+.*exit_class=\K[^ ]+' | head -20

# Confidence-0.5 patterns (potential false positives or genuine ambiguous exits)
CONF_HALF=$(echo "$JOURNAL_RAW" | grep "confidence=0.5" \
  | grep -oP 'agent=\K[^ ]+' | sort | uniq -c | sort -rn)
if [ -n "$CONF_HALF" ]; then
  echo ""
  echo "Agents with confidence=0.5 exits (pattern/exit_code conflict):"
  echo "$CONF_HALF"
fi

# Wall-time maxing out (agents consistently hitting their budget)
WALL_MAX=$(echo "$JOURNAL_RAW" | grep -E "wall_time_secs=29[0-9]\." \
  | grep -oP 'agent=\K[^ ]+' | sort | uniq -c | sort -rn)
if [ -n "$WALL_MAX" ]; then
  echo ""
  echo "Agents hitting wall-time ceiling (>290s wall_time):"
  echo "$WALL_MAX"
fi

## Step 2: Read previous Fleet-Health wiki page for comparison

echo ""
echo "--- Yesterday's fleet health ---"
PREV_HEALTH=$(gtr wiki-get --owner terraphim --repo terraphim-ai \
  --name "Fleet-Health-${YESTERDAY}-Mneme" 2>/dev/null | python3 -c \
  'import json,sys; d=json.load(sys.stdin); print(d.get("content","") or d.get("body",""))' \
  2>/dev/null | head -30 || echo "(no previous report)")
echo "$PREV_HEALTH" | head -20

## Step 3: Read latest infra-health report

echo ""
echo "--- Latest infra state ---"
LATEST_INFRA=$(ls -t /opt/ai-dark-factory/reports/infra-health-*.md 2>/dev/null | head -1)
if [ -n "$LATEST_INFRA" ]; then
  echo "Source: $LATEST_INFRA"
  cat "$LATEST_INFRA" | tail -30
else
  echo "(no infra-health report found)"
fi

## Step 4: Count open Gitea issues by Theme-ID

echo ""
echo "--- Open Gitea issues with Theme-IDs ---"
OPEN_ISSUES=$(gtr list-issues --owner terraphim --repo terraphim-ai --state open 2>/dev/null)
THEME_IDS=$(echo "$OPEN_ISSUES" | python3 -c '
import json, sys, re
issues = json.load(sys.stdin)
themes = {}
for i in issues:
    body = i.get("body", "") or ""
    m = re.search(r"Theme-ID:\s*(\S+)", body)
    if m:
        tid = m.group(1)
        themes[tid] = themes.get(tid, 0) + 1
for tid, count in sorted(themes.items(), key=lambda x: -x[1]):
    print(f"  {count:2d}x  {tid}")
' 2>/dev/null || echo "(could not parse issues)")
echo "$THEME_IDS"

OPEN_COUNT=$(echo "$OPEN_ISSUES" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))' 2>/dev/null || echo "?")
echo "Total open issues: $OPEN_COUNT"

## Step 5: LLM synthesis -- pattern identification and recommendations

SYNTHESIS_PROMPT="You are Mneme, the meta-learning agent for the terraphim-ai AI Dark Factory. You are the fleet's memory and pattern keeper.

Your role today: synthesise the overnight run data and identify patterns worth noting. You NEVER implement code, dispatch agents, or create branches. You observe, correlate, and advise.

## Fleet data for $TODAY

**Journal summary (last 24h):**
Total runs: $TOTAL_RUNS
Failures: $FAILURE_COUNT
Confidence=0.5 agents: $CONF_HALF
Wall-time ceiling agents: $WALL_MAX

**Open Gitea issues (Theme-IDs):**
$THEME_IDS

**Latest infra state:**
$(cat "$LATEST_INFRA" 2>/dev/null | tail -20)

**Yesterday's fleet health (for comparison):**
$PREV_HEALTH

## Your task

1. Identify up to 3 patterns worth flagging. For each:
   - Name the pattern
   - Severity: P0 (agent completely broken), P1 (degraded, >50% failure rate), P2 (informational trend), P3 (observation only)
   - Evidence (which agents, how many runs affected)
   - Recommendation (what a human should look at)

2. Classify any P0/P1 patterns. These warrant a Gitea alert issue.

3. Note any improvements vs yesterday (patterns resolved, exit_class distribution better).

4. Write a concise fleet health verdict: HEALTHY / DEGRADED / CRITICAL

Output format:
VERDICT: <HEALTHY|DEGRADED|CRITICAL>
PATTERNS:
- P<N> <name>: <evidence> -- <recommendation>
IMPROVEMENTS:
- <what got better>
NOTES:
- <other observations>"

SYNTHESIS=$(/home/alex/.local/bin/claude -p --model sonnet --allowedTools "" --max-turns 3 "$SYNTHESIS_PROMPT" 2>&1)
echo ""
echo "--- Synthesis ---"
echo "$SYNTHESIS"

## Step 6: Write Fleet-Health wiki page

WIKI_CONTENT="## Fleet Health Report: $TODAY

**Agent**: meta-learning (Mneme)
**Time**: $(date -u +%Y-%m-%dT%H:%M:%SZ)

### Verdict

$(echo "$SYNTHESIS" | grep "^VERDICT:" || echo "VERDICT: UNKNOWN")

### Patterns

$(echo "$SYNTHESIS" | awk '/^PATTERNS:/,/^IMPROVEMENTS:|^NOTES:|^$/')

### Improvements vs Yesterday

$(echo "$SYNTHESIS" | awk '/^IMPROVEMENTS:/,/^NOTES:|^$/')

### Raw Stats

- Total runs: $TOTAL_RUNS
- Non-success exits: $FAILURE_COUNT
- Open issues with Theme-IDs: $(echo "$THEME_IDS" | wc -l)
- Open issues total: $OPEN_COUNT

### Notes

$(echo "$SYNTHESIS" | awk '/^NOTES:/,0')"

gtr wiki-create --owner terraphim --repo terraphim-ai \
  --title "Fleet-Health-${TODAY}-Mneme" \
  --content "$WIKI_CONTENT" \
  --message "Daily fleet health synthesis from Mneme" \
  2>&1 || echo "Warning: wiki-create failed (page may already exist)"

echo "Fleet-Health-${TODAY}-Mneme wiki page written"

## Step 7: Create Gitea alert issue ONLY for P0/P1 patterns

P0P1=$(echo "$SYNTHESIS" | grep -E "^- P[01] " | head -5)
VERDICT=$(echo "$SYNTHESIS" | grep "^VERDICT:" | grep -oE "HEALTHY|DEGRADED|CRITICAL")

if [ -n "$P0P1" ] && [ "$VERDICT" != "HEALTHY" ]; then
  # Check if an alert issue was already created today
  EXISTING=$(gtr list-issues --owner terraphim --repo terraphim-ai --state open 2>/dev/null \
    | python3 -c "import json,sys; issues=json.load(sys.stdin); [print(i['number']) for i in issues if 'Theme-ID: adf-fleet-health-alert' in i.get('body','') and '${TODAY}' in i.get('title','')]" \
    | head -1)

  if [ -z "$EXISTING" ]; then
    gtr create-issue --owner terraphim --repo terraphim-ai \
      --title "[ADF] Fleet health alert $TODAY: $VERDICT" \
      --body "## Mneme Fleet Health Alert

**Date**: $TODAY
**Verdict**: $VERDICT

## P0/P1 Patterns

$P0P1

## Full Synthesis

$SYNTHESIS

## Data Sources

- Journal: $TOTAL_RUNS agent runs analysed
- Open issues: $OPEN_COUNT ($THEME_IDS)

Theme-ID: adf-fleet-health-alert"
    echo "Created fleet health alert issue"
  else
    echo "Alert issue already exists for today (#$EXISTING); skipping"
  fi
else
  echo "Verdict: $VERDICT -- no P0/P1 patterns -- no alert issue created"
fi

echo ""
echo "=== Mneme synthesis complete ==="
'''
```

---

## Deployment Sequence

### Step 0 (prerequisite -- manual, one-off on bigbox)

```bash
cd /home/alex/projects/terraphim/gitea
git remote add upstream https://github.com/go-gitea/gitea.git
echo "Done"
```

Verify: `git remote -v | grep upstream`

### Step 1: Commit template files to git (local)

Create the three `.toml` files in `scripts/adf-setup/agents/` and commit.

### Step 2: Apply conf.d changes on bigbox

Python script `/tmp/apply_gap_fixes.py`:
1. Load current `/opt/ai-dark-factory/conf.d/terraphim.toml`
2. Rename `upstream-synchronizer` → `runtime-guardian` in the existing agent block
3. Append new `upstream-synchronizer` block (fork-sync) before `[[flows]]` section
4. Append new `meta-learning` block after the new `upstream-synchronizer`
5. Write to `/tmp/terraphim_gap_fixed.toml`
6. Verify: assert both new agent names present, old name absent
7. `sudo cp` to conf.d

### Step 3: Restart orchestrator once

```bash
sudo systemctl restart adf-orchestrator
echo "Restarted"
```

### Step 4: Verify in journal

```bash
# At next :15 past the hour -- should see runtime-guardian not upstream-synchronizer
sudo journalctl -u adf-orchestrator -f | grep "cron schedule"
# Expected: "cron schedule fired agent=runtime-guardian"
# NOT: "cron schedule fired agent=upstream-synchronizer" (until 1:30am)
```

---

## Test Strategy

### Immediate (after restart)

| Check | Command | Expected |
|-------|---------|----------|
| runtime-guardian fires at :15 | `journalctl \| grep cron.*runtime-guardian` | Fires at next :15 |
| upstream-synchronizer NOT at :15 | `journalctl \| grep cron.*upstream-synchronizer` | Silent until 1:30am |
| No conf.d parse errors | `systemctl status adf-orchestrator` | Active, no errors |

### Tonight (1:30am)

| Check | Expected |
|-------|---------|
| `cron schedule fired agent=upstream-synchronizer` in journal | Yes |
| Gitea fork divergence reported | Stdout shows `$AHEAD` and `$BEHIND` counts |
| No issue created (likely no security commits) | No new `[Infra] gitea fork` issue |

### Tomorrow (11am)

| Check | Expected |
|-------|---------|
| `cron schedule fired agent=meta-learning` in journal | Yes |
| `Fleet-Health-YYYYMMDD-Mneme` wiki page created | `gtr wiki-get ...` returns content |
| Verdict line present | `VERDICT: HEALTHY` (or appropriate) |
| No spurious alert issue | Only created if genuine P0/P1 |

---

## Rollback Plan

If any agent misbehaves:
1. Stop it: edit conf.d, change `schedule` to `"0 0 31 2 *"` (never fires), restart
2. Full rollback: `sudo cp /tmp/terraphim_renamed.toml /opt/ai-dark-factory/conf.d/terraphim.toml` and restart (pre-apply backup)

---

## Open Items

| Item | Status |
|------|--------|
| Manual `git remote add upstream` on bigbox | Must be done before upstream-synchronizer first runs at 1:30am |
| Verify `date -d 'yesterday'` works on bigbox (GNU date) | Low risk -- fallback `date -v-1d` included for macOS |

---

## Approval

- [ ] Template file content reviewed (runtime-guardian, upstream-synchronizer, meta-learning)
- [ ] Deployment sequence approved
- [ ] Human sign-off received
