# Runbook: ADF Memory Watchdog Install

**Created**: 2026-05-23
**Refs**: #1817 step (b)
**Files**: `systemd/adf-orchestrator.service.d/memory.conf`, `systemd/adf-orchestrator-restart.service`

## Why

Pre-2026-05-23 the orchestrator reached **84.3 G of its 90 G `MemoryHigh` ceiling** after long uptime + leaked agent tasks (1086 tasks counted). One bad spawn from OOM-kill of the whole service. Restart cleanup brought it to 3.1 G but the underlying risk recurs as soon as uptime accumulates.

## What it does

1. **`memory.conf` drop-in** -- lowers `MemoryHigh` from 90 G to 80 G so the kernel applies back-pressure earlier; gives the operator 10 G of headroom before the harder `MemoryMax` (115 G) limit triggers actual OOM-kill of agents.
2. **`adf-orchestrator-restart.service` oneshot** -- triggered by `OnFailure=` on the main service. Sleeps 60 s (lets pressure subside) then `systemctl restart adf-orchestrator.service`.

Net: if the orchestrator misbehaves (OOM, panic, deadlock), systemd automatically restarts it after 60 s, surfacing the failure in the journal without operator intervention.

## Install (on bigbox)

```bash
cd /data/projects/terraphim/terraphim-ai-fresh

# 1. Drop-in directory
sudo mkdir -p /etc/systemd/system/adf-orchestrator.service.d

# 2. Memory ceiling
sudo install -m 644 systemd/adf-orchestrator.service.d/memory.conf \
    /etc/systemd/system/adf-orchestrator.service.d/memory.conf

# 3. Restart oneshot
sudo install -m 644 systemd/adf-orchestrator-restart.service \
    /etc/systemd/system/adf-orchestrator-restart.service

# 4. Wire OnFailure into main unit (one-time edit; persists across deploys)
sudo systemctl edit adf-orchestrator.service
# In the editor, add to [Unit] section:
#   OnFailure=adf-orchestrator-restart.service
# Save and exit.

# 5. Reload + restart
sudo systemctl daemon-reload
sudo systemctl restart adf-orchestrator
```

## Verify

```bash
# MemoryHigh should now be 80 G
systemctl show adf-orchestrator | grep -i memory
# Expected: MemoryHigh=85899345920  (80 GiB = 80 * 2^30)

# Drop-in is recognised
systemctl status adf-orchestrator | head -10
# Should mention /etc/systemd/system/adf-orchestrator.service.d/memory.conf

# OnFailure is wired
systemctl show adf-orchestrator -p OnFailure
# Expected: OnFailure=adf-orchestrator-restart.service

# Restart unit exists
systemctl status adf-orchestrator-restart.service
# Expected: Loaded; inactive (dead) -- it is a oneshot, only fires on parent failure
```

## Test failure path (optional, do NOT run in business hours)

```bash
# Manually send the orchestrator a signal it cannot handle gracefully
sudo systemctl kill --signal=SIGSEGV adf-orchestrator
# OR
sudo kill -SEGV $(pgrep -f /usr/local/bin/adf)

# Watch journal -- expect:
#   1. Service fails
#   2. OnFailure triggers adf-orchestrator-restart.service
#   3. After 60 s, restart fires
#   4. Service is_active again
sudo journalctl -u adf-orchestrator -u adf-orchestrator-restart --since "2 minutes ago"
```

## Rollback

```bash
sudo rm /etc/systemd/system/adf-orchestrator.service.d/memory.conf
sudo rm /etc/systemd/system/adf-orchestrator-restart.service
# Edit back the parent unit to remove OnFailure line
sudo systemctl edit adf-orchestrator.service
sudo systemctl daemon-reload
sudo systemctl restart adf-orchestrator
```

## Related

- `.docs/runbook-bigbox-sync-2026-05-23.md` -- deploy script that should run BEFORE this install (the script restarts the orchestrator at the end; install the watchdog first so the restart picks it up).
- #1817 -- tracking issue
- #1818 -- bigbox repo corruption (separate concern, not addressed by this watchdog)
