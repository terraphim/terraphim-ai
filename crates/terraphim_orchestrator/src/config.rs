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
    /// Provider prefix for model routing (e.g., "opencode-go", "kimi-for-coding", "claude-code").
    #[serde(default)]
    pub provider: Option<String>,
    /// Fallback provider if primary fails/times out.
    #[serde(default)]
    pub fallback_provider: Option<String>,
    /// Fallback model to use with fallback_provider.
    #[serde(default)]
    pub fallback_model: Option<String>,
    /// Provider tier classification.
    #[serde(default)]
    pub provider_tier: Option<ProviderTier>,
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

/// Model routing tier based on task complexity and cost.
/// See ADR-003: Four-tier model routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderTier {
    /// Routine docs, advisory. Primary: opencode-go/minimax-m2.5. Timeout: 30s.
    Quick,
    /// Quality gates, compound review, security. Primary: opencode-go/glm-5. Timeout: 60s.
    Deep,
    /// Code generation, twins, tests. Primary: kimi-for-coding/k2p5. Timeout: 120s.
    Implementation,
    /// Spec validation, deep reasoning. Primary: claude-code opus-4-6. Timeout: 300s. No fallback.
    Oracle,
}

impl ProviderTier {
    /// Timeout in seconds for this tier
    pub fn timeout_secs(&self) -> u64 {
        match self {
            Self::Quick => 30,
            Self::Deep => 60,
            Self::Implementation => 120,
            Self::Oracle => 300,
        }
    }
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
    fn test_config_parse_with_provider_fields() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "security-sentinel"
layer = "Safety"
cli_tool = "opencode"
provider = "opencode-go"
model = "kimi-k2.5"
fallback_provider = "opencode-go"
fallback_model = "glm-5"
provider_tier = "Deep"
task = "Run security audit"
capabilities = ["security", "vulnerability-scanning"]
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "security-sentinel");
        assert_eq!(config.agents[0].provider, Some("opencode-go".to_string()));
        assert_eq!(config.agents[0].model, Some("kimi-k2.5".to_string()));
        assert_eq!(
            config.agents[0].fallback_provider,
            Some("opencode-go".to_string())
        );
        assert_eq!(config.agents[0].fallback_model, Some("glm-5".to_string()));
        assert_eq!(config.agents[0].provider_tier, Some(ProviderTier::Deep));
    }

    #[test]
    fn test_provider_tier_timeout_secs() {
        assert_eq!(ProviderTier::Quick.timeout_secs(), 30);
        assert_eq!(ProviderTier::Deep.timeout_secs(), 60);
        assert_eq!(ProviderTier::Implementation.timeout_secs(), 120);
        assert_eq!(ProviderTier::Oracle.timeout_secs(), 300);
    }

    #[test]
    fn test_provider_fields_backward_compatible() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "legacy-agent"
layer = "Safety"
cli_tool = "codex"
task = "Legacy task without new fields"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "legacy-agent");
        assert!(config.agents[0].provider.is_none());
        assert!(config.agents[0].fallback_provider.is_none());
        assert!(config.agents[0].fallback_model.is_none());
        assert!(config.agents[0].provider_tier.is_none());
    }

    #[test]
    fn test_all_provider_tier_variants() {
        let tiers = vec![
            ("Quick", ProviderTier::Quick),
            ("Deep", ProviderTier::Deep),
            ("Implementation", ProviderTier::Implementation),
            ("Oracle", ProviderTier::Oracle),
        ];
        for (name, tier) in tiers {
            let toml_str = format!(
                r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "codex"
provider_tier = "{}"
task = "Test"
"#,
                name
            );
            let config = OrchestratorConfig::from_toml(&toml_str).unwrap();
            assert_eq!(config.agents[0].provider_tier, Some(tier));
        }
    }
}
