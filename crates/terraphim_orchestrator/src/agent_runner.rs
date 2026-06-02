use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::{AgentDefinition, OrchestratorConfig};
use crate::{AgentOrchestrator, OrchestratorError};

const LEGACY_PROJECT: &str = "<legacy>";

/// Request parameters for running or validating a named agent.
#[derive(Debug, Clone)]
pub struct AgentRunRequest {
    /// Name of the agent to run, as declared in the orchestrator configuration.
    pub agent_name: String,
    /// Optional project scope. When set, only agents belonging to this project are considered.
    pub project: Option<String>,
}

impl AgentRunRequest {
    /// Create a new run request for the given agent name with no project scope.
    pub fn new(agent_name: impl Into<String>) -> Self {
        Self {
            agent_name: agent_name.into(),
            project: None,
        }
    }

    /// Attach a project scope to this request, restricting resolution to agents in that project.
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }
}

/// Serialisable report describing the Gitea repository target associated with an agent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GiteaTargetReport {
    /// Base URL of the Gitea instance (e.g. `https://git.example.com`).
    pub base_url: String,
    /// Repository owner (organisation or user).
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Optional Gitea issue number linked to this agent.
    pub issue: Option<u64>,
}

/// How an agent execution was initiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    /// Agent was started by a cron schedule.
    Cron,
    /// Agent was triggered by a @-mention in a Gitea issue or PR comment.
    Mention,
    /// Agent was triggered by a git push event.
    Push,
    /// Agent was triggered by a pull-request event.
    PullRequest,
    /// Agent was invoked directly from the local machine.
    Local,
    /// Agent was triggered by an incoming webhook.
    Webhook,
}

/// Result of probing a single trigger mode for a given agent.
#[derive(Debug, Clone, Serialize, Eq)]
pub struct ModeResult {
    /// The trigger mode that was evaluated.
    pub trigger_mode: TriggerMode,
    /// Whether the agent is considered runnable under this mode.
    pub runnable: bool,
    /// Result of the CLI-tool probe, or `None` if the probe was skipped.
    pub cli_tool_probe: Option<bool>,
    /// Result of the model availability probe, or `None` if the probe was skipped.
    pub model_probe: Option<bool>,
    /// Whether a synthetic event could be constructed for this mode, or `None` if not applicable.
    pub synthetic_event_ok: Option<bool>,
    /// Human-readable warnings produced during mode evaluation.
    pub warnings: Vec<String>,
}

impl PartialEq for ModeResult {
    fn eq(&self, other: &Self) -> bool {
        self.runnable == other.runnable
            && self.cli_tool_probe == other.cli_tool_probe
            && self.model_probe == other.model_probe
            && self.synthetic_event_ok == other.synthetic_event_ok
    }
}

/// Aggregated validation report covering all trigger modes for a single agent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentValidationReport {
    /// Name of the validated agent.
    pub agent_name: String,
    /// Project the agent belongs to, or `"<legacy>"` for global agents.
    pub project: String,
    /// Per-mode probe results keyed by [`TriggerMode`].
    pub mode_results: HashMap<TriggerMode, ModeResult>,
    /// `true` when every trigger mode is runnable.
    pub all_modes_runnable: bool,
}

/// A synthetic event used to test an agent's response without a real source-control trigger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyntheticEvent {
    /// Synthetic pull-request event.
    PullRequest {
        /// Pull-request number.
        number: u64,
        /// SHA of the head commit on the PR branch.
        head_sha: String,
        /// Username of the PR author.
        author: String,
        /// PR title.
        title: String,
        /// Approximate diff size in lines of code.
        diff_loc: usize,
    },
    /// Synthetic push event.
    Push {
        /// SHA of the pushed commit.
        sha: String,
        /// Full ref name, e.g. `refs/heads/main`.
        ref_name: String,
        /// Username of the person who pushed.
        pusher: String,
        /// List of files changed by the push.
        files: Vec<String>,
    },
}

impl SyntheticEvent {
    /// Convert this event into a map of environment variables understood by ADF agent runners.
    pub fn env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        match self {
            SyntheticEvent::PullRequest {
                number,
                head_sha,
                author,
                title,
                diff_loc,
            } => {
                vars.insert("ADF_EVENT_TYPE".to_string(), "pull_request".to_string());
                vars.insert("ADF_PR_NUMBER".to_string(), number.to_string());
                vars.insert("ADF_PR_HEAD_SHA".to_string(), head_sha.clone());
                vars.insert("ADF_PR_AUTHOR".to_string(), author.clone());
                vars.insert("ADF_PR_TITLE".to_string(), title.clone());
                vars.insert("ADF_PR_DIFF_LOC".to_string(), diff_loc.to_string());
            }
            SyntheticEvent::Push {
                sha,
                ref_name,
                pusher,
                files,
            } => {
                vars.insert("ADF_EVENT_TYPE".to_string(), "push".to_string());
                vars.insert("ADF_PUSH_SHA".to_string(), sha.clone());
                vars.insert("ADF_PUSH_REF".to_string(), ref_name.clone());
                vars.insert("ADF_PUSH_PUSHER".to_string(), pusher.clone());
                vars.insert("ADF_PUSH_FILES".to_string(), files.join(","));
            }
        }
        vars
    }
}

/// Detailed runtime validation report for a single agent, produced before execution.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentRuntimeValidationReport {
    /// Name of the agent that was validated.
    pub agent_name: String,
    /// Project the agent belongs to, or `"<legacy>"` for global agents.
    pub project: String,
    /// Human-readable label for the agent's execution layer.
    pub layer: String,
    /// Cron schedule expression, if defined for the agent.
    pub schedule: Option<String>,
    /// CLI tool or command used to invoke the agent.
    pub cli_tool: String,
    /// LLM model identifier the agent uses, if configured.
    pub model: Option<String>,
    /// Absolute path to the working directory for this agent.
    pub working_dir: String,
    /// `true` when the working directory exists on disk.
    pub repo_ok: bool,
    /// Gitea repository target resolved for this agent, if available.
    pub gitea_target: Option<GiteaTargetReport>,
    /// `true` when the agent definition has `evolution_enabled` set.
    pub evolution_requested: bool,
    /// `true` when both the global evolution feature and the agent's flag are enabled.
    pub evolution_available: bool,
    /// `true` when all preconditions for running the agent are satisfied.
    pub runnable: bool,
    /// Result of probing whether the CLI tool binary is present and executable.
    pub cli_tool_probe: Option<bool>,
    /// Result of probing whether the configured model is available.
    pub model_probe: Option<bool>,
    /// Human-readable warnings produced during validation.
    pub warnings: Vec<String>,
}

impl AgentOrchestrator {
    /// Validate the runtime environment for the agent described by `request`.
    pub fn validate_agent_runtime(
        &self,
        request: &AgentRunRequest,
    ) -> Result<AgentRuntimeValidationReport, OrchestratorError> {
        validate_agent_runtime(self.config(), request)
    }
}

/// Validate the runtime environment for an agent using the provided configuration.
///
/// Resolves the agent by name (and optional project), checks whether its working
/// directory exists, probes the CLI tool and model, and collects any warnings.
pub fn validate_agent_runtime(
    config: &OrchestratorConfig,
    request: &AgentRunRequest,
) -> Result<AgentRuntimeValidationReport, OrchestratorError> {
    let agent = resolve_agent(config, request)?;
    let project_id = agent
        .project
        .as_deref()
        .unwrap_or(LEGACY_PROJECT)
        .to_string();
    let project = agent
        .project
        .as_deref()
        .map(|id| {
            config
                .project_by_id(id)
                .ok_or_else(|| OrchestratorError::UnknownAgentProject {
                    agent: agent.name.clone(),
                    project: id.to_string(),
                })
        })
        .transpose()?;

    let working_dir = config.working_dir_for_agent(agent);
    let repo_ok = working_dir.is_dir();
    let mut warnings = Vec::new();
    if !repo_ok {
        warnings.push(format!(
            "working directory does not exist: {}",
            working_dir.display()
        ));
    }
    if !agent.enabled {
        warnings.push("agent is disabled".to_string());
    }
    if agent.event_only {
        warnings
            .push("agent is event-only and direct runs should use trigger commands".to_string());
    }
    if agent.cli_tool.trim().is_empty() {
        warnings.push("agent cli_tool is empty".to_string());
    }

    let gitea = project
        .and_then(|p| p.gitea.as_ref())
        .or(config.gitea.as_ref())
        .map(|target| GiteaTargetReport {
            base_url: target.base_url.clone(),
            owner: target.owner.clone(),
            repo: target.repo.clone(),
            issue: agent.gitea_issue,
        });

    let cli_tool_probe = if !agent.cli_tool.trim().is_empty() {
        Some(probe_cli_tool(&agent.cli_tool).unwrap_or(false))
    } else {
        None
    };

    let model_probe = agent
        .model
        .as_ref()
        .map(|m| probe_model_available(m, agent.provider.as_deref()).unwrap_or(false));

    let runnable = repo_ok
        && agent.enabled
        && !agent.cli_tool.trim().is_empty()
        && cli_tool_probe.unwrap_or(false);

    Ok(AgentRuntimeValidationReport {
        agent_name: agent.name.clone(),
        project: project_id,
        layer: format!("{:?}", agent.layer),
        schedule: agent.schedule.clone(),
        cli_tool: agent.cli_tool.clone(),
        model: agent.model.clone(),
        working_dir: working_dir.display().to_string(),
        repo_ok,
        gitea_target: gitea,
        evolution_requested: agent.evolution_enabled,
        evolution_available: config.evolution.enabled && agent.evolution_enabled,
        runnable,
        cli_tool_probe,
        model_probe,
        warnings,
    })
}

/// Probe whether the CLI tool specified in an agent definition is available and executable.
///
/// Absolute paths are checked for existence and execute permission (on Unix).
/// Relative or bare names are resolved via `which`.  Returns `Ok(false)` for
/// empty strings rather than an error.
pub fn probe_cli_tool(cli_tool: &str) -> Result<bool, OrchestratorError> {
    if cli_tool.trim().is_empty() {
        return Ok(false);
    }
    let cmd = if cli_tool.contains(' ') {
        cli_tool.split_whitespace().next()
    } else {
        Some(cli_tool)
    };
    let Some(executable) = cmd else {
        return Ok(false);
    };
    let path = Path::new(executable);
    if path.is_absolute() {
        Ok(path.exists() && {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                path.metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
            }
            #[cfg(not(unix))]
            {
                true
            }
        })
    } else {
        Ok(which::which(executable).is_ok())
    }
}

/// Probe whether the given model identifier is available under a subscription-based provider.
///
/// Currently recognises `kimi-`, `minimax-`, `glm-`, and `zai-` prefixes as available
/// subscription models.  Unknown or empty model strings return `Ok(false)`.
pub fn probe_model_available(
    model: &str,
    _provider: Option<&str>,
) -> Result<bool, OrchestratorError> {
    if model.trim().is_empty() {
        return Ok(false);
    }
    if model.starts_with("kimi-")
        || model.starts_with("minimax-")
        || model.contains("glm-")
        || model.starts_with("zai-")
    {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Attempt to run an agent with a synthetic event for testing purposes.
///
/// This function is not yet implemented and always returns a `ModeResult` with
/// `runnable: false` and a warning indicating the stub status.
pub fn run_agent_synthetic(
    _config: &OrchestratorConfig,
    _request: &AgentRunRequest,
    _event: SyntheticEvent,
) -> Result<ModeResult, OrchestratorError> {
    Ok(ModeResult {
        trigger_mode: TriggerMode::PullRequest,
        runnable: false,
        cli_tool_probe: None,
        model_probe: None,
        synthetic_event_ok: Some(false),
        warnings: vec!["run_agent_synthetic not yet implemented".to_string()],
    })
}

fn resolve_agent<'a>(
    config: &'a OrchestratorConfig,
    request: &AgentRunRequest,
) -> Result<&'a AgentDefinition, OrchestratorError> {
    let matches = config
        .agents
        .iter()
        .filter(|agent| agent.name == request.agent_name)
        .filter(|agent| {
            request
                .project
                .as_deref()
                .map_or(true, |project| agent.project.as_deref() == Some(project))
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [agent] => Ok(*agent),
        [] => Err(OrchestratorError::AgentNotFound(request.agent_name.clone())),
        _ => Err(OrchestratorError::Config(format!(
            "agent '{}' exists in multiple projects; pass --project",
            request.agent_name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AgentLayer, CompoundReviewConfig, EvolutionConfig, GiteaOutputConfig, LearningConfig,
        NightwatchConfig, Project,
    };
    use tempfile::TempDir;

    fn agent(name: &str, project: Option<&str>) -> AgentDefinition {
        AgentDefinition {
            name: name.to_string(),
            layer: AgentLayer::Core,
            cli_tool: "echo".to_string(),
            task: "hello".to_string(),
            schedule: Some("0 2 * * *".to_string()),
            model: Some("minimax-coding-plan/MiniMax-M2.7-highspeed".to_string()),
            default_tier: None,
            capabilities: vec!["build".to_string()],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: Some("opencode".to_string()),
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
            event_only: false,
            project: project.map(str::to_string),
            evolution_enabled: false,
            rlm_enabled: None,
            bypass_kg_routing: false,
            enabled: true,
        }
    }

    fn config(working_dir: &std::path::Path) -> OrchestratorConfig {
        OrchestratorConfig {
            working_dir: working_dir.to_path_buf(),
            nightwatch: NightwatchConfig::default(),
            compound_review: CompoundReviewConfig {
                schedule: "0 2 * * *".to_string(),
                repo_path: working_dir.to_path_buf(),
                ..Default::default()
            },
            workflow: None,
            agents: vec![],
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            restart_budget_window_secs: 43_200,
            disk_usage_threshold: 90,
            tick_interval_secs: 30,
            gate_reconcile_interval_ticks: 20,
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
            providers: vec![],
            provider_budget_state_file: None,
            pause_dir: None,
            project_circuit_breaker_threshold: 3,
            fleet_escalation_owner: None,
            fleet_escalation_repo: None,
            post_merge_gate: None,
            learning: LearningConfig::default(),
            evolution: EvolutionConfig::default(),
            pr_dispatch: None,
            pr_dispatch_per_project: std::collections::HashMap::new(),
            gitea_skill_repo: None,
            direct_dispatch: None,
        }
    }

    #[test]
    fn validate_global_agent_runtime() -> Result<(), OrchestratorError> {
        let tmp = TempDir::new()?;
        let mut config = config(tmp.path());
        config.agents.push(agent("builder", None));

        let report = validate_agent_runtime(&config, &AgentRunRequest::new("builder"))?;

        assert_eq!(report.agent_name, "builder");
        assert_eq!(report.project, LEGACY_PROJECT);
        assert_eq!(report.cli_tool, "echo");
        assert_eq!(
            report.model.as_deref(),
            Some("minimax-coding-plan/MiniMax-M2.7-highspeed")
        );
        assert!(report.repo_ok);
        assert!(report.runnable);
        Ok(())
    }

    #[test]
    fn validate_project_agent_runtime() -> Result<(), OrchestratorError> {
        let top = TempDir::new()?;
        let project = TempDir::new()?;
        let mut config = config(top.path());
        config.projects.push(Project {
            id: "terraphim".to_string(),
            working_dir: project.path().to_path_buf(),
            schedule_offset_minutes: 0,
            gitea: Some(GiteaOutputConfig {
                base_url: "https://git.terraphim.cloud".to_string(),
                token: "redacted-in-debug".to_string(),
                owner: "terraphim".to_string(),
                repo: "terraphim-ai".to_string(),
                agent_tokens_path: None,
            }),
            mentions: None,
            workflow: None,
            #[cfg(feature = "quickwit")]
            quickwit: None,
            max_concurrent_agents: None,
            max_concurrent_mention_agents: None,
        });
        let mut project_agent = agent("builder", Some("terraphim"));
        project_agent.gitea_issue = Some(42);
        config.agents.push(project_agent);

        let report = validate_agent_runtime(
            &config,
            &AgentRunRequest::new("builder").with_project("terraphim"),
        )?;

        assert_eq!(report.project, "terraphim");
        assert_eq!(report.working_dir, project.path().display().to_string());
        assert_eq!(
            report.gitea_target.as_ref().map(|t| t.issue),
            Some(Some(42))
        );
        assert!(report.runnable);
        Ok(())
    }

    #[test]
    fn validate_evolution_flags() -> Result<(), OrchestratorError> {
        let tmp = TempDir::new()?;
        let mut config = config(tmp.path());
        config.evolution.enabled = true;
        let mut def = agent("evolver", None);
        def.evolution_enabled = true;
        config.agents.push(def);

        let report = validate_agent_runtime(&config, &AgentRunRequest::new("evolver"))?;

        assert!(report.evolution_requested);
        assert!(report.evolution_available);
        Ok(())
    }

    #[test]
    fn validate_missing_project_agent_fails() {
        let tmp = TempDir::new().expect("temp dir");
        let mut config = config(tmp.path());
        config.agents.push(agent("orphan", Some("missing")));

        let err = validate_agent_runtime(&config, &AgentRunRequest::new("orphan"))
            .expect_err("missing project should fail");

        assert!(matches!(err, OrchestratorError::UnknownAgentProject { .. }));
    }

    #[test]
    fn probe_cli_tool_existing() {
        assert!(probe_cli_tool("echo").unwrap_or(false));
    }

    #[test]
    fn probe_cli_tool_missing() {
        assert!(!probe_cli_tool("/nonexistent-binary-xyz").unwrap_or(true));
    }

    #[test]
    fn probe_model_available_kimi() {
        assert!(probe_model_available("kimi-for-coding/k2p6", None).unwrap_or(false));
    }

    #[test]
    fn probe_model_available_minimax() {
        assert!(probe_model_available("minimax-coding-plan/MiniMax-M2.5", None).unwrap_or(false));
    }

    #[test]
    fn probe_model_available_glm() {
        assert!(probe_model_available("zai-coding-plan/glm-5.1", None).unwrap_or(false));
    }

    #[test]
    fn probe_model_available_unknown() {
        assert!(!probe_model_available("unknown-model", None).unwrap_or(true));
    }

    #[test]
    fn synthetic_pr_env_vars() {
        let event = SyntheticEvent::PullRequest {
            number: 42,
            head_sha: "abc123".to_string(),
            author: "testuser".to_string(),
            title: "Test PR".to_string(),
            diff_loc: 500,
        };
        let vars = event.env_vars();
        assert_eq!(
            vars.get("ADF_EVENT_TYPE").map(String::as_str),
            Some("pull_request")
        );
        assert_eq!(vars.get("ADF_PR_NUMBER").map(String::as_str), Some("42"));
        assert_eq!(
            vars.get("ADF_PR_HEAD_SHA").map(String::as_str),
            Some("abc123")
        );
        assert_eq!(
            vars.get("ADF_PR_AUTHOR").map(String::as_str),
            Some("testuser")
        );
        assert_eq!(
            vars.get("ADF_PR_TITLE").map(String::as_str),
            Some("Test PR")
        );
        assert_eq!(vars.get("ADF_PR_DIFF_LOC").map(String::as_str), Some("500"));
    }

    #[test]
    fn synthetic_push_env_vars() {
        let event = SyntheticEvent::Push {
            sha: "def456".to_string(),
            ref_name: "refs/heads/main".to_string(),
            pusher: "devuser".to_string(),
            files: vec!["src/lib.rs".to_string(), "Cargo.toml".to_string()],
        };
        let vars = event.env_vars();
        assert_eq!(vars.get("ADF_EVENT_TYPE").map(String::as_str), Some("push"));
        assert_eq!(vars.get("ADF_PUSH_SHA").map(String::as_str), Some("def456"));
        assert_eq!(
            vars.get("ADF_PUSH_REF").map(String::as_str),
            Some("refs/heads/main")
        );
        assert_eq!(
            vars.get("ADF_PUSH_PUSHER").map(String::as_str),
            Some("devuser")
        );
        assert_eq!(
            vars.get("ADF_PUSH_FILES").map(String::as_str),
            Some("src/lib.rs,Cargo.toml")
        );
    }

    #[test]
    fn validate_with_cli_probe() {
        let tmp = TempDir::new().expect("temp dir");
        let mut config = config(tmp.path());
        config.agents.push(agent("builder", None));

        let report = validate_agent_runtime(&config, &AgentRunRequest::new("builder")).unwrap();

        assert!(report.cli_tool_probe.is_some());
        assert!(report.model_probe.is_some());
        assert!(report.runnable);
    }
}
