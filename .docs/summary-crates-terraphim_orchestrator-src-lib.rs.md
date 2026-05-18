# Summary: terraphim_orchestrator/src/lib.rs

**Purpose:** Multi-agent orchestration with scheduling, budgeting, and compound review for the "dark factory" pattern.

**Core Components:**
- **AgentOrchestrator**: Main orchestrator running dark factory pattern
- **DualModeOrchestrator**: Real-time and batch processing with fairness scheduling
- **CompoundReviewWorkflow**: Multi-agent review swarm with persona-based specialization
- **Scheduler**: Time-based and event-driven task scheduling
- **HandoffBuffer**: Inter-agent state transfer with TTL management
- **CostTracker**: Budget enforcement and spending monitoring
- **NightwatchMonitor**: Drift detection and rate limiting
- **MetaCoordinator**: Cross-project issue-driven agent dispatch with PageRank prioritisation

**Key Modules:**
- `adf_commands`: ADF-specific command handling
- `compound`: Compound review workflow with swarm config
- `control_plane`: Routing and fleet management
- `cost_tracker`: Metrics, verdicts, budget tracking
- `dispatcher`: Task dispatch with stats
- `evolution`: Agent evolution with memory snapshots
- `handoff`: Inter-agent state transfer
- `kg_router`: Knowledge graph-based routing
- `pr_dispatch`, `pr_gate`, `pr_review`: PR lifecycle management
- `provider_probe`: LLM provider health monitoring

**Key Exports:**
- `AgentRunRecord`, `ExitClass`, `ExitClassification`
- `CompoundReviewResult`, `SwarmConfig`
- `DualModeOrchestrator`
- `CostTracker`, `AgentMetrics`, `BudgetVerdict`
- `Dispatcher`, `DispatchTask`
- `HandoffBuffer`, `HandoffContext`
- `MentionTracker`, `MentionChainTracker`
- `OrchestratorConfig`, `AgentDefinition`, `AgentLayer`

**Configuration:**
- OrchestratorConfig for main settings
- AgentDefinition with evolution, model, capabilities
- CompoundReviewConfig for review workflows
- NightwatchConfig for rate limiting
- WebhookConfig for output posting

**Features:**
- PageRank-based issue prioritisation
- Token budget tracking with time limits
- Agent learning capture and evolution
- Multi-provider LLM routing (kimi, opencode-go, minimax, claude, etc.)
- Quickwit integration for log search