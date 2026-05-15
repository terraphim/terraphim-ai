# Handover: ADF Orchestrator Investigation & Fix

**Date:** 2026-05-15
**Session:** ADF Orchestrator Investigation

## Progress Summary

### Tasks Completed

1. **Investigated ADF orchestrator crash loop on bigbox**
   - Root cause: `GITEA_TOKEN` environment variable not available to systemd service
   - Token stored in `~/.config/terraphim/gitea-env` but not sourced by systemd

2. **Fixed systemd environment for ADF orchestrator**
   - Created drop-in at `/etc/systemd/system/adf-orchestrator.service.d/env-gitea.conf`
   - Added `GITEA_URL` and `GITEA_TOKEN` as environment variables
   - Reloaded systemd and restarted service
   - ADF orchestrator now running successfully with 39 agents loaded

3. **Fixed stale path references in ADF configuration files**
   - Updated 10 files with old path `/home/alex/terraphim-ai` to canonical path `/home/alex/projects/terraphim/terraphim-ai`
   - Committed and pushed to both origin (GitHub) and gitea remotes
   - Also fixed live config on bigbox at `/opt/ai-dark-factory/conf.d/terraphim.toml`

4. **Validated odilo-developer agent execution**
   - Triggered via webhook with correct `action: "created"` payload
   - Agent spawned successfully with skills: disciplined-research, disciplined-design, disciplined-implementation, quality-gate
   - Worktree created at `/tmp/adf-worktrees/odilo-developer-9af65ac8`
   - Agent actively running with model kimi-for-coding/k2p5

### Current Implementation State

- **ADF orchestrator:** Running on bigbox as systemd service
- **Configuration:** `/opt/ai-dark-factory/orchestrator.toml` with agents in `/opt/ai-dark-factory/conf.d/`
- **Webhook endpoint:** `http://172.18.0.1:9091/webhooks/gitea`
- **Agents loaded:** 39 agents across multiple projects (terraphim-ai, odilo, digital-twins, gitea, etc.)

### What's Working

- ADF orchestrator service starting and staying running
- Workflow tracker connecting to Gitea with token from systemd environment
- Agent dispatch via webhooks (verified with odilo-developer)
- Knowledge graph routing and model fallback
- Provider probing on startup

### What's Blocked

- **Branch protection API errors:** Several projects return 403/404 for branch protection (odilo, atomic-server, better-auth-rust, digital-twins, gitea-robot, gitea)
- **odilo project:** No build-runner or pr-reviewer configured - Push and PR events are skipped
- **Provider health:** anthropic and openai providers unhealthy (using kimi fallback)

## Technical Context

```bash
# Current branch
main

# Recent commits
4442671e9 Merge remote-tracking branch 'origin/main'
c018f3623 fix(adf-setup): update stale /home/alex/terraphim-ai paths to canonical path
2d41798cd fix(agent): populate concepts_matched in robot-mode search envelope (#1486)
e8febcbbf feat(terraphim_rlm): implement DockerExecutor for container-based isolation (#1485)
669bebadd feat(terraphim_rlm): implement DockerExecutor for container-based isolation

# Modified files
Untracked: docs/handovers/2026-05-14-rlm-opencode-handover.md
```

## Key Files Modified

| File | Change |
|------|--------|
| `/etc/systemd/system/adf-orchestrator.service.d/env-gitea.conf` | Added GITEA_URL and GITEA_TOKEN env vars |
| `scripts/adf-setup/agents/*.toml` | Updated stale paths |
| `scripts/adf-setup/tests/fixtures/*.toml` | Updated stale paths |
| `scripts/adf-setup/tests/expected/*.toml` | Updated stale paths |
| `crates/terraphim_orchestrator/tests/fixtures/conf.d/terraphim.toml` | Updated stale paths |

## Key Lessons

1. **Systemd environment:** When running a service that needs user environment variables, either:
   - Use systemd Environment= directives (what we did)
   - Source the file in ExecStartPre
   - Don't rely on ~/.profile being sourced

2. **Webhook action field:** Gitea webhooks send `action: "created"` for new comments, not `action: "comment"`

3. **Process tracking:** Agent processes spawn under the ADF parent but may exit/change quickly; use working directory and log files to verify

## Next Steps

1. Configure build-runner and pr-reviewer for odilo project
2. Investigate branch protection API permissions (403/404 errors)
3. Add webhook secret to enable HMAC signature verification
4. Monitor odilo-developer agent completion and review output
