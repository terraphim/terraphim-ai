# ADF Operations Guide

Day-to-day operations for the AI Dark Factory orchestrator on bigbox.
Covers the `adf-ctl` CLI, service management, monitoring, and known
operational procedures.

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
`/Users/alex/projects/terraphim/terraphim-ai/target/debug/adf-ctl`.
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
will NOT be found — `resolve_mention` searches only the terraphim project's
agent list.

**Workaround for odilo agents:**
```bash
# On bigbox — sign and POST with the correct repo
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

**Workaround for mention-triggered agents in any project:**
Use `gtr comment` to post a real `@adf:<agent>` mention in the agent's
native repo. The mention poll will pick it up within 10 min.

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

This was added on 2026-04-27. The proper fix (issue #951, branch
`task/860-f1-2-exit-codes`) makes `handle_webhook_dispatch` fall back to
project-level mentions config — but the fix has not been merged to main or
deployed.

---

## Persona loading

Personas are loaded **once at startup** from `persona_data_dir`
(`/home/alex/terraphim-ai/data/personas/`). If a persona file is added or
modified after the orchestrator starts, it will not be picked up until restart.

Available personas (as of 2026-04-27): Carthos, Conduit, Echo, Ferrox, Lux,
Meridian, Mneme, Themis, Vigil — 9 total.

Each persona adds ~2KB of metaprompt context and shapes the agent's
communication style and decision-making framework.

---

## Monitoring: key journal patterns

```bash
# All spawns
grep 'spawning agent'

# All exits with class
grep 'exit classified'

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
rm -rf /tmp/adf-worktrees/sentinel-*/  # old-format orphans if any remain
```

A permanent fix is tracked in a Gitea issue — the orchestrator should prune
orphaned worktrees on startup.

---

## Provider probe failures

The orchestrator probes all providers in the KG routing table at startup and
periodically. Providers that time out (60s) trip their circuit breakers and
are skipped in routing until the next probe window.

Current expected probe failures (openai subscription not yet active as of
2026-04-27; activates 2026-04-28):
- `openai/gpt-5.3-codex`
- `openai/gpt-5.4`
- `openai/gpt-5.4-mini`
- `minimax-coding-plan/MiniMax-M2.5`

These are in the KG routing tables under
`docs/taxonomy/routing_scenarios/adf/`. Do NOT remove them — the OpenAI
subscription and MiniMax access will restore these providers automatically.

---

## Timeout configuration

| Agent | max_cpu_seconds | Notes |
|---|---|---|
| security-sentinel | 1200 | |
| meta-coordinator | 1200 | bumped from 300 on 2026-04-27 |
| runtime-guardian | 1200 | |
| compliance-watchdog | 7200 | |
| drift-detector | 7200 | |
| spec-validator | 7200 | can run 50+ min on large backlogs |
| test-guardian | 7200 | 119 min observed for full test suite |
| odilo-developer | 7200 | |
| developer/implementation-swarm | 7200 | |

Timeout is enforced by `poll_wall_timeouts` which runs each reconciliation
tick. When exceeded, the agent process is killed and a fallback respawn is
attempted if a `fallback_provider` is configured.

---

## Config file locations on bigbox

| File | Purpose |
|---|---|
| `/opt/ai-dark-factory/orchestrator.toml` | Top-level config (persona dir, skill dir, tick interval, [mentions]) |
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | 22 terraphim agents + project mentions config |
| `/opt/ai-dark-factory/conf.d/odilo.toml` | 2 odilo agents |
| `/opt/ai-dark-factory/conf.d/digital-twins.toml` | 2 digital-twins agents |
| `/opt/ai-dark-factory/agent_tokens.json` | Per-agent Gitea tokens for attribution |
| `/opt/ai-dark-factory/persona_roles_config.json` | terraphim-agent KG role config for in-task searches |
| `/opt/ai-dark-factory/skills/` | Skill SKILL.md files injected into prompts |
| `/opt/ai-dark-factory/scenarios/` | Scenario files for browser-qa |
| `/opt/ai-dark-factory/reports/` | Agent-written reports |
| `/home/alex/terraphim-ai/data/personas/` | Persona TOML files (loaded at startup) |
| `/home/alex/terraphim-ai/docs/taxonomy/routing_scenarios/adf/` | KG routing tables |

---

## Validate config before restart

```bash
sudo /usr/local/bin/adf --check /opt/ai-dark-factory/orchestrator.toml
```

Prints the full routing table per agent. Exits non-zero on TOML parse errors.

---

## Further reading

- `docs/adf/agent-fleet.md` — agent roster with roles, personas, skills, evidence
- `docs/adf/model-selection-and-spawn.md` — routing engine deep-dive
- `crates/terraphim_orchestrator/src/lib.rs` — orchestrator core
- `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` — CLI source
