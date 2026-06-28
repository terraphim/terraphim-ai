//! Workspace management for agent execution.
//!
//! Creates, reuses, and cleans up per-issue workspace directories.
//! Enforces path safety invariants and runs lifecycle hooks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

pub mod config;
use config::DANGEROUS_ENV_VARS;
pub use config::{HooksConfig, WorkspaceConfig};

/// Errors that can occur during workspace operations.
#[derive(thiserror::Error, Debug)]
pub enum WorkspaceError {
    /// A general workspace operation failed.
    #[error("workspace error for {identifier}: {reason}")]
    Workspace {
        /// Issue identifier for the workspace being operated on.
        identifier: String,
        /// Description of what went wrong.
        reason: String,
    },

    /// A path traversal safety check failed.
    #[error("path {path} is outside workspace root")]
    PathOutsideRoot {
        /// The offending path that was outside the root.
        path: String,
    },

    /// A lifecycle hook returned a non-zero exit code.
    #[error("hook '{hook}' failed: {reason}")]
    HookFailed {
        /// Name of the hook that failed.
        hook: String,
        /// Description of the failure.
        reason: String,
    },

    /// A lifecycle hook exceeded its time limit.
    #[error("hook '{hook}' timed out after {timeout_ms}ms")]
    HookTimeout {
        /// Name of the hook that timed out.
        hook: String,
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },

    /// A hook script contains potentially dangerous content.
    #[error("hook script invalid: {reason}")]
    HookScriptInvalid {
        /// Description of the validation failure.
        reason: String,
    },
}

/// Result type for workspace operations.
pub type Result<T> = std::result::Result<T, WorkspaceError>;

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
#[derive(Debug)]
pub struct WorkspaceManager {
    root: PathBuf,
    hooks: HooksConfig,
    hook_timeout_ms: u64,
}

impl WorkspaceManager {
    /// Create a new workspace manager.
    ///
    /// Returns an error if the root directory cannot be created or canonicalized,
    /// or if any hook script contains forbidden shell metacharacters.
    pub fn new(config: &WorkspaceConfig) -> Result<Self> {
        let root = &config.root;

        // Validate hook scripts before accepting the configuration.
        config
            .hooks
            .validate_scripts()
            .map_err(|reason| WorkspaceError::HookScriptInvalid { reason })?;

        // Ensure root exists
        std::fs::create_dir_all(root).map_err(|e| WorkspaceError::Workspace {
            identifier: "<root>".into(),
            reason: format!("failed to create workspace root {}: {e}", root.display()),
        })?;

        let root = root.canonicalize().map_err(|e| WorkspaceError::Workspace {
            identifier: "<root>".into(),
            reason: format!(
                "failed to canonicalise workspace root {}: {e}",
                root.display()
            ),
        })?;

        Ok(Self {
            root,
            hooks: config.hooks.clone(),
            hook_timeout_ms: config.hook_timeout_ms,
        })
    }

    /// Prepare a workspace for the given issue identifier.
    ///
    /// Creates the directory if it does not exist, runs the `after_create` hook
    /// for new workspaces, and returns workspace info.
    ///
    /// `env_vars` contains environment variables that are injected into hook processes.
    pub async fn prepare(
        &self,
        identifier: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<WorkspaceInfo> {
        let key = sanitise_workspace_key(identifier);
        let path = self.root.join(&key);

        // Safety: ensure path stays under root
        self.validate_path(&path, identifier)?;

        let created_now = !path.exists();
        if created_now {
            std::fs::create_dir_all(&path).map_err(|e| WorkspaceError::Workspace {
                identifier: identifier.into(),
                reason: format!("failed to create workspace directory: {e}"),
            })?;
            tracing::info!(
                issue_identifier = identifier,
                workspace_key = key,
                "created new workspace"
            );

            // Run after_create hook
            if let Some(script) = &self.hooks.after_create
                && let Err(e) = self.run_hook("after_create", script, &path, env_vars).await
            {
                // after_create failure is fatal: remove the directory
                tracing::warn!(
                    issue_identifier = identifier,
                    "after_create hook failed, removing workspace: {e}"
                );
                let _ = std::fs::remove_dir_all(&path);
                return Err(e);
            }
        } else {
            tracing::debug!(
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
    pub async fn run_before_run_hook(
        &self,
        workspace: &WorkspaceInfo,
        env_vars: &HashMap<String, String>,
    ) -> Result<()> {
        if let Some(script) = &self.hooks.before_run {
            self.run_hook("before_run", script, &workspace.path, env_vars)
                .await?;
        }
        Ok(())
    }

    /// Run the `after_run` hook. Failure is logged and ignored.
    pub async fn run_after_run_hook(
        &self,
        workspace: &WorkspaceInfo,
        env_vars: &HashMap<String, String>,
    ) {
        if let Some(script) = &self.hooks.after_run
            && let Err(e) = self
                .run_hook("after_run", script, &workspace.path, env_vars)
                .await
        {
            tracing::warn!(
                workspace_key = workspace.workspace_key,
                "after_run hook failed (ignored): {e}"
            );
        }
    }

    /// Clean up a workspace directory.
    pub async fn cleanup(&self, identifier: &str) -> Result<()> {
        let key = sanitise_workspace_key(identifier);
        let path = self.root.join(&key);

        if !path.exists() {
            return Ok(());
        }

        // Run before_remove hook (failure logged, ignored)
        if let Some(script) = &self.hooks.before_remove
            && let Err(e) = self
                .run_hook("before_remove", script, &path, &HashMap::new())
                .await
        {
            tracing::warn!(
                issue_identifier = identifier,
                "before_remove hook failed (ignored): {e}"
            );
        }

        std::fs::remove_dir_all(&path).map_err(|e| WorkspaceError::Workspace {
            identifier: identifier.into(),
            reason: format!("failed to remove workspace: {e}"),
        })?;
        tracing::info!(issue_identifier = identifier, "removed workspace");
        Ok(())
    }

    /// Remove workspaces for identifiers in terminal states (startup cleanup).
    pub async fn cleanup_terminal_workspaces(&self, terminal_identifiers: &[String]) {
        for identifier in terminal_identifiers {
            if let Err(e) = self.cleanup(identifier).await {
                tracing::warn!(
                    issue_identifier = identifier,
                    "startup terminal cleanup failed: {e}"
                );
            }
        }
    }

    /// Archive a workspace by renaming it with a timestamp instead of deleting.
    ///
    /// The destination name is `<key>_archived_<YYYYMMDD_HHMMSS>`. If that path
    /// already exists — which happens when the same key is archived twice within
    /// one second — a numeric disambiguator (`_2`, `_3`, …) is appended until a
    /// free path is found. Without this guard `rename(2)` fails with `ENOTEMPTY`
    /// on Linux and the second archive would be lost (issue #2885).
    pub async fn archive(&self, identifier: &str) -> Result<PathBuf> {
        let key = sanitise_workspace_key(identifier);
        let path = self.root.join(&key);

        if !path.exists() {
            return Err(WorkspaceError::Workspace {
                identifier: identifier.into(),
                reason: "cannot archive non-existent workspace".into(),
            });
        }

        let timestamp = jiff::Zoned::now().strftime("%Y%m%d_%H%M%S").to_string();
        let archive_path = self.resolve_archive_path(&key, &timestamp);

        std::fs::rename(&path, &archive_path).map_err(|e| WorkspaceError::Workspace {
            identifier: identifier.into(),
            reason: format!("failed to archive workspace: {e}"),
        })?;

        tracing::info!(
            issue_identifier = identifier,
            archive_path = %archive_path.display(),
            "archived workspace"
        );

        Ok(archive_path)
    }

    /// Pick a non-existent archive destination for `key` under `root`.
    ///
    /// Starts from `<key>_archived_<timestamp>` and, on collision, appends an
    /// incrementing numeric suffix (`_2`, `_3`, …). Bounded by [`u32::MAX`]; in
    /// practice the loop exits on the first or second iteration.
    fn resolve_archive_path(&self, key: &str, timestamp: &str) -> PathBuf {
        let base = format!("{key}_archived_{timestamp}");
        let mut candidate = self.root.join(&base);
        for suffix in 2u32.. {
            if !candidate.exists() {
                return candidate;
            }
            candidate = self.root.join(format!("{base}_{suffix}"));
        }
        // Unreachable: the loop above runs u32::MAX iterations before reaching here.
        candidate
    }

    /// Validate that a workspace path stays under the workspace root.
    ///
    /// Uses component-aware [`Path::starts_with`] rather than string comparison to
    /// prevent prefix-confusion attacks where a root of `/tmp/ws` would incorrectly
    /// accept `/tmp/ws_evil` under string-based matching.
    fn validate_path(&self, path: &Path, identifier: &str) -> Result<()> {
        if !path.starts_with(&self.root) {
            return Err(WorkspaceError::PathOutsideRoot {
                path: path.to_string_lossy().into_owned(),
            });
        }

        // Reject if workspace key would create subdirectories
        let key = sanitise_workspace_key(identifier);
        if key.contains('/') || key.contains('\\') {
            return Err(WorkspaceError::PathOutsideRoot {
                path: path.to_string_lossy().into_owned(),
            });
        }
        Ok(())
    }

    /// Execute a hook shell script in the workspace directory.
    async fn run_hook(
        &self,
        hook_name: &str,
        script: &str,
        cwd: &Path,
        env_vars: &HashMap<String, String>,
    ) -> Result<()> {
        tracing::debug!(hook = hook_name, cwd = %cwd.display(), "running hook");

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(script)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (key, val) in env_vars {
            // Strip dynamic-linker variables that enable library injection attacks.
            if DANGEROUS_ENV_VARS.contains(&key.as_str()) {
                tracing::warn!(
                    hook = hook_name,
                    env_key = key,
                    "stripping dangerous env var from hook subprocess"
                );
                continue;
            }
            cmd.env(key, val);
        }

        let child = cmd.spawn().map_err(|e| WorkspaceError::HookFailed {
            hook: hook_name.into(),
            reason: format!("failed to spawn: {e}"),
        })?;

        let result = tokio::time::timeout(
            Duration::from_millis(self.hook_timeout_ms),
            child.wait_with_output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    tracing::debug!(hook = hook_name, "hook completed successfully");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let truncated = if stderr.len() > 500 {
                        format!("{}...", &stderr[..500])
                    } else {
                        stderr.to_string()
                    };
                    Err(WorkspaceError::HookFailed {
                        hook: hook_name.into(),
                        reason: format!(
                            "exit code {}: {}",
                            output.status.code().unwrap_or(-1),
                            truncated
                        ),
                    })
                }
            }
            Ok(Err(e)) => Err(WorkspaceError::HookFailed {
                hook: hook_name.into(),
                reason: format!("IO error: {e}"),
            }),
            Err(_) => {
                tracing::error!(
                    hook = hook_name,
                    timeout_ms = self.hook_timeout_ms,
                    "hook timed out"
                );
                Err(WorkspaceError::HookTimeout {
                    hook: hook_name.into(),
                    timeout_ms: self.hook_timeout_ms,
                })
            }
        }
    }

    /// Get the workspace root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Maximum character count for a workspace key directory name.
const MAX_WORKSPACE_KEY_LEN: usize = 200;

/// Sanitise an issue identifier for use as a directory name.
///
/// Replaces any character not in `[A-Za-z0-9._-]` with `_` and caps the output
/// at [`MAX_WORKSPACE_KEY_LEN`] characters to prevent filesystem path-length issues.
pub fn sanitise_workspace_key(identifier: &str) -> String {
    identifier
        .chars()
        .take(MAX_WORKSPACE_KEY_LEN)
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
        assert_eq!(sanitise_workspace_key("issue-é"), "issue-_");
    }

    #[tokio::test]
    async fn workspace_create_and_reuse() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig::default(),
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        let env = HashMap::new();

        // First call creates
        let info = mgr.prepare("MT-42", &env).await.unwrap();
        assert!(info.created_now);
        assert!(info.path.exists());
        assert_eq!(info.workspace_key, "MT-42");

        // Second call reuses
        let info2 = mgr.prepare("MT-42", &env).await.unwrap();
        assert!(!info2.created_now);
        assert_eq!(info.path, info2.path);
    }

    #[tokio::test]
    async fn workspace_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig::default(),
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        let env = HashMap::new();

        mgr.prepare("MT-99", &env).await.unwrap();
        assert!(tmp.path().join("MT-99").exists());

        mgr.cleanup("MT-99").await.unwrap();
        assert!(!tmp.path().join("MT-99").exists());
    }

    #[tokio::test]
    async fn cleanup_nonexistent_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig::default(),
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();

        // Should not error
        mgr.cleanup("NONEXISTENT-1").await.unwrap();
    }

    #[tokio::test]
    async fn hook_execution_success() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig {
                after_create: Some("touch created.txt".into()),
                ..Default::default()
            },
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        let env = HashMap::new();

        let info = mgr.prepare("HOOK-1", &env).await.unwrap();
        assert!(info.path.join("created.txt").exists());
    }

    #[tokio::test]
    async fn hook_failure_removes_new_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig {
                after_create: Some("exit 1".into()),
                ..Default::default()
            },
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        let env = HashMap::new();

        let result = mgr.prepare("HOOK-FAIL", &env).await;
        assert!(result.is_err());
        // Directory should be cleaned up
        assert!(!tmp.path().join("HOOK-FAIL").exists());
    }

    #[tokio::test]
    async fn hook_receives_env_vars() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig {
                after_create: Some("echo $TEST_VAR > var.txt".into()),
                ..Default::default()
            },
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        let mut env = HashMap::new();
        env.insert("TEST_VAR".into(), "hello".into());

        let info = mgr.prepare("ENV-TEST", &env).await.unwrap();
        let content = std::fs::read_to_string(info.path.join("var.txt")).unwrap();
        assert_eq!(content.trim(), "hello");
    }

    #[test]
    fn path_outside_root_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig::default(),
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();

        let bad_path = PathBuf::from("/tmp/elsewhere/evil");
        let result = mgr.validate_path(&bad_path, "../evil");
        assert!(result.is_err());
    }

    #[test]
    fn path_prefix_confusion_is_rejected() {
        // Verify that component-aware Path::starts_with is used, not string starts_with.
        // e.g. root="/tmp/ws" must NOT accept path="/tmp/ws_evil/file".
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        // Construct a sibling path whose string representation shares the root prefix
        // but differs at the path-component boundary.
        let sibling_name = format!("{}_evil", tmp.path().file_name().unwrap().to_string_lossy());
        let sibling = tmp.path().parent().unwrap().join(sibling_name).join("file");

        let config = WorkspaceConfig {
            root,
            hooks: HooksConfig::default(),
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();

        let result = mgr.validate_path(&sibling, "file");
        assert!(
            result.is_err(),
            "sibling path with shared string prefix must be rejected"
        );
    }

    #[test]
    fn sanitise_key_truncates_long_identifier() {
        let long = "a".repeat(500);
        let key = sanitise_workspace_key(&long);
        assert_eq!(key.len(), MAX_WORKSPACE_KEY_LEN);
    }

    #[test]
    fn sanitise_key_exact_boundary() {
        let at_limit = "x".repeat(MAX_WORKSPACE_KEY_LEN);
        let key = sanitise_workspace_key(&at_limit);
        assert_eq!(key.len(), MAX_WORKSPACE_KEY_LEN);

        let over_limit = "x".repeat(MAX_WORKSPACE_KEY_LEN + 1);
        let key2 = sanitise_workspace_key(&over_limit);
        assert_eq!(key2.len(), MAX_WORKSPACE_KEY_LEN);
    }

    #[test]
    fn hook_script_injection_rejected_at_construction() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig::new().with_after_create("evil $(cmd)"),
            hook_timeout_ms: 60000,
        };
        let result = WorkspaceManager::new(&config);
        assert!(
            result.is_err(),
            "WorkspaceManager::new must reject scripts with injection patterns"
        );
        assert!(matches!(
            result.unwrap_err(),
            WorkspaceError::HookScriptInvalid { .. }
        ));
    }

    #[tokio::test]
    async fn dangerous_env_var_is_stripped_from_hook() {
        let tmp = tempfile::tempdir().unwrap();
        // Script writes the LD_PRELOAD value to a file; if stripped it produces an empty line.
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig {
                after_create: Some("printf '%s' \"$LD_PRELOAD\" > ld_preload.txt".into()),
                ..Default::default()
            },
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        let mut env = HashMap::new();
        env.insert("LD_PRELOAD".into(), "libevil.so".into());

        let info = mgr.prepare("SEC-TEST", &env).await.unwrap();
        let content = std::fs::read_to_string(info.path.join("ld_preload.txt")).unwrap();
        // LD_PRELOAD must have been stripped; the script sees an empty variable.
        assert!(
            content.is_empty(),
            "LD_PRELOAD should have been stripped, got: {content:?}"
        );
    }

    // --- archive() coverage (issue #2885) ---------------------------------

    /// Helper: build a WorkspaceManager rooted at a fresh tempdir.
    fn manager_in_tmp() -> (tempfile::TempDir, WorkspaceManager) {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkspaceConfig {
            root: tmp.path().to_path_buf(),
            hooks: HooksConfig::default(),
            hook_timeout_ms: 60000,
        };
        let mgr = WorkspaceManager::new(&config).unwrap();
        (tmp, mgr)
    }

    #[tokio::test]
    async fn archive_creates_timestamped_directory() {
        let (tmp, mgr) = manager_in_tmp();
        let env = HashMap::new();

        // Prepare a workspace with a marker file so we can verify content survives the rename.
        let info = mgr.prepare("ARCH-1", &env).await.unwrap();
        std::fs::write(info.path.join("marker.txt"), b"payload").unwrap();

        let archive_path = mgr.archive("ARCH-1").await.unwrap();

        // Original path must no longer exist...
        assert!(
            !info.path.exists(),
            "original workspace path must be gone after archive"
        );
        // ...archive directory must exist under root ...
        assert!(
            archive_path.starts_with(tmp.path()),
            "archive path must stay under workspace root"
        );
        assert!(archive_path.is_dir(), "archive path must be a directory");
        // ...with the documented naming scheme <key>_archived_<YYYYMMDD_HHMMSS>.
        let name = archive_path.file_name().unwrap().to_string_lossy();
        assert!(
            name.starts_with("ARCH-1_archived_"),
            "archive dir name must start with '<key>_archived_', got: {name}"
        );
        let ts = &name["ARCH-1_archived_".len()..];
        assert_eq!(
            ts.len(),
            15,
            "timestamp suffix must be YYYYMMDD_HHMMSS (15 chars), got: {ts:?}"
        );
        assert_eq!(
            ts.as_bytes()[8],
            b'_',
            "timestamp must use '_' between date and time, got: {ts:?}"
        );

        // Content must survive the rename.
        assert_eq!(
            std::fs::read(archive_path.join("marker.txt")).unwrap(),
            b"payload",
            "archived workspace must retain its contents"
        );
    }

    #[tokio::test]
    async fn archive_nonexistent_returns_error() {
        let (_tmp, mgr) = manager_in_tmp();

        let result = mgr.archive("DOES-NOT-EXIST").await;
        let err = result.expect_err("archiving a non-existent workspace must error");
        assert!(
            matches!(err, WorkspaceError::Workspace { .. }),
            "expected WorkspaceError::Workspace, got: {err:?}"
        );
        // The reason must mention the non-existence (no opaque ENOTEMPTY leak).
        let WorkspaceError::Workspace { reason, .. } = &err else {
            unreachable!()
        };
        assert!(
            reason.contains("non-existent"),
            "error reason must explain the cause, got: {reason}"
        );
    }

    /// Regression for the timestamp-collision bug from #2885: archiving the same
    /// key twice within the same second previously targeted an identical
    /// `<key>_archived_<ts>` path; on Linux `rename(2)` returns `ENOTEMPTY` for a
    /// non-empty target directory, surfacing as an opaque error and discarding
    /// the second archive. Both archives must now coexist.
    #[tokio::test]
    async fn archive_collision_uses_disambiguating_suffix() {
        let (tmp, mgr) = manager_in_tmp();
        let env = HashMap::new();

        // Two distinct source workspaces that sanitise to the SAME key.
        let a = mgr.prepare("COLLIDE-1", &env).await.unwrap();
        std::fs::write(a.path.join("a.txt"), b"a").unwrap();
        let first = mgr.archive("COLLIDE-1").await.unwrap();

        // Recreate a workspace with the identical key and archive again immediately.
        let b = mgr.prepare("COLLIDE-1", &env).await.unwrap();
        std::fs::write(b.path.join("b.txt"), b"b").unwrap();
        let second = mgr.archive("COLLIDE-1").await.unwrap();

        // The two archive paths must differ and both survive under root.
        assert_ne!(
            first, second,
            "collision must be disambiguated, not overwrite the first archive"
        );
        assert!(first.is_dir(), "first archive must still exist");
        assert!(second.is_dir(), "second archive must exist");
        assert!(first.starts_with(tmp.path()));
        assert!(second.starts_with(tmp.path()));

        // Both archives' content must be intact (no clobber).
        assert_eq!(
            std::fs::read(first.join("a.txt")).unwrap(),
            b"a",
            "first archive content must survive the second archive"
        );
        assert_eq!(std::fs::read(second.join("b.txt")).unwrap(), b"b");

        // The disambiguated name must share the timestamp prefix but carry a
        // numeric suffix, so both remain sortable/identifiable as archives of the
        // same key.
        let first_name = first.file_name().unwrap().to_string_lossy().into_owned();
        let second_name = second.file_name().unwrap().to_string_lossy().into_owned();
        let prefix = "COLLIDE-1_archived_";
        assert!(
            first_name.starts_with(prefix) && second_name.starts_with(prefix),
            "both archives must keep the documented prefix; got {first_name:?} / {second_name:?}"
        );
        assert_ne!(first_name, second_name);
    }
}
