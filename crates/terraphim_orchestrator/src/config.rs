use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Strategy for gating agent spawns.
/// All strategies fail-open: if the check itself fails, the agent spawns anyway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PreCheckStrategy {
    /// Always spawn the agent. No gating.
    Always,
    /// Check git diff between last recorded commit and HEAD.
    /// Only spawn if changed files match watch_paths prefixes.
    GitDiff { watch_paths: Vec<String> },
    /// Query latest comments on a Gitea issue. Skip if PASS verdict
    /// and no new commits since.
    GiteaIssue { issue_number: u64 },
    /// Run an arbitrary shell command via sh -c.
    /// Exit 0 + non-empty stdout = Findings; Exit 0 + empty stdout = NoFindings;
    /// Non-zero exit or timeout = Failed (fail-open).
    Shell {
        script: String,
        #[serde(default = "default_pre_check_timeout")]
        timeout_secs: u64,
    },
}

fn default_pre_check_timeout() -> u64 {
    60
}

const GRACE_PERIOD_MIN_SECS: u64 = 5;
const GRACE_PERIOD_MAX_SECS: u64 = 300;
const MAX_CPU_MIN_SECS: u64 = 60;
const MAX_CPU_MAX_SECS: u64 = 7200;
const PROBE_TTL_MIN_SECS: u64 = 60;

/// Definition of a single project within a multi-project fleet.
///
/// Each project carries its own working directory, Gitea target, mention
/// rate caps, workflow tracker, and Quickwit index. Agents and flows are
/// bound to exactly one project via `project` fields; the orchestrator
/// routes per-project configuration to each agent at dispatch time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project id (e.g. "odilo", "digital-twins", "terraphim").
    pub id: String,
    /// Per-project working directory (overrides top-level working_dir for this project's agents).
    pub working_dir: PathBuf,
    /// Minutes offset added to cron schedules of agents in this project (stagger the fleet).
    #[serde(default)]
    pub schedule_offset_minutes: u8,
    /// Per-project Gitea output config (owner/repo).
    #[serde(default)]
    pub gitea: Option<GiteaOutputConfig>,
    /// Per-project mention config (rate caps).
    #[serde(default)]
    pub mentions: Option<MentionConfig>,
    /// Per-project workflow / tracker config.
    #[serde(default)]
    pub workflow: Option<WorkflowConfig>,
    /// Per-project Quickwit index config.
    #[cfg(feature = "quickwit")]
    #[serde(default)]
    pub quickwit: Option<QuickwitConfig>,
    /// Maximum concurrent agents (time + issue + mention) for this project.
    /// Unset = no per-project cap beyond the global concurrency controller.
    #[serde(default)]
    pub max_concurrent_agents: Option<usize>,
    /// Maximum concurrent mention-driven agents for this project.
    /// Unset = fall back to global mention cap only.
    #[serde(default)]
    pub max_concurrent_mention_agents: Option<usize>,
}

/// Top-level orchestrator configuration (parsed from TOML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Working directory for all agents.
    pub working_dir: PathBuf,
    /// Nightwatch configuration.
    pub nightwatch: NightwatchConfig,
    /// Compound review configuration.
    pub compound_review: CompoundReviewConfig,
    /// Optional workflow configuration for issue-driven mode.
    #[serde(default)]
    pub workflow: Option<WorkflowConfig>,
    /// Agent definitions.
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
    /// Seconds to wait before restarting a Safety agent after it exits.
    #[serde(default = "default_restart_cooldown")]
    pub restart_cooldown_secs: u64,
    /// Maximum number of restarts per Safety agent before giving up.
    #[serde(default = "default_max_restart_count")]
    pub max_restart_count: u32,
    /// Time window for restart budget accounting for Safety agents.
    /// Restart counts older than this window are ignored.
    #[serde(default = "default_restart_budget_window")]
    pub restart_budget_window_secs: u64,
    /// Disk usage percentage threshold (0-100) above which agent spawning is refused.
    /// Set to 100 to disable the guard. Default: 90.
    #[serde(default = "default_disk_usage_threshold")]
    pub disk_usage_threshold: u8,
    /// Reconciliation tick interval in seconds.
    #[serde(default = "default_tick_interval")]
    pub tick_interval_secs: u64,
    /// How many reconcile ticks between PR gate reconciliation runs.
    /// Default: 20 (~10 minutes at 30s tick, ~20 minutes at 60s tick).
    #[serde(default = "default_gate_reconcile_interval_ticks")]
    pub gate_reconcile_interval_ticks: u32,
    /// Default TTL in seconds for handoff buffer entries (None = 86400).
    #[serde(default)]
    pub handoff_buffer_ttl_secs: Option<u64>,
    /// Directory for persona data and configuration files.
    #[serde(default)]
    pub persona_data_dir: Option<PathBuf>,
    /// Directory containing skill definitions (SKILL.md files in subdirectories).
    /// Used to inject skill_chain content into agent prompts.
    #[serde(default)]
    pub skill_data_dir: Option<PathBuf>,
    /// Flow DAG definitions for multi-step workflows.
    #[serde(default)]
    pub flows: Vec<crate::flow::config::FlowDefinition>,
    /// Directory for flow run state persistence.
    #[serde(default)]
    pub flow_state_dir: Option<PathBuf>,
    /// Gitea configuration for posting agent output to issues.
    #[serde(default)]
    pub gitea: Option<GiteaOutputConfig>,
    /// Mention-driven dispatch configuration.
    #[serde(default)]
    pub mentions: Option<MentionConfig>,
    /// Webhook configuration for real-time mention dispatch.
    #[serde(default)]
    pub webhook: Option<WebhookConfig>,
    /// Path to persona role configuration JSON for terraphim-agent.
    #[serde(default)]
    pub role_config_path: Option<PathBuf>,
    /// KG-driven model routing configuration.
    #[serde(default)]
    pub routing: Option<RoutingConfig>,
    /// Quickwit log shipping configuration (only available with quickwit feature).
    #[cfg(feature = "quickwit")]
    #[serde(default)]
    pub quickwit: Option<QuickwitConfig>,
    /// Multi-project definitions. Empty means single-project legacy mode (globals are used).
    #[serde(default)]
    pub projects: Vec<Project>,
    /// Include globs. Each matching file is merged into this config
    /// (appends to projects / agents / flows). Globs are expanded relative
    /// to the base config file's parent directory.
    #[serde(default)]
    pub include: Vec<String>,
    /// Per-provider spend caps for hour / day tumbling UTC windows.
    /// Empty means no provider-level budget gating.
    #[serde(default)]
    pub providers: Vec<crate::provider_budget::ProviderBudgetConfig>,
    /// Optional path for persisting ProviderBudgetTracker state across
    /// restarts. Ignored when `providers` is empty.
    #[serde(default)]
    pub provider_budget_state_file: Option<PathBuf>,
    /// Directory containing per-project pause flag files.
    ///
    /// When a file named `<pause_dir>/<project_id>` exists, the orchestrator
    /// skips dispatching any agent belonging to that project. Operators
    /// remove the file to resume dispatches. The project circuit breaker
    /// creates entries here automatically after repeated `project-meta`
    /// failures.
    ///
    /// Default: [`crate::project_control::DEFAULT_PAUSE_DIR`].
    #[serde(default)]
    pub pause_dir: Option<PathBuf>,
    /// Number of consecutive `project-meta` failures required before the
    /// orchestrator pauses dispatch for the affected project and opens an
    /// `[ADF]` escalation issue.
    ///
    /// Default: [`crate::project_control::DEFAULT_PROJECT_CIRCUIT_BREAKER_THRESHOLD`].
    #[serde(default = "default_project_circuit_breaker_threshold")]
    pub project_circuit_breaker_threshold: u32,
    /// Owner of the Gitea repo where the project circuit breaker opens
    /// escalation issues (e.g. `terraphim`). When `None`, the orchestrator
    /// falls back to [`GiteaOutputConfig::owner`] if configured.
    #[serde(default)]
    pub fleet_escalation_owner: Option<String>,
    /// Repo name for fleet-level escalation issues (e.g. `adf-fleet`). When
    /// `None`, the orchestrator falls back to [`GiteaOutputConfig::repo`].
    #[serde(default)]
    pub fleet_escalation_repo: Option<String>,
    /// Optional post-merge test gate configuration (ROC v1 Step H).
    ///
    /// When omitted the orchestrator uses all defaults — 10 minute test
    /// budget, push revert to `origin main`. Configure explicitly only to
    /// raise the test timeout or suppress the push (self-hosted fleets
    /// without a remote).
    #[serde(default)]
    pub post_merge_gate: Option<PostMergeGateConfig>,
    /// Shared learning system configuration.
    #[serde(default)]
    pub learning: LearningConfig,
    /// Agent evolution system configuration (requires `evolution` feature).
    #[serde(default)]
    pub evolution: EvolutionConfig,
    /// PR-fan-out dispatch configuration (ADF Phase 2, plan §5).
    ///
    /// Lists the agents that should be dispatched on a Gitea
    /// `pull_request.opened` event. When omitted, the orchestrator falls back
    /// to the legacy single-agent behaviour (just `pr-reviewer`) so existing
    /// deployments continue to work without a config edit.
    #[serde(default)]
    pub pr_dispatch: Option<PrDispatchConfig>,
    /// Per-project PR-fan-out tables, aggregated from `[pr_dispatch]` blocks
    /// declared inside `IncludeFragment`s (issue #962). Populated by
    /// `from_file` after include expansion; deliberately not exposed to TOML
    /// (`skip_deserializing`) -- the operator authors per-project blocks
    /// inside their `conf.d/<project>.toml`, not at the top level.
    ///
    /// Lookup precedence in [`OrchestratorConfig::agents_on_pr_open_for_project`]:
    /// 1. this map keyed by project id;
    /// 2. top-level [`OrchestratorConfig::pr_dispatch`] (backward-compat fallback);
    /// 3. [`PrDispatchConfig::legacy_default`].
    #[serde(default, skip_deserializing)]
    pub pr_dispatch_per_project: std::collections::HashMap<String, PrDispatchConfig>,
    /// Gitea skill repository configuration for loading skills from a remote repo.
    #[serde(default)]
    pub gitea_skill_repo: Option<GiteaSkillRepoConfig>,
    /// Direct dispatch configuration for Unix domain socket access by adf-ctl.
    #[serde(default)]
    pub direct_dispatch: Option<DirectDispatchConfig>,
}

/// Configuration for loading skills from a Gitea repository.
///
/// `Debug` is implemented manually so the `token` field is redacted in any
/// log/panic output. Do not derive `Debug` on this struct.
#[derive(Clone, Serialize, Deserialize)]
pub struct GiteaSkillRepoConfig {
    /// Repository URL.
    pub url: String,
    /// Repository owner.
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Git reference (branch, tag, or commit).
    #[serde(default = "default_git_ref")]
    pub git_ref: String,
    /// Local cache directory. Defaults to `$XDG_CACHE_HOME/terraphim/skills`
    /// (or `$HOME/.cache/terraphim/skills`, or `$TMPDIR/terraphim/skills` as
    /// further fallbacks).
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    /// Optional authentication token. Redacted in `Debug` output.
    #[serde(default)]
    pub token: Option<String>,
    /// Fetch timeout in seconds.
    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout_secs: u64,
    /// List of skills to load.
    #[serde(default)]
    pub skills: Vec<String>,
}

impl std::fmt::Debug for GiteaSkillRepoConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GiteaSkillRepoConfig")
            .field("url", &self.url)
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("git_ref", &self.git_ref)
            .field("cache_dir", &self.cache_dir)
            .field("token", &self.token.as_ref().map(|_| "***REDACTED***"))
            .field("fetch_timeout_secs", &self.fetch_timeout_secs)
            .field("skills", &self.skills)
            .finish()
    }
}

fn default_git_ref() -> String {
    "main".to_string()
}

fn default_fetch_timeout() -> u64 {
    30
}

/// Compute the default cache directory for `GiteaSkillRepoConfig::cache_dir`.
///
/// Order of preference:
/// 1. `$XDG_CACHE_HOME/terraphim/skills`
/// 2. `$HOME/.cache/terraphim/skills`
/// 3. `$TMPDIR/terraphim/skills` (last resort - file-system-writable)
fn default_cache_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("terraphim/skills");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".cache/terraphim/skills");
        }
    }
    std::env::temp_dir().join("terraphim/skills")
}

/// PR-fan-out routing table for the `pull_request.opened` event (ADF Phase 2,
/// per `.docs/plan-adf-agents-replace-gitea-actions.md` §5).
///
/// Each entry names an agent (resolved against [`OrchestratorConfig::agents`]
/// for the PR's project) and the Gitea commit-status `context` the orchestrator
/// must POST `pending` for after a successful spawn. Agents that don't resolve
/// or that fail their per-agent subscription / budget gate are skipped silently
/// — no `pending` is posted for them, otherwise an unresolved status would
/// block the PR forever.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrDispatchConfig {
    /// Ordered list of agents to fan out on PR open. An empty list means
    /// "no fan-out" — different from the absent-block default which keeps
    /// the legacy `pr-reviewer`-only behaviour.
    #[serde(default)]
    pub agents_on_pr_open: Vec<PrDispatchEntry>,
}

/// One row in [`PrDispatchConfig::agents_on_pr_open`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrDispatchEntry {
    /// Agent name (matched against `AgentDefinition.name` for the PR project).
    pub name: String,
    /// Gitea commit-status context (e.g. `adf/build`, `adf/pr-reviewer`).
    pub context: String,
}

impl PrDispatchConfig {
    /// Default fan-out used when no `[pr_dispatch]` block is present in the
    /// config — preserves legacy behaviour by dispatching only `pr-reviewer`.
    pub fn legacy_default() -> Self {
        Self {
            agents_on_pr_open: vec![PrDispatchEntry {
                name: "pr-reviewer".to_string(),
                context: "adf/pr-reviewer".to_string(),
            }],
        }
    }
}

/// Configuration for the shared learning system.
///
/// When enabled, the orchestrator injects prior learnings into agent
/// prompts at spawn time and records exit outcomes as validation evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_learning_min_trust")]
    pub min_trust: String,
    #[serde(default = "default_learning_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_learning_max_entries")]
    pub max_entries: usize,
    #[serde(default = "default_learning_archive_days")]
    pub archive_days: u32,
    #[serde(default = "default_learning_consolidation_ticks")]
    pub consolidation_ticks: u64,
}

fn default_learning_min_trust() -> String {
    "L1".to_string()
}

fn default_learning_max_tokens() -> usize {
    1500
}

fn default_learning_max_entries() -> usize {
    10
}

fn default_learning_archive_days() -> u32 {
    30
}

fn default_learning_consolidation_ticks() -> u64 {
    100
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_trust: default_learning_min_trust(),
            max_tokens: default_learning_max_tokens(),
            max_entries: default_learning_max_entries(),
            archive_days: default_learning_archive_days(),
            consolidation_ticks: default_learning_consolidation_ticks(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_evolution_max_memory_tokens")]
    pub max_memory_tokens: usize,
    #[serde(default = "default_evolution_max_snapshots")]
    pub max_snapshots_per_agent: usize,
    #[serde(default = "default_evolution_consolidation_ticks")]
    pub consolidation_interval_ticks: u64,
}

fn default_evolution_max_memory_tokens() -> usize {
    1500
}

fn default_evolution_max_snapshots() -> usize {
    100
}

fn default_evolution_consolidation_ticks() -> u64 {
    200
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_memory_tokens: default_evolution_max_memory_tokens(),
            max_snapshots_per_agent: default_evolution_max_snapshots(),
            consolidation_interval_ticks: default_evolution_consolidation_ticks(),
        }
    }
}

/// Post-merge test gate (ROC v1 Step H) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMergeGateConfig {
    /// Wall-time cap for `cargo test --workspace` in seconds. Defaults to
    /// [`crate::post_merge_gate::DEFAULT_MAX_TEST_DURATION_SECS`] (600).
    #[serde(default = "default_post_merge_gate_max_test_duration_secs")]
    pub max_test_duration_secs: u64,
    /// Git remote to push the revert commit to on red. Defaults to
    /// `Some("origin")`. Set to `None` to suppress the push.
    #[serde(default = "default_post_merge_gate_remote")]
    pub revert_push_remote: Option<String>,
    /// Branch to push the revert to. Defaults to `"main"`.
    #[serde(default = "default_post_merge_gate_branch")]
    pub revert_push_branch: String,
}

fn default_post_merge_gate_max_test_duration_secs() -> u64 {
    crate::post_merge_gate::DEFAULT_MAX_TEST_DURATION_SECS
}

fn default_post_merge_gate_remote() -> Option<String> {
    Some("origin".to_string())
}

fn default_post_merge_gate_branch() -> String {
    "main".to_string()
}

impl Default for PostMergeGateConfig {
    fn default() -> Self {
        Self {
            max_test_duration_secs: default_post_merge_gate_max_test_duration_secs(),
            revert_push_remote: default_post_merge_gate_remote(),
            revert_push_branch: default_post_merge_gate_branch(),
        }
    }
}

/// Configuration for KG-driven model routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Path to directory containing KG routing rule markdown files.
    pub taxonomy_path: PathBuf,
    /// Provider probe TTL in seconds (default: 1800 = 30 minutes).
    #[serde(default = "default_probe_ttl")]
    pub probe_ttl_secs: u64,
    /// Directory for saving probe results JSON (default: ~/.terraphim/benchmark-results).
    #[serde(default)]
    pub probe_results_dir: Option<PathBuf>,
    /// Run provider probes on startup (default: true).
    #[serde(default = "default_true_routing")]
    pub probe_on_startup: bool,
    /// Use RoutingDecisionEngine instead of inline model selection.
    ///
    /// When enabled, `spawn_agent()` delegates model selection to the
    /// control-plane routing engine which combines KG routing, keyword
    /// routing, provider health, budget pressure, and live telemetry
    /// signals (throughput, latency, subscription limits).
    ///
    /// Telemetry data is persisted across restarts and restored on startup.
    ///
    /// Default: `false` (uses inline model selection logic).
    #[serde(default)]
    pub use_routing_engine: bool,
    /// Strategy for selecting the best model when telemetry data is available.
    ///
    /// - `Fastest`: Select the model with lowest average latency.
    /// - `Cheapest`: Select the model with lowest cost per 1K output tokens.
    /// - `FreeThenCheapest`: Select free models first, then cheapest.
    ///
    /// Default: `Fastest`.
    #[serde(default)]
    pub route_selection_strategy: crate::control_plane::RouteSelectionStrategy,
}

fn default_probe_ttl() -> u64 {
    1800
}

fn default_true_routing() -> bool {
    true
}

/// Configuration for posting agent output to Gitea issues.
#[derive(Clone, Serialize, Deserialize)]
pub struct GiteaOutputConfig {
    pub base_url: String,
    /// Gitea API token. Redacted in `Debug` output.
    pub token: String,
    pub owner: String,
    pub repo: String,
    /// Path to JSON file mapping agent names to Gitea API tokens.
    /// When present, agents post comments under their own Gitea user.
    #[serde(default)]
    pub agent_tokens_path: Option<PathBuf>,
}

impl std::fmt::Debug for GiteaOutputConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GiteaOutputConfig")
            .field("base_url", &self.base_url)
            .field("token", &"***REDACTED***")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("agent_tokens_path", &self.agent_tokens_path)
            .finish()
    }
}

/// Configuration for mention-driven dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionConfig {
    /// Poll every N reconciliation ticks (default 2).
    #[serde(default = "default_poll_modulo")]
    pub poll_modulo: u64,
    /// Max mentions to dispatch per poll tick (default 3).
    #[serde(default = "default_max_dispatches_per_tick")]
    pub max_dispatches_per_tick: u32,
    /// Max concurrent mention-spawned agents (default 5).
    #[serde(default = "default_max_concurrent_mention_agents")]
    pub max_concurrent_mention_agents: u32,
    /// Max mention chain nesting depth (default 3).
    /// Depth 0 = direct human mention, depth N = mention of mention.
    /// Set to 0 to disable nested mentions entirely.
    #[serde(default = "default_max_mention_depth")]
    pub max_mention_depth: u32,
}

fn default_poll_modulo() -> u64 {
    2
}

fn default_max_dispatches_per_tick() -> u32 {
    3
}

fn default_max_concurrent_mention_agents() -> u32 {
    5
}

fn default_max_mention_depth() -> u32 {
    crate::mention_chain::DEFAULT_MAX_MENTION_DEPTH
}

impl Default for MentionConfig {
    fn default() -> Self {
        Self {
            poll_modulo: default_poll_modulo(),
            max_dispatches_per_tick: default_max_dispatches_per_tick(),
            max_concurrent_mention_agents: default_max_concurrent_mention_agents(),
            max_mention_depth: default_max_mention_depth(),
        }
    }
}

/// Configuration for the webhook server.
#[derive(Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Bind address for the webhook server (default "127.0.0.1:9090").
    /// Use 127.0.0.1 (localhost) to avoid exposing the webhook endpoint to all network interfaces.
    /// Set to "0.0.0.0:9090" explicitly if external access is required.
    #[serde(default = "default_webhook_bind")]
    pub bind: String,
    /// Shared HMAC secret for webhook signature verification. Redacted in `Debug` output.
    /// Must match the secret configured in Gitea webhook settings.
    pub secret: Option<String>,
}

impl std::fmt::Debug for WebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookConfig")
            .field("bind", &self.bind)
            .field("secret", &self.secret.as_ref().map(|_| "***REDACTED***"))
            .finish()
    }
}

fn default_webhook_bind() -> String {
    "127.0.0.1:9090".to_string()
}

/// Configuration for direct dispatch via Unix domain socket.
///
/// When present, the orchestrator listens on the specified Unix domain socket
/// and accepts JSON dispatch commands from `adf-ctl --local trigger --direct`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDispatchConfig {
    /// Path to the Unix domain socket.  Defaults to `/tmp/adf-ctl.sock`.
    #[serde(default = "DirectDispatchConfig::default_socket_path")]
    pub socket_path: PathBuf,
}

impl Default for DirectDispatchConfig {
    fn default() -> Self {
        Self {
            socket_path: Self::default_socket_path(),
        }
    }
}

impl DirectDispatchConfig {
    fn default_socket_path() -> PathBuf {
        PathBuf::from("/tmp/adf-ctl.sock")
    }
}

/// Quickwit log shipping configuration.
#[cfg(feature = "quickwit")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickwitConfig {
    /// Whether Quickwit logging is enabled.
    #[serde(default = "default_quickwit_enabled")]
    pub enabled: bool,
    /// Quickwit API endpoint.
    #[serde(default = "default_quickwit_endpoint")]
    pub endpoint: String,
    /// Index ID for log ingestion.
    #[serde(default = "default_quickwit_index_id")]
    pub index_id: String,
    /// Maximum documents per batch.
    #[serde(default = "default_quickwit_batch_size")]
    pub batch_size: usize,
    /// Seconds between forced flushes.
    #[serde(default = "default_quickwit_flush_interval_secs")]
    pub flush_interval_secs: u64,
    /// Use Elasticsearch-compatible bulk API instead of native ingest.
    #[serde(default = "default_quickwit_use_es_bulk")]
    pub use_es_bulk: bool,
}

#[cfg(feature = "quickwit")]
impl Default for QuickwitConfig {
    fn default() -> Self {
        Self {
            enabled: default_quickwit_enabled(),
            endpoint: default_quickwit_endpoint(),
            index_id: default_quickwit_index_id(),
            batch_size: default_quickwit_batch_size(),
            flush_interval_secs: default_quickwit_flush_interval_secs(),
            use_es_bulk: default_quickwit_use_es_bulk(),
        }
    }
}

#[cfg(feature = "quickwit")]
fn default_quickwit_enabled() -> bool {
    false
}

#[cfg(feature = "quickwit")]
fn default_quickwit_endpoint() -> String {
    "http://127.0.0.1:7280".to_string()
}

#[cfg(feature = "quickwit")]
fn default_quickwit_index_id() -> String {
    "adf-logs".to_string()
}

#[cfg(feature = "quickwit")]
fn default_quickwit_batch_size() -> usize {
    100
}

#[cfg(feature = "quickwit")]
fn default_quickwit_flush_interval_secs() -> u64 {
    5
}

#[cfg(feature = "quickwit")]
fn default_quickwit_use_es_bulk() -> bool {
    false
}

/// Lightweight reference to an SFIA skill code and level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SfiaSkillRef {
    pub code: String,
    pub level: u8,
}

/// Definition of a single agent in the fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique name (e.g., "security-sentinel").
    pub name: String,
    /// Which layer: Safety, Core, Growth.
    pub layer: AgentLayer,
    /// CLI tool to use (maps to Provider).
    pub cli_tool: String,
    /// Task/prompt to give the agent on spawn.
    pub task: String,
    /// Cron schedule (None = always running for Safety, or on-demand for Growth).
    pub schedule: Option<String>,
    /// Model to use with the CLI tool (e.g., "o3" for codex, "sonnet" for claude).
    pub model: Option<String>,
    /// Default KG tier concept for this agent (e.g., "review_tier" or "review").
    /// When KG routing matches a different tier with confidence below the
    /// escalation threshold, the router falls back to this tier instead.
    #[serde(default)]
    pub default_tier: Option<String>,
    /// Capabilities this agent provides.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Maximum memory in bytes (optional resource limit).
    pub max_memory_bytes: Option<u64>,
    /// Monthly USD budget in cents (e.g., 5000 = $50.00).
    /// None means unlimited (subscription model).
    #[serde(default)]
    pub budget_monthly_cents: Option<u64>,
    /// LLM provider for this agent (e.g., "openai", "anthropic", "openrouter").
    #[serde(default)]
    pub provider: Option<String>,
    /// Persona name for this agent (e.g., "Security Analyst", "Code Reviewer").
    #[serde(default)]
    pub persona: Option<String>,
    /// Terraphim role identifier (e.g., "Terraphim Engineer", "Terraphim Designer").
    #[serde(default)]
    pub terraphim_role: Option<String>,
    /// Chain of skills to invoke for this agent.
    #[serde(default)]
    pub skill_chain: Vec<String>,
    /// SFIA skills with proficiency levels.
    #[serde(default)]
    pub sfia_skills: Vec<SfiaSkillRef>,
    /// Fallback LLM provider if primary fails.
    #[serde(default)]
    pub fallback_provider: Option<String>,
    /// Fallback model if primary fails.
    #[serde(default)]
    pub fallback_model: Option<String>,
    /// Grace period in seconds before killing an unresponsive agent.
    #[serde(default)]
    pub grace_period_secs: Option<u64>,
    /// Maximum CPU seconds allowed per agent execution.
    #[serde(default)]
    pub max_cpu_seconds: Option<u64>,
    /// Optional pre-check strategy to gate agent spawns.
    /// If None, the agent always spawns (equivalent to Always).
    #[serde(default)]
    pub pre_check: Option<PreCheckStrategy>,

    /// Gitea issue number to post output to (optional).
    #[serde(default)]
    pub gitea_issue: Option<u64>,

    /// True if this agent must NOT be dispatched from comment mentions.
    /// Used for event-only agents like `build-runner` that are spawned only
    /// by `handle_push` with `ADF_PUSH_*` context. When true:
    ///   - `handle_webhook_dispatch::SpawnAgent` rejects any mention of this
    ///     agent and logs an explicit rejection.
    ///   - The post-exit OutputPoster hook skips publishing stdout/stderr to
    ///     `gitea_issue` even if it is somehow set, because event-only agents
    ///     post their verdict via a different channel (e.g. the Gitea Commit
    ///     Status API).
    ///
    /// Default `false` keeps existing LLM agents mention-dispatchable.
    #[serde(default)]
    pub event_only: bool,

    /// Project this agent belongs to. Must match a `Project.id` when any
    /// projects are defined. `None` means the agent is global / legacy
    /// single-project mode; mixing per-project and global agents is
    /// rejected at load time.
    #[serde(default)]
    pub project: Option<String>,
    /// Enable evolution tracking for this agent (requires `evolution` feature
    /// and top-level `evolution.enabled = true`). Default: false.
    #[serde(default)]
    pub evolution_enabled: bool,
    /// Enable RLM sandboxed code execution for this agent.
    /// When Some(true), the orchestrator injects RLM session info
    /// into the agent prompt, enabling the agent to request
    /// isolated code execution via terraphim_rlm.
    /// Default: None (disabled).
    #[serde(default)]
    pub rlm_enabled: Option<bool>,

    /// If true, `spawn_agent` honours the explicit `cli_tool` and `model` on
    /// this definition and skips the KG tier-routing override block. Set by
    /// the quota-exit and wall-clock-timeout fallback respawns so the
    /// operator-chosen fallback provider is not overridden by the same
    /// tier-routing rule that selected the now-blocked primary.
    ///
    /// Default `false` preserves existing behaviour for normal (non-fallback)
    /// spawns -- KG tier routing continues to dominate static config.
    #[serde(default)]
    pub bypass_kg_routing: bool,

    /// Whether this agent is enabled. Set to false by the config-error
    /// circuit-breaker after 3 consecutive ConfigError exits. Defaults to true.
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
}

fn default_agent_enabled() -> bool {
    true
}

/// Agent layer in the dark factory hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentLayer {
    /// Always running, auto-restart on failure.
    Safety,
    /// Cron-scheduled or event-triggered.
    Core,
    /// On-demand, spawned when needed.
    Growth,
}

/// Nightwatch drift detection thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightwatchConfig {
    /// How often to evaluate drift (seconds).
    #[serde(default = "default_eval_interval")]
    pub eval_interval_secs: u64,
    /// Drift percentage threshold for Minor correction.
    #[serde(default = "default_minor_threshold")]
    pub minor_threshold: f64,
    /// Drift percentage threshold for Moderate correction.
    #[serde(default = "default_moderate_threshold")]
    pub moderate_threshold: f64,
    /// Drift percentage threshold for Severe correction.
    #[serde(default = "default_severe_threshold")]
    pub severe_threshold: f64,
    /// Drift percentage threshold for Critical correction.
    #[serde(default = "default_critical_threshold")]
    pub critical_threshold: f64,
    /// Hour (0-23) when nightwatch evaluation starts. Default: 0 (midnight).
    #[serde(default)]
    pub active_start_hour: u8,
    /// Hour (0-23) when nightwatch evaluation ends. Default: 24 (always active).
    #[serde(default = "default_active_end_hour")]
    pub active_end_hour: u8,
    /// Weight for error rate in drift calculation (default: 0.35).
    #[serde(default = "default_error_weight")]
    pub error_weight: f64,
    /// Weight for command success rate in drift calculation (default: 0.25).
    #[serde(default = "default_success_weight")]
    pub success_weight: f64,
    /// Weight for health score in drift calculation (default: 0.20).
    #[serde(default = "default_health_weight")]
    pub health_weight: f64,
    /// Weight for budget exhaustion in drift calculation (default: 0.20).
    #[serde(default = "default_budget_weight")]
    pub budget_weight: f64,
}

impl Default for NightwatchConfig {
    fn default() -> Self {
        Self {
            eval_interval_secs: default_eval_interval(),
            minor_threshold: default_minor_threshold(),
            moderate_threshold: default_moderate_threshold(),
            severe_threshold: default_severe_threshold(),
            critical_threshold: default_critical_threshold(),
            active_start_hour: 0,
            active_end_hour: default_active_end_hour(),
            error_weight: default_error_weight(),
            success_weight: default_success_weight(),
            health_weight: default_health_weight(),
            budget_weight: default_budget_weight(),
        }
    }
}

fn default_eval_interval() -> u64 {
    300
}
fn default_minor_threshold() -> f64 {
    0.10
}
fn default_moderate_threshold() -> f64 {
    0.20
}
fn default_severe_threshold() -> f64 {
    0.40
}
fn default_critical_threshold() -> f64 {
    0.70
}
fn default_active_end_hour() -> u8 {
    24
}
fn default_error_weight() -> f64 {
    0.35
}
fn default_success_weight() -> f64 {
    0.25
}
fn default_health_weight() -> f64 {
    0.20
}
fn default_budget_weight() -> f64 {
    0.20
}

/// Compound review settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundReviewConfig {
    /// Cron schedule for compound review (e.g., "0 2 * * *").
    pub schedule: String,
    /// Maximum duration in seconds.
    #[serde(default = "default_max_duration")]
    pub max_duration_secs: u64,
    /// Git repository path.
    pub repo_path: PathBuf,
    /// Whether to create PRs (false = dry run).
    #[serde(default)]
    pub create_prs: bool,
    /// Root directory for worktrees.
    #[serde(default = "default_worktree_root")]
    pub worktree_root: PathBuf,
    /// Base branch for comparison.
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
    /// Maximum number of concurrent agents.
    #[serde(default = "default_max_concurrent_agents")]
    pub max_concurrent_agents: usize,
    /// CLI tool override for compound review agents.
    #[serde(default)]
    pub cli_tool: Option<String>,
    /// LLM provider for compound review agents.
    #[serde(default)]
    pub provider: Option<String>,
    /// Model override for compound review agents.
    #[serde(default)]
    pub model: Option<String>,
    /// Gitea issue number to post compound review summaries.
    #[serde(default)]
    pub gitea_issue: Option<u64>,
    /// Auto-file Gitea issues for CRITICAL and HIGH severity findings.
    #[serde(default)]
    pub auto_file_issues: bool,
    /// Spawn remediation agents for CRITICAL findings.
    #[serde(default)]
    pub auto_remediate: bool,
    /// Map of finding categories to remediation agent names.
    #[serde(default)]
    pub remediation_agents: std::collections::HashMap<String, String>,
}

fn default_max_duration() -> u64 {
    1800
}

fn default_worktree_root() -> PathBuf {
    PathBuf::from(".worktrees")
}

fn default_base_branch() -> String {
    "main".to_string()
}

fn default_max_concurrent_agents() -> usize {
    3
}

impl Default for CompoundReviewConfig {
    fn default() -> Self {
        Self {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: default_max_duration(),
            repo_path: PathBuf::from("."),
            create_prs: false,
            worktree_root: default_worktree_root(),
            base_branch: default_base_branch(),
            max_concurrent_agents: default_max_concurrent_agents(),
            cli_tool: None,
            provider: None,
            model: None,
            gitea_issue: None,
            auto_file_issues: false,
            auto_remediate: false,
            remediation_agents: std::collections::HashMap::new(),
        }
    }
}

/// Workflow configuration for issue-driven mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Whether issue-driven mode is enabled.
    pub enabled: bool,
    /// Poll interval in seconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    /// Path to WORKFLOW.md file.
    pub workflow_file: PathBuf,
    /// Tracker configuration.
    pub tracker: TrackerConfig,
    /// Concurrency configuration.
    #[serde(default)]
    pub concurrency: ConcurrencyConfig,
}

/// Tracker configuration for issue-driven mode.
///
/// `Debug` is implemented manually so the `api_key` field is redacted in any
/// log/panic output. Do not derive `Debug` on this struct.
#[derive(Clone, Serialize, Deserialize)]
pub struct TrackerConfig {
    /// Tracker kind: "gitea" or "linear".
    pub kind: String,
    /// API endpoint URL.
    pub endpoint: String,
    /// API key (supports env var substitution like "${GITEA_TOKEN}"). Redacted in `Debug` output.
    pub api_key: String,
    /// Repository owner (for Gitea).
    pub owner: String,
    /// Repository name (for Gitea).
    pub repo: String,
    /// Project slug for Linear (optional, Linear-specific).
    #[serde(default)]
    pub project_slug: Option<String>,
    /// Whether to use gitea-robot PageRank API.
    #[serde(default)]
    pub use_robot_api: bool,
    /// State configuration.
    #[serde(default)]
    pub states: TrackerStates,
}

impl std::fmt::Debug for TrackerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackerConfig")
            .field("kind", &self.kind)
            .field("endpoint", &self.endpoint)
            .field("api_key", &"***REDACTED***")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("project_slug", &self.project_slug)
            .field("use_robot_api", &self.use_robot_api)
            .field("states", &self.states)
            .finish()
    }
}

/// Tracker state configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerStates {
    /// Active states that trigger agent dispatch.
    #[serde(default = "default_active_states")]
    pub active: Vec<String>,
    /// Terminal states that trigger workspace cleanup.
    #[serde(default = "default_terminal_states")]
    pub terminal: Vec<String>,
}

impl Default for TrackerStates {
    fn default() -> Self {
        Self {
            active: default_active_states(),
            terminal: default_terminal_states(),
        }
    }
}

/// Concurrency configuration for issue-driven mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// Global maximum concurrent agents (both modes combined).
    #[serde(default = "default_global_max")]
    pub global_max: usize,
    /// Maximum issue-driven agents.
    #[serde(default = "default_issue_max")]
    pub issue_max: usize,
    /// Fairness strategy: "round_robin", "priority", "proportional".
    #[serde(default = "default_fairness")]
    pub fairness: String,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            global_max: default_global_max(),
            issue_max: default_issue_max(),
            fairness: default_fairness(),
        }
    }
}

fn default_poll_interval() -> u64 {
    120 // 2 minutes
}

fn default_active_states() -> Vec<String> {
    vec!["Todo".into(), "In Progress".into()]
}

fn default_terminal_states() -> Vec<String> {
    vec!["Done".into(), "Closed".into(), "Cancelled".into()]
}

fn default_global_max() -> usize {
    5
}

fn default_issue_max() -> usize {
    3
}

fn default_fairness() -> String {
    "round_robin".into()
}

fn default_restart_cooldown() -> u64 {
    60
}

fn default_max_restart_count() -> u32 {
    10
}

fn default_restart_budget_window() -> u64 {
    43_200
}

fn default_disk_usage_threshold() -> u8 {
    90
}

fn default_tick_interval() -> u64 {
    30
}

fn default_gate_reconcile_interval_ticks() -> u32 {
    20
}

fn default_project_circuit_breaker_threshold() -> u32 {
    crate::project_control::DEFAULT_PROJECT_CIRCUIT_BREAKER_THRESHOLD
}

/// Partial config parsed from `include`d files. Only project definitions,
/// agents, flows, and the optional per-include `pr_dispatch` block are
/// merged in; all top-level globals are ignored.
///
/// `pr_dispatch` carried inside an include applies to every project
/// declared in the same fragment -- see
/// `.docs/design-pr-dispatch-per-project.md` for the aggregation rule
/// (Gitea issue #962).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct IncludeFragment {
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    agents: Vec<AgentDefinition>,
    #[serde(default)]
    flows: Vec<crate::flow::config::FlowDefinition>,
    #[serde(default)]
    pr_dispatch: Option<PrDispatchConfig>,
}

/// Subscription-only LLM providers allowed by constraint C1.
/// Any `model` or `fallback_model` string with a `/`-prefixed provider
/// name must appear in this list; bare names (`sonnet`, `opus`) are
/// interpreted as claude-code CLI targets and always allowed.
pub const ALLOWED_PROVIDER_PREFIXES: &[&str] = &[
    "claude-code",
    "opencode-go",
    "kimi-for-coding",
    "minimax-coding-plan",
    "zai-coding-plan",
    "openai",
];

/// Explicitly banned provider prefixes. Anything matching these is rejected
/// at load time so a misconfigured fleet never reaches a pay-per-use
/// provider at runtime. Note `minimax/` is banned but the
/// `minimax-coding-plan/` subscription variant remains allowed.
pub const BANNED_PROVIDER_PREFIXES: &[&str] = &[
    "opencode",
    "github-copilot",
    "google",
    "huggingface",
    "minimax",
];

/// Bare model names routed through claude-code CLI (no explicit provider prefix).
pub const CLAUDE_CLI_BARE_MODELS: &[&str] = &["sonnet", "opus", "haiku"];

/// Anthropic-branded bare models that map onto the claude-code CLI.
pub const ANTHROPIC_BARE_PROVIDERS: &[&str] = &["anthropic"];

/// Runtime check: is this model's provider prefix in the allowed subscription
/// set? Returns `true` for known bare names (routed through claude-code CLI
/// or explicitly listed in [`ALLOWED_PROVIDER_PREFIXES`]) and for strings
/// whose `/`-delimited prefix appears in [`ALLOWED_PROVIDER_PREFIXES`].
/// Anthropic-branded bare models also pass.
///
/// This is an explicit allow-list: unknown bare names (including bare banned
/// ids like `"minimax"`) are rejected. Matches prefixes by exact equality --
/// `opencode-go` is allowed, `opencode` is banned, `minimax-coding-plan` is
/// allowed, `minimax` is banned.
///
/// Accepts either the full `provider/model` string (e.g. `kimi-for-coding/k2p5`)
/// or a bare provider id (e.g. `opencode-go`).
pub fn is_allowed_provider(provider_or_model: &str) -> bool {
    // `/`-prefixed form: check the prefix against the explicit allow-list.
    if let Some((prefix, _)) = provider_or_model.split_once('/') {
        if ANTHROPIC_BARE_PROVIDERS.contains(&prefix) {
            return true;
        }
        // Exact prefix match. Banned prefixes take precedence so
        // `minimax/` is rejected even though `minimax-coding-plan/` is allowed.
        if BANNED_PROVIDER_PREFIXES.contains(&prefix) {
            return false;
        }
        return ALLOWED_PROVIDER_PREFIXES.contains(&prefix);
    }

    // Bare name (no slash): explicit allow-list only. Previously this
    // branch fell through to `true` for any unknown id, which meant a
    // bare banned id (e.g. `model = "minimax"`) silently passed. Now
    // every bare name must appear in one of the known-safe lists.
    CLAUDE_CLI_BARE_MODELS.contains(&provider_or_model)
        || ANTHROPIC_BARE_PROVIDERS.contains(&provider_or_model)
        || ALLOWED_PROVIDER_PREFIXES.contains(&provider_or_model)
}

/// Validate that a `model` / `fallback_model` string routes through an
/// allowed subscription provider. Returns `Ok(())` for allowed strings,
/// `Err(OrchestratorError::BannedProvider)` for banned ones.
pub(crate) fn validate_model_provider(
    agent_name: &str,
    field: &str,
    model: &str,
) -> Result<(), crate::error::OrchestratorError> {
    // Bare names like "sonnet", "opus", "haiku" -> claude-code CLI.
    if !model.contains('/') {
        if CLAUDE_CLI_BARE_MODELS.contains(&model)
            || ANTHROPIC_BARE_PROVIDERS.contains(&model)
            || ALLOWED_PROVIDER_PREFIXES.contains(&model)
        {
            return Ok(());
        }
        // Unknown bare name: treat as banned. A bare id like `"minimax"`
        // must not slip through just because the operator omitted the
        // slash; operators have to opt in via an explicit allow-list entry.
        return Err(crate::error::OrchestratorError::BannedProvider {
            agent: agent_name.to_string(),
            provider: model.to_string(),
            field: field.to_string(),
        });
    }

    let prefix = model.split('/').next().unwrap_or("");

    if ANTHROPIC_BARE_PROVIDERS.contains(&prefix) {
        return Ok(());
    }

    for banned in BANNED_PROVIDER_PREFIXES {
        if prefix == *banned {
            return Err(crate::error::OrchestratorError::BannedProvider {
                agent: agent_name.to_string(),
                provider: model.to_string(),
                field: field.to_string(),
            });
        }
    }

    if ALLOWED_PROVIDER_PREFIXES.contains(&prefix) {
        return Ok(());
    }

    // Unknown prefix: treat as banned so the fleet cannot silently route
    // to a provider the operator has not approved.
    Err(crate::error::OrchestratorError::BannedProvider {
        agent: agent_name.to_string(),
        provider: model.to_string(),
        field: field.to_string(),
    })
}

/// Emit a tracing error if `path` is world-readable (Unix mode & 0o004 != 0).
///
/// Sensitive configuration files (orchestrator config, agent tokens) must not
/// be world-readable. Any other system user could read API tokens or webhook
/// secrets. The correct permissions are 0600 (owner read/write only).
///
/// This is advisory: the orchestrator continues loading even if the check fails,
/// so operators can still recover from misconfigured deployments by fixing
/// permissions while the service is running.
pub fn warn_if_world_readable(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(path) {
            Ok(meta) => {
                let mode = meta.permissions().mode();
                if mode & 0o004 != 0 {
                    tracing::error!(
                        path = %path.display(),
                        mode = format!("{:04o}", mode & 0o777),
                        "SECURITY: sensitive file is world-readable. \
                         Fix immediately: chmod 600 {}",
                        path.display()
                    );
                } else if mode & 0o040 != 0 {
                    tracing::warn!(
                        path = %path.display(),
                        mode = format!("{:04o}", mode & 0o777),
                        "SECURITY: sensitive file is group-readable. \
                         Consider: chmod 600 {}",
                        path.display()
                    );
                }
            }
            Err(e) => {
                tracing::debug!(path = %path.display(), error = %e, "could not stat file for permission check");
            }
        }
    }
}

/// Expand `${VAR_NAME}` placeholders in a TOML string using environment variables.
///
/// Variables that are not set in the environment are replaced with an empty string,
/// making secrets optional at parse time (the application can validate afterwards).
/// Only `${UPPER_CASE}` syntax is supported; `$VAR` and `$(cmd)` are left as-is.
pub(crate) fn expand_env_vars(s: &str) -> String {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("valid env-var expansion regex")
    });
    re.replace_all(s, |caps: &regex::Captures| {
        std::env::var(&caps[1]).unwrap_or_default()
    })
    .into_owned()
}

impl OrchestratorConfig {
    /// Find a project definition by id.
    pub fn project_by_id(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// Effective PR-fan-out list for the `pull_request.opened` event,
    /// resolved for a specific project.
    ///
    /// Lookup precedence (issue #962, see
    /// `.docs/design-pr-dispatch-per-project.md`):
    /// 1. `pr_dispatch_per_project[project]` -- per-project block declared
    ///    inside the project's `conf.d/<project>.toml`. First preference,
    ///    needed for multi-project fleets where each project picks its own
    ///    fan-out.
    /// 2. Top-level [`OrchestratorConfig::pr_dispatch`] -- backward-compat
    ///    fallback. Configs that declared `[pr_dispatch]` at the top level
    ///    of `orchestrator.toml` continue to work; that block now means
    ///    "default for any project without its own block".
    /// 3. [`PrDispatchConfig::legacy_default`] -- single `pr-reviewer`
    ///    entry. Preserves pre-Phase-2 behaviour for fleets that never
    ///    declared a `[pr_dispatch]` block at all.
    pub fn agents_on_pr_open_for_project(&self, project: &str) -> Vec<PrDispatchEntry> {
        if let Some(d) = self.pr_dispatch_per_project.get(project) {
            return d.agents_on_pr_open.clone();
        }
        if let Some(d) = self.pr_dispatch.as_ref() {
            return d.agents_on_pr_open.clone();
        }
        PrDispatchConfig::legacy_default().agents_on_pr_open
    }

    /// Resolve the effective working directory for an agent: the project's
    /// `working_dir` if the agent has a `project` and it matches a known
    /// project, else the top-level working_dir.
    pub fn working_dir_for_agent(&self, agent: &AgentDefinition) -> PathBuf {
        agent
            .project
            .as_deref()
            .and_then(|pid| self.project_by_id(pid))
            .map(|p| p.working_dir.clone())
            .unwrap_or_else(|| self.working_dir.clone())
    }

    /// Parse an OrchestratorConfig from a TOML string. Does not expand
    /// `include` globs; use `from_file` when include expansion is needed.
    ///
    /// `${VAR_NAME}` placeholders are expanded from the environment before
    /// parsing, allowing secrets (e.g. `[webhook] secret = "${ADF_WEBHOOK_SECRET}"`)
    /// to be injected at runtime rather than stored in git-tracked files.
    pub fn from_toml(toml_str: &str) -> Result<Self, crate::error::OrchestratorError> {
        let expanded = expand_env_vars(toml_str);
        toml::from_str(&expanded)
            .map_err(|e| crate::error::OrchestratorError::Config(e.to_string()))
    }

    /// Load an OrchestratorConfig from a TOML file, expanding any
    /// `include = [...]` globs relative to the base file's parent dir.
    /// Each include file is parsed as an `IncludeFragment` (projects /
    /// agents / flows only) and appended onto the base config.
    ///
    /// Validation (project id uniqueness, project refs, banned providers,
    /// mixed mode) runs after merging -- use `validate()` to trigger it.
    pub fn from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, crate::error::OrchestratorError> {
        let path = path.as_ref();
        warn_if_world_readable(path);
        let content = std::fs::read_to_string(path)?;
        let mut config = Self::from_toml(&content)?;

        if config.include.is_empty() {
            return Ok(config);
        }

        let base_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let patterns = std::mem::take(&mut config.include);

        for pattern in &patterns {
            let full_pattern = if std::path::Path::new(pattern).is_absolute() {
                pattern.clone()
            } else {
                base_dir.join(pattern).to_string_lossy().into_owned()
            };

            let entries = glob::glob(&full_pattern).map_err(|e| {
                crate::error::OrchestratorError::InvalidIncludeGlob {
                    pattern: pattern.clone(),
                    reason: e.to_string(),
                }
            })?;

            let mut matched: Vec<std::path::PathBuf> = entries
                .filter_map(|r| r.ok())
                .filter(|p| p != path)
                .collect();
            matched.sort();

            for include_path in matched {
                let include_content = std::fs::read_to_string(&include_path)?;
                let fragment: IncludeFragment = toml::from_str(&include_content).map_err(|e| {
                    crate::error::OrchestratorError::Config(format!(
                        "failed to parse include file '{}': {e}",
                        include_path.display()
                    ))
                })?;
                // Aggregate per-include `pr_dispatch` blocks against every
                // project declared in the same fragment (issue #962).
                // A `[pr_dispatch]` block in an include without any
                // `[[projects]]` is orphaned -- log a warn and drop it
                // rather than silently losing the operator's intent.
                if let Some(dispatch) = fragment.pr_dispatch.as_ref() {
                    if fragment.projects.is_empty() {
                        tracing::warn!(
                            include_path = %include_path.display(),
                            "pr_dispatch block in include without projects; ignored"
                        );
                    } else {
                        for project in &fragment.projects {
                            config
                                .pr_dispatch_per_project
                                .insert(project.id.clone(), dispatch.clone());
                        }
                    }
                }
                config.projects.extend(fragment.projects);
                config.agents.extend(fragment.agents);
                config.flows.extend(fragment.flows);
            }
        }

        // Preserve the original include patterns so downstream tools can
        // show what was merged.
        config.include = patterns;

        Ok(config)
    }

    /// Load from a TOML file, expand include globs, and validate.
    ///
    /// Single entry-point for production startup and `adf --check`. Callers
    /// that need a pre-parsed config can call `from_file` + `validate` directly.
    pub fn load_and_validate(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, crate::error::OrchestratorError> {
        let mut cfg = Self::from_file(path)?;
        cfg.substitute_env_vars();
        cfg.validate()?;
        Ok(cfg)
    }

    /// Substitute environment variables in workflow config.
    /// Replaces ${VAR} or $VAR with the value of the environment variable.
    pub fn substitute_env_vars(&mut self) {
        if let Some(ref mut workflow) = self.workflow {
            workflow.tracker.api_key = substitute_env(&workflow.tracker.api_key);
        }
    }

    /// Validate the configuration.
    ///
    /// Runs all load-time checks: workflow requirements, pre-check strategy
    /// dependencies, duplicate project ids, agent/flow project references,
    /// banned LLM providers (C1), and mixed single/multi-project mode.
    pub fn validate(&self) -> Result<(), crate::error::OrchestratorError> {
        // Validate workflow config if present
        if let Some(ref workflow) = self.workflow {
            if workflow.enabled {
                if workflow.tracker.api_key.is_empty() {
                    return Err(crate::error::OrchestratorError::Config(
                        "workflow.tracker.api_key is required when workflow is enabled".into(),
                    ));
                }
                if workflow.tracker.endpoint.is_empty() {
                    return Err(crate::error::OrchestratorError::Config(
                        "workflow.tracker.endpoint is required when workflow is enabled".into(),
                    ));
                }
            }
        }

        // Validate pre-check strategies
        for agent in &self.agents {
            if let Some(PreCheckStrategy::GiteaIssue { .. }) = &agent.pre_check {
                if self.workflow.is_none() {
                    return Err(crate::error::OrchestratorError::PreCheckConfig {
                        agent: agent.name.clone(),
                        reason: "gitea-issue strategy requires [workflow] config section".into(),
                    });
                }
            }
        }

        // Validate project ids are unique.
        let mut seen_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for project in &self.projects {
            if !seen_ids.insert(project.id.as_str()) {
                return Err(crate::error::OrchestratorError::DuplicateProjectId(
                    project.id.clone(),
                ));
            }
        }

        let multi_project = !self.projects.is_empty();

        // Every agent.project (if Some) must reference a known project id.
        // In multi-project mode, every agent must set project; in legacy
        // mode agent.project must be None.
        for agent in &self.agents {
            match (&agent.project, multi_project) {
                (Some(pid), _) => {
                    if !seen_ids.contains(pid.as_str()) {
                        return Err(crate::error::OrchestratorError::UnknownAgentProject {
                            agent: agent.name.clone(),
                            project: pid.clone(),
                        });
                    }
                }
                (None, true) => {
                    return Err(crate::error::OrchestratorError::MixedProjectMode {
                        kind: "agent",
                        name: agent.name.clone(),
                    });
                }
                (None, false) => {}
            }
        }

        // Every flow.project must reference a known project id.
        // Flows are always per-project (D14); if projects is empty but a
        // flow has a project string, treat that as an unresolved reference
        // so operators see a clear error instead of a silent orphan.
        for flow in &self.flows {
            if !multi_project {
                // Empty projects list but flows exist -> mixed mode error.
                return Err(crate::error::OrchestratorError::MixedProjectMode {
                    kind: "flow",
                    name: flow.name.clone(),
                });
            }
            if !seen_ids.contains(flow.project.as_str()) {
                return Err(crate::error::OrchestratorError::UnknownFlowProject {
                    flow: flow.name.clone(),
                    project: flow.project.clone(),
                });
            }
        }

        // C1: banned subscription providers.
        for agent in &self.agents {
            if let Some(model) = &agent.model {
                validate_model_provider(&agent.name, "model", model)?;
            }
            if let Some(model) = &agent.fallback_model {
                validate_model_provider(&agent.name, "fallback_model", model)?;
            }
        }

        // D2: grace_period_secs range validation (5s - 300s)
        for agent in &self.agents {
            if let Some(grace) = agent.grace_period_secs {
                if !(GRACE_PERIOD_MIN_SECS..=GRACE_PERIOD_MAX_SECS).contains(&grace) {
                    return Err(crate::error::OrchestratorError::AgentFieldOutOfRange {
                        agent: agent.name.clone(),
                        field: "grace_period_secs".into(),
                        value: grace,
                        min: GRACE_PERIOD_MIN_SECS,
                        max: GRACE_PERIOD_MAX_SECS,
                    });
                }
            }
        }

        // D3: max_cpu_seconds range validation (60s - 7200s)
        for agent in &self.agents {
            if let Some(cpu) = agent.max_cpu_seconds {
                if !(MAX_CPU_MIN_SECS..=MAX_CPU_MAX_SECS).contains(&cpu) {
                    return Err(crate::error::OrchestratorError::AgentFieldOutOfRange {
                        agent: agent.name.clone(),
                        field: "max_cpu_seconds".into(),
                        value: cpu,
                        min: MAX_CPU_MIN_SECS,
                        max: MAX_CPU_MAX_SECS,
                    });
                }
            }
        }

        // D4: RoutingConfig probe_ttl_secs minimum validation (60s) if routing is enabled
        if let Some(ref routing) = self.routing {
            if routing.probe_ttl_secs < PROBE_TTL_MIN_SECS {
                return Err(crate::error::OrchestratorError::ProbeTtlTooShort {
                    value: routing.probe_ttl_secs,
                    min: PROBE_TTL_MIN_SECS,
                });
            }
        }

        Ok(())
    }
}

/// Substitute environment variables in a string.
/// Supports ${VAR} syntax. Bare $VAR syntax is not implemented.
fn substitute_env(s: &str) -> String {
    let mut result = s.to_string();

    // Handle ${VAR} syntax
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let var_value = std::env::var(var_name).unwrap_or_default();
            result.replace_range(start..start + end + 1, &var_value);
        } else {
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitea_skill_repo_token_redacted_in_debug() {
        let cfg = GiteaSkillRepoConfig {
            url: "https://git.example".to_string(),
            owner: "acme".to_string(),
            repo: "skills".to_string(),
            git_ref: "main".to_string(),
            cache_dir: PathBuf::from("/tmp/skills"),
            token: Some("super-secret-do-not-leak".to_string()),
            fetch_timeout_secs: 30,
            skills: vec![],
        };
        let dbg = format!("{:?}", cfg);
        assert!(
            !dbg.contains("super-secret-do-not-leak"),
            "token must be redacted in Debug output"
        );
        assert!(
            dbg.contains("***REDACTED***"),
            "Debug output should mark token as redacted"
        );
    }

    #[test]
    fn gitea_skill_repo_default_cache_dir_non_empty() {
        let dir = default_cache_dir();
        assert!(!dir.as_os_str().is_empty());
        assert!(dir.ends_with("terraphim/skills"));
    }

    #[test]
    fn test_config_parse_minimal() {
        let toml_str = r#"
working_dir = "/tmp/terraphim"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "codex"
task = "Run tests"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "test-agent");
        assert_eq!(config.agents[0].layer, AgentLayer::Safety);
        assert!(config.agents[0].schedule.is_none());
        assert!(config.agents[0].capabilities.is_empty());
    }

    #[test]
    fn test_config_parse_full() {
        let toml_str = r#"
working_dir = "/Users/alex/projects/terraphim/terraphim-ai"

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"
max_duration_secs = 1800
repo_path = "/Users/alex/projects/terraphim/terraphim-ai"
create_prs = false

[[agents]]
name = "security-sentinel"
layer = "Safety"
cli_tool = "codex"
task = "Scan for CVEs"
capabilities = ["security", "vulnerability-scanning"]
max_memory_bytes = 2147483648

[[agents]]
name = "upstream-synchronizer"
layer = "Core"
cli_tool = "codex"
task = "Sync upstream"
schedule = "0 3 * * *"
capabilities = ["sync"]

[[agents]]
name = "code-reviewer"
layer = "Growth"
cli_tool = "claude"
task = "Review PRs"
capabilities = ["code-review", "architecture"]
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 3);

        // Safety agent
        assert_eq!(config.agents[0].name, "security-sentinel");
        assert_eq!(config.agents[0].layer, AgentLayer::Safety);
        assert!(config.agents[0].schedule.is_none());
        assert_eq!(config.agents[0].max_memory_bytes, Some(2_147_483_648));

        // Core agent with schedule
        assert_eq!(config.agents[1].name, "upstream-synchronizer");
        assert_eq!(config.agents[1].layer, AgentLayer::Core);
        assert_eq!(config.agents[1].schedule.as_deref(), Some("0 3 * * *"));

        // Growth agent
        assert_eq!(config.agents[2].name, "code-reviewer");
        assert_eq!(config.agents[2].layer, AgentLayer::Growth);
        assert!(config.agents[2].schedule.is_none());
        assert_eq!(
            config.agents[2].capabilities,
            vec!["code-review", "architecture"]
        );

        // Nightwatch config
        assert_eq!(config.nightwatch.eval_interval_secs, 300);
        assert!((config.nightwatch.minor_threshold - 0.10).abs() < f64::EPSILON);
        assert!((config.nightwatch.critical_threshold - 0.70).abs() < f64::EPSILON);

        // Compound review config
        assert_eq!(config.compound_review.schedule, "0 2 * * *");
        assert_eq!(config.compound_review.max_duration_secs, 1800);
        assert!(!config.compound_review.create_prs);
    }

    #[test]
    fn test_config_defaults() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "codex"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();

        // Nightwatch defaults
        assert_eq!(config.nightwatch.eval_interval_secs, 300);
        assert!((config.nightwatch.minor_threshold - 0.10).abs() < f64::EPSILON);
        assert!((config.nightwatch.moderate_threshold - 0.20).abs() < f64::EPSILON);
        assert!((config.nightwatch.severe_threshold - 0.40).abs() < f64::EPSILON);
        assert!((config.nightwatch.critical_threshold - 0.70).abs() < f64::EPSILON);

        // Compound review defaults
        assert_eq!(config.compound_review.max_duration_secs, 1800);
        assert!(!config.compound_review.create_prs);

        // Agent defaults
        assert!(config.agents[0].capabilities.is_empty());
        assert!(config.agents[0].max_memory_bytes.is_none());
        assert!(config.agents[0].schedule.is_none());
    }

    #[test]
    fn test_config_restart_defaults() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.restart_cooldown_secs, 60);
        assert_eq!(config.max_restart_count, 10);
        assert_eq!(config.restart_budget_window_secs, 43_200);
        assert_eq!(config.tick_interval_secs, 30);
    }

    #[test]
    fn test_config_restart_custom() {
        let toml_str = r#"
working_dir = "/tmp"
restart_cooldown_secs = 120
max_restart_count = 5
restart_budget_window_secs = 3600
tick_interval_secs = 15

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.restart_cooldown_secs, 120);
        assert_eq!(config.max_restart_count, 5);
        assert_eq!(config.restart_budget_window_secs, 3600);
        assert_eq!(config.tick_interval_secs, 15);
    }

    #[test]
    fn test_example_config_parses() {
        let example_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("orchestrator.example.toml");
        let config = OrchestratorConfig::from_file(&example_path).unwrap();
        assert_eq!(config.agents.len(), 18);
        assert_eq!(config.agents[0].layer, AgentLayer::Safety);
        assert_eq!(config.agents[1].layer, AgentLayer::Safety);
        assert_eq!(config.agents[2].layer, AgentLayer::Core);
        assert!(config.agents[2].schedule.is_some());
    }

    #[test]
    fn test_config_backward_compatible_without_workflow() {
        // Old config without workflow section should still parse
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.workflow.is_none());
    }

    #[test]
    fn test_config_with_workflow_section() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[workflow]
enabled = true
poll_interval_secs = 120
workflow_file = "./WORKFLOW.md"

[workflow.tracker]
kind = "gitea"
endpoint = "https://git.terraphim.cloud"
api_key = "..."
owner = "terraphim"
repo = "terraphim-ai"
use_robot_api = true

[workflow.tracker.states]
active = ["Todo", "In Progress"]
terminal = ["Done", "Closed"]

[workflow.concurrency]
global_max = 5
issue_max = 3
fairness = "round_robin"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();

        let workflow = config.workflow.expect("workflow config should exist");
        assert!(workflow.enabled);
        assert_eq!(workflow.poll_interval_secs, 120);
        assert_eq!(
            workflow.workflow_file,
            std::path::PathBuf::from("./WORKFLOW.md")
        );

        assert_eq!(workflow.tracker.kind, "gitea");
        assert_eq!(workflow.tracker.endpoint, "https://git.terraphim.cloud");
        assert_eq!(workflow.tracker.owner, "terraphim");
        assert!(workflow.tracker.use_robot_api);

        assert_eq!(workflow.tracker.states.active.len(), 2);
        assert_eq!(workflow.tracker.states.terminal.len(), 2);

        assert_eq!(workflow.concurrency.global_max, 5);
        assert_eq!(workflow.concurrency.issue_max, 3);
        assert_eq!(workflow.concurrency.fairness, "round_robin");
    }

    #[test]
    fn test_workflow_defaults() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[workflow]
enabled = true
workflow_file = "./WORKFLOW.md"

[workflow.tracker]
kind = "gitea"
endpoint = "https://git.example.com"
api_key = "..."
owner = "owner"
repo = "repo"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let workflow = config.workflow.expect("workflow config should exist");

        // Check defaults
        assert_eq!(workflow.poll_interval_secs, 120);
        assert!(!workflow.tracker.use_robot_api);
        assert_eq!(workflow.concurrency.global_max, 5);
        assert_eq!(workflow.concurrency.issue_max, 3);
        assert_eq!(workflow.concurrency.fairness, "round_robin");
    }

    #[test]
    fn test_validate_workflow_missing_api_key() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[workflow]
enabled = true
workflow_file = "./WORKFLOW.md"

[workflow.tracker]
kind = "gitea"
endpoint = "https://git.example.com"
api_key = ""
owner = "owner"
repo = "repo"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_parse_with_budget() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
budget_monthly_cents = 5000
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].budget_monthly_cents, Some(5000));
    }

    #[test]
    fn test_config_parse_without_budget() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert!(config.agents[0].budget_monthly_cents.is_none());
    }

    #[test]
    fn test_config_parse_with_persona_fields() {
        let toml_str = r#"
working_dir = "/tmp"
persona_data_dir = "/tmp/personas"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "codex"
task = "Test task"
provider = "openai"
persona = "Security Analyst"
terraphim_role = "Terraphim Engineer"
skill_chain = ["security", "analysis"]
sfia_skills = [{code = "SCTY", level = 5}, {code = "PROG", level = 4}]
fallback_provider = "anthropic"
fallback_model = "claude-sonnet"
grace_period_secs = 30
max_cpu_seconds = 300
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        let agent = &config.agents[0];
        assert_eq!(agent.provider, Some("openai".to_string()));
        assert_eq!(agent.persona, Some("Security Analyst".to_string()));
        assert_eq!(agent.terraphim_role, Some("Terraphim Engineer".to_string()));
        assert_eq!(agent.skill_chain, vec!["security", "analysis"]);
        assert_eq!(agent.sfia_skills.len(), 2);
        assert_eq!(agent.sfia_skills[0].code, "SCTY");
        assert_eq!(agent.sfia_skills[0].level, 5);
        assert_eq!(agent.sfia_skills[1].code, "PROG");
        assert_eq!(agent.sfia_skills[1].level, 4);
        assert_eq!(agent.fallback_provider, Some("anthropic".to_string()));
        assert_eq!(agent.fallback_model, Some("claude-sonnet".to_string()));
        assert_eq!(agent.grace_period_secs, Some(30));
        assert_eq!(agent.max_cpu_seconds, Some(300));
        assert_eq!(
            config.persona_data_dir,
            Some(PathBuf::from("/tmp/personas"))
        );
    }

    #[test]
    fn test_config_parse_without_persona_fields() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "codex"
task = "Test task"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        let agent = &config.agents[0];
        assert!(agent.provider.is_none());
        assert!(agent.persona.is_none());
        assert!(agent.terraphim_role.is_none());
        assert!(agent.skill_chain.is_empty());
        assert!(agent.sfia_skills.is_empty());
        assert!(agent.fallback_provider.is_none());
        assert!(agent.fallback_model.is_none());
        assert!(agent.grace_period_secs.is_none());
        assert!(agent.max_cpu_seconds.is_none());
        assert!(config.persona_data_dir.is_none());
    }

    #[test]
    fn test_config_persona_defaults() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let agent = &config.agents[0];
        assert!(agent.provider.is_none());
        assert!(agent.persona.is_none());
        assert!(agent.terraphim_role.is_none());
        assert!(agent.skill_chain.is_empty());
        assert!(agent.sfia_skills.is_empty());
        assert!(agent.fallback_provider.is_none());
        assert!(agent.fallback_model.is_none());
        assert!(agent.grace_period_secs.is_none());
        assert!(agent.max_cpu_seconds.is_none());
    }

    #[test]
    fn test_config_sfia_skills_parse() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
sfia_skills = [{code = "SCTY", level = 5}]
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents[0].sfia_skills.len(), 1);
        assert_eq!(config.agents[0].sfia_skills[0].code, "SCTY");
        assert_eq!(config.agents[0].sfia_skills[0].level, 5);
    }

    #[test]
    fn test_config_skill_chain_parse() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
skill_chain = ["a", "b"]
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents[0].skill_chain, vec!["a", "b"]);
    }

    #[test]
    fn test_config_persona_data_dir() {
        let toml_str = r#"
working_dir = "/tmp"
persona_data_dir = "/tmp/personas"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(
            config.persona_data_dir,
            Some(PathBuf::from("/tmp/personas"))
        );
    }

    #[test]
    fn test_config_persona_data_dir_default() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.persona_data_dir.is_none());
    }

    #[test]
    fn test_example_config_parses_with_persona() {
        let example_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("orchestrator.example.toml");
        if example_path.exists() {
            let config = OrchestratorConfig::from_file(&example_path).unwrap();
            assert!(config.agents.len() >= 3);
        }
    }

    #[test]
    fn test_config_parse_pre_check_always() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "always"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents[0].pre_check, Some(PreCheckStrategy::Always));
    }

    #[test]
    fn test_config_parse_pre_check_git_diff() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "git-diff"
watch_paths = ["crates/", "Cargo.toml"]
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        match &config.agents[0].pre_check {
            Some(PreCheckStrategy::GitDiff { watch_paths }) => {
                assert_eq!(
                    watch_paths,
                    &vec!["crates/".to_string(), "Cargo.toml".to_string()]
                );
            }
            other => panic!("expected GitDiff, got {:?}", other),
        }
    }

    #[test]
    fn test_config_parse_pre_check_gitea_issue() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "gitea-issue"
issue_number = 637
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        match &config.agents[0].pre_check {
            Some(PreCheckStrategy::GiteaIssue { issue_number }) => {
                assert_eq!(*issue_number, 637);
            }
            other => panic!("expected GiteaIssue, got {:?}", other),
        }
    }

    #[test]
    fn test_config_parse_pre_check_shell() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "shell"
script = "echo hello"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        match &config.agents[0].pre_check {
            Some(PreCheckStrategy::Shell {
                script,
                timeout_secs,
            }) => {
                assert_eq!(script, "echo hello");
                assert_eq!(*timeout_secs, 60); // default
            }
            other => panic!("expected Shell, got {:?}", other),
        }
    }

    #[test]
    fn test_config_parse_pre_check_shell_custom_timeout() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "shell"
script = "test -f /tmp/flag"
timeout_secs = 10
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        match &config.agents[0].pre_check {
            Some(PreCheckStrategy::Shell {
                script,
                timeout_secs,
            }) => {
                assert_eq!(script, "test -f /tmp/flag");
                assert_eq!(*timeout_secs, 10);
            }
            other => panic!("expected Shell, got {:?}", other),
        }
    }

    #[test]
    fn test_config_parse_no_pre_check() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.agents[0].pre_check.is_none());
    }

    #[test]
    fn test_config_validate_gitea_issue_requires_workflow() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "gitea-issue"
issue_number = 42
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("gitea-issue"),
            "error should mention gitea-issue: {}",
            err
        );
    }

    #[test]
    fn test_config_validate_gitea_issue_with_workflow_ok() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[workflow]
enabled = true
workflow_file = "./WORKFLOW.md"
[workflow.tracker]
kind = "gitea"
endpoint = "https://git.example.com"
api_key = "token123"
owner = "owner"
repo = "repo"
[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
[agents.pre_check]
kind = "gitea-issue"
issue_number = 42
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_with_flows() {
        let toml_str = r#"
working_dir = "/tmp"
flow_state_dir = "/tmp/flow-states"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"

[[flows]]
name = "test-flow"
project = "default"
repo_path = "/home/user/project"

[[flows.steps]]
name = "build"
kind = "action"
command = "cargo build"

[[flows.steps]]
name = "test"
kind = "action"
command = "cargo test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();

        // Check flows parsed correctly
        assert_eq!(config.flows.len(), 1);
        assert_eq!(config.flows[0].name, "test-flow");
        assert_eq!(config.flows[0].repo_path, "/home/user/project");
        assert_eq!(config.flows[0].steps.len(), 2);

        // Check step data
        assert_eq!(config.flows[0].steps[0].name, "build");
        assert_eq!(
            config.flows[0].steps[0].kind,
            crate::flow::config::StepKind::Action
        );
        assert_eq!(
            config.flows[0].steps[0].command,
            Some("cargo build".to_string())
        );

        assert_eq!(config.flows[0].steps[1].name, "test");
        assert_eq!(
            config.flows[0].steps[1].command,
            Some("cargo test".to_string())
        );

        // Check flow_state_dir
        assert_eq!(
            config.flow_state_dir,
            Some(PathBuf::from("/tmp/flow-states"))
        );
    }

    #[test]
    fn test_config_without_flows() {
        // Existing config without flows section should still parse
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "echo"
task = "t"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();

        // Flows should be empty by default
        assert!(config.flows.is_empty());
        assert!(config.flow_state_dir.is_none());
    }

    // --- is_allowed_provider: C1/C3 runtime gate ---

    #[test]
    fn test_is_allowed_provider_c1_allowed_prefixes() {
        // Every prefix in ALLOWED_PROVIDER_PREFIXES must pass in `provider/model` form.
        assert!(is_allowed_provider("claude-code/sonnet-4.5"));
        assert!(is_allowed_provider("opencode-go/kimi-k2.5"));
        assert!(is_allowed_provider("kimi-for-coding/k2p5"));
        assert!(is_allowed_provider("minimax-coding-plan/MiniMax-M2.5"));
        assert!(is_allowed_provider("zai-coding-plan/glm-4.6"));
        assert!(is_allowed_provider("openai/gpt-5.4"));
        assert!(is_allowed_provider("openai/gpt-5.3-codex"));
        assert!(is_allowed_provider("openai/gpt-5.4"));
    }

    #[test]
    fn test_is_allowed_provider_c3_banned_prefixes() {
        // Every prefix in BANNED_PROVIDER_PREFIXES must be rejected.
        assert!(!is_allowed_provider("opencode/whatever"));
        assert!(!is_allowed_provider("github-copilot/gpt-4.1"));
        assert!(!is_allowed_provider("google/gemini-2.5"));
        assert!(!is_allowed_provider("huggingface/llama-3"));
        assert!(!is_allowed_provider("minimax/MiniMax-M2.5"));
    }

    #[test]
    fn test_is_allowed_provider_c3_prefix_boundary() {
        // Exact prefix match: banned `opencode` must not shadow allowed
        // `opencode-go`; banned `minimax` must not shadow `minimax-coding-plan`.
        assert!(is_allowed_provider("opencode-go/any"));
        assert!(!is_allowed_provider("opencode/any"));
        assert!(is_allowed_provider("minimax-coding-plan/any"));
        assert!(!is_allowed_provider("minimax/any"));
    }

    #[test]
    fn test_is_allowed_provider_bare_claude_cli() {
        // Bare model names route through claude-code CLI and pass.
        assert!(is_allowed_provider("sonnet"));
        assert!(is_allowed_provider("opus"));
        assert!(is_allowed_provider("haiku"));
        assert!(is_allowed_provider("anthropic"));
    }

    #[test]
    fn test_is_allowed_provider_bare_allowed_id() {
        // Bare provider ids that appear in the allow-list pass.
        assert!(is_allowed_provider("claude-code"));
        assert!(is_allowed_provider("opencode-go"));
        assert!(is_allowed_provider("kimi-for-coding"));
    }

    #[test]
    fn test_is_allowed_provider_anthropic_prefixed() {
        // `anthropic/<model>` routes through claude-code CLI and must pass.
        assert!(is_allowed_provider("anthropic/claude-3.5-sonnet"));
        assert!(is_allowed_provider("anthropic/claude-opus-4"));
    }

    #[test]
    fn test_is_allowed_provider_rejects_unknown_bare_names() {
        // P1 fix: bare banned ids must now reject. Previously any bare
        // string was waved through, so `model = "minimax"` silently
        // bypassed the C3 banlist.
        assert!(!is_allowed_provider("minimax"));
        assert!(!is_allowed_provider("opencode"));
        assert!(!is_allowed_provider("google"));
        assert!(!is_allowed_provider("github-copilot"));
        assert!(!is_allowed_provider("huggingface"));
        // And any other unknown bare id.
        assert!(!is_allowed_provider("unknown"));
        assert!(!is_allowed_provider(""));
    }

    #[test]
    fn test_validate_model_provider_rejects_bare_banned() {
        // The load-time check must also refuse bare banned ids -- the
        // runtime filter alone cannot catch them if the validator waves
        // the config through at startup.
        let err = validate_model_provider("bare-banned", "model", "minimax")
            .expect_err("bare 'minimax' must be rejected");
        assert!(matches!(
            err,
            crate::error::OrchestratorError::BannedProvider { .. }
        ));
        // And the allowed bare forms still pass.
        validate_model_provider("ok", "model", "sonnet").expect("sonnet is a bare claude CLI");
        validate_model_provider("ok", "model", "anthropic").expect("anthropic bare passes");
    }

    /// ADF Phase 2 (issue #944): when no `[pr_dispatch]` block is present in
    /// the config TOML, the field deserialises to `None` so the orchestrator
    /// can fall back to the legacy single-agent default. This preserves
    /// backward compatibility for every existing deployment that has not yet
    /// adopted the fan-out config.
    #[test]
    fn pr_dispatch_absent_block_yields_none() {
        let toml_str = r#"
working_dir = "/tmp/pr-dispatch-default-test"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"
"#;
        let cfg = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(cfg.pr_dispatch.is_none());
    }

    /// `legacy_default()` returns a single `pr-reviewer` entry with the
    /// canonical `adf/pr-reviewer` context. The orchestrator uses this when
    /// the config block is absent.
    #[test]
    fn pr_dispatch_legacy_default_lists_only_pr_reviewer() {
        let dft = PrDispatchConfig::legacy_default();
        assert_eq!(dft.agents_on_pr_open.len(), 1);
        assert_eq!(dft.agents_on_pr_open[0].name, "pr-reviewer");
        assert_eq!(dft.agents_on_pr_open[0].context, "adf/pr-reviewer");
    }

    /// `agents_on_pr_open_for_project()` falls all the way through to the
    /// legacy default when neither a per-project block nor a top-level block
    /// exists. Pre-Phase-2 deployments must keep working without any edit.
    #[test]
    fn agents_on_pr_open_for_project_falls_back_to_legacy_default() {
        let toml_str = r#"
working_dir = "/tmp/pr-dispatch-accessor-test"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"
"#;
        let cfg = OrchestratorConfig::from_toml(toml_str).unwrap();
        let list = cfg.agents_on_pr_open_for_project("any-project");
        assert_eq!(list.len(), 1, "legacy default must be a single entry");
        assert_eq!(list[0].name, "pr-reviewer");
        assert_eq!(list[0].context, "adf/pr-reviewer");
    }

    /// `agents_on_pr_open_for_project()` falls back to the top-level
    /// `[pr_dispatch]` block when no per-project block is registered for the
    /// requested project. Backward-compat for fleets that authored the
    /// global block in `orchestrator.toml`.
    #[test]
    fn agents_on_pr_open_for_project_falls_back_to_top_level_block() {
        let toml_str = r#"
working_dir = "/tmp/pr-dispatch-accessor-configured"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[pr_dispatch]
agents_on_pr_open = [
    { name = "build-runner", context = "adf/build" },
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#;
        let cfg = OrchestratorConfig::from_toml(toml_str).unwrap();
        let list = cfg.agents_on_pr_open_for_project("any-project");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "build-runner");
        assert_eq!(list[1].name, "pr-reviewer");
    }

    /// `agents_on_pr_open_for_project()` returns the per-project block in
    /// preference to the top-level fallback when both are present. This is
    /// the multi-project path: each project's `conf.d/<project>.toml`
    /// declares its own block and that block wins.
    #[test]
    fn agents_on_pr_open_for_project_returns_per_project_block() {
        let mut cfg = OrchestratorConfig::from_toml(
            r#"
working_dir = "/tmp/per-project"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[pr_dispatch]
agents_on_pr_open = [
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#,
        )
        .unwrap();
        cfg.pr_dispatch_per_project.insert(
            "alpha".to_string(),
            PrDispatchConfig {
                agents_on_pr_open: vec![
                    PrDispatchEntry {
                        name: "build-runner".to_string(),
                        context: "adf/build".to_string(),
                    },
                    PrDispatchEntry {
                        name: "pr-reviewer".to_string(),
                        context: "adf/pr-reviewer".to_string(),
                    },
                ],
            },
        );

        let alpha = cfg.agents_on_pr_open_for_project("alpha");
        assert_eq!(
            alpha.len(),
            2,
            "per-project block must override the top-level fallback for alpha"
        );
        assert_eq!(alpha[0].name, "build-runner");

        let other = cfg.agents_on_pr_open_for_project("beta");
        assert_eq!(
            other.len(),
            1,
            "beta has no per-project block so the top-level fallback applies"
        );
        assert_eq!(other[0].name, "pr-reviewer");
    }

    /// A configured `[pr_dispatch]` block with two entries deserialises to
    /// the canonical Phase 2 D2 minimal shape: build-runner + pr-reviewer.
    #[test]
    fn pr_dispatch_block_parses_two_entry_list() {
        let toml_str = r#"
working_dir = "/tmp/pr-dispatch-parse-test"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[pr_dispatch]
agents_on_pr_open = [
    { name = "build-runner", context = "adf/build" },
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#;
        let cfg = OrchestratorConfig::from_toml(toml_str).unwrap();
        let dispatch = cfg
            .pr_dispatch
            .as_ref()
            .expect("pr_dispatch block must deserialise when present");
        assert_eq!(dispatch.agents_on_pr_open.len(), 2);
        assert_eq!(dispatch.agents_on_pr_open[0].name, "build-runner");
        assert_eq!(dispatch.agents_on_pr_open[0].context, "adf/build");
        assert_eq!(dispatch.agents_on_pr_open[1].name, "pr-reviewer");
        assert_eq!(dispatch.agents_on_pr_open[1].context, "adf/pr-reviewer");
    }

    /// Issue #962: an `IncludeFragment` (the per-include conf.d shape) must
    /// accept its own `[pr_dispatch]` block. Until this test passed, the
    /// `#[serde(deny_unknown_fields)]` guard on `IncludeFragment` rejected
    /// any conf.d file that tried to declare per-project fan-out -- which is
    /// exactly the failure observed on bigbox when `terraphim.toml` got a
    /// `[pr_dispatch]` block. See `.docs/design-pr-dispatch-per-project.md`.
    #[test]
    fn include_fragment_parses_pr_dispatch_block() {
        let toml_str = r#"
[[projects]]
id = "terraphim"
working_dir = "/tmp/terraphim"

[[agents]]
name = "pr-reviewer"
layer = "Safety"
cli_tool = "echo"
task = "review"
project = "terraphim"

[pr_dispatch]
agents_on_pr_open = [
    { name = "build-runner", context = "adf/build" },
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#;
        let fragment: IncludeFragment =
            toml::from_str(toml_str).expect("IncludeFragment must accept pr_dispatch block");
        assert_eq!(fragment.projects.len(), 1);
        assert_eq!(fragment.projects[0].id, "terraphim");
        assert_eq!(fragment.agents.len(), 1);
        let dispatch = fragment
            .pr_dispatch
            .expect("pr_dispatch block must deserialise inside an include");
        assert_eq!(dispatch.agents_on_pr_open.len(), 2);
        assert_eq!(dispatch.agents_on_pr_open[0].name, "build-runner");
        assert_eq!(dispatch.agents_on_pr_open[0].context, "adf/build");
        assert_eq!(dispatch.agents_on_pr_open[1].name, "pr-reviewer");
        assert_eq!(dispatch.agents_on_pr_open[1].context, "adf/pr-reviewer");
    }

    /// Issue #962 Step 2: `from_file` walks every include fragment, finds
    /// each `[pr_dispatch]` block, and indexes it under every project id
    /// declared in the same fragment. Two include files (one per project)
    /// each carrying their own block must produce a two-entry map.
    #[test]
    fn from_file_aggregates_pr_dispatch_from_includes() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let conf_d = tmp.path().join("conf.d");
        std::fs::create_dir(&conf_d).unwrap();

        std::fs::write(
            conf_d.join("alpha.toml"),
            r#"
[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[pr_dispatch]
agents_on_pr_open = [
    { name = "build-runner", context = "adf/build" },
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#,
        )
        .unwrap();

        std::fs::write(
            conf_d.join("beta.toml"),
            r#"
[[projects]]
id = "beta"
working_dir = "/tmp/beta"

[pr_dispatch]
agents_on_pr_open = [
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#,
        )
        .unwrap();

        let base_path = tmp.path().join("orchestrator.toml");
        std::fs::write(
            &base_path,
            r#"
working_dir = "/tmp/o"
include = ["conf.d/*.toml"]

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp/o"
"#,
        )
        .unwrap();

        let cfg = OrchestratorConfig::from_file(&base_path).unwrap();
        assert_eq!(
            cfg.projects.len(),
            2,
            "include expansion must merge both project blocks; got: {:?}",
            cfg.projects.iter().map(|p| &p.id).collect::<Vec<_>>()
        );
        assert_eq!(
            cfg.pr_dispatch_per_project.len(),
            2,
            "both per-project blocks must be aggregated; map keys: {:?}",
            cfg.pr_dispatch_per_project.keys().collect::<Vec<_>>()
        );
        let alpha = cfg
            .pr_dispatch_per_project
            .get("alpha")
            .expect("alpha block must be present");
        assert_eq!(alpha.agents_on_pr_open.len(), 2);
        assert_eq!(alpha.agents_on_pr_open[0].name, "build-runner");
        let beta = cfg
            .pr_dispatch_per_project
            .get("beta")
            .expect("beta block must be present");
        assert_eq!(beta.agents_on_pr_open.len(), 1);
        assert_eq!(beta.agents_on_pr_open[0].name, "pr-reviewer");
    }

    /// Issue #962 Step 2: a `[pr_dispatch]` block in an include file with no
    /// `[[projects]]` is orphaned -- nothing to key it under. The load must
    /// succeed (don't break existing fleets) but the map stays empty and a
    /// warn is logged.
    #[test]
    fn from_file_warns_when_pr_dispatch_in_include_has_no_projects() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let conf_d = tmp.path().join("conf.d");
        std::fs::create_dir(&conf_d).unwrap();
        std::fs::write(
            conf_d.join("orphan.toml"),
            r#"
[pr_dispatch]
agents_on_pr_open = [
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
"#,
        )
        .unwrap();

        let base_path = tmp.path().join("orchestrator.toml");
        std::fs::write(
            &base_path,
            r#"
working_dir = "/tmp/o"
include = ["conf.d/*.toml"]

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp/o"
"#,
        )
        .unwrap();

        let cfg = OrchestratorConfig::from_file(&base_path).unwrap();
        assert!(
            cfg.pr_dispatch_per_project.is_empty(),
            "orphan pr_dispatch (no projects) must not pollute the map"
        );
    }

    #[test]
    fn test_agent_definition_event_only_default_false() {
        let toml_str = r#"
working_dir = "/tmp/terraphim"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[agents]]
name = "llm-agent"
layer = "Growth"
cli_tool = "codex"
task = "Do something"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert!(
            !config.agents[0].event_only,
            "event_only must default to false when not specified in TOML"
        );
    }

    #[test]
    fn test_agent_definition_event_only_true_round_trip() {
        let toml_str = r#"
working_dir = "/tmp/terraphim"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[agents]]
name = "build-runner"
layer = "Core"
cli_tool = "/bin/bash"
event_only = true
task = "build"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert!(
            config.agents[0].event_only,
            "event_only must survive TOML round-trip when set to true"
        );
    }

    #[cfg(unix)]
    mod permission_tests {
        use std::os::unix::fs::PermissionsExt;

        /// Creates a temp file with the given octal mode and returns its path.
        fn temp_file_with_mode(content: &str, mode: u32) -> tempfile::TempDir {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("test_config.toml");
            std::fs::write(&path, content).unwrap();
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(mode);
            std::fs::set_permissions(&path, perms).unwrap();
            dir
        }

        #[test]
        fn warn_if_world_readable_does_not_panic_on_secure_file() {
            let dir = temp_file_with_mode("content", 0o600);
            let path = dir.path().join("test_config.toml");
            // Should complete without panic; warnings are advisory only.
            super::super::warn_if_world_readable(&path);
        }

        #[test]
        fn warn_if_world_readable_does_not_panic_on_world_readable_file() {
            let dir = temp_file_with_mode("content", 0o644);
            let path = dir.path().join("test_config.toml");
            // Logs an error but must not panic or return an error.
            super::super::warn_if_world_readable(&path);
        }

        #[test]
        fn warn_if_world_readable_does_not_panic_on_missing_file() {
            let path = std::path::Path::new("/nonexistent/path/config.toml");
            // Should silently log debug and return without panic.
            super::super::warn_if_world_readable(path);
        }
    }

    // === D2: grace_period_secs validation tests ===

    #[test]
    fn test_validate_grace_period_too_low() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
grace_period_secs = 2
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::AgentFieldOutOfRange {
                ref field,
                ..
            } if field == "grace_period_secs"
        ));
    }

    #[test]
    fn test_validate_grace_period_too_high() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
grace_period_secs = 500
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::AgentFieldOutOfRange {
                ref field,
                ..
            } if field == "grace_period_secs"
        ));
    }

    #[test]
    fn test_validate_grace_period_in_range() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
grace_period_secs = 30
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.validate().is_ok());
    }

    // === D3: max_cpu_seconds validation tests ===

    #[test]
    fn test_validate_max_cpu_too_low() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
max_cpu_seconds = 30
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::AgentFieldOutOfRange {
                ref field,
                ..
            } if field == "max_cpu_seconds"
        ));
    }

    #[test]
    fn test_validate_max_cpu_too_high() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
max_cpu_seconds = 10000
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::AgentFieldOutOfRange {
                ref field,
                ..
            } if field == "max_cpu_seconds"
        ));
    }

    #[test]
    fn test_validate_max_cpu_in_range() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
max_cpu_seconds = 3600
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.validate().is_ok());
    }

    // === D4: RoutingConfig probe_ttl_secs validation tests ===

    #[test]
    fn test_validate_probe_ttl_too_short() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[routing]
taxonomy_path = "/tmp/taxonomy"
probe_ttl_secs = 30
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::ProbeTtlTooShort { .. }
        ));
    }

    #[test]
    fn test_validate_probe_ttl_in_range() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[routing]
taxonomy_path = "/tmp/taxonomy"
probe_ttl_secs = 120
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_no_routing_no_probe_validation() {
        let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn gitea_output_config_token_redacted_in_debug() {
        let cfg = GiteaOutputConfig {
            base_url: "https://git.example".to_string(),
            token: "super-secret-gitea-token".to_string(),
            owner: "acme".to_string(),
            repo: "platform".to_string(),
            agent_tokens_path: None,
        };
        let dbg = format!("{:?}", cfg);
        assert!(
            !dbg.contains("super-secret-gitea-token"),
            "Gitea token must be redacted in Debug output, got: {dbg}"
        );
        assert!(
            dbg.contains("***REDACTED***"),
            "Debug output should mark token as redacted, got: {dbg}"
        );
    }

    #[test]
    fn webhook_config_secret_redacted_in_debug() {
        let cfg = WebhookConfig {
            bind: "127.0.0.1:9090".to_string(),
            secret: Some("hmac-webhook-secret".to_string()),
        };
        let dbg = format!("{:?}", cfg);
        assert!(
            !dbg.contains("hmac-webhook-secret"),
            "Webhook secret must be redacted in Debug output, got: {dbg}"
        );
        assert!(
            dbg.contains("***REDACTED***"),
            "Debug output should mark secret as redacted, got: {dbg}"
        );
    }

    #[test]
    fn webhook_config_none_secret_debug_shows_none() {
        let cfg = WebhookConfig {
            bind: "127.0.0.1:9090".to_string(),
            secret: None,
        };
        let dbg = format!("{:?}", cfg);
        assert!(
            dbg.contains("None"),
            "None secret should show as None in Debug output, got: {dbg}"
        );
    }

    #[test]
    fn expand_env_vars_substitutes_set_variable() {
        std::env::set_var("_TEST_EXPAND_VAR_1546", "my_secret_value");
        let result = expand_env_vars("secret = \"${_TEST_EXPAND_VAR_1546}\"");
        assert_eq!(result, "secret = \"my_secret_value\"");
        std::env::remove_var("_TEST_EXPAND_VAR_1546");
    }

    #[test]
    fn expand_env_vars_empty_string_for_unset_variable() {
        std::env::remove_var("_TEST_UNSET_VAR_1546");
        let result = expand_env_vars("secret = \"${_TEST_UNSET_VAR_1546}\"");
        // Unset variable expands to empty string
        assert_eq!(result, "secret = \"\"");
    }

    #[test]
    fn expand_env_vars_dollar_brace_syntax_only() {
        // $VAR (without braces) must NOT be expanded — avoids breaking TOML values
        let result = expand_env_vars("secret = \"$PLAIN_VAR\"");
        assert_eq!(result, "secret = \"$PLAIN_VAR\"");
    }

    #[test]
    fn tracker_config_api_key_redacted_in_debug() {
        let cfg = TrackerConfig {
            kind: "gitea".to_string(),
            endpoint: "https://git.example/api/v1".to_string(),
            api_key: "live-gitea-api-token".to_string(),
            owner: "acme".to_string(),
            repo: "platform".to_string(),
            project_slug: None,
            use_robot_api: false,
            states: TrackerStates::default(),
        };
        let dbg = format!("{:?}", cfg);
        assert!(
            !dbg.contains("live-gitea-api-token"),
            "Tracker api_key must be redacted in Debug output, got: {dbg}"
        );
        assert!(
            dbg.contains("***REDACTED***"),
            "Debug output should mark api_key as redacted, got: {dbg}"
        );
    }
}
