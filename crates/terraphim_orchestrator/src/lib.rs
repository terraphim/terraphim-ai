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

pub mod compound;
pub mod concurrency;
pub mod config;
pub mod cost_tracker;
pub mod dispatcher;
pub mod dual_mode;
pub mod error;
pub mod flow;
pub mod handoff;
pub mod mention;
pub mod metrics_persistence;
pub mod mode;
pub mod nightwatch;
pub mod persona;
pub mod output_poster;
pub mod scheduler;
pub mod scope;

pub use compound::{CompoundReviewResult, CompoundReviewWorkflow, ReviewGroupDef, SwarmConfig};
pub use concurrency::{ConcurrencyController, FairnessPolicy, ModeQuotas};
pub use config::{
    AgentDefinition, AgentLayer, CompoundReviewConfig, ConcurrencyConfig, GiteaOutputConfig,
    MentionConfig, NightwatchConfig, OrchestratorConfig, PreCheckStrategy, TrackerConfig,
    TrackerStates, WorkflowConfig,
};
pub use cost_tracker::{AgentMetrics, BudgetVerdict, CostSnapshot, CostTracker, ExecutionMetrics};
pub use mention::{parse_mentions, DetectedMention, MentionThesaurus, MentionTracker};
pub use dispatcher::{DispatchTask, Dispatcher, DispatcherStats};
pub use dual_mode::DualModeOrchestrator;
pub use error::OrchestratorError;
pub use handoff::{HandoffBuffer, HandoffContext, HandoffLedger};
pub use metrics_persistence::{
    InMemoryMetricsPersistence, MetricsPersistence, MetricsPersistenceConfig,
    MetricsPersistenceError, PersistedAgentMetrics,
};
pub use mode::{IssueMode, TimeMode};
pub use output_poster::OutputPoster;
pub use nightwatch::{
    dual_panel_evaluate, validate_certificate, Claim, CorrectionAction, CorrectionLevel,
    DriftAlert, DriftMetrics, DriftScore, DualPanelResult, NightwatchMonitor, RateLimitTracker,
    RateLimitWindow, ReasoningCertificate,
};
pub use persona::{MetapromptRenderError, MetapromptRenderer, PersonaRegistry};
pub use scheduler::{ScheduleEvent, TimeScheduler};

use chrono::Timelike;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, Instant};

use std::sync::{Arc, Mutex};

use terraphim_router::RoutingEngine;
use terraphim_spawner::health::{CircuitBreaker, HealthStatus};
use terraphim_spawner::output::OutputEvent;
use terraphim_spawner::{AgentHandle, AgentSpawner, ResourceLimits, SpawnRequest};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

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
    /// Total restart count per agent (persists across agent lifecycle).
    restart_counts: HashMap<String, u32>,
    /// Last exit time per agent (for cooldown enforcement).
    restart_cooldowns: HashMap<String, Instant>,
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
    /// Tracker for processed @adf: mentions (dedup + depth limiting).
    mention_tracker: MentionTracker,
    /// Pre-built capability Thesauri for agent mention scoring.
    cap_thesauri: mention::CapabilityThesauri,
    /// Pre-built mention Thesaurus for efficient @adf: mention detection.
    mention_thesaurus: mention::MentionThesaurus,
    /// Monotonically increasing tick counter for poll_modulo gating.
    tick_count: u64,
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
        let spawner = AgentSpawner::new().with_working_dir(&config.working_dir);
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

        // Initialize output poster if Gitea config is provided
        let output_poster = config.gitea.as_ref().map(OutputPoster::new);

        // Initialize mention tracker with configured depth limit (default 3)
        let max_mention_depth = config
            .mentions
            .as_ref()
            .map(|m| m.max_mention_depth)
            .unwrap_or(3);
        let mention_tracker = MentionTracker::new(max_mention_depth);

        // Pre-build capability Thesauri for agent mention scoring
        let cap_thesauri = mention::CapabilityThesauri::build(&config.agents);

        // Pre-build mention Thesaurus for efficient @adf: mention detection
        let mention_thesaurus = mention::MentionThesaurus::build(&config.agents, &persona_registry);

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
            restart_counts: HashMap::new(),
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
            mention_tracker,
            cap_thesauri,
            mention_thesaurus,
            tick_count: 0,
        })
    }

    /// Create from a TOML config file path.
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, OrchestratorError> {
        let config = OrchestratorConfig::from_file(path)?;
        Self::new(config)
    }

    /// Initialize persistence and load saved MentionTracker state.
    /// Call this after construction and before run().
    pub async fn init_persistence(&mut self) {
        match terraphim_persistence::DeviceStorage::instance().await {
            Ok(_) => {
                self.mention_tracker = MentionTracker::load_or_new(self.mention_tracker.max_depth()).await;
            }
            Err(e) => {
                tracing::warn!(?e, "DeviceStorage init failed, MentionTracker will not persist");
            }
        }
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

        // Reconciliation tick interval
        let mut tick = tokio::time::interval(Duration::from_secs(self.config.tick_interval_secs));

        // Main reconciliation loop
        loop {
            if self.shutdown_requested {
                info!("shutdown requested, stopping reconciliation loop");
                break;
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

    /// Load skill chain content from SKILL.md files for the given agent definition.
    ///
    /// Reads each skill named in `def.skill_chain` from `{skill_data_dir}/{name}/SKILL.md`.
    /// Returns a formatted string with all skill contents, or empty string if no skills
    /// or no skill_data_dir is configured.
    fn load_skill_chain_content(&self, def: &AgentDefinition) -> String {
        if def.skill_chain.is_empty() {
            return String::new();
        }
        let skills_dir = match &self.config.skill_data_dir {
            Some(dir) => dir.clone(),
            None => {
                // Default: ~/.claude/skills (resolve HOME from env)
                match std::env::var("HOME") {
                    Ok(home) => std::path::PathBuf::from(home)
                        .join(".claude")
                        .join("skills"),
                    Err(_) => return String::new(),
                }
            }
        };

        let mut sections = Vec::new();
        for skill_name in &def.skill_chain {
            let skill_path = skills_dir.join(skill_name).join("SKILL.md");
            match std::fs::read_to_string(&skill_path) {
                Ok(content) => {
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
                        bytes = body.len(),
                        "loaded skill content"
                    );
                }
                Err(e) => {
                    warn!(
                        agent = %def.name,
                        skill = %skill_name,
                        path = %skill_path.display(),
                        error = %e,
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

    /// Spawn an agent from its definition.
    ///
    /// Model selection: if the agent has an explicit `model` field, use it.
    /// Otherwise, route the task prompt through the RoutingEngine to select
    /// a model based on keyword matching.
    async fn spawn_agent(&mut self, def: &AgentDefinition) -> Result<(), OrchestratorError> {
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

        let model = if let Some(m) = &def.model {
            info!(agent = %def.name, model = %m, "using explicit model");
            Some(m.clone())
        } else if supports_model_flag {
            // Route the task prompt to find the best model
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
                    info!(agent = %def.name, "no model matched via routing, using CLI default");
                    None
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

        info!(agent = %def.name, layer = ?def.layer, cli = %def.cli_tool, model = ?model, "spawning agent");

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

        // Build primary Provider from the agent definition for the spawner
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: def.cli_tool.clone(),
                working_dir: self.config.working_dir.clone(),
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
                    working_dir: self.config.working_dir.clone(),
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

        let handle = self
            .spawner
            .spawn_with_fallback(&request)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        // Subscribe to the output broadcast for nightwatch drain
        let output_rx = handle.subscribe_output();

        // Get the restart count from the orchestrator-level counter
        let restart_count = self.restart_counts.get(&def.name).copied().unwrap_or(0);

        self.active_agents.insert(
            def.name.clone(),
            ManagedAgent {
                definition: def.clone(),
                handle,
                started_at: Instant::now(),
                restart_count,
                output_rx,
            },
        );

        // === RECORD COMMIT FOR GIT-DIFF STRATEGY ===
        if let Ok(head) = self.get_current_head().await {
            self.last_run_commits.insert(def.name.clone(), head);
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

        let tracker_cfg = terraphim_tracker::GiteaConfig {
            base_url: gitea_cfg.base_url.clone(),
            token: gitea_cfg.token.clone(),
            owner: gitea_cfg.owner.clone(),
            repo: gitea_cfg.repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
        };
        let tracker = match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "failed to create GiteaTracker for mention polling");
                return;
            }
        };

        let agents = self.config.agents.clone();
        let persona_registry = self.persona_registry.clone();

        for &issue_number in &mention_cfg.watch_issues {
            if self.mention_tracker.depth_exceeded(issue_number) {
                tracing::debug!(issue = issue_number, "mention depth exceeded, skipping");
                continue;
            }

            match tracker.fetch_comments(issue_number, None).await {
                Ok(comments) => {
                    for comment in &comments {
                        let detected = mention::parse_mentions(
                            comment,
                            issue_number,
                            &agents,
                            &persona_registry,
                            &self.mention_thesaurus,
                            &self.cap_thesauri,
                        );

                        for m in detected {
                            if self.mention_tracker.is_processed(&m) {
                                continue;
                            }

                            tracing::info!(
                                agent = %m.agent_name,
                                issue = m.issue_number,
                                comment_id = m.comment_id,
                                "dispatching mention-driven task"
                            );

                            // Spawn the mentioned agent
                            if let Some(def) = agents.iter().find(|a| a.name == m.agent_name).cloned() {
                                let mut mention_def = def.clone();
                                // Append mention context
                                mention_def.task = format!(
                                    "{}

## Mention Context
Triggered by @adf:{} mention in issue #{} (comment {}).
Comment: {}",
                                    def.task,
                                    m.agent_name,
                                    m.issue_number,
                                    m.comment_id,
                                    if m.comment_body.len() > 500 { &m.comment_body[..500] } else { &m.comment_body }
                                );
                                // Post output back to the same issue
                                mention_def.gitea_issue = Some(m.issue_number);

                                if let Err(e) = self.spawn_agent(&mention_def).await {
                                    tracing::error!(
                                        agent = %m.agent_name,
                                        issue = m.issue_number,
                                        error = %e,
                                        "failed to spawn mentioned agent"
                                    );
                                }
                            }

                            self.mention_tracker.mark_processed(&m);
                            self.mention_tracker.increment_depth(issue_number);
                            self.mention_tracker.persist().await;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        issue = issue_number,
                        error = %e,
                        "failed to fetch comments for mention polling"
                    );
                }
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

        // 4. Drain output events to nightwatch
        self.drain_output_events();

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
        let completed_flows: Vec<String> = self.active_flows
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

        // 12. Update last_tick_time and increment tick counter
        self.last_tick_time = chrono::Utc::now();
        self.tick_count = self.tick_count.wrapping_add(1);
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
                    let count = self.restart_counts.entry(name.clone()).or_insert(0);
                    *count += 1;
                    self.restart_cooldowns.insert(name.clone(), Instant::now());
                    info!(
                        agent = %name,
                        restart_count = *count,
                        "safety agent timed out, will restart after cooldown"
                    );
                } else {
                    info!(agent = %name, layer = ?managed.definition.layer, "agent timed out");
                }
            }
        }

        // Drain output from exiting agents BEFORE removing them
        for (name, def, status) in &exited {
            // Drain remaining output events
            let mut output_lines: Vec<String> = Vec::new();
            if let Some(managed) = self.active_agents.get_mut(name) {
                while let Ok(event) = managed.output_rx.try_recv() {
                    self.nightwatch.observe(name, &event);
                    match &event {
                        crate::OutputEvent::Stdout { line, .. } => {
                            output_lines.push(line.clone());
                        }
                        crate::OutputEvent::Stderr { line, .. } => {
                            output_lines.push(format!("[stderr] {}", line));
                        }
                        _ => {}
                    }
                }
            }

            // Post output to Gitea if configured
            if let (Some(poster), Some(issue)) = (&self.output_poster, def.gitea_issue) {
                let exit_code = status.code();
                if let Err(e) = poster.post_agent_output(name, issue, &output_lines, exit_code).await {
                    warn!(agent = %name, issue = issue, error = %e, "failed to post output to Gitea");
                }
            }
        }

        // Process natural exits

        // NOW remove from active_agents and handle exits
        for (name, def, status) in exited {
            self.active_agents.remove(&name);
            self.handle_agent_exit(&name, &def, status);
        }
    }

    /// Handle an agent exit based on its layer.
    fn handle_agent_exit(
        &mut self,
        name: &str,
        def: &AgentDefinition,
        status: std::process::ExitStatus,
    ) {
        match def.layer {
            AgentLayer::Safety => {
                let count = self.restart_counts.entry(name.to_string()).or_insert(0);
                *count += 1;
                self.restart_cooldowns
                    .insert(name.to_string(), Instant::now());
                if *count <= self.config.max_restart_count {
                    info!(
                        agent = %name,
                        exit_status = %status,
                        restart_count = *count,
                        cooldown_secs = self.config.restart_cooldown_secs,
                        "safety agent exited, will restart after cooldown"
                    );
                } else {
                    error!(
                        agent = %name,
                        exit_status = %status,
                        restart_count = *count,
                        max = self.config.max_restart_count,
                        "safety agent exceeded max restarts, not restarting"
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
                // Must have a restart cooldown entry (meaning it exited)
                let last_exit = match self.restart_cooldowns.get(&def.name) {
                    Some(t) => t,
                    None => return false,
                };
                // Must have passed the cooldown
                if last_exit.elapsed() < cooldown {
                    return false;
                }
                // Must be under max restart count
                let count = self.restart_counts.get(&def.name).copied().unwrap_or(0);
                count <= max_restarts
            })
            .cloned()
            .collect();

        for def in to_restart {
            info!(
                agent = %def.name,
                restart_count = self.restart_counts.get(&def.name).copied().unwrap_or(0),
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
            let should_fire = compound_sched
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next()
                .is_some();
            if should_fire {
                self.handle_schedule_event(ScheduleEvent::CompoundReview)
                    .await;
            }
        }
    }

    /// Drain broadcast output events from all active agents into nightwatch.
    fn drain_output_events(&mut self) {
        // Collect events first to avoid borrow conflict (active_agents + nightwatch)
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
        for (name, event) in &events {
            self.nightwatch.observe(name, event);
        }
    }

    /// Check flow schedules and trigger due flows.
    async fn check_flow_schedules(&mut self) {
        let now = chrono::Utc::now();
        let mut to_trigger: Vec<flow::config::FlowDefinition> = Vec::new();

        for flow_def in &self.config.flows {
            let Some(ref schedule_str) = flow_def.schedule else { continue };
            let Ok(schedule) = cron::Schedule::from_str(schedule_str) else { continue };

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
            self.handle_schedule_event(
                ScheduleEvent::Flow(Box::new(flow_def))
            ).await;
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
                    }
                    Err(e) => {
                        error!(error = %e, "compound review failed");
                    }
                }
            }
            ScheduleEvent::Flow(flow_def) => {
                let flow_name = flow_def.name.clone();
                let flow_state_dir = self.config.flow_state_dir.clone()
                    .unwrap_or_else(|| PathBuf::from("/tmp/flow-states"));
                let working_dir = self.config.compound_review.repo_path.clone();
                let flow_def = *flow_def;
                let flow_name_for_closure = flow_name.clone();
                // FlowExecutor contains non-Send types (Regex via AgentSpawner),
                // so we use spawn_blocking + Handle::block_on as a Send-safe bridge.
                let rt_handle = tokio::runtime::Handle::current();
                let handle = tokio::task::spawn_blocking(move || {
                    let executor = flow::executor::FlowExecutor::new(working_dir, flow_state_dir);
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
                },
            ],
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            tick_interval_secs: 30,
            handoff_buffer_ttl_secs: None,
            persona_data_dir: None,
            skill_data_dir: None,
            flows: vec![],
            flow_state_dir: None,
            gitea: None,
            mentions: None,
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
            }],
            restart_cooldown_secs: 0, // instant restart for testing
            max_restart_count: 3,
            tick_interval_secs: 1,
            handoff_buffer_ttl_secs: None,
            persona_data_dir: None,
            skill_data_dir: None,
            flows: vec![],
            flow_state_dir: None,
            gitea: None,
            mentions: None,
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

        // Restart count should be recorded
        assert_eq!(
            orch.restart_counts.get("echo-safety").copied(),
            Some(1),
            "restart count should be incremented"
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
        let mut orch = AgentOrchestrator::new(config).unwrap();
        let def = orch.config.agents[0].clone();

        // Cycle through max_restart_count + 1 exits
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

        // After 3 exits, restart count = 3, max = 2
        // restart_pending should NOT restart (count > max)
        orch.restart_pending_safety_agents().await;
        assert!(
            !orch.active_agents.contains_key("echo-safety"),
            "agent should not restart after exceeding max_restart_count"
        );
        assert_eq!(orch.restart_counts.get("echo-safety").copied(), Some(3));
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
        assert_eq!(
            orch.restart_counts.get("echo-safety").copied(),
            Some(1),
            "restart count should be 1 after first exit+restart cycle"
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
        assert!(orch.is_ok(), "orchestrator should initialize with empty flows");

        let orch = orch.unwrap();
        assert!(orch.active_flows.is_empty(), "active_flows should be empty initially");
    }

    /// Test that flow scheduling overlap prevention works
    #[tokio::test]
    async fn test_flow_overlap_prevention() {
        use crate::flow::config::{FlowDefinition, FlowStepDef, StepKind};

        let mut config = test_config_fast_lifecycle();

        // Add a test flow with a schedule
        config.flows = vec![FlowDefinition {
            name: "test-flow".to_string(),
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
        assert!(orch.active_flows.is_empty(), "active_flows should be empty initially");
    }
}
