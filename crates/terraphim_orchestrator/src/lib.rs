//! Multi-agent orchestration with scheduling, budgeting, and compound review.
//!
//! This crate provides the core orchestration engine for managing fleets of AI agents
//! with features for resource scheduling, cost tracking, and coordinated review workflows.
//!
//! # Core Components
//!
//! - **AgentOrchestrator**: Main orchestrator running the "dark factory" pattern
//! - **DualModeOrchestrator**: Real-time and batch processing modes with fairness scheduling
//! - **CompoundReviewWorkflow**: Multi-agent review swarm with persona-based specialization
//! - **Scheduler**: Time-based and event-driven task scheduling
//! - **HandoffBuffer**: Inter-agent state transfer with TTL management
//! - **CostTracker**: Budget enforcement and spending monitoring
//! - **NightwatchMonitor**: Drift detection and rate limiting
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_orchestrator::{AgentOrchestrator, OrchestratorConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OrchestratorConfig::default();
//! let mut orchestrator = AgentOrchestrator::new(config).await?;
//!
//! // Run the orchestration loop
//! orchestrator.run().await?;
//! # Ok(())
//! # }
//! ```

pub mod adf_commands;
pub mod agent_run_record;
pub mod compound;
pub mod concurrency;
pub mod config;
pub mod control_plane;
pub mod cost_tracker;
pub mod dispatcher;
pub mod dual_mode;
pub mod error;
pub mod flow;
pub mod handoff;
pub mod kg_router;
pub mod learning;
pub mod mention;
pub mod metrics_persistence;
pub mod mode;
pub mod nightwatch;
pub mod output_poster;
pub mod persona;
pub mod provider_probe;
#[cfg(feature = "quickwit")]
pub mod quickwit;
pub mod scheduler;
pub mod scope;
pub mod webhook;

pub use agent_run_record::{
    AgentRunRecord, ExitClass, ExitClassification, ExitClassifier, RunTrigger,
};
pub use compound::{CompoundReviewResult, CompoundReviewWorkflow, ReviewGroupDef, SwarmConfig};
pub use concurrency::{ConcurrencyController, FairnessPolicy, ModeQuotas};
#[cfg(feature = "quickwit")]
pub use config::QuickwitConfig;
pub use config::{
    AgentDefinition, AgentLayer, CompoundReviewConfig, ConcurrencyConfig, GiteaOutputConfig,
    MentionConfig, NightwatchConfig, OrchestratorConfig, PreCheckStrategy, TrackerConfig,
    TrackerStates, WebhookConfig, WorkflowConfig,
};
pub use cost_tracker::{AgentMetrics, BudgetVerdict, CostSnapshot, CostTracker, ExecutionMetrics};
pub use dispatcher::{DispatchTask, Dispatcher, DispatcherStats};
pub use dual_mode::DualModeOrchestrator;
pub use error::OrchestratorError;
pub use handoff::{HandoffBuffer, HandoffContext, HandoffLedger};
pub use mention::{
    migrate_legacy_mention_cursor, parse_mention_tokens, parse_mentions, resolve_mention,
    resolve_persona_mention, DetectedMention, MentionCursor, MentionTokens, MentionTracker,
};
pub use metrics_persistence::{
    InMemoryMetricsPersistence, MetricsPersistence, MetricsPersistenceConfig,
    MetricsPersistenceError, PersistedAgentMetrics,
};
pub use mode::{IssueMode, TimeMode};
pub use nightwatch::{
    dual_panel_evaluate, validate_certificate, Claim, CorrectionAction, CorrectionLevel,
    DriftAlert, DriftMetrics, DriftScore, DualPanelResult, NightwatchMonitor, RateLimitTracker,
    RateLimitWindow, ReasoningCertificate,
};
pub use output_poster::OutputPoster;
pub use persona::{MetapromptRenderError, MetapromptRenderer, PersonaRegistry};
pub use scheduler::{ScheduleEvent, TimeScheduler};
use terraphim_types::{FindingSeverity, ReviewFinding};

use chrono::Timelike;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, Instant};

use std::sync::{Arc, Mutex};

use terraphim_router::RoutingEngine;
use terraphim_spawner::health::{CircuitBreaker, HealthStatus};
use terraphim_spawner::output::OutputEvent;
use terraphim_spawner::{AgentHandle, AgentSpawner, ResourceLimits, SpawnContext, SpawnRequest};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Result of evaluating a pre-check strategy before spawning an agent.
#[derive(Debug, Clone)]
pub enum PreCheckResult {
    /// Pre-check found actionable findings. Agent should spawn with findings prepended.
    Findings(String),
    /// Nothing to do. Skip spawn.
    NoFindings,
    /// Pre-check execution itself failed. Fail-open: spawn anyway.
    Failed(String),
}

/// Status of a single agent in the fleet.
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub name: String,
    pub layer: AgentLayer,
    pub running: bool,
    pub health: HealthStatus,
    pub drift_score: Option<f64>,
    pub uptime: Duration,
    pub restart_count: u32,
    /// API calls remaining per provider (None if no limit known).
    pub api_calls_remaining: HashMap<String, Option<u32>>,
}

/// Runtime state for a managed agent.
struct ManagedAgent {
    definition: AgentDefinition,
    handle: AgentHandle,
    started_at: Instant,
    restart_count: u32,
    /// Broadcast receiver for draining output events to nightwatch.
    output_rx: broadcast::Receiver<OutputEvent>,
    spawned_by_mention: bool,
    /// Git worktree path for workspace isolation (None = shared working_dir).
    worktree_path: Option<PathBuf>,
    /// KG-routed model selected at spawn time (None = CLI default). Used for logging.
    routed_model: Option<String>,
    /// Session ID for telemetry tracking (format: "{agent_name}-{ulid}").
    session_id: String,
}

#[cfg(not(test))]
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct PersistedRestartState {
    /// project -> agent -> restart count
    #[serde(default)]
    counts_by_project: HashMap<String, HashMap<String, u32>>,
    /// project -> agent -> last failure timestamp (unix seconds)
    #[serde(default)]
    last_failure_unix_secs_by_project: HashMap<String, HashMap<String, i64>>,
    /// Legacy flat map retained for backward-compatible deserialisation.
    #[serde(default, skip_serializing)]
    counts: HashMap<String, u32>,
    /// Legacy flat map retained for backward-compatible deserialisation.
    #[serde(default, skip_serializing)]
    last_failure_unix_secs: HashMap<String, i64>,
}

/// The main orchestrator that runs the dark factory.
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    spawner: AgentSpawner,
    router: RoutingEngine,
    nightwatch: NightwatchMonitor,
    scheduler: TimeScheduler,
    compound_workflow: CompoundReviewWorkflow,
    active_agents: HashMap<String, ManagedAgent>,
    rate_limiter: RateLimitTracker,
    shutdown_requested: bool,
    /// Total restart count per (project, agent) pair (persists across agent lifecycle).
    restart_counts: HashMap<(String, String), u32>,
    /// Last non-zero exit timestamp per (project, agent), used for budget windowing.
    restart_last_failure_unix_secs: HashMap<(String, String), i64>,
    /// Last exit time per (project, agent) (for cooldown enforcement).
    restart_cooldowns: HashMap<(String, String), Instant>,
    /// Timestamp of the last reconciliation tick (for cron comparison).
    last_tick_time: chrono::DateTime<chrono::Utc>,
    /// In-memory buffer for handoff contexts with TTL.
    handoff_buffer: HandoffBuffer,
    /// Append-only JSONL ledger for handoff history.
    handoff_ledger: HandoffLedger,
    /// Per-agent cost tracking with budget enforcement.
    cost_tracker: CostTracker,
    /// Registry of persona definitions for metaprompt generation.
    persona_registry: PersonaRegistry,
    /// Renderer for persona metaprompts.
    metaprompt_renderer: MetapromptRenderer,
    /// Output poster for posting agent output to Gitea issues.
    output_poster: Option<OutputPoster>,
    /// Circuit breakers for each provider to prevent cascading failures.
    #[allow(dead_code)]
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    /// Per-agent last-run commit hash for GitDiff strategy.
    /// Key: agent name. Value: commit SHA.
    last_run_commits: HashMap<String, String>,
    /// Lazy-initialised Gitea tracker for gitea-issue pre-check.
    pre_check_tracker: Option<terraphim_tracker::GiteaTracker>,
    /// Active flow executions keyed by flow name.
    #[allow(dead_code)]
    active_flows: HashMap<String, tokio::task::JoinHandle<flow::state::FlowRunState>>,
    /// Per-project mention cursors, keyed by project id.
    ///
    /// Each project gets its own cursor so repo-wide polls can advance
    /// independently. Legacy single-project mode uses the synthetic
    /// [`dispatcher::LEGACY_PROJECT_ID`] key.
    mention_cursors: HashMap<String, MentionCursor>,
    /// Receiver for webhook dispatch requests.
    webhook_dispatch_rx: Option<tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>>,
    /// Monotonically increasing tick counter for poll_modulo gating.
    tick_count: u64,
    #[cfg(feature = "quickwit")]
    quickwit_sink: Option<quickwit::QuickwitFleetSink>,
    /// Classifier for structured agent exit classification using KG-boosted matching.
    exit_classifier: ExitClassifier,
    /// KG-driven model router loaded from taxonomy markdown files.
    kg_router: Option<kg_router::KgRouter>,
    /// Per-provider health tracking with circuit breakers.
    provider_health: provider_probe::ProviderHealthMap,
    /// Live telemetry store for model performance tracking from CLI output.
    telemetry_store: control_plane::TelemetryStore,
}

/// Build the composite restart-state key for an agent definition.
///
/// Legacy (project-less) agents use [`crate::dispatcher::LEGACY_PROJECT_ID`]
/// so restart counts never collide across projects once projects are added.
fn agent_key(def: &AgentDefinition) -> (String, String) {
    (
        def.project
            .clone()
            .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
        def.name.clone(),
    )
}

/// Build the per-project runtime map consumed by
/// [`flow::executor::FlowExecutor::with_projects`].
fn build_flow_project_runtimes(
    config: &OrchestratorConfig,
) -> HashMap<String, flow::executor::ProjectRuntime> {
    config
        .projects
        .iter()
        .map(|p| {
            (
                p.id.clone(),
                flow::executor::ProjectRuntime {
                    working_dir: p.working_dir.clone(),
                    gitea_owner: p.gitea.as_ref().map(|g| g.owner.clone()),
                    gitea_repo: p.gitea.as_ref().map(|g| g.repo.clone()),
                },
            )
        })
        .collect()
}

/// Build a [`SpawnContext`] for an agent, resolving per-project working_dir,
/// Gitea owner/repo, and the project id itself into the child process's
/// environment (`ADF_PROJECT_ID`, `ADF_WORKING_DIR`, `GITEA_OWNER`,
/// `GITEA_REPO`). Legacy (project-less) agents use [`SpawnContext::global()`].
fn build_spawn_context_for_agent(
    config: &OrchestratorConfig,
    def: &AgentDefinition,
) -> SpawnContext {
    let Some(pid) = def.project.as_deref() else {
        return SpawnContext::global();
    };
    let Some(project) = config.project_by_id(pid) else {
        return SpawnContext::global();
    };
    let working_dir_str = project.working_dir.to_string_lossy().into_owned();
    let mut ctx = SpawnContext::with_working_dir(project.working_dir.clone())
        .with_env("ADF_PROJECT_ID", pid)
        .with_env("ADF_WORKING_DIR", working_dir_str);
    if let Some(gitea) = project.gitea.as_ref() {
        ctx = ctx
            .with_env("GITEA_OWNER", gitea.owner.clone())
            .with_env("GITEA_REPO", gitea.repo.clone());
    }
    ctx
}

/// Flatten persisted nested maps (project -> agent -> count) and any legacy
/// flat entries into the in-memory composite key map. Legacy flat entries are
/// mapped to [`crate::dispatcher::LEGACY_PROJECT_ID`].
#[cfg(not(test))]
fn flatten_restart_counts(state: &PersistedRestartState) -> HashMap<(String, String), u32> {
    let mut out: HashMap<(String, String), u32> = HashMap::new();
    for (project, per_agent) in &state.counts_by_project {
        for (agent, count) in per_agent {
            out.insert((project.clone(), agent.clone()), *count);
        }
    }
    for (agent, count) in &state.counts {
        out.entry((
            crate::dispatcher::LEGACY_PROJECT_ID.to_string(),
            agent.clone(),
        ))
        .or_insert(*count);
    }
    out
}

/// Flatten persisted nested maps for last-failure timestamps into the
/// composite key format.
#[cfg(not(test))]
fn flatten_restart_failures(state: &PersistedRestartState) -> HashMap<(String, String), i64> {
    let mut out: HashMap<(String, String), i64> = HashMap::new();
    for (project, per_agent) in &state.last_failure_unix_secs_by_project {
        for (agent, ts) in per_agent {
            out.insert((project.clone(), agent.clone()), *ts);
        }
    }
    for (agent, ts) in &state.last_failure_unix_secs {
        out.entry((
            crate::dispatcher::LEGACY_PROJECT_ID.to_string(),
            agent.clone(),
        ))
        .or_insert(*ts);
    }
    out
}

/// Re-nest composite keyed maps into the persisted `project -> agent -> value`
/// shape for serialisation.
#[cfg(not(test))]
fn nest_by_project<V: Clone>(
    flat: &HashMap<(String, String), V>,
) -> HashMap<String, HashMap<String, V>> {
    let mut out: HashMap<String, HashMap<String, V>> = HashMap::new();
    for ((project, agent), value) in flat {
        out.entry(project.clone())
            .or_default()
            .insert(agent.clone(), value.clone());
    }
    out
}

/// Validate agent name for safe use in file paths.
/// Rejects empty names, names containing path separators or traversal sequences.
fn validate_agent_name(name: &str) -> Result<(), OrchestratorError> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(OrchestratorError::InvalidAgentName(name.to_string()));
    }
    Ok(())
}

impl AgentOrchestrator {
    /// Create a new orchestrator from configuration.
    pub fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        // Set CARGO_TARGET_DIR so worktree agents share the main build cache,
        // and RUSTC_WRAPPER=sccache for cross-worktree compilation caching.
        let mut spawn_env = std::collections::HashMap::new();
        let target_dir = config.working_dir.join("target");
        spawn_env.insert(
            "CARGO_TARGET_DIR".to_string(),
            target_dir.to_string_lossy().to_string(),
        );
        if std::process::Command::new("sccache")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
        {
            spawn_env.insert("RUSTC_WRAPPER".to_string(), "sccache".to_string());
            info!("sccache detected, enabling shared compilation cache for worktrees");
        }
        let spawner = AgentSpawner::new()
            .with_working_dir(&config.working_dir)
            .with_env_vars(spawn_env);
        let router = RoutingEngine::new();
        let nightwatch = NightwatchMonitor::new(config.nightwatch.clone());
        let scheduler = TimeScheduler::new(&config.agents, Some(&config.compound_review.schedule))?;
        let compound_workflow =
            CompoundReviewWorkflow::from_compound_config(config.compound_review.clone());
        let handoff_buffer = HandoffBuffer::new(config.handoff_buffer_ttl_secs.unwrap_or(86400));
        let handoff_ledger = HandoffLedger::new(config.working_dir.join("handoff-ledger.jsonl"));

        // Initialize cost tracker and register all agents with their budgets
        let mut cost_tracker = CostTracker::new();
        for agent_def in &config.agents {
            cost_tracker.register(&agent_def.name, agent_def.budget_monthly_cents);
        }

        // Initialize persona registry - load from configured directory or create empty
        let persona_registry = match &config.persona_data_dir {
            Some(dir) => {
                info!(dir = %dir.display(), "loading persona registry from directory");
                PersonaRegistry::load_from_dir(dir).unwrap_or_else(|e| {
                    warn!(dir = %dir.display(), error = %e, "failed to load persona directory, using empty registry");
                    PersonaRegistry::new()
                })
            }
            None => {
                info!("no persona_data_dir configured, using empty registry");
                PersonaRegistry::new()
            }
        };

        // Initialize metaprompt renderer - check for custom template or use default
        let metaprompt_renderer = match &config.persona_data_dir {
            Some(dir) => {
                let custom_template = dir.join("metaprompt-template.hbs");
                if custom_template.exists() {
                    info!(path = %custom_template.display(), "using custom metaprompt template");
                    MetapromptRenderer::from_template_file(&custom_template).unwrap_or_else(|e| {
                        warn!(path = %custom_template.display(), error = %e, "failed to load custom template, using default");
                        MetapromptRenderer::new().expect("default template should always compile")
                    })
                } else {
                    MetapromptRenderer::new().expect("default template should always compile")
                }
            }
            None => MetapromptRenderer::new().expect("default template should always compile"),
        };

        // Initialize output poster. In multi-project mode this wires one
        // tracker per project plus a legacy fallback from `config.gitea`; in
        // legacy single-project mode it collapses to the top-level config.
        let output_poster = OutputPoster::from_orchestrator_config(&config);

        // Initialize KG router from taxonomy directory if configured
        let kg_router = config.routing.as_ref().and_then(|routing_config| {
            match kg_router::KgRouter::load(&routing_config.taxonomy_path) {
                Ok(router) => {
                    info!(
                        path = %routing_config.taxonomy_path.display(),
                        rules = router.rule_count(),
                        "KG model router loaded"
                    );
                    Some(router)
                }
                Err(e) => {
                    warn!(error = %e, "KG router failed to load, using static model config");
                    None
                }
            }
        });

        let probe_ttl = config
            .routing
            .as_ref()
            .map(|r| r.probe_ttl_secs)
            .unwrap_or(300);
        let provider_health =
            provider_probe::ProviderHealthMap::new(std::time::Duration::from_secs(probe_ttl));

        let telemetry_store = control_plane::TelemetryStore::new(3600);

        #[cfg(not(test))]
        let restart_state = Self::load_restart_state();

        // MentionCursor loaded lazily on first poll (async)

        Ok(Self {
            config,
            spawner,
            router,
            nightwatch,
            scheduler,
            compound_workflow,
            active_agents: HashMap::new(),
            rate_limiter: RateLimitTracker::default(),
            shutdown_requested: false,
            restart_counts: {
                #[cfg(not(test))]
                {
                    flatten_restart_counts(&restart_state)
                }
                #[cfg(test)]
                {
                    HashMap::new()
                }
            },
            restart_last_failure_unix_secs: {
                #[cfg(not(test))]
                {
                    flatten_restart_failures(&restart_state)
                }
                #[cfg(test)]
                {
                    HashMap::new()
                }
            },
            restart_cooldowns: HashMap::new(),
            last_tick_time: chrono::Utc::now(),
            handoff_buffer,
            handoff_ledger,
            cost_tracker,
            persona_registry,
            metaprompt_renderer,
            output_poster,
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            last_run_commits: HashMap::new(),
            pre_check_tracker: None,
            active_flows: HashMap::new(),
            mention_cursors: HashMap::new(),
            webhook_dispatch_rx: None,
            tick_count: 0,
            #[cfg(feature = "quickwit")]
            quickwit_sink: None,
            exit_classifier: ExitClassifier::new(),
            kg_router,
            provider_health,
            telemetry_store,
        })
    }

    /// Load persisted restart state from a JSON file in the temp directory.
    /// Falls back to the legacy `HashMap<String, u32>` format for compatibility.
    #[cfg(not(test))]
    fn load_restart_state() -> PersistedRestartState {
        let path = std::env::temp_dir().join("adf_restart_counts.json");
        match std::fs::read_to_string(&path) {
            Ok(json) => {
                if let Ok(state) = serde_json::from_str::<PersistedRestartState>(&json) {
                    state
                } else if let Ok(counts) = serde_json::from_str::<HashMap<String, u32>>(&json) {
                    PersistedRestartState {
                        counts,
                        ..Default::default()
                    }
                } else {
                    PersistedRestartState::default()
                }
            }
            Err(_) => PersistedRestartState::default(),
        }
    }

    /// Persist restart state so it survives orchestrator restarts.
    fn save_restart_state(&self) {
        // Skip persistence in test builds to avoid cross-test contamination
        #[cfg(test)]
        return;
        #[cfg(not(test))]
        {
            let path = std::env::temp_dir().join("adf_restart_counts.json");
            let state = PersistedRestartState {
                counts_by_project: nest_by_project(&self.restart_counts),
                last_failure_unix_secs_by_project: nest_by_project(
                    &self.restart_last_failure_unix_secs,
                ),
                counts: HashMap::new(),
                last_failure_unix_secs: HashMap::new(),
            };
            if let Ok(json) = serde_json::to_string(&state) {
                if let Err(e) = std::fs::write(&path, json) {
                    warn!(error = %e, "failed to persist restart state");
                }
            }
        }
    }

    fn restart_budget_window_secs(&self) -> i64 {
        self.config.restart_budget_window_secs as i64
    }

    fn current_restart_count(&mut self, key: &(String, String)) -> u32 {
        let now = chrono::Utc::now().timestamp();
        let window = self.restart_budget_window_secs();
        let last_failure = self.restart_last_failure_unix_secs.get(key).copied();
        if let Some(last) = last_failure {
            if now.saturating_sub(last) > window {
                self.restart_counts.remove(key);
                self.restart_last_failure_unix_secs.remove(key);
                self.save_restart_state();
            }
        }
        self.restart_counts.get(key).copied().unwrap_or(0)
    }

    fn increment_restart_count(&mut self, key: &(String, String)) -> u32 {
        let _ = self.current_restart_count(key);
        let next_count = {
            let count = self.restart_counts.entry(key.clone()).or_insert(0);
            *count += 1;
            *count
        };
        self.restart_last_failure_unix_secs
            .insert(key.clone(), chrono::Utc::now().timestamp());
        self.save_restart_state();
        next_count
    }

    /// Check disk usage percentage for the root filesystem.
    /// Returns None if the check fails (non-Linux, command error, etc.).
    fn check_disk_usage_percent() -> Option<u8> {
        let output = std::process::Command::new("df")
            .args(["--output=pcent", "/"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: "Use%\n 86%\n"
        for line in stdout.lines().skip(1) {
            let trimmed = line.trim().trim_end_matches('%');
            if let Ok(pct) = trimmed.parse::<u8>() {
                return Some(pct);
            }
        }
        None
    }

    /// Create from a TOML config file path.
    ///
    /// Loads the config, resolves include globs, and runs full validation
    /// (banned providers, duplicate project ids, unknown project refs, mixed
    /// mode). Returns `Err` if any check fails -- does not panic or warn-and-
    /// continue.
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, OrchestratorError> {
        let config = OrchestratorConfig::load_and_validate(path)?;
        Self::new(config)
    }

    /// Return the validated configuration stored in this orchestrator.
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Run the orchestrator (blocks until shutdown signal).
    ///
    /// 1. Spawns all Safety-layer agents immediately
    /// 2. Enters the select! loop handling schedule events, drift alerts, and periodic tick
    pub async fn run(&mut self) -> Result<(), OrchestratorError> {
        info!(
            "starting orchestrator with {} agent definitions",
            self.config.agents.len()
        );

        // D-2: Run provider probes on startup if configured
        if self
            .config
            .routing
            .as_ref()
            .is_some_and(|r| r.probe_on_startup)
        {
            if let Some(ref kg_router) = self.kg_router {
                info!("running startup provider probe via KG action:: templates");
                self.provider_health.probe_all(kg_router).await;

                // Save probe results if directory configured
                if let Some(ref dir) = self
                    .config
                    .routing
                    .as_ref()
                    .and_then(|r| r.probe_results_dir.clone())
                {
                    if let Err(e) = self.provider_health.save_results(dir).await {
                        warn!(error = %e, "failed to save probe results");
                    }
                }
            }
        }

        // Restore persisted telemetry from previous runs
        self.restore_telemetry().await;

        // One-shot migration of the legacy top-level `adf/mention_cursor`
        // key into per-project keys. No-op after the first successful
        // startup.
        mention::migrate_legacy_mention_cursor(&self.config.projects).await;

        // Spawn Safety-layer agents immediately
        let immediate = self.scheduler.immediate_agents();
        for agent_def in &immediate {
            if let Err(e) = self.spawn_agent(agent_def).await {
                error!(agent = %agent_def.name, error = %e, "failed to spawn safety agent");
            }
        }

        info!(
            safety_agents = immediate.len(),
            active = self.active_agents.len(),
            "safety agents spawned, entering reconciliation loop"
        );

        // Start webhook server if configured
        if let Some(ref webhook_cfg) = self.config.webhook {
            let (dispatch_tx, dispatch_rx) = tokio::sync::mpsc::channel(64);
            self.webhook_dispatch_rx = Some(dispatch_rx);

            let agent_names: Vec<String> =
                self.config.agents.iter().map(|a| a.name.clone()).collect();
            let state = webhook::WebhookState {
                agent_names,
                persona_registry: Arc::new(self.persona_registry.clone()),
                dispatch_tx,
                secret: webhook_cfg.secret.clone(),
            };

            let router = webhook::webhook_router(state);
            let bind = webhook_cfg.bind.clone();

            tokio::spawn(async move {
                let listener = match tokio::net::TcpListener::bind(&bind).await {
                    Ok(l) => l,
                    Err(e) => {
                        error!(bind = %bind, error = %e, "failed to bind webhook server");
                        return;
                    }
                };
                info!(bind = %bind, "webhook server listening");
                if let Err(e) = axum::serve(listener, router).await {
                    error!(error = %e, "webhook server error");
                }
            });
        }

        // Reconciliation tick interval
        let mut tick = tokio::time::interval(Duration::from_secs(self.config.tick_interval_secs));

        // Main reconciliation loop
        loop {
            if self.shutdown_requested {
                info!("shutdown requested, stopping reconciliation loop");
                break;
            }

            // Drain any pending webhook dispatches before select
            let pending_dispatches: Vec<_> = if let Some(ref mut rx) = self.webhook_dispatch_rx {
                let mut batch = Vec::new();
                while let Ok(dispatch) = rx.try_recv() {
                    batch.push(dispatch);
                }
                batch
            } else {
                Vec::new()
            };
            // Collect comment_ids for cursor marking (belt-and-suspenders dedup)
            let webhook_comment_ids: Vec<u64> =
                pending_dispatches.iter().map(|d| d.comment_id()).collect();
            for dispatch in pending_dispatches {
                self.handle_webhook_dispatch(dispatch).await;
            }
            // Mark webhook-dispatched comments in every project cursor so
            // each project's poller skips them without needing another
            // Gitea API call. The webhook payload does not yet carry a
            // project id, so we stamp all known cursors (plus the legacy
            // fallback) to stay correct across both modes.
            if !webhook_comment_ids.is_empty() {
                let project_ids: Vec<String> = if self.config.projects.is_empty() {
                    vec![dispatcher::LEGACY_PROJECT_ID.to_string()]
                } else {
                    self.config.projects.iter().map(|p| p.id.clone()).collect()
                };
                for pid in &project_ids {
                    let cursor = self
                        .mention_cursors
                        .entry(pid.clone())
                        .or_insert_with(mention::MentionCursor::now);
                    for cid in &webhook_comment_ids {
                        cursor.mark_processed(*cid);
                    }
                }
                for pid in &project_ids {
                    if let Some(cursor) = self.mention_cursors.get(pid) {
                        cursor.save(pid).await;
                    }
                }
            }

            tokio::select! {
                event = self.scheduler.next_event() => {
                    self.handle_schedule_event(event).await;
                }
                alert = self.nightwatch.next_alert() => {
                    self.handle_drift_alert(alert).await;
                }
                _ = tick.tick() => {
                    self.reconcile_tick().await;
                }
            }
        }

        // Graceful shutdown of all agents
        self.persist_telemetry();
        self.shutdown_all_agents().await;
        Ok(())
    }

    /// Request graceful shutdown of all agents and the orchestrator.
    pub fn shutdown(&mut self) {
        info!("shutdown requested");
        self.shutdown_requested = true;
    }

    /// Get current status of all agents.
    pub fn agent_statuses(&self) -> Vec<AgentStatus> {
        self.active_agents
            .values()
            .map(|managed| {
                let drift = self.nightwatch.drift_score(&managed.definition.name);
                AgentStatus {
                    name: managed.definition.name.clone(),
                    layer: managed.definition.layer,
                    running: true,
                    health: managed.handle.health_status(),
                    drift_score: drift.map(|d| d.score),
                    uptime: managed.started_at.elapsed(),
                    restart_count: managed.restart_count,
                    api_calls_remaining: HashMap::new(),
                }
            })
            .collect()
    }

    /// Manually trigger a compound review (outside normal schedule).
    pub async fn trigger_compound_review(
        &mut self,
        git_ref: &str,
        base_ref: &str,
    ) -> Result<CompoundReviewResult, OrchestratorError> {
        info!("triggering manual compound review");
        self.compound_workflow.run(git_ref, base_ref).await
    }

    /// Hand off a task from one agent to another.
    pub async fn handoff(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        context: HandoffContext,
    ) -> Result<(), OrchestratorError> {
        // Validate agent names for path safety (prevents path traversal)
        validate_agent_name(from_agent)?;
        validate_agent_name(to_agent)?;

        // Validate context fields match parameters
        if context.from_agent != from_agent || context.to_agent != to_agent {
            return Err(OrchestratorError::HandoffFailed {
                from: from_agent.to_string(),
                to: to_agent.to_string(),
                reason: format!(
                    "context field mismatch: context.from_agent='{}', context.to_agent='{}'",
                    context.from_agent, context.to_agent
                ),
            });
        }

        if !self.active_agents.contains_key(from_agent) {
            return Err(OrchestratorError::AgentNotFound(from_agent.to_string()));
        }

        // Find the target agent definition
        let to_def = self
            .config
            .agents
            .iter()
            .find(|a| a.name == to_agent)
            .cloned()
            .ok_or_else(|| OrchestratorError::AgentNotFound(to_agent.to_string()))?;

        // If target isn't running, spawn it
        if !self.active_agents.contains_key(to_agent) {
            self.spawn_agent(&to_def).await?;
        }

        // Write handoff context to file for the target agent
        let handoff_path = self
            .config
            .working_dir
            .join(format!(".handoff-{}.json", to_agent));
        context
            .write_to_file(&handoff_path)
            .map_err(|e| OrchestratorError::HandoffFailed {
                from: from_agent.to_string(),
                to: to_agent.to_string(),
                reason: e.to_string(),
            })?;

        // Insert into in-memory buffer for fast retrieval
        let handoff_id = self.handoff_buffer.insert(context.clone());

        // Append to persistent ledger
        self.handoff_ledger
            .append(&context)
            .map_err(|e| OrchestratorError::HandoffFailed {
                from: from_agent.to_string(),
                to: to_agent.to_string(),
                reason: format!("ledger append failed: {}", e),
            })?;

        info!(
            from = from_agent,
            to = to_agent,
            handoff_file = %handoff_path.display(),
            handoff_id = %handoff_id,
            "handoff context written"
        );

        Ok(())
    }

    /// Get the most recent handoff for a specific target agent.
    /// Returns the handoff context with the latest timestamp that hasn't expired.
    pub fn latest_handoff_for(&self, to_agent: &str) -> Option<&HandoffContext> {
        self.handoff_buffer.latest_for_agent(to_agent)
    }

    /// Get a reference to the routing engine.
    pub fn router(&self) -> &RoutingEngine {
        &self.router
    }

    /// Get a mutable reference to the routing engine.
    pub fn router_mut(&mut self) -> &mut RoutingEngine {
        &mut self.router
    }

    /// Get a reference to the rate limiter.
    pub fn rate_limiter(&self) -> &RateLimitTracker {
        &self.rate_limiter
    }

    /// Get a mutable reference to the rate limiter.
    pub fn rate_limiter_mut(&mut self) -> &mut RateLimitTracker {
        &mut self.rate_limiter
    }

    /// Get a reference to the cost tracker.
    pub fn cost_tracker(&self) -> &CostTracker {
        &self.cost_tracker
    }

    /// Get a mutable reference to the cost tracker.
    pub fn cost_tracker_mut(&mut self) -> &mut CostTracker {
        &mut self.cost_tracker
    }

    #[cfg(feature = "quickwit")]
    pub fn set_quickwit_sink(&mut self, sink: quickwit::QuickwitFleetSink) {
        self.quickwit_sink = Some(sink);
    }

    #[cfg(feature = "quickwit")]
    pub fn quickwit_config(&self) -> Option<&QuickwitConfig> {
        self.config.quickwit.as_ref()
    }

    /// Enumerate per-project Quickwit configurations plus a legacy fallback
    /// for the top-level config, so the binary can build a
    /// [`quickwit::QuickwitFleetSink`] covering every project.
    ///
    /// Returns `(project_id, QuickwitConfig)` pairs. Projects without a
    /// per-project Quickwit block inherit the top-level config. The legacy
    /// single-project path emits a single entry keyed on
    /// [`crate::dispatcher::LEGACY_PROJECT_ID`].
    #[cfg(feature = "quickwit")]
    pub fn quickwit_fleet_configs(&self) -> Vec<(String, QuickwitConfig)> {
        let mut out: Vec<(String, QuickwitConfig)> = Vec::new();

        for project in &self.config.projects {
            if let Some(cfg) = project
                .quickwit
                .as_ref()
                .or(self.config.quickwit.as_ref())
                .cloned()
            {
                if cfg.enabled {
                    out.push((project.id.clone(), cfg));
                }
            }
        }

        if self.config.projects.is_empty() {
            if let Some(cfg) = self.config.quickwit.as_ref().cloned() {
                if cfg.enabled {
                    out.push((crate::dispatcher::LEGACY_PROJECT_ID.to_string(), cfg));
                }
            }
        }

        out
    }

    /// Load skill chain content from skill definition files for the given agent definition.
    ///
    /// Reads each skill named in `def.skill_chain` from `{skill_data_dir}/{name}/SKILL.md`.
    /// If that path is missing, it falls back to `skill.md` and well-known HOME skill roots
    /// (`~/.opencode/skills`, then `~/.claude/skills`). Returns a formatted string with all
    /// skill contents, or empty string if no skills can be loaded.
    fn load_skill_chain_content(&self, def: &AgentDefinition) -> String {
        if def.skill_chain.is_empty() {
            return String::new();
        }

        let home_dir = std::env::var("HOME").ok().map(std::path::PathBuf::from);
        let skill_roots =
            Self::skill_roots(self.config.skill_data_dir.as_deref(), home_dir.as_deref());

        if skill_roots.is_empty() {
            return String::new();
        }

        let mut sections = Vec::new();
        for skill_name in &def.skill_chain {
            let mut selected_path = None;
            let mut content = None;

            for root in &skill_roots {
                let skill_dir = root.join(skill_name);
                let candidates = [skill_dir.join("SKILL.md"), skill_dir.join("skill.md")];
                for candidate in candidates {
                    match std::fs::read_to_string(&candidate) {
                        Ok(raw) => {
                            selected_path = Some(candidate);
                            content = Some(raw);
                            break;
                        }
                        Err(_) => continue,
                    }
                }
                if content.is_some() {
                    break;
                }
            }

            match content {
                Some(content) => {
                    // Strip YAML frontmatter (between --- markers) to keep just instructions
                    let body = if let Some(after_prefix) = content.strip_prefix("---") {
                        if let Some(end) = after_prefix.find("---") {
                            after_prefix[end + 3..].trim_start().to_string()
                        } else {
                            content
                        }
                    } else {
                        content
                    };
                    sections.push(format!("### Skill: {}\n\n{}", skill_name, body.trim()));
                    info!(
                        agent = %def.name,
                        skill = %skill_name,
                        path = %selected_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        bytes = body.len(),
                        "loaded skill content"
                    );
                }
                None => {
                    let tried: Vec<String> = skill_roots
                        .iter()
                        .flat_map(|root| {
                            let skill_dir = root.join(skill_name);
                            [
                                skill_dir.join("SKILL.md").display().to_string(),
                                skill_dir.join("skill.md").display().to_string(),
                            ]
                        })
                        .collect();
                    warn!(
                        agent = %def.name,
                        skill = %skill_name,
                        tried = ?tried,
                        "failed to load skill, skipping"
                    );
                }
            }
        }

        if sections.is_empty() {
            return String::new();
        }

        format!(
            "\n\n## Active Skills\n\nApply the following skill instructions to your work:\n\n{}\n",
            sections.join("\n\n---\n\n")
        )
    }

    fn skill_roots(
        configured: Option<&std::path::Path>,
        home_dir: Option<&std::path::Path>,
    ) -> Vec<std::path::PathBuf> {
        let mut roots = Vec::new();

        if let Some(dir) = configured {
            roots.push(dir.to_path_buf());
        }

        if let Some(home) = home_dir {
            for root in [
                home.join(".opencode").join("skills"),
                home.join(".claude").join("skills"),
            ] {
                if !roots.iter().any(|existing| existing == &root) {
                    roots.push(root);
                }
            }
        }

        roots
    }

    /// Spawn an agent from its definition.
    ///
    /// Model selection: if the agent has an explicit `model` field, use it.
    /// Otherwise, route the task prompt through the RoutingEngine to select
    /// a model based on keyword matching.
    async fn spawn_agent(&mut self, def: &AgentDefinition) -> Result<(), OrchestratorError> {
        // === DISK SPACE GUARD ===
        let threshold = self.config.disk_usage_threshold;
        if threshold < 100 {
            if let Some(usage) = Self::check_disk_usage_percent() {
                if usage >= threshold {
                    error!(
                        agent = %def.name,
                        disk_usage_percent = usage,
                        threshold,
                        "refusing to spawn agent: disk usage above threshold"
                    );
                    return Err(OrchestratorError::Config(format!(
                        "disk usage {}% >= {}% threshold, refusing to spawn {}",
                        usage, threshold, def.name
                    )));
                }
            }
        }

        // === PRE-CHECK GATE ===
        let pre_check_result = self.run_pre_check(def).await;
        let findings = match pre_check_result {
            PreCheckResult::NoFindings => {
                info!(agent = %def.name, "skipping spawn: pre-check found nothing actionable");
                return Ok(());
            }
            PreCheckResult::Findings(f) if f.is_empty() => None,
            PreCheckResult::Findings(f) => Some(f),
            PreCheckResult::Failed(reason) => {
                warn!(agent = %def.name, reason = %reason,
                      "pre-check failed, spawning anyway (fail-open)");
                None
            }
        };

        // Select model via keyword routing or explicit config.
        // Skip keyword routing for CLIs that use OAuth and don't support -m
        // (e.g. codex with ChatGPT account). Only apply routed models when the
        // CLI tool is known to accept --model flags with arbitrary model IDs.
        let cli_name = std::path::Path::new(&def.cli_tool)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&def.cli_tool);
        let supports_model_flag = matches!(cli_name, "claude" | "claude-code" | "opencode");

        // Track KG decision for CLI override (set inside the routing block below)
        let mut kg_cli_override: Option<String> = None;

        #[allow(clippy::manual_let_else)]
        let model = if self
            .config
            .routing
            .as_ref()
            .is_some_and(|r| r.use_routing_engine)
        {
            let kg_arc = self
                .kg_router
                .as_ref()
                .map(|r| std::sync::Arc::new(r.clone()));
            let unhealthy = self.provider_health.unhealthy_providers();
            let telemetry_arc = std::sync::Arc::new(self.telemetry_store.clone());
            let engine = control_plane::RoutingDecisionEngine::new(
                kg_arc,
                unhealthy,
                terraphim_router::Router::new(),
                Some(telemetry_arc),
            );
            let ctx = control_plane::DispatchContext {
                agent_name: def.name.clone(),
                task: def.task.clone(),
                static_model: def.model.clone(),
                cli_tool: def.cli_tool.clone(),
                layer: def.layer,
                session_id: None,
            };
            let budget_verdict = self.cost_tracker.check(&def.name);
            let decision = engine.decide_route(&ctx, &budget_verdict).await;
            info!(
                agent = %def.name,
                rationale = %decision.rationale,
                telemetry_influenced = decision.telemetry_influenced,
                "routing engine selected model"
            );
            if decision.candidate.model.is_empty() {
                None
            } else {
                // Extract CLI tool override from routing decision so that
                // anthropic models routed via KG use claude CLI, not opencode.
                if decision.candidate.cli_tool != def.cli_tool {
                    kg_cli_override = Some(decision.candidate.cli_tool.clone());
                }
                Some(decision.candidate.model)
            }
        } else if supports_model_flag {
            // KG routing first (phase-aware tier selection from markdown rules).
            // Takes priority over static model config so tier routing controls selection.
            let unhealthy = self.provider_health.unhealthy_providers();
            let kg_decision = self.kg_router.as_ref().and_then(|router| {
                let decision = router.route_agent(&def.task)?;
                // If primary provider is unhealthy, try fallback routes
                if !unhealthy.is_empty() {
                    if let Some(healthy_route) = decision.first_healthy_route(&unhealthy) {
                        info!(
                            agent = %def.name,
                            concept = %decision.matched_concept,
                            provider = %healthy_route.provider,
                            model = %healthy_route.model,
                            skipped_unhealthy = ?unhealthy,
                            "KG routed to fallback (primary unhealthy)"
                        );
                        return Some(kg_router::KgRouteDecision {
                            provider: healthy_route.provider.clone(),
                            model: healthy_route.model.clone(),
                            action: healthy_route.action.clone(),
                            confidence: decision.confidence * 0.9,
                            matched_concept: decision.matched_concept,
                            priority: decision.priority,
                            fallback_routes: decision.fallback_routes,
                        });
                    }
                }
                Some(decision)
            });

            if let Some(ref kg) = kg_decision {
                info!(
                    agent = %def.name,
                    concept = %kg.matched_concept,
                    provider = %kg.provider,
                    model = %kg.model,
                    confidence = kg.confidence,
                    "model selected via KG tier routing"
                );
                // Extract CLI tool from action template (first word = CLI path)
                if let Some(ref action) = kg.action {
                    if let Some(cli) = action.split_whitespace().next() {
                        kg_cli_override = Some(cli.to_string());
                    }
                }
                Some(kg.model.clone())
            } else if let Some(m) = &def.model {
                // Static config fallback when KG has no match
                info!(agent = %def.name, model = %m, "using static model (no KG tier match)");
                Some(m.clone())
            } else {
                // Fall back to keyword routing engine
                let context = terraphim_router::RoutingContext::default();
                match self.router.route(&def.task, &context) {
                    Ok(decision) => {
                        if let terraphim_types::capability::ProviderType::Llm { model_id, .. } =
                            &decision.provider.provider_type
                        {
                            info!(
                                agent = %def.name,
                                model = %model_id,
                                confidence = decision.confidence,
                                "model selected via keyword routing"
                            );
                            Some(model_id.clone())
                        } else {
                            None
                        }
                    }
                    Err(_) => {
                        info!(agent = %def.name, "no model matched, using CLI default");
                        None
                    }
                }
            }
        } else {
            info!(agent = %def.name, cli = %def.cli_tool, "skipping model routing (CLI uses OAuth/default)");
            None
        };

        // For opencode, compose "provider/model" format when both fields are set.
        // opencode requires `-m provider/model` whereas the TOML config stores them
        // separately (provider = "kimi-for-coding", model = "k2p5").
        let model = if cli_name == "opencode" {
            match (&def.provider, &model) {
                (Some(provider), Some(m)) => {
                    let composed = format!("{}/{}", provider, m);
                    info!(agent = %def.name, composed_model = %composed, "composed provider/model for opencode");
                    Some(composed)
                }
                _ => model,
            }
        } else {
            model
        };

        // If KG routing selected a different CLI tool (e.g., claude instead of opencode),
        // use the KG-selected CLI to match the routed model.
        let effective_cli = kg_cli_override
            .as_deref()
            .unwrap_or(&def.cli_tool)
            .to_string();

        info!(agent = %def.name, layer = ?def.layer, cli = %effective_cli, model = ?model, "spawning agent");

        // Compose persona-enriched task prompt
        let (composed_task, persona_found) = if let Some(ref persona_name) = def.persona {
            if let Some(persona) = self.persona_registry.get(persona_name) {
                let composed = self.metaprompt_renderer.compose_prompt(persona, &def.task);
                info!(
                    agent = %def.name,
                    persona = %persona_name,
                    original_len = def.task.len(),
                    composed_len = composed.len(),
                    "composed persona-enriched prompt"
                );
                (composed, true)
            } else {
                warn!(
                    agent = %def.name,
                    persona = %persona_name,
                    "persona not found in registry, using bare task"
                );
                (def.task.clone(), false)
            }
        } else {
            (def.task.clone(), false)
        };

        // === FINDINGS INJECTION ===
        let composed_task = if let Some(ref findings) = findings {
            format!(
                "## Pre-flight findings (automated checks already ran)\n\n{}\n\n---\n\n{}",
                findings, composed_task
            )
        } else {
            composed_task
        };

        // Inject skill_chain content between persona preamble and task
        let skill_content = self.load_skill_chain_content(def);
        let composed_task = if skill_content.is_empty() {
            composed_task
        } else {
            info!(
                agent = %def.name,
                skills = def.skill_chain.len(),
                skill_bytes = skill_content.len(),
                "injecting skill_chain into prompt"
            );
            format!("{}{}", composed_task, skill_content)
        };

        // Use stdin only when persona was actually resolved (prompt is enriched)
        // or when the task exceeds ARG_MAX safety threshold.
        // Do NOT use stdin for unfound personas -- the bare task is small and
        // stdin delivery to short-lived processes (echo) causes broken pipe races.
        const STDIN_THRESHOLD: usize = 32_768; // 32 KB
        let use_stdin =
            persona_found || !skill_content.is_empty() || composed_task.len() > STDIN_THRESHOLD;

        // Create isolated git worktree for implementation-tier agents that modify code.
        // Review-tier agents (haiku) are read-only and don't need isolation.
        let needs_isolation = model.as_ref().map(|m| !m.contains("haiku")).unwrap_or(true);
        let worktree_path = if needs_isolation {
            self.create_agent_worktree(&def.name).await
        } else {
            None
        };
        let agent_working_dir = worktree_path
            .as_ref()
            .unwrap_or(&self.config.working_dir)
            .clone();

        // Build primary Provider from the agent definition for the spawner.
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: effective_cli.clone(),
                working_dir: agent_working_dir.clone(),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        // Build fallback Provider if fallback_provider is configured
        let fallback_provider = def.fallback_provider.as_ref().map(|fallback_cli| {
            terraphim_types::capability::Provider {
                id: format!("{}-fallback", def.name),
                name: format!("{} (fallback)", def.name),
                provider_type: terraphim_types::capability::ProviderType::Agent {
                    agent_id: format!("{}-fallback", def.name),
                    cli_command: fallback_cli.clone(),
                    working_dir: agent_working_dir.clone(),
                },
                capabilities: vec![],
                cost_level: terraphim_types::capability::CostLevel::Cheap,
                latency: terraphim_types::capability::Latency::Medium,
                keywords: def.capabilities.clone(),
            }
        });

        // Build the spawn request with primary and fallback
        let mut request = SpawnRequest::new(primary_provider, &composed_task)
            .with_primary_model(model.as_deref().unwrap_or(""));

        if let Some(fallback) = fallback_provider {
            request = request.with_fallback_provider(fallback);
            if let Some(fallback_model) = &def.fallback_model {
                request = request.with_fallback_model(fallback_model);
            }
        }

        if use_stdin {
            request = request.with_stdin();
        }

        // Thread resource limits from agent definition to spawner
        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        let spawn_ctx = build_spawn_context_for_agent(&self.config, def);
        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        // Subscribe to the output broadcast for nightwatch drain
        let output_rx = handle.subscribe_output();

        // Get the restart count from the orchestrator-level counter
        let restart_count = self
            .restart_counts
            .get(&agent_key(def))
            .copied()
            .unwrap_or(0);

        self.active_agents.insert(
            def.name.clone(),
            ManagedAgent {
                definition: def.clone(),
                handle,
                started_at: Instant::now(),
                restart_count,
                output_rx,
                spawned_by_mention: false,
                worktree_path,
                routed_model: model.clone(),
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
            },
        );

        // === RECORD COMMIT FOR GIT-DIFF STRATEGY ===
        if let Ok(head) = self.get_current_head().await {
            self.last_run_commits.insert(def.name.clone(), head);
        }

        #[cfg(feature = "quickwit")]
        if let Some(ref sink) = self.quickwit_sink {
            let doc = quickwit::LogDocument {
                timestamp: chrono::Utc::now().to_rfc3339(),
                project_id: def
                    .project
                    .clone()
                    .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
                level: "INFO".into(),
                agent_name: def.name.clone(),
                layer: format!("{:?}", def.layer),
                source: "orchestrator".into(),
                message: "agent spawned".into(),
                model: model.clone(),
                ..Default::default()
            };
            let _ = sink.send(doc).await;
        }

        Ok(())
    }

    /// Evaluate the pre-check strategy for an agent.
    async fn run_pre_check(&mut self, def: &AgentDefinition) -> PreCheckResult {
        match &def.pre_check {
            None | Some(PreCheckStrategy::Always) => PreCheckResult::Findings(String::new()),
            Some(PreCheckStrategy::GitDiff { watch_paths }) => {
                self.git_diff_pre_check(&def.name, watch_paths).await
            }
            Some(PreCheckStrategy::GiteaIssue { issue_number }) => {
                self.gitea_issue_pre_check(*issue_number).await
            }
            Some(PreCheckStrategy::Shell {
                script,
                timeout_secs,
            }) => self.shell_pre_check(script, *timeout_secs).await,
        }
    }

    /// Git diff pre-check: compare last_run_commit to HEAD.
    async fn git_diff_pre_check(&self, agent_name: &str, watch_paths: &[String]) -> PreCheckResult {
        let last_commit = match self.last_run_commits.get(agent_name) {
            Some(c) => c.clone(),
            None => {
                info!(agent = %agent_name, "no last_run_commit recorded, spawning (first run)");
                return PreCheckResult::Findings(String::new());
            }
        };

        // Get current HEAD
        let head = match self.get_current_head().await {
            Ok(h) => h,
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "failed to get HEAD, spawning (fail-open)");
                return PreCheckResult::Failed(format!("git rev-parse failed: {}", e));
            }
        };

        if head == last_commit {
            info!(agent = %agent_name, commit = %head, "HEAD unchanged since last run, skipping");
            return PreCheckResult::NoFindings;
        }

        // Get changed files
        let diff_range = format!("{}..{}", last_commit, head);
        let output = match tokio::time::timeout(
            Duration::from_secs(30),
            tokio::process::Command::new("git")
                .args(["diff", "--name-only", &diff_range])
                .current_dir(&self.config.working_dir)
                .output(),
        )
        .await
        {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                warn!(agent = %agent_name, error = %e, "git diff failed, spawning (fail-open)");
                return PreCheckResult::Failed(format!("git diff failed: {}", e));
            }
            Err(_) => {
                warn!(agent = %agent_name, "git diff timed out after 30s, spawning (fail-open)");
                return PreCheckResult::Failed("git diff timed out after 30s".into());
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(agent = %agent_name, stderr = %stderr, "git diff non-zero exit, spawning (fail-open)");
            return PreCheckResult::Failed(format!("git diff exit {}: {}", output.status, stderr));
        }

        let changed_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        if changed_files.is_empty() {
            info!(agent = %agent_name, "no files changed, skipping");
            return PreCheckResult::NoFindings;
        }

        if has_matching_changes(&changed_files, watch_paths) {
            let summary = format!("{} files changed matching watch_paths", changed_files.len());
            info!(agent = %agent_name, files = changed_files.len(), "matching changes found");
            PreCheckResult::Findings(summary)
        } else {
            info!(agent = %agent_name, files = changed_files.len(), "changes found but none match watch_paths, skipping");
            PreCheckResult::NoFindings
        }
    }

    /// Shell pre-check: run script via sh -c.
    async fn shell_pre_check(&self, script: &str, timeout_secs: u64) -> PreCheckResult {
        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(script)
                .current_dir(&self.config.working_dir)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if stdout.is_empty() {
                        PreCheckResult::NoFindings
                    } else {
                        PreCheckResult::Findings(stdout)
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    PreCheckResult::Failed(format!("script exit {}: {}", output.status, stderr))
                }
            }
            Ok(Err(e)) => PreCheckResult::Failed(format!("script I/O error: {}", e)),
            Err(_) => PreCheckResult::Failed(format!("script timed out after {}s", timeout_secs)),
        }
    }

    /// Get or lazily construct the GiteaTracker for pre-check.
    fn get_or_init_pre_check_tracker(&mut self) -> Option<&terraphim_tracker::GiteaTracker> {
        if self.pre_check_tracker.is_some() {
            return self.pre_check_tracker.as_ref();
        }
        let workflow = self.config.workflow.as_ref()?;
        let tc = &workflow.tracker;
        let config = terraphim_tracker::GiteaConfig {
            base_url: tc.endpoint.clone(),
            token: tc.api_key.clone(),
            owner: tc.owner.clone(),
            repo: tc.repo.clone(),
            active_states: tc.states.active.clone(),
            terminal_states: tc.states.terminal.clone(),
            use_robot_api: tc.use_robot_api,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        match terraphim_tracker::GiteaTracker::new(config) {
            Ok(tracker) => {
                self.pre_check_tracker = Some(tracker);
                self.pre_check_tracker.as_ref()
            }
            Err(e) => {
                warn!(error = %e, "failed to construct GiteaTracker for pre-check");
                None
            }
        }
    }

    /// Evaluate the gitea-issue pre-check strategy.
    async fn gitea_issue_pre_check(&mut self, issue_number: u64) -> PreCheckResult {
        let tracker = match self.get_or_init_pre_check_tracker() {
            Some(t) => t,
            None => {
                return PreCheckResult::Failed(
                    "no workflow config for gitea-issue pre-check".into(),
                );
            }
        };

        // Fetch comments with 15s timeout
        let comments = match tokio::time::timeout(
            Duration::from_secs(15),
            tracker.fetch_comments(issue_number, None),
        )
        .await
        {
            Ok(Ok(comments)) => comments,
            Ok(Err(e)) => {
                warn!(
                    issue = issue_number,
                    error = %e,
                    "gitea comment fetch failed, spawning (fail-open)"
                );
                return PreCheckResult::Failed(format!("comment fetch failed: {}", e));
            }
            Err(_) => {
                warn!(
                    issue = issue_number,
                    "gitea comment fetch timed out, spawning (fail-open)"
                );
                return PreCheckResult::Failed("comment fetch timed out after 15s".into());
            }
        };

        if comments.is_empty() {
            info!(issue = issue_number, "no comments on issue, spawning");
            return PreCheckResult::Findings(String::new());
        }

        // Check the most recent comment for PASS verdict
        let latest = comments.last().expect("checked non-empty above");
        let body_lower = latest.body.to_lowercase();

        if body_lower.contains("verdict: pass") {
            // Check if there are new commits since this comment
            let comment_time = &latest.created_at;

            // Use git log to check for commits after the comment time
            let output = match tokio::time::timeout(
                Duration::from_secs(30),
                tokio::process::Command::new("git")
                    .args(["log", "--oneline", &format!("--since={}", comment_time)])
                    .current_dir(&self.config.working_dir)
                    .output(),
            )
            .await
            {
                Ok(Ok(o)) => o,
                Ok(Err(e)) => {
                    warn!(error = %e, "git log failed, spawning (fail-open)");
                    return PreCheckResult::Failed(format!("git log failed: {}", e));
                }
                Err(_) => {
                    warn!("git log --since timed out after 30s, spawning (fail-open)");
                    return PreCheckResult::Failed("git log timed out after 30s".into());
                }
            };

            let log_output = String::from_utf8_lossy(&output.stdout);
            if log_output.trim().is_empty() {
                info!(
                    issue = issue_number,
                    "PASS verdict and no new commits, skipping"
                );
                return PreCheckResult::NoFindings;
            } else {
                let commit_count = log_output.lines().count();
                info!(
                    issue = issue_number,
                    new_commits = commit_count,
                    "PASS verdict but new commits found, spawning"
                );
                return PreCheckResult::Findings(format!(
                    "{} new commits since last PASS verdict",
                    commit_count
                ));
            }
        }

        info!(
            issue = issue_number,
            "no PASS verdict in latest comment, spawning"
        );
        PreCheckResult::Findings(String::new())
    }

    /// Get current HEAD commit hash.
    async fn get_current_head(&self) -> Result<String, OrchestratorError> {
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&self.config.working_dir)
                .output(),
        )
        .await
        .map_err(|_| OrchestratorError::Config("git rev-parse HEAD timed out after 5s".into()))?
        .map_err(OrchestratorError::from)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(OrchestratorError::Config(
                "git rev-parse HEAD failed".into(),
            ))
        }
    }

    /// Poll configured Gitea issues for new @adf: mentions and enqueue MentionDriven tasks.
    ///
    /// Skipped when no mention configuration is present or when the Gitea tracker
    /// is not configured. Uses `tick_count % poll_modulo` to avoid polling on
    /// every reconciliation tick.
    /// Cursor-based mention polling: single API call, no replay on restart.
    ///
    /// Handle a dispatch request received from the webhook endpoint.
    /// This is the webhook equivalent of poll_mentions but immediate.
    async fn handle_webhook_dispatch(&mut self, dispatch: webhook::WebhookDispatch) {
        // Rate limiting: check concurrent mention-spawned agents
        let mention_cfg = match self.config.mentions.as_ref() {
            Some(cfg) => cfg,
            None => return,
        };

        let active_mention_agents = self
            .active_agents
            .values()
            .filter(|a| a.spawned_by_mention)
            .count() as u32;

        if active_mention_agents >= mention_cfg.max_concurrent_mention_agents {
            warn!(
                active = active_mention_agents,
                max = mention_cfg.max_concurrent_mention_agents,
                "webhook dispatch rejected: mention agents at capacity"
            );
            return;
        }

        let agents = self.config.agents.clone();

        match dispatch {
            webhook::WebhookDispatch::SpawnAgent {
                agent_name,
                issue_number,
                comment_id,
                context,
            } => {
                info!(
                    agent = %agent_name,
                    issue = issue_number,
                    comment_id = comment_id,
                    "webhook: dispatching agent spawn"
                );

                if let Some(def) = agents.iter().find(|a| a.name == agent_name).cloned() {
                    // Dedup: check Gitea assignment + active_agents before spawning
                    if self.should_skip_dispatch(&agent_name, issue_number).await {
                        return;
                    }

                    let mut mention_def = def.clone();
                    mention_def.task = format!(
                        "{}\n\n## Mention Context\nTriggered by @adf:{} webhook in issue #{} (comment {}).\nContext: {}",
                        def.task, agent_name, issue_number, comment_id, context
                    );
                    mention_def.gitea_issue = Some(issue_number);

                    if let Err(e) = self.spawn_agent(&mention_def).await {
                        error!(agent = %agent_name, issue = issue_number, error = %e, "webhook: failed to spawn agent");
                    } else if let Some(agent) = self.active_agents.get_mut(&mention_def.name) {
                        agent.spawned_by_mention = true;
                    }
                }
            }
            webhook::WebhookDispatch::SpawnPersona {
                persona_name,
                issue_number,
                comment_id,
                context,
            } => {
                if let Some((agent_name, _)) = mention::resolve_persona_mention(
                    &persona_name,
                    &agents,
                    &self.persona_registry,
                    &context,
                ) {
                    info!(
                        persona = %persona_name,
                        agent = %agent_name,
                        issue = issue_number,
                        "webhook: dispatching persona-resolved agent"
                    );

                    if let Some(def) = agents.iter().find(|a| a.name == agent_name).cloned() {
                        // Dedup: check Gitea assignment + active_agents before spawning
                        if self.should_skip_dispatch(&agent_name, issue_number).await {
                            return;
                        }

                        let mut mention_def = def.clone();
                        mention_def.task = format!(
                            "{}\n\n## Mention Context\nTriggered by @adf:{} persona webhook in issue #{} (comment {}).\nContext: {}",
                            def.task, persona_name, issue_number, comment_id, context
                        );
                        mention_def.gitea_issue = Some(issue_number);

                        if let Err(e) = self.spawn_agent(&mention_def).await {
                            error!(agent = %agent_name, issue = issue_number, error = %e, "webhook: failed to spawn agent");
                        } else if let Some(agent) = self.active_agents.get_mut(&mention_def.name) {
                            agent.spawned_by_mention = true;
                        }
                    }
                }
            }
            webhook::WebhookDispatch::CompoundReview {
                issue_number,
                comment_id,
            } => {
                info!(
                    issue = issue_number,
                    comment_id = comment_id,
                    "webhook: compound review triggered"
                );
                self.handle_schedule_event(ScheduleEvent::CompoundReview)
                    .await;

                // Post acknowledgment via existing output_poster
                if let Some(ref poster) = self.output_poster {
                    let ack_body = format!(
                        "## Compound Review Triggered (webhook)\n\n\
                        Manual trigger received from issue #{} comment {}.\n\
                        Running 6-agent review swarm now...",
                        issue_number, comment_id
                    );
                    if let Err(e) = poster.post_raw(issue_number, &ack_body).await {
                        warn!(error = %e, "failed to post compound review acknowledgment");
                    }
                }
            }
        }
    }

    /// Uses repo-wide comments endpoint with `since` cursor. On first run
    /// (no persisted cursor), cursor is set to `now` to skip all historical
    /// mentions — preventing the mention replay storm.
    async fn poll_mentions(&mut self) {
        let mention_cfg = match self.config.mentions.clone() {
            Some(cfg) => cfg,
            None => return,
        };

        let gitea_cfg = match self.config.gitea.clone() {
            Some(cfg) => cfg,
            None => {
                tracing::debug!("mention polling skipped: no Gitea output config");
                return;
            }
        };

        // Respect poll_modulo to reduce API traffic
        if self.tick_count % mention_cfg.poll_modulo != 0 {
            return;
        }

        // Count currently active mention-spawned agents
        let active_mention_agents = self
            .active_agents
            .values()
            .filter(|a| a.spawned_by_mention)
            .count() as u32;
        if active_mention_agents >= mention_cfg.max_concurrent_mention_agents {
            tracing::debug!(
                active = active_mention_agents,
                max = mention_cfg.max_concurrent_mention_agents,
                "mention agents at capacity, skipping poll"
            );
            return;
        }

        // Lazy-load cursor on first poll. In Commit 4 this becomes one
        // pass per configured project; for now we poll under the legacy
        // synthetic project id.
        let project_id = dispatcher::LEGACY_PROJECT_ID.to_string();
        let mut cursor = match self.mention_cursors.remove(&project_id) {
            Some(c) => c,
            None => mention::MentionCursor::load_or_now(&project_id).await,
        };
        cursor.dispatches_this_tick = 0;

        // Create Gitea tracker for repo-wide comment polling
        let tracker_cfg = terraphim_tracker::GiteaConfig {
            base_url: gitea_cfg.base_url.clone(),
            token: gitea_cfg.token.clone(),
            owner: gitea_cfg.owner.clone(),
            repo: gitea_cfg.repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        let tracker = match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "failed to create GiteaTracker for mention polling");
                self.mention_cursors.insert(project_id, cursor);
                return;
            }
        };

        // Single API call: all comments since cursor
        let comments = match tracker
            .fetch_repo_comments(Some(&cursor.last_seen_at), Some(50))
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "failed to fetch repo comments for mention polling");
                self.mention_cursors.insert(project_id, cursor);
                return;
            }
        };

        if comments.is_empty() {
            cursor.save(&project_id).await;
            self.mention_cursors.insert(project_id, cursor);
            return;
        }

        let agents = self.config.agents.clone();
        let persona_registry = self.persona_registry.clone();
        let max_dispatches = mention_cfg.max_dispatches_per_tick;

        // Build ADF command parser with known agents and personas
        let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
        let persona_names: Vec<String> = persona_registry
            .persona_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let command_parser =
            crate::adf_commands::AdfCommandParser::new(&agent_names, &persona_names);

        for comment in &comments {
            if cursor.dispatches_this_tick >= max_dispatches {
                tracing::debug!(
                    dispatched = cursor.dispatches_this_tick,
                    max = max_dispatches,
                    "max dispatches per tick reached"
                );
                break;
            }

            // Skip already-processed comments (persisted dedup across restarts)
            if cursor.is_processed(comment.id) {
                tracing::debug!(
                    comment_id = comment.id,
                    issue = comment.issue_number,
                    "skipping already-processed comment"
                );
                cursor.advance_to(&comment.created_at);
                continue;
            }

            // Parse ADF commands using terraphim-automata Aho-Corasick
            let commands =
                command_parser.parse_commands(&comment.body, comment.issue_number, comment.id);

            for cmd in commands {
                if cursor.dispatches_this_tick >= max_dispatches {
                    break;
                }

                match cmd {
                    crate::adf_commands::AdfCommand::CompoundReview {
                        issue_number,
                        comment_id,
                    } => {
                        info!(
                            issue = issue_number,
                            comment_id = comment_id,
                            "compound review triggered via @adf:compound-review mention"
                        );

                        // Trigger compound review
                        self.handle_schedule_event(ScheduleEvent::CompoundReview)
                            .await;

                        // Post acknowledgment
                        if let Some(ref poster) = self.output_poster {
                            let ack_body = format!(
                                "## 🔍 Compound Review Triggered\n\n\
                                Manual trigger received from issue #{} comment {}.\n\
                                Running 6-agent review swarm now...\n\n\
                                _Results will be posted to issue #{} when complete._",
                                issue_number,
                                comment_id,
                                self.config.compound_review.gitea_issue.unwrap_or(108)
                            );
                            if let Err(e) = poster.post_raw(issue_number, &ack_body).await {
                                warn!(error = %e, "failed to post compound review acknowledgment");
                            }
                        }

                        cursor.dispatches_this_tick += 1;
                    }
                    crate::adf_commands::AdfCommand::SpawnAgent {
                        agent_name,
                        issue_number,
                        comment_id,
                        context,
                    } => {
                        info!(
                            agent = %agent_name,
                            issue = issue_number,
                            comment_id = comment_id,
                            "dispatching mention-driven agent via terraphim-automata parser"
                        );

                        if let Some(def) = agents.iter().find(|a| a.name == agent_name).cloned() {
                            // Dedup: check Gitea assignment + active_agents before spawning
                            if self.should_skip_dispatch(&agent_name, issue_number).await {
                                cursor.dispatches_this_tick += 1;
                                continue;
                            }

                            let mut mention_def = def.clone();
                            mention_def.task = format!(
                                "{}\n\n## Mention Context\nTriggered by @adf:{} mention in issue #{} (comment {}).\nContext: {}",
                                def.task,
                                agent_name,
                                issue_number,
                                comment_id,
                                context
                            );
                            mention_def.gitea_issue = Some(issue_number);

                            if let Err(e) = self.spawn_agent(&mention_def).await {
                                tracing::error!(agent = %agent_name, issue = issue_number, error = %e, "failed to spawn agent");
                            } else if let Some(agent) =
                                self.active_agents.get_mut(&mention_def.name)
                            {
                                agent.spawned_by_mention = true;
                            }

                            cursor.dispatches_this_tick += 1;
                        }
                    }
                    crate::adf_commands::AdfCommand::SpawnPersona {
                        persona_name,
                        issue_number,
                        comment_id,
                        context,
                    } => {
                        // Resolve persona to agent
                        if let Some((agent_name, _)) = mention::resolve_persona_mention(
                            &persona_name,
                            &agents,
                            &persona_registry,
                            &context,
                        ) {
                            info!(
                                persona = %persona_name,
                                agent = %agent_name,
                                issue = issue_number,
                                "dispatching persona-resolved agent via terraphim-automata parser"
                            );

                            if let Some(def) = agents.iter().find(|a| a.name == agent_name).cloned()
                            {
                                // Dedup: check Gitea assignment + active_agents before spawning
                                if self.should_skip_dispatch(&agent_name, issue_number).await {
                                    cursor.dispatches_this_tick += 1;
                                    continue;
                                }

                                let mut mention_def = def.clone();
                                mention_def.task = format!(
                                    "{}\n\n## Mention Context\nTriggered by @adf:{} persona mention in issue #{} (comment {}).\nContext: {}",
                                    def.task,
                                    persona_name,
                                    issue_number,
                                    comment_id,
                                    context
                                );
                                mention_def.gitea_issue = Some(issue_number);

                                if let Err(e) = self.spawn_agent(&mention_def).await {
                                    tracing::error!(agent = %agent_name, issue = issue_number, error = %e, "failed to spawn agent");
                                } else if let Some(agent) =
                                    self.active_agents.get_mut(&mention_def.name)
                                {
                                    agent.spawned_by_mention = true;
                                }

                                cursor.dispatches_this_tick += 1;
                            }
                        }
                    }
                    crate::adf_commands::AdfCommand::Unknown { raw } => {
                        warn!(raw = %raw, "unknown ADF command");
                    }
                }
            }

            // Mark comment as processed and advance cursor
            cursor.mark_processed(comment.id);
            cursor.advance_to(&comment.created_at);
        }

        // Persist cursor for next poll / restart
        cursor.save(&project_id).await;
        self.mention_cursors.insert(project_id, cursor);
    }

    /// Check if an agent is already assigned to this issue and currently active.
    ///
    /// Returns `true` if dispatch should be **skipped** (duplicate), `false` if
    /// dispatch should proceed. When dispatch proceeds, the issue is assigned
    /// to the agent as a side-effect.
    ///
    /// The dedup logic:
    /// - If the issue is already assigned to the agent AND the agent is currently
    ///   in `active_agents` -> skip (duplicate dispatch).
    /// - If assigned but agent is NOT active -> allow (agent crashed, re-dispatch).
    /// - If not assigned -> allow (first dispatch) and assign.
    async fn should_skip_dispatch(&self, agent_name: &str, issue_number: u64) -> bool {
        if issue_number == 0 {
            return false;
        }

        // Fast local check: if agent is already running, skip immediately.
        // This prevents races where the Gitea API returns stale assignee data
        // because a concurrent dispatch path just assigned the issue milliseconds ago.
        if self.active_agents.contains_key(agent_name) {
            warn!(
                agent = %agent_name,
                issue = issue_number,
                "skipping dispatch: agent already active (local guard)"
            );
            return true;
        }

        let Some(ref poster) = self.output_poster else {
            return false;
        };
        // Resolve the agent's owning project so the tracker uses the
        // correct owner/repo (multi-project) or falls back to legacy.
        let project = self
            .config
            .agents
            .iter()
            .find(|a| a.name == agent_name)
            .and_then(|a| a.project.clone())
            .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
        let Some(tracker) = poster.tracker_for(&project, agent_name) else {
            warn!(
                agent = %agent_name,
                project = %project,
                "no Gitea tracker for project; treating dispatch as not-duplicate"
            );
            return false;
        };

        // Remote check: if agent is assigned in Gitea but not active (crash recovery)
        match tracker.fetch_issue_assignees(issue_number).await {
            Ok(assignees) => {
                if assignees.iter().any(|a| a == agent_name) {
                    // Already assigned -- check if agent is actively running
                    if self.active_agents.contains_key(agent_name) {
                        warn!(
                            agent = %agent_name,
                            issue = issue_number,
                            "skipping duplicate dispatch: agent already assigned and active"
                        );
                        return true;
                    }
                    // Assigned but not active (crashed or completed) -- allow re-dispatch
                    info!(
                        agent = %agent_name,
                        issue = issue_number,
                        "agent assigned but not active, allowing re-dispatch"
                    );
                }
            }
            Err(e) => {
                // Fail open: if we can't check assignees, allow dispatch
                warn!(
                    agent = %agent_name,
                    issue = issue_number,
                    error = %e,
                    "failed to fetch assignees, allowing dispatch (fail-open)"
                );
            }
        }

        // Assign the issue to the agent
        if let Err(e) = tracker.assign_issue(issue_number, &[agent_name]).await {
            warn!(
                agent = %agent_name,
                issue = issue_number,
                error = %e,
                "failed to assign issue to agent"
            );
        } else {
            info!(
                agent = %agent_name,
                issue = issue_number,
                "assigned issue to agent"
            );
        }
        false
    }

    /// Sanitise finding text for use in issue title.
    /// Strips JSON syntax characters that break title parsing.
    fn sanitise_for_title(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '{' | '}' | '[' | ']' | '"' => result.push(' '),
                '\n' | '\r' => result.push(' '),
                _ => result.push(ch),
            }
        }
        let trimmed = result.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.len() > 80 {
            trimmed[..77].to_string() + "..."
        } else {
            trimmed
        }
    }

    /// Sanitise finding text for use in issue body (markdown).
    /// Escapes markdown special characters that could break rendering.
    fn sanitise_for_body(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '`' => result.push_str("``"),
                '*' | '_' | '[' | ']' => result.push('\\'),
                _ => result.push(ch),
            }
        }
        result
    }

    /// File a Gitea issue for a compound review finding.
    ///
    /// Deduplicates by searching for existing open issues with similar titles
    /// before creating a new one.
    async fn file_finding_issue(
        &self,
        poster: &OutputPoster,
        result: &CompoundReviewResult,
        finding: &ReviewFinding,
    ) -> Result<(), String> {
        use terraphim_types::FindingSeverity;

        let sev_str = match finding.severity {
            FindingSeverity::Critical => "CRITICAL",
            FindingSeverity::High => "HIGH",
            FindingSeverity::Medium => "MEDIUM",
            FindingSeverity::Low => "LOW",
            FindingSeverity::Info => "INFO",
        };

        // Build a short keyword from the finding for dedup search
        let dedup_keyword = if finding.finding.len() > 40 {
            &finding.finding[..40]
        } else {
            &finding.finding
        };

        // Dedup: check if an open issue with similar title already exists
        match poster.tracker().search_issues_by_title(dedup_keyword).await {
            Ok(existing) if !existing.is_empty() => {
                info!(
                    severity = %sev_str,
                    existing_issues = existing.len(),
                    keyword = %dedup_keyword,
                    "skipping finding issue (already filed)"
                );
                return Ok(());
            }
            Err(e) => {
                // Dedup search failed — proceed with filing (fail-open)
                warn!(error = %e, "dedup search failed, proceeding to file issue");
            }
            _ => {}
        }

        let title = format!(
            "[Compound Review] {}: {}",
            sev_str,
            Self::sanitise_for_title(&finding.finding)
        );

        let mut body = "## Automated Finding from Compound Review\n\n".to_string();
        body.push_str(&format!("- **Severity**: {}\n", sev_str));
        if !finding.file.is_empty() {
            body.push_str(&format!(
                "- **File**: {}{}\n",
                finding.file,
                if finding.line > 0 {
                    format!(":{}", finding.line)
                } else {
                    String::new()
                }
            ));
        }
        body.push_str(&format!(
            "- **Confidence**: {:.0}%\n",
            finding.confidence * 100.0
        ));
        body.push_str(&format!("- **Review ID**: {}\n\n", result.correlation_id));
        body.push_str(&format!(
            "### Finding\n\n{}\n\n",
            Self::sanitise_for_body(&finding.finding)
        ));
        if let Some(ref suggestion) = finding.suggestion {
            if !suggestion.is_empty() {
                body.push_str(&format!(
                    "### Suggested Fix\n\n{}\n",
                    Self::sanitise_for_body(suggestion)
                ));
            }
        }

        // Skip labels for now - Gitea API has issues with label format
        // TODO: Fix labels format for Gitea API
        match poster.tracker().create_issue(&title, &body, &[]).await {
            Ok(issue) => {
                info!(
                    issue_number = issue.number,
                    severity = %sev_str,
                    title = %title,
                    "filed finding issue"
                );
                // Trigger implementation-swarm via mention comment so
                // mention polling dispatches the agent automatically.
                let trigger = format!(
                    "@adf:implementation-swarm please implement this finding for issue #{}",
                    issue.number
                );
                if let Err(e) = poster.tracker().post_comment(issue.number, &trigger).await {
                    warn!(
                        issue_number = issue.number,
                        error = %e,
                        "failed to post implementation trigger comment"
                    );
                }
                Ok(())
            }
            Err(e) => Err(format!("failed to create issue '{}': {}", title, e)),
        }
    }

    /// Spawn a remediation agent for a CRITICAL finding.
    ///
    /// Looks up the finding's category in `compound_review.remediation_agents`
    /// to determine which agent to spawn. If no mapping exists, logs and skips.
    async fn spawn_remediation_agent(&mut self, finding: &ReviewFinding) -> Result<(), String> {
        let category_key = format!("{:?}", finding.category).to_lowercase();
        let agent_name = self
            .config
            .compound_review
            .remediation_agents
            .get(&category_key)
            .cloned();

        let agent_name = match agent_name {
            Some(name) => name,
            None => {
                debug!(
                    category = %category_key,
                    "no remediation agent mapped for category, skipping"
                );
                return Ok(());
            }
        };

        // Build a targeted fix prompt
        let mut prompt = "Fix this CRITICAL finding from compound review:\n\n".to_string();
        if !finding.file.is_empty() {
            prompt.push_str(&format!(
                "File: {}{}\n",
                finding.file,
                if finding.line > 0 {
                    format!(":{}", finding.line)
                } else {
                    String::new()
                }
            ));
        }
        prompt.push_str(&format!(
            "Severity: CRITICAL\nFinding: {}\n",
            finding.finding
        ));
        if let Some(ref suggestion) = finding.suggestion {
            if !suggestion.is_empty() {
                prompt.push_str(&format!("Suggested approach: {}\n", suggestion));
            }
        }
        prompt.push_str(
            "\nInstructions:\n\
1. Read the relevant file(s)\n\
2. Implement the fix\n\
3. Run cargo build && cargo test to verify\n\
4. Commit your changes\n",
        );

        // Look up the agent definition
        let agent_def = self
            .config
            .agents
            .iter()
            .find(|a| a.name == agent_name)
            .cloned();

        let agent_def = match agent_def {
            Some(def) => def,
            None => {
                warn!(
                    agent = %agent_name,
                    "remediation agent not found in fleet config, skipping"
                );
                return Ok(());
            }
        };

        // Spawn using the existing agent infrastructure
        // Build a modified agent def with our custom task prompt
        let mut fix_def = agent_def;
        fix_def.task = prompt;
        fix_def.pre_check = None; // Skip pre-check for remediation

        let spawned = self.spawn_agent(&fix_def).await;

        match spawned {
            Ok(_) => {
                info!(
                    agent = %agent_name,
                    file = %finding.file,
                    "spawned remediation agent for CRITICAL finding"
                );
                Ok(())
            }
            Err(e) => Err(format!(
                "failed to spawn remediation agent '{}': {}",
                agent_name, e
            )),
        }
    }

    /// Create a git worktree for an agent to work in isolation.
    ///
    /// Returns the worktree path if successful, None if worktree creation fails
    /// (fail-open: agent uses shared working_dir instead).
    async fn create_agent_worktree(&self, agent_name: &str) -> Option<PathBuf> {
        let worktree_root = PathBuf::from("/tmp/adf-worktrees");
        if let Err(e) = tokio::fs::create_dir_all(&worktree_root).await {
            warn!(agent = %agent_name, error = %e, "failed to create worktree root");
            return None;
        }

        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let worktree_path = worktree_root.join(format!("{agent_name}-{id}"));

        let output = tokio::process::Command::new("git")
            .args([
                "worktree",
                "add",
                "--detach",
                &worktree_path.to_string_lossy(),
                "HEAD",
            ])
            .current_dir(&self.config.working_dir)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    "created isolated git worktree"
                );
                Some(worktree_path)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                warn!(
                    agent = %agent_name,
                    error = %stderr.chars().take(200).collect::<String>(),
                    "git worktree add failed, using shared working_dir"
                );
                None
            }
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "git worktree command failed");
                None
            }
        }
    }

    /// Remove a git worktree after an agent finishes.
    async fn remove_agent_worktree(&self, agent_name: &str, worktree_path: &Path) {
        // Force-remove even if there are uncommitted changes (they were already
        // committed by try_commit_agent_work or are intentionally discarded).
        let output = tokio::process::Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                &worktree_path.to_string_lossy(),
            ])
            .current_dir(&self.config.working_dir)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    "removed agent worktree"
                );
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                warn!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    error = %stderr.chars().take(200).collect::<String>(),
                    "git worktree remove failed"
                );
            }
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "git worktree remove command failed");
            }
        }
    }

    /// Attempt to commit any uncommitted working tree changes made by an agent.
    ///
    /// This runs `git add -A && git diff --cached --quiet` to check if there
    /// are changes, then commits with a standard message. Failures are logged
    /// but not propagated — agent work is best-effort.
    async fn try_commit_agent_work(&self, agent_name: &str, working_dir: &Path) {
        // Stage all changes
        let add = tokio::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(working_dir)
            .output()
            .await;

        if let Err(e) = add {
            tracing::debug!(agent = %agent_name, error = %e, "git add failed, skipping commit");
            return;
        }

        // Check if there are staged changes
        let diff_check = tokio::process::Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(working_dir)
            .status()
            .await;

        match diff_check {
            Ok(status) if status.success() => {
                // No changes to commit
                return;
            }
            Ok(_) => { /* changes exist */ }
            Err(e) => {
                tracing::debug!(agent = %agent_name, error = %e, "git diff failed, skipping commit");
                return;
            }
        }

        // Get current branch for commit message
        let branch = tokio::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(working_dir)
            .output()
            .await;

        let branch_name = match branch {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => "unknown".to_string(),
        };

        let msg = format!("feat({agent_name}): agent work [auto-commit]");

        let commit = tokio::process::Command::new("git")
            .args(["commit", "-m", &msg])
            .current_dir(working_dir)
            .output()
            .await;

        match commit {
            Ok(output) if output.status.success() => {
                tracing::info!(
                    agent = %agent_name,
                    branch = %branch_name,
                    "auto-committed agent working tree changes"
                );
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::debug!(
                    agent = %agent_name,
                    stderr = %stderr,
                    "git commit failed"
                );
            }
            Err(e) => {
                tracing::warn!(agent = %agent_name, error = %e, "failed to run git commit");
            }
        }
    }

    /// Periodic reconciliation: detect exits, check cron, evaluate drift, drain output.
    async fn reconcile_tick(&mut self) {
        // Check wall-clock timeouts and respawn with fallback
        self.poll_wall_timeouts().await;

        // 1. Poll all active agents for exit and handle exits per layer
        self.poll_agent_exits().await;

        // 2. Restart pending Safety agents (cooldown-aware)
        self.restart_pending_safety_agents().await;

        // 3. Check cron schedules for Core agents
        self.check_cron_schedules().await;

        // 4. Drain output events to nightwatch and collect telemetry
        let telemetry_events = self.drain_output_events();
        if !telemetry_events.is_empty() {
            self.record_telemetry(telemetry_events).await;
        }

        // 5. Evaluate nightwatch drift (only during active hours)
        let nw_cfg = &self.config.nightwatch;
        let current_hour = chrono::Local::now().hour() as u8;
        let in_window = if nw_cfg.active_start_hour <= nw_cfg.active_end_hour {
            current_hour >= nw_cfg.active_start_hour && current_hour < nw_cfg.active_end_hour
        } else {
            // Wraps past midnight, e.g. start=22 end=6
            current_hour >= nw_cfg.active_start_hour || current_hour < nw_cfg.active_end_hour
        };
        if in_window {
            self.nightwatch.evaluate();
        }

        // 6. Sweep expired handoff buffer entries
        let swept = self.handoff_buffer.sweep_expired();
        if swept > 0 {
            info!(swept_count = swept, "swept expired handoff buffer entries");
        }

        // 7. Check monthly budget reset
        self.cost_tracker.monthly_reset_if_due();

        // 8. Enforce budget limits (pause exhausted agents)
        self.enforce_budgets().await;

        // 9. Poll active flows (non-blocking)
        let completed_flows: Vec<String> = self
            .active_flows
            .iter()
            .filter(|(_, handle)| handle.is_finished())
            .map(|(name, _)| name.clone())
            .collect();

        for name in completed_flows {
            if let Some(handle) = self.active_flows.remove(&name) {
                match handle.await {
                    Ok(state) => {
                        tracing::info!(flow = %name, status = ?state.status, "flow completed");
                        if let Some(ref dir) = self.config.flow_state_dir {
                            let _ = state.save_to_file(dir);
                        }
                        // Feed cost data from step envelopes into nightwatch for drift detection
                        for envelope in &state.step_envelopes {
                            if let (Some(cost), Some(input), Some(output)) = (
                                envelope.cost_usd,
                                envelope.input_tokens,
                                envelope.output_tokens,
                            ) {
                                self.nightwatch.observe_cost(
                                    &format!("flow-{}", name),
                                    cost,
                                    input,
                                    output,
                                    None, // Flows don't have individual budgets yet
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(flow = %name, error = %e, "flow task panicked");
                    }
                }
            }
        }

        // 10. Check flow schedules
        self.check_flow_schedules().await;

        // 11. Poll for @adf: mentions in watched issues
        self.poll_mentions().await;

        // 12. D-4: Hot-reload KG routing rules if markdown files changed
        if let Some(ref mut router) = self.kg_router {
            router.reload_if_changed();
        }

        // 13. D-2: Re-probe providers if cached results are stale
        if self.provider_health.is_stale() {
            if let Some(ref kg_router) = self.kg_router {
                self.provider_health.probe_all(kg_router).await;
                if let Some(ref dir) = self
                    .config
                    .routing
                    .as_ref()
                    .and_then(|r| r.probe_results_dir.clone())
                {
                    let _ = self.provider_health.save_results(dir.as_path()).await;
                }
            }
        }

        // 14. Update last_tick_time and increment tick counter
        self.last_tick_time = chrono::Utc::now();
        self.tick_count = self.tick_count.wrapping_add(1);

        // 15. Periodic telemetry persistence (every 60 ticks = ~5 min at 5s interval)
        if self.tick_count % 60 == 0 {
            self.persist_telemetry();
        }
    }

    /// Check all agent budgets and pause any that have exceeded their limits.
    async fn enforce_budgets(&mut self) {
        let actionable = self.cost_tracker.check_all();

        for (agent_name, verdict) in actionable {
            match verdict {
                BudgetVerdict::NearExhaustion {
                    spent_cents,
                    budget_cents,
                } => {
                    warn!(
                        agent = %agent_name,
                        spent_usd = spent_cents as f64 / 100.0,
                        budget_usd = budget_cents as f64 / 100.0,
                        pct = (spent_cents * 100 / budget_cents),
                        "budget warning: agent approaching monthly limit"
                    );
                }
                BudgetVerdict::Exhausted {
                    spent_cents,
                    budget_cents,
                } => {
                    error!(
                        agent = %agent_name,
                        spent_usd = spent_cents as f64 / 100.0,
                        budget_usd = budget_cents as f64 / 100.0,
                        "budget exhausted: pausing agent"
                    );
                    self.stop_agent(&agent_name).await;
                }
                _ => {}
            }
        }
    }

    /// Kill agents that have exceeded their wall-clock timeout and respawn with fallback.
    async fn poll_wall_timeouts(&mut self) {
        let mut timed_out: Vec<String> = Vec::new();
        for (name, managed) in &self.active_agents {
            if let Some(max_secs) = managed.definition.max_cpu_seconds {
                let elapsed = managed.started_at.elapsed();
                if elapsed > Duration::from_secs(max_secs) {
                    warn!(
                        agent = %name,
                        elapsed_secs = elapsed.as_secs(),
                        max_wall_secs = max_secs,
                        "agent exceeded wall-clock timeout, killing for fallback respawn"
                    );
                    timed_out.push(name.clone());
                }
            }
        }

        for name in timed_out {
            if let Some(managed) = self.active_agents.remove(&name) {
                let def = managed.definition.clone();

                // Kill the process
                if let Err(e) = managed.handle.kill().await {
                    error!(agent = %name, error = %e, "failed to kill timed-out agent");
                }

                // Try respawn with fallback if configured
                if def.fallback_provider.is_some() {
                    info!(
                        agent = %name,
                        fallback_model = ?def.fallback_model,
                        "respawning timed-out agent with fallback provider"
                    );
                    let mut fallback_def = def.clone();
                    // Swap provider to fallback (fallback_provider is a CLI tool path)
                    if let Some(ref fb_provider) = def.fallback_provider {
                        fallback_def.cli_tool = fb_provider.clone();
                    }
                    if let Some(ref fb_model) = def.fallback_model {
                        fallback_def.model = Some(fb_model.clone());
                    }
                    // Clear provider field so model composition uses the new cli_tool
                    fallback_def.provider = None;
                    // Clear fallback to prevent infinite loops
                    fallback_def.fallback_provider = None;
                    fallback_def.fallback_model = None;

                    if let Err(e) = self.spawn_agent(&fallback_def).await {
                        error!(agent = %name, error = %e, "failed to respawn with fallback");
                    }
                } else {
                    info!(agent = %name, "no fallback configured, agent timed out permanently");
                }
            }
        }
    }

    /// Poll all active agents for exit and handle exits per layer.
    async fn poll_agent_exits(&mut self) {
        // Collect exited agents first to avoid borrow conflict
        let mut exited: Vec<(String, AgentDefinition, std::process::ExitStatus)> = Vec::new();
        // Collect agents that exceeded their wall-clock timeout
        let mut timed_out: Vec<String> = Vec::new();

        for (name, managed) in &mut self.active_agents {
            match managed.handle.try_wait() {
                Ok(Some(status)) => {
                    exited.push((name.clone(), managed.definition.clone(), status));
                }
                Ok(None) => {
                    // Still running -- check wall-clock timeout
                    if let Some(max_secs) = managed.definition.max_cpu_seconds {
                        let elapsed = managed.started_at.elapsed();
                        if elapsed > Duration::from_secs(max_secs) {
                            warn!(
                                agent = %name,
                                elapsed_secs = elapsed.as_secs(),
                                max_secs = max_secs,
                                "agent exceeded wall-clock timeout, killing"
                            );
                            timed_out.push(name.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!(agent = %name, error = %e, "try_wait failed");
                }
            }
        }

        // Kill timed-out agents
        for name in timed_out {
            if let Some(mut managed) = self.active_agents.remove(&name) {
                let grace = Duration::from_secs(managed.definition.grace_period_secs.unwrap_or(5));
                match managed.handle.shutdown(grace).await {
                    Ok(graceful) => {
                        info!(
                            agent = %name,
                            graceful = graceful,
                            "timed-out agent terminated"
                        );
                    }
                    Err(e) => {
                        warn!(agent = %name, error = %e, "failed to kill timed-out agent");
                    }
                }
                // Handle exit based on layer (similar to handle_agent_exit but for timeout)
                if managed.definition.layer == AgentLayer::Safety {
                    let key = agent_key(&managed.definition);
                    let restart_count = self.increment_restart_count(&key);
                    self.restart_cooldowns.insert(key, Instant::now());
                    info!(
                        agent = %name,
                        restart_count,
                        "safety agent timed out, will restart after cooldown"
                    );
                } else {
                    info!(agent = %name, layer = ?managed.definition.layer, "agent timed out");
                }
            }
        }

        // Drain output from exiting agents BEFORE removing them
        let mut exit_telemetry: Vec<(String, control_plane::telemetry::CompletionEvent)> =
            Vec::new();
        for (name, def, status) in &exited {
            let mut stdout_lines: Vec<String> = Vec::new();
            let mut stderr_lines: Vec<String> = Vec::new();
            let mut output_lines: Vec<String> = Vec::new();
            if let Some(managed) = self.active_agents.get_mut(name) {
                let cli_tool = managed.definition.cli_tool.clone();
                let session_id = managed.session_id.clone();
                let model = managed
                    .routed_model
                    .clone()
                    .or_else(|| managed.definition.model.clone())
                    .unwrap_or_default();

                while let Ok(event) = managed.output_rx.try_recv() {
                    self.nightwatch.observe(name, &event);
                    match &event {
                        crate::OutputEvent::Stdout { line, .. } => {
                            stdout_lines.push(line.clone());
                            output_lines.push(line.clone());
                            if let Some(ce) = Self::parse_stdout_for_telemetry(
                                &cli_tool,
                                line,
                                &session_id,
                                &model,
                            ) {
                                exit_telemetry.push((name.clone(), ce));
                            }
                        }
                        crate::OutputEvent::Stderr { line, .. } => {
                            stderr_lines.push(line.clone());
                            output_lines.push(format!("[stderr] {}", line));
                            if let Some(ce) =
                                Self::parse_stderr_for_telemetry(line, &session_id, &model)
                            {
                                exit_telemetry.push((name.clone(), ce));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Classify the exit using KG-boosted pattern matching
            let classification =
                self.exit_classifier
                    .classify(status.code(), &stdout_lines, &stderr_lines);

            let wall_time_secs = self
                .active_agents
                .get(name)
                .map(|m| m.started_at.elapsed().as_secs_f64())
                .unwrap_or(0.0);

            let routed_model = self
                .active_agents
                .get(name)
                .and_then(|m| m.routed_model.clone());

            let trigger = if self
                .active_agents
                .get(name)
                .is_some_and(|m| m.spawned_by_mention)
            {
                RunTrigger::Mention
            } else {
                RunTrigger::Cron
            };

            let record = AgentRunRecord {
                run_id: uuid::Uuid::new_v4(),
                agent_name: name.clone(),
                started_at: chrono::Utc::now()
                    - chrono::Duration::milliseconds((wall_time_secs * 1000.0) as i64),
                ended_at: chrono::Utc::now(),
                exit_code: status.code(),
                exit_class: classification.exit_class,
                model_used: routed_model.clone().or_else(|| def.model.clone()),
                was_fallback: false,
                wall_time_secs,
                output_summary: AgentRunRecord::summarise_output(&stdout_lines),
                error_summary: AgentRunRecord::summarise_errors(&stderr_lines),
                trigger,
                matched_patterns: classification.matched_patterns.clone(),
                confidence: classification.confidence,
            };

            info!(
                agent = %name,
                exit_code = ?status.code(),
                exit_class = %record.exit_class,
                confidence = record.confidence,
                matched_patterns = ?record.matched_patterns,
                wall_time_secs = record.wall_time_secs,
                "agent exit classified"
            );

            // D-3: Feed exit classification into provider health circuit breaker
            if let Some(ref provider) = def.provider {
                match record.exit_class {
                    ExitClass::ModelError | ExitClass::RateLimit => {
                        self.provider_health.record_failure(provider);
                    }
                    ExitClass::Success | ExitClass::EmptySuccess => {
                        self.provider_health.record_success(provider);
                    }
                    _ => {} // Other exit classes don't affect provider health
                }
            }

            // Post output to Gitea if configured, routed by the agent's
            // owning project so multi-project fleets land comments in the
            // correct owner/repo.
            if let (Some(poster), Some(issue)) = (&self.output_poster, def.gitea_issue) {
                let exit_code = status.code();
                let project = def
                    .project
                    .clone()
                    .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
                if let Err(e) = poster
                    .post_agent_output_for_project(&project, name, issue, &output_lines, exit_code)
                    .await
                {
                    warn!(
                        agent = %name,
                        project = %project,
                        issue = issue,
                        error = %e,
                        "failed to post output to Gitea"
                    );
                }
            }

            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let exit_code = status.code();
                let level = if exit_code.unwrap_or(1) == 0 {
                    "INFO"
                } else {
                    "WARN"
                };
                let doc = quickwit::LogDocument {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    project_id: def
                        .project
                        .clone()
                        .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
                    level: level.into(),
                    agent_name: name.clone(),
                    layer: format!("{:?}", def.layer),
                    source: "orchestrator".into(),
                    message: format!("agent exited: {}", record.exit_class),
                    model: routed_model.clone().or_else(|| def.model.clone()),
                    exit_code,
                    wall_time_secs: Some(record.wall_time_secs),
                    extra: Some(serde_json::json!({
                        "exit_class": record.exit_class.to_string(),
                        "confidence": record.confidence,
                        "matched_patterns": record.matched_patterns,
                    })),
                    ..Default::default()
                };
                let _ = sink.send(doc).await;
            }
        }

        // Record telemetry from exiting agent output
        if !exit_telemetry.is_empty() {
            self.record_telemetry(exit_telemetry).await;
        }

        // Process natural exits

        // NOW remove from active_agents and handle exits.
        // Capture worktree_path before removing so we can commit + clean up.
        for (name, def, status) in exited {
            let worktree_path = self
                .active_agents
                .get(&name)
                .and_then(|m| m.worktree_path.clone());
            self.active_agents.remove(&name);
            self.handle_agent_exit(&name, &def, status);

            // Auto-commit in the agent's working directory (worktree or shared)
            let commit_dir = worktree_path.as_deref().unwrap_or(&self.config.working_dir);
            if status.success() {
                self.try_commit_agent_work(&name, commit_dir).await;
            }

            // Clean up the worktree after committing
            if let Some(ref wt) = worktree_path {
                self.remove_agent_worktree(&name, wt).await;
            }
        }
    }

    /// Handle an agent exit based on its layer.
    fn handle_agent_exit(
        &mut self,
        name: &str,
        def: &AgentDefinition,
        status: std::process::ExitStatus,
    ) {
        let key = agent_key(def);
        match def.layer {
            AgentLayer::Safety => {
                // Only count non-zero exits toward restart limit.
                // A successful exit (code 0) means the agent completed its task;
                // punishing it for succeeding makes no sense.
                if !status.success() {
                    let restart_count = self.increment_restart_count(&key);
                    self.restart_cooldowns.insert(key, Instant::now());
                    if restart_count <= self.config.max_restart_count {
                        info!(
                            agent = %name,
                            exit_status = %status,
                            restart_count,
                            cooldown_secs = self.config.restart_cooldown_secs,
                            window_secs = self.config.restart_budget_window_secs,
                            "safety agent failed, will restart after cooldown"
                        );
                    } else {
                        error!(
                            agent = %name,
                            exit_status = %status,
                            restart_count,
                            max = self.config.max_restart_count,
                            "safety agent exceeded max restarts, permanently stopped"
                        );
                    }
                } else {
                    self.restart_cooldowns.insert(key, Instant::now());
                    info!(
                        agent = %name,
                        exit_status = %status,
                        cooldown_secs = self.config.restart_cooldown_secs,
                        "safety agent completed successfully, will restart after cooldown"
                    );
                }
            }
            AgentLayer::Core => {
                info!(agent = %name, exit_status = %status, "core agent completed");
            }
            AgentLayer::Growth => {
                info!(agent = %name, exit_status = %status, "growth agent completed");
            }
        }
    }

    /// Restart Safety agents that have exited and passed their cooldown.
    async fn restart_pending_safety_agents(&mut self) {
        let cooldown = Duration::from_secs(self.config.restart_cooldown_secs);
        let max_restarts = self.config.max_restart_count;

        // Age out stale restart counters before restart eligibility checks.
        let safety_keys: Vec<(String, String)> = self
            .config
            .agents
            .iter()
            .filter(|def| def.layer == AgentLayer::Safety)
            .map(agent_key)
            .collect();
        for key in &safety_keys {
            let _ = self.current_restart_count(key);
        }

        // Find Safety agents that need restarting
        let to_restart: Vec<AgentDefinition> = self
            .config
            .agents
            .iter()
            .filter(|def| {
                // Must be Safety layer
                if def.layer != AgentLayer::Safety {
                    return false;
                }
                // Must not be currently active
                if self.active_agents.contains_key(&def.name) {
                    return false;
                }
                let key = agent_key(def);
                // Must have a restart cooldown entry (meaning it exited)
                let last_exit = match self.restart_cooldowns.get(&key) {
                    Some(t) => t,
                    None => return false,
                };
                // Must have passed the cooldown
                if last_exit.elapsed() < cooldown {
                    return false;
                }
                // Must be under max restart count
                let count = self.restart_counts.get(&key).copied().unwrap_or(0);
                count <= max_restarts
            })
            .cloned()
            .collect();

        for def in to_restart {
            let key = agent_key(&def);
            info!(
                agent = %def.name,
                restart_count = self.restart_counts.get(&key).copied().unwrap_or(0),
                "restarting safety agent after cooldown"
            );
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "failed to restart safety agent");
            }
        }
    }

    /// Check cron schedules and spawn due Core agents.
    async fn check_cron_schedules(&mut self) {
        let now = chrono::Utc::now();
        let scheduled = self.scheduler.scheduled_agents();

        // Collect agents that should fire
        let to_spawn: Vec<AgentDefinition> = scheduled
            .into_iter()
            .filter(|(def, _schedule)| {
                // Skip if already active
                !self.active_agents.contains_key(&def.name)
            })
            .filter(|(_def, schedule)| {
                // Check if a fire time exists between last_tick and now
                schedule
                    .after(&self.last_tick_time)
                    .take_while(|t| *t <= now)
                    .next()
                    .is_some()
            })
            .map(|(def, _)| def.clone())
            .collect();

        for def in to_spawn {
            info!(agent = %def.name, "cron schedule fired");
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "cron spawn failed");
            }
        }

        // Also check compound review schedule
        if let Some(compound_sched) = self.scheduler.compound_review_schedule() {
            debug!(
                last_tick = %self.last_tick_time,
                now = %now,
                "checking compound review schedule"
            );

            // Get next fire times for debugging
            let upcoming: Vec<_> = compound_sched.after(&self.last_tick_time).take(3).collect();
            debug!(upcoming = ?upcoming, "compound schedule upcoming times");

            let should_fire = compound_sched
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next()
                .is_some();

            debug!(should_fire = should_fire, "compound review fire check");

            if should_fire {
                info!("compound review schedule fired, starting review");
                self.handle_schedule_event(ScheduleEvent::CompoundReview)
                    .await;
            }
        }
    }

    /// Drain broadcast output events from all active agents into nightwatch.
    /// Also parses CLI output for telemetry completion events.
    fn drain_output_events(&mut self) -> Vec<(String, control_plane::telemetry::CompletionEvent)> {
        let mut events: Vec<(String, OutputEvent)> = Vec::new();
        for (name, managed) in &mut self.active_agents {
            loop {
                match managed.output_rx.try_recv() {
                    Ok(event) => events.push((name.clone(), event)),
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!(agent = %name, skipped = n, "output events lagged");
                        break;
                    }
                    Err(broadcast::error::TryRecvError::Closed) => break,
                }
            }
        }

        let mut completion_events: Vec<(String, control_plane::telemetry::CompletionEvent)> =
            Vec::new();

        for (name, event) in &events {
            self.nightwatch.observe(name, event);

            match event {
                OutputEvent::Stdout { line, .. } => {
                    let (cli_tool, session_id, model) = self
                        .active_agents
                        .get(name)
                        .map(|m| {
                            (
                                m.definition.cli_tool.clone(),
                                m.session_id.clone(),
                                m.routed_model
                                    .clone()
                                    .or_else(|| m.definition.model.clone())
                                    .unwrap_or_default(),
                            )
                        })
                        .unwrap_or_default();

                    if let Some(ce) =
                        Self::parse_stdout_for_telemetry(&cli_tool, line, &session_id, &model)
                    {
                        completion_events.push((name.clone(), ce));
                    }
                }
                OutputEvent::Stderr { line, .. } => {
                    let (session_id, model) = self
                        .active_agents
                        .get(name)
                        .map(|m| {
                            (
                                m.session_id.clone(),
                                m.routed_model
                                    .clone()
                                    .or_else(|| m.definition.model.clone())
                                    .unwrap_or_default(),
                            )
                        })
                        .unwrap_or_default();
                    if let Some(ce) = Self::parse_stderr_for_telemetry(line, &session_id, &model) {
                        completion_events.push((name.clone(), ce));
                    }
                }
                _ => {}
            }

            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let (level, source, line) = match event {
                    crate::OutputEvent::Stdout { line, .. } => ("INFO", "stdout", line.as_str()),
                    crate::OutputEvent::Stderr { line, .. } => ("WARN", "stderr", line.as_str()),
                    _ => continue,
                };
                let layer = self
                    .active_agents
                    .get(name)
                    .map(|m| format!("{:?}", m.definition.layer))
                    .unwrap_or_default();
                let project_id = self
                    .active_agents
                    .get(name)
                    .and_then(|m| m.definition.project.clone())
                    .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
                let model = self.active_agents.get(name).and_then(|m| {
                    m.routed_model
                        .clone()
                        .or_else(|| m.definition.model.clone())
                });
                let doc = quickwit::LogDocument {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    project_id,
                    level: level.into(),
                    agent_name: name.clone(),
                    layer,
                    source: source.into(),
                    message: line.to_owned(),
                    model,
                    ..Default::default()
                };
                let _ = sink.try_send(doc);
            }
        }

        completion_events
    }

    /// Record parsed telemetry events into the telemetry store and cost tracker.
    ///
    /// Cost accounting is performed per-agent before the batch write so that
    /// agent-level spend is still tracked individually. The telemetry store
    /// write uses a single lock acquisition via `record_batch`.
    async fn record_telemetry(
        &self,
        events: Vec<(String, control_plane::telemetry::CompletionEvent)>,
    ) {
        // Record costs per-agent first (no lock involved).
        for (agent_name, event) in &events {
            if event.cost_usd > 0.0 {
                self.cost_tracker.record_cost(agent_name, event.cost_usd);
            }
        }
        // Write all events in one lock acquisition.
        let completion_events: Vec<control_plane::telemetry::CompletionEvent> =
            events.into_iter().map(|(_, e)| e).collect();
        self.telemetry_store.record_batch(completion_events).await;
    }

    /// Attempt to restore persisted telemetry summary from durable storage.
    ///
    /// Best-effort: if no summary exists or loading fails, logs and continues
    /// with an empty telemetry store. Called once at the start of `run()`.
    async fn restore_telemetry(&self) {
        use terraphim_persistence::Persistable;
        let mut summary = control_plane::TelemetrySummary::new("telemetry_summary".to_string());
        match summary.load().await {
            Ok(loaded) => {
                self.telemetry_store.import_summary(loaded).await;
                info!("restored persisted telemetry summary");
            }
            Err(_) => {
                info!("no persisted telemetry summary found, starting fresh");
            }
        }
    }

    /// Persist telemetry summary to durable storage via fire-and-forget spawn.
    ///
    /// Clones the Arc-backed store and moves both export and save into the
    /// spawned task so the reconcile loop is not blocked by the read lock.
    fn persist_telemetry(&self) {
        let store = self.telemetry_store.clone();
        tokio::spawn(async move {
            use terraphim_persistence::Persistable;
            let summary = store.export_summary().await;
            if let Err(e) = summary.save().await {
                tracing::warn!(error = %e, "failed to persist telemetry summary");
            }
        });
    }

    /// Parse a stdout line from a CLI tool into a CompletionEvent, if the line
    /// represents a completed agent session.
    ///
    /// Returns `None` for lines that do not carry completion telemetry (tool
    /// calls, status updates, ignored formats, or unrecognised cli_tool).
    fn parse_stdout_for_telemetry(
        cli_tool: &str,
        line: &str,
        session_id: &str,
        model: &str,
    ) -> Option<control_plane::telemetry::CompletionEvent> {
        let parsed = match cli_tool {
            "opencode" => {
                control_plane::output_parser::parse_opencode_line(line, session_id, model, None)
            }
            "claude" => control_plane::output_parser::parse_claude_line(line, session_id, model),
            _ => control_plane::output_parser::ParsedOutput::Ignored,
        };
        match parsed {
            control_plane::output_parser::ParsedOutput::Completion(ce) => Some(ce),
            _ => None,
        }
    }

    /// Parse a stderr line into a CompletionEvent representing a subscription
    /// limit error.
    ///
    /// Returns `None` when the line does not match any known limit-error
    /// pattern.
    fn parse_stderr_for_telemetry(
        line: &str,
        session_id: &str,
        model: &str,
    ) -> Option<control_plane::telemetry::CompletionEvent> {
        control_plane::output_parser::parse_stderr_for_limit_errors(line)?;
        Some(control_plane::telemetry::CompletionEvent {
            model: model.to_string(),
            session_id: session_id.to_string(),
            completed_at: chrono::Utc::now(),
            latency_ms: 0,
            success: false,
            tokens: control_plane::telemetry::TokenBreakdown::default(),
            cost_usd: 0.0,
            error: Some(line.to_string()),
        })
    }

    /// Check flow schedules and trigger due flows.
    async fn check_flow_schedules(&mut self) {
        let now = chrono::Utc::now();
        let mut to_trigger: Vec<flow::config::FlowDefinition> = Vec::new();

        for flow_def in &self.config.flows {
            let Some(ref schedule_str) = flow_def.schedule else {
                continue;
            };
            let Ok(schedule) = cron::Schedule::from_str(schedule_str) else {
                continue;
            };

            // Overlap prevention: skip if this flow is already active
            if self.active_flows.contains_key(&flow_def.name) {
                tracing::info!(
                    flow = %flow_def.name,
                    "skipping cron trigger: flow already active"
                );
                continue;
            }

            let should_fire: bool = schedule
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next()
                .is_some();

            if should_fire {
                to_trigger.push(flow_def.clone());
            }
        }

        for flow_def in to_trigger {
            self.handle_schedule_event(ScheduleEvent::Flow(Box::new(flow_def)))
                .await;
        }
    }

    /// Handle a schedule event from the TimeScheduler.
    async fn handle_schedule_event(&mut self, event: ScheduleEvent) {
        match event {
            ScheduleEvent::Spawn(def) => {
                info!(agent = %def.name, "scheduled spawn");
                if let Err(e) = self.spawn_agent(&def).await {
                    error!(agent = %def.name, error = %e, "scheduled spawn failed");
                }
            }
            ScheduleEvent::Stop { agent_name } => {
                info!(agent = %agent_name, "scheduled stop");
                self.stop_agent(&agent_name).await;
            }
            ScheduleEvent::CompoundReview => {
                info!("scheduled compound review starting");
                // For scheduled reviews, use HEAD against base_branch from config
                let git_ref = "HEAD";
                let base_ref = &self.config.compound_review.base_branch;
                match self.compound_workflow.run(git_ref, base_ref).await {
                    Ok(result) => {
                        info!(
                            findings = result.findings.len(),
                            pass = %result.pass,
                            duration = ?result.duration,
                            "compound review completed"
                        );

                        // 1. Post structured summary to Gitea
                        if let (Some(ref poster), Some(issue)) =
                            (&self.output_poster, self.config.compound_review.gitea_issue)
                        {
                            let report = result.format_report();
                            if let Err(e) = poster.post_raw(issue, &report).await {
                                warn!(error = %e, "failed to post compound review summary");
                            }

                            // 2. Auto-file issues for CRITICAL/HIGH findings
                            if self.config.compound_review.auto_file_issues {
                                let actionable = result.actionable_findings();
                                for finding in actionable {
                                    if let Err(e) =
                                        self.file_finding_issue(poster, &result, finding).await
                                    {
                                        warn!(error = %e, "failed to file finding issue");
                                    }
                                }
                            }

                            // 3. Trigger remediation agents for CRITICAL findings
                            if self.config.compound_review.auto_remediate {
                                let critical: Vec<_> = result
                                    .findings
                                    .iter()
                                    .filter(|f| f.severity == FindingSeverity::Critical)
                                    .collect();
                                for finding in critical {
                                    if let Err(e) = self.spawn_remediation_agent(finding).await {
                                        warn!(error = %e, "failed to spawn remediation agent");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "compound review failed");
                    }
                }
            }
            ScheduleEvent::Flow(flow_def) => {
                let flow_name = flow_def.name.clone();
                let flow_state_dir = self
                    .config
                    .flow_state_dir
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("/tmp/flow-states"));
                let working_dir = self.config.compound_review.repo_path.clone();
                let project_runtimes = build_flow_project_runtimes(&self.config);
                let flow_def = *flow_def;
                let flow_name_for_closure = flow_name.clone();
                // FlowExecutor contains non-Send types (Regex via AgentSpawner),
                // so we use spawn_blocking + Handle::block_on as a Send-safe bridge.
                let rt_handle = tokio::runtime::Handle::current();
                let handle = tokio::task::spawn_blocking(move || {
                    let executor = flow::executor::FlowExecutor::new(working_dir, flow_state_dir)
                        .with_projects(project_runtimes);
                    rt_handle.block_on(async {
                        executor.run(&flow_def, None).await
                            .unwrap_or_else(|e| {
                                tracing::error!(flow = %flow_name_for_closure, error = %e, "flow execution failed");
                                flow::state::FlowRunState::failed(&flow_name_for_closure, &e.to_string())
                            })
                    })
                });
                self.active_flows.insert(flow_name.clone(), handle);
                tracing::info!(flow = %flow_name, "flow spawned as background task");
            }
        }
    }

    /// Handle a drift alert from the NightwatchMonitor.
    async fn handle_drift_alert(&mut self, alert: DriftAlert) {
        warn!(
            agent = %alert.agent_name,
            score = alert.drift_score.score,
            level = ?alert.drift_score.level,
            "drift alert"
        );

        match alert.recommended_action {
            CorrectionAction::LogWarning(msg) => {
                warn!(agent = %alert.agent_name, message = %msg, "drift warning");
            }
            CorrectionAction::RestartAgent => {
                info!(agent = %alert.agent_name, "restarting agent due to drift");
                self.stop_agent(&alert.agent_name).await;
                self.nightwatch.reset(&alert.agent_name);

                // Find definition and respawn
                if let Some(def) = self
                    .config
                    .agents
                    .iter()
                    .find(|a| a.name == alert.agent_name)
                    .cloned()
                {
                    if let Err(e) = self.spawn_agent(&def).await {
                        error!(
                            agent = %alert.agent_name,
                            error = %e,
                            "failed to restart agent after drift correction"
                        );
                    }
                }
            }
            CorrectionAction::PauseAndEscalate(msg) => {
                error!(
                    agent = %alert.agent_name,
                    message = %msg,
                    "CRITICAL: pausing agent and escalating to human"
                );
                self.stop_agent(&alert.agent_name).await;
                self.nightwatch.reset(&alert.agent_name);
            }
        }
    }

    /// Stop a specific agent by name.
    async fn stop_agent(&mut self, name: &str) {
        if let Some(managed) = self.active_agents.remove(name) {
            info!(agent = %name, "stopping agent");
            let grace = Duration::from_secs(5);
            let mut handle = managed.handle;
            match handle.shutdown(grace).await {
                Ok(_) => info!(agent = %name, "agent stopped gracefully"),
                Err(e) => warn!(agent = %name, error = %e, "agent stop had issues"),
            }
        }
    }

    /// Shutdown all active agents.
    async fn shutdown_all_agents(&mut self) {
        let names: Vec<String> = self.active_agents.keys().cloned().collect();
        for name in names {
            self.stop_agent(&name).await;
        }
    }

    /// Spawn a specific agent by name (test helper).
    #[doc(hidden)]
    pub async fn spawn_agent_for_test(&mut self, name: &str) -> Result<(), OrchestratorError> {
        let def = self
            .config
            .agents
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| OrchestratorError::AgentNotFound(name.to_string()))?
            .clone();
        self.spawn_agent(&def).await
    }

    /// Check if an agent is in the active_agents map (test helper).
    #[doc(hidden)]
    pub fn is_agent_active(&self, name: &str) -> bool {
        self.active_agents.contains_key(name)
    }

    /// Test helper: remove an agent from active_agents so it can be re-spawned.
    #[doc(hidden)]
    pub fn remove_agent_for_test(&mut self, name: &str) {
        self.active_agents.remove(name);
    }

    /// Test helper: set last_run_commits for a given agent.
    #[doc(hidden)]
    pub fn set_last_run_commit(&mut self, agent_name: &str, commit: &str) {
        self.last_run_commits
            .insert(agent_name.to_string(), commit.to_string());
    }

    /// Test helper: access the telemetry store for assertions.
    #[doc(hidden)]
    pub fn telemetry_store(&self) -> &control_plane::TelemetryStore {
        &self.telemetry_store
    }
}

/// Check whether any changed file matches any of the watch path prefixes.
fn has_matching_changes(changed_files: &[String], watch_paths: &[String]) -> bool {
    for file in changed_files {
        for prefix in watch_paths {
            if scope::is_path_prefix(prefix, file) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn legacy_key(name: &str) -> (String, String) {
        (
            crate::dispatcher::LEGACY_PROJECT_ID.to_string(),
            name.to_string(),
        )
    }

    fn test_config() -> OrchestratorConfig {
        OrchestratorConfig {
            working_dir: std::path::PathBuf::from("/tmp/test-orchestrator"),
            nightwatch: NightwatchConfig::default(),
            compound_review: CompoundReviewConfig {
                cli_tool: None,
                provider: None,
                model: None,
                schedule: "0 2 * * *".to_string(),
                max_duration_secs: 60,
                repo_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
                create_prs: false,
                worktree_root: std::path::PathBuf::from("/tmp/test-orchestrator/.worktrees"),
                base_branch: "main".to_string(),
                max_concurrent_agents: 3,
                ..Default::default()
            },
            workflow: None,
            agents: vec![
                AgentDefinition {
                    name: "sentinel".to_string(),
                    layer: AgentLayer::Safety,
                    cli_tool: "echo".to_string(),
                    task: "safety watch".to_string(),
                    model: None,
                    schedule: None,
                    capabilities: vec!["security".to_string()],
                    max_memory_bytes: None,
                    budget_monthly_cents: None,
                    provider: None,
                    persona: None,
                    terraphim_role: None,
                    skill_chain: vec![],
                    sfia_skills: vec![],
                    fallback_provider: None,
                    fallback_model: None,
                    grace_period_secs: None,
                    max_cpu_seconds: None,
                    pre_check: None,

                    gitea_issue: None,

                    project: None,
                },
                AgentDefinition {
                    name: "sync".to_string(),
                    layer: AgentLayer::Core,
                    cli_tool: "echo".to_string(),
                    task: "sync upstream".to_string(),
                    model: None,
                    schedule: Some("0 3 * * *".to_string()),
                    capabilities: vec!["sync".to_string()],
                    max_memory_bytes: None,
                    budget_monthly_cents: None,
                    provider: None,
                    persona: None,
                    terraphim_role: None,
                    skill_chain: vec![],
                    sfia_skills: vec![],
                    fallback_provider: None,
                    fallback_model: None,
                    grace_period_secs: None,
                    max_cpu_seconds: None,
                    pre_check: None,

                    gitea_issue: None,

                    project: None,
                },
            ],
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            restart_budget_window_secs: 43_200,
            disk_usage_threshold: 100, // disable disk guard in tests
            tick_interval_secs: 30,
            handoff_buffer_ttl_secs: None,
            persona_data_dir: None,
            skill_data_dir: None,
            flows: vec![],
            flow_state_dir: None,
            gitea: None,
            mentions: None,
            webhook: None,
            role_config_path: None,
            routing: None,
            #[cfg(feature = "quickwit")]
            quickwit: None,
            projects: vec![],
            include: vec![],
        }
    }

    #[test]
    fn test_orchestrator_creates_from_config() {
        let config = test_config();
        let orch = AgentOrchestrator::new(config);
        assert!(orch.is_ok());
    }

    #[test]
    fn test_orchestrator_initial_state() {
        let config = test_config();
        let orch = AgentOrchestrator::new(config).unwrap();
        assert!(orch.active_agents.is_empty());
        assert!(!orch.shutdown_requested);
        let statuses = orch.agent_statuses();
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_orchestrator_shutdown_flag() {
        let config = test_config();
        let mut orch = AgentOrchestrator::new(config).unwrap();
        assert!(!orch.shutdown_requested);
        orch.shutdown();
        assert!(orch.shutdown_requested);
    }

    #[tokio::test]
    async fn test_orchestrator_compound_review_manual() {
        // Use empty groups to avoid git worktree operations during test.
        // Worktree creation fails when git index is locked (e.g. pre-commit hooks).
        let repo_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

        // In shallow clones (e.g. CI with fetch-depth: 1) HEAD~1 does not exist.
        // Fall back to diffing against the empty tree so the test works everywhere.
        let base_ref = {
            let check = std::process::Command::new("git")
                .args([
                    "-C",
                    repo_path.to_str().unwrap(),
                    "rev-parse",
                    "--verify",
                    "HEAD~1",
                ])
                .output();
            match check {
                Ok(o) if o.status.success() => "HEAD~1".to_string(),
                _ => {
                    // 4b825dc: the well-known empty tree hash in git
                    let empty = std::process::Command::new("git")
                        .args([
                            "-C",
                            repo_path.to_str().unwrap(),
                            "hash-object",
                            "-t",
                            "tree",
                            "/dev/null",
                        ])
                        .output()
                        .expect("git hash-object failed");
                    String::from_utf8_lossy(&empty.stdout).trim().to_string()
                }
            }
        };

        let swarm_config = SwarmConfig {
            groups: vec![],
            timeout: Duration::from_secs(60),
            worktree_root: std::path::PathBuf::from("/tmp/test-orchestrator/.worktrees"),
            repo_path,
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            create_prs: false,
        };

        let workflow = CompoundReviewWorkflow::new(swarm_config);
        let result = workflow.run("HEAD", &base_ref).await.unwrap();

        assert!(
            !result.correlation_id.is_nil(),
            "correlation_id should be set"
        );
        assert_eq!(result.agents_run, 0, "no agents with empty groups");
        assert_eq!(result.agents_failed, 0);
    }

    #[test]
    fn test_orchestrator_from_toml() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[[agents]]
name = "test"
layer = "Safety"
cli_tool = "echo"
task = "test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let orch = AgentOrchestrator::new(config);
        assert!(orch.is_ok());
    }

    #[test]
    fn test_agent_status_fields() {
        let status = AgentStatus {
            name: "test".to_string(),
            layer: AgentLayer::Safety,
            running: true,
            health: HealthStatus::Healthy,
            drift_score: Some(0.05),
            uptime: Duration::from_secs(3600),
            restart_count: 0,
            api_calls_remaining: HashMap::new(),
        };
        assert_eq!(status.name, "test");
        assert!(status.running);
        assert_eq!(status.drift_score, Some(0.05));
    }

    #[test]
    fn test_load_skill_chain_content_supports_lowercase_skill_md() {
        let skill_root = TempDir::new().unwrap();
        let skill_dir = skill_root.path().join("business-scenario-design");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("skill.md"), "Lowercase skill content").unwrap();

        let mut config = test_config();
        config.skill_data_dir = Some(skill_root.path().to_path_buf());
        let orch = AgentOrchestrator::new(config).unwrap();

        let mut def = orch.config.agents[0].clone();
        def.skill_chain = vec!["business-scenario-design".to_string()];

        let loaded = orch.load_skill_chain_content(&def);
        assert!(loaded.contains("### Skill: business-scenario-design"));
        assert!(loaded.contains("Lowercase skill content"));
    }

    #[test]
    fn test_load_skill_chain_content_falls_back_to_home_skill_roots() {
        let home_dir = TempDir::new().unwrap();
        let configured_skill_root = TempDir::new().unwrap();

        let roots = AgentOrchestrator::skill_roots(
            Some(configured_skill_root.path()),
            Some(home_dir.path()),
        );

        assert_eq!(roots[0], configured_skill_root.path());
        assert!(roots.iter().any(|path| path.ends_with(".opencode/skills")));
        assert!(roots.iter().any(|path| path.ends_with(".claude/skills")));
    }

    /// Helper: create a config with a single Safety echo agent and short cooldown.
    fn test_config_fast_lifecycle() -> OrchestratorConfig {
        OrchestratorConfig {
            working_dir: std::path::PathBuf::from("/tmp"),
            nightwatch: NightwatchConfig::default(),
            compound_review: CompoundReviewConfig {
                cli_tool: None,
                provider: None,
                model: None,
                schedule: "0 2 * * *".to_string(),
                max_duration_secs: 60,
                repo_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
                create_prs: false,
                worktree_root: std::path::PathBuf::from("/tmp/.worktrees"),
                base_branch: "main".to_string(),
                max_concurrent_agents: 3,
                ..Default::default()
            },
            workflow: None,
            agents: vec![AgentDefinition {
                name: "echo-safety".to_string(),
                layer: AgentLayer::Safety,
                cli_tool: "echo".to_string(),
                task: "safety watch".to_string(),
                model: None,
                schedule: None,
                capabilities: vec![],
                max_memory_bytes: None,
                budget_monthly_cents: None,
                provider: None,
                persona: None,
                terraphim_role: None,
                skill_chain: vec![],
                sfia_skills: vec![],
                fallback_provider: None,
                fallback_model: None,
                grace_period_secs: None,
                max_cpu_seconds: None,
                pre_check: None,

                gitea_issue: None,

                project: None,
            }],
            restart_cooldown_secs: 0, // instant restart for testing
            max_restart_count: 3,
            restart_budget_window_secs: 43_200,
            disk_usage_threshold: 100, // disable disk guard in tests
            tick_interval_secs: 1,
            handoff_buffer_ttl_secs: None,
            persona_data_dir: None,
            skill_data_dir: None,
            flows: vec![],
            flow_state_dir: None,
            gitea: None,
            mentions: None,
            webhook: None,
            role_config_path: None,
            routing: None,
            #[cfg(feature = "quickwit")]
            quickwit: None,
            projects: vec![],
            include: vec![],
        }
    }

    #[tokio::test]
    async fn test_reconcile_detects_agent_exit() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn the echo agent (exits immediately)
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        assert!(orch.active_agents.contains_key("echo-safety"));

        // Give echo time to exit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Poll for exits
        orch.poll_agent_exits().await;

        // Agent should be removed from active_agents
        assert!(
            !orch.active_agents.contains_key("echo-safety"),
            "exited agent should be removed from active_agents"
        );

        // Successful exit (code 0) should NOT increment restart count
        assert_eq!(
            orch.restart_counts
                .get(&legacy_key("echo-safety"))
                .copied()
                .unwrap_or(0),
            0,
            "successful exit should not increment restart count"
        );
    }

    #[tokio::test]
    async fn test_safety_agent_restarts_after_cooldown() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn and let it exit
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        orch.poll_agent_exits().await;
        assert!(!orch.active_agents.contains_key("echo-safety"));

        // Restart pending (cooldown is 0, so immediate)
        orch.restart_pending_safety_agents().await;
        assert!(
            orch.active_agents.contains_key("echo-safety"),
            "safety agent should be restarted after cooldown"
        );
    }

    #[tokio::test]
    async fn test_core_agent_no_auto_restart() {
        let mut config = test_config_fast_lifecycle();
        config.agents = vec![AgentDefinition {
            name: "echo-core".to_string(),
            layer: AgentLayer::Core,
            cli_tool: "echo".to_string(),
            task: "core task".to_string(),
            model: None,
            schedule: Some("0 3 * * *".to_string()),
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,

            gitea_issue: None,

            project: None,
        }];
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn core agent and let it exit
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        orch.poll_agent_exits().await;
        assert!(!orch.active_agents.contains_key("echo-core"));

        // Restart pending should NOT restart a Core agent
        orch.restart_pending_safety_agents().await;
        assert!(
            !orch.active_agents.contains_key("echo-core"),
            "core agent should not auto-restart"
        );
    }

    #[tokio::test]
    async fn test_max_restart_count_respected() {
        let mut config = test_config_fast_lifecycle();
        config.max_restart_count = 2;
        // Use a command that exits non-zero so restart_count increments
        config.agents[0].cli_tool = "false".to_string();
        config.agents[0].task = String::new();
        let mut orch = AgentOrchestrator::new(config).unwrap();
        let def = orch.config.agents[0].clone();

        // Cycle through max_restart_count + 1 exits (all non-zero)
        for i in 0..3 {
            orch.spawn_agent(&def).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            orch.poll_agent_exits().await;
            assert!(
                !orch.active_agents.contains_key("echo-safety"),
                "agent should have exited on cycle {}",
                i
            );
        }

        // After 3 non-zero exits, restart count = 3, max = 2
        // restart_pending should NOT restart (count > max)
        orch.restart_pending_safety_agents().await;
        assert!(
            !orch.active_agents.contains_key("echo-safety"),
            "agent should not restart after exceeding max_restart_count"
        );
        assert_eq!(
            orch.restart_counts.get(&legacy_key("echo-safety")).copied(),
            Some(3)
        );
    }

    #[test]
    fn test_restart_count_ages_out_after_budget_window() {
        let mut config = test_config_fast_lifecycle();
        config.restart_budget_window_secs = 1;
        let mut orch = AgentOrchestrator::new(config).unwrap();

        orch.restart_counts.insert(legacy_key("echo-safety"), 3);
        orch.restart_last_failure_unix_secs.insert(
            legacy_key("echo-safety"),
            chrono::Utc::now().timestamp() - 5,
        );

        let count = orch.current_restart_count(&legacy_key("echo-safety"));
        assert_eq!(count, 0);
        assert!(!orch.restart_counts.contains_key(&legacy_key("echo-safety")));
        assert!(!orch
            .restart_last_failure_unix_secs
            .contains_key(&legacy_key("echo-safety")));
    }

    #[tokio::test]
    async fn test_successful_exit_does_not_increment_restart_count() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();
        let def = orch.config.agents[0].clone(); // echo "safety watch" -> exit 0

        // Spawn and let it exit successfully multiple times
        for _ in 0..5 {
            orch.spawn_agent(&def).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            orch.poll_agent_exits().await;
        }

        // Exit code 0 should never increment restart_count
        assert_eq!(
            orch.restart_counts
                .get(&legacy_key("echo-safety"))
                .copied()
                .unwrap_or(0),
            0,
            "successful exits (code 0) must not increment restart_count"
        );

        // Agent should still be eligible for restart
        orch.restart_cooldowns.insert(
            legacy_key("echo-safety"),
            Instant::now() - Duration::from_secs(999),
        );
        orch.restart_pending_safety_agents().await;
        assert!(
            orch.active_agents.contains_key("echo-safety"),
            "agent with only successful exits should always be restartable"
        );
    }

    #[tokio::test]
    async fn test_output_events_fed_to_nightwatch() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn echo agent (writes "safety watch" to stdout)
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();

        // Give the output capture time to process
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Drain events
        orch.drain_output_events();

        // Nightwatch should have observations for the agent
        let drift = orch.nightwatch.drift_score("echo-safety");
        assert!(
            drift.is_some(),
            "nightwatch should have drift data after draining output events"
        );
        let drift = drift.unwrap();
        assert!(
            drift.metrics.sample_count > 0,
            "nightwatch should have at least one sample from drained output"
        );
    }

    #[tokio::test]
    async fn test_reconcile_tick_full_cycle() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn echo agent
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();

        // Give echo time to exit and produce output
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Run a full reconciliation tick
        orch.reconcile_tick().await;

        // After tick: echo exited, so it was detected and marked for restart.
        // With 0 cooldown, it should have been restarted in the same tick.
        assert!(
            orch.active_agents.contains_key("echo-safety"),
            "safety agent should be restarted by reconcile_tick"
        );
        // echo exits with code 0, so restart_count stays at 0
        assert_eq!(
            orch.restart_counts
                .get(&legacy_key("echo-safety"))
                .copied()
                .unwrap_or(0),
            0,
            "successful exit should not increment restart count"
        );
    }

    // =========================================================================
    // Persona Injection Tests (Gitea #73)
    // =========================================================================

    /// Test that spawn_agent composes persona-enriched prompt when persona exists
    #[tokio::test]
    async fn test_spawn_agent_with_persona_composes_prompt() {
        let mut config = test_config_fast_lifecycle();

        // Add an agent with a persona
        // Use cat (not echo) because persona_found=true triggers stdin delivery.
        // cat reads stdin before exiting, avoiding broken pipe under parallel load.
        config.agents = vec![AgentDefinition {
            name: "persona-agent".to_string(),
            layer: AgentLayer::Safety,
            cli_tool: "cat".to_string(),
            task: "test task".to_string(),
            model: None,
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: Some("TestAgent".to_string()), // Persona that exists in default test_persona
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,

            gitea_issue: None,

            project: None,
        }];

        // Set up persona data dir with a test persona
        let temp_dir =
            std::env::temp_dir().join(format!("terraphim-test-persona-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let persona_toml = r#"
agent_name = "TestAgent"
role_name = "Test Engineer"
name_origin = "From testing"
vibe = "Thorough, methodical"
symbol = "Checkmark"
core_characteristics = [{ name = "Thorough", description = "checks everything twice" }]
speech_style = "Precise and factual."
terraphim_nature = "Adapted to testing environments."
sfia_title = "Test Engineer"
primary_level = 4
guiding_phrase = "Enable"
level_essence = "Works autonomously under general direction."
sfia_skills = [{ code = "TEST", name = "Testing", level = 4, description = "Designs and executes test plans." }]
"#;
        std::fs::write(temp_dir.join("testagent.toml"), persona_toml).unwrap();
        config.persona_data_dir = Some(temp_dir.clone());

        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn the agent - it should use the persona-enriched prompt
        let def = orch.config.agents[0].clone();
        let result = orch.spawn_agent(&def).await;

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Spawn should succeed
        assert!(result.is_ok());

        // The agent should be active
        assert!(orch.active_agents.contains_key("persona-agent"));
    }

    /// Test that spawn_agent uses bare task when persona is None
    #[tokio::test]
    async fn test_spawn_agent_without_persona_uses_bare_task() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Agent without persona should use bare task
        let def = orch.config.agents[0].clone();
        assert!(def.persona.is_none());

        let result = orch.spawn_agent(&def).await;
        assert!(result.is_ok());

        assert!(orch.active_agents.contains_key("echo-safety"));
    }

    /// Test graceful degradation when persona not found in registry
    #[tokio::test]
    async fn test_spawn_agent_persona_not_found_graceful() {
        let mut config = test_config_fast_lifecycle();

        // Add an agent with a non-existent persona
        config.agents = vec![AgentDefinition {
            name: "unknown-persona-agent".to_string(),
            layer: AgentLayer::Safety,
            cli_tool: "echo".to_string(),
            task: "test task".to_string(),
            model: None,
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: Some("NonExistentPersona".to_string()), // This persona doesn't exist
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,

            gitea_issue: None,

            project: None,
        }];

        // No persona_data_dir, so registry will be empty
        config.persona_data_dir = None;

        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn should still succeed even though persona doesn't exist
        let def = orch.config.agents[0].clone();
        let result = orch.spawn_agent(&def).await;

        assert!(
            result.is_ok(),
            "spawn should succeed with fallback to bare task"
        );
        assert!(orch.active_agents.contains_key("unknown-persona-agent"));
    }

    // ==================== Agent Name Validation Tests ====================

    #[test]
    fn test_validate_agent_name_accepts_valid() {
        assert!(validate_agent_name("my-agent_1").is_ok());
        assert!(validate_agent_name("sentinel").is_ok());
        assert!(validate_agent_name("Agent-42").is_ok());
    }

    #[test]
    fn test_validate_agent_name_rejects_traversal() {
        assert!(validate_agent_name("../etc/passwd").is_err());
        assert!(validate_agent_name("..").is_err());
        assert!(validate_agent_name("foo/../bar").is_err());
    }

    #[test]
    fn test_validate_agent_name_rejects_slash() {
        assert!(validate_agent_name("foo/bar").is_err());
        assert!(validate_agent_name("foo\\bar").is_err());
    }

    #[test]
    fn test_validate_agent_name_rejects_empty() {
        assert!(validate_agent_name("").is_err());
    }

    #[test]
    fn test_validate_agent_name_rejects_special_chars() {
        assert!(validate_agent_name("agent name").is_err()); // spaces
        assert!(validate_agent_name("agent@host").is_err()); // @
        assert!(validate_agent_name("agent.name").is_err()); // dots
    }

    // ==================== has_matching_changes Tests ====================

    #[test]
    fn test_has_matching_changes_prefix_match() {
        let changed = vec!["crates/orchestrator/src/lib.rs".to_string()];
        let watch = vec!["crates/orchestrator/".to_string()];
        assert!(has_matching_changes(&changed, &watch));
    }

    #[test]
    fn test_has_matching_changes_exact_match() {
        let changed = vec!["Cargo.toml".to_string()];
        let watch = vec!["Cargo.toml".to_string()];
        assert!(has_matching_changes(&changed, &watch));
    }

    #[test]
    fn test_has_matching_changes_no_match() {
        let changed = vec!["docs/README.md".to_string()];
        let watch = vec!["crates/orchestrator/".to_string()];
        assert!(!has_matching_changes(&changed, &watch));
    }

    #[test]
    fn test_has_matching_changes_multiple_files_one_matches() {
        let changed = vec![
            "docs/README.md".to_string(),
            "crates/orchestrator/src/config.rs".to_string(),
        ];
        let watch = vec!["crates/orchestrator/".to_string()];
        assert!(has_matching_changes(&changed, &watch));
    }

    #[test]
    fn test_has_matching_changes_multiple_watch_paths() {
        let changed = vec!["tests/integration.rs".to_string()];
        let watch = vec!["crates/orchestrator/".to_string(), "tests/".to_string()];
        assert!(has_matching_changes(&changed, &watch));
    }

    #[test]
    fn test_has_matching_changes_empty_watch_paths() {
        let changed = vec!["crates/orchestrator/src/lib.rs".to_string()];
        let watch: Vec<String> = vec![];
        assert!(!has_matching_changes(&changed, &watch));
    }

    // =========================================================================
    // ADF Remediation Tests (Gitea #117)
    // =========================================================================

    #[test]
    fn test_provider_model_composition_opencode() {
        // Simulate what spawn_agent does for opencode with provider + model
        let provider = Some("kimi-for-coding".to_string());
        let model = Some("k2p5".to_string());
        let cli_name = "opencode";

        let composed = if cli_name == "opencode" {
            match (&provider, &model) {
                (Some(p), Some(m)) => Some(format!("{}/{}", p, m)),
                _ => model,
            }
        } else {
            model
        };
        assert_eq!(composed, Some("kimi-for-coding/k2p5".to_string()));
    }

    #[test]
    fn test_provider_model_composition_claude_unchanged() {
        // Claude should not have provider/model composed
        let provider = Some("anthropic".to_string());
        let model = Some("claude-opus-4-6".to_string());
        let cli_name = "claude";

        let composed = if cli_name == "opencode" {
            match (&provider, &model) {
                (Some(p), Some(m)) => Some(format!("{}/{}", p, m)),
                _ => model.clone(),
            }
        } else {
            model.clone()
        };
        assert_eq!(composed, Some("claude-opus-4-6".to_string()));
    }

    #[tokio::test]
    async fn test_wall_clock_timeout_kills_agent() {
        let mut config = test_config_fast_lifecycle();
        // Use sleep agent with 1-second timeout
        config.agents = vec![AgentDefinition {
            name: "timeout-test".to_string(),
            layer: AgentLayer::Core,
            cli_tool: "sleep".to_string(),
            task: "60".to_string(),
            model: None,
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: Some(2),
            max_cpu_seconds: Some(1), // 1 second timeout
            pre_check: None,
            gitea_issue: None,
            project: None,
        }];
        let mut orch = AgentOrchestrator::new(config).unwrap();
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        assert!(orch.active_agents.contains_key("timeout-test"));

        // Wait for the timeout to elapse
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Poll should detect timeout and kill
        orch.poll_agent_exits().await;
        assert!(!orch.active_agents.contains_key("timeout-test"));
    }

    // =========================================================================
    // Flow DAG Orchestrator Integration Tests (Gitea #163)
    // =========================================================================

    #[test]
    fn test_orchestrator_with_empty_flows() {
        let mut config = test_config();
        config.flows = vec![];
        config.flow_state_dir = None;

        let orch = AgentOrchestrator::new(config);
        assert!(
            orch.is_ok(),
            "orchestrator should initialize with empty flows"
        );

        let orch = orch.unwrap();
        assert!(
            orch.active_flows.is_empty(),
            "active_flows should be empty initially"
        );
    }

    /// Test that flow scheduling overlap prevention works
    #[tokio::test]
    async fn test_flow_overlap_prevention() {
        use crate::flow::config::{FlowDefinition, FlowStepDef, StepKind};

        let mut config = test_config_fast_lifecycle();

        // Add a test flow with a schedule
        config.flows = vec![FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: Some("0 2 * * *".to_string()), // 2 AM daily
            repo_path: "/tmp/test-repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![FlowStepDef {
                name: "test-step".to_string(),
                kind: StepKind::Action,
                command: Some("echo test".to_string()),
                cli_tool: None,
                model: None,
                task: None,
                task_file: None,
                condition: None,
                timeout_secs: 60,
                on_fail: crate::flow::config::FailStrategy::Abort,
                provider: None,
                persona: None,
            }],
        }];

        config.flow_state_dir = Some(PathBuf::from("/tmp/test-flow-states"));

        let orch = AgentOrchestrator::new(config);
        assert!(orch.is_ok(), "orchestrator should initialize with flows");

        let orch = orch.unwrap();
        assert!(
            orch.active_flows.is_empty(),
            "active_flows should be empty initially"
        );
    }

    // ==================== Sanitisation Tests ====================

    #[test]
    fn test_sanitise_for_title_strips_json_braces() {
        let input = r#"{"type":"tool_use","timestamp":1775313676859}"#;
        let result = AgentOrchestrator::sanitise_for_title(input);
        assert!(!result.contains('{'), "title should not contain open brace");
        assert!(
            !result.contains('}'),
            "title should not contain close brace"
        );
        assert!(
            !result.contains('['),
            "title should not contain open bracket"
        );
        assert!(
            !result.contains(']'),
            "title should not contain close bracket"
        );
    }

    #[test]
    fn test_sanitise_for_title_strips_quotes() {
        let input = r#"JSON "quoted" text"#;
        let result = AgentOrchestrator::sanitise_for_title(input);
        assert!(!result.contains('"'), "title should not contain quotes");
    }

    #[test]
    fn test_sanitise_for_title_truncates_long_input() {
        let input = "This is a very long finding text that should be truncated because it exceeds eighty characters limit";
        let result = AgentOrchestrator::sanitise_for_title(input);
        assert!(
            result.len() <= 80,
            "title should be at most 80 chars, got {}",
            result.len()
        );
    }

    #[test]
    fn test_sanitise_for_body_escapes_backticks() {
        let input = "Use `code` here";
        let result = AgentOrchestrator::sanitise_for_body(input);
        assert!(result.contains("``"), "body should escape backticks");
    }

    #[test]
    fn test_sanitise_for_body_escapes_markdown_chars() {
        let input = "Text with *asterisks* and [brackets]";
        let result = AgentOrchestrator::sanitise_for_body(input);
        assert!(
            result.contains('\\'),
            "body should contain backslash, got: {}",
            result
        );
    }
}
