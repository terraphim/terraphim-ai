//! Configuration layer for Symphony.
//!
//! Provides typed access to values from `WORKFLOW.md` front matter with
//! defaults, environment variable resolution, and path expansion.

pub mod template;
pub mod workflow;

#[cfg(feature = "file-watch")]
pub mod watcher;

use crate::error::{Result, SymphonyError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use workflow::WorkflowDefinition;

/// Typed view of the workflow configuration.
///
/// Provides typed getters for all values defined in the WORKFLOW.md front matter.
/// Handles defaults, `$VAR` environment variable indirection, and `~` expansion.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    workflow: WorkflowDefinition,
}

impl ServiceConfig {
    /// Load configuration from a WORKFLOW.md file path.
    pub fn load(path: &Path) -> Result<Self> {
        let workflow = WorkflowDefinition::load(path)?;
        Ok(Self { workflow })
    }

    /// Create configuration from already-parsed workflow content.
    pub fn from_workflow(workflow: WorkflowDefinition) -> Self {
        Self { workflow }
    }

    /// Get the raw prompt template body.
    pub fn prompt_template(&self) -> &str {
        &self.workflow.prompt_template
    }

    /// Get the raw workflow definition.
    pub fn workflow(&self) -> &WorkflowDefinition {
        &self.workflow
    }

    // --- Tracker ---

    /// Tracker kind (e.g. "linear", "gitea"). Required for dispatch.
    pub fn tracker_kind(&self) -> Option<String> {
        self.get_str(&["tracker", "kind"])
    }

    /// Tracker API endpoint URL.
    pub fn tracker_endpoint(&self) -> String {
        self.get_str(&["tracker", "endpoint"])
            .unwrap_or_else(|| match self.tracker_kind().as_deref() {
                Some("linear") => "https://api.linear.app/graphql".into(),
                Some("gitea") => std::env::var("GITEA_URL")
                    .unwrap_or_else(|_| "https://git.terraphim.cloud".into()),
                _ => String::new(),
            })
    }

    /// Tracker API key, with `$VAR` resolution.
    pub fn tracker_api_key(&self) -> Option<String> {
        let raw = self.get_str(&["tracker", "api_key"])?;
        let resolved = resolve_env_var(&raw);
        if resolved.is_empty() { None } else { Some(resolved) }
    }

    /// Tracker project slug (required for Linear).
    pub fn tracker_project_slug(&self) -> Option<String> {
        self.get_str(&["tracker", "project_slug"])
    }

    /// Gitea owner (for gitea tracker kind).
    pub fn tracker_gitea_owner(&self) -> Option<String> {
        self.get_str(&["tracker", "owner"])
    }

    /// Gitea repo (for gitea tracker kind).
    pub fn tracker_gitea_repo(&self) -> Option<String> {
        self.get_str(&["tracker", "repo"])
    }

    /// Active issue states (issues eligible for dispatch).
    pub fn active_states(&self) -> Vec<String> {
        self.get_str_list(&["tracker", "active_states"])
            .unwrap_or_else(|| vec!["Todo".into(), "In Progress".into()])
    }

    /// Terminal issue states (issues considered done/cancelled).
    pub fn terminal_states(&self) -> Vec<String> {
        self.get_str_list(&["tracker", "terminal_states"])
            .unwrap_or_else(|| {
                vec![
                    "Closed".into(),
                    "Cancelled".into(),
                    "Canceled".into(),
                    "Duplicate".into(),
                    "Done".into(),
                ]
            })
    }

    // --- Polling ---

    /// Polling interval in milliseconds.
    pub fn poll_interval_ms(&self) -> u64 {
        self.get_u64(&["polling", "interval_ms"]).unwrap_or(30_000)
    }

    // --- Workspace ---

    /// Workspace root directory, with `~` and `$VAR` expansion.
    pub fn workspace_root(&self) -> PathBuf {
        if let Some(raw) = self.get_str(&["workspace", "root"]) {
            expand_path(&raw)
        } else {
            std::env::temp_dir().join("symphony_workspaces")
        }
    }

    // --- Hooks ---

    /// Shell script to run after workspace creation.
    pub fn hooks_after_create(&self) -> Option<String> {
        self.get_str(&["hooks", "after_create"])
    }

    /// Shell script to run before each agent attempt.
    pub fn hooks_before_run(&self) -> Option<String> {
        self.get_str(&["hooks", "before_run"])
    }

    /// Shell script to run after each agent attempt.
    pub fn hooks_after_run(&self) -> Option<String> {
        self.get_str(&["hooks", "after_run"])
    }

    /// Shell script to run before workspace removal.
    pub fn hooks_before_remove(&self) -> Option<String> {
        self.get_str(&["hooks", "before_remove"])
    }

    /// Hook timeout in milliseconds.
    pub fn hooks_timeout_ms(&self) -> u64 {
        let val = self.get_u64(&["hooks", "timeout_ms"]).unwrap_or(60_000);
        if val == 0 { 60_000 } else { val }
    }

    // --- Agent ---

    /// Maximum concurrent agent sessions.
    pub fn max_concurrent_agents(&self) -> usize {
        self.get_u64(&["agent", "max_concurrent_agents"])
            .unwrap_or(10) as usize
    }

    /// Maximum turns per agent session.
    pub fn max_turns(&self) -> u32 {
        self.get_u64(&["agent", "max_turns"]).unwrap_or(20) as u32
    }

    /// Maximum retry backoff in milliseconds.
    pub fn max_retry_backoff_ms(&self) -> u64 {
        self.get_u64(&["agent", "max_retry_backoff_ms"])
            .unwrap_or(300_000)
    }

    /// Per-state concurrency limits.
    pub fn max_concurrent_agents_by_state(&self) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        if let Some(val) = self.get_value(&["agent", "max_concurrent_agents_by_state"]) {
            if let Some(mapping) = val.as_mapping() {
                for (k, v) in mapping {
                    if let (Some(state), Some(limit)) = (k.as_str(), v.as_u64()) {
                        if limit > 0 {
                            map.insert(state.to_lowercase(), limit as usize);
                        }
                    }
                }
            }
        }
        map
    }

    // --- Runner ---

    /// Runner kind: `"codex"` (default) or `"claude-code"`.
    ///
    /// Determines which agent session type to use for dispatching issues.
    pub fn runner_kind(&self) -> String {
        self.get_str(&["agent", "runner"])
            .unwrap_or_else(|| "codex".into())
    }

    /// Additional CLI flags for the Claude Code runner (e.g.
    /// `"--dangerously-skip-permissions --max-turns 10"`).
    pub fn claude_flags(&self) -> Option<String> {
        self.get_str(&["agent", "claude_flags"])
    }

    /// Path to a Claude Code settings JSON file or inline JSON string.
    ///
    /// Passed as `--settings <value>` to `claude -p`. Use this to configure
    /// hooks (PreToolUse, PostToolUse), permissions, MCP servers, and other
    /// Claude Code settings that should apply to agent sessions.
    pub fn claude_settings(&self) -> Option<String> {
        self.get_str(&["agent", "settings"])
    }

    // --- Codex ---

    /// Coding-agent command to execute.
    pub fn codex_command(&self) -> String {
        self.get_str(&["codex", "command"])
            .unwrap_or_else(|| "codex app-server".into())
    }

    /// Turn timeout in milliseconds.
    pub fn codex_turn_timeout_ms(&self) -> u64 {
        self.get_u64(&["codex", "turn_timeout_ms"])
            .unwrap_or(3_600_000)
    }

    /// Read timeout in milliseconds (for request/response during startup).
    pub fn codex_read_timeout_ms(&self) -> u64 {
        self.get_u64(&["codex", "read_timeout_ms"]).unwrap_or(5_000)
    }

    /// Stall timeout in milliseconds. `<= 0` disables stall detection.
    pub fn codex_stall_timeout_ms(&self) -> i64 {
        self.get_i64(&["codex", "stall_timeout_ms"])
            .unwrap_or(300_000)
    }

    /// Codex approval policy (pass-through to app-server).
    pub fn codex_approval_policy(&self) -> Option<String> {
        self.get_str(&["codex", "approval_policy"])
    }

    /// Codex thread sandbox mode (pass-through to app-server).
    pub fn codex_thread_sandbox(&self) -> Option<String> {
        self.get_str(&["codex", "thread_sandbox"])
    }

    /// Codex turn sandbox policy (pass-through to app-server).
    pub fn codex_turn_sandbox_policy(&self) -> Option<String> {
        self.get_str(&["codex", "turn_sandbox_policy"])
    }

    // --- Optional HTTP Server (extension) ---

    /// Server port for the optional HTTP API.
    pub fn server_port(&self) -> Option<u16> {
        self.get_u64(&["server", "port"]).map(|p| p as u16)
    }

    // --- Dispatch Preflight Validation ---

    /// Validate the configuration before dispatch.
    ///
    /// Returns `Ok(())` if valid, or `Err(ValidationFailed)` with
    /// a list of problems.
    pub fn validate_for_dispatch(&self) -> Result<()> {
        let mut checks = Vec::new();

        if self.tracker_kind().is_none() {
            checks.push("tracker.kind is required".into());
        }

        match self.tracker_kind().as_deref() {
            Some("linear") => {
                if self.tracker_api_key().is_none()
                    && std::env::var("LINEAR_API_KEY")
                        .ok()
                        .filter(|v| !v.is_empty())
                        .is_none()
                {
                    checks.push(
                        "tracker.api_key or LINEAR_API_KEY environment variable is required"
                            .into(),
                    );
                }
                if self.tracker_project_slug().is_none() {
                    checks.push("tracker.project_slug is required for linear".into());
                }
            }
            Some("gitea") => {
                if self.tracker_api_key().is_none()
                    && std::env::var("GITEA_TOKEN")
                        .ok()
                        .filter(|v| !v.is_empty())
                        .is_none()
                {
                    checks.push(
                        "tracker.api_key or GITEA_TOKEN environment variable is required"
                            .into(),
                    );
                }
                if self.tracker_gitea_owner().is_none() {
                    checks.push("tracker.owner is required for gitea".into());
                }
                if self.tracker_gitea_repo().is_none() {
                    checks.push("tracker.repo is required for gitea".into());
                }
            }
            Some(kind) => {
                checks.push(format!("unsupported tracker.kind: {kind}"));
            }
            None => {} // Already caught above
        }

        match self.runner_kind().as_str() {
            "codex" => {
                if self.codex_command().is_empty() {
                    checks.push("codex.command must not be empty".into());
                }
            }
            "claude-code" => {
                // No codex.command needed; claude CLI is invoked directly.
            }
            other => {
                checks.push(format!("unsupported agent.runner: {other}"));
            }
        }

        if checks.is_empty() {
            Ok(())
        } else {
            Err(SymphonyError::ValidationFailed { checks })
        }
    }

    // --- Internal helpers ---

    /// Navigate a path of keys into the YAML config mapping, returning a cloned value.
    fn get_value(&self, path: &[&str]) -> Option<serde_yaml::Value> {
        let mut current = serde_yaml::Value::Mapping(self.workflow.config.clone());
        for key in path {
            current = current.as_mapping()?.get(*key)?.clone();
        }
        Some(current)
    }

    fn get_str(&self, path: &[&str]) -> Option<String> {
        let val = self.get_value(path)?;
        val.as_str().map(|s| s.to_string())
    }

    fn get_u64(&self, path: &[&str]) -> Option<u64> {
        let val = self.get_value(path)?;
        val.as_u64().or_else(|| {
            val.as_str()
                .and_then(|s| s.parse::<u64>().ok())
        })
    }

    fn get_i64(&self, path: &[&str]) -> Option<i64> {
        let val = self.get_value(path)?;
        val.as_i64().or_else(|| {
            val.as_str()
                .and_then(|s| s.parse::<i64>().ok())
        })
    }

    fn get_str_list(&self, path: &[&str]) -> Option<Vec<String>> {
        let val = self.get_value(path)?;
        let seq = val.as_sequence()?;
        Some(
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        )
    }
}

/// Resolve `$VAR_NAME` references in a string to environment variable values.
fn resolve_env_var(value: &str) -> String {
    if let Some(var_name) = value.strip_prefix('$') {
        std::env::var(var_name).unwrap_or_default()
    } else {
        value.to_string()
    }
}

/// Expand `~` and `$VAR` in a path string.
fn expand_path(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix('~') {
        if let Some(home) = dirs_home(rest) {
            home
        } else {
            PathBuf::from(raw)
        }
    } else if raw.starts_with('$') {
        PathBuf::from(resolve_env_var(raw))
    } else {
        PathBuf::from(raw)
    }
}

/// Expand `~` to the user's home directory.
fn dirs_home(rest: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let mut path = PathBuf::from(home);
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    if !rest.is_empty() {
        path.push(rest);
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_from_yaml(yaml: &str) -> ServiceConfig {
        let content = format!("---\n{yaml}\n---\nPrompt body.");
        let workflow = WorkflowDefinition::parse(&content).unwrap();
        ServiceConfig::from_workflow(workflow)
    }

    #[test]
    fn default_poll_interval() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.poll_interval_ms(), 30_000);
    }

    #[test]
    fn custom_poll_interval() {
        let cfg = config_from_yaml("polling:\n  interval_ms: 5000");
        assert_eq!(cfg.poll_interval_ms(), 5_000);
    }

    #[test]
    fn string_integer_poll_interval() {
        let cfg = config_from_yaml("polling:\n  interval_ms: \"10000\"");
        assert_eq!(cfg.poll_interval_ms(), 10_000);
    }

    #[test]
    fn default_active_states() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.active_states(), vec!["Todo", "In Progress"]);
    }

    #[test]
    fn custom_active_states() {
        let cfg = config_from_yaml(
            "tracker:\n  active_states:\n    - Backlog\n    - Started",
        );
        assert_eq!(cfg.active_states(), vec!["Backlog", "Started"]);
    }

    #[test]
    fn default_terminal_states() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        let states = cfg.terminal_states();
        assert!(states.contains(&"Done".to_string()));
        assert!(states.contains(&"Closed".to_string()));
        assert!(states.contains(&"Cancelled".to_string()));
    }

    #[test]
    fn default_max_concurrent() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.max_concurrent_agents(), 10);
    }

    #[test]
    fn custom_max_concurrent() {
        let cfg = config_from_yaml("agent:\n  max_concurrent_agents: 3");
        assert_eq!(cfg.max_concurrent_agents(), 3);
    }

    #[test]
    fn default_codex_command() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.codex_command(), "codex app-server");
    }

    #[test]
    fn custom_codex_command() {
        let cfg = config_from_yaml("codex:\n  command: \"claude --agent\"");
        assert_eq!(cfg.codex_command(), "claude --agent");
    }

    #[test]
    fn default_workspace_root_is_temp() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        let root = cfg.workspace_root();
        assert!(root.to_str().unwrap().contains("symphony_workspaces"));
    }

    #[test]
    fn hooks_timeout_default() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.hooks_timeout_ms(), 60_000);
    }

    #[test]
    fn hooks_timeout_zero_uses_default() {
        let cfg = config_from_yaml("hooks:\n  timeout_ms: 0");
        assert_eq!(cfg.hooks_timeout_ms(), 60_000);
    }

    #[test]
    fn max_retry_backoff_default() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.max_retry_backoff_ms(), 300_000);
    }

    #[test]
    fn max_turns_default() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.max_turns(), 20);
    }

    #[test]
    fn per_state_concurrency() {
        let cfg = config_from_yaml(
            "agent:\n  max_concurrent_agents_by_state:\n    Todo: 2\n    In Progress: 5",
        );
        let map = cfg.max_concurrent_agents_by_state();
        assert_eq!(map.get("todo"), Some(&2));
        assert_eq!(map.get("in progress"), Some(&5));
    }

    #[test]
    fn per_state_concurrency_ignores_invalid() {
        let cfg = config_from_yaml(
            "agent:\n  max_concurrent_agents_by_state:\n    Todo: 0\n    Bad: -1",
        );
        let map = cfg.max_concurrent_agents_by_state();
        assert!(map.is_empty());
    }

    #[test]
    fn validation_missing_tracker_kind() {
        let cfg = config_from_yaml("polling:\n  interval_ms: 1000");
        let err = cfg.validate_for_dispatch().unwrap_err();
        match err {
            SymphonyError::ValidationFailed { checks } => {
                assert!(checks.iter().any(|c| c.contains("tracker.kind")));
            }
            _ => panic!("expected ValidationFailed"),
        }
    }

    #[test]
    fn validation_linear_missing_project_slug() {
        let cfg = config_from_yaml(
            "tracker:\n  kind: linear\n  api_key: test-key",
        );
        let err = cfg.validate_for_dispatch().unwrap_err();
        match err {
            SymphonyError::ValidationFailed { checks } => {
                assert!(checks.iter().any(|c| c.contains("project_slug")));
            }
            _ => panic!("expected ValidationFailed"),
        }
    }

    #[test]
    fn validation_unsupported_kind() {
        let cfg = config_from_yaml("tracker:\n  kind: jira");
        let err = cfg.validate_for_dispatch().unwrap_err();
        match err {
            SymphonyError::ValidationFailed { checks } => {
                assert!(checks.iter().any(|c| c.contains("unsupported")));
            }
            _ => panic!("expected ValidationFailed"),
        }
    }

    #[test]
    fn stall_timeout_default() {
        let cfg = config_from_yaml("tracker:\n  kind: linear");
        assert_eq!(cfg.codex_stall_timeout_ms(), 300_000);
    }

    #[test]
    fn default_runner_kind() {
        let cfg = config_from_yaml("tracker:\n  kind: gitea");
        assert_eq!(cfg.runner_kind(), "codex");
    }

    #[test]
    fn custom_runner_kind() {
        let cfg = config_from_yaml("agent:\n  runner: claude-code");
        assert_eq!(cfg.runner_kind(), "claude-code");
    }

    #[test]
    fn claude_flags_none_by_default() {
        let cfg = config_from_yaml("tracker:\n  kind: gitea");
        assert!(cfg.claude_flags().is_none());
    }

    #[test]
    fn claude_flags_present() {
        let cfg = config_from_yaml(
            "agent:\n  claude_flags: \"--dangerously-skip-permissions --max-turns 10\"",
        );
        assert_eq!(
            cfg.claude_flags().unwrap(),
            "--dangerously-skip-permissions --max-turns 10"
        );
    }

    #[test]
    fn claude_settings_none_by_default() {
        let cfg = config_from_yaml("tracker:\n  kind: gitea");
        assert!(cfg.claude_settings().is_none());
    }

    #[test]
    fn claude_settings_file_path() {
        let cfg = config_from_yaml(
            "agent:\n  settings: /home/alex/.claude/symphony-settings.json",
        );
        assert_eq!(
            cfg.claude_settings().unwrap(),
            "/home/alex/.claude/symphony-settings.json"
        );
    }

    #[test]
    fn claude_settings_tilde_path() {
        let cfg = config_from_yaml(
            "agent:\n  settings: ~/.claude/symphony-settings.json",
        );
        assert_eq!(
            cfg.claude_settings().unwrap(),
            "~/.claude/symphony-settings.json"
        );
    }

    #[test]
    fn validation_claude_code_runner_no_codex_command_needed() {
        let cfg = config_from_yaml(
            "tracker:\n  kind: gitea\n  api_key: test\n  owner: o\n  repo: r\nagent:\n  runner: claude-code",
        );
        assert!(cfg.validate_for_dispatch().is_ok());
    }

    #[test]
    fn validation_unsupported_runner() {
        let cfg = config_from_yaml(
            "tracker:\n  kind: gitea\n  api_key: test\n  owner: o\n  repo: r\nagent:\n  runner: unknown-runner",
        );
        let err = cfg.validate_for_dispatch().unwrap_err();
        match err {
            SymphonyError::ValidationFailed { checks } => {
                assert!(checks.iter().any(|c| c.contains("unsupported agent.runner")));
            }
            _ => panic!("expected ValidationFailed"),
        }
    }

    #[test]
    fn env_var_resolution() {
        // SAFETY: test is single-threaded and uses a unique env var name
        unsafe { std::env::set_var("SYMPHONY_TEST_KEY_RES", "resolved_value") };
        assert_eq!(resolve_env_var("$SYMPHONY_TEST_KEY_RES"), "resolved_value");
        assert_eq!(resolve_env_var("literal"), "literal");
        unsafe { std::env::remove_var("SYMPHONY_TEST_KEY_RES") };
    }

    #[test]
    fn env_var_empty_resolution() {
        // SAFETY: test is single-threaded and uses a unique env var name
        unsafe { std::env::set_var("SYMPHONY_TEST_EMPTY_RES", "") };
        let cfg = config_from_yaml("tracker:\n  api_key: $SYMPHONY_TEST_EMPTY_RES");
        assert!(cfg.tracker_api_key().is_none());
        unsafe { std::env::remove_var("SYMPHONY_TEST_EMPTY_RES") };
    }
}
