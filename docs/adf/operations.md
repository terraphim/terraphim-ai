# ADF Operations Guide

Day-to-day operations for the AI Dark Factory orchestrator on bigbox.
Covers the `adf-ctl` CLI, service management, monitoring, rch build dispatch, and known operational procedures.

## Quick reference

```bash
# Status snapshot
adf-ctl status

# Trigger an agent (fire-and-forget)
adf-ctl trigger <agent-name>

# Trigger and wait for completion (up to 20 min)
adf-ctl trigger <agent-name> --wait --timeout 1200

# List configured agents
adf-ctl agents

# Cancel a running agent (best-effort)
adf-ctl cancel <agent-name>
```

`adf-ctl` is installed locally at
`/Users/alex/projects/terraphim/terraphim-ai/target/release/adf-ctl`.
It operates over SSH to bigbox and requires `ADF_WEBHOOK_SECRET` or reads
the secret from `/opt/ai-dark-factory/orchestrator.toml` via SSH.

---

## Service management

```bash
# Restart (picks up config changes and new persona files)
sudo systemctl restart adf-orchestrator

# Status
sudo systemctl status adf-orchestrator

# Live journal
journalctl -u adf-orchestrator -f --no-pager

# Last 1h of agent activity
journalctl -u adf-orchestrator --since '1h ago' --no-pager \
  | grep -E 'exit classified|spawning agent'

# Non-success exits only
journalctl -u adf-orchestrator --since '24h ago' --no-pager \
  | grep 'exit classified' \
  | grep -v 'exit_class=success\|exit_class=empty_success'
```

---

## adf-ctl trigger

`adf-ctl trigger` sends a synthetic webhook to the orchestrator. The
orchestrator processes it at the next reconciliation tick (up to 300s delay).

### How it works

1. Builds a JSON payload: `{"action":"created","comment":{"body":"@adf:<name>"},...}`
2. Signs it with HMAC-SHA256 using the webhook secret
3. SSHes into bigbox and pipes the payload to `curl --data-binary @-`
4. The orchestrator receives it, parses `@adf:<name>`, resolves the agent,
   and queues a `SpawnAgent` dispatch

### Secret resolution order

1. `--secret <S>` flag
2. `ADF_WEBHOOK_SECRET` env var
3. SSH read from `/opt/ai-dark-factory/orchestrator.toml`

### Tick delay

The dispatch is queued on webhook receipt but processed at the next
reconciliation tick (`tick_interval_secs = 300`). Expect up to 5 min before
the agent spawns. Use `--wait` to block until the journal shows the exit line.

### Limitation: cross-project agents

`adf-ctl trigger` hardcodes `"repository": {"full_name": "terraphim/terraphim-ai"}`
in the payload. Agents in `conf.d/odilo.toml` or `conf.d/digital-twins.toml`
will NOT be found.

**Workaround for other projects:**
```bash
# On bigbox -- sign and POST with the correct repo
SECRET=$(sudo grep "secret" /opt/ai-dark-factory/orchestrator.toml | head -1 \
         | grep -oP '"[^"]+"' | tr -d '"')
NOW=$(date -u '+%Y-%m-%dT%H:%M:%S.000Z')
PAYLOAD='{"action":"created","comment":{"id":1,"body":"@adf:odilo-reviewer",
  "user":{"login":"adf-cli"},"created_at":"'$NOW'"},
  "issue":{"number":0,"title":"CLI trigger","state":"open"},
  "repository":{"full_name":"zestic-ai/odilo"}}'
SIG=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $2}')
curl -s -X POST http://172.18.0.1:9091/webhooks/gitea \
  -H 'X-Gitea-Event: issue_comment' \
  -H "X-Gitea-Signature: sha256=$SIG" \
  -H 'Content-Type: application/json' \
  --data-binary "$PAYLOAD"
```

---

## Required config: top-level [mentions]

`handle_webhook_dispatch` checks `self.config.mentions` (top-level) before
processing any webhook-triggered dispatch. Without it, ALL webhook dispatches
are silently dropped.

The current fix is a top-level `[mentions]` section in `orchestrator.toml`:

```toml
[mentions]
poll_modulo = 2
max_dispatches_per_tick = 3
max_concurrent_mention_agents = 8
```

---

## PR fan-out (ADF replaces Gitea Actions)

### Overview

Since 2026-04-27, every `pull_request.opened` (or reopened) event on
`terraphim/terraphim-ai` triggers a 6-agent fan-out via `[pr_dispatch]`:

| Agent | Gitea status context | Role |
|---|---|---|
| build-runner | `adf/build` | cargo fmt + clippy + test via rch |
| pr-reviewer | `adf/pr-reviewer` | structural PR review (claude sonnet) |
| pr-spec-validator | `adf/spec` | requirements traceability |
| pr-security-sentinel | `adf/security` | licence + CVE + secrets scan |
| pr-compliance-watchdog | `adf/compliance` | responsible-AI compliance |
| pr-test-guardian | `adf/test` | test coverage and contract review |

All 6 contexts are required status checks on `main` (branch protection).
A PR cannot be merged until all 6 post a non-pending result.

### How the build-runner calls rch

`build-runner` is a pure-bash agent that dispatches cargo commands through
`rch exec` (remote compilation helper):

```
rch is at: /home/alex/.local/bin/rch
rchd is at: /home/alex/.local/bin/rchd
rchd service: user daemon (PID varies), started at boot via ~/.config/systemd/user/
rch workers: 1 worker (bigbox-local) at 127.0.0.1, 6 slots, all healthy
```

Check rch health:
```bash
# On bigbox
/home/alex/.local/bin/rch status
/home/alex/.local/bin/rch workers probe --all
```

The build-runner script runs from `GITEA_WORKING_DIR=/home/alex/terraphim-ai`
and calls:
```bash
/home/alex/.local/bin/rch exec -- cargo fmt --all -- --check
/home/alex/.local/bin/rch exec -- cargo clippy --workspace --all-targets -- -D warnings
/home/alex/.local/bin/rch exec -- cargo test --workspace --no-fail-fast
```

`rch exec` inherits the CWD of the calling process. After `cd "$GITEA_WORKING_DIR"`,
rch SSH-dispatches to 127.0.0.1 and finds the Cargo workspace there.

### Manually unblocking a PR (bootstrap workaround)

When the spawner bug was active or during initial deployment, status checks
could not post. To unblock a PR temporarily:

```bash
TOKEN="..."
SHA="<40-char head SHA>"
for ctx in adf/build adf/pr-reviewer adf/spec adf/security adf/compliance adf/test; do
  curl -s -X POST \
    -H "Authorization: token $TOKEN" \
    -H "Content-Type: application/json" \
    "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/statuses/$SHA" \
    -d "{\"state\":\"success\",\"context\":\"$ctx\",\"description\":\"manually unblocked\"}"
done
```

Do NOT use this as a permanent workflow. Fix the underlying agent instead.

### Re-triggering a PR

If agents missed a PR open event (e.g. orchestrator was restarting):
```bash
# Via API (close then reopen)
TOKEN="..."
curl -X PATCH -H "Authorization: token $TOKEN" -H "Content-Type: application/json" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/pulls/NNN" \
  -d '{"state":"closed"}'
sleep 3
curl -X PATCH -H "Authorization: token $TOKEN" -H "Content-Type: application/json" \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/pulls/NNN" \
  -d '{"state":"open"}'
```

The orchestrator must be running when the reopen fires (webhook is not queued
across restarts).

---

## Persona loading

Personas are loaded **once at startup** from `persona_data_dir`
(`/home/alex/terraphim-ai/data/personas/`). If a persona file is added or
modified after the orchestrator starts, it will not be picked up until restart.

Available personas (as of 2026-04-27): Carthos, Conduit, Echo, Ferrox, Lux,
Meridian, Mneme, Themis, Vigil — 9 total.

---

## Monitoring: key journal patterns

```bash
# All spawns
grep 'spawning agent'

# All exits with class
grep 'exit classified'

# PR fan-out dispatched
grep 'ReviewPr spawned'

# PR fan-out skipped (no agent for project, budget, allow-list)
grep 'ReviewPr skipped'

# Commit status posted
grep 'post_pending_status\|posted.*status'

# Persona enrichment applied
grep 'composed persona-enriched prompt'

# Skills injected
grep 'injecting skill_chain'

# Gitea post success
grep 'posted agent output to Gitea'

# Gitea post failed (issue=0 expected for CLI triggers)
grep 'failed to post output'

# Worktree created/removed (isolation working)
grep 'created isolated git worktree\|removed agent worktree'

# Wall-clock kills
grep 'exceeded wall-clock timeout'

# Dedup guard firing
grep 'skipping dispatch'

# Circuit breaker events
grep 'Circuit breaker\|circuit breaker'

# Persona not found
grep 'persona not found'

# rch build events (look for task_len in spawner audit)
grep 'task_len'
```

---

## Stale worktree cleanup

When the orchestrator restarts mid-run, agent processes are killed but their
git worktrees are not cleaned up. Over time these accumulate.

**Check:**
```bash
# On bigbox
ls /tmp/adf-worktrees/ | wc -l
du -sh /tmp/adf-worktrees/
git -C /home/alex/terraphim-ai worktree list | wc -l
```

**Clean (safe to run — preserves worktrees modified in last 30 min):**
```bash
# On bigbox
KEEP=$(find /tmp/adf-worktrees/ -maxdepth 1 -mindepth 1 -type d -mmin -30 \
       -printf '%f\n' | tr '\n' ' ')

for dir in /tmp/adf-worktrees/*/; do
  name=$(basename "$dir")
  echo "$KEEP" | grep -qF "$name" && continue
  git -C /home/alex/terraphim-ai worktree remove --force "$dir" 2>/dev/null
done
git -C /home/alex/terraphim-ai worktree prune
rm -rf /tmp/adf-worktrees/sentinel-*/
```

---

## Provider probe failures

The orchestrator probes all providers at startup and periodically. Expected
failures as of 2026-04-27:
- `openai/gpt-5.3-codex`
- `openai/gpt-5.4`
- `openai/gpt-5.4-mini`
- `minimax-coding-plan/MiniMax-M2.5`

These are in the KG routing tables under
`docs/taxonomy/routing_scenarios/adf/`. Do NOT remove them.

---

## Timeout configuration

| Agent | max_cpu_seconds | Notes |
|---|---|---|
| build-runner | 1800 | includes full test suite via rch |
| pr-reviewer | 600 | structural review |
| pr-spec-validator | 7200 | can run long on large diffs |
| pr-security-sentinel | 7200 | |
| pr-compliance-watchdog | 7200 | |
| pr-test-guardian | 7200 | |
| security-sentinel | 1200 | cron full-repo audit |
| meta-coordinator | 1200 | bumped from 300 on 2026-04-27 |
| runtime-guardian | 1200 | |
| compliance-watchdog | 7200 | |
| drift-detector | 7200 | |
| spec-validator | 7200 | can run 50+ min on large backlogs |
| test-guardian | 7200 | 119 min observed for full test suite |
| odilo-developer | 7200 | |
| developer/implementation-swarm | 7200 | |

---

## Config file locations on bigbox

| File | Purpose |
|---|---|
| `/opt/ai-dark-factory/orchestrator.toml` | Top-level config (persona dir, skill dir, tick interval, [mentions], [pr_dispatch]) |
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | 25 terraphim agents + project mentions config |
| `/opt/ai-dark-factory/conf.d/odilo.toml` | 2 odilo agents |
| `/opt/ai-dark-factory/conf.d/digital-twins.toml` | 2 digital-twins agents |
| `/opt/ai-dark-factory/agent_tokens.json` | Per-agent Gitea tokens for attribution |
| `/opt/ai-dark-factory/persona_roles_config.json` | terraphim-agent KG role config for in-task searches |
| `/opt/ai-dark-factory/skills/` | Skill SKILL.md files injected into prompts |
| `/opt/ai-dark-factory/scenarios/` | Scenario files for browser-qa |
| `/opt/ai-dark-factory/reports/` | Agent-written reports |
| `/home/alex/terraphim-ai/data/personas/` | Persona TOML files (loaded at startup) |
| `/home/alex/terraphim-ai/docs/taxonomy/routing_scenarios/adf/` | KG routing tables |

**Important:** `[pr_dispatch]` must be in the top-level `orchestrator.toml`.
The `IncludeFragment` parser (used by `conf.d/*.toml`) rejects it.

---

## Validate config before restart

```bash
sudo /usr/local/bin/adf --check /opt/ai-dark-factory/orchestrator.toml
```

Prints the full routing table per agent. Exits non-zero on TOML parse errors
or agent-to-project mismatches.

---

## Rebuilding the ADF binary on bigbox

```bash
# On bigbox
cd ~/projects/terraphim/terraphim-ai
git fetch gitea && git reset --hard gitea/main
cargo build --release -p terraphim_orchestrator --bin adf
sudo install -m755 target/release/adf /usr/local/bin/adf
sudo systemctl restart adf-orchestrator
sleep 5
systemctl is-active adf-orchestrator
journalctl -u adf-orchestrator --no-pager -n 10
```

Incremental builds after a spawner-only change take ~45s. Full rebuilds from
scratch take ~10 min.

**Note:** bigbox tracks the Gitea remote as `gitea`, not `origin`. GitHub is
`origin`. Always pull from `gitea` before rebuilding.

---

## Further reading

- `docs/adf/agent-fleet.md` — agent roster with roles, personas, skills, evidence
- `docs/adf/model-selection-and-spawn.md` — routing engine deep-dive
- `crates/terraphim_orchestrator/src/lib.rs` — orchestrator core
- `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` — CLI source
- `scripts/adf-setup/agents/` — agent TOML templates
- `.docs/plan-adf-agents-replace-gitea-actions.md` — original implementation plan
