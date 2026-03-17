use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    /// Reconciliation tick interval in seconds.
    #[serde(default = "default_tick_interval")]
    pub tick_interval_secs: u64,
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
}

impl Default for NightwatchConfig {
    fn default() -> Self {
        Self {
            eval_interval_secs: default_eval_interval(),
            minor_threshold: default_minor_threshold(),
            moderate_threshold: default_moderate_threshold(),
            severe_threshold: default_severe_threshold(),
            critical_threshold: default_critical_threshold(),
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
}

fn default_max_duration() -> u64 {
    1800
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
        Ok(())
    }
}

/// Substitute environment variables in a string.
/// Supports ${VAR} and $VAR syntax.
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

    // Handle $VAR syntax (simplistic)
    // Note: This is a basic implementation. A full implementation would use regex.
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
        assert_eq!(config.agents.len(), 3);
        assert_eq!(config.agents[0].layer, AgentLayer::Safety);
        assert_eq!(config.agents[1].layer, AgentLayer::Core);
        assert_eq!(config.agents[2].layer, AgentLayer::Growth);
        assert!(config.agents[1].schedule.is_some());
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
api_key = "${GITEA_TOKEN}"
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
api_key = "test"
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
}
