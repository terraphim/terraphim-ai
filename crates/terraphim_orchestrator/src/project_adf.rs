use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::config::{AgentDefinition, AgentLayer, PrDispatchConfig, PrDispatchEntry, Project};
use crate::OrchestratorError;

impl FromStr for AgentLayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Safety" => Ok(AgentLayer::Safety),
            "Core" => Ok(AgentLayer::Core),
            "Growth" => Ok(AgentLayer::Growth),
            _ => Err(format!(
                "invalid layer '{}'; expected Safety, Core, or Growth",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlProjectAdfConfig {
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub agents: Vec<TomlAdfAgent>,
    #[serde(default)]
    pub pr_dispatch: Option<Vec<TomlPrDispatchEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlAdfAgent {
    pub name: String,
    pub layer: String,
    pub cli_tool: String,
    pub task: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub budget_monthly_cents: Option<u64>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default)]
    pub skill_chain: Vec<String>,
    #[serde(default)]
    pub fallback_provider: Option<String>,
    #[serde(default)]
    pub fallback_model: Option<String>,
    #[serde(default)]
    pub grace_period_secs: Option<u64>,
    #[serde(default)]
    pub max_cpu_seconds: Option<u64>,
    #[serde(default)]
    pub pre_check: Option<crate::config::PreCheckStrategy>,
    #[serde(default)]
    pub gitea_issue: Option<u64>,
    #[serde(default)]
    pub event_only: bool,
    #[serde(default)]
    pub evolution_enabled: bool,
    #[serde(default)]
    pub rlm_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlPrDispatchEntry {
    pub name: String,
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct ProjectAdfConfig {
    pub project_id: String,
    pub name: String,
    pub agents: Vec<TomlAdfAgent>,
    pub pr_dispatch: Option<PrDispatchConfig>,
    pub discovered_path: PathBuf,
}

impl TomlProjectAdfConfig {
    fn expand_env_vars_in_string(s: &str) -> String {
        crate::config::expand_env_vars(s)
    }

    fn try_expand_env_vars(&mut self) {
        let expand = Self::expand_env_vars_in_string;

        self.project_id = expand(&self.project_id);
        self.name = expand(&self.name);

        for agent in &mut self.agents {
            agent.name = expand(&agent.name);
            agent.cli_tool = expand(&agent.cli_tool);
            agent.task = expand(&agent.task);
            if let Some(ref mut m) = agent.model {
                *m = expand(m);
            }
            if let Some(ref mut sc) = agent.schedule {
                *sc = expand(sc);
            }
            for cap in &mut agent.capabilities {
                *cap = expand(cap);
            }
            if let Some(ref mut p) = agent.provider {
                *p = expand(p);
            }
            if let Some(ref mut pers) = agent.persona {
                *pers = expand(pers);
            }
            for sc in &mut agent.skill_chain {
                *sc = expand(sc);
            }
            if let Some(ref mut fp) = agent.fallback_provider {
                *fp = expand(fp);
            }
            if let Some(ref mut fm) = agent.fallback_model {
                *fm = expand(fm);
            }
        }

        if let Some(ref mut entries) = self.pr_dispatch {
            for entry in entries {
                entry.name = expand(&entry.name);
                entry.context = expand(&entry.context);
            }
        }
    }

    fn into_adf_config(
        self,
        discovered_path: PathBuf,
    ) -> Result<ProjectAdfConfig, OrchestratorError> {
        let pr_dispatch = self.pr_dispatch.map(|entries| PrDispatchConfig {
            agents_on_pr_open: entries
                .into_iter()
                .map(|e| PrDispatchEntry {
                    name: e.name,
                    context: e.context,
                })
                .collect(),
        });

        Ok(ProjectAdfConfig {
            project_id: self.project_id,
            name: self.name,
            agents: self.agents,
            pr_dispatch,
            discovered_path,
        })
    }
}

impl ProjectAdfConfig {
    pub fn project_root(&self) -> PathBuf {
        self.discovered_path
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.discovered_path.clone())
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.project_root().join(".terraphim/skills")
    }

    fn discover_terraphim_dir(start_dir: &Path) -> Option<PathBuf> {
        let mut current = Some(start_dir.to_path_buf());

        while let Some(dir) = current {
            let terraphim_dir = dir.join(".terraphim");
            if terraphim_dir.is_dir() {
                if let Ok(canonical) = terraphim_dir.canonicalize() {
                    return Some(canonical);
                }
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }

        None
    }

    pub fn discover_and_load(cwd: &Path) -> Result<Option<Self>, OrchestratorError> {
        let terraphim_dir = match Self::discover_terraphim_dir(cwd) {
            Some(d) => d,
            None => return Ok(None),
        };

        let adf_path = terraphim_dir.join("adf.toml");
        if !adf_path.is_file() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&adf_path).map_err(|e| {
            OrchestratorError::Config(format!("failed to read {}: {}", adf_path.display(), e))
        })?;

        let expanded = crate::config::expand_env_vars(&content);
        let mut toml_config: TomlProjectAdfConfig = toml::from_str(&expanded).map_err(|e| {
            OrchestratorError::Config(format!("failed to parse {}: {}", adf_path.display(), e))
        })?;

        toml_config.try_expand_env_vars();
        toml_config.into_adf_config(adf_path).map(Some)
    }
}

impl TryFrom<&ProjectAdfConfig> for (Project, Vec<AgentDefinition>) {
    type Error = OrchestratorError;

    fn try_from(cfg: &ProjectAdfConfig) -> Result<Self, Self::Error> {
        let project = Project {
            id: cfg.project_id.clone(),
            working_dir: cfg.project_root(),
            schedule_offset_minutes: 0,
            gitea: None,
            mentions: None,
            workflow: None,
            #[cfg(feature = "quickwit")]
            quickwit: None,
            max_concurrent_agents: None,
            max_concurrent_mention_agents: None,
        };

        let agents = cfg
            .agents
            .iter()
            .map(|ta| -> Result<AgentDefinition, OrchestratorError> {
                let layer = AgentLayer::from_str(&ta.layer).map_err(|e| {
                    OrchestratorError::Config(format!(
                        "invalid layer '{}' for agent '{}': {}",
                        ta.layer, ta.name, e
                    ))
                })?;

                Ok(AgentDefinition {
                    name: ta.name.clone(),
                    layer,
                    cli_tool: ta.cli_tool.clone(),
                    task: ta.task.clone(),
                    schedule: ta.schedule.clone(),
                    model: ta.model.clone(),
                    default_tier: None,
                    capabilities: ta.capabilities.clone(),
                    max_memory_bytes: None,
                    budget_monthly_cents: ta.budget_monthly_cents,
                    provider: ta.provider.clone(),
                    persona: ta.persona.clone(),
                    terraphim_role: None,
                    skill_chain: ta.skill_chain.clone(),
                    sfia_skills: vec![],
                    fallback_provider: ta.fallback_provider.clone(),
                    fallback_model: ta.fallback_model.clone(),
                    grace_period_secs: ta.grace_period_secs,
                    max_cpu_seconds: ta.max_cpu_seconds,
                    pre_check: ta.pre_check.clone(),
                    gitea_issue: ta.gitea_issue,
                    event_only: ta.event_only,
                    project: Some(cfg.project_id.clone()),
                    evolution_enabled: ta.evolution_enabled,
                    rlm_enabled: ta.rlm_enabled,
                    bypass_kg_routing: false,
                    enabled: true,
                })
            })
            .collect::<Result<Vec<AgentDefinition>, OrchestratorError>>()?;

        Ok((project, agents))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_project_with_adf(tmp: &TempDir, toml_content: &str) -> PathBuf {
        let project_dir = tmp.path().join("myproject");
        fs::create_dir_all(project_dir.join(".terraphim")).unwrap();
        fs::write(project_dir.join(".terraphim/adf.toml"), toml_content).unwrap();
        project_dir
    }

    #[test]
    fn discover_and_load_parses_valid_file() {
        let tmp = TempDir::new().unwrap();
        let project_dir = temp_project_with_adf(
            &tmp,
            r#"
project_id = "test-project"
name = "Test Project"

[[agents]]
name = "safety-bot"
layer = "Safety"
cli_tool = "echo"
task = "Run safety checks"
"#,
        );

        let result = ProjectAdfConfig::discover_and_load(&project_dir)
            .unwrap()
            .expect("adf.toml must be found");
        assert_eq!(result.project_id, "test-project");
        assert_eq!(result.name, "Test Project");
        assert_eq!(result.project_root(), project_dir.canonicalize().unwrap());
        assert_eq!(
            result.skills_dir(),
            project_dir
                .canonicalize()
                .unwrap()
                .join(".terraphim/skills")
        );
        assert_eq!(result.agents.len(), 1);
        assert_eq!(result.agents[0].name, "safety-bot");
        assert_eq!(result.agents[0].layer, "Safety");
    }

    #[test]
    fn discover_and_load_returns_none_when_no_terraphim_dir() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        let result = ProjectAdfConfig::discover_and_load(&tmp.path().join("src")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn discover_and_load_returns_none_when_no_adf_toml() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".terraphim")).unwrap();
        let result = ProjectAdfConfig::discover_and_load(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn discover_and_load_expands_env_vars() {
        let tmp = TempDir::new().unwrap();
        std::env::set_var("TEST_ADF_PROJECT_ID", "env-project-id");
        let project_dir = temp_project_with_adf(
            &tmp,
            r#"
project_id = "${TEST_ADF_PROJECT_ID}"
name = "Test"
"#,
        );

        let result = ProjectAdfConfig::discover_and_load(&project_dir)
            .unwrap()
            .expect("adf.toml must be found");
        assert_eq!(result.project_id, "env-project-id");
        std::env::remove_var("TEST_ADF_PROJECT_ID");
    }

    #[test]
    fn discover_and_load_parses_pr_dispatch() {
        let tmp = TempDir::new().unwrap();
        let project_dir = temp_project_with_adf(
            &tmp,
            r#"
project_id = "alpha"
name = "Alpha"

[[agents]]
name = "agent-a"
layer = "Safety"
cli_tool = "echo"
task = "a"

[[pr_dispatch]]
name = "agent-a"
context = "adf/build"
"#,
        );

        let result = ProjectAdfConfig::discover_and_load(&project_dir)
            .unwrap()
            .unwrap();
        let dispatch = result.pr_dispatch.expect("pr_dispatch must be present");
        assert_eq!(dispatch.agents_on_pr_open.len(), 1);
        assert_eq!(dispatch.agents_on_pr_open[0].name, "agent-a");
        assert_eq!(dispatch.agents_on_pr_open[0].context, "adf/build");
    }

    #[test]
    fn convert_to_project_and_agents() {
        let tmp = TempDir::new().unwrap();
        let project_dir = temp_project_with_adf(
            &tmp,
            r#"
project_id = "conv-test"
name = "Conversion Test"

[[agents]]
name = "core-agent"
layer = "Core"
cli_tool = "codex"
task = "Do core work"
schedule = "0 3 * * *"
model = "kimi-for-coding/k2p6"
"#,
        );

        let adf = ProjectAdfConfig::discover_and_load(&project_dir)
            .unwrap()
            .unwrap();

        let (project, agents) = (&adf).try_into().expect("conversion must succeed");
        assert_eq!(project.id, "conv-test");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "core-agent");
        assert_eq!(agents[0].layer, AgentLayer::Core);
        assert_eq!(agents[0].project, Some("conv-test".to_string()));
        assert_eq!(agents[0].schedule.as_deref(), Some("0 3 * * *"));
    }

    #[test]
    fn convert_invalid_layer_returns_error() {
        let tmp = TempDir::new().unwrap();
        let project_dir = temp_project_with_adf(
            &tmp,
            r#"
project_id = "bad-layer"
name = "Bad Layer"

[[agents]]
name = "bad-agent"
layer = "Review"
cli_tool = "echo"
task = "task"
"#,
        );

        let adf = ProjectAdfConfig::discover_and_load(&project_dir)
            .unwrap()
            .unwrap();

        let result: Result<(Project, Vec<AgentDefinition>), _> = (&adf).try_into();
        let err = result.expect_err("invalid layer must fail");
        assert!(err.to_string().contains("invalid layer"));
    }

    #[test]
    fn convert_all_layers() {
        for layer_str in &["Safety", "Core", "Growth"] {
            let tmp = TempDir::new().unwrap();
            let project_dir = temp_project_with_adf(
                &tmp,
                &format!(
                    r#"
project_id = "layers"
name = "Layers"

[[agents]]
name = "agent"
layer = "{}"
cli_tool = "echo"
task = "task"
"#,
                    layer_str
                ),
            );

            let adf = ProjectAdfConfig::discover_and_load(&project_dir)
                .unwrap()
                .unwrap();
            let (_, agents) = (&adf).try_into().expect("conversion must succeed");
            let expected = match *layer_str {
                "Safety" => AgentLayer::Safety,
                "Core" => AgentLayer::Core,
                "Growth" => AgentLayer::Growth,
                _ => unreachable!(),
            };
            assert_eq!(
                agents[0].layer, expected,
                "layer {} must parse correctly",
                layer_str
            );
        }
    }

    #[test]
    fn agent_layer_from_str_valid() {
        assert_eq!(AgentLayer::from_str("Safety").unwrap(), AgentLayer::Safety);
        assert_eq!(AgentLayer::from_str("Core").unwrap(), AgentLayer::Core);
        assert_eq!(AgentLayer::from_str("Growth").unwrap(), AgentLayer::Growth);
    }

    #[test]
    fn agent_layer_from_str_invalid() {
        let result = AgentLayer::from_str("Review");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid layer"));
    }

    #[test]
    fn discover_and_load_returns_err_on_malformed_toml() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("proj");
        fs::create_dir_all(project_dir.join(".terraphim")).unwrap();
        fs::write(
            project_dir.join(".terraphim/adf.toml"),
            "not valid toml {{{{",
        )
        .unwrap();

        let result = ProjectAdfConfig::discover_and_load(&project_dir).unwrap_err();
        assert!(result.to_string().contains("failed to parse"));
    }

    #[test]
    fn discover_and_load_returns_err_when_project_id_missing() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("proj");
        fs::create_dir_all(project_dir.join(".terraphim")).unwrap();
        fs::write(
            project_dir.join(".terraphim/adf.toml"),
            r#"
name = "Missing Project Id"
[[agents]]
name = "agent"
layer = "Safety"
cli_tool = "echo"
task = "task"
"#,
        )
        .unwrap();

        let result = ProjectAdfConfig::discover_and_load(&project_dir).unwrap_err();
        assert!(result.to_string().contains("missing field `project_id`"));
    }

    #[test]
    fn full_config_passes_validation() {
        let tmp = TempDir::new().unwrap();
        let project_dir = temp_project_with_adf(
            &tmp,
            r#"
project_id = "validation-test"
name = "Validation Test"

[[agents]]
name = "safety-echo"
layer = "Safety"
cli_tool = "echo"
task = "Say hello"

[[pr_dispatch]]
name = "safety-echo"
context = "adf/build"
"#,
        );

        let adf = ProjectAdfConfig::discover_and_load(&project_dir)
            .unwrap()
            .unwrap();
        let (project, agents) = (&adf).try_into().expect("conversion must succeed");

        let nightwatch = crate::config::NightwatchConfig::default();
        let compound_review = crate::config::CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            repo_path: adf.discovered_path.parent().unwrap().to_path_buf(),
            ..Default::default()
        };

        let mut config = crate::config::OrchestratorConfig {
            working_dir: adf.discovered_path.parent().unwrap().to_path_buf(),
            nightwatch,
            compound_review,
            workflow: None,
            agents,
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            restart_budget_window_secs: 43_200,
            disk_usage_threshold: 90,
            tick_interval_secs: 30,
            gate_reconcile_interval_ticks: 20,
            handoff_buffer_ttl_secs: Some(86400),
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
            projects: vec![project],
            include: vec![],
            providers: vec![],
            provider_budget_state_file: None,
            pause_dir: None,
            project_circuit_breaker_threshold: 5,
            fleet_escalation_owner: None,
            fleet_escalation_repo: None,
            auto_merge: None,
            post_merge_gate: None,
            learning: crate::config::LearningConfig::default(),
            evolution: crate::config::EvolutionConfig::default(),
            pr_dispatch: adf.pr_dispatch,
            pr_dispatch_per_project: std::collections::HashMap::new(),
            gitea_skill_repo: None,
            direct_dispatch: None,
        };

        config.substitute_env_vars();
        config
            .validate()
            .expect("full OrchestratorConfig must validate");
    }
}
