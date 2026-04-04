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
    pub agents: Vec<AgentDefinition>,
    /// Seconds to wait before restarting a Safety agent after it exits.
    #[serde(default = "default_restart_cooldown")]
    pub restart_cooldown_secs: u64,
    /// Maximum number of restarts per Safety agent before giving up.
    #[serde(default = "default_max_restart_count")]
    pub max_restart_count: u32,
    /// Disk usage percentage threshold (0-100) above which agent spawning is refused.
    /// Set to 100 to disable the guard. Default: 90.
    #[serde(default = "default_disk_usage_threshold")]
    pub disk_usage_threshold: u8,
    /// Reconciliation tick interval in seconds.
    #[serde(default = "default_tick_interval")]
    pub tick_interval_secs: u64,
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
}

/// Configuration for posting agent output to Gitea issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaOutputConfig {
    pub base_url: String,
    pub token: String,
    pub owner: String,
    pub repo: String,
}

/// Configuration for mention-driven dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionConfig {
    /// DEPRECATED: Issue numbers to watch. Ignored when cursor polling is active.
    #[serde(default)]
    pub watch_issues: Vec<u64>,
    /// DEPRECATED: Max dispatch depth per issue. Replaced by max_dispatches_per_tick.
    #[serde(default = "default_max_mention_depth")]
    pub max_mention_depth: u32,
    /// Poll every N reconciliation ticks (default 2).
    #[serde(default = "default_poll_modulo")]
    pub poll_modulo: u64,
    /// Max mentions to dispatch per poll tick (default 3).
    #[serde(default = "default_max_dispatches_per_tick")]
    pub max_dispatches_per_tick: u32,
    /// Max concurrent mention-spawned agents (default 5).
    #[serde(default = "default_max_concurrent_mention_agents")]
    pub max_concurrent_mention_agents: u32,
}

fn default_max_mention_depth() -> u32 {
    3
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

/// Configuration for the webhook server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Bind address for the webhook server (default "0.0.0.0:9090").
    #[serde(default = "default_webhook_bind")]
    pub bind: String,
    /// Shared secret for HMAC signature verification.
    /// Must match the secret configured in Gitea webhook settings.
    pub secret: Option<String>,
}

fn default_webhook_bind() -> String {
    "0.0.0.0:9090".to_string()
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerConfig {
    /// Tracker kind: "gitea" or "linear".
    pub kind: String,
    /// API endpoint URL.
    pub endpoint: String,
    /// API key (supports env var substitution like "${GITEA_TOKEN}").
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

fn default_disk_usage_threshold() -> u8 {
    90
}

fn default_tick_interval() -> u64 {
    30
}

impl OrchestratorConfig {
    /// Parse an OrchestratorConfig from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, crate::error::OrchestratorError> {
        toml::from_str(toml_str).map_err(|e| crate::error::OrchestratorError::Config(e.to_string()))
    }

    /// Load an OrchestratorConfig from a TOML file.
    pub fn from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, crate::error::OrchestratorError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::from_toml(&content)
    }

    /// Substitute environment variables in workflow config.
    /// Replaces ${VAR} or $VAR with the value of the environment variable.
    pub fn substitute_env_vars(&mut self) {
        if let Some(ref mut workflow) = self.workflow {
            workflow.tracker.api_key = substitute_env(&workflow.tracker.api_key);
        }
    }

    /// Validate the configuration.
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
        assert_eq!(config.tick_interval_secs, 30);
    }

    #[test]
    fn test_config_restart_custom() {
        let toml_str = r#"
working_dir = "/tmp"
restart_cooldown_secs = 120
max_restart_count = 5
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
        assert_eq!(config.tick_interval_secs, 15);
    }

    #[test]
    fn test_example_config_parses() {
        let example_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("orchestrator.example.toml");
        let config = OrchestratorConfig::from_file(&example_path).unwrap();
        assert_eq!(config.agents.len(), 14);
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
}
