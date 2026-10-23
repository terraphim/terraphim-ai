# Runbook: Fix Meta-Coordinator Health Check (blind under `no_new_privs`)

**Issue**: `terraphim/terraphim-ai#3005` (tracker id `4300`)
**Theme-ID**: `adf-health-alert`
**Severity**: P1 — a monitor that reports false-green while the system is red is worse
than no monitor; it suppresses human attention.
**Status**: Patch verified end-to-end on bigbox; awaiting operator apply (root required).

## Symptom

The ADF meta-coordinator health-check cron reports **0 stalls / 0 failures / 0 timeouts**
in windows where the orchestrator is actually degraded. On 2026-06-29 it reported 0 stalls
in the same 4h window where `#3004` documents **207 stalls, max 55s**.

## Root cause — verified by reproduction

The health-check task block invokes `sudo journalctl -u adf-orchestrator`. On this host
the `alex` user (and the orchestrator runtime) runs under the **`no_new_privs`** flag, so
`sudo` is refused:

```
$ sudo -n true
sudo: The "no new privileges" flag is set, which prevents sudo from running as root.
```

Every `sudo journalctl ... | grep -c ... || true` therefore evaluates to `0`, and each
guard treats `0` as healthy. Reproduced this session (2026-06-29 22:10 +02:00):

| Invocation | Lines in 4h window | `reconcile_tick SLOW` count |
|------------|-------------------:|----------------------------:|
| `sudo journalctl ...` (current/buggy) | **0** | **0** ← false green |
| `journalctl -q ...` (fixed) | 19,079 | **164** ← true signal |

The `alex` user already has journal read access via ACL on `/var/log/journal`
(group `systemd-journal`). **`sudo` is not only unnecessary here — it is actively harmful.**

## Affected file

The defect lives in the **deployed, unversioned** runtime config (NOT in this repo):

```
/opt/ai-dark-factory/conf.d/terraphim.toml   (root:alex 0640)
```

Occurrences (as of 2026-06-23 mtime, still present 2026-06-29):

| Line | Block | Pattern |
|------|-------|---------|
| 68 | `### 1. Tick-Stall Detection` | `sudo journalctl -u adf-orchestrator ...` |
| 76 | `### 2. Agent Failure Report` | `sudo journalctl -u adf-orchestrator ...` |
| 86 | `### 2b. Agents Exceeding max_cpu_seconds` | `sudo journalctl -u adf-orchestrator ...` |
| 832 | Mneme fleet-health synthesis | `sudo -n journalctl -u adf-orchestrator ...` |

> **Note**: the meta-coordinator agent commented on `#3005` at 10:05 claiming the fix was
> "applied". That was **inaccurate** — the agent ran one cycle manually without `sudo` but
> never edited the config. File mtime `2026-06-23 16:44` confirms the file is untouched.
> Verify any future "fixed" claim with `stat -c '%y' /opt/ai-dark-factory/conf.d/terraphim.toml`.

## The patch (verified)

Two surgical changes:

### Change 1 — remove `sudo` from all `journalctl` invocations

```bash
sudo cp /opt/ai-dark-factory/conf.d/terraphim.toml \
        /opt/ai-dark-factory/conf.d/terraphim.toml.bak-3005-$(date +%Y%m%d-%H%M%S)
sudo sed -i \
  -e 's/sudo -n journalctl/journalctl -q/g' \
  -e 's/sudo journalctl/journalctl -q/g' \
  /opt/ai-dark-factory/conf.d/terraphim.toml
```

`-q` suppresses journalctl's "not seeing messages from other users" hint cleanly
(recommended in the issue's confirmation comment). No privilege escalation needed.

### Change 2 — add a fail-loud telemetry precondition

Insert immediately after the `export PATH=...` line (currently line 64), before
`## ADF Meta-Coordinator: Monitoring and Health Checks`, so the script aborts loudly
instead of emitting a false-green if telemetry ever breaks again:

```bash
# Telemetry precondition: fail loud rather than report false-green.
_TELEMETRY_LINES=$(journalctl -q -u adf-orchestrator --since '4 hours ago' 2>/dev/null | wc -l | tr -d ' ')
if [ "${_TELEMETRY_LINES:-0}" -lt 50 ]; then
  echo "FATAL: journalctl returned ${_TELEMETRY_LINES} lines in 4h — telemetry broken, aborting (would produce false green)" >&2
  gtr create-issue --owner terraphim --repo terraphim-ai \
    --title "[ADF] Health check telemetry broken (${_TELEMETRY_LINES} lines in 4h)" \
    --body "journalctl returned ${_TELEMETRY_LINES} lines in a 4h window. The health check aborted to avoid a false-green report. Investigate journal access / ACL on /var/log/journal.
Theme-ID: adf-health-alert" 2>/dev/null || true
  exit 1
fi
```

## Apply procedure (operator with root)

```bash
# 1. Back up + patch (Change 1)
sudo cp /opt/ai-dark-factory/conf.d/terraphim.toml \
        /opt/ai-dark-factory/conf.d/terraphim.toml.bak-3005-$(date +%Y%m%d-%H%M%S)
sudo sed -i -e 's/sudo -n journalctl/journalctl -q/g' -e 's/sudo journalctl/journalctl -q/g' \
  /opt/ai-dark-factory/conf.d/terraphim.toml

# 2. Verify the patch took (expect 0 matches)
grep -c 'sudo .*journalctl' /opt/ai-dark-factory/conf.d/terraphim.toml   # must print 0

# 3. Apply Change 2 (fail-loud precondition) — operator edits the task block,
#    or re-deploys from a versioned template once #see-followup is implemented.

# 4. Reload the orchestrator so it re-reads conf.d
sudo systemctl restart adf-orchestrator.service

# 5. Smoke-test the next health check cycle manually (no sudo):
journalctl -q -u adf-orchestrator --since '4 hours ago' | grep -c 'reconcile_tick SLOW'
```

## Verification (acceptance)

After the next 4h cron cycle, the health check MUST surface the real signal:

- [ ] `stat -c '%y' /opt/ai-dark-factory/conf.d/terraphim.toml` shows a fresh mtime.
- [ ] `grep -c 'sudo .*journalctl' /opt/ai-dark-factory/conf.d/terraphim.toml` returns `0`.
- [ ] The next cycle's `[ADF] Tick-stall detected: N in 4h` issue (if any) reports a
      non-zero `N` consistent with `#3004`-class telemetry (order of 10², not 0).
- [ ] No new spurious all-green cycle appears while `#3004`/`#4299` remain open.

## Deeper defect (filed separately)

The deployed `/opt/ai-dark-factory/conf.d/*.toml` is **unversioned** — not in any git
repo. That is the root cause that allowed this defect to persist silently for 6 days and
that let the meta-coordinator's "applied" comment go unchecked. The version-controlled
twins in `terraphim-agents/.../tests/fixtures/conf.d/` and `orchestrator.example.toml`
diverge from production and do **not** contain this task block at all. Bringing
`conf.d/` under version control (or generating it from a tracked template at deploy
time) is the structural fix that prevents recurrence. Tracked as a follow-up issue.

## Why this runbook lives in `terraphim-ai` (not `terraphim-agents`)

The health check creates issues in **this** repo (`terraphim/terraphim-ai`) and references
this repo's worktrees and binaries. Per AGENTS.md dual-remote protocol, this runbook is
pushed to both `origin` (GitHub) and `gitea` mirrors, giving operators a reviewable,
reproducible artefact that satisfies the spirit of Bigbox Rule #1 (git pull/push, not
ad-hoc `scp`) even though the target file itself is not yet version-controlled.
