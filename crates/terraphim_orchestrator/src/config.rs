use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A review pair definition: when a producer agent completes, request review from another agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPair {
    pub producer: String,
    pub reviewer: String,
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
    /// Allowed provider prefixes. Providers not in this list are rejected at spawn time.
    /// Empty list = allow all (backward compatible).
    #[serde(default)]
    pub allowed_providers: Vec<String>,
    /// Explicitly banned provider prefixes. These are rejected even if not in allowlist.
    /// Default: ["opencode"] (Zen proxy, see ADR-002)
    #[serde(default = "default_banned_providers")]
    pub banned_providers: Vec<String>,
    /// Skill chain registry for agent validation
    #[serde(default)]
    pub skill_registry: SkillChainRegistry,
    /// Milliseconds to wait between spawning Safety agents (thundering herd prevention).
    #[serde(default = "default_stagger_delay_ms")]
    pub stagger_delay_ms: u64,
    /// Cross-agent review pairs: when producer completes, request review from reviewer.
    #[serde(default)]
    pub review_pairs: Vec<ReviewPair>,
    /// Strategic drift detection configuration.
    #[serde(default)]
    pub drift_detection: DriftDetectionConfig,
    /// Session rotation configuration.
    #[serde(default)]
    pub session_rotation: SessionRotationConfig,
    /// Convergence detection configuration.
    #[serde(default)]
    pub convergence: ConvergenceConfig,
}

/// Configuration for convergence detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceConfig {
    /// Similarity threshold (0.0 - 1.0) for convergence detection.
    #[serde(default = "default_convergence_threshold")]
    pub threshold: f64,
    /// Number of consecutive similar outputs required.
    #[serde(default = "default_consecutive_threshold")]
    pub consecutive_threshold: u32,
    /// Whether to skip next run on convergence.
    #[serde(default)]
    pub skip_on_convergence: bool,
}

impl Default for ConvergenceConfig {
    fn default() -> Self {
        Self {
            threshold: default_convergence_threshold(),
            consecutive_threshold: default_consecutive_threshold(),
            skip_on_convergence: false,
        }
    }
}

fn default_convergence_threshold() -> f64 {
    0.95
}

fn default_consecutive_threshold() -> u32 {
    3
}

/// Configuration for session rotation (fresh eyes mechanism).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRotationConfig {
    /// Maximum number of sessions before rotation (0 = disabled).
    #[serde(default = "default_max_sessions_before_rotation")]
    pub max_sessions_before_rotation: u32,
    /// Optional maximum session duration in seconds.
    #[serde(default)]
    pub max_session_duration_secs: Option<u64>,
}

impl Default for SessionRotationConfig {
    fn default() -> Self {
        Self {
            max_sessions_before_rotation: default_max_sessions_before_rotation(),
            max_session_duration_secs: None,
        }
    }
}

fn default_max_sessions_before_rotation() -> u32 {
    10
}

/// Configuration for strategic drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetectionConfig {
    /// How many ticks between drift checks.
    #[serde(default = "default_drift_check_interval")]
    pub check_interval_ticks: u32,
    /// Drift score threshold (0.0 - 1.0) above which to log warnings.
    #[serde(default = "default_drift_threshold")]
    pub drift_threshold: f64,
    /// Path to the plans directory containing strategic goals.
    #[serde(default = "default_plans_dir")]
    pub plans_dir: PathBuf,
    /// Whether to pause agents when drift is detected.
    #[serde(default)]
    pub pause_on_drift: bool,
}

impl Default for DriftDetectionConfig {
    fn default() -> Self {
        Self {
            check_interval_ticks: default_drift_check_interval(),
            drift_threshold: default_drift_threshold(),
            plans_dir: default_plans_dir(),
            pause_on_drift: false,
        }
    }
}

fn default_drift_check_interval() -> u32 {
    10
}

fn default_drift_threshold() -> f64 {
    0.6
}

fn default_plans_dir() -> PathBuf {
    PathBuf::from("plans")
}

/// Registry of available skill chains from terraphim-skills and zestic-engineering-skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillChainRegistry {
    /// Available skills from terraphim-engineering-skills
    pub terraphim_skills: Vec<String>,
    /// Available skills from zestic-engineering-skills
    pub zestic_skills: Vec<String>,
}

impl Default for SkillChainRegistry {
    fn default() -> Self {
        Self {
            terraphim_skills: vec![
                "security-audit".into(),
                "code-review".into(),
                "architecture".into(),
                "implementation".into(),
                "rust-development".into(),
                "testing".into(),
                "debugging".into(),
                "documentation".into(),
                "devops".into(),
                "session-search".into(),
                "local-knowledge".into(),
                "disciplined-research".into(),
                "disciplined-design".into(),
                "disciplined-implementation".into(),
                "disciplined-verification".into(),
                "disciplined-validation".into(),
                "quality-gate".into(),
                "requirements-traceability".into(),
                "acceptance-testing".into(),
                "visual-testing".into(),
                "git-safety-guard".into(),
                "community-engagement".into(),
                "open-source-contribution".into(),
                "rust-performance".into(),
                "md-book".into(),
                "terraphim-hooks".into(),
                "gpui-components".into(),
                "quickwit-log-search".into(),
                "ubs-scanner".into(),
                "disciplined-specification".into(),
                "disciplined-quality-evaluation".into(),
            ],
            zestic_skills: vec![
                "quality-oversight".into(),
                "responsible-ai".into(),
                "insight-synthesis".into(),
                "perspective-investigation".into(),
                "product-vision".into(),
                "wardley-mapping".into(),
                "business-scenario-design".into(),
                "prompt-agent-spec".into(),
                "frontend".into(),
                "cross-platform".into(),
                "rust-mastery".into(),
                "backend-architecture".into(),
                "rapid-prototyping".into(),
                "via-negativa-analysis".into(),
                "strategy-execution".into(),
                "technical-leadership".into(),
            ],
        }
    }
}

impl SkillChainRegistry {
    /// Validate that all skills in the chain exist in the registry
    pub fn validate_chain(&self, chain: &[String]) -> Result<(), Vec<String>> {
        let missing: Vec<String> = chain
            .iter()
            .filter(|s| !self.terraphim_skills.contains(s) && !self.zestic_skills.contains(s))
            .cloned()
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

fn default_banned_providers() -> Vec<String> {
    vec!["opencode".to_string()]
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

    /// Terraphim persona name (e.g., "Ferrox", "Vigil", "Carthos")
    #[serde(default)]
    pub persona_name: Option<String>,

    /// Persona symbol (e.g., "Fe", "Shield-lock", "Compass rose")
    #[serde(default)]
    pub persona_symbol: Option<String>,

    /// Persona vibe/personality (e.g., "Meticulous, zero-waste, compiler-minded")
    #[serde(default)]
    pub persona_vibe: Option<String>,

    /// Meta-cortex connections: agent names this persona naturally collaborates with
    #[serde(default)]
    pub meta_cortex_connections: Vec<String>,

    /// Skill chain: ordered list of skills this agent uses
    #[serde(default)]
    pub skill_chain: Vec<String>,
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

pub fn default_stagger_delay_ms() -> u64 {
    5000
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

    /// Validate all agent skill chains against the registry
    pub fn validate_skill_chains(&self) -> Vec<(String, Vec<String>)> {
        self.agents
            .iter()
            .filter(|a| !a.skill_chain.is_empty())
            .filter_map(|a| {
                self.skill_registry
                    .validate_chain(&a.skill_chain)
                    .err()
                    .map(|missing| (a.name.clone(), missing))
            })
            .collect()
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

    #[test]
    fn test_default_banned_providers() {
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
task = "Test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.banned_providers, vec!["opencode".to_string()]);
        assert!(config.allowed_providers.is_empty());
    }

    #[test]
    fn test_custom_banned_providers() {
        let toml_str = r#"
working_dir = "/tmp"
banned_providers = ["zen", "prohibited"]

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "codex"
task = "Test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(
            config.banned_providers,
            vec!["zen".to_string(), "prohibited".to_string()]
        );
    }

    #[test]
    fn test_allowed_providers() {
        let toml_str = r#"
working_dir = "/tmp"
allowed_providers = ["opencode-go", "kimi-for-coding"]

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "codex"
task = "Test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(
            config.allowed_providers,
            vec!["opencode-go".to_string(), "kimi-for-coding".to_string()]
        );
    }

    #[test]
    fn test_backward_compatible_no_provider_fields() {
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
task = "Legacy task"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.allowed_providers.is_empty());
        assert_eq!(config.banned_providers, vec!["opencode".to_string()]);
    }

    #[test]
    fn test_config_parse_with_persona_fields() {
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
persona_name = "Vigil"
persona_symbol = "Shield-lock"
persona_vibe = "Professionally paranoid, calm under breach"
meta_cortex_connections = ["Ferrox", "Conduit"]
skill_chain = ["security-audit", "code-review", "quality-oversight"]
task = "Run security audit"
capabilities = ["security", "vulnerability-scanning"]
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "security-sentinel");
        assert_eq!(config.agents[0].persona_name, Some("Vigil".to_string()));
        assert_eq!(
            config.agents[0].persona_symbol,
            Some("Shield-lock".to_string())
        );
        assert_eq!(
            config.agents[0].persona_vibe,
            Some("Professionally paranoid, calm under breach".to_string())
        );
        assert_eq!(
            config.agents[0].meta_cortex_connections,
            vec!["Ferrox".to_string(), "Conduit".to_string()]
        );
        assert_eq!(
            config.agents[0].skill_chain,
            vec![
                "security-audit".to_string(),
                "code-review".to_string(),
                "quality-oversight".to_string()
            ]
        );
    }

    #[test]
    fn test_persona_fields_backward_compatible() {
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
task = "Legacy task without persona fields"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "legacy-agent");
        assert!(config.agents[0].persona_name.is_none());
        assert!(config.agents[0].persona_symbol.is_none());
        assert!(config.agents[0].persona_vibe.is_none());
        assert!(config.agents[0].meta_cortex_connections.is_empty());
        assert!(config.agents[0].skill_chain.is_empty());
    }

    #[test]
    fn test_meta_cortex_connections_as_vec() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "connector-agent"
layer = "Core"
cli_tool = "opencode"
persona_name = "Conduit"
meta_cortex_connections = ["Vigil", "Ferrox", "Architect"]
task = "Coordinate between agents"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents[0].meta_cortex_connections.len(), 3);
        assert_eq!(
            config.agents[0].meta_cortex_connections,
            vec![
                "Vigil".to_string(),
                "Ferrox".to_string(),
                "Architect".to_string()
            ]
        );
    }

    #[test]
    fn test_skill_chain_as_vec() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "skilled-agent"
layer = "Growth"
cli_tool = "opencode"
persona_name = "Ferrox"
skill_chain = ["requirements-analysis", "architecture", "implementation", "review"]
task = "Execute full development cycle"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.agents[0].skill_chain.len(), 4);
        assert_eq!(
            config.agents[0].skill_chain,
            vec![
                "requirements-analysis".to_string(),
                "architecture".to_string(),
                "implementation".to_string(),
                "review".to_string()
            ]
        );
    }

    #[test]
    fn test_skill_chain_registry_default_has_expected_skills() {
        let registry = SkillChainRegistry::default();

        // Test terraphim skills
        assert!(registry
            .terraphim_skills
            .contains(&"security-audit".to_string()));
        assert!(registry
            .terraphim_skills
            .contains(&"code-review".to_string()));
        assert!(registry
            .terraphim_skills
            .contains(&"rust-development".to_string()));
        assert!(registry
            .terraphim_skills
            .contains(&"disciplined-research".to_string()));
        assert!(registry
            .terraphim_skills
            .contains(&"ubs-scanner".to_string()));

        // Test zestic skills
        assert!(registry
            .zestic_skills
            .contains(&"quality-oversight".to_string()));
        assert!(registry
            .zestic_skills
            .contains(&"insight-synthesis".to_string()));
        assert!(registry.zestic_skills.contains(&"rust-mastery".to_string()));
        assert!(registry
            .zestic_skills
            .contains(&"strategy-execution".to_string()));
        assert!(registry
            .zestic_skills
            .contains(&"technical-leadership".to_string()));

        // Verify we have the expected counts
        assert_eq!(registry.terraphim_skills.len(), 31);
        assert_eq!(registry.zestic_skills.len(), 16);
    }

    #[test]
    fn test_validate_chain_with_valid_skills() {
        let registry = SkillChainRegistry::default();

        // Valid terraphim skill
        let result = registry.validate_chain(&vec!["security-audit".to_string()]);
        assert!(result.is_ok());

        // Valid zestic skill
        let result = registry.validate_chain(&vec!["quality-oversight".to_string()]);
        assert!(result.is_ok());

        // Mixed valid skills
        let result = registry.validate_chain(&vec![
            "code-review".to_string(),
            "quality-oversight".to_string(),
            "rust-development".to_string(),
        ]);
        assert!(result.is_ok());

        // Empty chain (should be valid - nothing to validate)
        let result = registry.validate_chain(&vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chain_with_unknown_skill() {
        let registry = SkillChainRegistry::default();

        // Single unknown skill
        let result = registry.validate_chain(&vec!["unknown-skill".to_string()]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["unknown-skill".to_string()]);

        // Mix of valid and invalid
        let result = registry.validate_chain(&vec![
            "security-audit".to_string(),
            "unknown-skill".to_string(),
            "also-unknown".to_string(),
        ]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.len(), 2);
        assert!(err.contains(&"unknown-skill".to_string()));
        assert!(err.contains(&"also-unknown".to_string()));
    }

    #[test]
    fn test_validate_skill_chains_across_agents() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[[agents]]
name = "valid-agent"
layer = "Safety"
cli_tool = "codex"
skill_chain = ["security-audit", "code-review"]
task = "Has valid skills"

[[agents]]
name = "invalid-agent"
layer = "Growth"
cli_tool = "opencode"
skill_chain = ["security-audit", "unknown-skill", "also-unknown"]
task = "Has invalid skills"

[[agents]]
name = "empty-chain-agent"
layer = "Core"
cli_tool = "claude"
task = "Has empty skill chain"

[[agents]]
name = "zestic-agent"
layer = "Safety"
cli_tool = "codex"
skill_chain = ["quality-oversight", "insight-synthesis"]
task = "Has zestic skills"
"#;

        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let invalid_chains = config.validate_skill_chains();

        assert_eq!(invalid_chains.len(), 1);
        assert_eq!(invalid_chains[0].0, "invalid-agent");
        assert_eq!(invalid_chains[0].1.len(), 2);
        assert!(invalid_chains[0].1.contains(&"unknown-skill".to_string()));
        assert!(invalid_chains[0].1.contains(&"also-unknown".to_string()));
    }

    #[test]
    fn test_backward_compatible_empty_skill_chain() {
        let registry = SkillChainRegistry::default();

        // Empty skill chain should pass validation
        let result = registry.validate_chain(&vec![]);
        assert!(result.is_ok());

        // Agent with empty skill_chain should be filtered out by validate_skill_chains
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
task = "Has no skill chain"
"#;

        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        assert!(config.agents[0].skill_chain.is_empty());

        // validate_skill_chains should return empty since empty chains are filtered out
        let invalid_chains = config.validate_skill_chains();
        assert!(invalid_chains.is_empty());
    }

    #[test]
    fn test_config_with_skill_registry() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"

[skill_registry]

[[agents]]
name = "agent"
layer = "Safety"
cli_tool = "codex"
task = "Test"
"#;

        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        // Should have default skills loaded
        assert!(!config.skill_registry.terraphim_skills.is_empty());
        assert!(!config.skill_registry.zestic_skills.is_empty());
    }
}
