# ROC v1 Staged Rollout Runbook

ROC v1 Step L — Refs terraphim/adf-fleet#40

Staged rollout of the full ROC v1 agent fleet to production projects.
Order: digital-twins (48h soak) → odilo (48h soak) → terraphim.

---

## Pre-flight Checklist

Complete all items before touching any project configuration.

### 1. Verify orchestrator version

All ROC v1 Steps A–K must be merged into `main` on bigbox before this runbook is executed.

```bash
ssh bigbox
cd /opt/ai-dark-factory/workspace
git log --oneline -5
# Confirm the merge commit for ROC v1 Step K (cron cadence) is present.
```

### 2. Confirm orchestrator service is healthy

```bash
ssh bigbox
sudo systemctl status adf-orchestrator
# Expected: Active: active (running)
journalctl -u adf-orchestrator --since "10 minutes ago" --no-pager | tail -20
# No FATAL or PANIC lines.
```

### 3. Verify webhook endpoint is reachable from bigbox

```bash
ssh bigbox
curl -sf https://adf-orchestrator.terraphim.internal/webhooks/gitea \
  -X POST -H "Content-Type: application/json" -d '{}' \
  -o /dev/null -w "%{http_code}\n"
# Expected: 4xx (payload rejected, but endpoint alive). 000 = not reachable.
```

### 4. Confirm Quickwit indices exist

```bash
ssh bigbox
curl -s http://127.0.0.1:7280/api/v1/indexes | jq '[.[].index_id]'
# Must include at minimum: adf-events-digital-twins, adf-events-odilo, adf-events-terraphim
# Create missing indices via the Quickwit UI or CLI before proceeding.
```

### 5. Read the webhook secret

```bash
ssh bigbox
grep -A3 '\[webhook\]' /opt/ai-dark-factory/orchestrator.toml
# Note the `secret` value. Used in webhook registration commands below.
# Do NOT echo this secret into terminal history.
```

---

## Phase 1: digital-twins (48h soak)

### Step 1.1 — Register Gitea webhook

Run from bigbox (replace `<WEBHOOK_SECRET>` with the value from Pre-flight step 5):

```bash
curl -s -X POST \
  -H "Authorization: token $GITEA_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/digital-twins/hooks" \
  -d '{
    "type": "gitea",
    "config": {
      "url": "https://adf-orchestrator.terraphim.internal/webhooks/gitea",
      "content_type": "json",
      "secret": "<WEBHOOK_SECRET>"
    },
    "events": ["pull_request", "issue_comment"],
    "active": true
  }' | jq '{id: .id, active: .active, events: .events}'
# Note the returned hook id for rollback reference.
```

### Step 1.2 — Update conf.d cron for digital-twins

Edit the live config on bigbox. This is **operator work**: do not apply via automation.

```bash
ssh bigbox
sudo nano /opt/ai-dark-factory/conf.d/digital-twins.toml
```

Find every `implementation-swarm` agent block and apply this diff:

```toml
# Before (hourly):
schedule = "0 * * * *"

# After (every 20 minutes):
schedule = "*/20 * * * *"
```

Reviewer (`pr-reviewer`) blocks have no `schedule` field — leave them unchanged.
Meta (`project-meta`, `fleet-meta`) blocks remain at `*/30 * * * *` — leave unchanged.

Validate the edited file:

```bash
adf --check /opt/ai-dark-factory/conf.d/digital-twins.toml
# Expected: OK (no parse errors)
```

### Step 1.3 — Restart orchestrator

```bash
sudo systemctl restart adf-orchestrator
sleep 5
sudo systemctl status adf-orchestrator
# Expected: Active: active (running)
```

### Step 1.4 — Verify webhook delivery (smoke test)

Open a test comment on any `terraphim/digital-twins` issue, then check:

```bash
curl -s "http://127.0.0.1:7280/api/v1/adf-events-digital-twins/search?query=webhook_received&max_hits=5" \
  | jq '.num_hits'
# Expected: >= 1 within 2 minutes of the comment.
```

If 0 after 5 minutes, check orchestrator logs:

```bash
journalctl -u adf-orchestrator --since "5 minutes ago" --no-pager | grep -i "webhook\|digital-twins"
```

### Step 1.5 — Monitor for 48 hours

Run acceptance queries at T+24h and T+48h (see Acceptance Queries section below).

---

## Phase 1 Acceptance Gate

Both conditions must be true before proceeding to Phase 2.

```bash
# Revert rate must be 0:
curl -s "http://127.0.0.1:7280/api/v1/adf-events-digital-twins/search?query=pr_auto_reverted&start_timestamp=$(date -d '48 hours ago' +%s)&max_hits=0" \
  | jq '.num_hits'
# MUST be 0 to proceed.

# At least one verified auto-merge:
curl -s "http://127.0.0.1:7280/api/v1/adf-events-digital-twins/search?query=pr_auto_merged_verified&start_timestamp=$(date -d '48 hours ago' +%s)&max_hits=1" \
  | jq '.num_hits'
# MUST be >= 1 to proceed.
```

If either condition fails, execute the Rollback Procedure for digital-twins before touching odilo.

---

## Phase 2: odilo (48h soak)

**Prerequisite**: Phase 1 acceptance gate passed.

### Step 2.1 — Register Gitea webhook

```bash
curl -s -X POST \
  -H "Authorization: token $GITEA_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/odilo/hooks" \
  -d '{
    "type": "gitea",
    "config": {
      "url": "https://adf-orchestrator.terraphim.internal/webhooks/gitea",
      "content_type": "json",
      "secret": "<WEBHOOK_SECRET>"
    },
    "events": ["pull_request", "issue_comment"],
    "active": true
  }' | jq '{id: .id, active: .active, events: .events}'
```

### Step 2.2 — Update conf.d cron for odilo

```bash
ssh bigbox
sudo nano /opt/ai-dark-factory/conf.d/odilo.toml
```

Apply the same diff as Phase 1 Step 1.2 to all `implementation-swarm` blocks.

```bash
adf --check /opt/ai-dark-factory/conf.d/odilo.toml
```

### Step 2.3 — Restart orchestrator

```bash
sudo systemctl restart adf-orchestrator
sleep 5
sudo systemctl status adf-orchestrator
```

### Step 2.4 — Verify webhook delivery

```bash
curl -s "http://127.0.0.1:7280/api/v1/adf-events-odilo/search?query=webhook_received&max_hits=5" \
  | jq '.num_hits'
```

### Step 2.5 — Monitor for 48 hours

---

## Phase 2 Acceptance Gate

```bash
# Revert rate must be 0:
curl -s "http://127.0.0.1:7280/api/v1/adf-events-odilo/search?query=pr_auto_reverted&start_timestamp=$(date -d '48 hours ago' +%s)&max_hits=0" \
  | jq '.num_hits'
# MUST be 0 to proceed.

# At least one verified auto-merge:
curl -s "http://127.0.0.1:7280/api/v1/adf-events-odilo/search?query=pr_auto_merged_verified&start_timestamp=$(date -d '48 hours ago' +%s)&max_hits=1" \
  | jq '.num_hits'
# MUST be >= 1 to proceed.
```

If either condition fails, execute the Rollback Procedure for odilo.

---

## Phase 3: terraphim

**Prerequisite**: Phase 2 acceptance gate passed.

### Step 3.1 — Register Gitea webhook

```bash
curl -s -X POST \
  -H "Authorization: token $GITEA_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/hooks" \
  -d '{
    "type": "gitea",
    "config": {
      "url": "https://adf-orchestrator.terraphim.internal/webhooks/gitea",
      "content_type": "json",
      "secret": "<WEBHOOK_SECRET>"
    },
    "events": ["pull_request", "issue_comment"],
    "active": true
  }' | jq '{id: .id, active: .active, events: .events}'
```

### Step 3.2 — Update conf.d cron for terraphim-ai

```bash
ssh bigbox
sudo nano /opt/ai-dark-factory/conf.d/terraphim-ai.toml
```

Apply the same `implementation-swarm` schedule diff:

```toml
# Before:
schedule = "0 * * * *"

# After:
schedule = "*/20 * * * *"
```

```bash
adf --check /opt/ai-dark-factory/conf.d/terraphim-ai.toml
```

### Step 3.3 — Restart orchestrator

```bash
sudo systemctl restart adf-orchestrator
sleep 5
sudo systemctl status adf-orchestrator
```

### Step 3.4 — Verify webhook delivery

```bash
curl -s "http://127.0.0.1:7280/api/v1/adf-events-terraphim/search?query=webhook_received&max_hits=5" \
  | jq '.num_hits'
```

### Step 3.5 — Monitor for 48 hours

---

## Phase 3 Acceptance Gate

```bash
# Revert rate must be 0:
curl -s "http://127.0.0.1:7280/api/v1/adf-events-terraphim/search?query=pr_auto_reverted&start_timestamp=$(date -d '48 hours ago' +%s)&max_hits=0" \
  | jq '.num_hits'
# MUST be 0 to proceed.

# At least one verified auto-merge:
curl -s "http://127.0.0.1:7280/api/v1/adf-events-terraphim/search?query=pr_auto_merged_verified&start_timestamp=$(date -d '48 hours ago' +%s)&max_hits=1" \
  | jq '.num_hits'
# MUST be >= 1.
```

---

## Rollback Procedure

Execute if revert rate > 0 in any phase, or if the orchestrator becomes unhealthy.

### 1. Disable the webhook for the affected project

```bash
# List hooks for the project:
curl -s -H "Authorization: token $GITEA_ADMIN_TOKEN" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/<PROJECT>/hooks" \
  | jq '.[] | {id, active}'

# Disable (replace <HOOK_ID>):
curl -s -X PATCH \
  -H "Authorization: token $GITEA_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/<PROJECT>/hooks/<HOOK_ID>" \
  -d '{"active": false}'
```

### 2. Revert the cron bump in conf.d

```bash
ssh bigbox
sudo nano /opt/ai-dark-factory/conf.d/<PROJECT>.toml
# Restore: schedule = "0 * * * *"  for all implementation-swarm blocks
adf --check /opt/ai-dark-factory/conf.d/<PROJECT>.toml
```

### 3. Restart orchestrator

```bash
sudo systemctl restart adf-orchestrator
sudo systemctl status adf-orchestrator
```

### 4. Open a retro issue on adf-fleet

```bash
gtr create-issue --owner terraphim --repo adf-fleet \
  --title "ROC v1 rollout retro: <PROJECT> phase reverted" \
  --body "Phase <N> soak for <PROJECT> failed acceptance gate.

Failure: revert_rate > 0 (or: no pr_auto_merged_verified events observed).

Quickwit evidence:
\`\`\`
<paste curl output here>
\`\`\`

Action: webhook disabled, cron reverted to 0 * * * *, orchestrator restarted." \
  --labels "retro,roc-v1,priority/high"
```

Do not proceed to the next phase until the retro issue is resolved.

---

## Observability: Quickwit Search Queries

Use these queries during and after each soak period to track fleet activity.
Replace `<INDEX>` with the project-specific index name.

| Event | Index suffix | Meaning |
|-------|-------------|---------|
| `pr_reviewed` | per-project | PR reviewer agent completed a review |
| `pr_auto_merged` | per-project | Merge-coordinator merged a PR automatically |
| `pr_auto_reverted` | per-project | A merged PR was subsequently reverted |
| `pr_auto_merged_verified` | per-project | Auto-merged PR passed post-merge test gate |

### Active event count (last 48h)

```bash
PROJECT=digital-twins   # or: odilo, terraphim
INDEX="adf-events-${PROJECT}"
SINCE=$(date -d '48 hours ago' +%s)

for event in pr_reviewed pr_auto_merged pr_auto_reverted pr_auto_merged_verified; do
  count=$(curl -s "http://127.0.0.1:7280/api/v1/${INDEX}/search?query=${event}&start_timestamp=${SINCE}&max_hits=0" | jq '.num_hits')
  echo "${event}: ${count}"
done
```

Expected healthy output after 48h soak:

```
pr_reviewed: <N>          # >0
pr_auto_merged: <N>       # >0
pr_auto_reverted: 0       # must be exactly 0
pr_auto_merged_verified: <N>  # >=1
```

### Tail recent events (troubleshooting)

```bash
curl -s "http://127.0.0.1:7280/api/v1/${INDEX}/search?query=*&max_hits=20&sort_by_field=timestamp&sort_order=desc" \
  | jq '.hits[] | {timestamp, event_type, pr_number, project}'
```

---

## Config Delta Reference

Shown for reference only. The operator applies these changes manually (see per-phase steps above). Do **not** apply these diffs via automation.

### `/opt/ai-dark-factory/conf.d/<PROJECT>.toml` — implementation-swarm blocks

```diff
 [[agents]]
 name = "implementation-swarm"
 layer = "Core"
-schedule = "0 * * * *"
+schedule = "*/20 * * * *"
```

All other agent blocks are unchanged:

- `project-meta` and `fleet-meta` remain at `*/30 * * * *`.
- `pr-reviewer` has no `schedule` field (event-driven via webhook mention dispatch).
- Safety tier agents (`security-sentinel`, `compliance-watchdog`, `drift-detector`, `test-guardian`, `spec-validator`, `documentation-generator`) retain their own schedules.

---

## Summary Checklist

| Step | digital-twins | odilo | terraphim |
|------|:---:|:---:|:---:|
| Webhook registered | [ ] | [ ] | [ ] |
| conf.d cron updated | [ ] | [ ] | [ ] |
| Orchestrator restarted | [ ] | [ ] | [ ] |
| Webhook delivery verified | [ ] | [ ] | [ ] |
| 48h soak complete | [ ] | [ ] | [ ] |
| pr_auto_reverted = 0 | [ ] | [ ] | [ ] |
| pr_auto_merged_verified >= 1 | [ ] | [ ] | [ ] |
