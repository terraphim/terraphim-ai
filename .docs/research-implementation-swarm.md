# Research Document: Implementation Swarm Reliability and End-to-End Delivery

## 1. Problem Restatement and Scope

The ADF implementation-swarm-A and implementation-swarm-B agents are the primary coding workforce for the terraphim-ai project. They are cron-scheduled to run hourly and are expected to pick up ready Gitea issues, implement fixes, create branches, and open pull requests. However, the swarm has exhibited multiple failure modes:

- **OOM-induced SIGKILL**: Agent processes were being killed by the kernel OOM killer after ~30-56 minutes, causing the 2-hour `max_cpu_seconds` timeout to never be reached.
- **Stdin delivery hang**: When the orchestrator delivered large task prompts (>50KB) to opencode via stdin, the process would hang indefinitely (never completing within 2 hours).
- **Provider probe failures**: Claude CLI (anthropic) probes are currently failing with `exit status: 1`, indicating rate-limit exhaustion.
- **Incomplete delivery**: Even when agents complete (exit code 0), branches are created but pull requests are not consistently opened.

**IN scope:** Understanding why swarms fail to complete their end-to-end workflow (issue -> branch -> PR), the interaction between OOM, stdin delivery, and provider routing, and identifying the gaps in observability.

**OUT of scope:** Redesigning the entire ADF architecture, adding new agent tiers, or changing the issue tracking system.

## 2. User & Business Outcomes

**Visible outcomes if fixed:**
- Hourly swarm runs produce actual code changes as pull requests on Gitea/GitHub.
- No more "zombie" agent processes hanging for hours and consuming memory.
- Clear observability into what each swarm run accomplished (or why it failed).
- Reduction in manual intervention to close stale/no-op agent runs.

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| `terraphim_orchestrator` | `crates/terraphim_orchestrator/src/lib.rs` | Cron scheduling, provider routing, agent lifecycle | `terraphim_spawner`, `terraphim_agent_evolution`, `terraphim_tracker` |
| `terraphim_spawner` | `crates/terraphim_spawner/src/lib.rs` | Process spawning, stdin vs arg delivery, OOM protection | `tokio::process::Command`, `nix` (rlimit), `/proc` (oom_score_adj) |
| `AgentConfig` | `crates/terraphim_spawner/src/config.rs` | Per-CLI tool configuration | Inferred from provider definitions |
| `implementation-swarm-A/B` | `/opt/ai-dark-factory/conf.d/terraphim.toml` | Agent definitions (cron, task, CLI, model, fallback) | `claude` (primary), `opencode` (fallback) |
| `provider_probe` | `crates/terraphim_orchestrator/src/provider_probe.rs` | Pre-spawn health check for model availability | Runs target CLI with `echo hello` |
| `KG tier router` | `crates/terraphim_orchestrator/src/kg_router.rs` | Routes agent to model based on concept/skill | `terraphim_automata` Aho-Corasick |
| `Worktree manager` | `crates/terraphim_orchestrator/src/scope.rs` | Creates isolated git worktrees for agents | `git worktree add` |
| `agent_run_record` | `crates/terraphim_orchestrator/src/agent_run_record.rs` | Exit classification, pattern matching | `regex` patterns for success/failure |

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|---------------|-------------|
| **Claude CLI rate limits** | Anthropic imposes per-session token/turn limits; probes fail with exit status 1 when rate limited | KG router falls back to opencode; opencode lacks Claude's reliability for large tasks |
| **opencode stdin bug** | opencode `run` with stdin hangs for >50KB tasks; positional args work reliably in ~12s | Must never use stdin for opencode; `supports_stdin=false` is required |
| **ARG_MAX = 2MB** | Linux kernel limit on command-line argument length | Positional args are safe for tasks up to ~1.5MB (well above current ~63KB tasks) |
| **systemd AmbientCapabilities** | `oom_score_adj=-1000` requires `CAP_SYS_RESOURCE` | Service unit must include `AmbientCapabilities=CAP_SYS_RESOURCE`; verified active on bigbox |
| **Dual-remote sync** | Origin (GitHub) and gitea (Gitea) must stay converged | Swarms pushing only to gitea create divergence; PRs must reference both remotes |
| **Wall-clock timeout = 7200s** | `max_cpu_seconds` is actually wall-clock in the orchestrator | Agents that hang (stdin, infinite loops) are killed after 2 hours; successful runs complete in ~20 min |
| **Provider probe interval** | Probes run every tick (30s) for every model | Constant probing of rate-limited Claude burns API budget and produces log noise |

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Claude rate limit permanently blocking anthropic provider | **High** | Fallback to opencode works but is less capable; need to detect rate-limit vs actual failure |
| Swarm creates branch but not PR | **Medium** | Agent task script includes PR creation step; may fail silently if `gtr` returns error |
| Branch pushed only to gitea, not origin | **Medium** | Violates dual-remote sync protocol; PRs on Gitea may not trigger GitHub CI |
| `supports_stdin` fix not applied to other opencode agents | **Low** | Only implementation swarms have `cli_tool = opencode` in terraphim.toml; other projects use opencode too |
| OOM protection lost on systemd unit edit | **Low** | `AmbientCapabilities` line in service unit must not be removed during future edits |

### Unknowns

1. **What did swarm-B do during its 19:10 UTC run?** It created branch `gitea/task/1719-redact-env-vars-debug` at 19:29 UTC and exited at 19:30 UTC. Did it attempt to create a PR? Why was the PR not found?
2. **Does the agent task script handle `gtr create-pull` failures gracefully?** If `gtr` fails (e.g., branch already exists, auth issue), does the agent retry or silently skip?
3. **Are the provider probes distinguishing between rate-limit (retryable) and permanent failure?** Currently all exit status 1 is treated the same, causing constant re-probing.

### Assumptions

- **ASSUMPTION**: The `supports_stdin` fix works for all opencode invocations because the field is set at `AgentConfig` creation time via `infer_supports_stdin()`.
- **ASSUMPTION**: Claude rate limit is a temporary state that clears at 2am CEST; it is not a permanent ban.
- **ASSUMPTION**: The swarm task script's `git push -u origin task/...` step is failing because the branch name may already exist or because `origin` is not configured in the worktree.

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Dual-fallback routing**: KG router -> primary provider -> fallback provider -> fallback model. Each layer adds latency and failure modes.
2. **Mixed CLI semantics**: `claude -p`, `opencode run`, `codex exec` all have different arg formats and stdin behaviour. The spawner must know each tool's quirks.
3. **Git worktree + dual-remote**: Agents work in isolated worktrees but must push to both remotes and create PRs. This creates a complex state machine.
4. **Large agent tasks**: Each swarm receives a 63KB prompt containing the full skill chain, session checkpoint, and task instructions. This is larger than the old 32KB stdin threshold.

### Simplification Opportunities

1. **Single CLI abstraction layer**: Instead of the spawner knowing every CLI tool's quirks, add a thin wrapper script (`/opt/ai-dark-factory/bin/agent-run`) that normalises the interface (always takes a task file path, never stdin).
2. **Push-to-both-remotes helper**: Add a `git push-both` alias or script that pushes to origin then gitea automatically, reducing the chance of divergence.
3. **PR creation idempotency**: Make `gtr create-pull` idempotent (no-op if PR already exists for branch) so the agent can safely retry.

## 7. Questions for Human Reviewer

1. **Should we add a `supports_stdin` field to the orchestrator.toml agent definition** so operators can override the spawner's inference without rebuilding code?
2. **Should provider probes be disabled for rate-limited providers** (exponential backoff) instead of probing every 30 seconds?
3. **Should the swarm task script include `git push gitea` after `git push origin`** to enforce dual-remote sync, or should the orchestrator handle this?
4. **Why was no PR created for branch `gitea/task/1719-redact-env-vars-debug`** -- is the agent task missing error handling around `gtr create-pull`?
5. **Should we add a `post-exit` hook to the orchestrator** that verifies the agent produced a branch/PR before marking the run as "success"?
6. **Is the Claude rate limit expected to clear at 2am CEST daily**, or is this a more persistent limitation that requires a subscription upgrade?
7. **Should opencode be promoted back to primary** if Claude rate limits are recurring, or should we keep the current Claude-primary setup?
8. **Do we need a separate Gitea issue to track the `merge-coordinator` duplicate agent name** across `digital-twins.toml` and `terraphim.toml`?
9. **Should the 2-hour `max_cpu_seconds` be reduced** to something closer to the observed 20-minute successful runtime, to fail faster on hangs?
10. **Should we add a `dry-run` mode to the swarms** that runs the full task but stops before `git push`, for testing task scripts without side effects?
