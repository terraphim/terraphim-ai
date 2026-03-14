//! Workspace management for Symphony.
//!
//! Creates, reuses, and cleans up per-issue workspace directories.
//! Enforces path safety invariants and runs lifecycle hooks.

use crate::config::ServiceConfig;
use crate::error::{Result, SymphonyError};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Result of workspace preparation.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Absolute path to the workspace directory.
    pub path: PathBuf,
    /// Sanitised issue identifier used as the directory name.
    pub workspace_key: String,
    /// Whether the directory was newly created (vs reused).
    pub created_now: bool,
}

/// Manages per-issue workspaces on the filesystem.
pub struct WorkspaceManager {
    root: PathBuf,
    config: ServiceConfig,
}

impl WorkspaceManager {
    /// Create a new workspace manager.
    pub fn new(config: &ServiceConfig) -> Result<Self> {
        let root = config.workspace_root();
        // Ensure root exists
        std::fs::create_dir_all(&root).map_err(|e| SymphonyError::Workspace {
            identifier: "<root>".into(),
            reason: format!("failed to create workspace root {}: {e}", root.display()),
        })?;

        let root = root.canonicalize().map_err(|e| SymphonyError::Workspace {
            identifier: "<root>".into(),
            reason: format!(
                "failed to canonicalise workspace root {}: {e}",
                config.workspace_root().display()
            ),
        })?;

        Ok(Self {
            root,
            config: config.clone(),
        })
    }

    /// Prepare a workspace for the given issue identifier.
    ///
    /// Creates the directory if it does not exist, runs the `after_create` hook
    /// for new workspaces, and returns workspace info.
    pub async fn prepare(&self, identifier: &str) -> Result<WorkspaceInfo> {
        let key = sanitise_workspace_key(identifier);
        let path = self.root.join(&key);

        // Safety: ensure path stays under root
        self.validate_path(&path, identifier)?;

        let created_now = !path.exists();
        if created_now {
            std::fs::create_dir_all(&path).map_err(|e| SymphonyError::Workspace {
                identifier: identifier.into(),
                reason: format!("failed to create workspace directory: {e}"),
            })?;
            info!(
                issue_identifier = identifier,
                workspace_key = key,
                "created new workspace"
            );

            // Run after_create hook
            if let Some(script) = self.config.hooks_after_create() {
                if let Err(e) = self.run_hook("after_create", &script, &path).await {
                    // after_create failure is fatal: remove the directory
                    warn!(
                        issue_identifier = identifier,
                        "after_create hook failed, removing workspace: {e}"
                    );
                    let _ = std::fs::remove_dir_all(&path);
                    return Err(e);
                }
            }
        } else {
            debug!(
                issue_identifier = identifier,
                workspace_key = key,
                "reusing existing workspace"
            );
        }

        Ok(WorkspaceInfo {
            path,
            workspace_key: key,
            created_now,
        })
    }

    /// Run the `before_run` hook. Failure aborts the current attempt.
    pub async fn run_before_run_hook(&self, workspace: &WorkspaceInfo) -> Result<()> {
        if let Some(script) = self.config.hooks_before_run() {
            self.run_hook("before_run", &script, &workspace.path).await?;
        }
        Ok(())
    }

    /// Run the `after_run` hook. Failure is logged and ignored.
    pub async fn run_after_run_hook(&self, workspace: &WorkspaceInfo) {
        if let Some(script) = self.config.hooks_after_run() {
            if let Err(e) = self.run_hook("after_run", &script, &workspace.path).await {
                warn!(
                    workspace_key = workspace.workspace_key,
                    "after_run hook failed (ignored): {e}"
                );
            }
        }
    }

    /// Clean up a workspace directory for a terminal issue.
    pub async fn cleanup(&self, identifier: &str) -> Result<()> {
        let key = sanitise_workspace_key(identifier);
        let path = self.root.join(&key);

        if !path.exists() {
            return Ok(());
        }

        // Run before_remove hook (failure logged, ignored)
        if let Some(script) = self.config.hooks_before_remove() {
            if let Err(e) = self.run_hook("before_remove", &script, &path).await {
                warn!(
                    issue_identifier = identifier,
                    "before_remove hook failed (ignored): {e}"
                );
            }
        }

        std::fs::remove_dir_all(&path).map_err(|e| SymphonyError::Workspace {
            identifier: identifier.into(),
            reason: format!("failed to remove workspace: {e}"),
        })?;
        info!(issue_identifier = identifier, "removed workspace");
        Ok(())
    }

    /// Remove workspaces for issues in terminal states (startup cleanup).
    pub async fn cleanup_terminal_workspaces(&self, terminal_identifiers: &[String]) {
        for identifier in terminal_identifiers {
            if let Err(e) = self.cleanup(identifier).await {
                warn!(
                    issue_identifier = identifier,
                    "startup terminal cleanup failed: {e}"
                );
            }
        }
    }

    /// Validate that a workspace path stays under the workspace root.
    fn validate_path(&self, path: &Path, identifier: &str) -> Result<()> {
        // Use the path as-is (it's constructed from root + sanitised key).
        // Extra check: ensure no path traversal via canonicalisation after creation.
        // Since we built the path ourselves from sanitised components, this is
        // primarily a defence-in-depth check.
        let path_str = path.to_string_lossy();
        let root_str = self.root.to_string_lossy();

        if !path_str.starts_with(root_str.as_ref()) {
            return Err(SymphonyError::WorkspacePathOutsideRoot {
                path: path_str.into_owned(),
            });
        }
        // Reject if workspace key would create subdirectories
        let key = sanitise_workspace_key(identifier);
        if key.contains('/') || key.contains('\\') {
            return Err(SymphonyError::WorkspacePathOutsideRoot {
                path: path_str.into_owned(),
            });
        }
        Ok(())
    }

    /// Execute a hook shell script in the workspace directory.
    async fn run_hook(&self, hook_name: &str, script: &str, cwd: &Path) -> Result<()> {
        let timeout_ms = self.config.hooks_timeout_ms();
        debug!(hook = hook_name, cwd = %cwd.display(), "running hook");

        let child = Command::new("sh")
            .arg("-lc")
            .arg(script)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SymphonyError::HookFailed {
                hook: hook_name.into(),
                reason: format!("failed to spawn: {e}"),
            })?;

        let result =
            tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    debug!(hook = hook_name, "hook completed successfully");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let truncated = if stderr.len() > 500 {
                        format!("{}...", &stderr[..500])
                    } else {
                        stderr.to_string()
                    };
                    Err(SymphonyError::HookFailed {
                        hook: hook_name.into(),
                        reason: format!(
                            "exit code {}: {}",
                            output.status.code().unwrap_or(-1),
                            truncated
                        ),
                    })
                }
            }
            Ok(Err(e)) => Err(SymphonyError::HookFailed {
                hook: hook_name.into(),
                reason: format!("IO error: {e}"),
            }),
            Err(_) => {
                error!(hook = hook_name, timeout_ms, "hook timed out");
                Err(SymphonyError::HookTimeout {
                    hook: hook_name.into(),
                    timeout_ms,
                })
            }
        }
    }

    /// Get the workspace root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Sanitise an issue identifier for use as a directory name.
///
/// Replaces any character not in `[A-Za-z0-9._-]` with `_`.
pub fn sanitise_workspace_key(identifier: &str) -> String {
    identifier
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_simple_identifier() {
        assert_eq!(sanitise_workspace_key("MT-42"), "MT-42");
    }

    #[test]
    fn sanitise_identifier_with_special_chars() {
        assert_eq!(sanitise_workspace_key("owner/repo#42"), "owner_repo_42");
    }

    #[test]
    fn sanitise_identifier_with_spaces() {
        assert_eq!(sanitise_workspace_key("MT 42"), "MT_42");
    }

    #[test]
    fn sanitise_preserves_dots_and_underscores() {
        assert_eq!(sanitise_workspace_key("v1.2_alpha"), "v1.2_alpha");
    }

    #[test]
    fn sanitise_unicode() {
        assert_eq!(sanitise_workspace_key("issue-\u{00e9}"), "issue-_");
    }

    #[tokio::test]
    async fn workspace_create_and_reuse() {
        let tmp = tempfile::TempDir::new().unwrap();
        let yaml = format!(
            "---\nworkspace:\n  root: \"{}\"\ntracker:\n  kind: linear\n---\nPrompt.",
            tmp.path().display()
        );
        let workflow =
            crate::config::workflow::WorkflowDefinition::parse(&yaml).unwrap();
        let cfg = crate::config::ServiceConfig::from_workflow(workflow);
        let mgr = WorkspaceManager::new(&cfg).unwrap();

        // First call creates
        let info = mgr.prepare("MT-42").await.unwrap();
        assert!(info.created_now);
        assert!(info.path.exists());
        assert_eq!(info.workspace_key, "MT-42");

        // Second call reuses
        let info2 = mgr.prepare("MT-42").await.unwrap();
        assert!(!info2.created_now);
        assert_eq!(info.path, info2.path);
    }

    #[tokio::test]
    async fn workspace_cleanup() {
        let tmp = tempfile::TempDir::new().unwrap();
        let yaml = format!(
            "---\nworkspace:\n  root: \"{}\"\ntracker:\n  kind: linear\n---\nPrompt.",
            tmp.path().display()
        );
        let workflow =
            crate::config::workflow::WorkflowDefinition::parse(&yaml).unwrap();
        let cfg = crate::config::ServiceConfig::from_workflow(workflow);
        let mgr = WorkspaceManager::new(&cfg).unwrap();

        mgr.prepare("MT-99").await.unwrap();
        assert!(tmp.path().join("MT-99").exists());

        mgr.cleanup("MT-99").await.unwrap();
        assert!(!tmp.path().join("MT-99").exists());
    }

    #[tokio::test]
    async fn cleanup_nonexistent_is_ok() {
        let tmp = tempfile::TempDir::new().unwrap();
        let yaml = format!(
            "---\nworkspace:\n  root: \"{}\"\ntracker:\n  kind: linear\n---\nPrompt.",
            tmp.path().display()
        );
        let workflow =
            crate::config::workflow::WorkflowDefinition::parse(&yaml).unwrap();
        let cfg = crate::config::ServiceConfig::from_workflow(workflow);
        let mgr = WorkspaceManager::new(&cfg).unwrap();

        // Should not error
        mgr.cleanup("NONEXISTENT-1").await.unwrap();
    }

    #[tokio::test]
    async fn hook_execution_success() {
        let tmp = tempfile::TempDir::new().unwrap();
        let yaml = format!(
            "---\nworkspace:\n  root: \"{}\"\nhooks:\n  after_create: \"touch created.txt\"\ntracker:\n  kind: linear\n---\nPrompt.",
            tmp.path().display()
        );
        let workflow =
            crate::config::workflow::WorkflowDefinition::parse(&yaml).unwrap();
        let cfg = crate::config::ServiceConfig::from_workflow(workflow);
        let mgr = WorkspaceManager::new(&cfg).unwrap();

        let info = mgr.prepare("HOOK-1").await.unwrap();
        assert!(info.path.join("created.txt").exists());
    }

    #[tokio::test]
    async fn hook_failure_removes_new_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let yaml = format!(
            "---\nworkspace:\n  root: \"{}\"\nhooks:\n  after_create: \"exit 1\"\ntracker:\n  kind: linear\n---\nPrompt.",
            tmp.path().display()
        );
        let workflow =
            crate::config::workflow::WorkflowDefinition::parse(&yaml).unwrap();
        let cfg = crate::config::ServiceConfig::from_workflow(workflow);
        let mgr = WorkspaceManager::new(&cfg).unwrap();

        let result = mgr.prepare("HOOK-FAIL").await;
        assert!(result.is_err());
        // Directory should be cleaned up
        assert!(!tmp.path().join("HOOK-FAIL").exists());
    }

    #[test]
    fn path_outside_root_is_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let yaml = format!(
            "---\nworkspace:\n  root: \"{}\"\ntracker:\n  kind: linear\n---\nPrompt.",
            tmp.path().display()
        );
        let workflow =
            crate::config::workflow::WorkflowDefinition::parse(&yaml).unwrap();
        let cfg = crate::config::ServiceConfig::from_workflow(workflow);
        let mgr = WorkspaceManager::new(&cfg).unwrap();

        let bad_path = PathBuf::from("/tmp/elsewhere/evil");
        let result = mgr.validate_path(&bad_path, "../evil");
        assert!(result.is_err());
    }
}
