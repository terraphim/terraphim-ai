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
pub mod compound;
pub mod concurrency;
pub mod config;
pub mod control_plane;
pub mod cost_tracker;
#[cfg(unix)]
pub mod direct_dispatch;
pub mod dispatcher;
pub mod dual_mode;
pub mod error;
pub mod error_signatures;
pub mod evolution;
pub mod flow;
pub mod gitea_skill_loader;
pub mod handoff;
pub mod kg_router;
pub mod learning;
pub mod local_skills;
pub mod mention;
pub mod mention_chain;
pub mod meta_coordinator;
pub mod metrics_persistence;
pub mod mode;
pub mod nightwatch;
pub mod output_poster;
pub mod persona;
pub mod post_merge_gate;
pub mod pr_dispatch;
pub mod pr_gate;
pub mod pr_poller;
pub mod pr_review;
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
pub mod scope;
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
use terraphim_types::ReviewFinding;
pub use worktree_guard::{with_worktree_guard, with_worktree_guard_async, WorktreeGuard};

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
///
/// When an [`OutputPoster`] is available and has a per-agent Gitea token
/// for `(project, agent_name)` (loaded from `agent_tokens.json`), the
/// token is injected as `GITEA_TOKEN` so the agent's own `gtr` / API
/// calls inside its task shell post under its own Gitea identity. Without
/// this, `source ~/.profile` would overlay the shared root token.
fn build_spawn_context_for_agent(
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
fn truncate_for_issue(s: &str, max_bytes: usize) -> String {
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

    /// Spawn an agent from its definition.
    ///
    /// Model selection: if the agent has an explicit `model` field, use it.
    /// Otherwise, route the task prompt through the RoutingEngine to select
    /// a model based on keyword matching.
    async fn spawn_agent(&mut self, def: &AgentDefinition) -> Result<(), OrchestratorError> {
        self.spawn_agent_with_event(def, None).await
    }

    async fn spawn_agent_with_event(
        &mut self,
        def: &AgentDefinition,
        synthetic_event: Option<&SyntheticEvent>,
    ) -> Result<(), OrchestratorError> {
        // === PROJECT PAUSE GATE ===
        // Operators and the project circuit breaker can block all dispatches
        // for a given project by creating a sentinel file at
        // `<pause_dir>/<project_id>`. The gate is project-scoped; legacy /
        // global agents (`def.project == None`) are never blocked here.
        if project_control::is_project_paused(&self.pause_dir, def.project.as_deref()) {
            info!(
                agent = %def.name,
                project = ?def.project,
                pause_dir = %self.pause_dir.display(),
                "skipping spawn: project is paused"
            );
            return Ok(());
        }

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

        // === BUDGET GATE ===
        // Skip spawn entirely if the agent's monthly budget is exhausted.
        // CostTracker::check is already called during routing (for budget
        // pressure scoring), but routing only deprioritises cheaper models;
        // it does not short-circuit dispatch. A fully exhausted agent must
        // not run at all this cycle.
        let budget_check = self.cost_tracker.check(&def.name);
        if budget_check.should_pause() {
            warn!(
                agent = %def.name,
                verdict = %budget_check,
                "skipping spawn: monthly budget exhausted"
            );
            return Ok(());
        }
        if budget_check.should_warn() {
            warn!(
                agent = %def.name,
                verdict = %budget_check,
                "budget near exhaustion; routing will prefer cheaper models"
            );
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
        let supports_model_flag = matches!(
            cli_name,
            "claude" | "claude-code" | "opencode" | "pi-rust" | "pi"
        );

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
            let strategy = self
                .config
                .routing
                .as_ref()
                .map(|r| r.route_selection_strategy)
                .unwrap_or(crate::control_plane::RouteSelectionStrategy::Fastest);
            let engine = control_plane::RoutingDecisionEngine::with_provider_budget_and_strategy(
                kg_arc,
                unhealthy,
                terraphim_router::Router::new(),
                Some(telemetry_arc),
                self.provider_budget_tracker.clone(),
                strategy,
            );
            let ctx = control_plane::DispatchContext {
                agent_name: def.name.clone(),
                task: def.task.clone(),
                static_model: def.model.clone(),
                cli_tool: def.cli_tool.clone(),
                layer: def.layer,
                session_id: None,
                default_tier: def.default_tier.clone(),
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
        } else if supports_model_flag && def.bypass_kg_routing {
            // Fallback respawn (quota exit, wall-clock timeout, or KG-fallback
            // route already selected): the caller has explicitly chosen
            // `cli_tool` and `model`, and re-running KG tier routing here
            // would override their decision and route the spawn back to the
            // just-blocked primary. Honour the static config verbatim.
            info!(
                agent = %def.name,
                "bypassing KG tier routing per agent definition (fallback respawn)"
            );
            def.model.clone()
        } else if supports_model_flag {
            // KG routing first (phase-aware tier selection from markdown rules).
            // Takes priority over static model config so tier routing controls selection.
            let mut unhealthy = self.provider_health.unhealthy_providers();
            unhealthy.extend(self.provider_rate_limits.blocked_providers());
            let kg_decision = self.kg_router.as_ref().and_then(|router| {
                let decision =
                    router.route_agent_with_tier(&def.task, def.default_tier.as_deref())?;
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
        // Skip composition if the model already contains a provider prefix (e.g.
        // from KG routing which returns full model ids like "kimi-for-coding/k2p5").
        let model = if cli_name == "opencode" {
            match (&def.provider, &model) {
                (Some(provider), Some(m)) if !m.contains('/') => {
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

        // Inject prior lessons from shared learning store
        let (lessons_section, lesson_ids) = self.render_lessons_section(&def.name);
        let mut composed_task = if lessons_section.is_empty() {
            composed_task
        } else {
            info!(
                agent = %def.name,
                lessons = lesson_ids.len(),
                "injecting prior lessons into prompt"
            );
            self.injected_learning_ids
                .insert(def.name.clone(), lesson_ids);
            format!("{}\n\n{}", composed_task, lessons_section)
        };

        // Inject evolution memory context if enabled for this agent.
        if def.evolution_enabled && self.evolution_manager.is_enabled() {
            self.evolution_manager.ensure_agent(&def.name);
            let _ = self
                .evolution_manager
                .record_task_start(&def.name, &def.task);
            let evo_ctx = self.evolution_manager.render_context(&def.name);
            if !evo_ctx.is_empty() {
                info!(agent = %def.name, "injecting evolution memory context");
                composed_task = format!("{}\n\n{}", composed_task, evo_ctx);
            }
        }

        // Inject RLM session info if enabled for this agent.
        if def.rlm_enabled.unwrap_or(false) {
            info!(agent = %def.name, "injecting RLM sandboxed execution context");
            composed_task = format!(
                "{}\n\n## RLM Sandboxed Code Execution\n\
                 You have access to sandboxed code execution via terraphim_rlm. \
                 Use the terraphim-rlm MCP tools to execute code in an isolated environment \
                 when you need to run, test, or validate code changes. \
                 Sessions are resource-limited and automatically cleaned up.",
                composed_task
            );
        }

        // Use stdin only when persona was actually resolved (prompt is enriched)
        // or when the task exceeds ARG_MAX safety threshold.
        // Do NOT use stdin for unfound personas -- the bare task is small and
        // stdin delivery to short-lived processes (echo) causes broken pipe races.
        const STDIN_THRESHOLD: usize = 32_768; // 32 KB
        let use_stdin =
            persona_found || !skill_content.is_empty() || composed_task.len() > STDIN_THRESHOLD;

        // Create isolated git worktrees for AI/model-backed agents that may modify code.
        // Review-tier agents (haiku) and simple local commands used in tests do not need isolation.
        let needs_isolation = requires_isolated_worktree(def, model.as_deref());

        // Resolve the git repo directory for worktree operations. Project-bound
        // agents need a worktree from their own repo, not the orchestrator's.
        let repo_dir: &Path = if let Some(pid) = def.project.as_deref() {
            match self.config.project_by_id(pid) {
                Some(p) => p.working_dir.as_path(),
                None => {
                    warn!(
                        agent = %def.name,
                        project_id = %pid,
                        fallback = %self.config.working_dir.display(),
                        "project_by_id returned None, falling back to orchestrator working_dir"
                    );
                    &self.config.working_dir
                }
            }
        } else {
            &self.config.working_dir
        };

        let (worktree_path, worktree_guard) = if needs_isolation {
            let path = self.create_agent_worktree(&def.name, repo_dir).await?;
            let guard = crate::worktree_guard::WorktreeGuard::for_managed(repo_dir, &path);
            (Some(path), Some(guard))
        } else {
            (None, None)
        };
        let agent_working_dir = worktree_path.as_deref().unwrap_or(repo_dir).to_path_buf();

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

        // === CONCURRENCY GATE ===
        let project_id = def
            .project
            .as_deref()
            .unwrap_or(crate::dispatcher::LEGACY_PROJECT_ID);
        let permit = self.concurrency_controller.acquire_any(project_id).await;
        if permit.is_none() {
            warn!(
                agent = %def.name,
                project = %project_id,
                active = self.active_agents.len(),
                "skipping spawn: global concurrency limit reached"
            );
            return Ok(());
        }

        let mut spawn_ctx =
            build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());
        spawn_ctx.working_dir = Some(agent_working_dir.clone());
        spawn_ctx = spawn_ctx.with_env(
            "ADF_WORKING_DIR",
            agent_working_dir.to_string_lossy().into_owned(),
        );
        if let Some(event) = synthetic_event {
            for (key, value) in event.env_vars() {
                spawn_ctx = spawn_ctx.with_env(key, value);
            }
        }

        // Pre-create temp log path so the spawner can write stderr directly
        // to disk, giving us a durable fallback if the broadcast drain lags.
        let _ = std::fs::create_dir_all(&self.agent_log_dir);
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let stderr_tmp_name = format!(".tmp-{}-{}.stderr.log", def.name, ts);
        let stderr_tmp_path = self.agent_log_dir.join(&stderr_tmp_name);
        spawn_ctx = spawn_ctx.with_stderr_log(&stderr_tmp_path);

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

        // Open a streaming log file and spawn a background drain task so
        // output is captured to disk even when the tick interval is long.
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);

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
                worktree_guard,
                routed_model: model.clone(),
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: permit,
                commit_status_post: None,
                output_tmp_path,
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

    /// Handle a `DispatchTask::ReviewPr` dispatch: run the routing engine,
    /// enforce the C1/C3 provider allow-list, and spawn the pr-reviewer agent
    /// with `ADF_PR_*` env overrides carrying the per-dispatch context.
    ///
    /// The task is a no-op (with a warn log) when no `pr-reviewer` agent is
    /// configured for the project yet. Step E adds the canonical
    /// `pr-reviewer.toml` fragment; until then this method must not crash the
    /// reconcile loop.
    ///
    /// Unlike [`spawn_agent`], this path skips persona composition, skill
    /// chain injection, and worktree creation. The pr-reviewer is review-tier
    /// (read-only), so the heavyweight scaffolding from the implementation
    /// spawn path is intentionally left out.
    ///
    /// [`spawn_agent`]: AgentOrchestrator::spawn_agent
    pub(crate) async fn handle_review_pr(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let (pr_number, project, head_sha, author_login, title, diff_loc) = match task {
            dispatcher::DispatchTask::ReviewPr {
                pr_number,
                project,
                head_sha,
                author_login,
                title,
                diff_loc,
            } => (pr_number, project, head_sha, author_login, title, diff_loc),
            other => {
                warn!(task = ?other, "handle_review_pr invoked with non-ReviewPr task; ignoring");
                return Ok(());
            }
        };

        let req = pr_dispatch::ReviewPrRequest {
            pr_number,
            project: project.clone(),
            head_sha: head_sha.clone(),
            author_login,
            title,
            diff_loc,
        };

        // ADF Phase 2 (issue #944): fan-out over the configured
        // `agents_on_pr_open` list. Each entry is gated independently
        // (subscription allow-list + per-agent monthly budget). Only
        // entries that successfully spawn get a `pending` commit status
        // — a `pending` from a skipped agent would block the PR forever.
        // When `[pr_dispatch]` is absent the legacy default ships a single
        // pr-reviewer entry, preserving pre-Phase-2 behaviour.
        let entries = self.config.agents_on_pr_open_for_project(&project);
        for entry in entries {
            let spawned = match entry.name.as_str() {
                "build-runner" => {
                    self.dispatch_build_runner_for_pr(&req, &entry.context)
                        .await?
                }
                _ => {
                    self.dispatch_pr_reviewer_for_pr(&req, &entry.name, &entry.context)
                        .await?
                }
            };
            if spawned {
                self.post_pending_status(
                    &head_sha,
                    pr_number,
                    &project,
                    &entry.context,
                    &format!("{} dispatched", entry.name),
                )
                .await;
            }
        }

        Ok(())
    }

    /// Phase 2 helper: spawn the LLM-style PR review agent (`pr-reviewer`
    /// or any future fan-out entry that runs through the routing engine).
    ///
    /// Returns `Ok(true)` when the agent was spawned and is now in
    /// `active_agents`; `Ok(false)` when it was gated out (no agent
    /// configured for the project, banned static or routed model, or
    /// budget exhausted). The caller posts a `pending` commit status only
    /// when this returns `true`.
    async fn dispatch_pr_reviewer_for_pr(
        &mut self,
        req: &pr_dispatch::ReviewPrRequest,
        agent_name: &str,
        commit_status_context: &str,
    ) -> Result<bool, OrchestratorError> {
        let pr_number = req.pr_number;
        let project = req.project.clone();
        let head_sha = req.head_sha.clone();

        // Look up the agent for this project. Missing entries in the
        // fan-out list must skip silently (no `pending` posted) — a
        // hung pending would block the PR forever.
        let def = match self
            .agent_registry
            .lookup_project(project.as_str(), agent_name)
        {
            Some(agent) => agent.definition.clone(),
            None => {
                warn!(
                    pr_number,
                    project = %project,
                    agent = %agent_name,
                    "ReviewPr skipped: no agent configured for project"
                );
                return Ok(false);
            }
        };

        // === STATIC ALLOW-LIST GATE (pre-routing) ===
        // Belt-and-braces: the load-time config validator rejects banned
        // providers, and `RoutingDecisionEngine` filters them from the
        // candidate pool, but this check guarantees the spawn never runs
        // against a banned static `model` even if the config was mutated
        // at runtime or a future refactor drops the routing filter.
        if let Some(static_model) = def.model.as_deref() {
            if !config::is_allowed_provider(static_model) {
                warn!(
                    agent = %def.name,
                    pr_number,
                    project = %project,
                    model = %static_model,
                    "ReviewPr skipped: static model rejected by subscription allow-list"
                );
                return Ok(false);
            }
        }

        // === BUDGET GATE ===
        let budget_verdict = self.cost_tracker.check(&def.name);
        if budget_verdict.should_pause() {
            warn!(
                agent = %def.name,
                pr_number,
                project = %project,
                verdict = %budget_verdict,
                "ReviewPr skipped: monthly budget exhausted"
            );
            return Ok(false);
        }

        // === ROUTING ===
        // Build a DispatchContext off the per-PR task string so KG/keyword
        // routing can pick a model based on "review" keywords and PR shape.
        let task_string = pr_dispatch::build_review_task(req);
        let kg_arc = self
            .kg_router
            .as_ref()
            .map(|r| std::sync::Arc::new(r.clone()));
        let unhealthy = self.provider_health.unhealthy_providers();
        let telemetry_arc = std::sync::Arc::new(self.telemetry_store.clone());
        let strategy = self
            .config
            .routing
            .as_ref()
            .map(|r| r.route_selection_strategy)
            .unwrap_or(crate::control_plane::RouteSelectionStrategy::Fastest);
        let engine = control_plane::RoutingDecisionEngine::with_provider_budget_and_strategy(
            kg_arc,
            unhealthy,
            terraphim_router::Router::new(),
            Some(telemetry_arc),
            self.provider_budget_tracker.clone(),
            strategy,
        );
        let dispatch_ctx = control_plane::DispatchContext {
            agent_name: def.name.clone(),
            task: task_string.clone(),
            static_model: def.model.clone(),
            cli_tool: def.cli_tool.clone(),
            layer: def.layer,
            session_id: None,
            default_tier: def.default_tier.clone(),
        };
        let decision = engine.decide_route(&dispatch_ctx, &budget_verdict).await;
        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            model = %decision.candidate.model,
            rationale = %decision.rationale,
            "ReviewPr routing decision"
        );

        // === C1/C3 ALLOW-LIST GATE ===
        // Routing may suggest a banned provider (e.g. via stale KG rules); the
        // subscription-only allow-list must still short-circuit the spawn so
        // unsanctioned providers never launch.
        let routed_model = decision.candidate.model.clone();
        let effective_cli = if decision.candidate.cli_tool.is_empty() {
            def.cli_tool.clone()
        } else {
            decision.candidate.cli_tool.clone()
        };
        if !routed_model.is_empty() && !config::is_allowed_provider(&routed_model) {
            warn!(
                agent = %def.name,
                pr_number,
                project = %project,
                model = %routed_model,
                "ReviewPr skipped: routed model rejected by subscription allow-list"
            );
            return Ok(false);
        }

        // === SPAWN ===
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: effective_cli.clone(),
                working_dir: self.config.working_dir_for_agent(&def),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let fallback_provider = def.fallback_provider.as_ref().map(|fallback_cli| {
            terraphim_types::capability::Provider {
                id: format!("{}-fallback", def.name),
                name: format!("{} (fallback)", def.name),
                provider_type: terraphim_types::capability::ProviderType::Agent {
                    agent_id: format!("{}-fallback", def.name),
                    cli_command: fallback_cli.clone(),
                    working_dir: self.config.working_dir_for_agent(&def),
                },
                capabilities: vec![],
                cost_level: terraphim_types::capability::CostLevel::Cheap,
                latency: terraphim_types::capability::Latency::Medium,
                keywords: def.capabilities.clone(),
            }
        });

        // Issue #1020: pass the TOML `task` body (script / system prompt)
        // to the spawner -- not the runtime informational summary.
        // The summary is layered as ADF_TASK_SUMMARY env so future TOML
        // scripts can reference it without a code change.
        // Bug #2450 fix: pr-reviewer agent was receiving `def.task` ("review")
        // instead of `task_string` (the full PR review description), causing
        // the agent to exit with empty_success in 2s. The TOML `task` field
        // is a label/placeholder for pr-reviewer; the actual work is built
        // by build_review_task(req) into task_string.
        let mut request = SpawnRequest::new(primary_provider, &task_string);
        if !routed_model.is_empty() {
            request = request.with_primary_model(&routed_model);
        }
        if let Some(fallback) = fallback_provider {
            request = request.with_fallback_provider(fallback);
            if let Some(fallback_model) = &def.fallback_model {
                request = request.with_fallback_model(fallback_model);
            }
        }

        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        let base_ctx =
            build_spawn_context_for_agent(&self.config, &def, self.output_poster.as_ref());
        let spawn_ctx = pr_dispatch::layer_pr_env(base_ctx, req)
            .with_env("ADF_TASK_SUMMARY", task_string.clone());

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        let output_rx = handle.subscribe_output();
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);
        let restart_count = self
            .restart_counts
            .get(&agent_key(&def))
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
                worktree_path: None,
                worktree_guard: None,
                routed_model: if routed_model.is_empty() {
                    None
                } else {
                    Some(routed_model)
                },
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: None,
                commit_status_post: Some((head_sha.clone(), commit_status_context.to_string())),
                output_tmp_path,
            },
        );

        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            head_sha = %head_sha,
            "ReviewPr spawned LLM review agent"
        );

        Ok(true)
    }

    /// Phase 2 helper: spawn the deterministic `build-runner` agent on a
    /// `pull_request.opened` event. Mirrors `handle_push`'s spawn pipeline
    /// but injects PR-shaped `ADF_PUSH_*` env (using
    /// `refs/pull/<n>/head` as the synthetic ref) so the same bash task
    /// script handles both push events and PR opens.
    ///
    /// Skips the routing engine — `build-runner` is bash-only (no LLM, no
    /// model) so a routing decision would invite a false-positive
    /// banned-provider check on an unset `def.model`. Logs a synthetic
    /// `model = "n/a"` row for parity with the LLM path.
    ///
    /// Returns `Ok(true)` on successful spawn (caller posts pending);
    /// `Ok(false)` when gated out.
    async fn dispatch_build_runner_for_pr(
        &mut self,
        req: &pr_dispatch::ReviewPrRequest,
        commit_status_context: &str,
    ) -> Result<bool, OrchestratorError> {
        let pr_number = req.pr_number;
        let project = req.project.clone();
        let head_sha = req.head_sha.clone();

        if self.active_agents.contains_key("build-runner") {
            info!(
                pr_number,
                project = %project,
                head_sha = %head_sha,
                "ReviewPr skipped build-runner: already active from concurrent push dispatch"
            );
            return Ok(false);
        }

        // Look up the build-runner agent for this project. Missing must
        // skip silently — no `pending` posted by the caller.
        let def = match self
            .agent_registry
            .lookup_project(project.as_str(), "build-runner")
        {
            Some(agent) => agent.definition.clone(),
            None => {
                warn!(
                    pr_number,
                    project = %project,
                    "ReviewPr skipped: no build-runner agent configured for project"
                );
                return Ok(false);
            }
        };

        // === STATIC ALLOW-LIST GATE ===
        // build-runner is bash-only (no LLM), so def.model is normally None
        // and this gate is a no-op. The check is retained for defence in
        // depth so a future config that mis-sets `model` cannot bypass C1/C3.
        if let Some(static_model) = def.model.as_deref() {
            if !config::is_allowed_provider(static_model) {
                warn!(
                    agent = %def.name,
                    pr_number,
                    project = %project,
                    model = %static_model,
                    "ReviewPr skipped: build-runner static model rejected by subscription allow-list"
                );
                return Ok(false);
            }
        }

        // === BUDGET GATE ===
        let budget_verdict = self.cost_tracker.check(&def.name);
        if budget_verdict.should_pause() {
            warn!(
                agent = %def.name,
                pr_number,
                project = %project,
                verdict = %budget_verdict,
                "ReviewPr skipped: build-runner monthly budget exhausted"
            );
            return Ok(false);
        }

        // === ROUTING DECISION (observability only) ===
        // build-runner is bash; mirror handle_push's synthetic log row so
        // dashboards see one entry per dispatch. No call to decide_route —
        // an LLM router on an unset model would surface false positives.
        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            model = "n/a",
            cost_estimate_cents = 0,
            rationale = "deterministic build-runner (no LLM)",
            "ReviewPr routing decision"
        );

        // === SPAWN ===
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: def.cli_tool.clone(),
                working_dir: self.config.working_dir_for_agent(&def),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let task_string = format!(
            "Build/test verdict for PR #{} (head={}, {} LOC, project={}, author={})",
            pr_number, head_sha, req.diff_loc, project, req.author_login,
        );

        // Issue #1020: pass the TOML `task` body (the bash script that
        // does git fetch / rch exec / curl status post) to the spawner
        // -- not the runtime informational summary, which would have
        // been interpreted as `bash -c "Build/test verdict ..."` and
        // exited 127 on the first non-existent command.
        let mut request = SpawnRequest::new(primary_provider, &def.task);

        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        // Layer ADF_PUSH_* env on top of the per-agent base context.
        // The ref is synthesised as `refs/pull/<n>/head` so the task
        // script can `git fetch origin <ref> && git checkout <sha>`
        // identically to a push-event dispatch. `ADF_PUSH_BEFORE_SHA`
        // is empty because the ReviewPr dispatch task does not carry
        // the PR base SHA — the build-runner script only requires
        // `ADF_PUSH_SHA` and `ADF_PUSH_REF`.
        // ADF_TASK_SUMMARY exposes the runtime summary so the task can
        // log it without a code change (issue #1020).
        let mut spawn_ctx =
            build_spawn_context_for_agent(&self.config, &def, self.output_poster.as_ref());
        spawn_ctx = spawn_ctx
            .with_env("ADF_PUSH_SHA", head_sha.clone())
            .with_env("ADF_PUSH_REF", format!("refs/pull/{}/head", pr_number))
            .with_env("ADF_PUSH_PROJECT", project.clone())
            .with_env("ADF_PUSH_BEFORE_SHA", String::new())
            .with_env("ADF_PUSH_PUSHER", req.author_login.clone())
            .with_env("ADF_PUSH_FILES", String::new())
            .with_env("ADF_TASK_SUMMARY", task_string.clone());

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        let output_rx = handle.subscribe_output();
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);
        let restart_count = self
            .restart_counts
            .get(&agent_key(&def))
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
                worktree_path: None,
                worktree_guard: None,
                routed_model: None,
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: None,
                commit_status_post: Some((head_sha.clone(), commit_status_context.to_string())),
                output_tmp_path,
            },
        );

        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            head_sha = %head_sha,
            "ReviewPr spawned build-runner"
        );

        Ok(true)
    }

    /// Post a `pending` commit status for the given `context` against the
    /// PR head SHA.
    ///
    /// Generalised from Phase 1's `post_pr_reviewer_pending_status` so the
    /// Phase 2 PR-fan-out path can post one pending per dispatched agent
    /// (one row per `agents_on_pr_open` entry that successfully spawned).
    ///
    /// Best-effort: when the workflow tracker isn't configured (e.g. in
    /// unit tests) or the API call fails we log and return without
    /// surfacing the error. The agent itself owns the final state
    /// transition (success / failure / error).
    async fn post_pending_status(
        &mut self,
        head_sha: &str,
        pr_number: u64,
        project: &str,
        context: &str,
        description: &str,
    ) {
        let tracker = match self.get_or_init_pre_check_tracker() {
            Some(t) => t,
            None => {
                debug!(
                    pr_number,
                    project,
                    context,
                    "ReviewPr: no workflow tracker configured; skipping pending status"
                );
                return;
            }
        };
        let owner = tracker.owner().to_string();
        let repo = tracker.repo().to_string();
        let result = tracker
            .set_commit_status(
                &owner,
                &repo,
                head_sha,
                terraphim_tracker::StatusState::Pending,
                context,
                description,
                None,
            )
            .await;
        match result {
            Ok(()) => {
                info!(
                    pr_number,
                    project, head_sha, context, "ReviewPr: posted pending status"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    pr_number,
                    project,
                    head_sha,
                    context,
                    "ReviewPr: failed to post pending status"
                );
            }
        }
    }

    /// Post a terminal (success/failure) commit status for an agent that
    /// exited. Best-effort: logs on failure but does not propagate errors.
    async fn post_terminal_commit_status(
        &mut self,
        head_sha: &str,
        context: &str,
        state: terraphim_tracker::StatusState,
        description: &str,
    ) {
        let tracker = match self.get_or_init_pre_check_tracker() {
            Some(t) => t,
            None => {
                debug!(
                    head_sha,
                    context, "post_terminal_commit_status: no workflow tracker; skipping"
                );
                return;
            }
        };
        let owner = tracker.owner().to_string();
        let repo = tracker.repo().to_string();
        match tracker
            .set_commit_status(&owner, &repo, head_sha, state, context, description, None)
            .await
        {
            Ok(()) => {
                info!(head_sha, context, "posted terminal commit status");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    head_sha,
                    context,
                    "failed to post terminal commit status"
                );
            }
        }
    }

    /// Handle a `DispatchTask::Push` dispatch (Phase 3 — ADF replaces Gitea
    /// Actions): look up the project's `build-runner` agent, gate on the
    /// subscription allow-list and monthly budget, log a routing decision row
    /// for observability (even though `build-runner` is bash, not LLM), then
    /// spawn it with `ADF_PUSH_*` env injection so the bash task can shell
    /// out to `rch exec` for the deterministic cargo gates.
    ///
    /// The handler is a no-op (with warn log) when no `build-runner` agent is
    /// configured for the project — repos without build-runner must not break
    /// the orchestrator drain loop.
    pub(crate) async fn handle_push(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let (project, ref_name, before_sha, after_sha, pusher_login, files_changed) = match task {
            dispatcher::DispatchTask::Push {
                project,
                ref_name,
                before_sha,
                after_sha,
                pusher_login,
                files_changed,
            } => (
                project,
                ref_name,
                before_sha,
                after_sha,
                pusher_login,
                files_changed,
            ),
            other => {
                warn!(task = ?other, "handle_push invoked with non-Push task; ignoring");
                return Ok(());
            }
        };

        // Look up the build-runner agent for this project. Repos without
        // build-runner shouldn't break the orchestrator -- log and skip.
        let def = match self
            .agent_registry
            .lookup_project(project.as_str(), "build-runner")
        {
            Some(agent) => agent.definition.clone(),
            None => {
                warn!(
                    project = %project,
                    after_sha = %after_sha,
                    "Push skipped: no build-runner agent configured for project"
                );
                return Ok(());
            }
        };

        if !def.enabled {
            info!(
                agent = %def.name,
                project = %project,
                "Push skipped: build-runner agent is disabled"
            );
            return Ok(());
        }

        if self.active_agents.contains_key("build-runner") {
            info!(
                project = %project,
                after_sha = %after_sha,
                "Push skipped build-runner: already active from concurrent dispatch"
            );
            return Ok(());
        }

        // === STATIC ALLOW-LIST GATE ===
        // build-runner is bash-only (no LLM), so def.model is normally None
        // and this gate is a no-op. The check is retained for defence in
        // depth so a future config that mis-sets `model` cannot bypass C1/C3.
        if let Some(static_model) = def.model.as_deref() {
            if !config::is_allowed_provider(static_model) {
                warn!(
                    agent = %def.name,
                    project = %project,
                    model = %static_model,
                    "Push skipped: static model rejected by subscription allow-list"
                );
                return Ok(());
            }
        }

        // === BUDGET GATE ===
        // build-runner has no LLM cost but the budget tracker still records
        // its dispatches; pause if the operator deliberately capped it.
        let budget_verdict = self.cost_tracker.check(&def.name);
        if budget_verdict.should_pause() {
            warn!(
                agent = %def.name,
                project = %project,
                verdict = %budget_verdict,
                "Push skipped: build-runner monthly budget exhausted"
            );
            return Ok(());
        }

        // === ROUTING DECISION (observability only) ===
        // Even though build-runner is bash, we still log a routing decision
        // row so the dashboard sees one entry per dispatch. Cost is 0 because
        // there is no LLM, and the model column reads "n/a".
        info!(
            agent = %def.name,
            project = %project,
            ref_name = %ref_name,
            after_sha = %after_sha,
            model = "n/a",
            cost_estimate_cents = 0,
            rationale = "deterministic build-runner (no LLM)",
            "Push routing decision"
        );

        // === SPAWN ===
        // build-runner is a plain bash agent: cli_tool from the def, no
        // primary model, no fallback model. Mirror the SpawnRequest shape
        // used by handle_review_pr but skip the LLM-specific overrides.
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: def.cli_tool.clone(),
                working_dir: self.config.working_dir_for_agent(&def),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let task_string = format!(
            "Build/test verdict for push to {} ({} → {}, {} files changed) on project={}, pushed by {}",
            ref_name,
            before_sha,
            after_sha,
            files_changed.len(),
            project,
            pusher_login,
        );

        // Issue #1020: pass the TOML `task` body (build-runner bash
        // script) to the spawner -- not the runtime informational
        // summary. The summary is layered as ADF_TASK_SUMMARY env.
        let mut request = SpawnRequest::new(primary_provider, &def.task);

        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        // Layer the ADF_PUSH_* env on top of the per-agent base context.
        let mut spawn_ctx =
            build_spawn_context_for_agent(&self.config, &def, self.output_poster.as_ref());
        spawn_ctx = spawn_ctx
            .with_env("ADF_PUSH_SHA", after_sha.clone())
            .with_env("ADF_PUSH_REF", ref_name.clone())
            .with_env("ADF_PUSH_PROJECT", project.clone())
            .with_env("ADF_PUSH_BEFORE_SHA", before_sha.clone())
            .with_env("ADF_PUSH_PUSHER", pusher_login.clone())
            .with_env("ADF_PUSH_FILES", files_changed.join("\n"))
            .with_env("ADF_TASK_SUMMARY", task_string.clone());

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        let output_rx = handle.subscribe_output();
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);
        let restart_count = self
            .restart_counts
            .get(&agent_key(&def))
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
                worktree_path: None,
                worktree_guard: None,
                routed_model: None,
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: None,
                commit_status_post: Some((after_sha.clone(), "adf/build".to_string())),
                output_tmp_path,
            },
        );

        self.post_pending_status(
            &after_sha,
            0,
            &project,
            "adf/build",
            "build-runner dispatched",
        )
        .await;

        info!(
            agent = %def.name,
            project = %project,
            ref_name = %ref_name,
            after_sha = %after_sha,
            "Push spawned build-runner"
        );

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
        let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
        let max_mention_depth = mention_cfg.max_mention_depth;

        match dispatch {
            webhook::WebhookDispatch::SpawnAgent {
                agent_name,
                detected_project,
                issue_number,
                comment_id,
                context,
                synthetic_event: _,
            } => {
                info!(
                    agent = %agent_name,
                    project = ?detected_project,
                    issue = issue_number,
                    comment_id = comment_id,
                    "webhook: dispatching agent spawn"
                );

                // Use project-aware resolver. For webhook dispatches we don't know which
                // project's repo the webhook came from, so we use LEGACY_PROJECT_ID as the
                // hint for unqualified mentions; qualified mentions carry detected_project.
                if let Some(def) = mention::resolve_mention(
                    detected_project.as_deref(),
                    dispatcher::LEGACY_PROJECT_ID,
                    &agent_name,
                    &agents,
                ) {
                    // Event-only agents (e.g. build-runner) must not be dispatched
                    // from comment mentions. They are spawned by handle_push or
                    // other event handlers with the appropriate context env vars.
                    // Rejecting here prevents ghost-issue posts and wasted spawns.
                    if def.event_only {
                        info!(
                            agent = %agent_name,
                            issue = issue_number,
                            comment_id = comment_id,
                            "webhook dispatch rejected: agent is event-only (push/event-driven), not mention-dispatchable"
                        );
                        return;
                    }

                    // Dedup: check Gitea assignment + active_agents before spawning
                    if self.should_skip_dispatch(&agent_name, issue_number).await {
                        return;
                    }

                    let chain_id = ulid::Ulid::new().to_string();
                    let depth: u32 = 0;
                    let parent_agent = String::new();

                    if let Err(e) = mention_chain::MentionChainTracker::check(
                        depth,
                        &parent_agent,
                        &agent_name,
                        max_mention_depth,
                    ) {
                        warn!(
                            agent = %agent_name,
                            chain_id = %chain_id,
                            depth,
                            error = %e,
                            "webhook mention chain check rejected dispatch"
                        );
                        if let Some(ref poster) = self.output_poster {
                            let body = format!(
                                "## Mention Dispatch Blocked\n\n\
                                Agent `{}` was not spawned: {}.\n\n\
                                _Webhook chain `{}` blocked._",
                                agent_name, e, chain_id
                            );
                            if let Err(pe) = poster.post_raw(issue_number, &body).await {
                                warn!(error = %pe, "failed to post webhook chain rejection comment");
                            }
                        }
                        return;
                    }

                    let ctx_args = mention_chain::MentionContextArgs {
                        parent_agent: parent_agent.clone(),
                        issue_number,
                        comment_body: context.clone(),
                        depth,
                        chain_id: chain_id.clone(),
                        available_agents: agent_names
                            .iter()
                            .filter(|n| *n != &agent_name)
                            .cloned()
                            .collect(),
                    };
                    let chain_ctx = mention_chain::MentionChainTracker::build_context(
                        &ctx_args,
                        max_mention_depth,
                    );

                    let mut mention_def = def.clone();
                    mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                    mention_def.gitea_issue = Some(issue_number);

                    if let Err(e) = self.spawn_agent(&mention_def).await {
                        error!(agent = %agent_name, issue = issue_number, error = %e, "webhook: failed to spawn agent");
                    } else if let Some(agent) = self.active_agents.get_mut(&mention_def.name) {
                        agent.spawned_by_mention = true;
                        agent.mention_chain_id = Some(chain_id);
                        agent.mention_depth = Some(depth);
                        agent.mention_parent_agent = None;
                    }
                }
            }
            webhook::WebhookDispatch::SpawnPersona {
                persona_name,
                issue_number,
                comment_id: _,
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
                        // Event-only agents must not be dispatched via persona-mention
                        // either. Same rationale as the SpawnAgent arm.
                        if def.event_only {
                            info!(
                                persona = %persona_name,
                                agent = %agent_name,
                                issue = issue_number,
                                "webhook dispatch rejected: persona-resolved agent is event-only (push/event-driven), not mention-dispatchable"
                            );
                            return;
                        }

                        // Dedup: check Gitea assignment + active_agents before spawning
                        if self.should_skip_dispatch(&agent_name, issue_number).await {
                            return;
                        }

                        let chain_id = ulid::Ulid::new().to_string();
                        let depth: u32 = 0;
                        let parent_agent = String::new();

                        if let Err(e) = mention_chain::MentionChainTracker::check(
                            depth,
                            &parent_agent,
                            &agent_name,
                            max_mention_depth,
                        ) {
                            warn!(
                                agent = %agent_name,
                                chain_id = %chain_id,
                                depth,
                                error = %e,
                                "webhook mention chain check rejected persona dispatch"
                            );
                            if let Some(ref poster) = self.output_poster {
                                let body = format!(
                                    "## Mention Dispatch Blocked\n\n\
                                    Agent `{}` (via persona) was not spawned: {}.\n\n\
                                    _Webhook chain `{}` blocked._",
                                    agent_name, e, chain_id
                                );
                                if let Err(pe) = poster.post_raw(issue_number, &body).await {
                                    warn!(error = %pe, "failed to post webhook chain rejection comment");
                                }
                            }
                            return;
                        }

                        let ctx_args = mention_chain::MentionContextArgs {
                            parent_agent: parent_agent.clone(),
                            issue_number,
                            comment_body: context.clone(),
                            depth,
                            chain_id: chain_id.clone(),
                            available_agents: agent_names
                                .iter()
                                .filter(|n| *n != &agent_name)
                                .cloned()
                                .collect(),
                        };
                        let chain_ctx = mention_chain::MentionChainTracker::build_context(
                            &ctx_args,
                            max_mention_depth,
                        );

                        let mut mention_def = def.clone();
                        mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                        mention_def.gitea_issue = Some(issue_number);

                        if let Err(e) = self.spawn_agent(&mention_def).await {
                            error!(agent = %agent_name, issue = issue_number, error = %e, "webhook: failed to spawn agent");
                        } else if let Some(agent) = self.active_agents.get_mut(&mention_def.name) {
                            agent.spawned_by_mention = true;
                            agent.mention_chain_id = Some(chain_id);
                            agent.mention_depth = Some(depth);
                            agent.mention_parent_agent = None;
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
            webhook::WebhookDispatch::ReviewPr {
                pr_number,
                project,
                head_sha,
                author_login,
                title,
                diff_loc,
            } => {
                info!(
                    pr = pr_number,
                    project = %project,
                    head_sha = %head_sha,
                    author = %author_login,
                    diff_loc = diff_loc,
                    "webhook: enqueuing ReviewPr dispatch task"
                );
                self.dispatcher.enqueue(dispatcher::DispatchTask::ReviewPr {
                    pr_number,
                    project,
                    head_sha,
                    author_login,
                    title,
                    diff_loc,
                });
            }
            webhook::WebhookDispatch::Push {
                project,
                ref_name,
                before_sha,
                after_sha,
                pusher_login,
                files_changed,
            } => {
                info!(
                    project = %project,
                    ref_name = %ref_name,
                    after_sha = %after_sha,
                    pusher = %pusher_login,
                    files = files_changed.len(),
                    "webhook: enqueuing Push dispatch task"
                );
                self.dispatcher.enqueue(dispatcher::DispatchTask::Push {
                    project,
                    ref_name,
                    before_sha,
                    after_sha,
                    pusher_login,
                    files_changed,
                });
            }
        }
    }

    async fn handle_direct_dispatch(&mut self, dispatch: webhook::WebhookDispatch) {
        match dispatch {
            webhook::WebhookDispatch::SpawnAgent {
                agent_name,
                detected_project,
                context,
                synthetic_event,
                ..
            } => {
                // Use project-aware resolution for qualified agent names.
                let def = mention::resolve_mention(
                    detected_project.as_deref(),
                    dispatcher::LEGACY_PROJECT_ID,
                    &agent_name,
                    &self.config.agents,
                );

                let def = match def {
                    Some(def) => def,
                    None => {
                        // Fallback to simple name lookup for legacy compatibility.
                        warn!(agent = %agent_name, "direct dispatch: agent not found in config");
                        return;
                    }
                };

                if !def.enabled {
                    info!(agent = %agent_name, "direct dispatch rejected: agent is disabled");
                    return;
                }

                let mut direct_def = def.clone();
                if !context.is_empty() {
                    direct_def.task =
                        format!("{}\n\n[direct dispatch context]\n{}", def.task, context);
                }

                if def.event_only {
                    info!(
                        agent = %agent_name,
                        event = ?synthetic_event,
                        "direct dispatch override: spawning event_only agent locally"
                    );
                } else {
                    info!(agent = %agent_name, "direct dispatch: spawning agent");
                }
                if let Err(e) = self
                    .spawn_agent_with_event(&direct_def, synthetic_event.as_ref())
                    .await
                {
                    error!(agent = %agent_name, error = %e, "direct dispatch: failed to spawn agent");
                }
            }
            other => {
                warn!(dispatch = ?other, "direct dispatch ignored unsupported dispatch type");
            }
        }
    }

    /// Uses repo-wide comments endpoint with `since` cursor. On first run
    /// (no persisted cursor), cursor is set to `now` to skip all historical
    /// mentions — preventing the mention replay storm.
    async fn poll_mentions(&mut self) {
        // Build the list of (project_id, gitea_cfg, mention_cfg) targets.
        //
        // - Legacy mode (no `[[projects]]`): one pass under the synthetic
        //   `__global__` id using the top-level `gitea` and `mentions`.
        // - Multi-project mode: one pass per configured project that
        //   declares a `gitea` block. Per-project `mentions` override the
        //   top-level `mentions`, which in turn falls back to
        //   `MentionConfig::default()` so operators need not repeat caps
        //   in every project.
        let targets: Vec<(String, config::GiteaOutputConfig, config::MentionConfig)> =
            if self.config.projects.is_empty() {
                match (self.config.mentions.clone(), self.config.gitea.clone()) {
                    (Some(m), Some(g)) => {
                        vec![(dispatcher::LEGACY_PROJECT_ID.to_string(), g, m)]
                    }
                    _ => {
                        tracing::debug!(
                            "mention polling skipped: legacy mode but no Gitea/mentions config"
                        );
                        return;
                    }
                }
            } else {
                let global_mentions = self.config.mentions.clone();
                self.config
                    .projects
                    .iter()
                    .filter_map(|project| {
                        if project.gitea.is_none() {
                            tracing::debug!(
                                project = project.id.as_str(),
                                "skipping mention poll: project has no gitea config"
                            );
                        }
                        let gitea = project.gitea.clone()?;
                        let mentions = project
                            .mentions
                            .clone()
                            .or_else(|| global_mentions.clone())
                            .unwrap_or_default();
                        Some((project.id.clone(), gitea, mentions))
                    })
                    .collect()
            };

        if targets.is_empty() {
            tracing::debug!("mention polling skipped: no projects with Gitea config");
            return;
        }

        for (project_id, gitea_cfg, mention_cfg) in targets {
            self.poll_mentions_for_project(&project_id, &gitea_cfg, &mention_cfg)
                .await;
        }
    }

    /// Run a single mention-poll pass for one project.
    ///
    /// Invoked by [`AgentOrchestrator::poll_mentions`] for each configured
    /// project (or once for legacy single-project mode under
    /// `__global__`). Loads/persists the project's cursor, honours the
    /// project's `MentionConfig`, and threads `project_id` onto every
    /// dispatched mention.
    async fn poll_mentions_for_project(
        &mut self,
        project_id: &str,
        gitea_cfg: &config::GiteaOutputConfig,
        mention_cfg: &config::MentionConfig,
    ) {
        // Respect poll_modulo to reduce API traffic.
        if self.tick_count % mention_cfg.poll_modulo != 0 {
            return;
        }

        // Count currently active mention-spawned agents for this project.
        //
        // We filter by the agent definition's project field so one noisy
        // project cannot exhaust the fleet-wide mention budget for others.
        // In legacy mode (project_id == "__global__") every agent
        // contributes because project binding isn't meaningful there.
        let active_mention_agents = if project_id == dispatcher::LEGACY_PROJECT_ID {
            self.active_agents
                .values()
                .filter(|a| a.spawned_by_mention)
                .count() as u32
        } else {
            self.active_agents
                .values()
                .filter(|a| {
                    a.spawned_by_mention && a.definition.project.as_deref() == Some(project_id)
                })
                .count() as u32
        };
        if active_mention_agents >= mention_cfg.max_concurrent_mention_agents {
            tracing::debug!(
                project = project_id,
                active = active_mention_agents,
                max = mention_cfg.max_concurrent_mention_agents,
                "mention agents at capacity, skipping poll"
            );
            return;
        }

        // Lazy-load the project's cursor.
        let mut cursor = match self.mention_cursors.remove(project_id) {
            Some(c) => c,
            None => mention::MentionCursor::load_or_now(project_id).await,
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
                tracing::warn!(
                    project = project_id,
                    error = %e,
                    "failed to create GiteaTracker for mention polling"
                );
                self.mention_cursors.insert(project_id.to_string(), cursor);
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
                tracing::warn!(
                    project = project_id,
                    error = %e,
                    "failed to fetch repo comments for mention polling"
                );
                self.mention_cursors.insert(project_id.to_string(), cursor);
                return;
            }
        };

        if comments.is_empty() {
            cursor.save(project_id).await;
            self.mention_cursors.insert(project_id.to_string(), cursor);
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

        let max_mention_depth = mention_cfg.max_mention_depth;

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

            // Handle qualified `@adf:project/name` mentions that AdfCommandParser cannot
            // see (its patterns are `@adf:{name}`; a `project/` prefix is not a substring).
            for token in mention::parse_mention_tokens(&comment.body) {
                if cursor.dispatches_this_tick >= max_dispatches {
                    break;
                }
                let proj = match token.project.as_deref() {
                    Some(p) => p,
                    None => continue, // unqualified mentions are handled by parse_commands below
                };
                match mention::resolve_mention(Some(proj), project_id, &token.agent, &agents) {
                    Some(def) => {
                        info!(
                            agent = %token.agent,
                            project = proj,
                            issue = comment.issue_number,
                            comment_id = comment.id,
                            "dispatching qualified mention-driven agent"
                        );
                        // Event-only agents (e.g. build-runner) must not be dispatched
                        // from comment mentions. Reject before any spawn-related work.
                        if def.event_only {
                            info!(
                                agent = %token.agent,
                                issue = comment.issue_number,
                                comment_id = comment.id,
                                "poll mention dispatch rejected: agent is event-only (push/event-driven), not mention-dispatchable"
                            );
                            cursor.dispatches_this_tick += 1;
                            continue;
                        }
                        if self
                            .should_skip_dispatch(&token.agent, comment.issue_number)
                            .await
                        {
                            cursor.dispatches_this_tick += 1;
                            continue;
                        }

                        let (chain_id, depth, parent_agent) = self.resolve_mention_chain(
                            &comment.user.login,
                            &agent_names,
                            max_mention_depth,
                        );

                        if let Err(e) = mention_chain::MentionChainTracker::check(
                            depth,
                            &parent_agent,
                            &token.agent,
                            max_mention_depth,
                        ) {
                            warn!(
                                agent = %token.agent,
                                chain_id = %chain_id,
                                depth,
                                error = %e,
                                "mention chain check rejected dispatch"
                            );
                            if let Some(ref poster) = self.output_poster {
                                let body = format!(
                                    "## Mention Dispatch Blocked\n\n\
                                    Agent `{}` was not spawned: {}.\n\n\
                                    _Chain `{}` at depth {} exceeds the configured limit._",
                                    token.agent, e, chain_id, depth
                                );
                                if let Err(pe) = poster
                                    .post_raw_for_project(project_id, comment.issue_number, &body)
                                    .await
                                {
                                    warn!(error = %pe, "failed to post chain rejection comment");
                                }
                            }
                            cursor.dispatches_this_tick += 1;
                            continue;
                        }

                        let ctx_args = mention_chain::MentionContextArgs {
                            parent_agent: parent_agent.clone(),
                            issue_number: comment.issue_number,
                            comment_body: comment.body.clone(),
                            depth,
                            chain_id: chain_id.clone(),
                            available_agents: agent_names
                                .iter()
                                .filter(|n| *n != &token.agent)
                                .cloned()
                                .collect(),
                        };
                        let chain_ctx = mention_chain::MentionChainTracker::build_context(
                            &ctx_args,
                            max_mention_depth,
                        );

                        let mut mention_def = def.clone();
                        mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                        mention_def.gitea_issue = Some(comment.issue_number);
                        if let Err(e) = self.spawn_agent(&mention_def).await {
                            tracing::error!(
                                agent = %token.agent,
                                project = proj,
                                issue = comment.issue_number,
                                error = %e,
                                "failed to spawn agent for qualified mention"
                            );
                        } else if let Some(active) = self.active_agents.get_mut(&mention_def.name) {
                            active.spawned_by_mention = true;
                            active.mention_chain_id = Some(chain_id);
                            active.mention_depth = Some(depth);
                            active.mention_parent_agent = if parent_agent.is_empty() {
                                None
                            } else {
                                Some(parent_agent)
                            };
                        }
                        cursor.dispatches_this_tick += 1;
                    }
                    None => {
                        tracing::warn!(
                            mention = format!("@adf:{}/{}", proj, token.agent),
                            project = project_id,
                            issue = comment.issue_number,
                            "qualified mention matched no agent"
                        );
                    }
                }
            }

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

                        if let Some(def) =
                            mention::resolve_mention(None, project_id, &agent_name, &agents)
                        {
                            // Event-only agents (e.g. build-runner) must not be dispatched
                            // from comment mentions. Reject before any spawn-related work.
                            if def.event_only {
                                info!(
                                    agent = %agent_name,
                                    issue = issue_number,
                                    comment_id = comment_id,
                                    "poll mention dispatch rejected: agent is event-only (push/event-driven), not mention-dispatchable"
                                );
                                cursor.dispatches_this_tick += 1;
                                continue;
                            }
                            if self.should_skip_dispatch(&agent_name, issue_number).await {
                                cursor.dispatches_this_tick += 1;
                                continue;
                            }

                            let (chain_id, depth, parent_agent) = self.resolve_mention_chain(
                                &comment.user.login,
                                &agent_names,
                                max_mention_depth,
                            );

                            if let Err(e) = mention_chain::MentionChainTracker::check(
                                depth,
                                &parent_agent,
                                &agent_name,
                                max_mention_depth,
                            ) {
                                warn!(
                                    agent = %agent_name,
                                    chain_id = %chain_id,
                                    depth,
                                    error = %e,
                                    "mention chain check rejected dispatch"
                                );
                                if let Some(ref poster) = self.output_poster {
                                    let body = format!(
                                        "## Mention Dispatch Blocked\n\n\
                                        Agent `{}` was not spawned: {}.\n\n\
                                        _Chain `{}` at depth {} exceeds the configured limit._",
                                        agent_name, e, chain_id, depth
                                    );
                                    if let Err(pe) = poster
                                        .post_raw_for_project(project_id, issue_number, &body)
                                        .await
                                    {
                                        warn!(error = %pe, "failed to post chain rejection comment");
                                    }
                                }
                                cursor.dispatches_this_tick += 1;
                                continue;
                            }

                            let ctx_args = mention_chain::MentionContextArgs {
                                parent_agent: parent_agent.clone(),
                                issue_number,
                                comment_body: context.clone(),
                                depth,
                                chain_id: chain_id.clone(),
                                available_agents: agent_names
                                    .iter()
                                    .filter(|n| *n != &agent_name)
                                    .cloned()
                                    .collect(),
                            };
                            let chain_ctx = mention_chain::MentionChainTracker::build_context(
                                &ctx_args,
                                max_mention_depth,
                            );

                            let mut mention_def = def.clone();
                            mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                            mention_def.gitea_issue = Some(issue_number);

                            if let Err(e) = self.spawn_agent(&mention_def).await {
                                tracing::error!(agent = %agent_name, issue = issue_number, error = %e, "failed to spawn agent");
                            } else if let Some(agent) =
                                self.active_agents.get_mut(&mention_def.name)
                            {
                                agent.spawned_by_mention = true;
                                agent.mention_chain_id = Some(chain_id);
                                agent.mention_depth = Some(depth);
                                agent.mention_parent_agent = if parent_agent.is_empty() {
                                    None
                                } else {
                                    Some(parent_agent)
                                };
                            }

                            cursor.dispatches_this_tick += 1;
                        }
                    }
                    crate::adf_commands::AdfCommand::SpawnPersona {
                        persona_name,
                        issue_number,
                        comment_id: _,
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
                                // Event-only agents (e.g. build-runner) must not be dispatched
                                // from persona mentions. Reject before any spawn-related work.
                                if def.event_only {
                                    info!(
                                        persona = %persona_name,
                                        agent = %agent_name,
                                        issue = issue_number,
                                        "poll mention dispatch rejected: persona-resolved agent is event-only (push/event-driven), not mention-dispatchable"
                                    );
                                    cursor.dispatches_this_tick += 1;
                                    continue;
                                }
                                // Dedup: check Gitea assignment + active_agents before spawning
                                if self.should_skip_dispatch(&agent_name, issue_number).await {
                                    cursor.dispatches_this_tick += 1;
                                    continue;
                                }

                                let (chain_id, depth, parent_agent) = self.resolve_mention_chain(
                                    &comment.user.login,
                                    &agent_names,
                                    max_mention_depth,
                                );

                                if let Err(e) = mention_chain::MentionChainTracker::check(
                                    depth,
                                    &parent_agent,
                                    &agent_name,
                                    max_mention_depth,
                                ) {
                                    warn!(
                                        agent = %agent_name,
                                        chain_id = %chain_id,
                                        depth,
                                        error = %e,
                                        "mention chain check rejected persona dispatch"
                                    );
                                    if let Some(ref poster) = self.output_poster {
                                        let body = format!(
                                            "## Mention Dispatch Blocked\n\n\
                                            Agent `{}` (via persona) was not spawned: {}.\n\n\
                                            _Chain `{}` at depth {} exceeds the configured limit._",
                                            agent_name, e, chain_id, depth
                                        );
                                        if let Err(pe) = poster
                                            .post_raw_for_project(project_id, issue_number, &body)
                                            .await
                                        {
                                            warn!(error = %pe, "failed to post chain rejection comment");
                                        }
                                    }
                                    cursor.dispatches_this_tick += 1;
                                    continue;
                                }

                                let ctx_args = mention_chain::MentionContextArgs {
                                    parent_agent: parent_agent.clone(),
                                    issue_number,
                                    comment_body: context.clone(),
                                    depth,
                                    chain_id: chain_id.clone(),
                                    available_agents: agent_names
                                        .iter()
                                        .filter(|n| *n != &agent_name)
                                        .cloned()
                                        .collect(),
                                };
                                let chain_ctx = mention_chain::MentionChainTracker::build_context(
                                    &ctx_args,
                                    max_mention_depth,
                                );

                                let mut mention_def = def.clone();
                                mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                                mention_def.gitea_issue = Some(issue_number);

                                if let Err(e) = self.spawn_agent(&mention_def).await {
                                    tracing::error!(agent = %agent_name, issue = issue_number, error = %e, "failed to spawn agent");
                                } else if let Some(agent) =
                                    self.active_agents.get_mut(&mention_def.name)
                                {
                                    agent.spawned_by_mention = true;
                                    agent.mention_chain_id = Some(chain_id);
                                    agent.mention_depth = Some(depth);
                                    agent.mention_parent_agent = if parent_agent.is_empty() {
                                        None
                                    } else {
                                        Some(parent_agent)
                                    };
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
        cursor.save(project_id).await;
        self.mention_cursors.insert(project_id.to_string(), cursor);
    }

    /// Poll every project with a Gitea config for open PRs, parse the latest
    /// structural-pr-review comment, and enqueue [`dispatcher::DispatchTask::AutoMerge`]
    /// for any PR that clears every gate in
    /// [`pr_review::AutoMergeCriteria::default`].
    ///
    /// Called once per reconcile tick after the
    /// dispatcher has been drained so AutoMerge tasks enqueued here are
    /// serviced on the next tick (deterministic ordering). The method is a
    /// no-op when no project has a `gitea` config.
    ///
    /// This is ROC v1 Step F — it enqueues auto-merge but does **not**
    /// actually merge the PR; that lands in Step G. Dedupe is process-local
    /// via [`pr_poller::AutoMergeDedupeSet`]; durable tracking is Step I.
    pub async fn poll_pending_reviews(&mut self) -> Result<(), OrchestratorError> {
        // Build the list of (project_id, gitea_cfg) targets. Mirrors the
        // legacy/multi-project split used by [`Self::poll_mentions`] so the
        // two pollers stay aligned as the config surface evolves.
        let targets: Vec<(String, config::GiteaOutputConfig)> = if self.config.projects.is_empty() {
            match self.config.gitea.clone() {
                Some(g) => vec![(dispatcher::LEGACY_PROJECT_ID.to_string(), g)],
                None => {
                    tracing::debug!(
                        "verdict polling skipped: legacy mode with no top-level gitea config"
                    );
                    return Ok(());
                }
            }
        } else {
            self.config
                .projects
                .iter()
                .filter_map(|project| {
                    let gitea = project.gitea.clone()?;
                    Some((project.id.clone(), gitea))
                })
                .collect()
        };

        if targets.is_empty() {
            tracing::debug!("verdict polling skipped: no projects with Gitea config");
            return Ok(());
        }

        let criteria = pr_review::AutoMergeCriteria::default();

        for (project_id, gitea_cfg) in targets {
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
                Ok(t) => pr_poller::GiteaPrTracker::new(t),
                Err(e) => {
                    tracing::warn!(
                        project = %project_id,
                        error = %e,
                        "failed to create GiteaTracker for verdict polling"
                    );
                    continue;
                }
            };

            self.poll_pending_reviews_for_project(&project_id, &tracker, &criteria)
                .await;
        }

        Ok(())
    }

    /// Inner per-project verdict poll. Accepts a generic [`pr_poller::PrTracker`]
    /// so integration tests can drive it with an in-memory tracker.
    pub async fn poll_pending_reviews_for_project<T: pr_poller::PrTracker + ?Sized>(
        &mut self,
        project_id: &str,
        tracker: &T,
        criteria: &pr_review::AutoMergeCriteria,
    ) {
        let prs = match tracker.list_open_prs().await {
            Ok(prs) => prs,
            Err(e) => {
                tracing::warn!(
                    project = %project_id,
                    error = %e,
                    "failed to list open PRs"
                );
                return;
            }
        };

        let now = std::time::Instant::now();
        for pr in prs {
            if !self.pr_poll_rate_limiter.allow(project_id, pr.number, now) {
                tracing::trace!(
                    project = %project_id,
                    pr = pr.number,
                    "skipping PR: poll rate limited"
                );
                continue;
            }

            let comments = match tracker.fetch_pr_comments(pr.number).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        error = %e,
                        "failed to fetch PR comments"
                    );
                    continue;
                }
            };

            let outcome = pr_poller::evaluate_pr_verdict(&pr, &comments, criteria);

            // Emit PrReviewed for any outcome that resolved a parsed verdict.
            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let has_verdict = matches!(
                    outcome,
                    pr_poller::EvaluationOutcome::Merge { .. }
                        | pr_poller::EvaluationOutcome::HumanReviewNeeded { .. }
                );
                if has_verdict {
                    if let Some(rc) = pr_poller::latest_reviewer_comment(&comments) {
                        if let Ok(v) = pr_review::parse_verdict(&rc.body, rc.id) {
                            let verdict_str = match &outcome {
                                pr_poller::EvaluationOutcome::Merge { .. } => "GO",
                                _ if v.p0_count > 0 => "NO-GO",
                                _ => "CONDITIONAL",
                            };
                            let event = quickwit::OrchestratorEvent::PrReviewed {
                                pr_number: pr.number,
                                project: project_id.to_string(),
                                head_sha: pr.head_sha.clone(),
                                reviewer_login: rc.user_login.clone(),
                                confidence: v.confidence,
                                p0_count: v.p0_count,
                                p1_count: v.p1_count,
                                verdict: verdict_str.to_string(),
                            };
                            let _ = sink.emit_event(project_id, event).await;
                        }
                    }
                }
            }

            match outcome {
                pr_poller::EvaluationOutcome::Merge { head_sha } => {
                    if !self
                        .auto_merge_enqueued
                        .record_if_new(project_id, pr.number, &head_sha)
                    {
                        tracing::debug!(
                            project = %project_id,
                            pr = pr.number,
                            head = %head_sha,
                            "auto-merge already enqueued for this revision"
                        );
                        continue;
                    }
                    tracing::info!(
                        project = %project_id,
                        pr = pr.number,
                        head = %head_sha,
                        "enqueuing AutoMerge for PR that cleared every gate"
                    );
                    self.dispatcher.enqueue(DispatchTask::AutoMerge {
                        pr_number: pr.number,
                        project: project_id.to_string(),
                        head_sha,
                    });
                }
                pr_poller::EvaluationOutcome::HumanReviewNeeded { reason } => {
                    tracing::info!(
                        project = %project_id,
                        pr = pr.number,
                        reason = %reason,
                        "PR requires human review"
                    );
                }
                pr_poller::EvaluationOutcome::NoReviewerComment => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        "no pr-reviewer comment yet; skipping"
                    );
                }
                pr_poller::EvaluationOutcome::ParseError { reason } => {
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        reason = %reason,
                        "reviewer comment failed to parse; skipping"
                    );
                }
            }
        }
    }

    /// Execute a [`DispatchTask::AutoMerge`] task — ROC v1 Step G.
    ///
    /// Builds the per-project [`pr_poller::GiteaPrTracker`] from config and
    /// delegates to [`AgentOrchestrator::handle_auto_merge_for_project`].
    /// The task's `project` field must match a configured project with a
    /// `gitea` block (or, for legacy configs, the top-level `gitea`);
    /// otherwise the call logs-and-skips so the dispatcher keeps draining.
    pub async fn handle_auto_merge(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let (pr_number, project, head_sha) = match &task {
            dispatcher::DispatchTask::AutoMerge {
                pr_number,
                project,
                head_sha,
            } => (*pr_number, project.clone(), head_sha.clone()),
            other => {
                warn!(task = ?other, "handle_auto_merge invoked with non-AutoMerge task; ignoring");
                return Ok(());
            }
        };

        // Resolve the Gitea config for this project. Mirrors the legacy /
        // multi-project split used by `poll_pending_reviews`.
        let gitea_cfg: config::GiteaOutputConfig = if self.config.projects.is_empty() {
            match self.config.gitea.clone() {
                Some(g) if project == dispatcher::LEGACY_PROJECT_ID => g,
                Some(_) => {
                    warn!(
                        pr_number,
                        project = %project,
                        "AutoMerge skipped: legacy mode but task project id does not match LEGACY_PROJECT_ID"
                    );
                    return Ok(());
                }
                None => {
                    warn!(
                        pr_number,
                        project = %project,
                        "AutoMerge skipped: legacy mode with no top-level gitea config"
                    );
                    return Ok(());
                }
            }
        } else {
            match self
                .config
                .projects
                .iter()
                .find(|p| p.id == project)
                .and_then(|p| p.gitea.clone())
            {
                Some(g) => g,
                None => {
                    warn!(
                        pr_number,
                        project = %project,
                        "AutoMerge skipped: project has no gitea config"
                    );
                    return Ok(());
                }
            }
        };

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
            Ok(t) => pr_poller::GiteaPrTracker::new(t),
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    error = %e,
                    "AutoMerge skipped: failed to create GiteaTracker"
                );
                return Ok(());
            }
        };

        self.handle_auto_merge_for_project(task, &tracker).await
    }

    /// Inner AutoMerge executor. Accepts any [`pr_poller::AutoMergeExecutor`]
    /// so integration tests can drive the full handler with an in-memory
    /// tracker. Real production code funnels through
    /// [`AgentOrchestrator::handle_auto_merge`].
    ///
    /// Steps:
    /// 1. Defensive re-check: list open PRs on the project. Skip when the
    ///    PR is absent (already closed/merged) or the HEAD SHA has moved.
    /// 2. Attempt the merge.
    /// 3. On success — enqueue [`DispatchTask::PostMergeTestGate`], record
    ///    the `(pr, head_sha)` in the dedupe set so late polls never
    ///    re-enqueue the same revision.
    /// 4. On failure — open an `[ADF]` tracking issue with the failure
    ///    reason via [`pr_poller::AutoMergeExecutor::open_failure_issue`];
    ///    do **not** enqueue a post-merge gate.
    pub async fn handle_auto_merge_for_project<T: pr_poller::AutoMergeExecutor + ?Sized>(
        &mut self,
        task: dispatcher::DispatchTask,
        tracker: &T,
    ) -> Result<(), OrchestratorError> {
        let (pr_number, project, head_sha) = match task {
            dispatcher::DispatchTask::AutoMerge {
                pr_number,
                project,
                head_sha,
            } => (pr_number, project, head_sha),
            other => {
                warn!(task = ?other, "handle_auto_merge_for_project invoked with non-AutoMerge task; ignoring");
                return Ok(());
            }
        };

        // 1. Defensive re-check: ensure the PR is still open and the HEAD
        // SHA matches what the verdict was computed against. If either has
        // moved, the merge decision is stale — skip silently.
        let open_prs = match tracker.list_open_prs().await {
            Ok(prs) => prs,
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    error = %e,
                    "AutoMerge skipped: failed to list open PRs for head_sha re-check"
                );
                return Ok(());
            }
        };

        let live = match open_prs.iter().find(|p| p.number == pr_number) {
            Some(p) => p,
            None => {
                info!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    "AutoMerge skipped: PR no longer in open list (closed/merged already)"
                );
                return Ok(());
            }
        };
        if live.head_sha != head_sha {
            info!(
                pr_number,
                project = %project,
                expected_head = %head_sha,
                live_head = %live.head_sha,
                "AutoMerge skipped: PR HEAD SHA moved since verdict (stale auto-merge decision)"
            );
            return Ok(());
        }

        // 2. Merge.
        match tracker.merge_pr(pr_number).await {
            Ok(outcome) => {
                info!(
                    pr_number,
                    project = %project,
                    merge_sha = %outcome.merge_commit_sha,
                    "pr_auto_merged"
                );

                #[cfg(feature = "quickwit")]
                if let Some(ref sink) = self.quickwit_sink {
                    let event = quickwit::OrchestratorEvent::PrAutoMerged {
                        pr_number,
                        project: project.clone(),
                        merge_sha: outcome.merge_commit_sha.clone(),
                        title: outcome.title.clone(),
                    };
                    let _ = sink.emit_event(&project, event).await;
                }

                // 3a. Defensive dedupe write — covers AutoMerge tasks that
                // reached the handler by a path other than the poller
                // (webhook, manual enqueue, etc.). `record_if_new` is a
                // no-op when the entry already exists.
                let _ = self
                    .auto_merge_enqueued
                    .record_if_new(&project, pr_number, &head_sha);

                // 3b. Enqueue the post-merge test gate (Step H stub).
                self.dispatcher
                    .enqueue(dispatcher::DispatchTask::PostMergeTestGate {
                        pr_number,
                        project: project.clone(),
                        merge_sha: outcome.merge_commit_sha,
                        title: outcome.title,
                    });

                Ok(())
            }
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    error = %e,
                    "pr_auto_merge_failed"
                );

                // 4. Open an [ADF] tracking issue with the failure reason,
                //    unless we already created one for this (project, pr, sha)
                //    within the TTL window.
                if self
                    .auto_merge_failure_dedupe
                    .is_recent(&project, pr_number, &head_sha)
                {
                    info!(
                        pr_number,
                        project = %project,
                        head = %head_sha,
                        "AutoMerge failure issue already exists for this PR/SHA; skipping duplicate"
                    );
                } else {
                    let title = format!("[ADF] Auto-merge failed for PR #{pr_number}");
                    let body = format!(
                        "AutoMerge handler failed to merge PR #{pr_number} on project `{project}`.\n\n\
                         Head SHA: `{head_sha}`\n\n\
                         Error: {e}\n\n\
                         The PR was left open; a human needs to investigate (merge conflict, \
                         protected branch, permissions, transient API failure).\n\n\
                         Refs: ROC v1 Step G handler, adf-fleet#35."
                    );
                    let labels = ["adf", "auto-merge-failed", "status/needs-triage"];
                    self.auto_merge_failure_dedupe
                        .record(&project, pr_number, &head_sha);
                    match tracker.open_failure_issue(&title, &body, &labels).await {
                        Ok(_issue_number) => {}
                        Err(issue_err) => {
                            warn!(
                                pr_number,
                                project = %project,
                                error = %issue_err,
                                "AutoMerge failure issue creation also failed; nothing to retry automatically"
                            );
                        }
                    }
                }

                Ok(())
            }
        }
    }

    /// Execute a [`DispatchTask::PostMergeTestGate`] task — ROC v1 Step H.
    ///
    /// Defers the heavy lifting to [`post_merge_gate::run_workspace_tests`]
    /// and [`post_merge_gate::revert_merge`] so those helpers stay fully
    /// testable without orchestrator state. This method resolves the
    /// project's `working_dir` as `repo_root`, constructs the [`post_merge_gate::GateConfig`]
    /// (picking up any overrides from `[post_merge_gate]` in
    /// orchestrator.toml), and funnels the result through the inner
    /// `handle_post_merge_test_gate_for_project` helper which takes a
    /// [`post_merge_gate::CommandRunner`] so integration tests can drive the
    /// full handler with a scripted runner.
    pub async fn handle_post_merge_test_gate(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let runner = post_merge_gate::TokioCommandRunner;
        self.handle_post_merge_test_gate_with_runner(task, &runner)
            .await
    }

    /// Inner handler that accepts any [`post_merge_gate::CommandRunner`].
    /// Integration tests use a [`post_merge_gate::ScriptedRunner`] here to
    /// assert on the exact `cargo test` / `git revert` / `git push` call
    /// sequence without spawning real processes.
    ///
    /// On green: logs `post_merge_gate_verified` at info.
    /// On red: classifies the failure, runs `git revert`, pushes to the
    /// configured remote, opens an `[ADF] post-merge test gate reverted`
    /// tracking issue on the project's Gitea repo, and logs
    /// `post_merge_gate_reverted` at warn. Returns `Ok(())` in every
    /// case the dispatcher should continue draining — only hard I/O
    /// errors that prevent even the attempt return `Err`.
    pub async fn handle_post_merge_test_gate_with_runner<R>(
        &mut self,
        task: dispatcher::DispatchTask,
        runner: &R,
    ) -> Result<(), OrchestratorError>
    where
        R: post_merge_gate::CommandRunner + ?Sized,
    {
        let (pr_number, project, merge_sha, title) = match task {
            dispatcher::DispatchTask::PostMergeTestGate {
                pr_number,
                project,
                merge_sha,
                title,
            } => (pr_number, project, merge_sha, title),
            other => {
                warn!(task = ?other, "handle_post_merge_test_gate invoked with non-PostMergeTestGate task; ignoring");
                return Ok(());
            }
        };

        // Resolve repo_root + gitea tracking target for this project.
        // Legacy mode uses the top-level `working_dir` and `gitea`.
        let (repo_root, gitea_cfg) = if self.config.projects.is_empty() {
            if project != dispatcher::LEGACY_PROJECT_ID {
                warn!(
                    pr_number,
                    project = %project,
                    "PostMergeTestGate skipped: legacy mode but task project id does not match LEGACY_PROJECT_ID"
                );
                return Ok(());
            }
            (self.config.working_dir.clone(), self.config.gitea.clone())
        } else {
            match self.config.projects.iter().find(|p| p.id == project) {
                Some(p) => (p.working_dir.clone(), p.gitea.clone()),
                None => {
                    warn!(
                        pr_number,
                        project = %project,
                        "PostMergeTestGate skipped: no project entry for id"
                    );
                    return Ok(());
                }
            }
        };

        // Build GateConfig from orchestrator overrides (if any).
        let gate_override = self.config.post_merge_gate.clone().unwrap_or_default();
        let cfg = post_merge_gate::GateConfig {
            repo_root,
            merge_sha: merge_sha.clone(),
            max_test_duration: std::time::Duration::from_secs(gate_override.max_test_duration_secs),
            revert_push_remote: gate_override.revert_push_remote,
            revert_push_branch: gate_override.revert_push_branch,
        };

        info!(
            pr_number,
            project = %project,
            merge_sha = %merge_sha,
            max_test_duration_secs = cfg.max_test_duration.as_secs(),
            title = %title,
            "post_merge_gate_start"
        );

        let outcome = match post_merge_gate::run_workspace_tests(runner, &cfg).await {
            Ok(o) => o,
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    merge_sha = %merge_sha,
                    error = %e,
                    "post_merge_gate: run_workspace_tests failed before producing an outcome"
                );
                return Ok(());
            }
        };

        if outcome.passed {
            info!(
                pr_number,
                project = %project,
                merge_sha = %merge_sha,
                wall_time_secs = outcome.wall_time.as_secs_f64(),
                "post_merge_gate_verified"
            );
            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let event = quickwit::OrchestratorEvent::PrAutoMergedVerified {
                    pr_number,
                    project: project.clone(),
                    merge_sha: merge_sha.clone(),
                    wall_time_secs: outcome.wall_time.as_secs_f64(),
                };
                let _ = sink.emit_event(&project, event).await;
            }
            return Ok(());
        }

        let classification = post_merge_gate::classify_failure(&outcome);
        warn!(
            pr_number,
            project = %project,
            merge_sha = %merge_sha,
            kind = ?classification.kind,
            failing_tests = ?classification.failing_tests,
            wall_time_secs = outcome.wall_time.as_secs_f64(),
            "post_merge_gate_failed"
        );

        let revert = match post_merge_gate::revert_merge(runner, &cfg).await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    merge_sha = %merge_sha,
                    error = %e,
                    "post_merge_gate: revert_merge failed — manual intervention required"
                );
                // Still try to file the tracking issue below so a human notices.
                post_merge_gate::RevertOutcome {
                    revert_sha: String::new(),
                    pushed: false,
                }
            }
        };

        warn!(
            pr_number,
            project = %project,
            merge_sha = %merge_sha,
            revert_sha = %revert.revert_sha,
            pushed = revert.pushed,
            reason = ?classification.kind,
            "post_merge_gate_reverted"
        );
        #[cfg(feature = "quickwit")]
        if let Some(ref sink) = self.quickwit_sink {
            let event = quickwit::OrchestratorEvent::PrAutoReverted {
                pr_number,
                project: project.clone(),
                merge_sha: merge_sha.clone(),
                revert_sha: revert.revert_sha.clone(),
                reason: format!("{:?}", classification.kind),
                stderr_tail_bytes: outcome.stderr_tail.len() as u32,
            };
            let _ = sink.emit_event(&project, event).await;
        }

        // Open an [ADF] tracking issue. Best-effort — a failure here is
        // logged but does not propagate: the revert has already landed.
        if let Some(gitea) = gitea_cfg {
            let issue_title =
                format!("[ADF] post-merge test gate reverted PR #{pr_number}: {title}");
            let stderr_excerpt = truncate_for_issue(&outcome.stderr_tail, 4000);
            let failing_list = if classification.failing_tests.is_empty() {
                "(none parsed)".to_string()
            } else {
                classification
                    .failing_tests
                    .iter()
                    .map(|t| format!("- `{t}`"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let body = format!(
                "Auto-merged PR #{pr_number} on project `{project}` failed the post-merge test gate.\n\n\
                 Merge SHA: `{merge_sha}`\n\
                 Revert SHA: `{}`\n\
                 Revert pushed: {}\n\
                 Failure kind: `{:?}`\n\
                 Wall time: {:.1}s\n\n\
                 Failing tests:\n\n{failing_list}\n\n\
                 stderr tail (truncated):\n\n```\n{stderr_excerpt}\n```\n\n\
                 Refs: ROC v1 Step H, adf-fleet#36.",
                revert.revert_sha,
                revert.pushed,
                classification.kind,
                outcome.wall_time.as_secs_f64(),
            );
            let labels = ["adf", "post-merge-gate", "status/needs-triage"];
            let tracker_cfg = terraphim_tracker::GiteaConfig {
                base_url: gitea.base_url.clone(),
                token: gitea.token.clone(),
                owner: gitea.owner.clone(),
                repo: gitea.repo.clone(),
                active_states: vec!["open".to_string()],
                terminal_states: vec!["closed".to_string()],
                use_robot_api: false,
                robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
                claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
            };
            match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
                Ok(tracker) => {
                    if let Err(e) = tracker.create_issue(&issue_title, &body, &labels).await {
                        warn!(
                            pr_number,
                            project = %project,
                            error = %e,
                            "post_merge_gate: failed to open [ADF] tracking issue"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        pr_number,
                        project = %project,
                        error = %e,
                        "post_merge_gate: failed to construct tracker for [ADF] issue"
                    );
                }
            }
        } else {
            warn!(
                pr_number,
                project = %project,
                "post_merge_gate: no gitea config for project; skipping [ADF] issue creation"
            );
        }

        Ok(())
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

    /// Resolve mention chain metadata from the comment author.
    ///
    /// If the comment was posted by a known agent, this is a nested mention:
    /// inherit the chain_id from the agent's current run and increment depth.
    /// If posted by a human, start a fresh chain with depth 0.
    fn resolve_mention_chain(
        &self,
        comment_author: &str,
        agent_names: &[String],
        _max_depth: u32,
    ) -> (String, u32, String) {
        if agent_names.iter().any(|n| n == comment_author) {
            if let Some(active) = self.active_agents.get(comment_author) {
                let parent_chain_id = active
                    .mention_chain_id
                    .clone()
                    .unwrap_or_else(|| ulid::Ulid::new().to_string());
                let parent_depth = active.mention_depth.unwrap_or(0).saturating_add(1);
                (parent_chain_id, parent_depth, comment_author.to_string())
            } else {
                let chain_id = ulid::Ulid::new().to_string();
                (chain_id, 1, comment_author.to_string())
            }
        } else {
            let chain_id = ulid::Ulid::new().to_string();
            (chain_id, 0, String::new())
        }
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
    /// `repo_dir` is the git repository root where `git worktree add` runs.
    /// For project-bound agents this is the project's working_dir; otherwise
    /// it is the orchestrator's top-level working_dir.
    ///
    /// Returns the worktree path if successful. Mutating agents fail closed when
    /// git cannot create an isolated worktree; they must not use the shared checkout.
    async fn create_agent_worktree(
        &self,
        agent_name: &str,
        repo_dir: &Path,
    ) -> Result<PathBuf, OrchestratorError> {
        let worktree_root = repo_dir.join(".worktrees");
        if let Err(e) = tokio::fs::create_dir_all(&worktree_root).await {
            return Err(OrchestratorError::WorktreeCreationFailed {
                agent: agent_name.to_string(),
                repo: repo_dir.display().to_string(),
                reason: format!("failed to create worktree root: {e}"),
            });
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
            .current_dir(repo_dir)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    repo = %repo_dir.display(),
                    "created isolated git worktree"
                );
                Ok(worktree_path)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                Err(OrchestratorError::WorktreeCreationFailed {
                    agent: agent_name.to_string(),
                    repo: repo_dir.display().to_string(),
                    reason: stderr.chars().take(500).collect::<String>(),
                })
            }
            Err(e) => Err(OrchestratorError::WorktreeCreationFailed {
                agent: agent_name.to_string(),
                repo: repo_dir.display().to_string(),
                reason: format!("git worktree command failed: {e}"),
            }),
        }
    }

    /// Remove a git worktree after an agent finishes.
    async fn remove_agent_worktree(&self, agent_name: &str, worktree_path: &Path, repo_dir: &Path) {
        // Force-remove even if there are uncommitted changes (they were already
        // committed by try_commit_agent_work or are intentionally discarded).
        let output = tokio::process::Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                &worktree_path.to_string_lossy(),
            ])
            .current_dir(repo_dir)
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

    /// PR gate reconciliation: for every project with Gitea config, read
    /// actual commit statuses and branch protection rules, classify each
    /// open PR head via [`pr_gate::reconcile_pr_gate`], and take action.
    ///
    /// Actions:
    /// - `ReadyForPolicy`: no action (Step 18 will handle it).
    /// - `EnqueueMissingChecks`: log which agents need dispatching.
    /// - `AwaitingChecks`: log and skip (rechecked next interval).
    /// - `BlockedByFailedChecks`: open deduplicated remediation issue.
    /// - `FactoryFault`: open deduplicated remediation issue with error.
    ///
    /// Remediation issues are deduplicated using [`pr_gate::remediation_key`]
    /// by searching for existing open issues containing the key.
    async fn reconcile_pr_gates(&mut self) -> Result<(), OrchestratorError> {
        let targets: Vec<(String, config::GiteaOutputConfig)> = if self.config.projects.is_empty() {
            match self.config.gitea.clone() {
                Some(g) => vec![(dispatcher::LEGACY_PROJECT_ID.to_string(), g)],
                None => return Ok(()),
            }
        } else {
            self.config
                .projects
                .iter()
                .filter_map(|p| p.gitea.clone().map(|g| (p.id.clone(), g)))
                .collect()
        };

        if targets.is_empty() {
            return Ok(());
        }

        for (project_id, gitea_cfg) in &targets {
            if let Err(e) = self
                .reconcile_pr_gates_for_project(project_id, gitea_cfg)
                .await
            {
                warn!(
                    project = %project_id,
                    error = %e,
                    "reconcile_pr_gates_for_project failed"
                );
            }
        }

        Ok(())
    }

    /// Inner per-project PR gate reconciliation.
    async fn reconcile_pr_gates_for_project(
        &mut self,
        project_id: &str,
        gitea_cfg: &config::GiteaOutputConfig,
    ) -> Result<(), OrchestratorError> {
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
                warn!(project = %project_id, error = %e, "failed to create GiteaTracker for PR gate reconciliation");
                return Ok(());
            }
        };

        let protection = match tracker
            .get_branch_protection(&gitea_cfg.owner, &gitea_cfg.repo, "main")
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    project = %project_id,
                    error = %e,
                    "failed to get branch protection; skipping PR gate reconciliation"
                );
                return Ok(());
            }
        };

        if !protection.enable_status_check || protection.status_check_contexts.is_empty() {
            tracing::debug!(
                project = %project_id,
                "branch protection has no required status checks; skipping"
            );
            return Ok(());
        }

        let required_contexts = protection.status_check_contexts.clone();

        let prs = match tracker.list_open_prs().await {
            Ok(prs) => prs,
            Err(e) => {
                warn!(project = %project_id, error = %e, "failed to list open PRs for gate reconciliation");
                return Ok(());
            }
        };

        for pr in prs {
            if pr.head_sha.is_empty() {
                continue;
            }

            let statuses = match tracker
                .list_commit_statuses(&gitea_cfg.owner, &gitea_cfg.repo, &pr.head_sha)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        project = %project_id,
                        pr = pr.number,
                        sha = %pr.head_sha,
                        error = %e,
                        "failed to list commit statuses"
                    );
                    continue;
                }
            };

            let head_statuses: Vec<pr_gate::CommitStatusSummary> = statuses
                .into_iter()
                .map(|s| pr_gate::CommitStatusSummary {
                    context: s.context,
                    state: pr_gate::CommitStatusState::from_api_str(&s.state),
                    created_at_unix: s.created_at.and_then(|ts| ts.parse::<i64>().ok()),
                })
                .collect();

            let snapshot = pr_gate::PrGateSnapshot {
                pr_number: pr.number,
                head_sha: pr.head_sha.clone(),
                base_branch: pr.base_ref.clone(),
                required_contexts: required_contexts.clone(),
                head_statuses,
                now_unix: chrono::Utc::now().timestamp(),
            };

            let decision = pr_gate::reconcile_pr_gate(&snapshot);

            match &decision {
                pr_gate::PrGateDecision::ReadyForPolicy => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        "PR gate: all required contexts green"
                    );
                }
                pr_gate::PrGateDecision::EnqueueMissingChecks { missing } => {
                    tracing::info!(
                        project = %project_id,
                        pr = pr.number,
                        missing = ?missing,
                        "PR gate: missing required contexts"
                    );
                }
                pr_gate::PrGateDecision::AwaitingChecks { pending } => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        pending = ?pending,
                        "PR gate: awaiting pending checks"
                    );
                }
                pr_gate::PrGateDecision::BlockedByFailedChecks { failed } => {
                    let key =
                        pr_gate::remediation_key(project_id, pr.number, &pr.head_sha, &decision);
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        failed = ?failed,
                        key = %key,
                        "PR gate: blocked by failed checks"
                    );
                    if let Err(e) = self
                        .open_remediation_issue_if_needed(
                            &tracker,
                            project_id,
                            pr.number,
                            &pr.head_sha,
                            &key,
                            &format!(
                                "PR #{} blocked by failed required contexts: {}",
                                pr.number,
                                failed
                                    .iter()
                                    .map(|(ctx, state)| format!("{ctx}={state}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        )
                        .await
                    {
                        warn!(error = %e, "failed to open remediation issue");
                    }
                }
                pr_gate::PrGateDecision::FactoryFault { error } => {
                    let key =
                        pr_gate::remediation_key(project_id, pr.number, &pr.head_sha, &decision);
                    tracing::error!(
                        project = %project_id,
                        pr = pr.number,
                        error = %error,
                        key = %key,
                        "PR gate: factory fault"
                    );
                    if let Err(e) = self
                        .open_remediation_issue_if_needed(
                            &tracker,
                            project_id,
                            pr.number,
                            &pr.head_sha,
                            &key,
                            &format!("PR #{} factory fault: {error}", pr.number),
                        )
                        .await
                    {
                        warn!(error = %e, "failed to open remediation issue");
                    }
                }
            }
        }

        Ok(())
    }

    /// Open a deduplicated remediation issue. Searches for existing open issues
    /// containing the remediation key before creating a new one.
    async fn open_remediation_issue_if_needed(
        &self,
        tracker: &terraphim_tracker::GiteaTracker,
        project_id: &str,
        pr_number: u64,
        head_sha: &str,
        dedup_key: &str,
        body: &str,
    ) -> Result<(), OrchestratorError> {
        let existing = tracker.search_issues_by_title(dedup_key).await;
        match existing {
            Ok(ids) if !ids.is_empty() => {
                tracing::debug!(
                    project = %project_id,
                    key = %dedup_key,
                    existing_count = ids.len(),
                    "remediation issue already exists; skipping"
                );
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    project = %project_id,
                    error = %e,
                    "failed to search for existing remediation issues; creating anyway"
                );
            }
            Ok(_) => {}
        }

        let title = format!("[ADF] PR gate remediation: {dedup_key}");
        let full_body = format!(
            "{body}\n\n\
             Project: `{project_id}`\n\
             PR: #{pr_number}\n\
             Head SHA: `{head_sha}`\n\
             Dedup key: `{dedup_key}`\n\n\
             This issue was auto-created by the PR gate reconciler.\
             It will be auto-closed when the gate clears."
        );
        let labels = ["adf", "pr-gate", "status/needs-triage"];

        tracker
            .create_issue(&title, &full_body, &labels)
            .await
            .map_err(|e| {
                OrchestratorError::Config(format!("failed to create remediation issue: {e}"))
            })?;

        tracing::info!(
            project = %project_id,
            pr = pr_number,
            key = %dedup_key,
            "opened PR gate remediation issue"
        );

        Ok(())
    }

    /// Check cron schedules and spawn due Core agents.
    async fn check_cron_schedules(&mut self) {
        let now = chrono::Utc::now();
        let scheduled = self.scheduler.scheduled_agents();

        // Collect agents that should fire
        let to_spawn: Vec<(AgentDefinition, chrono::DateTime<chrono::Utc>)> = scheduled
            .into_iter()
            .filter(|(def, _schedule)| {
                // Skip quarantined agents
                !self.quarantined_agents.contains(&def.name)
            })
            .filter(|(def, _schedule)| {
                // Skip if already active
                !self.active_agents.contains_key(&def.name)
            })
            .filter_map(|(def, schedule)| {
                // Get the next fire time after last_tick_time
                let next_fire = schedule.after(&self.last_tick_time).next()?;
                // Check if fire time is within window
                if next_fire > now {
                    return None;
                }
                // Skip if agent already fired at this schedule occurrence
                if let Some(last_fire) = self.last_cron_fire.get(&def.name) {
                    if next_fire <= *last_fire {
                        return None;
                    }
                }
                Some((def.clone(), next_fire))
            })
            .collect();

        for (def, fire_time) in to_spawn {
            info!(agent = %def.name, fire_time = %fire_time, "cron schedule fired");
            // Record fire time before spawning to prevent rapid re-trigger
            self.last_cron_fire.insert(def.name.clone(), fire_time);
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "cron spawn failed");
            }
        }

        // Also check compound review schedule
        if let Some(compound_sched) = self.scheduler.compound_review_schedule() {
            debug!(
                last_tick = %self.last_tick_time,
                last_fired = ?self.last_compound_review_fired_at,
                now = %now,
                "checking compound review schedule"
            );

            // Compute the earliest occurrence strictly after
            // `last_tick_time` that is also <= now. This is the same
            // occurrence the buggy code would have refired forever when
            // the reconcile-tick future was cancelled mid-await by the
            // 90 s `tokio::time::timeout` safety wrapper (#1562).
            let next_fire = compound_sched
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next();
            debug!(next_fire = ?next_fire, "compound schedule next fire");

            if let Some(fire_time) = next_fire {
                // Gate against re-firing the same occurrence. The
                // cursor `last_compound_review_fired_at` is the per-
                // occurrence dedup key, mirroring `last_cron_fire` for
                // per-agent crons. It is updated *before* the `.await`
                // below so a cancelled future cannot lose the update.
                let already_fired = self
                    .last_compound_review_fired_at
                    .map(|prev| fire_time <= prev)
                    .unwrap_or(false);

                if !already_fired {
                    // Record fire time BEFORE awaiting
                    // `handle_schedule_event` so that future
                    // cancellation cannot lose the update and
                    // re-trigger the same occurrence on the next tick.
                    self.last_compound_review_fired_at = Some(fire_time);
                    info!(
                        fire_time = %fire_time,
                        "compound review schedule fired, starting review"
                    );
                    self.handle_schedule_event(ScheduleEvent::CompoundReview)
                        .await;
                }
            }
        }
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
                if self.active_compound_review.is_some() {
                    info!("compound review already running, skipping");
                    return;
                }
                info!("scheduled compound review starting (background task)");
                // For scheduled reviews, use HEAD against base_branch from config
                let git_ref = "HEAD".to_string();
                let base_ref = self.config.compound_review.base_branch.clone();
                let workflow = self.compound_workflow.clone();
                let handle = tokio::spawn(async move { workflow.run(&git_ref, &base_ref).await });
                self.active_compound_review = Some(handle);
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

    /// Open a `[ADF] unknown error signature on <provider>/<model>` Gitea
    /// issue so fleet-meta can classify the pattern. Deduped by
    /// [`error_signatures::unknown_dedupe_key`] within the process lifetime
    /// so retries of the same stderr shape don't spam the tracker.
    ///
    /// The target tracker is the orchestrator's default [`GiteaTracker`]
    /// from [`OutputPoster::tracker`], which points at the fleet-meta repo
    /// configured in `orchestrator.toml`. If no `OutputPoster` is wired
    /// (tests, legacy configs), this is a no-op.
    async fn escalate_unknown_error(
        &self,
        provider: &str,
        model: Option<&str>,
        stderr_lines: &[String],
    ) {
        let joined = stderr_lines.join("\n");
        let dedupe_key = error_signatures::unknown_dedupe_key(provider, &joined);
        {
            let mut set = match self.unknown_error_dedupe.lock() {
                Ok(g) => g,
                Err(_) => {
                    warn!(
                        provider = %provider,
                        "unknown_error_dedupe lock poisoned; skipping escalation"
                    );
                    return;
                }
            };
            if !set.insert(dedupe_key.clone()) {
                // Already escalated this shape in this process. Skip quietly.
                return;
            }
        }

        let Some(poster) = self.output_poster.as_ref() else {
            info!(
                provider = %provider,
                dedupe_key = %dedupe_key,
                "no output_poster configured; unknown stderr logged only"
            );
            return;
        };

        // Cap stderr in the body so a runaway CLI never posts megabytes.
        const MAX_STDERR_CHARS: usize = 4000;
        let truncated: String = if joined.len() > MAX_STDERR_CHARS {
            format!(
                "{}\n...[truncated, original {} chars]",
                joined.chars().take(MAX_STDERR_CHARS).collect::<String>(),
                joined.len()
            )
        } else {
            joined
        };
        let model_slug = model.unwrap_or("<unknown-model>");
        let title = format!(
            "[ADF] unknown error signature on {}/{}",
            provider, model_slug
        );
        let body = format!(
            "A spawned agent produced stderr that matched neither the \
             throttle nor the flake regex lists for provider `{}` \
             (model `{}`). Please review and extend the provider's \
             `error_signatures` config so future occurrences classify \
             correctly.\n\n\
             **Dedupe key:** `{}`\n\n\
             ## Captured stderr\n\n```\n{}\n```\n",
            provider, model_slug, dedupe_key, truncated
        );
        let labels = ["adf", "error-signature", "triage"];
        let tracker = poster.tracker();
        if let Err(e) = tracker.create_issue(&title, &body, &labels).await {
            warn!(
                provider = %provider,
                model = %model_slug,
                error = %e,
                "failed to escalate unknown-error signature to Gitea"
            );
        } else {
            info!(
                provider = %provider,
                model = %model_slug,
                dedupe_key = %dedupe_key,
                "escalated unknown error signature to fleet-meta"
            );
        }
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

/// Determine whether an agent requires an isolated git worktree.
///
/// Review-tier agents (haiku) and simple local commands used in tests do not
/// need isolation. AI/model-backed agents that may modify code do.
fn requires_isolated_worktree(def: &AgentDefinition, model: Option<&str>) -> bool {
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
