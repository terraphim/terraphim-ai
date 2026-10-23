# Research Document: AI Dark Factory -- End-to-End Multi-Agent Orchestration

**Status**: Draft
**Author**: Terraphim AI / Phase 1 Disciplined Research
**Date**: 2026-03-06
**Reviewers**: Alexander Mikhalev

## Executive Summary

Terraphim-AI already has ~80% of the infrastructure needed for an AI Dark Factory: process spawning with health checks (terraphim_spawner), capability-based LLM routing (terraphim_router), OTP-style supervision trees (terraphim_agent_supervisor), Erlang-style messaging (terraphim_agent_messaging), and agent pooling with load balancing (terraphim_multi_agent). The missing ~20% is a unified AgentOrchestrator that wires these together with time-based scheduling, keyword-triggered process switching, Nightwatch behavioral drift detection, context handoff between agents, and nightly compound review integration.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Core product differentiator -- autonomous AI agent factory running on BigBox |
| Leverages strengths? | Yes | Uniquely positioned with Rust OTP patterns, KG-based routing, and existing 10-crate agent ecosystem |
| Meets real need? | Yes | terraphim-ai-agent-system.md marked "Ready for Implementation" 2026-03-04; compound review pattern already validated manually |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
Build a lights-out AI Dark Factory where 13+ agents (Claude Code, OpenCode, Codex) autonomously operate on the terraphim-ai project. Agents must spin up/down based on task type, switch LLM models by keyword/time, detect behavioral drift via Nightwatch pattern, and coordinate through shared knowledge graph and gate checkpoints.

### Impact
- **Without solution**: Manual agent management, no autonomous improvement loops, missed CVEs, delayed PR reviews, no cross-agent learning
- **With solution**: Nightly autonomous compound improvements, continuous security monitoring, event-driven code review, pattern-based meta-learning

### Success Criteria
1. 13 agents spawn, run, and are supervised across 3 layers (Safety/Core/Growth)
2. Time-based scheduling triggers agent spin-up/down on configured schedule
3. Keyword routing switches between CLI tools (e.g., "security scan" -> Codex, "code review" -> Claude)
4. Nightwatch detects drift >30% and applies correction (gentle to escalation)
5. Agents share context via knowledge graph and decision records
6. Nightly compound review loop runs autonomously producing PRs
7. Observer/Manager provides single-pane-of-glass monitoring

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose | Maturity |
|-----------|----------|---------|----------|
| AgentSpawner | `crates/terraphim_spawner/src/lib.rs` | Process lifecycle: spawn, shutdown (SIGTERM->SIGKILL), auto-restart | Production-ready |
| AgentPool (spawner) | `crates/terraphim_spawner/src/lib.rs:203` | Warm pool for reusable agent processes, LIFO checkout | Production-ready |
| HealthChecker | `crates/terraphim_spawner/src/health.rs` | 30s interval health monitoring, atomic status flag | Production-ready |
| CircuitBreaker | `crates/terraphim_spawner/src/health.rs` | 3-state (Closed/Open/HalfOpen), failure isolation | Production-ready |
| HealthHistory | `crates/terraphim_spawner/src/health.rs` | Sliding-window trend analysis, success_rate() | Production-ready |
| OutputCapture | `crates/terraphim_spawner/src/output.rs` | Line-by-line async capture, @mention detection | Production-ready |
| MentionRouter | `crates/terraphim_spawner/src/mention.rs` | Agent-to-agent @mention coordination | Partial (logs only) |
| AuditEvent | `crates/terraphim_spawner/src/audit.rs` | Structured audit logging via tracing | Production-ready |
| AgentConfig | `crates/terraphim_spawner/src/config.rs` | Provider-to-config, API key inference, resource limits (rlimit) | Production-ready |
| RoutingEngine | `crates/terraphim_router/src/engine.rs` | Keyword extraction + strategy selection + provider registry | Production-ready |
| ProviderRegistry | `crates/terraphim_router/src/registry.rs` | Markdown YAML frontmatter provider discovery, change notifications | Production-ready |
| RoutingStrategy trait | `crates/terraphim_router/src/strategy.rs` | CostOptimized, LatencyOptimized, CapabilityFirst, RoundRobin, Weighted | Production-ready |
| StrategyRegistry | `crates/terraphim_router/src/strategy.rs` | Runtime strategy lookup with factory functions | Production-ready |
| KeywordRouter | `crates/terraphim_router/src/keyword.rs` | Regex-based capability extraction from prompt text | Production-ready |
| KnowledgeGraphRouter | `crates/terraphim_router/src/knowledge_graph.rs` | Semantic routing via KG | Stub |
| AgentSupervisor | `crates/terraphim_agent_supervisor/src/supervisor.rs` | OTP supervision trees, restart strategies | Production-ready |
| RestartStrategy | `crates/terraphim_agent_supervisor/src/restart_strategy.rs` | OneForOne/OneForAll/RestForOne with intensity limits | Production-ready |
| AgentFactory trait | `crates/terraphim_agent_supervisor/src/agent.rs` | Factory pattern for creating supervised agents | Production-ready |
| AgentMessage | `crates/terraphim_agent_messaging/src/message.rs` | Call/Cast/Info/Reply/Ack message types | Production-ready |
| MessageRouter trait | `crates/terraphim_agent_messaging/src/router.rs` | Message routing with delivery guarantees + retries | Production-ready |
| MailboxManager | `crates/terraphim_agent_messaging/src/mailbox.rs` | Per-agent mailboxes with bounded channels | Production-ready |
| DeliveryManager | `crates/terraphim_agent_messaging/src/delivery.rs` | At-most/at-least/exactly-once delivery | Production-ready |
| AgentPool (multi) | `crates/terraphim_multi_agent/src/pool.rs` | Warm pool with LB strategies (LeastConnections, RoundRobin, etc.) | Production-ready |
| PoolManager | `crates/terraphim_multi_agent/src/pool_manager.rs` | Multi-role pool orchestration, on-demand creation | Production-ready |
| AgentRegistry | `crates/terraphim_multi_agent/src/registry.rs` | Centralized agent lookup by ID/capability/role | Production-ready |
| AgentEvolutionSystem | `crates/terraphim_agent_evolution/src/evolution.rs` | Memory, tasks, lessons evolution per agent | Production-ready |
| Routing workflow | `crates/terraphim_agent_evolution/src/workflows/routing.rs` | Task complexity analysis + route selection | Production-ready |
| HybridLlmRouter | `crates/terraphim_tinyclaw/src/agent/agent_loop.rs` | Proxy (tool-calling) + direct (Ollama) routing | Production-ready |
| SessionManager | `crates/terraphim_tinyclaw/src/session.rs` | Session state with history, persistence | Production-ready |
| MultiAgentWorkflowExecutor | `terraphim_server/src/workflows/` | Parallel multi-agent analysis with WebSocket broadcast | Production-ready |

### Data Flow (Current)

```
User Request
  -> RoutingEngine (keyword extraction + strategy)
    -> ProviderRegistry (find matching provider)
      -> AgentSpawner (spawn CLI process)
        -> HealthChecker (30s interval monitoring)
        -> OutputCapture (line-by-line + @mention detection)
      -> AgentSupervisor (fault tolerance, auto-restart)
      -> AgentPool (reuse warm agents)
  -> Response
```

### Data Flow (Target Dark Factory)

```
Schedule/Event Trigger
  -> AgentOrchestrator (NEW - central decision loop)
    -> TimeScheduler (NEW - cron-like agent scheduling)
    -> KeywordDispatcher (uses existing RoutingEngine)
    -> NightwatchMonitor (NEW - behavioral drift detection)
    |
    -> AgentSpawner (spawn/shutdown CLI processes)
    -> AgentSupervisor (fault tolerance, restart trees)
    -> PoolManager (warm pools per role)
    -> MessageRouter (inter-agent coordination)
    -> EvolutionSystem (learning capture)
    |
    -> CompoundReviewLoop (NEW - nightly improvement cycle)
      -> Review agent output from last 24h
      -> Identify highest-priority improvement
      -> Create PRD -> Tasks -> Implement -> PR
    |
    -> ContextHandoff (NEW - session state transfer)
      -> Serialize agent A's context
      -> Deserialize into agent B's session
    |
    -> ObserverDashboard (NEW - monitoring UI/API)
      -> WebSocket broadcasting (existing)
      -> Agent status, drift scores, pool stats
```

### Integration Points

| Integration | Protocol | Status |
|-------------|----------|--------|
| CLI tools (claude, opencode, codex) | Process spawn + stdout/stderr | Working via AgentSpawner |
| LLM APIs (OpenRouter, Ollama) | HTTP REST | Working via RoutingEngine |
| Knowledge Graph | In-memory automata | Working via terraphim_automata |
| Git operations | CLI commands | Available via tool execution |
| GitHub Issues/PRs | `gh` CLI | Available via tool execution |
| WebSocket monitoring | WS broadcast | Working via server workflows |
| MCP Agent Mail | HTTP API to bigbox:8765 | Available (external) |
| Beads task tracking | `bd` CLI | Available (external) |

## Constraints

### Technical Constraints
- **Platform**: macOS (development) + Linux BigBox (deployment). No `timeout` command on macOS.
- **Runtime**: Rust + tokio async. All agents run as child processes (not threads).
- **Process model**: Each agent is an OS process (claude, opencode, codex CLI). Communication via stdout/stderr capture + @mention routing.
- **Health checking**: Process-level only (is PID alive?). No semantic health (is agent producing useful output?).
- **Resource limits**: Applied via pre_exec rlimit (max_memory, max_cpu, max_files). Works on Linux, limited on macOS.

### Business Constraints
- **Deployment target**: Single BigBox server via tmux sessions
- **API costs**: Different models have different costs -- routing strategy must consider budget
- **Time**: Nightly compound review window ~30 minutes max
- **Human oversight**: Gate checkpoints require human approval before production changes

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Agent spawn time | < 2s | ~1s (CLI launch) |
| Health check interval | 30s | 30s (configurable) |
| Drift detection latency | < 5 min | N/A (not implemented) |
| Compound review duration | < 30 min | N/A (manual) |
| Max concurrent agents | 15 | Tested to 10 (PoolManager) |
| Agent restart time | < 10s | ~5s (spawn + init) |
| Context handoff latency | < 5s | N/A (not implemented) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| AgentOrchestrator must wire existing crates (no rewrites) | 80% infrastructure exists; rewriting wastes effort and introduces regressions | 10 production-ready crates already pass 3,133 tests |
| Nightwatch drift detection must be non-blocking | Monitoring that blocks agents defeats the purpose of autonomous operation | Erlang OTP principle: supervisor observes, does not impede |
| Context handoff must preserve session state | Agents switching without context lose accumulated knowledge, repeat work | terraphim_tinyclaw SessionManager already serializes session state |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Rewriting spawner/router/supervisor | Already production-ready, well-tested |
| Multi-machine distributed agents | BigBox is single server; tmux sessions sufficient for 15 agents |
| Real-time UI dashboard (Svelte) | WebSocket broadcast exists; CLI monitoring via Observer agent sufficient for Phase 1 |
| A/B/C test framework for meta-learning | Phase 2 concern after basic orchestration works |
| Cross-project agent coordination | Single project (terraphim-ai) first; multi-project via MCP Agent Mail later |
| Privacy filters for deep state sharing | Phase 2 concern; start with shallow sharing (patterns + corrections only) |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_spawner | Core process management. AgentOrchestrator wraps it. | Low -- stable, tested |
| terraphim_router | Keyword/capability routing. Orchestrator delegates routing decisions. | Low -- stable, tested |
| terraphim_agent_supervisor | Fault tolerance. Orchestrator registers agents with supervisor. | Low -- stable, tested |
| terraphim_agent_messaging | Inter-agent comms. Orchestrator uses for agent-to-agent coordination. | Low -- stable, tested |
| terraphim_multi_agent | Pool management. Orchestrator uses PoolManager for warm agents. | Low -- stable, tested |
| terraphim_agent_evolution | Learning capture. Observer uses for drift analysis. | Medium -- needs integration glue |
| terraphim_automata | Knowledge graph for semantic routing. | Low -- stable, core crate |
| terraphim_tinyclaw | Session management pattern. Context handoff borrows from this. | Medium -- currently a separate binary |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| tokio | 1.0 (full features) | Low | N/A (core runtime) |
| chrono | 0.4 | Low | jiff (not yet adopted) |
| serde/serde_json | 1.0 | Low | N/A |
| tracing | 0.1 | Low | N/A |
| cron (new dep) | ~0.12 | Low | Manual time parsing |
| claude CLI | External | Medium | Must be installed on BigBox |
| opencode CLI | External | Medium | Must be installed on BigBox |
| codex CLI | External | Medium | Must be installed on BigBox |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CLI tools not available on BigBox | Medium | High | Validate during startup; skip unavailable agents |
| Agent process leaks (zombie processes) | Medium | Medium | Existing SIGTERM->SIGKILL shutdown + process group cleanup |
| Drift detection false positives | High | Medium | Conservative thresholds (30%+); gentle correction first |
| Context handoff data loss | Medium | High | Serialize to JSON file; verify round-trip before switching |
| Nightly compound review produces bad PRs | Medium | Medium | Human gate review required; auto-revert on test failure |
| Resource exhaustion (15 agents on one box) | Low | High | Resource limits via rlimit; PoolManager caps at max_pool_size |
| API rate limits during parallel agent runs | Medium | Medium | Stagger agent schedules; use CostOptimized routing strategy |

### Open Questions

1. **What CLI flags do opencode and codex need for non-interactive/headless mode?** -- Needs investigation on BigBox
2. **How to serialize Claude Code session state for context handoff?** -- Session files are JSONL; need format validation
3. **Should Nightwatch run as a separate process or as a tokio task within the orchestrator?** -- Tokio task preferred (lower overhead), but separate process provides crash isolation
4. **What is the budget ceiling for nightly compound review API calls?** -- Needs business decision from Alexander
5. **Should agents share a single git worktree or use separate worktrees?** -- Separate worktrees prevent conflicts but use more disk

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| BigBox has claude, opencode, codex CLIs installed | Deployment plan references BigBox | Agents cannot spawn | No -- needs verification |
| Single-server deployment is sufficient for 15 agents | BigBox specs + 10-agent pool test passing | Need to scale or shed agents | Partially -- 10 agents tested |
| Nightwatch can detect drift from stdout/stderr patterns | Output capture already extracts lines + @mentions | May need LLM-based semantic analysis | No -- needs spike |
| JSONL session files can be parsed for context handoff | terraphim_tinyclaw already serializes sessions | Format may differ between CLI tools | No -- needs format analysis |
| Nightly 30-min window is sufficient for compound review | CTO executive system design specifies this | May need longer window or parallel execution | No -- needs measurement |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A: Orchestrator as library crate** | New `terraphim_orchestrator` crate wiring existing crates | **Chosen** -- composable, testable, follows existing pattern |
| **B: Orchestrator as standalone binary** | Separate process managing agents via IPC | Rejected -- adds IPC complexity, existing crates designed for in-process composition |
| **C: Orchestrator as terraphim_server extension** | Add orchestration endpoints to existing server | Rejected -- server is HTTP API; orchestration is background daemon |
| **A: Nightwatch as tokio task** | Runs within orchestrator process | **Chosen** for Phase 1 -- lower overhead, simpler |
| **B: Nightwatch as separate process** | Independent crash domain | Consider for Phase 2 if orchestrator instability observed |
| **A: Shallow context handoff (task description only)** | Transfer task text between agents | **Chosen** for Phase 1 -- simple, sufficient for most cases |
| **B: Deep context handoff (full session state)** | Transfer conversation history | Phase 2 -- requires CLI-specific session format parsing |

## Research Findings

### Key Insights

1. **The wiring gap is small**: ~300 lines of `AgentOrchestrator` connecting `AgentSpawner` + `RoutingEngine` + `AgentSupervisor` + `PoolManager` covers 70% of the dark factory. The remaining 30% is Nightwatch + scheduling + context handoff.

2. **Time scheduling maps to cron expressions**: The 3-layer architecture (Safety=always, Core=scheduled, Growth=on-demand) maps directly to:
   - Safety agents: spawned at orchestrator startup, restarted on failure
   - Core agents: cron-triggered (e.g., "0 2 * * *" for nightly sync)
   - Growth agents: event-triggered via message bus or @mention

3. **Keyword routing already works**: `RoutingEngine` extracts capabilities from prompts via `KeywordRouter` and routes to matching `Provider`. Adding CLI-tool-specific keywords (e.g., "security" -> codex, "review" -> claude) requires only provider configuration, not code changes.

4. **Nightwatch maps to HealthHistory + new metrics**: Existing `HealthHistory::success_rate()` provides the foundation. Adding output-based metrics (error patterns in stdout, learning capture rate, command success tracking) extends this to behavioral drift detection.

5. **Compound review is a workflow, not a new crate**: The nightly review loop is a sequence of existing operations: git log scan -> prioritize -> create task -> route to agent -> execute -> create PR. This is a workflow in `terraphim_server/src/workflows/` or the orchestrator.

6. **terraphim_tinyclaw's HybridLlmRouter is the model for agent switching**: It already routes between proxy (expensive/tool-calling) and direct Ollama (cheap/text-only) based on task type. The dark factory needs the same pattern but for CLI tools instead of LLM endpoints.

### Relevant Prior Art

- **Erlang/OTP Supervision Trees**: terraphim_agent_supervisor directly implements this. The dark factory adds a top-level supervisor for the entire agent fleet.
- **Kubernetes Controller Pattern**: Reconciliation loop (desired state vs actual state) maps to the orchestrator's decision loop.
- **Netflix Zuul / Envoy**: Routing + circuit breaker + health checking patterns already in terraphim_router + terraphim_spawner.
- **CTO Executive System Compound Review**: `~/cto-executive-system/automation/compound/` defines the nightly autonomous improvement pattern.
- **Nightwatch Pattern**: From `~/cto-executive-system/plans/terraphim-ai-agent-system.md` -- behavioral drift detection with correction levels.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| CLI headless mode verification | Verify claude/opencode/codex run headless on BigBox | 2 hours |
| Session format analysis | Parse JSONL from each CLI tool for context handoff | 4 hours |
| Nightwatch prototype | Implement stdout pattern analysis for drift detection | 4 hours |
| Compound review prototype | Run one complete nightly review cycle manually | 2 hours |

## Recommendations

### Proceed/No-Proceed
**Proceed** -- The gap is well-defined (~300 lines orchestrator + ~200 lines nightwatch + ~100 lines scheduler + ~150 lines context handoff), the infrastructure is production-tested (3,133 tests passing), and the design is validated by the CTO executive system plan.

### Scope Recommendations

**Phase 1 (MVP Dark Factory)** -- Estimated ~800 lines of new code in 1 new crate:
1. `terraphim_orchestrator` crate with:
   - `AgentOrchestrator` struct wiring spawner + router + supervisor + pool_manager
   - `TimeScheduler` using cron expressions for agent lifecycle
   - `NightwatchMonitor` with stdout-based drift detection
   - `CompoundReviewWorkflow` for nightly improvement loop
   - `ContextHandoff` for shallow task-description-level transfers

**Phase 2 (Deep Learning)**:
- Meta-Learning Agent for pattern mining
- Deep context handoff with full session state
- A/B test framework for comparing agent configurations
- Observer dashboard (WebSocket + CLI)

**Phase 3 (Self-Improvement)**:
- Recursive self-modification (agents proposing config/code changes)
- Cross-agent learning depth experiments
- Multi-project coordination via MCP Agent Mail

### Risk Mitigation Recommendations
1. **Start with 3 agents** (one per layer) before scaling to 13
2. **Conservative drift thresholds** -- 30% triggers gentle correction, 70% triggers human escalation
3. **Dry-run compound review** -- first 2 weeks create PRs but don't auto-merge
4. **Resource monitoring** -- track BigBox CPU/memory before adding more agents
5. **Verify CLI availability** on BigBox before any implementation

## Architecture: Proposed Crate Structure

```
crates/terraphim_orchestrator/
  src/
    lib.rs              -- Public API, AgentOrchestrator
    scheduler.rs        -- TimeScheduler (cron-based agent lifecycle)
    nightwatch.rs       -- NightwatchMonitor (drift detection + correction)
    compound.rs         -- CompoundReviewWorkflow
    handoff.rs          -- ContextHandoff
    config.rs           -- OrchestratorConfig (agent definitions, schedules, thresholds)
  Cargo.toml            -- Dependencies: spawner, router, supervisor, messaging, multi_agent, evolution
  tests/
    orchestrator_tests.rs
    nightwatch_tests.rs
    scheduler_tests.rs
```

### AgentOrchestrator Core Loop (Pseudocode)

```rust
pub struct AgentOrchestrator {
    spawner: AgentSpawner,
    router: RoutingEngine,
    supervisor: AgentSupervisor,
    pool_manager: PoolManager,
    nightwatch: NightwatchMonitor,
    scheduler: TimeScheduler,
    config: OrchestratorConfig,
}

impl AgentOrchestrator {
    pub async fn run(&mut self) -> Result<()> {
        // 1. Spawn Safety layer agents (always running)
        for agent_def in &self.config.safety_agents {
            self.spawn_and_supervise(agent_def).await?;
        }

        // 2. Start scheduler for Core/Growth layers
        self.scheduler.start(self.config.scheduled_agents.clone()).await?;

        // 3. Main reconciliation loop
        loop {
            tokio::select! {
                // a. Scheduler fires: spawn/shutdown scheduled agents
                event = self.scheduler.next_event() => {
                    self.handle_schedule_event(event).await?;
                }
                // b. Nightwatch detects drift: apply correction
                drift = self.nightwatch.next_alert() => {
                    self.handle_drift(drift).await?;
                }
                // c. Message from agent: route or handle
                msg = self.message_rx.recv() => {
                    self.handle_agent_message(msg).await?;
                }
                // d. Compound review trigger (nightly)
                _ = self.scheduler.compound_review_trigger() => {
                    self.run_compound_review().await?;
                }
            }
        }
    }
}
```

### NightwatchMonitor Drift Metrics

```rust
pub struct DriftMetrics {
    pub error_rate: f64,           // Errors / total commands
    pub command_success_rate: f64,  // Successful / total commands
    pub learning_capture_rate: f64, // Learnings captured / errors encountered
    pub output_consistency: f64,    // Std dev of output patterns
    pub health_score: f64,          // From HealthHistory::success_rate()
}

pub enum CorrectionLevel {
    Minor,    // 10-20% drift: log warning, refresh context
    Moderate, // 20-40% drift: reload config, clear caches
    Severe,   // 40-70% drift: restart agent
    Critical, // >70% drift: escalate to human, pause agent
}
```

## Next Steps

If approved, proceed to Phase 2 (Disciplined Design):
1. Design `AgentOrchestrator` public API and internal architecture
2. Define `OrchestratorConfig` schema (agent definitions, schedules, thresholds)
3. Design `NightwatchMonitor` drift calculation algorithm
4. Design `CompoundReviewWorkflow` step sequence
5. Design `ContextHandoff` serialization format
6. Create ADR (Architecture Decision Record) for key design choices
7. Define test plan covering all 7 success criteria

## Appendix

### Reference Materials
- `~/cto-executive-system/plans/terraphim-ai-agent-system.md` -- 13-agent architecture + Nightwatch
- `~/cto-executive-system/automation/compound/README.md` -- Compound review pattern
- `~/cto-executive-system/knowledge/context-engineering.md` -- Semantic backbone for agent alignment
- `~/cto-executive-system/knowledge/tooling-requirements-6d.md` -- System requirements for agent orchestration
- `.docs/codebase-report.md` -- Full codebase exploration report with dependency graph

### Existing Test Coverage
- terraphim_spawner: spawn, shutdown, health, circuit breaker, pool, output capture, mention routing
- terraphim_router: keyword extraction, strategy selection, provider registry, integration tests, benchmarks
- terraphim_agent_supervisor: supervision tree, restart strategies, restart intensity
- terraphim_agent_messaging: call/cast/info, delivery guarantees, routing, mailbox
- terraphim_multi_agent: pool, pool_manager, registry, workflows
- **Total**: 3,133 tests passing across workspace
