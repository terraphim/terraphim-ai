# Implementation Plan: terraphim_orchestrator -- AI Dark Factory

**Status**: Draft
**Research Doc**: `.docs/research-dark-factory-orchestration.md`
**Author**: Terraphim AI / Phase 2 Disciplined Design
**Date**: 2026-03-06
**Estimated Effort**: 3-4 days

## Overview

### Summary
New library crate `terraphim_orchestrator` that wires existing spawner, router, supervisor, messaging, and pool crates into a reconciliation loop. Adds time-based scheduling, Nightwatch drift detection, nightly compound review, and shallow context handoff.

### Approach
Kubernetes-style reconciliation loop: declare desired agent fleet state in config, orchestrator continuously reconciles actual state to match. Uses `tokio::select!` to multiplex schedule triggers, drift alerts, agent messages, and compound review events.

### Scope
**In Scope (Top 5):**
1. AgentOrchestrator reconciliation loop
2. TimeScheduler with cron expressions
3. NightwatchMonitor with drift metrics and correction levels
4. CompoundReviewWorkflow for nightly autonomous improvement
5. OrchestratorConfig with TOML-based agent fleet definition

**Out of Scope:**
- Meta-Learning Agent (Phase 2)
- Deep context handoff with full session state (Phase 2)
- A/B test framework (Phase 2)
- UI dashboard (Phase 2)
- Multi-project coordination (Phase 3)

**Avoid At All Cost** (5/25 rule):
- Custom process IPC protocol (use existing stdout/stderr capture)
- Custom serialization format (use serde_json)
- Agent-to-agent direct communication channels (use existing MessageRouter)
- Plugin/extension system for custom agents (config-driven is enough)
- Distributed consensus / leader election (single server)
- Custom logging framework (use existing tracing)
- WebSocket protocol for agent communication (stdout capture works)
- Custom cron parser (use `cron` crate)
- Agent sandboxing beyond existing rlimits (use Firecracker for that)
- Real-time metrics aggregation service (tracing spans are sufficient)

## Architecture

### Component Diagram

```
OrchestratorConfig (TOML)
        |
        v
AgentOrchestrator
  |-- TimeScheduler -------> cron triggers
  |-- NightwatchMonitor ----> drift alerts
  |-- CompoundReview -------> nightly events
  |
  |-- AgentSpawner (existing) --> OS processes
  |-- RoutingEngine (existing) -> keyword dispatch
  |-- AgentSupervisor (existing) -> fault tolerance
  |-- PoolManager (existing) ---> warm agents
  |-- OutputCapture (existing) -> stdout/stderr
```

### Data Flow

```
[Cron tick / Event / Message]
  -> AgentOrchestrator::run()
    -> tokio::select! {
         scheduler.next()    -> spawn_or_shutdown(agent_def)
         nightwatch.next()   -> apply_correction(agent_id, level)
         message_rx.recv()   -> route_or_handle(msg)
         compound_trigger()  -> run_compound_review()
       }
    -> AgentSpawner::spawn(provider, task)
      -> HealthChecker (30s)
      -> OutputCapture (lines -> NightwatchMonitor)
    -> AgentSupervisor::handle_agent_exit(id, reason)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Library crate, not binary | Composable, testable, can embed in server or standalone | Standalone binary adds IPC overhead |
| TOML config (not JSON) | Consistent with workspace settings.toml pattern | JSON lacks comments, YAML too complex |
| `cron` crate for scheduling | Battle-tested, standard cron syntax | Manual parsing is error-prone |
| Reconciliation loop pattern | Declarative (desired vs actual), self-healing | Imperative step sequences are fragile |
| Stdout-based drift detection | Already captured by OutputCapture, zero new I/O | LLM-based analysis too expensive for continuous monitoring |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Agent-to-agent gRPC | Agents are CLI processes, not gRPC servers | Would require modifying all CLI tools |
| Database for agent state | In-memory + tracing is sufficient for single server | Adds dep, schema maintenance, migration |
| Custom health protocol | Process alive + stdout patterns covers 95% of cases | Over-engineering for Phase 1 |
| ContextHandoff as separate crate | Too small; 1 struct + serialize/deserialize | Crate proliferation |
| Hot config reload | Start/stop orchestrator is fast enough | Adds complexity to reconciliation loop |

### Simplicity Check

> **What if this could be easy?**

The orchestrator is a `loop { tokio::select! { ... } }` that reacts to 4 event sources. Each handler calls 1-2 existing crate methods. The entire crate is ~800 lines including tests. No new protocols, no new serialization, no new I/O -- just glue between existing production-ready crates.

**Senior Engineer Test**: A senior engineer would say "this is just a controller loop with cron, health checks, and a review script. That's the right level of complexity."

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose | Est. Lines |
|------|---------|------------|
| `crates/terraphim_orchestrator/Cargo.toml` | Crate manifest with workspace deps | 30 |
| `crates/terraphim_orchestrator/src/lib.rs` | Public API: `AgentOrchestrator`, re-exports | 80 |
| `crates/terraphim_orchestrator/src/config.rs` | `OrchestratorConfig`, `AgentDefinition`, TOML parsing | 120 |
| `crates/terraphim_orchestrator/src/scheduler.rs` | `TimeScheduler`, cron evaluation, event channel | 100 |
| `crates/terraphim_orchestrator/src/nightwatch.rs` | `NightwatchMonitor`, `DriftMetrics`, `CorrectionLevel` | 150 |
| `crates/terraphim_orchestrator/src/compound.rs` | `CompoundReviewWorkflow`, git scan, PR creation | 120 |
| `crates/terraphim_orchestrator/src/handoff.rs` | `ContextHandoff`, shallow serialize/deserialize | 60 |
| `crates/terraphim_orchestrator/src/error.rs` | `OrchestratorError` enum | 30 |
| `crates/terraphim_orchestrator/tests/orchestrator_tests.rs` | Integration tests for reconciliation loop | 120 |
| `crates/terraphim_orchestrator/tests/nightwatch_tests.rs` | Drift calculation and correction tests | 80 |
| `crates/terraphim_orchestrator/tests/scheduler_tests.rs` | Cron scheduling tests | 60 |

**Total new code**: ~950 lines (including tests)

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `"crates/terraphim_orchestrator"` to members |

### Deleted Files
None.

## API Design

### Public Types

```rust
// config.rs

/// Top-level orchestrator configuration (parsed from TOML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Working directory for all agents
    pub working_dir: PathBuf,
    /// Nightwatch configuration
    pub nightwatch: NightwatchConfig,
    /// Compound review configuration
    pub compound_review: CompoundReviewConfig,
    /// Agent definitions by layer
    pub agents: Vec<AgentDefinition>,
}

/// Definition of a single agent in the fleet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique name (e.g., "security-sentinel")
    pub name: String,
    /// Which layer: safety, core, growth
    pub layer: AgentLayer,
    /// CLI tool to use (maps to Provider)
    pub cli_tool: String,
    /// Task/prompt to give the agent on spawn
    pub task: String,
    /// Cron schedule (None = always running for safety, or on-demand for growth)
    pub schedule: Option<String>,
    /// Capabilities this agent provides
    pub capabilities: Vec<String>,
    /// Maximum memory in bytes (optional resource limit)
    pub max_memory_bytes: Option<u64>,
}

/// Agent layer in the dark factory hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentLayer {
    /// Always running, auto-restart on failure
    Safety,
    /// Cron-scheduled or event-triggered
    Core,
    /// On-demand, spawned when needed
    Growth,
}

/// Nightwatch thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightwatchConfig {
    /// How often to evaluate drift (seconds)
    pub eval_interval_secs: u64,
    /// Drift percentage thresholds for each correction level
    pub minor_threshold: f64,    // default: 0.10
    pub moderate_threshold: f64, // default: 0.20
    pub severe_threshold: f64,   // default: 0.40
    pub critical_threshold: f64, // default: 0.70
}

/// Compound review settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundReviewConfig {
    /// Cron schedule for compound review (e.g., "0 2 * * *")
    pub schedule: String,
    /// Maximum duration in seconds
    pub max_duration_secs: u64,
    /// Git repository path
    pub repo_path: PathBuf,
    /// Whether to create PRs (false = dry run)
    pub create_prs: bool,
}
```

```rust
// nightwatch.rs

/// Behavioral drift metrics for a single agent
#[derive(Debug, Clone, Default)]
pub struct DriftMetrics {
    /// Errors / total output lines
    pub error_rate: f64,
    /// Non-error commands / total commands
    pub command_success_rate: f64,
    /// Process health from HealthHistory
    pub health_score: f64,
    /// Number of samples in the evaluation window
    pub sample_count: u64,
}

/// Drift score combining all metrics into a single 0.0-1.0 value
#[derive(Debug, Clone)]
pub struct DriftScore {
    pub agent_name: String,
    pub score: f64,
    pub metrics: DriftMetrics,
    pub level: CorrectionLevel,
}

/// Correction level based on drift severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrectionLevel {
    /// No drift detected
    Normal,
    /// 10-20% drift: log warning, refresh context
    Minor,
    /// 20-40% drift: reload config
    Moderate,
    /// 40-70% drift: restart agent
    Severe,
    /// >70% drift: pause agent, escalate to human
    Critical,
}

/// Alert emitted by NightwatchMonitor when drift exceeds threshold
#[derive(Debug, Clone)]
pub struct DriftAlert {
    pub agent_name: String,
    pub drift_score: DriftScore,
    pub recommended_action: CorrectionAction,
}

/// Action the orchestrator should take in response to drift
#[derive(Debug, Clone)]
pub enum CorrectionAction {
    /// Log and continue
    LogWarning(String),
    /// Restart the agent
    RestartAgent,
    /// Pause agent and notify human
    PauseAndEscalate(String),
}
```

```rust
// scheduler.rs

/// Schedule event indicating an agent should be spawned or stopped
#[derive(Debug, Clone)]
pub enum ScheduleEvent {
    /// Time to spawn this agent
    Spawn(AgentDefinition),
    /// Time to stop this agent
    Stop { agent_name: String },
    /// Time to run compound review
    CompoundReview,
}
```

```rust
// handoff.rs

/// Tracks API rate limits per agent per provider
#[derive(Debug, Clone, Default)]
pub struct RateLimitTracker {
    /// Calls made per (agent_name, provider_id) in current window
    pub calls: HashMap<(String, String), RateLimitWindow>,
}

/// Sliding window for rate limit tracking
#[derive(Debug, Clone)]
pub struct RateLimitWindow {
    /// Calls in current hour
    pub calls_this_hour: u32,
    /// Provider-reported limit (from HTTP headers, if available)
    pub hourly_limit: Option<u32>,
    /// Window start
    pub window_start: chrono::DateTime<chrono::Utc>,
}

impl RateLimitTracker {
    /// Record an API call for an agent+provider pair
    pub fn record_call(&mut self, agent_name: &str, provider_id: &str);

    /// Check if an agent can make more calls to a provider
    pub fn can_call(&self, agent_name: &str, provider_id: &str) -> bool;

    /// Update limit from provider response headers
    pub fn update_limit(&mut self, provider_id: &str, limit: u32);

    /// Get remaining calls for an agent+provider
    pub fn remaining(&self, agent_name: &str, provider_id: &str) -> Option<u32>;
}

/// Shallow context transferred between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    /// Task description being handed off
    pub task: String,
    /// Summary of work completed so far
    pub progress_summary: String,
    /// Key decisions made
    pub decisions: Vec<String>,
    /// Files modified
    pub files_touched: Vec<PathBuf>,
    /// Timestamp of handoff
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### Public Functions

```rust
// lib.rs

/// The main orchestrator that runs the dark factory
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    spawner: AgentSpawner,
    router: RoutingEngine,
    nightwatch: NightwatchMonitor,
    scheduler: TimeScheduler,
    // Internal state
    active_agents: HashMap<String, AgentHandle>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl AgentOrchestrator {
    /// Create a new orchestrator from configuration
    pub fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError>;

    /// Create from a TOML config file path
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, OrchestratorError>;

    /// Run the orchestrator (blocks until shutdown signal)
    ///
    /// This is the main reconciliation loop. It:
    /// 1. Spawns all Safety-layer agents immediately
    /// 2. Starts the scheduler for Core-layer agents
    /// 3. Enters the select! loop handling events
    ///
    /// Returns when shutdown is signaled or a critical error occurs.
    pub async fn run(&mut self) -> Result<(), OrchestratorError>;

    /// Request graceful shutdown of all agents and the orchestrator
    pub fn shutdown(&mut self);

    /// Get current status of all agents
    pub fn agent_statuses(&self) -> Vec<AgentStatus>;

    /// Manually trigger a compound review (outside normal schedule)
    pub async fn trigger_compound_review(&mut self) -> Result<CompoundReviewResult, OrchestratorError>;

    /// Hand off a task from one agent to another
    pub async fn handoff(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        context: HandoffContext,
    ) -> Result<(), OrchestratorError>;
}

/// Status of a single agent in the fleet
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub name: String,
    pub layer: AgentLayer,
    pub running: bool,
    pub health: HealthStatus,
    pub drift_score: Option<f64>,
    pub uptime: Duration,
    pub restart_count: u32,
    /// API calls remaining per provider (None if no limit known)
    pub api_calls_remaining: HashMap<String, Option<u32>>,
}
```

```rust
// nightwatch.rs

/// Monitors agent behavior and detects drift
pub struct NightwatchMonitor {
    config: NightwatchConfig,
    // Per-agent metric accumulators
    agent_metrics: HashMap<String, AgentMetricAccumulator>,
    // Channel for emitting alerts
    alert_tx: mpsc::Sender<DriftAlert>,
    alert_rx: mpsc::Receiver<DriftAlert>,
}

impl NightwatchMonitor {
    /// Create a new monitor with the given configuration
    pub fn new(config: NightwatchConfig) -> Self;

    /// Feed an output event from an agent into the monitor
    ///
    /// Called by the orchestrator for every OutputEvent received
    /// from agent stdout/stderr. The monitor accumulates metrics
    /// and evaluates drift on the configured interval.
    pub fn observe(&mut self, agent_name: &str, event: &OutputEvent);

    /// Feed a health status update into the monitor
    pub fn observe_health(&mut self, agent_name: &str, status: HealthStatus);

    /// Get the next drift alert (async, used in select!)
    pub async fn next_alert(&mut self) -> DriftAlert;

    /// Get current drift score for an agent (synchronous query)
    pub fn drift_score(&self, agent_name: &str) -> Option<DriftScore>;

    /// Get all current drift scores
    pub fn all_drift_scores(&self) -> Vec<DriftScore>;

    /// Reset metrics for an agent (after restart)
    pub fn reset(&mut self, agent_name: &str);
}
```

```rust
// scheduler.rs

/// Cron-based scheduler for agent lifecycle events
pub struct TimeScheduler {
    schedules: Vec<ScheduleEntry>,
    compound_schedule: Option<cron::Schedule>,
    event_tx: mpsc::Sender<ScheduleEvent>,
    event_rx: mpsc::Receiver<ScheduleEvent>,
}

impl TimeScheduler {
    /// Create a new scheduler from agent definitions
    pub fn new(agents: &[AgentDefinition], compound_schedule: Option<&str>)
        -> Result<Self, OrchestratorError>;

    /// Start the scheduler background task
    pub fn start(&self) -> tokio::task::JoinHandle<()>;

    /// Get the next scheduled event (async, used in select!)
    pub async fn next_event(&mut self) -> ScheduleEvent;
}
```

```rust
// compound.rs

/// Result of a compound review cycle
#[derive(Debug, Clone)]
pub struct CompoundReviewResult {
    /// What was found during review
    pub findings: Vec<String>,
    /// Highest-priority improvement identified
    pub top_improvement: Option<String>,
    /// Whether a PR was created
    pub pr_created: bool,
    /// PR URL if created
    pub pr_url: Option<String>,
    /// Duration of the review
    pub duration: Duration,
}

/// Nightly compound review workflow
pub struct CompoundReviewWorkflow {
    config: CompoundReviewConfig,
}

impl CompoundReviewWorkflow {
    pub fn new(config: CompoundReviewConfig) -> Self;

    /// Run a full compound review cycle
    ///
    /// 1. Scan git log for last 24h of changes
    /// 2. Identify top improvement opportunity
    /// 3. Route improvement task to appropriate agent
    /// 4. Create PR with results (if config.create_prs is true)
    pub async fn run(
        &self,
        spawner: &AgentSpawner,
        router: &RoutingEngine,
    ) -> Result<CompoundReviewResult, OrchestratorError>;
}
```

### Error Types

```rust
// error.rs

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("agent spawn failed for '{agent}': {reason}")]
    SpawnFailed { agent: String, reason: String },

    #[error("agent '{0}' not found")]
    AgentNotFound(String),

    #[error("scheduler error: {0}")]
    SchedulerError(String),

    #[error("compound review failed: {0}")]
    CompoundReviewFailed(String),

    #[error("handoff failed from '{from}' to '{to}': {reason}")]
    HandoffFailed { from: String, to: String, reason: String },

    #[error(transparent)]
    Spawner(#[from] terraphim_spawner::SpawnerError),

    #[error(transparent)]
    Routing(#[from] terraphim_router::types::RoutingError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_config_parse_minimal` | `config.rs` | Parse minimal valid TOML config |
| `test_config_parse_full` | `config.rs` | Parse config with all agents and options |
| `test_config_defaults` | `config.rs` | Default values for optional fields |
| `test_drift_metrics_zero` | `nightwatch.rs` | Zero events = Normal drift |
| `test_drift_metrics_minor` | `nightwatch.rs` | 10-20% error rate = Minor |
| `test_drift_metrics_moderate` | `nightwatch.rs` | 20-40% error rate = Moderate |
| `test_drift_metrics_severe` | `nightwatch.rs` | 40-70% error rate = Severe |
| `test_drift_metrics_critical` | `nightwatch.rs` | >70% error rate = Critical |
| `test_drift_reset` | `nightwatch.rs` | Reset clears accumulated metrics |
| `test_correction_level_ordering` | `nightwatch.rs` | Normal < Minor < Moderate < Severe < Critical |
| `test_schedule_parse_cron` | `scheduler.rs` | Valid cron expression parses |
| `test_schedule_invalid_cron` | `scheduler.rs` | Invalid cron returns error |
| `test_schedule_safety_always` | `scheduler.rs` | Safety agents have no schedule (always on) |
| `test_handoff_roundtrip` | `handoff.rs` | Serialize -> deserialize preserves context |
| `test_compound_review_dry_run` | `compound.rs` | Dry run produces findings but no PR |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_orchestrator_spawns_safety_agents` | `orchestrator_tests.rs` | Safety agents start on run() |
| `test_orchestrator_shutdown_cleans_up` | `orchestrator_tests.rs` | All agents stopped on shutdown |
| `test_orchestrator_handles_drift_alert` | `orchestrator_tests.rs` | Drift -> correction action applied |
| `test_nightwatch_accumulates_from_output` | `nightwatch_tests.rs` | OutputEvents feed into metrics |
| `test_scheduler_fires_at_cron_time` | `scheduler_tests.rs` | Cron trigger emits ScheduleEvent |

### Tests NOT Needed (Essentialism)
- End-to-end tests requiring actual CLI tools (covered by existing spawner tests)
- Performance benchmarks (not needed for Phase 1 controller loop)
- Property/fuzzing tests (input space is small and well-defined)

## Implementation Steps

### Step 1: Crate Scaffold + Config
**Files:** `Cargo.toml`, `src/lib.rs`, `src/config.rs`, `src/error.rs`
**Description:** Create crate, define OrchestratorConfig with TOML parsing, define error types
**Tests:** `test_config_parse_minimal`, `test_config_parse_full`, `test_config_defaults`
**Dependencies:** None
**Estimated:** 3 hours

Key code:
```rust
// Cargo.toml deps
[dependencies]
terraphim_spawner = { path = "../terraphim_spawner" }
terraphim_router = { path = "../terraphim_router" }
terraphim_types = { path = "../terraphim_types" }
tokio = { workspace = true }
serde = { workspace = true }
toml = "0.8"
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = "0.1"
cron = "0.13"
```

### Step 2: NightwatchMonitor
**Files:** `src/nightwatch.rs`
**Description:** Drift metrics accumulation from OutputEvents, drift score calculation, alert emission via mpsc channel
**Tests:** All `test_drift_*` tests, `test_correction_level_ordering`
**Dependencies:** Step 1 (error types)
**Estimated:** 4 hours

Key algorithm:
```rust
fn calculate_drift(&self, metrics: &DriftMetrics) -> f64 {
    // Weighted average of individual metric deviations from baseline
    let error_weight = 0.4;
    let success_weight = 0.3;
    let health_weight = 0.3;

    let error_drift = metrics.error_rate; // 0.0 = perfect
    let success_drift = 1.0 - metrics.command_success_rate; // 0.0 = perfect
    let health_drift = 1.0 - metrics.health_score; // 0.0 = perfect

    error_weight * error_drift
        + success_weight * success_drift
        + health_weight * health_drift
}
```

### Step 3: TimeScheduler
**Files:** `src/scheduler.rs`
**Description:** Parse cron expressions from AgentDefinitions, background task that evaluates schedules and emits ScheduleEvents
**Tests:** `test_schedule_parse_cron`, `test_schedule_invalid_cron`, `test_schedule_safety_always`
**Dependencies:** Step 1 (config types)
**Estimated:** 3 hours

### Step 4: CompoundReviewWorkflow
**Files:** `src/compound.rs`
**Description:** Git log scan, finding prioritization, task routing to agent, PR creation via `gh` CLI
**Tests:** `test_compound_review_dry_run`
**Dependencies:** Step 1 (config, error types)
**Estimated:** 3 hours

### Step 5: ContextHandoff
**Files:** `src/handoff.rs`
**Description:** HandoffContext struct, JSON serialization to file, deserialization
**Tests:** `test_handoff_roundtrip`
**Dependencies:** Step 1 (types)
**Estimated:** 1 hour

### Step 6: AgentOrchestrator (Core Loop)
**Files:** `src/lib.rs` (expand)
**Description:** Wire spawner + router + nightwatch + scheduler into reconciliation loop. Implement `run()`, `shutdown()`, `agent_statuses()`, `handoff()`, `trigger_compound_review()`
**Tests:** `test_orchestrator_spawns_safety_agents`, `test_orchestrator_shutdown_cleans_up`, `test_orchestrator_handles_drift_alert`
**Dependencies:** Steps 1-5
**Estimated:** 4 hours

### Step 7: Workspace Integration + Example Config
**Files:** `Cargo.toml` (workspace), example TOML config
**Description:** Add to workspace members, create example config with 3 agents (one per layer)
**Tests:** `cargo test -p terraphim_orchestrator`
**Dependencies:** Step 6
**Estimated:** 1 hour

## Rollback Plan

If issues discovered:
1. Remove `crates/terraphim_orchestrator` from workspace members
2. No other crates are modified, so zero rollback risk to existing code
3. Git revert the single commit adding the crate

No feature flags needed -- the crate is purely additive and opt-in.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| `cron` | 0.13 | Parse standard cron expressions for scheduling |
| `toml` | 0.8 | Parse TOML config files (consistent with existing settings.toml pattern) |

### Existing Workspace Dependencies Used
- `tokio` (full), `serde`/`serde_json`, `chrono`, `thiserror`, `tracing`, `anyhow`

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Reconciliation loop latency | < 10ms per iteration | tracing spans |
| Drift evaluation | < 1ms per agent | Unit test timing |
| Cron evaluation | < 1ms per tick | Unit test timing |
| Memory per agent metrics | < 10KB (sliding window) | Struct size calculation |

No benchmarks needed for Phase 1. The reconciliation loop is I/O bound (waiting on channels), not CPU bound.

## Example Configuration

```toml
# orchestrator.toml -- Dark Factory Agent Fleet

working_dir = "/Users/alex/projects/terraphim/terraphim-ai"

[nightwatch]
eval_interval_secs = 300  # 5 minutes
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"  # 2 AM daily
max_duration_secs = 1800  # 30 minutes
repo_path = "/Users/alex/projects/terraphim/terraphim-ai"
create_prs = false  # Dry run for first 2 weeks

# --- Safety Layer (always running) ---

[[agents]]
name = "security-sentinel"
layer = "Safety"
cli_tool = "codex"
task = "Continuously scan for CVEs and security vulnerabilities in dependencies. Run cargo audit and report findings."
capabilities = ["security", "vulnerability-scanning"]
max_memory_bytes = 2_147_483_648  # 2GB

# --- Core Layer (scheduled) ---

[[agents]]
name = "upstream-synchronizer"
layer = "Core"
cli_tool = "codex"
task = "Sync with upstream repositories. Check for new releases of key dependencies."
schedule = "0 3 * * *"  # 3 AM daily
capabilities = ["sync", "dependency-management"]

# --- Growth Layer (on-demand) ---

[[agents]]
name = "code-reviewer"
layer = "Growth"
cli_tool = "claude"
task = "Review the latest PR for code quality, security issues, and adherence to project conventions."
capabilities = ["code-review", "architecture"]
```

## Open Items (Resolved)

| Item | Decision | Date |
|------|----------|------|
| CLI headless flags on BigBox | Yes -- all CLIs run non-interactively | 2026-03-06 |
| API budget for nightly compound review | Track session rate limits per provider; no fixed dollar ceiling | 2026-03-06 |
| Shared vs separate git worktrees | Shared worktree -- all agents work in same repo checkout | 2026-03-06 |

### Design Implications of Decisions

**Shared worktree**: Agents must coordinate file access. Use MCP Agent Mail `file_reservation_paths()` for exclusive file locks. Compound review creates branches, not worktrees.

**Rate limit tracking**: NightwatchMonitor gains a `RateLimitTracker` that counts API calls per agent per provider per hour. CompoundReviewWorkflow checks remaining budget before spawning tasks. Exposed via `AgentStatus::api_calls_remaining`.

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
