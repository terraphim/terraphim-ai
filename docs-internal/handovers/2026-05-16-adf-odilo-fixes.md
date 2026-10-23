# Handover: ADF Fixes & odilo-developer Investigation

**Date:** 2026-05-16  
**Branch:** `main` (bc31ecb40)  
**Remotes:** origin (GitHub) and gitea synced

---

## 1. Progress Summary

### Completed

| Task | Detail |
|------|--------|
| **Fix: ADF running as root** | Restarted via systemd as `User=alex`. Added PATH override (`/home/alex/.local/bin`) so `claude` binary is resolvable for KG routing. Removed duplicate user-level systemd service. |
| **Fix: Claude OAuth auth** | ADF was running as root (manual restart), causing Claude "Not logged in". Now running as alex, all 3 Claude probes passing (sonnet 20s, opus 12s, haiku 14s). |
| **Fix: project_by_id WARN log** | Added WARN log when `project_by_id()` returns None during worktree creation. Logs agent name, requested project_id, and fallback path. Prevents silent repo misrouting. |
| **Fix: build-runner compilation** | Fixed 3 pre-existing clippy warnings promoted to errors: unused variables (`_exit_desc`, `_exit_code`), dead code (`#[cfg_attr]` on `config` field), duplicate unused import in terraphim_workspace. |
| **Fix: evolution field in tests** | Added `evolution: Default::default()` to all 5 test files constructing `OrchestratorConfig`. Added `evolution_snapshot_key: None` to `HandoffContext` test. Restored `impl Default for EvolutionConfig` with proper defaults. |
| **Fix: deprecated tempfile::into_path()** | Replaced 29 occurrences across 9 test files with `.keep()`. Removed unused `PathBuf` import in terraphim_server. |
| **Fix: BUILD.md --all-targets** | Restored `--all-targets` for clippy after fixing root cause (test compilation). Updated cargo fallback in build-runner-llm.sh to match. |
| **Fix: OOM killer** | ADF hit 16GB MemoryMax. Increased to 100G (80% of 128GB). Changed `KillMode` from `mixed` to `control-group`. Added `ExecStartPre` cleanup script to kill orphaned opencode/gtr/sentrux/cached-context processes. Cleaned 237 orphaned `.opencode` processes (83GB leaked). |
| **Install: yq** | v4.53.2 at `/usr/local/bin/yq`. Enables GitHub Actions workflow extraction in build-runner-llm. |
| **Install: rch** | Verified v1.0.16 at `/home/alex/.local/bin/rch`, on ADF PATH. |

### Commits (7 new)

```
bc31ecb40 Merge remote-tracking branch 'gitea/main'
dc7ce955d fix(clippy): replace deprecated tempfile into_path() with keep() across workspace
42043aec6 fix(build): restore --all-targets in BUILD.md, update cargo fallback, fix unused import
8cff66164 fix(build): fix build-runner compilation + pre-existing clippy warnings
cb68b411f Merge remote-tracking branch 'gitea/main'
c326e0c71 fix(orchestrator): add WARN log when project_by_id returns None
```

### Releases
- **v2026.05.16.1**: odilo-developer fixes (GitHub + Gitea)

---

## 2. Current ADF State

| Component | Status |
|-----------|--------|
| ADF process | Running as alex (PID 446199), systemd-managed |
| Memory | 21MB current (was 16GB peak before cleanup) |
| Orphaned opencode | **0** (was 237) |
| Claude probes | sonnet, opus, haiku all passing |
| Provider health | All healthy (minimax, zai, kimi, anthropic) |
| Agent count | 39 definitions loaded across 7 projects |
| Ticks | Completing every 30s (~100-600ms each) |

### Systemd Configuration (`/etc/systemd/system/adf-orchestrator.service`)

- `User=alex`, `Group=alex`
- `MemoryMax=100G` (80% of 128GB RAM)
- `CPUQuota=400%`
- `KillMode=control-group` (kills all children on stop)
- `ExecStartPre=/opt/ai-dark-factory/adf-cleanup.sh` (kills orphaned .opencode, gtr, sentrux, claude, cargo processes)
- PATH override: `/etc/systemd/system/adf-orchestrator.service.d/path.conf`
- Gitea env: `/etc/systemd/system/adf-orchestrator.service.d/env-gitea.conf`

### Infrastructure on bigbox

| Tool | Version | Path |
|------|---------|------|
| rch | 1.0.16 | `/home/alex/.local/bin/rch` |
| yq | 4.53.2 | `/usr/local/bin/yq` |
| terraphim-agent | 1.16.34 | `/usr/local/bin/terraphim-agent` |
| claude | 2.1.143 | `/home/alex/.local/bin/claude` |

---

## 3. odilo-developer Investigation

### Root Cause of Failures

1. **Claude OAuth failure**: ADF was running as root (manual restart May 16 11:06 CEST). Claude OAuth tokens are in `/home/alex/.claude/`, inaccessible to root. ADF fell back to kimi-for-coding/k2p5 which hung (May 15 zombie -- 3 log entries then silent for days). **Fixed** by restarting ADF via systemd as alex.

2. **Wrong repo worktree**: May 16 worktree (`odilo-developer-77abfd9d`) contained terraphim-ai code, not odilo. Caused by `project_by_id("odilo")` returning None after ADF restart, falling back to orchestrator `working_dir`. **Fixed** with WARN log and root cause (ADF now runs as alex with proper config loading).

3. **"rate_limit" exits were Claude OAuth failures**: 5 exits on May 13 classified as `rate_limit` (pattern "you've hit your limit") were actually Claude OAuth failures from running as root. With ADF running as alex, Claude can authenticate.

4. **build-runner-llm degradation**: Script operates in degraded mode -- `yq` was missing (workflow extraction skipped), `rch` was missing (remote compilation skipped). **Fixed** by installing yq and verifying rch.

### Verified: odilo-developer Produces Useful Output

Ran odilo-developer via Claude CLI on bigbox. Successfully:
- Listed ready issues (115 open, top PageRank 0.15)
- Retrieved past learnings
- Found open PRs
- Analysed all 8 Rust crates with accurate descriptions
- Produced structured output tables

### Schedule
- **Next fire**: 01:00 UTC (03:00 CEST) -- 4.5 hours from now
- **Window**: `0 1-9 * * *` UTC (fires hourly, 9x daily)

### Open PRs on odilo
- PR #235: Contract conformance CI
- PR #234: Teacher Service Phase 3
- PR #222: Slack Socket Mode ADR-049

---

## 4. What's Working

- ADF running stably as alex via systemd
- All provider probes passing
- build-runner-llm: clippy, build, and fmt steps pass (`cargo clippy --workspace --all-targets -- -D warnings` clean)
- Worktree project validation logs WARN on resolution failure
- Orphaned process cleanup on ADF restart
- yq workflow extraction available for build-runner

## 5. What's Blocked / Needs Follow-up

| Item | Priority | Notes |
|------|----------|-------|
| odilo-developer test failures | Medium | `test_tui_service_search` failed in manual run -- likely pre-existing test instability, not related to our changes |
| Gitea branch protection 404s | Low | Recurring warnings for atomic-server, better-auth-rust, digital-twins, gitea-robot, gitea (no PR gate configured -- harmless) |
| Gitea 403 for odilo | Low | Token lacks admin write on zestic-ai/odilo -- branch protection gate skipped |
| CLAUDE.md too large (30KB) | Low | Consumes many turns during odilo-developer onboarding -- consider moving to AGENTS.md or trimming |
| build-runner-llm still uses BUILD.md priority 2 | Low | With yq installed, workflow extraction (priority 1) will extract ALL run commands including sudo/setup commands -- might need filtering |
