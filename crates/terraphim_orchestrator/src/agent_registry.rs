//! Project-scoped registry for ADF agent definitions.
//!
//! This module is a read-only index over the already merged
//! [`OrchestratorConfig`](crate::config::OrchestratorConfig). It does not load
//! TOML, merge project sources, spawn agents, or post statuses.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::{AgentDefinition, OrchestratorConfig};
use crate::error::OrchestratorError;

/// Scope component for an agent's configured identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentScope {
    /// Legacy single-project mode. No `[[projects]]` are configured.
    Legacy,
    /// Multi-project mode, keyed by `Project.id`.
    Project(String),
}

impl AgentScope {
    /// Build a scope from an optional project id.
    pub fn from_project(project: Option<&str>) -> Self {
        match project {
            Some(project) => Self::Project(project.to_string()),
            None => Self::Legacy,
        }
    }

    /// Human-readable scope name for diagnostics.
    pub fn label(&self) -> &str {
        match self {
            Self::Legacy => "<legacy>",
            Self::Project(project) => project.as_str(),
        }
    }
}

/// Stable key for a registered agent definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentKey {
    pub scope: AgentScope,
    pub name: String,
}

impl AgentKey {
    pub fn new(scope: AgentScope, name: impl Into<String>) -> Self {
        Self {
            scope,
            name: name.into(),
        }
    }

    pub fn project(project: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(AgentScope::Project(project.into()), name)
    }

    pub fn legacy(name: impl Into<String>) -> Self {
        Self::new(AgentScope::Legacy, name)
    }
}

impl std::fmt::Display for AgentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.scope.label(), self.name)
    }
}

/// Source attribution for an agent entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSource {
    /// The entry came from the merged `OrchestratorConfig`.
    ConfigMerged,
}

/// Registry entry for an agent definition.
#[derive(Debug, Clone)]
pub struct RegisteredAgent {
    pub key: AgentKey,
    pub definition: AgentDefinition,
    pub source: AgentSource,
}

impl RegisteredAgent {
    pub fn project_id(&self) -> Option<&str> {
        self.definition.project.as_deref()
    }

    pub fn event_only(&self) -> bool {
        self.definition.event_only
    }
}

/// Read-only index of all effective agents after config merging.
#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    by_key: BTreeMap<AgentKey, RegisteredAgent>,
    by_project: BTreeMap<AgentScope, BTreeSet<String>>,
}

impl AgentRegistry {
    /// Build the registry from the already merged config.
    pub fn from_config(config: &OrchestratorConfig) -> Result<Self, OrchestratorError> {
        let mut registry = Self::default();

        for agent in &config.agents {
            let scope = AgentScope::from_project(agent.project.as_deref());
            let key = AgentKey::new(scope.clone(), agent.name.clone());

            if registry.by_key.contains_key(&key) {
                return Err(OrchestratorError::Config(format!(
                    "duplicate agent '{}' in project '{}'",
                    agent.name,
                    scope.label()
                )));
            }

            registry
                .by_project
                .entry(scope)
                .or_default()
                .insert(agent.name.clone());
            registry.by_key.insert(
                key.clone(),
                RegisteredAgent {
                    key,
                    definition: agent.clone(),
                    source: AgentSource::ConfigMerged,
                },
            );
        }

        Ok(registry)
    }

    /// Number of registered agents.
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Lookup by explicit key.
    pub fn get(&self, key: &AgentKey) -> Option<&RegisteredAgent> {
        self.by_key.get(key)
    }

    /// Lookup a project-scoped agent by project id and name.
    pub fn lookup_project(&self, project: &str, name: &str) -> Option<&RegisteredAgent> {
        self.get(&AgentKey::project(project, name))
    }

    /// Lookup a legacy single-project agent by name.
    pub fn lookup_legacy(&self, name: &str) -> Option<&RegisteredAgent> {
        self.get(&AgentKey::legacy(name))
    }

    /// Lookup with an optional project id, mirroring `AgentDefinition.project`.
    pub fn lookup(&self, project: Option<&str>, name: &str) -> Option<&RegisteredAgent> {
        self.get(&AgentKey::new(AgentScope::from_project(project), name))
    }

    /// List registered agent names for a scope in sorted order.
    pub fn names_for_scope(&self, scope: &AgentScope) -> Vec<&str> {
        self.by_project
            .get(scope)
            .map(|names| names.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentKey, AgentRegistry, AgentScope};
    use crate::config::OrchestratorConfig;
    use crate::error::OrchestratorError;

    fn config_from(toml: &str) -> Result<OrchestratorConfig, OrchestratorError> {
        OrchestratorConfig::from_toml(toml)
    }

    #[test]
    fn registry_builds_legacy_agents() -> Result<(), Box<dyn std::error::Error>> {
        let config = config_from(
            r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[agents]]
name = "legacy-agent"
layer = "Safety"
cli_tool = "echo"
task = "legacy"
"#,
        )?;

        let registry = AgentRegistry::from_config(&config)?;
        let agent = registry.lookup_legacy("legacy-agent").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing legacy-agent")
        })?;
        assert_eq!(agent.key, AgentKey::legacy("legacy-agent"));
        assert_eq!(agent.project_id(), None);
        assert_eq!(
            registry.names_for_scope(&AgentScope::Legacy),
            vec!["legacy-agent"]
        );
        Ok(())
    }

    #[test]
    fn registry_allows_same_name_across_projects() -> Result<(), Box<dyn std::error::Error>> {
        let config = config_from(
            r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[projects]]
id = "beta"
working_dir = "/tmp/beta"

[[agents]]
name = "build-runner"
layer = "Growth"
cli_tool = "echo"
task = "alpha-build"
project = "alpha"
event_only = true

[[agents]]
name = "build-runner"
layer = "Growth"
cli_tool = "echo"
task = "beta-build"
project = "beta"
event_only = true
"#,
        )?;

        let registry = AgentRegistry::from_config(&config)?;
        assert_eq!(registry.len(), 2);
        let alpha_runner = registry
            .lookup_project("alpha", "build-runner")
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "missing alpha build-runner")
            })?;
        let beta_runner = registry
            .lookup_project("beta", "build-runner")
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "missing beta build-runner")
            })?;
        assert_eq!(alpha_runner.definition.task, "alpha-build");
        assert_eq!(beta_runner.definition.task, "beta-build");
        assert!(registry.lookup_legacy("build-runner").is_none());
        Ok(())
    }

    #[test]
    fn registry_rejects_duplicate_agent_in_same_scope() -> Result<(), Box<dyn std::error::Error>> {
        let config = config_from(
            r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[agents]]
name = "dupe"
layer = "Safety"
cli_tool = "echo"
task = "first"

[[agents]]
name = "dupe"
layer = "Safety"
cli_tool = "echo"
task = "second"
"#,
        )?;

        let err = AgentRegistry::from_config(&config)
            .err()
            .ok_or_else(|| std::io::Error::other("expected duplicate agent error"))?;
        assert!(err.to_string().contains("duplicate agent 'dupe'"));
        Ok(())
    }

    #[test]
    fn names_for_scope_returns_sorted_names() -> Result<(), Box<dyn std::error::Error>> {
        let config = config_from(
            r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[agents]]
name = "zeta"
layer = "Safety"
cli_tool = "echo"
task = "z"
project = "alpha"

[[agents]]
name = "alpha"
layer = "Safety"
cli_tool = "echo"
task = "a"
project = "alpha"
"#,
        )?;

        let registry = AgentRegistry::from_config(&config)?;
        assert_eq!(
            registry.names_for_scope(&AgentScope::Project("alpha".to_string())),
            vec!["alpha", "zeta"]
        );
        Ok(())
    }
}
