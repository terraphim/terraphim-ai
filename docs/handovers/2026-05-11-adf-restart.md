# Session Handover: 2026-05-11 ADF Fleet Restart

## Progress Summary

### Completed
1. **ADF Orchestrator restarted on bigbox** (systemd: `adf-orchestrator.service`)
   - 40 agents across 7 projects validated and running
   - Provider probes executing (Anthropic, Kimi, OpenAI, GLM)
   - KG routing active (3 rules, 82 synonyms)
   - Mention cursors loaded for all 7 projects
   - Quickwit logging enabled for 3 projects (terraphim-ai, odilo, digital-twins)
   - PR gate reconciliation running (auto-merge threshold 5/5)

2. **Odilo project bootstrapped on bigbox**
   - Cloned `zestic-ai/odilo` from Gitea to `/home/alex/projects/zestic-ai/odilo/`
   - Fixed `odilo-developer` cli_tool from `/home/alex/.local/bin/Terraphim AI` to `/home/alex/.local/bin/claude`
   - 2 agents configured: odilo-developer (sonnet, cron 1-9am), odilo-reviewer (opencode, k2p5 fallback)

3. **ADF binary rebuilt and deployed** (v1.8.0, `/usr/local/bin/adf`)

4. **Config schema fixed** -- conf.d include files must NOT have top-level `working_dir`/`nightwatch`/`compound_review`; those belong only in the main `orchestrator.toml` which uses `include = ["conf.d/*.toml"]`

5. **6 meta-coordinator/noise agents disabled** across 3 projects:
   - terraphim-ai: meta-coordinator, drift-detector, runtime-guardian, log-analyst, repo-steward (5 disabled)
   - gitea: gitea-meta-coordinator (1 disabled)
   - atomic-server: atomic-meta-coordinator (1 disabled)
   - digital-twins: meta-coordinator was already commented out

6. **Security fix**: `agent_tokens.json` permissions set to 600

### What's Working
- ADF orchestrator: active (running), PID 233256 on bigbox
- Provider routing: KG-based model selection with fallback chain
- Worktree isolation: agents create isolated git worktrees under `/tmp/adf-worktrees/`
- PR gates: confidence-based auto-merge (threshold 5/5)
- Mention dispatch: all 7 projects have loaded mention cursors

### Blocked / Watch Items
- Some provider probes failing (anthropic sonnet quota-exited, fallback to kimi)
- `reports/` directory has stale report files (untracked, not committed)
- `scripts/adf-setup/tests/__pycache__/` .pyc files were deleted but not committed
- Odilo project has 112 ready issues on Gitea but no active development yet (cron starts at 01:00 UTC)

## Technical Context

```bash
# Current branch
main

# Recent commits
d53356893 Merge remote syncing branch 'origin/main'
4bb4e5c4a fix(ci): add --no-fail-fast to cargo test --workspace Refs #1355
a83efc0da fix(symphony): enforce RetryBound invariant and add regression tests Refs #1389 #251
e3e0f843f fix(symphony): enforce RetryBound invariant and add regression tests Refs #1389 #251
6cb7b3f96 docs: add session handover for 2026-05-11

# Modified files (uncommitted)
deleted:    scripts/adf-setup/tests/__pycache__/*.pyc
untracked:  reports/doc-report-2026-05-09.md
untracked:  reports/spec-validation-20260509-*.md
```

## Fleet Inventory (40 agents, 7 projects)

### terraphim-ai (21 agents) -- Active Development
| Agent | Layer | Schedule | Model | Flow |
|-------|-------|----------|-------|------|
| implementation-swarm | Core | 45 0-10 * * * | k2p5 | Full V-model: research/design/implementation/verification/validation + rust-mastery |
| product-development | Core | 25 0-10 * * * | haiku | disciplined skills |
| product-owner | Core | 55 0-10 * * * | unset | disciplined skills |
| spec-validator | Core | 30 0-10 * * * | unset | disciplined skills |
| test-guardian | Core | 35 0-10 * * * | sonnet | disciplined skills |
| documentation-generator | Core | 40 0-10 * * * | sonnet | disciplined skills |
| roadmap-planner | Core | 0 2 * * * | sonnet | disciplined skills |
| security-sentinel | Core | 0 */6 * * * | k2p5 | security-audit |
| upstream-synchronizer | Core | 0 0 * * * | haiku | devops + git-safety-guard |
| meta-learning | Core | 0 11 * * * | sonnet | learning capture |
| compliance-watchdog | Core | 5 0-10 * * * | k2p5 | compliance |
| build-runner | Growth | event_only | unset | CI: fmt+clippy+test |
| pr-reviewer | Growth | event_only | sonnet | PR review |
| pr-spec-validator | Safety | event_only | sonnet | spec validation |
| pr-security-sentinel | Safety | event_only | sonnet | security audit |
| pr-compliance-watchdog | Growth | event_only | sonnet | compliance |
| pr-test-guardian | Growth | event_only | sonnet | test review |
| quality-coordinator | Growth | event_only | sonnet | quality gate |
| browser-qa | Growth | event_only | unset | visual testing |
| merge-coordinator | Growth | 0 */4 * * * | k2p5 | merge orchestration |

### odilo (2 agents) -- Active Development
| Agent | Layer | Schedule | Model |
|-------|-------|----------|-------|
| odilo-developer | Core | 0 1-9 * * * | sonnet (claude) |
| odilo-reviewer | Core | mention | k2p5 (opencode) |

### gitea (4 agents) -- Upstream Sync
| Agent | Layer | Schedule | Model |
|-------|-------|----------|-------|
| gitea-developer | Core | mention | sonnet |
| gitea-reviewer | Core | mention | haiku |
| gitea-build-runner | Growth | event_only | unset |
| gitea-upstream-synchronizer | Core | 45 1 * * * | haiku |

### atomic-server (4 agents) -- Upstream Sync
| Agent | Layer | Schedule | Model |
|-------|-------|----------|-------|
| atomic-developer | Core | mention | sonnet |
| atomic-reviewer | Core | mention | haiku |
| atomic-build-runner | Growth | event_only | unset |
| atomic-upstream-synchronizer | Core | 45 1 * * * | haiku |

### gitea-robot (3 agents) -- Active Development
| Agent | Layer | Schedule | Model |
|-------|-------|----------|-------|
| gitea-robot-developer | Core | mention | sonnet |
| gitea-robot-reviewer | Core | mention | haiku |
| gitea-robot-build-runner | Growth | event_only | unset |

### better-auth-rust (3 agents) -- Active Development
| Agent | Layer | Schedule | Model |
|-------|-------|----------|-------|
| better-auth-rust-developer | Core | mention | sonnet |
| better-auth-rust-reviewer | Core | mention | haiku |
| better-auth-rust-build-runner | Growth | event_only | unset |

### digital-twins (4 agents) -- Active Development
| Agent | Layer | Schedule | Model |
|-------|-------|----------|-------|
| developer | Core | mention | sonnet |
| reviewer | Core | mention | haiku |
| reviewer-2 | Core | mention | MiniMax-M2.5 |
| merge-coordinator | Growth | 0 */4 * * * | haiku |

## Key Files on Bigbox

```
/usr/local/bin/adf                              -- v1.8.0 orchestrator binary
/opt/ai-dark-factory/orchestrator.toml          -- main config (includes conf.d/*.toml)
/opt/ai-dark-factory/conf.d/<project>.toml      -- per-project agent definitions
/opt/ai-dark-factory/agent_tokens.json          -- per-agent Gitea API tokens (chmod 600)
/etc/systemd/system/adf-orchestrator.service    -- systemd unit
/home/alex/projects/zestic-ai/odilo/            -- odilo repo (cloned from Gitea)
/home/alex/projects/terraphim/gitea/            -- gitea fork
/home/alex/projects/terraphim/atomic-server/    -- atomic-server fork
/home/alex/projects/terraphim/better-auth-rust/ -- better-auth-rust
/home/alex/projects/terraphim/gitea-robot/      -- gitea-robot
/home/alex/projects/zestic-ai/digital-twins/    -- digital-twins
/data/projects/terraphim/terraphim-ai/          -- terraphim-ai (on bigbox)
```

## Next Steps

1. Monitor orchestrator health: `ssh bigbox sudo journalctl -u adf-orchestrator -f`
2. Check odilo-developer starts at 01:00 UTC (cron schedule)
3. Verify provider probe results stabilize (sonnet quota issues)
4. Review any PRs auto-created by agents
5. Clean up stale `reports/` and `__pycache__` if desired
