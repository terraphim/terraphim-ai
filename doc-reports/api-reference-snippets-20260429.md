# API Reference Snippets

**Generated:** 2026-04-29
**Agent:** documentation-generator (Ferrox)

## terraphim_orchestrator

### Core Types

```rust
/// The main orchestrator that runs the dark factory.
pub struct AgentOrchestrator { ... }

/// Status of a single agent in the fleet.
pub struct AgentStatus {
    pub name: String,
    pub layer: AgentLayer,
    pub running: bool,
    pub health: HealthStatus,
    pub drift_score: Option<f64>,
    pub uptime: Duration,
    pub restart_count: u32,
    pub api_calls_remaining: HashMap<String, Option<u32>>,
}

/// Result of evaluating a pre-check strategy before spawning an agent.
pub enum PreCheckResult {
    Findings(String),
    NoFindings,
    Failed(String),
}
```

### Key Methods on AgentOrchestrator

```rust
impl AgentOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError>;
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, OrchestratorError>;
    pub async fn run(&mut self) -> Result<(), OrchestratorError>;
    pub fn shutdown(&mut self);
    pub fn agent_statuses(&self) -> Vec<AgentStatus>;
    pub async fn trigger_compound_review(&mut self, git_ref: &str, base_ref: &str) -> Result<CompoundReviewResult, OrchestratorError>;
    pub async fn handoff(&mut self, from_agent: &str, to_agent: &str, context: HandoffContext) -> Result<(), OrchestratorError>;
}
```

### Re-exports

```rust
pub use agent_run_record::{AgentRunRecord, ExitClass, ExitClassification, ExitClassifier, RunTrigger};
pub use compound::{CompoundReviewResult, CompoundReviewWorkflow, ReviewGroupDef, SwarmConfig};
pub use concurrency::{ConcurrencyController, FairnessPolicy, ModeQuotas};
pub use config::{AgentDefinition, AgentLayer, CompoundReviewConfig, ConcurrencyConfig, GiteaOutputConfig, LearningConfig, MentionConfig, NightwatchConfig, OrchestratorConfig, PreCheckStrategy, TrackerConfig, TrackerStates, WebhookConfig, WorkflowConfig};
pub use cost_tracker::{AgentMetrics, BudgetVerdict, CostSnapshot, CostTracker, ExecutionMetrics};
pub use dispatcher::{DispatchTask, Dispatcher, DispatcherStats};
pub use dual_mode::DualModeOrchestrator;
pub use error::OrchestratorError;
pub use handoff::{HandoffBuffer, HandoffContext, HandoffLedger};
pub use mention::{migrate_legacy_mention_cursor, parse_mention_tokens, parse_mentions, resolve_mention, resolve_persona_mention, DetectedMention, MentionCursor, MentionTokens, MentionTracker};
pub use mention_chain::{MentionChainError, MentionChainTracker, MentionContextArgs, DEFAULT_MAX_MENTION_DEPTH};
pub use metrics_persistence::{InMemoryMetricsPersistence, MetricsPersistence, MetricsPersistenceConfig, MetricsPersistenceError, PersistedAgentMetrics};
pub use mode::{IssueMode, TimeMode};
pub use nightwatch::{dual_panel_evaluate, validate_certificate, Claim, CorrectionAction, CorrectionLevel, DriftAlert, DriftMetrics, DriftScore, DualPanelResult, NightwatchMonitor, RateLimitTracker, RateLimitWindow, ReasoningCertificate};
pub use output_poster::OutputPoster;
pub use persona::{MetapromptRenderError, MetapromptRenderer, PersonaRegistry};
pub use scheduler::{ScheduleEvent, TimeScheduler};
```

## terraphim_service

```rust
#[derive(thiserror::Error, Debug)]
pub enum ServiceError { ... }

pub type Result<T> = std::result::Result<T, ServiceError>;

pub struct TerraphimService { ... }
```

## terraphim_agent

```rust
pub use client::*;
pub use robot::{...};
pub use forgiving::{AliasRegistry, ForgivingParser, ParseResult};
pub use repl::*;
pub use commands::*;
```

## terraphim_types

Core types for documents, scores, and knowledge graph entities.

```rust
pub struct Document { ... }
pub struct Score { ... }
pub struct Role { ... }
```

---

*Note: These are extracted public API signatures. Full rustdoc generated documentation should be built with `cargo doc --workspace --no-deps`.*
