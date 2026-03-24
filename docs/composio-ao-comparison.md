# Composio Agent Orchestrator vs Terraphim ADF -- Comparison Analysis

- **Date**: 2026-03-24
- **Author**: Alex Mikhalev (CTO)
- **Source**: [github.com/ComposioHQ/agent-orchestrator](https://github.com/ComposioHQ/agent-orchestrator)
- **Tags**: architecture, agent-orchestration, competitive-analysis, adf

## 1. Overview

| Property | Composio AO | Terraphim ADF |
|----------|-------------|---------------|
| Language | TypeScript / Node.js | Rust |
| Agent model | 8 pluggable interface slots | TOML-configured CLI subprocesses |
| Runtime | tmux / Docker / K8s (pluggable) | `tokio::process::Command` + tmux |
| Workspace isolation | Git worktrees (first-class) | `ScopeRegistry` + `WorktreeManager` |
| Agent identity | Generic "agent" abstraction | Persona stack (Persona -> Role -> SFIA -> Skill Chain) |
| Configuration | YAML (`agent-orchestrator.yaml`) | TOML (`orchestrator.toml`) |
| Process model | Node.js orchestrator spawns agents | Rust binary with `tokio::select!` reconciliation loop |
| Supported agents | Claude Code, Codex, Aider, OpenCode | Claude Code, opencode, codex, pi |

## 2. Architecture Comparison

### 2.1 Agent Lifecycle

| Capability | Composio AO | Terraphim ADF |
|------------|-------------|---------------|
| Spawn mechanism | `SessionManager.spawn()` via plugin | `AgentSpawner` via `tokio::process::Command` |
| Session states | 16 states (spawning -> terminated) | 3 layers (Safety/Core/Growth) + health states |
| Health checking | `isAlive()` on `RuntimeHandle` | `HealthChecker` with `CircuitBreaker` (open/half-open/closed) |
| Activity detection | `getActivityState()` (6 states) | Output drain + reconciliation tick (30s) |
| Session persistence | Workspace restore, session remap | `--resume` for Claude, session IDs (GH#639 planned) |
| Permission model | 4 modes (permissionless/default/auto-edit/suggest) | `--dangerously-skip-permissions` or default |

### 2.2 Orchestration Patterns

| Pattern | Composio AO | Terraphim ADF |
|---------|-------------|---------------|
| Scheduling | Event-driven (webhook/poll) | Cron-based (`TimeScheduler`) + reconciliation loop |
| Parallel agents | One agent per issue, all parallel | Safety (always-on, 4), Core (cron, 11), Growth (on-demand, 45+) |
| Compound review | Not built-in (single agent per PR) | 18-agent compound review swarm (6 groups x 3) |
| Nightly automation | None | Two-phase compound loop (22:30 + 23:00) |
| Feedback loop | CI failure -> route back to agent | Judge system (CJE: quick/deep tiers) + verdict JSONL |
| Issue prioritisation | GitHub/Linear native | PageRank via `gitea-robot` (dependency-aware) |

### 2.3 Plugin Architecture (Composio)

Composio defines 8 swappable plugin slots, each with a TypeScript interface:

| Slot | Purpose | Default | Alternatives |
|------|---------|---------|--------------|
| Runtime | Execution environment | tmux | Docker, K8s, process |
| Agent | AI coding tool | Claude Code | Codex, Aider, OpenCode |
| Workspace | Code isolation | worktree | clone |
| Tracker | Issue management | GitHub Issues | Linear, Jira |
| SCM | PR lifecycle + CI + reviews | GitHub | GitLab |
| Notifier | Human alerts | desktop | Slack, webhook |
| Terminal | UI for interaction | iTerm2 | web, headless |
| Lifecycle Manager | Core state machine | built-in | (not pluggable) |

Terraphim ADF has no equivalent plugin system. Components are compiled Rust crates:

| ADF Crate | Rough Equivalent |
|-----------|-----------------|
| `terraphim_spawner` | Runtime + Agent |
| `terraphim_workspace` | Workspace |
| `terraphim_tracker` | Tracker |
| `terraphim_orchestrator` | Lifecycle Manager |
| `terraphim_agent_supervisor` | (no Composio equivalent) |
| `terraphim_goal_alignment` | (no Composio equivalent) |

### 2.4 Event and Reaction System (Composio)

Composio has a declarative reaction system with 25+ typed events:

```yaml
reactions:
  ci-failed:
    action: send-to-agent
    retries: 2
  changes-requested:
    action: send-to-agent
    escalateAfter: 30m
  approved:
    action: auto-merge
    conditions:
      ciPassing: true
```

ADF has no declarative reaction config. The reconciliation loop handles events procedurally:
- Poll agent exits -> restart Safety with cooldown
- Check cron schedule -> spawn Core agents
- Drain output -> evaluate drift

## 3. Where Composio AO is Stronger

### 3.1 Plugin Architecture
Eight cleanly-defined swappable slots with TypeScript interfaces. Adding a new runtime (K8s) or tracker (Jira) requires implementing one interface. ADF has no formal plugin system -- adding a new tracker means writing a Rust crate.

### 3.2 Declarative Reaction Config
CI failures, review comments, and approvals trigger configurable reactions with retry counts and escalation timeouts. ADF's reconciliation loop is procedural and less configurable.

### 3.3 Web Dashboard
Built-in web UI at localhost:3000 for monitoring all sessions. ADF has no dashboard -- monitoring is via `journalctl` and tmux.

### 3.4 Workspace Isolation
Worktrees are first-class with create/destroy/list/restore. ADF has `WorktreeManager` but it is newer and less exercised.

### 3.5 Multi-Project Support
Native per-project config with repo/branch/agent overrides in YAML. ADF currently handles one `working_dir` per agent definition in TOML.

### 3.6 Ease of Adoption
`npm install -g @composio/ao && ao start` vs building a Rust binary and managing a systemd service.

## 4. Where Terraphim ADF is Stronger

### 4.1 Compound Review
18 parallel review agents (security, architecture, performance, quality, domain) with deduplication and judge system. Composio has no equivalent -- it runs one agent per issue with no multi-agent review.

### 4.2 Agent Identity and Persona System
Full identity stack: Persona (Ferrox, Vigil, Carthos...) -> Terraphim Role -> SFIA Profile -> Skill Chain. Each agent has a name, symbol, speech pattern, and competency level. Composio agents are generic.

### 4.3 Judge System
Multi-tier quality evaluation:
- Quick tier: GPT-5 Nano (advisory, ~84% verdict agreement)
- Deep tier: Kimi K2.5 (quality gate, r=0.63/0.71/0.53, 62.5% NO-GO detection)
- Oracle tier: Claude Opus 4.6 (calibration baseline, 100% inter-rater reliability)

Composio relies on CI + human review only.

### 4.4 Knowledge Graph Integration
Aho-Corasick automata for domain term matching, per-project KG namespaces (`~/.config/terraphim/kg/projects/<slug>/`). Text replacement hooks enforce terminology. No equivalent in Composio.

### 4.5 Self-Learning Loops
Six nested learning loops (minutes to monthly):
1. Failure-to-guardrail (immediate)
2. Prediction calibration (per-task)
3. Nightly pattern extraction
4. Friction detection (weekly)
5. Playbook curation (Elo-gated)
6. Monthly meta-review

Plus MemSkill novelty scoring, autoresearch optimiser, and config backtester. Composio has CI retry but no learning architecture.

### 4.6 Budget Enforcement
Per-agent `budget_monthly_cents`, execution tiers (Safe/Review/Critical) with hard-stop gates at $5 soft / $10 hard per agent session. Composio mentions budget but has no implementation.

### 4.7 Provider Routing
Subscription-aware model routing with fallback chains:
- `opencode-go/*` (MiniMax, GLM, Kimi via Go subscription)
- `kimi-for-coding/*` (Moonshot subscription)
- `anthropic` (Claude OAuth)
- `zai-coding-plan/*` (z.ai subscription)

Composio delegates model choice entirely to the agent plugin.

### 4.8 Three-Layer Agent Hierarchy
- **Safety** (always-on, auto-restart with cooldown): security-sentinel, meta-coordinator, compliance-watchdog, drift-detector
- **Core** (cron-scheduled): upstream-synchronizer, product-development, spec-validator, test-guardian, etc.
- **Growth** (on-demand, scales to 45+): implementation-swarm, compound-review, scenario-tester, etc.

Composio has a flat orchestrator -> worker model.

## 5. Patterns to Adopt from Composio AO

| Pattern | Benefit | ADF Implementation Path |
|---------|---------|------------------------|
| Typed plugin interfaces | Swap runtimes/trackers without recompilation | Define Rust traits for Runtime, Tracker, Notifier, SCM. Partially started with `GenAgent` (GH#535). |
| Declarative reaction config | `ci-failed: retries: 2, escalateAfter: 30m` in TOML | Extend `orchestrator.toml` with `[reactions]` table. Wire into reconciliation loop. |
| Web dashboard | Visual monitoring instead of journalctl | Tauri desktop app or simple web UI reading ADF's activity log (GH#641). |
| Session restore | Resume interrupted work seamlessly | Already planned (GH#639). Composio's `restore()` + `remap()` pattern validates the approach. |
| Issue decomposition | Auto-split large issues before dispatch | Leverage existing `terraphim_task_decomposition` crate. Composio's `decomposer` config (depth, model, approval) is a good reference. |
| Webhook-driven events | React to GitHub/Gitea webhooks instead of polling | Add webhook receiver to ADF reconciliation loop. Composio's `parseWebhook()` + `verifyWebhook()` pattern. |
| Multi-project config | Per-project overrides in a single config file | Extend `orchestrator.toml` with `[projects.<name>]` tables mapping to repos, agents, and budgets. |

## 6. Patterns NOT to Adopt

| Pattern | Reason |
|---------|--------|
| Node.js runtime | Rust gives us memory safety, single binary, lower resource usage. The orchestrator runs on a shared bigbox with 20+ agents -- Node.js GC pauses and memory bloat are unacceptable. |
| Generic agent identity | Persona + SFIA + Skill Chain provides measurably better agent output quality (calibrated via CJE). Reverting to generic agents loses this. |
| Single-agent-per-PR review | Compound 18-agent review with judge system catches issues that a single agent misses. CJE calibration proves this empirically. |
| External plugin registry | Composio's `PluginRegistry` loads plugins dynamically. Rust's compile-time guarantees via trait objects + feature flags are safer for production orchestration. |

## 7. Conclusion

Composio AO and Terraphim ADF solve overlapping but distinct problems:

- **Composio AO**: Developer-friendly orchestration for parallel issue resolution. Strengths in UX, pluggability, and event-driven reactions. Weak in quality assurance and learning.
- **Terraphim ADF**: Production-grade autonomous factory with compound review, calibrated judge, self-learning, and budget enforcement. Weak in UX, ease of adoption, and multi-project flexibility.

The two systems are complementary. Composio's plugin architecture and reaction system should inform ADF's `GenAgent` trait design (GH#535) and future `[reactions]` config. ADF's compound review, judge, and learning loops have no equivalent in the Composio ecosystem.

**Priority adoptions for terraphim-ai**:
1. Declarative `[reactions]` in orchestrator.toml (low effort, high value)
2. Web dashboard via activity log (GH#641 prerequisite)
3. Multi-project config tables (enables client isolation per autonomous-org plan Section 16, Decision #19)
