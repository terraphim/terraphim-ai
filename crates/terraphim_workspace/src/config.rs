//! Configuration types for workspace management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for workspace management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Root directory for all workspaces.
    pub root: PathBuf,
    /// Hook scripts to run during workspace lifecycle.
    #[serde(default)]
    pub hooks: HooksConfig,
    /// Timeout for hook execution in milliseconds.
    #[serde(default = "default_hook_timeout_ms")]
    pub hook_timeout_ms: u64,
}

impl WorkspaceConfig {
    /// Create a new workspace config with the given root.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            hooks: HooksConfig::default(),
            hook_timeout_ms: default_hook_timeout_ms(),
        }
    }

    /// Set the hooks configuration.
    pub fn with_hooks(mut self, hooks: HooksConfig) -> Self {
        self.hooks = hooks;
        self
    }

    /// Set the hook timeout in milliseconds.
    pub fn with_hook_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.hook_timeout_ms = timeout_ms;
        self
    }
}

fn default_hook_timeout_ms() -> u64 {
    60000 // 60 seconds
}

/// Hook scripts that run during workspace lifecycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfig {
    /// Script to run after workspace creation.
    /// If this fails, the workspace is removed.
    pub after_create: Option<String>,
    /// Script to run before agent execution.
    /// If this fails, the agent run is aborted.
    pub before_run: Option<String>,
    /// Script to run after agent execution.
    /// Failures are logged but ignored.
    pub after_run: Option<String>,
    /// Script to run before workspace removal.
    /// Failures are logged but ignored.
    pub before_remove: Option<String>,
}

impl HooksConfig {
    /// Create a new empty hooks config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the after_create hook.
    pub fn with_after_create(mut self, script: impl Into<String>) -> Self {
        self.after_create = Some(script.into());
        self
    }

    /// Set the before_run hook.
    pub fn with_before_run(mut self, script: impl Into<String>) -> Self {
        self.before_run = Some(script.into());
        self
    }

    /// Set the after_run hook.
    pub fn with_after_run(mut self, script: impl Into<String>) -> Self {
        self.after_run = Some(script.into());
        self
    }

    /// Set the before_remove hook.
    pub fn with_before_remove(mut self, script: impl Into<String>) -> Self {
        self.before_remove = Some(script.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn workspace_config_default_timeout() {
        let config = WorkspaceConfig::new(PathBuf::from("/tmp/work"));
        assert_eq!(config.hook_timeout_ms, 60000);
    }

    #[test]
    fn workspace_config_builder() {
        let config = WorkspaceConfig::new(PathBuf::from("/tmp/work"))
            .with_hook_timeout_ms(30000)
            .with_hooks(
                HooksConfig::new()
                    .with_after_create("echo created")
                    .with_before_run("echo starting"),
            );

        assert_eq!(config.hook_timeout_ms, 30000);
        assert_eq!(config.hooks.after_create, Some("echo created".into()));
        assert_eq!(config.hooks.before_run, Some("echo starting".into()));
        assert!(config.hooks.after_run.is_none());
    }

    #[test]
    fn hooks_config_builder() {
        let hooks = HooksConfig::new()
            .with_after_create("touch created")
            .with_before_run("echo start")
            .with_after_run("echo done")
            .with_before_remove("echo cleanup");

        assert_eq!(hooks.after_create, Some("touch created".into()));
        assert_eq!(hooks.before_run, Some("echo start".into()));
        assert_eq!(hooks.after_run, Some("echo done".into()));
        assert_eq!(hooks.before_remove, Some("echo cleanup".into()));
    }
}
