# Summary: terraphim_orchestrator/src/lib.rs

## Purpose

Multi-agent orchestration with scheduling, budgeting, and compound review. Implements the "dark factory" pattern for managing fleets of AI agents with resource scheduling, cost tracking, and coordinated review workflows.

## Core Components

- **AgentOrchestrator**: Main orchestrator running the dark factory pattern
- **DualModeOrchestrator**: Real-time and batch processing with fairness scheduling
- **CompoundReviewWorkflow**: Multi-agent review swarm with persona-based specialization
- **Scheduler**: Time-based and event-driven task scheduling
- **HandoffBuffer**: Inter-agent state transfer with TTL management
- **CostTracker**: Budget enforcement and spending monitoring
- **NightwatchMonitor**: Drift detection and rate limiting
- **MetaCoordinator**: Cross-project issue-driven agent dispatch with PageRank prioritisation

## Key Features

### Agent Management
- Safety-layer agents spawned immediately on startup
- Layer-based agent classification (Safety, Meta, Review, Implementation)
- Worktree management with automatic cleanup on agent crash
- Concurrency control with per-project agent limits
- Agent restart tracking with budget windowing

### Knowledge Graph Integration
- KG router loaded from taxonomy markdown files
- KG-boosted exit classification for structured error categorisation
- Provider health tracking with circuit breakers
- Stderr error signature classification per provider

### Routing & Budgeting
- Provider budget tracking (hour/day spend)
- Model routing via KG action templates
- Rate limiter with backoff support
- Telemetry store for model performance tracking

### Dark Factory Pattern
- Unified priority queue for all dispatch sources (time, issue, mention, review-pr, auto-merge, post-merge-gate)
- Per-PR rate limiting for verdict polling
- Auto-merge deduplication
- TTL-based failure dedupe cache

### Learning & Evolution
- Shared learning store integration
- Evolution manager for agent snapshots
- Session handoff with context transfer

## Key Modules

- `dispatcher`: Unified `Dispatcher` for multi-source task dispatch
- `scheduler`: `TimeScheduler` for cron-based agent firing
- `nightwatch`: `NightwatchMonitor` for drift detection
- `cost_tracker`: `CostTracker` for budget enforcement
- `compound`: `CompoundReviewWorkflow` for multi-agent review
- `handoff`: `HandoffBuffer`, `HandoffLedger` for inter-agent communication
- `kg_router`: KG-driven model router
- `evolution`: Agent evolution manager
- `learning`: Shared learning store
- `pr_gate`, `pr_poller`, `pr_dispatch`: PR workflow automation
- `worktree_guard`: Automatic worktree cleanup
- `provider_probe`: Provider health monitoring

## Configuration

Uses TOML config with agents defined as `[[agents]]` with:
- `name`, `layer`, `model`, `budget_monthly_cents`
- `schedule` (cron expression)
- `run_policy`, `restart_threshold`
- Project-scoped agents with `project` field