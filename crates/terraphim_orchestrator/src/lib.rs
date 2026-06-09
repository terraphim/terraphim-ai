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
//! - **MetaCoordinator**: Cross-project issue-driven agent dispatch with PageRank prioritisation
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
pub mod agent_registry;
pub mod agent_run_command;
pub mod agent_run_record;
pub mod agent_runner;
mod auto_merge_impl;
pub mod compound;
pub mod concurrency;
pub mod config;
pub mod control_plane;
pub mod cost_tracker;
#[cfg(unix)]
pub mod direct_dispatch;
mod dispatch_impl;
pub mod dispatcher;
pub mod dual_mode;
pub mod error;
pub mod error_signatures;
mod escalation_impl;
pub mod evolution;
pub mod flow;
pub mod gitea_skill_loader;
pub mod handoff;
pub mod kg_router;
pub mod learning;
pub mod local_skills;
pub mod mention;
pub mod mention_chain;
mod mentions_impl;
pub mod meta_coordinator;
pub mod metrics_persistence;
pub mod mode;
pub mod nightwatch;
pub mod output_poster;
pub mod persona;
pub mod post_merge_gate;
pub mod pr_dispatch;
pub mod pr_gate;
pub mod pr_gate_result;
mod pr_handlers_impl;
pub mod pr_poller;
pub mod pr_review;
mod pre_check_impl;
pub mod project_adf;
pub mod project_control;
pub mod provider_budget;
pub mod provider_probe;
#[cfg(feature = "quickwit")]
pub mod quickwit;
#[cfg(feature = "quickwit")]
pub mod quickwit_bulk;
pub mod rate_limiter;
mod reconcile_impl;
pub mod scheduler;
mod scheduling_impl;
pub mod scope;
mod spawn_impl;
mod telemetry_impl;
pub mod webhook;
pub mod worktree_guard;

pub use agent_registry::{AgentKey, AgentRegistry, AgentScope, AgentSource, RegisteredAgent};
pub use agent_run_command::{
    applicable_modes, is_cron_schedule_valid, parse_agent_args, run_synthetic, run_validate,
    run_validate_all, schedule_for_agent, validate_agent_all_modes, AgentSubcommand,
    AgentValidateAllReport, OutputFormat,
};
pub use agent_run_record::{
    AgentRunRecord, ExitClass, ExitClassification, ExitClassifier, RunTrigger,
};
pub use agent_runner::{
    probe_cli_tool, probe_model_available, run_agent_synthetic, validate_agent_runtime,
    AgentRunRequest, AgentRuntimeValidationReport, GiteaTargetReport, ModeResult, SyntheticEvent,
    TriggerMode,
};
pub use compound::{CompoundReviewResult, CompoundReviewWorkflow, ReviewGroupDef, SwarmConfig};
pub use concurrency::{ConcurrencyController, FairnessPolicy, ModeQuotas};
#[cfg(feature = "quickwit")]
pub use config::QuickwitConfig;
pub use config::{
    AgentDefinition, AgentLayer, CompoundReviewConfig, ConcurrencyConfig, EvolutionConfig,
    GiteaOutputConfig, LearningConfig, MentionConfig, NightwatchConfig, OrchestratorConfig,
    PreCheckStrategy, TrackerConfig, TrackerStates, WebhookConfig, WorkflowConfig,
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
pub use mention_chain::{
    MentionChainError, MentionChainTracker, MentionContextArgs, DEFAULT_MAX_MENTION_DEPTH,
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
pub use project_adf::ProjectAdfConfig;
#[cfg(feature = "quickwit")]
pub use quickwit_bulk::QuickwitEsBulkSink;
pub use rate_limiter::{is_rate_limit_backoff_enabled, RateLimiter};
pub use scheduler::{ScheduleEvent, TimeScheduler};
pub use worktree_guard::{with_worktree_guard, with_worktree_guard_async, WorktreeGuard};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use std::sync::{Arc, Mutex};

use terraphim_router::RoutingEngine;
use terraphim_spawner::health::{CircuitBreaker, HealthStatus};
use terraphim_spawner::output::OutputEvent;
use terraphim_spawner::{AgentHandle, AgentSpawner, SpawnContext};
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
    output_rx: broadcast::Receiver<OutputEvent>,
    spawned_by_mention: bool,
    worktree_path: Option<PathBuf>,
    routed_model: Option<String>,
    session_id: String,
    mention_chain_id: Option<String>,
    mention_depth: Option<u32>,
    mention_parent_agent: Option<String>,
    /// Concurrency permit held while the agent is running. Released on drop.
    #[allow(dead_code)]
    concurrency_permit: Option<concurrency::AgentPermit>,
    /// When set, post a terminal commit status on agent exit.
    /// Tuple of (head_sha, context).
    commit_status_post: Option<(String, String)>,
    /// When set, derive the terminal commit status from a parsed
    /// `adf:gate-result` block in the agent's drain log instead of the
    /// process exit code. Populated for PR gate agents only.
    gate_meta: Option<crate::pr_gate_result::PrGateMeta>,
    /// Temp file path for streaming agent output. Renamed to final path on exit.
    output_tmp_path: Option<PathBuf>,
    /// Worktree guard for automatic cleanup on agent crash.
    #[allow(dead_code)]
    worktree_guard: Option<crate::worktree_guard::WorktreeGuard>,
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
    agent_registry: AgentRegistry,
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
    /// Per-agent last cron fire timestamp to prevent re-triggering within same schedule window.
    /// Key: agent name. Value: timestamp of last fire.
    last_cron_fire: HashMap<String, chrono::DateTime<chrono::Utc>>,
    /// Last compound-review fire time, used to gate the compound schedule
    /// independently of `last_tick_time`. Mirrors the `last_cron_fire`
    /// pattern for per-agent crons. Without this cursor, if the
    /// `reconcile_tick` future is cancelled mid-await by its 90 s
    /// `tokio::time::timeout` safety wrapper, `last_tick_time` is never
    /// advanced and the same compound-review occurrence re-fires on the
    /// very next tick, producing a worktree storm (#1562).
    last_compound_review_fired_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Lazy-initialised Gitea tracker for gitea-issue pre-check.
    pre_check_tracker: Option<terraphim_tracker::GiteaTracker>,
    /// Active flow executions keyed by flow name.
    #[allow(dead_code)]
    active_flows: HashMap<String, tokio::task::JoinHandle<flow::state::FlowRunState>>,
    /// Active compound review execution (spawned in background to avoid
    /// blocking reconcile_tick). None when no compound review is running.
    active_compound_review:
        Option<tokio::task::JoinHandle<Result<CompoundReviewResult, OrchestratorError>>>,
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
    /// Per-provider hour/day spend tracker consulted by the routing
    /// engine. `None` when the config declares no [[providers]] entries.
    provider_budget_tracker: Option<Arc<provider_budget::ProviderBudgetTracker>>,
    /// Compiled per-provider stderr classifiers built from
    /// `[[providers]].error_signatures`. Providers without signatures
    /// are absent from the map and classify as
    /// [`error_signatures::ErrorKind::Unknown`] (fail-safe).
    provider_error_signatures: error_signatures::ProviderSignatureMap,
    provider_rate_limits: ProviderRateLimitWindow,
    retry_counts: HashMap<String, (u32, Instant)>,
    /// Dedupe set of [`error_signatures::unknown_dedupe_key`] values so we
    /// don't open a new `[ADF]` Gitea issue for every retry of the same
    /// stderr shape. Process-lifetime in-memory; intentional since the
    /// window between duplicates is short and restarting the orchestrator
    /// is an acceptable dedupe reset.
    unknown_error_dedupe: Arc<Mutex<std::collections::HashSet<String>>>,
    /// Counter of consecutive `project-meta` failures per project. Tripped
    /// entries cause the orchestrator to create a pause flag and open an
    /// `[ADF]` Gitea escalation issue.
    project_failure_counter: project_control::ProjectFailureCounter,
    /// Resolved pause-flag directory. Derived from
    /// [`OrchestratorConfig::pause_dir`] or [`project_control::DEFAULT_PAUSE_DIR`].
    pause_dir: PathBuf,
    /// Unified priority queue for all dispatch sources (time, issue, mention,
    /// review-pr, auto-merge, post-merge-gate).
    dispatcher: dispatcher::Dispatcher,
    /// Per-(project, PR) rate limiter for verdict polling. Keeps the
    /// orchestrator from re-hitting Gitea for the same PR every tick.
    pr_poll_rate_limiter: pr_poller::PrPollRateLimiter,
    /// Per-project dedupe set of `(pr_number, head_sha)` already enqueued
    /// for auto-merge so the same revision is never dispatched twice.
    auto_merge_enqueued: pr_poller::AutoMergeDedupeSet,
    /// TTL-based dedupe cache for auto-merge failure issues. Prevents
    /// duplicate `[ADF] Auto-merge failed` issues for the same PR within
    /// a 24-hour window.
    auto_merge_failure_dedupe: pr_poller::AutoMergeFailureDedupe,
    /// Shared learning store. `None` when `learning.enabled = false` or
    /// when initialisation failed (graceful degradation).
    learning_store: Option<learning::SharedLearningStore>,
    /// Learning config snapshot (min_trust, max_tokens, etc.).
    learning_config: config::LearningConfig,
    /// Agent name -> learning IDs injected at spawn time, used to record
    /// outcome feedback at exit.
    injected_learning_ids: HashMap<String, Vec<String>>,
    /// Global concurrency controller enforcing agent limits and fairness.
    concurrency_controller: concurrency::ConcurrencyController,
    /// Directory for per-agent output log files. Each completed agent run
    /// writes its stdout+stderr to `<agent_log_dir>/<name>-<timestamp>.log`.
    agent_log_dir: PathBuf,
    /// Cache directory populated from Gitea at startup (issue #1434).
    /// `None` when `gitea_skill_repo` is not configured. When `Some`, this
    /// directory is prepended to the skill search path so remote skills
    /// shadow local ones while local directories remain as fallback.
    gitea_skill_cache_dir: Option<PathBuf>,
    /// Agent evolution manager. No-op when evolution feature is disabled
    /// or `evolution.enabled = false` in config.
    evolution_manager: evolution::EvolutionManager,
    config_error_counters: HashMap<String, u32>,
    quarantined_agents: std::collections::HashSet<String>,
}

/// Build the composite restart-state key for an agent definition.
///
/// Legacy (project-less) agents use [`crate::dispatcher::LEGACY_PROJECT_ID`]
/// so restart counts never collide across projects once projects are added.
pub(crate) fn agent_key(def: &AgentDefinition) -> (String, String) {
    (
        def.project
            .clone()
            .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
        def.name.clone(),
    )
}

struct ProviderRateLimitWindow {
    blocked_until: HashMap<String, Instant>,
}

impl ProviderRateLimitWindow {
    fn new() -> Self {
        Self {
            blocked_until: HashMap::new(),
        }
    }

    fn block_until(&mut self, provider: &str, until: Instant) {
        self.blocked_until.insert(provider.to_string(), until);
    }

    #[allow(dead_code)]
    fn is_blocked(&self, provider: &str) -> bool {
        self.blocked_until
            .get(provider)
            .is_some_and(|until| Instant::now() < *until)
    }

    fn blocked_providers(&self) -> Vec<String> {
        let now = Instant::now();
        self.blocked_until
            .iter()
            .filter(|(_, until)| **until > now)
            .map(|(k, _)| k.clone())
            .collect()
    }

    fn clean_expired(&mut self) {
        let now = Instant::now();
        self.blocked_until.retain(|_, until| *until > now);
    }
}

/// Conservative fallback block applied at the quota-detect site when no
/// "resets" line is present in stderr at all (i.e. `parse_reset_time` cannot
/// even be invoked with useful input). Long enough to skip the next cron
/// firing (~30 min active-window cadence); short enough that a misclassified
/// non-quota error does not disable the provider for the day.
pub(crate) const DEFAULT_RATE_LIMIT_BLOCK: Duration = Duration::from_secs(900);

pub(crate) fn parse_reset_time(quota_line: &str) -> Option<Instant> {
    let line = quota_line.to_lowercase();

    if let Some(idx) = line.find("resets in ") {
        let rest = &line[idx + "resets in ".len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<u64>() {
            if rest.contains("hour") {
                return Some(Instant::now() + Duration::from_secs(n * 3600));
            }
            if rest.contains("minute") {
                return Some(Instant::now() + Duration::from_secs(n * 60));
            }
            // Short abbreviations used by Claude Code: "resets in 4h", "resets in 30m".
            // The character immediately after the digits disambiguates the unit.
            let unit = rest.chars().nth(digits.len());
            match unit {
                Some('h') => return Some(Instant::now() + Duration::from_secs(n * 3600)),
                Some('m') => {
                    // Guard against "min..." which is already handled above; the
                    // "minute" branch returned first, so any remaining 'm' here
                    // is the short form.
                    return Some(Instant::now() + Duration::from_secs(n * 60));
                }
                _ => {}
            }
        }
    }

    // "resets 11pm" / "resets 7am" -- wall-clock hour with am/pm suffix, no "in"/"at".
    // Treat the hour as UTC (bigbox locale); compute next future occurrence.
    if let Some(idx) = line.find("resets ") {
        let rest = line[idx + "resets ".len()..].trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let after_digits = &rest[digits.len()..];
            let is_pm = after_digits.starts_with("pm");
            let is_am = after_digits.starts_with("am");
            if is_pm || is_am {
                if let Ok(h) = digits.parse::<u32>() {
                    if h <= 12 {
                        let hour_24 = match (h, is_pm) {
                            (12, true) => 12,
                            (12, false) => 0,
                            (h, true) => h + 12,
                            (h, false) => h,
                        };
                        if let Some(time) = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0) {
                            let now = chrono::Utc::now();
                            let mut target_date = now.date_naive();
                            let mut target_utc =
                                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                    target_date.and_time(time),
                                    chrono::Utc,
                                );
                            if target_utc <= now {
                                target_date += chrono::Duration::days(1);
                                target_utc =
                                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                        target_date.and_time(time),
                                        chrono::Utc,
                                    );
                            }
                            if let Ok(delta) = (target_utc - now).to_std() {
                                return Some(Instant::now() + delta);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(idx) = line.find("resets at ") {
        let rest = &line[idx + "resets at ".len()..];
        if let Some(time_str) = rest
            .strip_suffix(" utc")
            .or_else(|| rest.strip_suffix(" utc."))
        {
            if let Ok(hour_min) = chrono::NaiveTime::parse_from_str(time_str.trim(), "%H:%M") {
                let now = chrono::Utc::now();
                let mut target_date = now.date_naive();
                let target = target_date.and_time(hour_min);
                let mut target_utc =
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(target, chrono::Utc);
                if target_utc <= now {
                    target_date += chrono::Duration::days(1);
                    let target = target_date.and_time(hour_min);
                    target_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                        target,
                        chrono::Utc,
                    );
                }
                let delta = (target_utc - now).to_std().ok()?;
                return Some(Instant::now() + delta);
            }
        }
    }

    if line.contains("resets ") {
        return Some(Instant::now() + Duration::from_secs(3600));
    }

    None
}

/// Build the optional provider-level hour/day budget tracker from the
/// [[providers]] config block. Returns `Ok(None)` when no providers are
/// declared (no cap = no tracker, routing works unchanged).
fn build_provider_budget_tracker(
    config: &OrchestratorConfig,
) -> Result<Option<Arc<provider_budget::ProviderBudgetTracker>>, OrchestratorError> {
    if config.providers.is_empty() {
        if config.provider_budget_state_file.is_some() {
            warn!("provider_budget_state_file set but no [[providers]] entries; tracker disabled");
        }
        return Ok(None);
    }
    let tracker = match config.provider_budget_state_file.as_ref() {
        Some(path) => provider_budget::ProviderBudgetTracker::with_persistence(
            config.providers.clone(),
            path.clone(),
        )
        .map_err(|e| {
            OrchestratorError::Config(format!(
                "failed to load provider budget state from {}: {}",
                path.display(),
                e
            ))
        })?,
        None => provider_budget::ProviderBudgetTracker::new(config.providers.clone()),
    };
    info!(
        providers = tracker.providers().collect::<Vec<_>>().join(","),
        persistence = ?config.provider_budget_state_file,
        "provider budget tracker initialised"
    );
    Ok(Some(Arc::new(tracker)))
}

/// Build the global concurrency controller from orchestrator config.
///
/// Uses `workflow.concurrency` when workflow mode is configured; otherwise
/// falls back to sensible defaults (global_max = 10, time_max = 10,
/// issue_max = 5, round-robin fairness).
fn build_concurrency_controller(config: &OrchestratorConfig) -> concurrency::ConcurrencyController {
    let (global_max, time_max, issue_max, fairness) = config
        .workflow
        .as_ref()
        .map(|w| {
            (
                w.concurrency.global_max,
                w.concurrency.global_max, // time-driven shares global pool
                w.concurrency.issue_max,
                w.concurrency
                    .fairness
                    .parse::<concurrency::FairnessPolicy>()
                    .unwrap_or(concurrency::FairnessPolicy::RoundRobin),
            )
        })
        .unwrap_or((10, 10, 5, concurrency::FairnessPolicy::RoundRobin));

    let quotas = concurrency::ModeQuotas {
        time_max,
        issue_max,
    };

    let project_caps: HashMap<String, concurrency::ProjectCaps> = config
        .projects
        .iter()
        .filter_map(|p| {
            p.max_concurrent_agents.map(|max| {
                (
                    p.id.clone(),
                    concurrency::ProjectCaps {
                        max_concurrent_agents: max,
                        max_concurrent_mention_agents: p.max_concurrent_mention_agents,
                    },
                )
            })
        })
        .collect();

    concurrency::ConcurrencyController::with_project_caps(
        global_max,
        quotas,
        fairness,
        project_caps,
    )
}

/// Build the per-project runtime map consumed by
/// [`flow::executor::FlowExecutor::with_projects`].
pub(crate) fn build_flow_project_runtimes(
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
///
/// When an [`OutputPoster`] is available and has a per-agent Gitea token
/// for `(project, agent_name)` (loaded from `agent_tokens.json`), the
/// token is injected as `GITEA_TOKEN` so the agent's own `gtr` / API
/// calls inside its task shell post under its own Gitea identity. Without
/// this, `source ~/.profile` would overlay the shared root token.
pub(crate) fn build_spawn_context_for_agent(
    config: &OrchestratorConfig,
    def: &AgentDefinition,
    output_poster: Option<&OutputPoster>,
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
            .with_env("GITEA_URL", gitea.base_url.clone())
            .with_env("GITEA_OWNER", gitea.owner.clone())
            .with_env("GITEA_REPO", gitea.repo.clone());
    }
    if let Some(poster) = output_poster {
        if let Some(token) = poster.agent_token(pid, &def.name) {
            ctx = ctx.with_env("GITEA_TOKEN", token.to_string());
        }
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

/// Truncate a string to at most `max_bytes` UTF-8 bytes, honouring char
/// boundaries. If truncation occurs, append a marker so the reader knows.
pub(crate) fn truncate_for_issue(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}... (truncated)", &s[..end])
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
        let agent_registry = AgentRegistry::from_config(&config)?;

        // Layer 2 startup sweep (epic #1567, issue #1570).
        //
        // Reconcile any worktree residue left by a previous instance
        // before we accept ticks. Synchronous: must finish before the
        // tick thread is spawned in `run()` so a fresh review cycle
        // never races against half-killed `review-*` directories.
        //
        // `extra_roots` mirrors the per-agent worktree convention
        // from `lib.rs:5393`. If you change that literal there, change
        // it here too.
        //
        // `cfg(not(test))` gate: the in-lib `test_config()` (see
        // `lib.rs:7926`) points `repo_path` at the live terraphim-ai
        // checkout. Without this gate, the sweep's
        // `git worktree prune --verbose` races against
        // `test_orchestrator_compound_review_manual`'s concurrent
        // `git worktree add` on that shared real repo's
        // `.git/worktrees/` admin registry. The production wiring is
        // exercised end-to-end by `tests/sweep_on_startup_test.rs`,
        // which builds an isolated `TempDir` repo and asserts the
        // sweep DOES run from `AgentOrchestrator::new`. Do not remove
        // this gate without first migrating the in-lib `test_config()`
        // to a TempDir-based `repo_path`.
        #[cfg(not(test))]
        {
            let sweep_report = compound_workflow.worktree_manager().sweep_stale(&[]);
            if sweep_report.swept_count + sweep_report.root_owned_skipped > 10 {
                warn!(
                    swept_count = sweep_report.swept_count,
                    root_owned_skipped = sweep_report.root_owned_skipped,
                    failed_count = sweep_report.failed_count,
                    "large worktree backlog at startup -- prior crash storm likely"
                );
            }
        }

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
            provider_probe::ProviderHealthMap::new(std::time::Duration::from_secs(probe_ttl))
                .with_rate_limiter(crate::rate_limiter::RateLimiter::new());

        let telemetry_store = control_plane::TelemetryStore::new(3600);

        let provider_budget_tracker = build_provider_budget_tracker(&config)?;

        // Compile per-provider stderr signatures declared under
        // `[[providers]].error_signatures`. Invalid regexes fail loud
        // at startup so misconfiguration can never silently disable
        // runtime classification.
        let provider_error_signatures = error_signatures::build_signature_map(&config.providers)
            .map_err(|e| OrchestratorError::Config(e.to_string()))?;

        let project_failure_counter =
            project_control::ProjectFailureCounter::new(config.project_circuit_breaker_threshold);
        let pause_dir = config
            .pause_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(project_control::DEFAULT_PAUSE_DIR));

        #[cfg(not(test))]
        let restart_state = Self::load_restart_state();

        let learning_config = config.learning.clone();

        // MentionCursor loaded lazily on first poll (async)

        Ok(Self {
            config: config.clone(),
            agent_registry,
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
            last_cron_fire: HashMap::new(),
            last_compound_review_fired_at: None,
            pre_check_tracker: None,
            active_flows: HashMap::new(),
            active_compound_review: None,
            mention_cursors: HashMap::new(),
            webhook_dispatch_rx: None,
            tick_count: 0,
            #[cfg(feature = "quickwit")]
            quickwit_sink: None,
            exit_classifier: ExitClassifier::new(),
            kg_router,
            provider_health,
            telemetry_store,
            provider_budget_tracker,
            provider_error_signatures,
            provider_rate_limits: ProviderRateLimitWindow::new(),
            retry_counts: HashMap::new(),
            unknown_error_dedupe: Arc::new(Mutex::new(std::collections::HashSet::new())),
            project_failure_counter,
            pause_dir,
            dispatcher: dispatcher::Dispatcher::new(),
            pr_poll_rate_limiter: pr_poller::PrPollRateLimiter::new(
                pr_poller::PR_POLL_MIN_INTERVAL,
            ),
            auto_merge_enqueued: pr_poller::AutoMergeDedupeSet::new(),
            auto_merge_failure_dedupe: pr_poller::AutoMergeFailureDedupe::new(
                std::time::Duration::from_secs(86400),
            ),
            learning_store: None,
            learning_config,
            injected_learning_ids: HashMap::new(),
            concurrency_controller: build_concurrency_controller(&config),
            agent_log_dir: {
                let adf_logs = std::path::PathBuf::from("/opt/ai-dark-factory/logs/agents");
                if adf_logs.parent().map(|p| p.exists()).unwrap_or(false) {
                    adf_logs
                } else {
                    config.working_dir.join("logs").join("agents")
                }
            },
            gitea_skill_cache_dir: None,
            evolution_manager: evolution::EvolutionManager::new(config.evolution.clone()),
            config_error_counters: HashMap::new(),
            quarantined_agents: std::collections::HashSet::new(),
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

    /// Open a temp log file for an agent and spawn a background task that
    /// continuously drains its output broadcast into the file.  Returns the
    /// temp-file path so it can be renamed to the final name on exit.
    fn start_output_log_drain(&self, agent_name: &str, handle: &AgentHandle) -> Option<PathBuf> {
        let _ = std::fs::create_dir_all(&self.agent_log_dir);
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let tmp_name = format!(".tmp-{}-{}.log", agent_name, ts);
        let tmp_path = self.agent_log_dir.join(&tmp_name);

        let file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "failed to create output log temp file");
                return None;
            }
        };

        let mut rx = handle.subscribe_output();
        let name = agent_name.to_string();
        let path = tmp_path.clone();

        tokio::spawn(async move {
            use std::io::Write;
            let mut writer = std::io::BufWriter::new(file);
            loop {
                match rx.recv().await {
                    Ok(event) => match &event {
                        crate::OutputEvent::Stdout { line, .. } => {
                            let _ = writeln!(writer, "{}", line);
                        }
                        crate::OutputEvent::Stderr { line, .. } => {
                            let _ = writeln!(writer, "[stderr] {}", line);
                        }
                        _ => {}
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let _ = writeln!(writer, "[... {} output lines dropped ...]", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            let _ = writer.flush();
            debug!(agent = %name, path = %path.display(), "output log drain finished");
        });

        Some(tmp_path)
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

    /// Read-only access to the unified dispatcher queue.
    ///
    /// Integration tests use this to assert that ROC v1 Step F polling
    /// enqueued the expected [`dispatcher::DispatchTask::AutoMerge`] work without the
    /// test itself holding a reference to the internal dispatcher.
    pub fn dispatcher(&self) -> &dispatcher::Dispatcher {
        &self.dispatcher
    }

    /// Read-only access to the in-memory `(project, pr_number, head_sha)`
    /// dedupe set populated by ROC v1 Step F polling and the Step G
    /// AutoMerge handler. Integration tests use this to assert that a
    /// successful merge leaves the revision recorded so subsequent polls
    /// never re-enqueue the same auto-merge.
    pub fn auto_merge_enqueued(&self) -> &pr_poller::AutoMergeDedupeSet {
        &self.auto_merge_enqueued
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

                // Send probe results to Quickwit for cost-aware routing
                if let Some(ref sink) = self.quickwit_sink {
                    let project_id = self
                        .config
                        .projects
                        .first()
                        .map(|p| p.id.clone())
                        .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
                    self.provider_health
                        .send_to_quickwit(sink, &project_id)
                        .await;
                }
            }
        }

        // Restore persisted telemetry from previous runs
        self.restore_telemetry().await;

        // Populate Gitea skill cache if configured (issue #1434).
        // On success, the cache dir becomes the first skill root.
        // On any error, populate_skill_cache logs a warning and returns the
        // cache dir path anyway; local skill roots are used as fallback.
        if let Some(ref skill_repo) = self.config.gitea_skill_repo.clone() {
            let cache_dir = gitea_skill_loader::populate_skill_cache(skill_repo, false).await;
            self.gitea_skill_cache_dir = Some(cache_dir);
        }

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

        // Webhook and direct dispatch use separate channels so the bridge tasks
        // can emit distinct LoopEvent variants without needing to tag messages.
        let webhook_dispatch_rx = if self.config.webhook.is_some() {
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            self.webhook_dispatch_rx = Some(rx);
            Some(tx)
        } else {
            self.webhook_dispatch_rx = None;
            None
        };

        #[cfg(unix)]
        let direct_dispatch_rx = if self.config.direct_dispatch.is_some() {
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            Some((tx, rx))
        } else {
            None
        };
        #[cfg(not(unix))]
        let direct_dispatch_rx: Option<(
            tokio::sync::mpsc::Sender<webhook::WebhookDispatch>,
            tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>,
        )> = None;

        // Start webhook server if configured
        if let Some(ref webhook_cfg) = self.config.webhook {
            let dispatch_tx = webhook_dispatch_rx
                .as_ref()
                .expect("webhook dispatch channel is initialised when webhook is configured")
                .clone();

            let agent_names: Vec<String> =
                self.config.agents.iter().map(|a| a.name.clone()).collect();
            let project_by_repo: std::collections::HashMap<String, String> = self
                .config
                .projects
                .iter()
                .filter_map(|p| {
                    p.gitea
                        .as_ref()
                        .map(|g| (format!("{}/{}", g.owner, g.repo), p.id.clone()))
                })
                .collect();
            let state = webhook::WebhookState {
                agent_names,
                persona_registry: Arc::new(self.persona_registry.clone()),
                dispatch_tx,
                secret: webhook_cfg.secret.clone(),
                project_by_repo,
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

        #[cfg(unix)]
        let direct_dispatch_rx = if let Some(ref direct_cfg) = self.config.direct_dispatch {
            let (direct_tx, direct_rx) = direct_dispatch_rx.expect(
                "direct dispatch channel is initialised when direct_dispatch is configured",
            );
            let agent_index =
                direct_dispatch::DirectDispatchAgentIndex::from_agents(&self.config.agents);

            direct_dispatch::start_direct_dispatch_listener(
                direct_cfg.socket_path.clone(),
                direct_tx,
                agent_index,
            );

            Some(direct_rx)
        } else {
            None
        };
        #[cfg(not(unix))]
        let direct_dispatch_rx: Option<
            tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>,
        > = None;

        enum LoopEvent {
            Tick,
            Schedule(ScheduleEvent),
            DriftAlert(DriftAlert),
            Webhook(webhook::WebhookDispatch),
            DirectDispatch(webhook::WebhookDispatch),
        }

        let tick_interval = self.config.tick_interval_secs;
        let (loop_tx, loop_rx) = std::sync::mpsc::channel::<LoopEvent>();
        let loop_tx = Arc::new(std::sync::Mutex::new(loop_tx));

        let sched_tx = loop_tx.clone();
        let sched_rx = self.scheduler.take_event_rx();
        if let Some(rx) = sched_rx {
            tokio::spawn(async move {
                let mut rx = rx;
                while let Some(event) = rx.recv().await {
                    if sched_tx
                        .lock()
                        .unwrap()
                        .send(LoopEvent::Schedule(event))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }

        let alert_tx = loop_tx.clone();
        let alert_rx = self.nightwatch.take_alert_rx();
        if let Some(rx) = alert_rx {
            tokio::spawn(async move {
                let mut rx = rx;
                while let Some(alert) = rx.recv().await {
                    if alert_tx
                        .lock()
                        .unwrap()
                        .send(LoopEvent::DriftAlert(alert))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }

        let tick_stx = loop_tx.clone();
        let tick_interval_secs = tick_interval;
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(tick_interval_secs));
            if tick_stx.lock().unwrap().send(LoopEvent::Tick).is_err() {
                break;
            }
        });

        let webhook_rx = self.webhook_dispatch_rx.take();
        if let Some(rx) = webhook_rx {
            let wh_tx = loop_tx.clone();
            tokio::spawn(async move {
                let mut rx = rx;
                while let Some(dispatch) = rx.recv().await {
                    if wh_tx
                        .lock()
                        .unwrap()
                        .send(LoopEvent::Webhook(dispatch))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }

        if let Some(direct_rx) = direct_dispatch_rx {
            let dd_tx = loop_tx.clone();
            tokio::spawn(async move {
                let mut rx = direct_rx;
                while let Some(dispatch) = rx.recv().await {
                    if dd_tx
                        .lock()
                        .unwrap()
                        .send(LoopEvent::DirectDispatch(dispatch))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }

        let reconcile_timeout = Duration::from_secs(self.config.tick_interval_secs.max(30) * 3);

        loop {
            if self.shutdown_requested {
                info!("shutdown requested, stopping reconciliation loop");
                break;
            }

            match loop_rx.recv_timeout(reconcile_timeout) {
                Ok(LoopEvent::Webhook(dispatch)) => {
                    let comment_id = dispatch.comment_id();
                    self.handle_webhook_dispatch(dispatch).await;
                    self.mark_webhook_comment_processed(comment_id).await;
                    let _ = loop_tx.lock().unwrap().send(LoopEvent::Tick);
                }
                Ok(LoopEvent::DirectDispatch(dispatch)) => {
                    self.handle_direct_dispatch(dispatch).await;
                }
                Ok(LoopEvent::Schedule(event)) => {
                    self.handle_schedule_event(event).await;
                }
                Ok(LoopEvent::DriftAlert(alert)) => {
                    self.handle_drift_alert(alert).await;
                }
                Ok(LoopEvent::Tick) => {
                    loop {
                        match loop_rx.try_recv() {
                            Ok(LoopEvent::Webhook(dispatch)) => {
                                let comment_id = dispatch.comment_id();
                                self.handle_webhook_dispatch(dispatch).await;
                                self.mark_webhook_comment_processed(comment_id).await;
                            }
                            Ok(LoopEvent::DirectDispatch(dispatch)) => {
                                self.handle_direct_dispatch(dispatch).await;
                            }
                            Ok(LoopEvent::Schedule(event)) => {
                                self.handle_schedule_event(event).await;
                            }
                            Ok(LoopEvent::DriftAlert(alert)) => {
                                self.handle_drift_alert(alert).await;
                            }
                            Ok(LoopEvent::Tick) => {
                                // Coalesce stale ticks so long reconciliation runs do not
                                // starve webhook and schedule events queued behind them.
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => break,
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                        }
                    }
                    match tokio::time::timeout(reconcile_timeout, self.reconcile_tick()).await {
                        Ok(()) => {}
                        Err(_) => {
                            warn!(
                                timeout_secs = reconcile_timeout.as_secs(),
                                "reconcile_tick exceeded timeout, forcing continuation"
                            );
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    warn!("loop: recv timeout, no events received");
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    info!("all event sources closed, exiting loop");
                    break;
                }
            }
        }

        // Graceful shutdown of all agents
        self.persist_telemetry();
        if let Some(tracker) = self.provider_budget_tracker.as_ref() {
            if let Err(e) = tracker.persist() {
                warn!(error = %e, "failed to persist provider budget snapshot during shutdown");
            }
        }
        self.shutdown_all_agents().await;
        Ok(())
    }

    /// Request graceful shutdown of all agents and the orchestrator.
    pub fn shutdown(&mut self) {
        info!("shutdown requested");
        self.shutdown_requested = true;
    }

    async fn mark_webhook_comment_processed(&mut self, comment_id: u64) {
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
            cursor.mark_processed(comment_id);
        }
        for pid in &project_ids {
            if let Some(cursor) = self.mention_cursors.get(pid) {
                cursor.save(pid).await;
            }
        }
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
        let skill_roots = Self::skill_roots(
            self.config.skill_data_dir.as_deref(),
            home_dir.as_deref(),
            self.gitea_skill_cache_dir.as_deref(),
        );

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

    fn render_lessons_section(&self, agent_name: &str) -> (String, Vec<String>) {
        let store = match self.learning_store {
            Some(ref s) => s,
            None => return (String::new(), Vec::new()),
        };

        let learnings = match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(store.query_relevant(agent_name))
        }) {
            Ok(l) => l,
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "failed to query learnings");
                return (String::new(), Vec::new());
            }
        };

        if learnings.is_empty() {
            return (String::new(), Vec::new());
        }

        let max_entries = self.learning_config.max_entries;
        let max_tokens = self.learning_config.max_tokens;
        let truncated: Vec<_> = learnings.into_iter().take(max_entries).collect();
        let ids: Vec<String> = truncated.iter().map(|l| l.id.clone()).collect();

        let mut section = String::from("## Prior Lessons\n\nLessons learned from previous agent runs. Apply relevant insights:\n\n");
        for l in &truncated {
            section.push_str(&format!(
                "- [{}] {} (trust: {}, verified {}x)\n",
                l.category, l.summary, l.trust_level, l.effective_count
            ));
            if let Some(ref details) = l.details {
                if let Some(first_line) = details.lines().next() {
                    section.push_str(&format!("  > {}\n", first_line));
                }
            }
        }

        if section.len() > max_tokens {
            let mut end = max_tokens;
            while end > 0 && !section.is_char_boundary(end) {
                end -= 1;
            }
            section.truncate(end);
            section.push_str("\n... (truncated)\n");
        }

        (section, ids)
    }

    /// Build the ordered list of directories to search for SKILL.md files.
    ///
    /// Priority (highest first):
    /// 1. Gitea skill cache dir (populated at startup from remote, if configured)
    /// 2. Configured `skill_data_dir` from TOML
    /// 3. `~/.opencode/skills`
    /// 4. `~/.claude/skills`
    fn skill_roots(
        configured: Option<&std::path::Path>,
        home_dir: Option<&std::path::Path>,
        gitea_cache: Option<&std::path::Path>,
    ) -> Vec<std::path::PathBuf> {
        let mut roots = Vec::new();

        // Remote skills take priority — operators push updates to Gitea
        // without modifying local files.
        if let Some(dir) = gitea_cache {
            roots.push(dir.to_path_buf());
        }

        if let Some(dir) = configured {
            if !roots.iter().any(|r| r == dir) {
                roots.push(dir.to_path_buf());
            }
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

    /// Test helper: set last_cron_fire for a given agent.
    #[doc(hidden)]
    pub fn set_last_cron_fire(
        &mut self,
        agent_name: &str,
        fire_time: chrono::DateTime<chrono::Utc>,
    ) {
        self.last_cron_fire
            .insert(agent_name.to_string(), fire_time);
    }

    /// Test helper: set last_tick_time for synthetic time testing.
    #[doc(hidden)]
    pub fn set_last_tick_time(&mut self, time: chrono::DateTime<chrono::Utc>) {
        self.last_tick_time = time;
    }

    /// Test helper: read the compound-review fire cursor.
    #[doc(hidden)]
    pub fn last_compound_review_fired_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_compound_review_fired_at
    }

    /// Test helper: clear the compound-review fire cursor for synthetic
    /// testing of the cancellation property.
    #[doc(hidden)]
    pub fn clear_last_compound_review_fired_at(&mut self) {
        self.last_compound_review_fired_at = None;
    }

    /// Test helper: access the telemetry store for assertions.
    #[doc(hidden)]
    pub fn telemetry_store(&self) -> &control_plane::TelemetryStore {
        &self.telemetry_store
    }

    /// Test helper: access the provider budget tracker (if any).
    #[doc(hidden)]
    pub fn provider_budget_tracker(&self) -> Option<&Arc<provider_budget::ProviderBudgetTracker>> {
        self.provider_budget_tracker.as_ref()
    }

    /// Test helper: inspect the unknown-error dedupe set (lock + clone).
    #[doc(hidden)]
    pub fn unknown_error_dedupe_snapshot(&self) -> std::collections::HashSet<String> {
        self.unknown_error_dedupe
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Test helper: drive `record_telemetry` from outside the crate so
    /// integration tests can verify the cost-tracker / provider-budget
    /// wiring without starting the full reconcile loop.
    #[doc(hidden)]
    pub async fn record_telemetry_for_test(
        &self,
        events: Vec<(String, control_plane::telemetry::CompletionEvent)>,
    ) {
        self.record_telemetry(events).await
    }

    /// Test helper: return the pause directory the orchestrator is using.
    #[doc(hidden)]
    pub fn pause_dir_for_test(&self) -> &std::path::Path {
        &self.pause_dir
    }

    /// Test helper: drive the project circuit breaker from outside the crate
    /// so integration tests can verify the trip → pause-flag path without
    /// spawning real agent processes.
    ///
    /// Simulates recording `failure_count` consecutive `project-meta` failures
    /// against `project_id`; on the Nth failure (where N == threshold) the
    /// underlying counter trips and this function touches the pause flag.
    /// Returns `true` iff the pause flag was (re)created by this call.
    #[doc(hidden)]
    pub async fn simulate_project_meta_failures_for_test(
        &mut self,
        project_id: &str,
        failure_count: u32,
    ) -> bool {
        let mut tripped = false;
        for _ in 0..failure_count {
            let verdict = self
                .project_failure_counter
                .record_project_meta_result(project_id, false);
            if verdict == project_control::ShouldPause::Yes {
                let _ = project_control::touch_pause_flag(&self.pause_dir, project_id);
                tripped = true;
            }
        }
        tripped
    }

    /// Test helper: record a successful project-meta run, resetting the
    /// per-project consecutive-failure counter.
    #[doc(hidden)]
    pub fn reset_project_meta_counter_for_test(&mut self, project_id: &str) {
        let _ = self
            .project_failure_counter
            .record_project_meta_result(project_id, true);
    }
}

/// Check whether any changed file matches any of the watch path prefixes.
pub(crate) fn has_matching_changes(changed_files: &[String], watch_paths: &[String]) -> bool {
    for file in changed_files {
        for prefix in watch_paths {
            if scope::is_path_prefix(prefix, file) {
                return true;
            }
        }
    }
    false
}

/// Determine whether an agent requires an isolated git worktree.
///
/// Review-tier agents (haiku) and simple local commands used in tests do not
/// need isolation. AI/model-backed agents that may modify code do.
pub(crate) fn requires_isolated_worktree(def: &AgentDefinition, model: Option<&str>) -> bool {
    if model
        .map(|m| m.to_ascii_lowercase().contains("haiku"))
        .unwrap_or(false)
    {
        return false;
    }

    if model.is_some() {
        return true;
    }

    let cli_name = Path::new(&def.cli_tool)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(def.cli_tool.as_str())
        .to_ascii_lowercase();

    matches!(
        cli_name.as_str(),
        "claude" | "codex" | "opencode" | "opencode-go" | "gemini" | "aider"
    )
}

#[cfg(test)]
mod lib_tests;
