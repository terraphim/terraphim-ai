# Design & Implementation Plan: Re-enable test-guardian and Complete Phase 2

## 1. Summary of Target Behavior

After this plan is executed:
- PR #1053 (docs) is merged to main
- test-guardian agent is re-enabled in the ADF orchestrator
- All 6 PR-fan-out agents (`adf/build`, `adf/pr-reviewer`, `adf/spec`, `adf/security`, `adf/compliance`, `adf/test`) are operational
- Branch protection requires all 6 status checks
- 24-hour monitoring confirms no spawn-die cycles or excessive failures
- Issue #238 is closed as complete

## 2. Key Invariants and Acceptance Criteria

### Invariants
1. **No direct push to main**: All changes via PR with ADF checks
2. **Agent stability**: test-guardian must not enter spawn-die cycles
3. **Cost control**: Monthly budget for test-guardian must not be exceeded
4. **Disk safety**: bigbox disk usage must stay below 85%

### Acceptance Criteria (Testable)
| Criterion | How Verified |
|-----------|--------------|
| PR #1053 merged | Gitea shows merged status |
| test-guardian spawns successfully | journalctl shows "spawning agent: test-guardian" without immediate exit |
| test-guardian runs to completion | journalctl shows "exit classified" with success or known failure class |
| All 6 status checks appear on new PRs | Gitea PR UI shows all 6 required checks |
| 24h no spawn-die cycles | journalctl query shows no "exit classified" for test-guardian within 60s of spawn |
| Disk usage < 85% | `df -h` on bigbox shows target/ under 85% |

## 3. High-Level Design and Boundaries

### Components Involved
```
Gitea PR event
    |
    v
webhook.rs (existing)
    |
    v
pr_dispatch.rs (existing) -- now includes test-guardian
    |
    +--> build-runner (existing)
    +--> pr-reviewer (existing)
    +--> spec-validator (existing)
    +--> security-sentinel (existing)
    +--> compliance-watchdog (existing)
    +--> test-guardian (RE-ENABLED) <-- this plan
```

### Boundaries
- **Inside scope**: Config change (uncomment agent), service restart, monitoring
- **Outside scope**: Code changes to test-guardian logic (assumed working), PR #1045 merge (separate decision)

### Complected Areas
- The orchestrator.toml mixes config for multiple concerns (webhook, agents, projects). We must be careful not to disturb other agent configurations when re-enabling test-guardian.

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `/opt/ai-dark-factory/orchestrator.toml` (bigbox) | Modify | test-guardian `[[agents]]` block commented out | Block uncommented, `grace_period_secs = 300`, fallback set | SSH access to bigbox |
| PR #1053 | Merge | Open PR | Merged to main | ADF status checks pass |
| `journalctl` logs | Read | - | Verify spawn and exit patterns | systemctl access |

## 5. Step-by-Step Implementation Sequence

### Step 1: Merge PR #1053 (Docs)
**Purpose**: Land documentation before operational changes
**Deployable state**: Yes (docs-only, no runtime impact)
**Procedure**:
1. Verify PR #1053 status checks are green
2. Merge via Gitea UI or `gtr merge-pull`
3. Verify `git diff origin/main gitea/main --stat` is empty after sync

### Step 2: Pre-Enable Verification
**Purpose**: Confirm system is healthy before re-enabling test-guardian
**Deployable state**: N/A (read-only verification)
**Procedure**:
1. SSH to bigbox
2. Check disk: `df -h /opt/ai-dark-factory/target` < 85%
3. Check current agent health: `journalctl -u adf-orchestrator --since '1h ago' | grep 'exit classified' | grep -v success | wc -l` should be low
4. Review test-guardian history: `journalctl -u adf-orchestrator --since '7 days ago' | grep test-guardian | tail -20`

### Step 3: Re-enable test-guardian
**Purpose**: Restore disabled agent
**Deployable state**: Yes (single config change, reversible)
**Procedure**:
1. On bigbox, edit `/opt/ai-dark-factory/orchestrator.toml`
2. Uncomment the `[[agents]]` block for test-guardian
3. Verify: `grace_period_secs = 300`, `fallback_provider` and `fallback_model` are set
4. Run `adf --check /opt/ai-dark-factory/orchestrator.toml` to validate config

### Step 4: Restart Orchestrator
**Purpose**: Pick up config change
**Deployable state**: Yes (standard restart procedure)
**Procedure**:
1. `sudo systemctl restart adf-orchestrator`
2. Monitor: `journalctl -u adf-orchestrator -f`
3. Verify all agents load: look for "registered N agents" in log
4. Wait for next cron tick or trigger test-guardian manually via `adf-ctl`

### Step 5: Verify test-guardian Spawns
**Purpose**: Confirm agent is functional
**Deployable state**: N/A (verification)
**Procedure**:
1. If cron schedule is far, use: `adf-ctl trigger test-guardian --wait --timeout 1200`
2. Watch journal for spawn and exit
3. Verify exit classification is not `spawn_loop` or `rapid_exit`
4. Check Gitea status: should post `adf/test` status if run via PR

### Step 6: 24-Hour Observation
**Purpose**: Confirm stability before closing issue
**Deployable state**: N/A (observation)
**Procedure**:
1. Set timer for 24 hours
2. Query journal: `journalctl -u adf-orchestrator --since '24h ago' | grep test-guardian`
3. Count spawn-die cycles (spawns without completion within 60s)
4. Verify disk usage stable
5. Check coordination report if available

### Step 7: Close Issue #238
**Purpose**: Mark work complete
**Deployable state**: N/A
**Procedure**:
1. Verify all acceptance criteria met
2. Comment on issue #238 with summary
3. Close issue

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| PR #1053 merged | Operational | Gitea PR UI |
| Config validation passes | Unit | `adf --check` CLI |
| test-guardian spawns | Integration | journalctl + adf-ctl trigger |
| test-guardian completes | Integration | journalctl exit classification |
| No spawn-die cycles | Observational | 24h journalctl query |
| Disk < 85% | Operational | `df -h` on bigbox |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| test-guardian still broken | Monitor closely; disable immediately if spawn-die | Low (prerequisites verified) |
| Disk fills during 24h | Check before; cleanup scripts should run | Low |
| Other agents affected by restart | Graceful restart; agents resume schedules | Low |
| PR #1053 check failures | Docs-only; should not fail | Very Low |

## 8. Open Questions / Decisions for Human Review

1. **Should PR #1045 be merged first?** It fixes memory exhaustion (#664). While not strictly blocking, it may improve test-guardian stability.
2. **Do we wait for natural cron tick or trigger manually?** Manual trigger gives faster feedback but may not test the full cron path.
3. **What is the rollback procedure if test-guardian still fails?** Re-comment the `[[agents]]` block and restart.

---

## Decision Requested

**Do you approve this plan as-is, or would you like to adjust any part?**

Specifically:
- Should PR #1045 be merged before re-enabling test-guardian?
- Should we trigger test-guardian manually or wait for natural cron tick?
